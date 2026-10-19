use nerva_experiments::{
    agency::{AgencyConfig, run_agency},
    experiment::{RecordedFrame, Recording, run_study},
    model::{KeyboardLayout, ModelConfig, ModelKind},
    retention::run_retention,
};
use nerva_qemu::{
    Result,
    keyboard::{Key, Keyboard},
    vision_encoder::Encoding,
};
use nerva_vm::visual_vm::DemoVm;
use std::{
    env, fs,
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
        eprintln!("nerva-experiments: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let mut args = env::args().skip(1);
    let command = args.next().unwrap_or_else(|| "help".into());
    if matches!(command.as_str(), "help" | "--help" | "-h") {
        println!(
            "Usage:\n  nerva-experiments study [OPTIONS]\n  nerva-experiments study-replay RECORDING_DIRECTORY [OPTIONS]\n  nerva-experiments record [--actions A,D,D,D,A,A,D,A] [OPTIONS]\n  nerva-experiments retention TRAIN_DIRECTORY PROBE_DIRECTORY [OPTIONS]\n  nerva-experiments agency [OPTIONS]\n\nOptions: --steps N (40), --seed N (42), --seeds 7,19,42,\n --encoding brightness|change, --keyboard full|wasd|ad, --motor-decoder timing|count,\n --model recurrent|direct, --processing N (256; agency 32), --frozen,\n --fixed-topology, --no-feedback, --no-homeostasis,\n --initial-drive F (2.5), --pruning-min-us N (300000), --output NEW_DIRECTORY\nRetention: --train-frames N (1000)\nAgency: --train-episodes N (32), --adapt-episodes N (32), --eval-episodes N (18),\n --episode-steps N (6), --jobs N (4), --seeds 0,1,2,3,4,5,6,7,8,9\n\nFor sensorimotor, M3 and serial protocols use Experimente/run.sh.\nThe running VM model lives in Modell/run-demo.sh."
        );
        return Ok(());
    }
    if !matches!(
        command.as_str(),
        "study" | "study-replay" | "record" | "retention" | "agency"
    ) {
        return Err("unknown experiment; use --help".into());
    }
    let recording_source = if matches!(command.as_str(), "study-replay" | "retention") {
        Some(PathBuf::from(
            args.next().ok_or("replay requires a recording directory")?,
        ))
    } else {
        None
    };
    let probe_source = if command == "retention" {
        Some(PathBuf::from(args.next().ok_or(
            "retention requires TRAIN_DIRECTORY PROBE_DIRECTORY",
        )?))
    } else {
        None
    };
    let mut steps = 40usize;
    let mut seed = 42u64;
    let mut mode = Encoding::Brightness;
    let mut learning = true;
    let study = matches!(command.as_str(), "study" | "study-replay");
    let retention = command == "retention";
    let agency = command == "agency";
    let mut layout = if agency {
        KeyboardLayout::Ad
    } else if study || retention {
        KeyboardLayout::Wasd
    } else {
        KeyboardLayout::Full
    };
    let mut kind = ModelKind::Recurrent;
    let mut processing = if agency { 32 } else { 256 };
    let mut development = true;
    let mut feedback = true;
    let mut homeostasis = true;
    let mut seeds = if agency {
        (0..10).collect()
    } else {
        vec![7u64, 19, 42]
    };
    let mut train_episodes = 32;
    let mut adapt_episodes = 32;
    let mut eval_episodes = 18;
    let mut episode_steps = 6;
    let mut jobs = 4;
    let mut output = None;
    let mut initial_structural_drive = 2.5;
    let mut motor_timing = true;
    let mut pruning_min_below_us = 300_000;
    let mut train_frames = 1000;
    let mut actions = vec![
        Key::A,
        Key::D,
        Key::D,
        Key::D,
        Key::A,
        Key::A,
        Key::D,
        Key::A,
    ];
    while let Some(option) = args.next() {
        match option.as_str() {
            "--steps" => steps = args.next().ok_or("missing step count")?.parse()?,
            "--seed" => seed = args.next().ok_or("missing seed")?.parse()?,
            "--encoding" => {
                mode = match args.next().as_deref() {
                    Some("brightness") => Encoding::Brightness,
                    Some("change") => Encoding::Change,
                    _ => return Err("encoding must be brightness or change".into()),
                }
            }
            "--frozen" => learning = false,
            "--motor-decoder" => {
                motor_timing = match args.next().as_deref() {
                    Some("timing") => true,
                    Some("count") => false,
                    _ => return Err("motor decoder must be timing or count".into()),
                }
            }
            "--processing" => {
                processing = args.next().ok_or("missing processing count")?.parse()?
            }
            "--model" => {
                kind = match args.next().as_deref() {
                    Some("recurrent") => ModelKind::Recurrent,
                    Some("direct") => ModelKind::Direct,
                    _ => return Err("model must be recurrent or direct".into()),
                }
            }
            "--fixed-topology" => development = false,
            "--no-feedback" => feedback = false,
            "--no-homeostasis" => homeostasis = false,
            "--initial-drive" => {
                initial_structural_drive = args.next().ok_or("missing initial drive")?.parse()?
            }
            "--pruning-min-us" => {
                pruning_min_below_us = args.next().ok_or("missing pruning duration")?.parse()?
            }
            "--train-frames" if retention => {
                train_frames = args.next().ok_or("missing training frame count")?.parse()?
            }
            "--actions" if command == "record" => {
                actions = args
                    .next()
                    .ok_or("missing A/D sequence")?
                    .split(',')
                    .map(|key| match key {
                        "A" | "a" => Ok(Key::A),
                        "D" | "d" => Ok(Key::D),
                        _ => Err("actions must be a comma-separated A/D sequence"),
                    })
                    .collect::<std::result::Result<Vec<_>, _>>()?;
            }
            "--train-episodes" if agency => {
                train_episodes = args
                    .next()
                    .ok_or("missing training episode count")?
                    .parse()?
            }
            "--adapt-episodes" if agency => {
                adapt_episodes = args
                    .next()
                    .ok_or("missing adaptation episode count")?
                    .parse()?
            }
            "--eval-episodes" if agency => {
                eval_episodes = args
                    .next()
                    .ok_or("missing evaluation episode count")?
                    .parse()?
            }
            "--episode-steps" if agency => {
                episode_steps = args.next().ok_or("missing episode step limit")?.parse()?
            }
            "--jobs" if agency => jobs = args.next().ok_or("missing job count")?.parse()?,
            "--seeds" if study || retention || agency => {
                seeds = args
                    .next()
                    .ok_or("missing study seeds")?
                    .split(',')
                    .map(str::parse)
                    .collect::<std::result::Result<Vec<_>, _>>()?
            }
            "--keyboard" => {
                layout = match args.next().as_deref() {
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
    let mut model_config = ModelConfig {
        seed,
        encoding: mode,
        keyboard: layout,
        kind,
        processing_neurons: processing,
        learning,
        development,
        homeostasis,
        feedback,
        initial_structural_drive,
        pruning_min_below_us,
        motor_competition: false,
        motor_inhibition_weight: 2.0,
        motor_weight_scale: 1.0,
        motor_refractory_us: 10_000,
        motor_timing,
        ..ModelConfig::default()
    };
    if !learning {
        model_config = model_config.frozen();
    }
    model_config.validate()?;
    let agency_config = if agency {
        let config = AgencyConfig {
            model: model_config.clone(),
            seeds: seeds.clone(),
            train_episodes,
            adapt_episodes,
            eval_episodes,
            episode_steps,
            jobs,
        };
        config.validate()?;
        Some(config)
    } else {
        None
    };
    if study
        && (kind != ModelKind::Recurrent
            || layout != KeyboardLayout::Wasd
            || !learning
            || !feedback
            || !homeostasis
            || !development)
    {
        return Err("study uses recurrent WASD with STDP, homeostasis and feedback; it compares enabled and fixed topology".into());
    }
    if study && (!(8..=200).contains(&steps) || !steps.is_multiple_of(4)) {
        return Err("study steps must be 8..=200 and a multiple of four".into());
    }
    if command == "record" && steps < 8 {
        return Err("record needs at least eight frames".into());
    }
    let cancelled = Arc::new(AtomicBool::new(false));
    let interrupt = Arc::clone(&cancelled);
    ctrlc::set_handler(move || interrupt.store(true, Ordering::Relaxed))?;
    let output = output.unwrap_or_else(|| {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../Messungen/Experimente")
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
    fs::create_dir(&output)?; // Never overwrite an earlier experiment.
    if let Some(config) = agency_config {
        let report = run_agency(&config, &output, &cancelled)?;
        println!(
            "AGENCY_COMPLETED: feasibility criteria met: {}. Output: {}",
            report["criteria"]["feasibility_demonstrated"],
            output.display()
        );
        return Ok(());
    }
    if let Some(recording_source) = recording_source {
        let recording = Recording::load(&recording_source)?;
        if let Some(probe_source) = probe_source {
            let probes = Recording::load(&probe_source)?;
            let report = run_retention(
                &recording,
                &probes,
                &model_config,
                train_frames,
                &seeds,
                &output,
                &cancelled,
            )?;
            println!(
                "RETENTION_PASS: learned parameters retained: {}. Output: {}",
                report["all_parameters_retained"],
                output.display()
            );
            return Ok(());
        }
        let report = run_study(&recording, &model_config, &seeds, &output, &cancelled)?;
        println!(
            "STUDY_REPLAY_PASS: {} paired seeds, exact core replay verified. Output: {}",
            seeds.len(),
            output.display()
        );
        println!(
            "Core activity differs: {}",
            report["any_core_activity_difference"]
        );
        return Ok(());
    }
    let (vm, qmp) = DemoVm::launch(&cancelled)?;
    let mut keyboard = Keyboard::new(qmp);
    let capture_path = output.join("current.ppm");
    {
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
    if command == "study" || command == "record" {
        let frame_directory = output.join("frames");
        fs::create_dir(&frame_directory)?;
        let mut recording = Recording {
            frames: Vec::with_capacity(steps),
        };
        for index in 0..steps {
            if cancelled.load(Ordering::Relaxed) {
                return Err("interrupted during A/D recording".into());
            }
            let key = if command == "study" {
                [Key::A, Key::A, Key::D, Key::D][index % 4]
            } else {
                actions[index % actions.len()]
            };
            pulse(&mut keyboard, key)?;
            let frame = keyboard.capture(&frame_directory.join(format!("{index:06}.ppm")))?;
            recording.frames.push(RecordedFrame {
                frame,
                executed_key: key,
            });
        }
        if command == "record" {
            recording.write_sequence_manifest(&output)?;
        } else {
            recording.write_manifest(&output)?;
        }
        keyboard.release_all()?;
        drop(keyboard);
        drop(vm);
        if command == "record" {
            println!(
                "RECORDING_PASS: {} A/D frames saved to {}",
                steps,
                output.display()
            );
            return Ok(());
        }
        println!(
            "Recorded {} A/D frames. Comparing {} paired seeds with exact replays...",
            steps,
            seeds.len()
        );
        let report = run_study(&recording, &model_config, &seeds, &output, &cancelled)?;
        println!(
            "STUDY_PASS: exact replay verified. Core activity differs: {}. Output: {}",
            report["any_core_activity_difference"],
            output.display()
        );
        return Ok(());
    }
    Err("experiment completed without a protocol".into())
}

fn pulse(keyboard: &mut Keyboard, key: Key) -> Result<()> {
    keyboard.set_keys([key])?;
    thread::sleep(Duration::from_millis(80));
    keyboard.set_keys([])?;
    thread::sleep(Duration::from_millis(40));
    Ok(())
}
