use nerva::core::{NeuronRole, Polarity};
use nerva_experiments::{
    model::{KeyboardLayout, Model, ModelConfig},
    sensorimotor::{SensorimotorConfig, run_sensorimotor},
};
use nerva_qemu::{capture::Frame, keyboard::Key};
use std::{
    fs,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

fn base() -> ModelConfig {
    ModelConfig {
        keyboard: KeyboardLayout::Ad,
        processing_neurons: 16,
        ..ModelConfig::default()
    }
}

#[test]
fn competition_adds_only_symmetric_inhibitory_paths_and_leaves_the_base_scaffold_intact() {
    let base = Model::from_config(base()).unwrap();
    let competing = Model::from_config(ModelConfig {
        motor_competition: true,
        ..base.config().clone()
    })
    .unwrap();
    assert_eq!(
        competing.network().neuron_count(),
        base.network().neuron_count() + 2
    );
    assert_eq!(
        competing.network().synapse_count(),
        base.network().synapse_count() + 4
    );
    for neuron in base.network().neurons() {
        assert_eq!(competing.network().neuron(neuron.id()), Some(neuron));
    }
    for synapse in base.network().synapses() {
        assert_eq!(competing.network().synapse(synapse.id()), Some(synapse));
    }
    let added: Vec<_> = competing
        .network()
        .synapses()
        .filter(|s| base.network().synapse(s.id()).is_none())
        .collect();
    for s in &added {
        assert!(!s.is_plastic());
        assert!(!competing.processing_ids().contains(&s.pre()));
    }
    assert_eq!(
        added
            .iter()
            .filter(
                |s| competing.network().neuron(s.pre()).unwrap().polarity() == Polarity::Inhibitory
            )
            .count(),
        2
    );
    assert_eq!(
        added
            .iter()
            .filter(
                |s| competing.network().neuron(s.post()).unwrap().role() == Some(NeuronRole::Motor)
            )
            .count(),
        2
    );
    assert!(
        Model::from_config(ModelConfig {
            motor_competition: true,
            ..ModelConfig::default()
        })
        .is_err()
    );
}

#[test]
fn isolated_motor_inputs_map_to_the_right_keys_and_inhibition_reaches_the_other_motor() {
    let black = Frame::new(64, 64, vec![0; 64 * 64 * 3]).unwrap();
    for competition in [false, true] {
        for key in [Key::A, Key::D] {
            let mut model = Model::from_config(ModelConfig {
                feedback: false,
                motor_competition: competition,
                ..base().frozen()
            })
            .unwrap();
            let initial = model.network().clone();
            assert!(model.diagnostic_motor_input(Key::W, 1000, 1.05).is_err());
            assert_eq!(model.network(), &initial);
            assert!(model.diagnostic_motor_input(key, 50_000, 1.05).is_err());
            model.diagnostic_motor_input(key, 1000, 1.05).unwrap();
            let step = model.step(&black).unwrap();
            assert_eq!(step.action, Some(key));
            assert_eq!(step.motor_events.len(), 1);
            assert_eq!(step.motor_events[0].channel, key.channel());
            assert_eq!(step.motor_events[0].at_us, 1002);
            assert!(step.core_events.is_empty());
            assert_eq!(step.weight_changes, 0);
            let opposite = if key == Key::A { Key::D } else { Key::A };
            assert_eq!(
                step.motor_inputs[opposite as usize].negative_arrivals,
                usize::from(competition)
            );
            if competition {
                assert!(step.motor_inputs[opposite as usize].negative_amplitude < 0.0);
            }
        }
    }
    let mut learning = Model::from_config(base()).unwrap();
    assert!(learning.diagnostic_motor_input(Key::A, 1000, 1.05).is_err());
    let mut feedback = Model::from_config(base().frozen()).unwrap();
    assert!(feedback.diagnostic_motor_input(Key::A, 1000, 1.05).is_err());
}

#[test]
fn effect_assay_replays_own_exploration_and_runs_as_a_separate_cli() {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root =
        std::env::temp_dir().join(format!("nerva-sensorimotor-{}-{stamp}", std::process::id()));
    let direct = root.join("direct");
    fs::create_dir_all(&direct).unwrap();
    let config = SensorimotorConfig {
        model: base(),
        seeds: vec![7],
        explore_steps: 16,
        probe_steps: 16,
        jobs: 1,
    };
    let report =
        run_sensorimotor(&config, &direct, &std::sync::atomic::AtomicBool::new(false)).unwrap();
    assert_eq!(report["isolated_motor_mapping_all_passed"], true);
    assert_eq!(report["isolated_cases"], 8);
    assert_eq!(report["controlled_effect_learning_evidence"], false);
    for r in report["results"].as_array().unwrap() {
        assert_eq!(r["pre_existing_scaffold_unchanged"], true);
        if r["variant"] != "inhibitory_competition" {
            assert_eq!(r["decoder_only_common_neural_stream_exact"], true);
        }
        for p in r["effect_probes"].as_array().unwrap() {
            assert_eq!(p["readout"]["parameters_retained"], true);
        }
        assert_eq!(
            r["effect_probes"][11]["readout"]["executed_action_copy_arrivals"],
            0
        );
        assert_eq!(
            r["effect_probes"][8]["readout"]["next_motion_from_core_counts"]["tested"],
            8
        );
    }
    let output = Command::new(env!("CARGO_BIN_EXE_nerva-sensorimotor"))
        .args([
            "--seed",
            "7",
            "--explore-steps",
            "16",
            "--probe-steps",
            "16",
            "--processing",
            "16",
            "--output",
        ])
        .arg(root.join("cli"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    for file in [
        "sensorimotor.json",
        "sensorimotor-summary.md",
        "motor-actuator.csv",
        "motor-windows.csv",
        "motor-spikes.csv",
        "effect-core.csv",
        "effect-streams.csv",
    ] {
        assert_eq!(
            fs::read(direct.join(file)).unwrap(),
            fs::read(root.join("cli").join(file)).unwrap()
        );
    }
    fs::remove_dir_all(root).unwrap();
}
