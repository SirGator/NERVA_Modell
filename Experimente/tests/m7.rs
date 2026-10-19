use nerva_experiments::m7;
use nerva_vm::m6;
use serde_json::{Value, json};
use std::{
    fs,
    path::PathBuf,
    process::Command,
    sync::atomic::AtomicBool,
    time::{SystemTime, UNIX_EPOCH},
};
fn folder() -> PathBuf {
    std::env::temp_dir().join(format!(
        "nerva-m7-test-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}
fn config() -> Value {
    json!({"protocol":"NERVA-M6-v1","variant":"D","base":{"protocol":"NERVA-M4-v1","case_id":"m7_test","seed":7,
    "params":{"sensor":"effect_2ch","resolution":0,"encoding":"deterministic","processing":32,"action_effect_delay_ms":50,"motor_feedback":true,"stdp":true,"homeostasis":false,"development":false,"initial_drive":0,"pruning_min_us":5000000,"normalization":"equal_input_current"},
    "arms":["before","aged_frozen","paired","scrambled"],"train_trials":8,"test_trials":8,"calibration_trials":8}})
}
#[test]
fn synaptic_audit_reproduces_m6_and_parallel_counterfactuals_are_byte_equal() {
    let root = folder();
    fs::create_dir(&root).unwrap();
    let config = m6::Config::parse(config()).unwrap();
    let reference = m6::run(
        &config,
        &root.join("m6.json"),
        false,
        &AtomicBool::new(false),
    )
    .unwrap();
    let path = root.join("reference.json");
    fs::write(&path, serde_json::to_vec(&reference).unwrap()).unwrap();
    let launch = |name: &str| {
        Command::new(env!("CARGO_BIN_EXE_nerva-m7"))
            .args(["--reference"])
            .arg(&path)
            .arg("--output")
            .arg(root.join(name))
            .spawn()
            .unwrap()
    };
    let mut a = launch("parallel_a");
    let mut b = launch("parallel_b");
    assert!(a.wait().unwrap().success());
    assert!(b.wait().unwrap().success());
    for file in ["result.json", "weights.csv", "events.csv", "pairs.csv"] {
        assert_eq!(
            fs::read(root.join("parallel_a").join(file)).unwrap(),
            fs::read(root.join("parallel_b").join(file)).unwrap()
        );
    }
    let r: Value =
        serde_json::from_slice(&fs::read(root.join("parallel_a/result.json")).unwrap()).unwrap();
    assert!(r["checks"].as_object().unwrap().values().all(|v| v == true));
    assert_eq!(r["runs"].as_array().unwrap().len(), 6);
    for run in r["runs"].as_array().unwrap() {
        assert_eq!(run["counterfactuals"].as_array().unwrap().len(), 10);
        for key in ["a", "d"] {
            let probes: Vec<_> = run["counterfactuals"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|p| p["response"]["key"] == key)
                .collect();
            assert!(
                probes
                    .iter()
                    .all(|p| p["response"]["effect_not_presented"] == true)
            );
            let silent = probes.iter().find(|p| p["condition"] == "no_copy").unwrap();
            assert_eq!(silent["response"]["copy_arrivals"], 0);
        }
    }
    let mut invalid = reference.clone();
    invalid["checks"] = json!({});
    assert!(m7::validate_reference(&invalid).is_err());
    invalid = reference.clone();
    invalid["seed"] = json!(99);
    assert!(m7::validate_reference(&invalid).is_err());
    invalid = reference;
    invalid["requested_config"]["base"]["params"]["homeostasis"] = json!(true);
    assert!(m7::validate_reference(&invalid).is_err());
    fs::remove_dir_all(root).unwrap();
}
