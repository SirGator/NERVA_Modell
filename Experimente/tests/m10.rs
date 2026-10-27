use nerva_experiments::{m9, m10};
use serde_json::{Value, json};
use std::{
    fs,
    sync::atomic::AtomicBool,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn replay_comparison_is_exact_for_integers_and_tolerates_only_float_roundoff() {
    assert!(!m10::replay_equal(
        &json!({"time":100}),
        &json!({"time":101})
    ));
    assert!(!m10::replay_equal(
        &json!({"count":1}),
        &json!({"count":1,"extra":1})
    ));
    assert!(m10::replay_equal(
        &json!([0.5]),
        &json!([0.5 + f64::EPSILON])
    ));
    assert!(!m10::replay_equal(&json!([0.5]), &json!([0.50001])));
}

#[test]
fn passive_trace_reproduces_m9_and_all_historical_files() {
    let root = std::env::temp_dir().join(format!(
        "nerva-m10-test-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&root).unwrap();
    let config = m9::parse(
        json!({"protocol":"NERVA-M9-v2","case_id":"weight4_tau100","seed":693,
        "train_trials":8,"test_trials":8,"calibration_trials":8,
        "condition":{"core_weight_scale":4,"stdp_tau_us":100000}}),
    )
    .unwrap();
    let flag = AtomicBool::new(false);
    let old = m9::run(&config, &root.join("old.json"), false, &flag).unwrap();
    let reference: Value = serde_json::from_slice(&serde_json::to_vec(&old).unwrap()).unwrap();
    let traced = m10::run(&reference, &root.join("trace.json"), &flag).unwrap();
    let repeated = m10::run(&reference, &root.join("repeat.json"), &flag).unwrap();
    assert_eq!(traced, repeated);
    assert!(m10::replay_equal(
        &m10::without_trace(&traced["m9_result"]),
        &reference
    ));
    let mut phases = 0;
    for arm in ["before", "aged_frozen", "paired", "scrambled"] {
        for mapping in ["normal_mapping", "reversed_mapping"] {
            for stage in ["checkpoint_probe", "rest_checkpoint_probe"] {
                for intervention in ["sham", "restore_core", "restore_all_plastic"] {
                    for trial in
                        traced["m9_result"]["arms"][arm][mapping]["weight_reset_probes"]["stages"]
                            [stage][intervention]
                            .as_array()
                            .unwrap()
                    {
                        for phase in trial["trace"].as_array().unwrap() {
                            phases += 1;
                            assert_eq!(phase["potentials_before"].as_array().unwrap().len(), 32);
                            assert_eq!(phase["potentials_after"].as_array().unwrap().len(), 32);
                            assert!(
                                phase["before_us"].as_u64().unwrap()
                                    < phase["start_us"].as_u64().unwrap()
                            );
                        }
                    }
                }
            }
        }
    }
    assert_eq!(phases, 384);
    for suffix in [
        ".details.csv",
        ".audit.csv",
        ".sensor.csv",
        ".copies.csv",
        ".branches.csv",
        ".propagation.csv",
    ] {
        assert_eq!(
            fs::read(root.join(format!("old.json{suffix}"))).unwrap(),
            fs::read(root.join(format!("trace.json{suffix}"))).unwrap()
        );
    }
    let mut invalid = reference.clone();
    invalid["protocol"] = json!("NERVA-M9-v1");
    assert!(m10::run(&invalid, &root.join("invalid.json"), &flag).is_err());
    fs::remove_dir_all(root).unwrap();
}
