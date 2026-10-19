use nerva_experiments::m8;
use serde_json::{Value, json};
use std::{
    fs,
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};
fn config() -> Value {
    json!({"protocol":"NERVA-M8-v1","case_id":"software_test","seed":9,"train_trials":8,"test_trials":8,"calibration_trials":8,"condition":{"parameter":"core_weight_scale","value":2}})
}
fn folder() -> PathBuf {
    std::env::temp_dir().join(format!(
        "nerva-m8-test-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}
#[test]
fn m8_rejects_unknown_fields_levels_and_combined_parameter_instructions() {
    assert!(m8::parse(config()).is_ok());
    for (key, value) in [("future_label", json!(1)), ("protocol", json!("NERVA-M7"))] {
        let mut bad = config();
        bad[key] = value;
        assert!(m8::parse(bad).is_err());
    }
    let mut bad = config();
    bad["condition"]["value"] = json!(3);
    assert!(m8::parse(bad).is_err());
    let mut bad = config();
    bad["condition"]["core_threshold"] = json!(0.6);
    assert!(m8::parse(bad).is_err());
}
#[test]
fn m8_parallel_outputs_and_full_checkpoint_rest_and_no_copy_contract_are_reproducible() {
    let root = folder();
    fs::create_dir(&root).unwrap();
    let cfg = root.join("config.json");
    fs::write(&cfg, serde_json::to_vec(&config()).unwrap()).unwrap();
    let launch = |name: &str| {
        Command::new(env!("CARGO_BIN_EXE_nerva-m8"))
            .arg("--config")
            .arg(&cfg)
            .arg("--output")
            .arg(root.join(name))
            .spawn()
            .unwrap()
    };
    let mut a = launch("a.json");
    let mut b = launch("b.json");
    assert!(a.wait().unwrap().success());
    assert!(b.wait().unwrap().success());
    for suffix in [
        "",
        ".details.csv",
        ".audit.csv",
        ".sensor.csv",
        ".copies.csv",
        ".branches.csv",
        ".propagation.csv",
    ] {
        assert_eq!(
            fs::read(root.join(format!("a.json{suffix}"))).unwrap(),
            fs::read(root.join(format!("b.json{suffix}"))).unwrap()
        );
    }
    let r: Value = serde_json::from_slice(&fs::read(root.join("a.json")).unwrap()).unwrap();
    for arm in ["before", "aged_frozen", "paired", "scrambled"] {
        for mapping in ["normal_mapping", "reversed_mapping"] {
            let model = &r["arms"][arm][mapping];
            assert_eq!(model["checkpoint_us"], model["immediate_probe_start_us"]);
            assert_eq!(model["rest_checkpoint_probe"]["test_count"], 2);
            assert_eq!(model["rest_checkpoint_probe"]["rest_us"], 2000000);
            assert_eq!(
                model["no_copy_probes"]["rest_checkpoint_probe"]["start_us"]
                    .as_u64()
                    .unwrap(),
                model["checkpoint_us"].as_u64().unwrap() + 2050000
            );
            assert!(model["parameters_preserved"].as_bool().unwrap());
        }
    }
    fs::remove_dir_all(root).unwrap();
}
