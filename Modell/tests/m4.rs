use nerva_qemu::keyboard::Key;
use nerva_vm::m4::{config::M4Config, world::DelayedWorld};
use serde_json::{Value, json};
use std::{
    fs,
    path::Path,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};
fn fixture() -> Value {
    json!({"protocol":"NERVA-M4-v1","case_id":"a01_minimal_2ch","seed":7,"params":{"sensor":"effect_2ch","resolution":0,"encoding":"deterministic","processing":16,"action_effect_delay_ms":8,"motor_feedback":true,"stdp":true,"homeostasis":true,"development":false,"initial_drive":0,"pruning_min_us":5000000,"normalization":"equal_input_current"},"arms":["before","aged_frozen","paired","scrambled"],"train_trials":8,"test_trials":8,"calibration_trials":8})
}
#[test]
fn config_contract_is_strict_and_unknown_inputs_never_fall_back() {
    M4Config::parse(fixture()).unwrap();
    for (field, value) in [
        ("sensor", json!("unknown")),
        ("encoding", json!("poission")),
        ("resolution", json!(32)),
        ("normalization", json!("none")),
        ("action_effect_delay_ms", json!(9)),
        ("stdp", json!(1)),
    ] {
        let mut cfg = fixture();
        cfg["params"][field] = value;
        assert!(M4Config::parse(cfg).is_err(), "{field}");
    }
    let mut cfg = fixture();
    cfg["params"]["future_frame"] = json!("forbidden");
    assert!(M4Config::parse(cfg).is_err());
    let mut cfg = fixture();
    cfg["correct_action"] = json!("A");
    assert!(M4Config::parse(cfg).is_err());
    let mut cfg = fixture();
    cfg["params"]["sensor"] = json!("pixels");
    assert!(M4Config::parse(cfg).is_err());
    let mut cfg = fixture();
    cfg["arms"] = json!(["paired"]);
    assert!(M4Config::parse(cfg).is_err());
}
#[test]
fn world_moves_only_at_the_real_scheduled_time_in_both_mappings() {
    for reversed in [false, true] {
        for delay in [2_000, 8_000, 20_000, 50_000] {
            let mut world = DelayedWorld::new(reversed).unwrap();
            let start = world.observe_native().unwrap();
            world.execute(Key::A, 100, delay).unwrap();
            world.advance(100 + delay - 1).unwrap();
            assert_eq!(world.observe_native().unwrap(), start);
            assert_eq!(world.motion_us, None);
            world.advance(100 + delay).unwrap();
            assert_ne!(world.observe_native().unwrap(), start);
            assert_eq!(world.motion_us, Some(100 + delay));
        }
    }
}
fn execute(config: &Path, output: &Path) {
    let result = Command::new(env!("CARGO_BIN_EXE_nerva-m4"))
        .args(["--config"])
        .arg(config)
        .arg("--output")
        .arg(output)
        .arg("--raw-events")
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
}
#[test]
fn serial_and_parallel_m4_runs_are_identical_and_csv_audits_verify_causality_and_ablations() {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("nerva-m4-test-{}-{stamp}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let config = root.join("config.json");
    fs::write(&config, serde_json::to_vec(&fixture()).unwrap()).unwrap();
    let first = root.join("first.json");
    execute(&config, &first);
    std::thread::scope(|scope| {
        let mut threads = Vec::new();
        for name in ["parallel1.json", "parallel2.json"] {
            let output = root.join(name);
            let cfg = &config;
            threads.push(scope.spawn(move || execute(cfg, &output)));
        }
        for t in threads {
            t.join().unwrap();
        }
    });
    for name in ["parallel1.json", "parallel2.json"] {
        for suffix in ["", ".details.csv", ".audit.csv", ".events.csv"] {
            assert_eq!(
                fs::read(format!("{}{suffix}", first.display())).unwrap(),
                fs::read(format!("{}{suffix}", root.join(name).display())).unwrap()
            );
        }
    }
    let report: Value = serde_json::from_slice(&fs::read(&first).unwrap()).unwrap();
    for (_, check) in report["checks"].as_object().unwrap() {
        assert_eq!(check, true);
    }
    for arm in ["before", "aged_frozen", "paired", "scrambled"] {
        let result = &report["arms"][arm];
        assert_eq!(result["test_count"], 16);
        for mapping in ["normal_mapping", "reversed_mapping"] {
            assert_eq!(result[mapping]["parameters_preserved"], true);
            assert_eq!(
                result[mapping]["probe"]["pixel_baseline"]["balanced_accuracy"],
                0.5
            );
            assert_eq!(result[mapping]["probe"]["count"]["classes"], 2);
            if arm == "scrambled" {
                assert_eq!(
                    result[mapping]["training_contingency_a_left_a_right_d_left_d_right"],
                    json!([2, 2, 2, 2])
                );
            }
        }
        if arm == "before" || arm == "aged_frozen" {
            assert_eq!(result["weight_updates"], 0);
        }
        assert_eq!(result["formed"], 0);
        assert_eq!(result["pruned"], 0);
    }
    let details = fs::read_to_string(format!("{}.details.csv", first.display())).unwrap();
    let mut start_hash = None;
    for line in details.lines().skip(1) {
        let columns: Vec<_> = line.split(',').collect();
        assert_eq!(columns.len(), 20);
        let action = columns[5].parse::<u64>().unwrap();
        let motion = columns[6].parse::<u64>().unwrap();
        let forecast = columns[7].parse::<u64>().unwrap();
        let image = columns[8].parse::<u64>().unwrap();
        assert!(action < forecast && forecast < motion && motion == image);
        assert_eq!(motion - action, 8000);
        if columns[2] != "training" {
            assert_eq!(columns[9], columns[10]);
        }
        assert_eq!(*start_hash.get_or_insert(columns[18]), columns[18]);
    }
    let mut invalid = fixture();
    invalid["params"]["encoding"] = json!("poission");
    let bad_config = root.join("bad-config.json");
    let bad_output = root.join("bad-result.json");
    fs::write(&bad_config, serde_json::to_vec(&invalid).unwrap()).unwrap();
    let invalid_run = Command::new(env!("CARGO_BIN_EXE_nerva-m4"))
        .arg("--config")
        .arg(&bad_config)
        .arg("--output")
        .arg(&bad_output)
        .output()
        .unwrap();
    assert!(!invalid_run.status.success());
    assert!(String::from_utf8_lossy(&invalid_run.stderr).contains("unknown encoding"));
    assert!(!bad_output.exists());
    let mut ablated = fixture();
    ablated["params"]["stdp"] = json!(false);
    ablated["params"]["motor_feedback"] = json!(false);
    fs::write(&config, serde_json::to_vec(&ablated).unwrap()).unwrap();
    let out = root.join("ablated.json");
    execute(&config, &out);
    let report: Value = serde_json::from_slice(&fs::read(&out).unwrap()).unwrap();
    for (_, arm) in report["arms"].as_object().unwrap() {
        assert_eq!(arm["weight_updates"], 0);
        for mapping in ["normal_mapping", "reversed_mapping"] {
            for (_, stage) in arm[mapping]["stage_diagnostics"].as_object().unwrap() {
                assert_eq!(stage["action_copy_arrivals"], 0);
                assert_eq!(stage["motor_feedback_arrivals"], 0);
            }
        }
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn isolated_stdp_signal_and_feedback_benches_verify_the_existing_rules() {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "nerva-m4-mechanisms-{}-{stamp}",
        std::process::id()
    ));
    fs::create_dir(&root).unwrap();
    let output = root.join("mechanisms.json");
    let report = nerva_vm::m4::mechanisms::run(&output).unwrap();
    assert_eq!(report["stdp_theory_all_passed"], true);
    assert_eq!(report["actuator_all_passed"], true);
    assert_eq!(report["stdp_pairs"].as_array().unwrap().len(), 9);
    for item in report["stdp_pairs"].as_array().unwrap() {
        assert!(item["absolute_error"].as_f64().unwrap() <= 1e-7);
    }
    let feedback = report["feedback_interventions"].as_array().unwrap();
    for seed in 0..3 {
        let find = |key: &str, intervention: &str| {
            feedback
                .iter()
                .find(|r| {
                    r["seed"] == seed
                        && r["executed_action"] == key
                        && r["intervention"] == intervention
                })
                .unwrap()
        };
        assert_ne!(
            find("a", "correct")["core_potentials_at_20ms"],
            find("d", "correct")["core_potentials_at_20ms"]
        );
        assert_eq!(
            find("a", "correct")["core_potentials_at_20ms"],
            find("d", "swapped")["core_potentials_at_20ms"]
        );
        assert_eq!(find("a", "correct")["first_copy_arrival_us"], 1501);
        assert_eq!(find("a", "delayed")["first_copy_arrival_us"], 5501);
        assert_eq!(find("a", "none")["copy_arrivals"], 0);
        assert_eq!(find("a", "correct")["core_spike_count"], 0);
    }
    fs::remove_dir_all(root).unwrap();
}
