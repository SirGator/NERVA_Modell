use nerva::core::NeuronRole;
use nerva_experiments::{
    m3::schedule,
    model::{KeyboardLayout, Model, ModelConfig},
};
use nerva_qemu::{capture::Frame, keyboard::Key};
use std::{
    fs,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn destroyed_pairing_preserves_the_exact_action_stream_and_both_marginals() {
    for seed in 0..10 {
        let paired = schedule(seed, 128, false, false);
        let scrambled = schedule(seed, 128, false, true);
        let reversed = schedule(seed, 128, true, false);
        for ((a, b), c) in paired.iter().zip(&scrambled).zip(&reversed) {
            assert_eq!(a.key, b.key);
            assert_eq!(a.key, c.key);
            assert_eq!(a.shown_motion, -c.shown_motion);
        }
        for block in scrambled.as_chunks::<4>().0 {
            for key in [Key::A, Key::D] {
                for label in [-1, 1] {
                    assert_eq!(
                        block
                            .iter()
                            .filter(|t| t.key == key && t.shown_motion == label)
                            .count(),
                        1
                    );
                }
            }
        }
    }
}

#[test]
fn motor_scan_scales_only_core_motor_edges_and_substeps_keep_pending_action_events() {
    let config = ModelConfig {
        keyboard: KeyboardLayout::Ad,
        processing_neurons: 16,
        ..ModelConfig::default().frozen()
    };
    let original = Model::from_config(config.clone()).unwrap();
    let scaled = Model::from_config(ModelConfig {
        motor_weight_scale: 0.5,
        ..config.clone()
    })
    .unwrap();
    for neuron in original.network().neurons() {
        assert_eq!(scaled.network().neuron(neuron.id()), Some(neuron));
    }
    for edge in original.network().synapses() {
        let other = scaled.network().synapse(edge.id()).unwrap();
        if original.network().neuron(edge.pre()).unwrap().role() == Some(NeuronRole::Processing)
            && original.network().neuron(edge.post()).unwrap().role() == Some(NeuronRole::Motor)
        {
            assert_eq!(other.weight().get(), edge.weight().get() * 0.5);
        } else {
            assert_eq!(edge, other);
        }
    }
    let mut model = Model::from_config(config).unwrap();
    let black = Frame::new(64, 64, vec![0; 64 * 64 * 3]).unwrap();
    let initial = model.network().clone();
    assert!(model.step_for(&black, 0).is_err());
    assert!(model.step_for(&black, 50_001).is_err());
    assert_eq!(model.network(), &initial);
    model.record_executed_keys(&[Key::A]).unwrap();
    let first = model.step_for(&black, 500).unwrap();
    let second = model.step_for(&black, 7500).unwrap();
    assert_eq!(first.start_us, 1);
    assert_eq!(second.start_us, first.end_us + 1);
    assert_eq!(second.end_us, 8000);
    assert_eq!(first.action_copy_arrivals + second.action_copy_arrivals, 3);
}

#[test]
fn m3_cli_replays_and_keeps_future_images_out_of_action_time_features() {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("nerva-m3-{}-{stamp}", std::process::id()));
    fs::create_dir(&root).unwrap();
    for name in ["first", "replay"] {
        let output = Command::new(env!("CARGO_BIN_EXE_nerva-m3"))
            .args([
                "--seed",
                "7",
                "--train-trials",
                "8",
                "--probe-trials",
                "8",
                "--calibration-trials",
                "8",
                "--retention-windows",
                "2",
                "--scan-repeats",
                "4",
                "--processing",
                "16",
                "--output",
            ])
            .arg(root.join(name))
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    for file in [
        "m3.json",
        "m3-summary.md",
        "m3-trials.csv",
        "m3-spikes.csv",
        "motor-scan.csv",
    ] {
        assert_eq!(
            fs::read(root.join("first").join(file)).unwrap(),
            fs::read(root.join("replay").join(file)).unwrap()
        );
    }
    let report: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("first/m3.json")).unwrap()).unwrap();
    assert_eq!(report["m3_criteria_met"], false); // one seed cannot establish M3
    for result in report["results"].as_array().unwrap() {
        assert_eq!(result["same_initial_network"], true);
        assert_eq!(result["parameters_retained"], true);
        if result["state"] == "aged_frozen" {
            assert_eq!(result["training_weight_changes"], 0);
            assert_eq!(result["formed"], 0);
            assert_eq!(result["pruned"], 0);
        }
        if result["state"] == "scrambled" {
            assert_eq!(
                result["training_contingency_a_left_a_right_d_left_d_right"],
                serde_json::json!([2, 2, 2, 2])
            );
        }
    }
    let csv = fs::read_to_string(root.join("first/m3-trials.csv")).unwrap();
    let mut fingerprint = None;
    for line in csv.lines().skip(1) {
        let fields: Vec<_> = line.split(',').collect();
        assert!(fields[10].parse::<u64>().unwrap() < fields[11].parse::<u64>().unwrap());
        assert_eq!(fields[13].parse::<usize>().unwrap(), 3);
        if fields[4] != "training" {
            assert_eq!(fields[7], fields[8]);
        }
        assert_eq!(*fingerprint.get_or_insert(fields[18]), fields[18]);
    }
    fs::remove_dir_all(root).unwrap();
}
