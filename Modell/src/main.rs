mod telemetry;

use nerva_qemu::{
    Result,
    capture::Frame,
    keyboard::{Key, Keyboard},
    qmp::Qmp,
    vision_encoder::Encoding,
};
use nerva_vm::{
    model::{KeyboardLayout, Model, ModelConfig, ModelKind, WINDOW_US},
    visual_vm::DemoVm,
};
use serde_json::json;
use std::{
    env, fs,
    io::Write,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

fn main() {
    if let Err(error) = run() {
        eprintln!("nerva-vm: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let mut args = env::args().skip(1);
    let command = args.next().unwrap_or_else(|| "help".into());
    if matches!(command.as_str(), "help" | "--help" | "-h") {
        println!(
            "Usage:\n  nerva-vm demo [OPTIONS]\n  nerva-vm smoke [OPTIONS]\n  nerva-vm attach QMP_SOCKET [OPTIONS]\n  nerva-vm layout\n\nOptions: --dashboard (write live observations), --steps N (40), --seed N (42),\n --keyboard full|wasd|ad (default full, German ISO-105),\n --encoding brightness|change, --motor-decoder timing|count (default timing),\n --model recurrent|direct, --processing N (256), --frozen,\n --fixed-topology, --no-feedback, --no-homeostasis,\n --initial-drive F (2.5), --pruning-min-us N (300000), --output NEW_DIRECTORY\n\nDemo/smoke boot a disposable VGA guest. Attach leaves the existing VM running.\nSmoke verifies A/D key effects and all 105 physical keyboard codes.\nResearch protocols live in Experimente/run.sh."
        );
        return Ok(());
    }
    if command == "layout" {
        if args.next().is_some() {
            return Err("layout takes no arguments".into());
        }
        println!(
            "German ISO-105 keyboard; configure the guest with a German keymap.\nchannel\tkey\tqcode"
        );
        for key in Key::ALL {
            println!("{}\t{}\t{}", key.channel(), key.label(), key.qcode());
        }
        return Ok(());
    }
    let socket = match command.as_str() {
        "demo" | "smoke" => None,
        "attach" => Some(PathBuf::from(
            args.next().ok_or("attach requires a QMP socket")?,
        )),
        _ => {
            return Err(
                "unknown model command; use --help (experiments: Experimente/run.sh)".into(),
            );
        }
    };
    let mut steps = 40usize;
    let mut model_config = ModelConfig::default();
    let mut output = None;
    let mut dashboard_enabled = false;
    while let Some(option) = args.next() {
        match option.as_str() {
            "--steps" => steps = args.next().ok_or("missing step count")?.parse()?,
            "--seed" => model_config.seed = args.next().ok_or("missing seed")?.parse()?,
            "--encoding" => {
                model_config.encoding = match args.next().as_deref() {
                    Some("brightness") => Encoding::Brightness,
                    Some("change") => Encoding::Change,
                    _ => return Err("encoding must be brightness or change".into()),
                }
            }
            "--frozen" => model_config.learning = false,
            "--dashboard" => dashboard_enabled = true,
            "--motor-decoder" => {
                model_config.motor_timing = match args.next().as_deref() {
                    Some("timing") => true,
                    Some("count") => false,
                    _ => return Err("motor decoder must be timing or count".into()),
                }
            }
            "--processing" => {
                model_config.processing_neurons =
                    args.next().ok_or("missing processing count")?.parse()?
            }
            "--model" => {
                model_config.kind = match args.next().as_deref() {
                    Some("recurrent") => ModelKind::Recurrent,
                    Some("direct") => ModelKind::Direct,
                    _ => return Err("model must be recurrent or direct".into()),
                }
            }
            "--fixed-topology" => model_config.development = false,
            "--no-feedback" => model_config.feedback = false,
            "--no-homeostasis" => model_config.homeostasis = false,
            "--initial-drive" => {
                model_config.initial_structural_drive =
                    args.next().ok_or("missing initial drive")?.parse()?
            }
            "--pruning-min-us" => {
                model_config.pruning_min_below_us =
                    args.next().ok_or("missing pruning duration")?.parse()?
            }
            "--keyboard" => {
                model_config.keyboard = match args.next().as_deref() {
                    Some("full") => KeyboardLayout::Full,
                    Some("wasd") => KeyboardLayout::Wasd,
                    Some("ad") => KeyboardLayout::Ad,
                    _ => return Err("keyboard must be full, wasd or ad".into()),
                }
            }
            "--output" if output.is_none() => {
                output = Some(PathBuf::from(
                    args.next().ok_or("missing output directory")?,
                ))
            }
            _ => return Err(format!("unexpected option: {option}").into()),
        }
    }
    if !(1..=10_000).contains(&steps) {
        return Err("steps must be in 1..=10000".into());
    }
    if !model_config.learning {
        model_config = model_config.frozen();
    }
    model_config.validate()?;
    let seed = model_config.seed;
    let mode = model_config.encoding;
    let learning = model_config.learning;
    let layout = model_config.keyboard;
    let cancelled = Arc::new(AtomicBool::new(false));
    let interrupt = Arc::clone(&cancelled);
    ctrlc::set_handler(move || interrupt.store(true, Ordering::Relaxed))?;
    let output = output.unwrap_or_else(|| {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../Messungen/Modell")
            .join(format!("{stamp}-{}", std::process::id()))
    });
    let output = if output.is_absolute() {
        output
    } else {
        env::current_dir()?.join(output)
    };
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::create_dir(&output)?;
    let mut dashboard = if dashboard_enabled {
        Some(telemetry::Telemetry::new(&output)?)
    } else {
        None
    };
    let (_vm, qmp) = match socket {
        Some(socket) => (None, Qmp::connect(&socket)?),
        None => {
            let (vm, qmp) = DemoVm::launch(&cancelled)?;
            (Some(vm), qmp)
        }
    };
    let mut keyboard = Keyboard::new(qmp);
    let capture_path = output.join("current.ppm");
    if _vm.is_some() {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if cancelled.load(Ordering::Relaxed) {
                return Err("interrupted while waiting for the VGA screen".into());
            }
            let frame = keyboard.capture(&capture_path)?;
            // Standard VGA may double mode 13h scanlines and pixel clocks.
            if matches!((frame.width(), frame.height()), (320, 200) | (640, 400)) {
                thread::sleep(Duration::from_millis(50));
                break;
            }
            if Instant::now() >= deadline {
                return Err("VGA guest did not boot within 10 seconds".into());
            }
            thread::sleep(Duration::from_millis(50));
        }
    }
    if command == "smoke" {
        let before = keyboard.capture(&capture_path)?;
        pulse(&mut keyboard, Key::D)?;
        let right = keyboard.capture(&capture_path)?;
        pulse(&mut keyboard, Key::A)?;
        let restored = keyboard.capture(&capture_path)?;
        if before == right || before != restored {
            return Err("A/D screenshot round trip failed".into());
        }
        println!("QMP_KEYBOARD_PASS: D changed the pixels; A restored the image.");
        for &key in Key::ALL {
            keyboard.set_keys([key])?;
            keyboard.set_keys([])?;
        }
        pulse(&mut keyboard, Key::Space)?;
        println!("QMP_FULL_KEYBOARD_PASS: all 105 key-down/up codes accepted.");
    }
    let mut model = Model::from_config(model_config)?;
    let effective_config = model.config().clone();
    let mut core_activity = fs::File::create_new(output.join("core-activity.csv"))?;
    writeln!(core_activity, "step,neuron,spikes")?;
    let mut structure = fs::File::create_new(output.join("structure.csv"))?;
    writeln!(structure, "step,event,synapse,source,target")?;
    let mut core_spikes = 0usize;
    let mut formed = 0usize;
    let mut pruned = 0usize;
    let mut raw_feedback = 0usize;
    let mut action_feedback = 0usize;
    let mut homeostasis_changes = 0usize;
    let mut key_layout = fs::File::create_new(output.join("keyboard-layout.csv"))?;
    writeln!(key_layout, "channel,label,qcode")?;
    for key in layout.keys() {
        writeln!(
            key_layout,
            "{},\"{}\",{}",
            key.channel(),
            key.label(),
            key.qcode()
        )?;
    }
    let mut motor_activity = fs::File::create_new(output.join("motor-activity.csv"))?;
    writeln!(motor_activity, "step,channel,qcode,spikes")?;
    let mut trace = fs::File::create_new(output.join("trace.csv"))?;
    writeln!(
        trace,
        "step,simulation_end_us,sensory_spikes,motor_spikes,weight_changes,key,frame_changed"
    )?;
    let mut previous: Option<Frame> = None;
    let mut sensory_spikes = 0usize;
    let mut motor_spikes = 0usize;
    let mut weight_changes = 0usize;
    let mut key_frames = 0usize;
    let mut frame_changes = 0usize;
    let mut completed_steps = 0usize;
    for step in 0..steps {
        if let Some(telemetry) = dashboard.as_mut() {
            loop {
                let control = telemetry.control()?;
                if control.stop {
                    cancelled.store(true, Ordering::Relaxed);
                }
                if cancelled.load(Ordering::Relaxed) {
                    break;
                }
                if control.freeze && model.plasticity().learning {
                    model.freeze()?;
                    telemetry.settings(&model);
                }
                if !control.paused {
                    break;
                }
                keyboard.release_all()?;
                telemetry.status("paused")?;
                thread::sleep(Duration::from_millis(100));
            }
        }
        if cancelled.load(Ordering::Relaxed) {
            break;
        }
        let frame = keyboard.capture(&capture_path)?;
        if step == 0 {
            fs::copy(&capture_path, output.join("first.ppm"))?;
        }
        let changed = previous.as_ref().is_some_and(|previous| *previous != frame);
        frame_changes += usize::from(changed);
        let result = model.step(&frame)?;
        keyboard.set_keys(result.keys.iter().copied())?;
        model.record_executed_keys(&result.keys)?;
        core_spikes += result.core_events.len();
        formed += result.formed.len();
        pruned += result.pruned.len();
        raw_feedback += result.feedback_arrivals;
        action_feedback += result.action_copy_arrivals;
        homeostasis_changes += result.homeostasis_changes;
        for (index, id) in model.processing_ids().iter().enumerate() {
            writeln!(
                core_activity,
                "{step},{},{}",
                id.0, result.core_counts[index]
            )?;
        }
        for &(synapse, source, target) in &result.formed {
            writeln!(structure, "{step},formed,{synapse},{source},{target}")?;
        }
        for &synapse in &result.pruned {
            writeln!(structure, "{step},pruned,{synapse},,")?;
        }
        sensory_spikes += result.sensory_spikes;
        motor_spikes += result.motor_spikes;
        weight_changes += result.weight_changes;
        key_frames += usize::from(!result.keys.is_empty());
        for key in layout.keys() {
            writeln!(
                motor_activity,
                "{},{},{},{}",
                step,
                key.channel(),
                key.qcode(),
                result.motor_counts[*key as usize]
            )?;
        }
        let keys = if result.keys.is_empty() {
            "none".into()
        } else {
            result
                .keys
                .iter()
                .map(|key| key.qcode())
                .collect::<Vec<_>>()
                .join("+")
        };
        writeln!(
            trace,
            "{},{},{},{},{},{},{}",
            step,
            (step as u64 + 1) * WINDOW_US,
            result.sensory_spikes,
            result.motor_spikes,
            result.weight_changes,
            keys,
            changed
        )?;
        if let Some(telemetry) = dashboard.as_mut() {
            telemetry.publish(&model, step, steps, &result, &frame)?;
        }
        previous = Some(frame);
        completed_steps = step + 1;
        // Wall time lets the VM consume keyboard events; simulation time is
        // explicitly advanced and is not locked to QEMU's guest clock.
        thread::sleep(Duration::from_micros(WINDOW_US));
    }
    keyboard.release_all()?;
    let last = keyboard.capture(&capture_path)?;
    if previous.as_ref().is_some_and(|previous| *previous != last) {
        frame_changes += 1;
    }
    fs::copy(&capture_path, output.join("last.ppm"))?;
    let interrupted = cancelled.load(Ordering::Relaxed);
    let report = json!({"steps": completed_steps, "requested_steps": steps,
        "interrupted": interrupted, "seed": seed, "encoding": format!("{mode:?}"),
        "learning_enabled": learning, "sensory_spikes": sensory_spikes,
        "keyboard_layout": layout.name(), "motor_channels": layout.keys().len(),
        "model_kind": format!("{:?}",effective_config.kind),"processing_neurons": model.processing_ids().len(),
        "development_enabled": effective_config.development,"homeostasis_enabled": effective_config.homeostasis,
        "feedback_enabled": effective_config.feedback,"core_spikes": core_spikes,
        "formed": formed,"pruned": pruned,"final_core_synapses": model.core_synapse_count(),
        "raw_motor_feedback_arrivals": raw_feedback,"executed_action_copy_arrivals": action_feedback,
        "homeostasis_changes": homeostasis_changes,
        "initial_structural_drive": effective_config.initial_structural_drive,
        "pruning_min_below_us": effective_config.pruning_min_below_us,
        "motor_decoder": if effective_config.motor_timing {"count_then_first_spike"}else{"count"},
        "keyboard_feedback_verified": command == "smoke", "full_layout_verified": command == "smoke",
        "motor_spikes": motor_spikes, "weight_changes": weight_changes,
        "key_frames": key_frames, "frame_changes": frame_changes,
        "output": output});
    fs::write(
        output.join("report.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    if let Some(telemetry) = dashboard.as_mut() {
        telemetry.status(if interrupted { "stopped" } else { "completed" })?;
    }
    if command == "smoke"
        && !interrupted
        && (sensory_spikes == 0
            || motor_spikes == 0
            || key_frames == 0
            || (effective_config.kind == ModelKind::Direct
                && layout == KeyboardLayout::Wasd
                && frame_changes == 0))
    {
        return Err(
            "neural activity or keyboard output missing; inspect report.json (direct WASD also requires visual changes)"
                .into(),
        );
    }
    println!("{report}");
    if interrupted {
        println!("VISUAL_CONTROL_STOPPED: keys released; partial report saved.");
    } else if command == "smoke" {
        println!("VISUAL_CONTROL_PASS: pixel-only perception and motor keyboard output verified.");
    } else {
        println!(
            "VISUAL_CONTROL_COMPLETED: experiment finished; inspect report.json for activity."
        );
    }
    Ok(())
}

fn pulse(keyboard: &mut Keyboard, key: Key) -> Result<()> {
    keyboard.set_keys([key])?;
    thread::sleep(Duration::from_millis(80));
    keyboard.set_keys([])?;
    thread::sleep(Duration::from_millis(40));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn black_pixels_produce_no_teacher_or_motor_drive() {
        let mut model = Model::new(42, Encoding::Brightness, true).unwrap();
        let black = Frame::new(64, 64, vec![0; 64 * 64 * 3]).unwrap();
        for _ in 0..3 {
            let step = model.step(&black).unwrap();
            assert_eq!(step.sensory_spikes, 0);
            assert_eq!(step.motor_spikes, 0);
            assert_eq!(step.weight_changes, 0);
            assert_eq!(step.action, None);
            assert!(step.keys.is_empty());
            assert_eq!(step.motor_counts.len(), 105);
        }
    }

    #[test]
    fn pixel_driven_network_replays_and_frozen_control_has_no_weight_changes() {
        let frame = Frame::new(64, 64, vec![255; 64 * 64 * 3]).unwrap();
        let mut first = Model::new(7, Encoding::Brightness, true).unwrap();
        let mut replay = Model::new(7, Encoding::Brightness, true).unwrap();
        let mut frozen = Model::new(7, Encoding::Brightness, false).unwrap();
        let mut changes = 0;
        for _ in 0..3 {
            let a = first.step(&frame).unwrap();
            let b = replay.step(&frame).unwrap();
            assert_eq!(
                (a.action, a.sensory_spikes, a.motor_spikes, a.weight_changes),
                (b.action, b.sensory_spikes, b.motor_spikes, b.weight_changes)
            );
            assert!(a.sensory_spikes > 0);
            assert!(a.motor_spikes > 0);
            assert_eq!(a.core_counts.len(), 256);
            assert!(!a.core_events.is_empty());
            assert_eq!(a.keys, b.keys);
            assert_eq!(a.motor_counts, b.motor_counts);
            changes += a.weight_changes;
            assert_eq!(frozen.step(&frame).unwrap().weight_changes, 0);
        }
        assert!(changes > 0);
    }
}
