use nerva_experiments::{
    experiment::{RecordedFrame, Recording},
    model::{KeyboardLayout, Model, ModelConfig},
    retention::{
        ActionControl, learned_parameters_equal, prediction_readout, probe, run_retention,
    },
};
use nerva_qemu::{capture::Frame, keyboard::Key};
use std::{
    fs,
    sync::atomic::AtomicBool,
    time::{SystemTime, UNIX_EPOCH},
};

fn config() -> ModelConfig {
    ModelConfig {
        processing_neurons: 16,
        keyboard: KeyboardLayout::Wasd,
        ..ModelConfig::default()
    }
}

fn recording(actions: &[Key], count: usize) -> Recording {
    Recording {
        frames: (0..count)
            .map(|index| {
                let mut rgb = vec![0; 64 * 64 * 3];
                // Small pixel-only stimulus; no guest-state metadata.
                for y in 24..32 {
                    for x in 8 * (index % 4)..8 * (index % 4) + 8 {
                        rgb[(y * 64 + x) * 3..(y * 64 + x) * 3 + 3].fill(255);
                    }
                }
                RecordedFrame {
                    frame: Frame::new(64, 64, rgb).unwrap(),
                    executed_key: actions[index % actions.len()],
                }
            })
            .collect(),
    }
}

#[test]
fn trained_instance_freezes_without_reset_and_keeps_all_learned_parameters() {
    let mut model = Model::from_config(config()).unwrap();
    let initial = model.network().clone();
    let white = Frame::new(64, 64, vec![255; 64 * 64 * 3]).unwrap();
    let mut changes = 0;
    let mut formations = 0;
    for _ in 0..12 {
        model.record_executed_keys(&[Key::D]).unwrap();
        let step = model.step(&white).unwrap();
        changes += step.weight_changes;
        formations += step.formed.len();
    }
    assert!(changes > 0 && formations > 0);
    assert!(!learned_parameters_equal(&initial, model.network()));
    // This queued action must survive the transition, together with neural state.
    model.record_executed_keys(&[Key::A]).unwrap();
    let checkpoint = model.network().clone();
    model.freeze().unwrap();
    model.freeze().unwrap();
    assert_eq!(model.network(), &checkpoint);
    assert!(!model.config().learning && !model.config().homeostasis && !model.config().development);
    let step = model.step(&white).unwrap();
    assert!(step.action_copy_arrivals > 0);
    assert!(step.core_events.iter().all(|&(_, time)| time > 12 * 50_000));
    assert!(learned_parameters_equal(&checkpoint, model.network()));
    let data = recording(&[Key::D, Key::A, Key::D], 16);
    let steps = probe(
        &mut model,
        &data,
        ActionControl::Aligned,
        &AtomicBool::new(false),
    )
    .unwrap();
    assert!(steps.iter().any(|step| !step.core_events.is_empty()));
    assert!(learned_parameters_equal(&checkpoint, model.network()));
}

#[test]
fn action_controls_preserve_pixels_and_change_only_executed_copies() {
    let data = recording(&[Key::A, Key::D, Key::D, Key::A], 16);
    assert_eq!(ActionControl::Swapped.key(&data, 0), Some(Key::D));
    assert_eq!(ActionControl::Delayed.key(&data, 0), None);
    assert_eq!(ActionControl::Delayed.key(&data, 1), Some(Key::A));
    let stop = AtomicBool::new(false);
    let mut runs = Vec::new();
    for control in &ActionControl::ALL[..4] {
        let mut model = Model::from_config(config().frozen()).unwrap();
        runs.push(probe(&mut model, &data, *control, &stop).unwrap());
    }
    for run in &runs[1..] {
        assert_eq!(
            runs[0].iter().map(|s| s.sensory_spikes).collect::<Vec<_>>(),
            run.iter().map(|s| s.sensory_spikes).collect::<Vec<_>>()
        );
    }
    assert_eq!(runs[2][0].action_copy_arrivals, 0);
    assert!(runs[3].iter().all(|s| s.action_copy_arrivals == 0));
    assert!(runs[0].iter().all(|s| s.action_copy_arrivals == 3));
    let mut active = Model::from_config(config()).unwrap();
    assert!(probe(&mut active, &data, ActionControl::Aligned, &stop).is_err());
}

#[test]
fn startup_growth_and_early_pruning_can_be_disabled_independently() {
    let mut no_start = config();
    no_start.initial_structural_drive = 0.0;
    no_start.homeostasis = false;
    no_start.pruning_min_below_us = 5_000_000;
    let mut delayed = Model::from_config(no_start.clone()).unwrap();
    let mut early_config = no_start;
    early_config.pruning_min_below_us = 300_000;
    let mut early = Model::from_config(early_config).unwrap();
    assert_eq!(delayed.network(), early.network());
    let black = Frame::new(64, 64, vec![0; 64 * 64 * 3]).unwrap();
    let white = Frame::new(64, 64, vec![255; 64 * 64 * 3]).unwrap();
    let mut pruned = 0;
    for index in 0..60 {
        // First collect utility evidence, then let it decay below the threshold.
        // Action events keep the event-driven runtime clock advancing in darkness.
        delayed.record_executed_keys(&[Key::D]).unwrap();
        early.record_executed_keys(&[Key::D]).unwrap();
        let frame = if index == 0 { &white } else { &black };
        let step = delayed.step(frame).unwrap();
        assert!(step.formed.is_empty() && step.pruned.is_empty());
        let step = early.step(frame).unwrap();
        assert!(step.formed.is_empty());
        pruned += step.pruned.len();
    }
    assert!(pruned > 0);
    for invalid in [f32::NAN, f32::INFINITY, -1.0, 3.1] {
        assert!(
            Model::from_config(ModelConfig {
                initial_structural_drive: invalid,
                ..config()
            })
            .is_err()
        );
    }
    assert!(
        Model::from_config(ModelConfig {
            pruning_min_below_us: 0,
            ..config()
        })
        .is_err()
    );
}

#[test]
fn retention_report_is_reproducible_and_uses_real_trained_checkpoints() {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory =
        std::env::temp_dir().join(format!("nerva-retention-{}-{stamp}", std::process::id()));
    let first = directory.join("first");
    let second = directory.join("second");
    fs::create_dir_all(&first).unwrap();
    fs::create_dir(&second).unwrap();
    let train = recording(&[Key::A, Key::A, Key::D, Key::D], 8);
    let test = recording(
        &[
            Key::A,
            Key::D,
            Key::D,
            Key::D,
            Key::A,
            Key::A,
            Key::D,
            Key::A,
        ],
        16,
    );
    let stop = AtomicBool::new(false);
    let a = run_retention(&train, &test, &config(), 12, &[7], &first, &stop).unwrap();
    let b = run_retention(&train, &test, &config(), 12, &[7], &second, &stop).unwrap();
    assert_eq!(a, b);
    assert_eq!(a["all_parameters_retained"], true);
    assert_eq!(a["unique_training_frames"], 8);
    assert_eq!(a["training_frames"], 12);
    assert_eq!(a["results"].as_array().unwrap().len(), 4);
    assert_eq!(a["results"][0]["training"]["frames"], 0);
    assert_eq!(a["results"][1]["training"]["weight_changes"], 0);
    assert_eq!(a["results"][2]["training"]["formed"], 0);
    assert!(a["results"][3]["training"]["formed"].as_u64().unwrap() > 0);
    for result in a["results"].as_array().unwrap() {
        assert_eq!(result["identical_prefix_checkpoints"], true);
        assert_eq!(result["freeze_preserved_full_network"], true);
        for control in result["controls"].as_array().unwrap() {
            assert_eq!(control["parameters_retained"], true);
        }
    }
    for name in [
        "retention.json",
        "retention-core.csv",
        "retention-topology.csv",
        "retention-neurons.csv",
        "retention-training.csv",
        "retention-spikes.csv",
        "retention-summary.md",
    ] {
        assert_eq!(
            fs::read(first.join(name)).unwrap(),
            fs::read(second.join(name)).unwrap()
        );
    }
    assert!(run_retention(&train, &train, &config(), 12, &[7], &first, &stop).is_err());
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn general_recordings_roundtrip_without_weakening_the_original_study_protocol() {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory =
        std::env::temp_dir().join(format!("nerva-sequence-{}-{stamp}", std::process::id()));
    fs::create_dir_all(directory.join("frames")).unwrap();
    let data = recording(&[Key::D, Key::D, Key::D, Key::A], 16);
    assert!(data.validate().is_err());
    for (index, frame) in data.frames.iter().enumerate() {
        let mut bytes = b"P6\n64 64\n255\n".to_vec();
        bytes.extend_from_slice(frame.frame.rgb());
        fs::write(directory.join(format!("frames/{index:06}.ppm")), bytes).unwrap();
    }
    data.write_sequence_manifest(&directory).unwrap();
    assert_eq!(data, Recording::load(&directory).unwrap());
    let mut model = Model::from_config(config().frozen()).unwrap();
    let steps = probe(
        &mut model,
        &data,
        ActionControl::Aligned,
        &AtomicBool::new(false),
    )
    .unwrap();
    let readout = prediction_readout(&data, &steps).unwrap();
    assert_eq!(readout["calibration_frames"], 7);
    assert_eq!(readout["test_frames"], 8);
    assert!(prediction_readout(&data, &steps[..8]).is_err());
    fs::remove_dir_all(directory).unwrap();
}
