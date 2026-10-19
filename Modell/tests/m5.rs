use nerva_vm::{m4, m5};
use serde_json::{Value, json};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::AtomicBool,
    time::{SystemTime, UNIX_EPOCH},
};
fn fixture(experiment: &str, scope: &str) -> Value {
    let motor = experiment == "motor_threshold";
    json!({"protocol":"NERVA-M5-v1","experiment":experiment,"homeostasis_scope":scope,
        "action_copy_weight":if motor{3.6}else{0.9},"core_membrane_tau_us":20000,
        "base":{"protocol":"NERVA-M4-v1","case_id":"m5_test","seed":0,
        "params":{"sensor":if motor{"effect_2ch"}else{"pixels"},"resolution":if motor{0}else{16},
        "encoding":"deterministic","processing":if motor{32}else{128},"action_effect_delay_ms":8,
        "motor_feedback":true,"stdp":true,"homeostasis":scope!="none","development":false,
        "initial_drive":0,"pruning_min_us":5000000,"normalization":"equal_input_current"},
        "arms":["before","aged_frozen","paired","scrambled"],"train_trials":8,"test_trials":8,"calibration_trials":8}})
}
fn root() -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("nerva-m5-{}-{stamp}", std::process::id()));
    fs::create_dir(&root).unwrap();
    root
}
#[test]
fn strict_m5_config_rejects_unknown_modes_scopes_and_invalid_signal_parameters() {
    for (field, value) in [
        ("experiment", json!("unknown")),
        ("homeostasis_scope", json!("sensors_only")),
        ("action_copy_weight", json!(-1)),
        ("core_membrane_tau_us", json!(0)),
        ("future_label", json!(1)),
    ] {
        let mut config = fixture("motor_threshold", "none");
        config[field] = value;
        assert!(m5::Config::parse(config).is_err(), "{field}");
    }
    let mut mismatch = fixture("sensor_homeostasis", "core");
    mismatch["base"]["params"]["homeostasis"] = json!(false);
    assert!(m5::Config::parse(mismatch).is_err());
}
#[test]
fn motor_signals_reach_real_plastic_paths_and_frozen_replays_are_identical() {
    let root = root();
    let config = m5::Config::parse(fixture("motor_threshold", "none")).unwrap();
    let flag = AtomicBool::new(false);
    let a = m5::run(&config, &root.join("a.json"), false, &flag).unwrap();
    let b = m5::run(&config, &root.join("b.json"), false, &flag).unwrap();
    assert_eq!(a, b);
    assert_eq!(a["signal_criteria"]["both_pass"], true);
    assert_eq!(
        fs::read(root.join("a.json.events.csv")).unwrap(),
        fs::read(root.join("b.json.events.csv")).unwrap()
    );
    for row in a["observations"].as_array().unwrap() {
        assert_eq!(row["generated_and_emitted_visual_spikes"], 0);
        if row["action"] == "none" {
            assert_eq!(row["core_spike_count"], 0);
        } else {
            assert_eq!(row["copy_spikes"], 3);
            assert_eq!(row["copy_arrivals"], 9);
            assert!(row["plastic_core_arrival_count"].as_u64().unwrap() > 0);
        }
    }
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn homeostasis_scopes_preserve_initial_network_and_legacy_m4_behavior() {
    let root = root();
    let flag = AtomicBool::new(false);
    let mut hashes = Vec::new();
    for scope in ["none", "core", "core_sensor", "all"] {
        let config = m5::Config::parse(fixture("sensor_homeostasis", scope)).unwrap();
        let result = m5::run(&config, &root.join(format!("{scope}.json")), false, &flag).unwrap();
        hashes.push(result["initial_network_fingerprint"].clone());
        assert_eq!(result["checks"]["homeostasis_population_respected"], true);
        for arm in ["before", "aged_frozen", "paired", "scrambled"] {
            for mapping in ["normal_mapping", "reversed_mapping"] {
                let value = &result["arms"][arm][mapping]["m5_sensor"];
                if scope == "none" || scope == "core" {
                    assert_eq!(
                        value["initial"]["sensor_intrinsic_currents"],
                        value["final"]["sensor_intrinsic_currents"]
                    );
                }
            }
        }
        if scope == "all" {
            let legacy = m4::run(&config.base, &root.join("legacy.json"), false, &flag).unwrap();
            let mut adapted = result.clone();
            for key in [
                "experiment",
                "homeostasis_scope",
                "initial_network_fingerprint",
                "initial_equals_global_scope_network",
            ] {
                adapted.as_object_mut().unwrap().remove(key);
            }
            adapted["protocol"] = json!("NERVA-M4-v1");
            adapted["requested_config"] = config.requested["base"].clone();
            adapted["checks"]
                .as_object_mut()
                .unwrap()
                .remove("homeostasis_population_respected");
            for arm in ["before", "aged_frozen", "paired", "scrambled"] {
                for mapping in ["normal_mapping", "reversed_mapping"] {
                    adapted["arms"][arm][mapping]
                        .as_object_mut()
                        .unwrap()
                        .remove("m5_sensor");
                }
            }
            assert_eq!(adapted, legacy);
            for suffix in [".audit.csv", ".details.csv"] {
                assert_eq!(
                    fs::read(root.join(format!("all.json{suffix}"))).unwrap(),
                    fs::read(root.join(format!("legacy.json{suffix}"))).unwrap()
                );
            }
        }
    }
    assert!(hashes.iter().all(|x| x == &hashes[0]));
    fs::remove_dir_all(root).unwrap();
}
