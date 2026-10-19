use nerva_qemu::keyboard::Key;
use nerva_vm::{
    m6,
    model::{KeyboardLayout, Model, ModelConfig, SensorConfig, SensorMode, SpikeCoding},
};
use serde_json::{Value, json};
use std::{
    fs,
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};
fn fixture(variant: &str) -> Value {
    json!({"protocol":"NERVA-M6-v1","variant":variant,
    "base":{"protocol":"NERVA-M4-v1","case_id":"m6_test","seed":9,
    "params":{"sensor":"effect_2ch","resolution":0,"encoding":"deterministic","processing":32,
    "action_effect_delay_ms":50,"motor_feedback":true,"stdp":true,"homeostasis":true,"development":false,
    "initial_drive":0,"pruning_min_us":5000000,"normalization":"equal_input_current"},
    "arms":["before","aged_frozen","paired","scrambled"],"train_trials":8,"test_trials":8,"calibration_trials":8}})
}
#[test]
fn m6_contract_rejects_unsupported_variants_timing_and_future_fields() {
    for (key, value) in [
        ("variant", json!("E")),
        ("future_label", json!(1)),
        ("protocol", json!("NERVA-M5-v1")),
    ] {
        let mut config = fixture("D");
        config[key] = value;
        assert!(m6::Config::parse(config).is_err());
    }
    let mut config = fixture("D");
    config["base"]["params"]["action_effect_delay_ms"] = json!(20);
    assert!(m6::Config::parse(config).is_err());
    let mut off = fixture("D");
    off["base"]["params"]["homeostasis"] = json!(false);
    assert!(m6::Config::parse(off.clone()).is_ok());
    off["variant"] = json!("A");
    assert!(m6::Config::parse(off).is_err());
}
#[test]
fn replay_comparison_detects_queued_inputs_even_when_networks_are_identical() {
    let cfg = ModelConfig {
        keyboard: KeyboardLayout::Ad,
        processing_neurons: 32,
        feedback: false,
        ..ModelConfig::default().frozen()
    };
    let sensor = SensorConfig {
        mode: SensorMode::Effect2Ch,
        coding: SpikeCoding::Deterministic,
        ..SensorConfig::default()
    };
    let mut a = Model::with_sensor(cfg.clone(), sensor).unwrap();
    let mut b = Model::with_sensor(cfg, sensor).unwrap();
    assert!(a.same_replay_state(&b));
    a.diagnostic_motor_input(Key::A, 1000, 1.05).unwrap();
    assert_eq!(a.network(), b.network());
    assert!(!a.same_replay_state(&b));
    b.diagnostic_motor_input(Key::A, 1000, 1.05).unwrap();
    assert!(a.same_replay_state(&b));
    let frame = nerva_vm::m4::world::image(3).unwrap();
    assert_eq!(
        a.step_for(&frame, 3000).unwrap(),
        b.step_for(&frame, 3000).unwrap()
    );
    assert!(a.same_replay_state(&b));
}
#[test]
fn immediate_calibration_forks_and_parallel_cli_runs_preserve_exact_replay_and_causality() {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("m6-{}-{stamp}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let cfg = root.join("config.json");
    fs::write(&cfg, serde_json::to_vec(&fixture("D")).unwrap()).unwrap();
    let spawn = |output: PathBuf| {
        Command::new(env!("CARGO_BIN_EXE_nerva-m6"))
            .arg("--config")
            .arg(&cfg)
            .arg("--output")
            .arg(output)
            .spawn()
            .unwrap()
    };
    assert!(spawn(root.join("serial.json")).wait().unwrap().success());
    let mut a = spawn(root.join("parallel-a.json"));
    let mut b = spawn(root.join("parallel-b.json"));
    assert!(a.wait().unwrap().success() && b.wait().unwrap().success());
    for suffix in [
        "",
        ".audit.csv",
        ".details.csv",
        ".sensor.csv",
        ".copies.csv",
        ".branches.csv",
    ] {
        let bytes = fs::read(root.join(format!("serial.json{suffix}"))).unwrap();
        for name in ["parallel-a", "parallel-b"] {
            assert_eq!(
                bytes,
                fs::read(root.join(format!("{name}.json{suffix}"))).unwrap()
            );
        }
    }
    let report: Value =
        serde_json::from_slice(&fs::read(root.join("serial.json")).unwrap()).unwrap();
    assert!(
        report["checks"]
            .as_object()
            .unwrap()
            .values()
            .all(|x| x == true)
    );
    for arm in ["before", "aged_frozen", "paired", "scrambled"] {
        for mapping in ["normal_mapping", "reversed_mapping"] {
            let m = &report["arms"][arm][mapping];
            assert_eq!(m["checkpoint_us"], m["immediate_probe_start_us"]);
            assert!(
                m["calibration_end_on_fork_us"].as_u64().unwrap()
                    > m["checkpoint_us"].as_u64().unwrap()
            );
            assert_eq!(m["checkpoint_probe"]["test_count"], 2);
            assert_eq!(m["parameters_preserved"], true);
        }
    }
    for row in fs::read_to_string(root.join("serial.json.copies.csv"))
        .unwrap()
        .lines()
        .skip(1)
    {
        let fields: Vec<_> = row.splitn(10, ',').collect();
        let action: u64 = fields[5].parse().unwrap();
        let forecast: u64 = fields[6].parse().unwrap();
        let image: u64 = fields[7].parse().unwrap();
        let arrivals: Vec<u64> = serde_json::from_str(fields[9].trim_matches('"')).unwrap();
        assert_eq!(arrivals.len(), 9);
        assert_eq!(
            arrivals,
            vec![
                action + 1500,
                action + 1500,
                action + 1500,
                action + 11500,
                action + 11500,
                action + 11500,
                action + 21500,
                action + 21500,
                action + 21500
            ]
        );
        assert_eq!(image - action, 50000);
        assert!(arrivals.iter().all(|&t| t <= forecast) && forecast < image);
    }
    fs::write(&cfg, serde_json::to_vec(&fixture("unknown")).unwrap()).unwrap();
    let failed = Command::new(env!("CARGO_BIN_EXE_nerva-m6"))
        .arg("--config")
        .arg(&cfg)
        .arg("--output")
        .arg(root.join("invalid.json"))
        .output()
        .unwrap();
    assert!(!failed.status.success());
    assert!(!root.join("invalid.json").exists());
    fs::remove_dir_all(root).unwrap();
}
