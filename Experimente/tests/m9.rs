use nerva_experiments::{m8, m9};
use serde_json::{Value, json};
use std::{
    fs,
    path::PathBuf,
    process::{Command, Stdio},
    sync::atomic::AtomicBool,
    time::{SystemTime, UNIX_EPOCH},
};

fn config(weight: u32, tau: u32) -> Value {
    json!({"protocol":"NERVA-M9-v1","case_id":"software_test","seed":691,
        "train_trials":8,"test_trials":8,"calibration_trials":8,
        "condition":{"core_weight_scale":weight,"stdp_tau_us":tau}})
}

#[test]
fn v2_adds_exact_frozen_weight_branches_without_changing_the_original_experiment() {
    let root = std::env::temp_dir().join(format!(
        "nerva-m9-v2-test-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&root).unwrap();
    let old_config = config(4, 100_000);
    let mut new_config = old_config.clone();
    new_config["protocol"] = json!("NERVA-M9-v2");
    let flag = AtomicBool::new(false);
    let old = m9::run(
        &m9::parse(old_config).unwrap(),
        &root.join("old.json"),
        false,
        &flag,
    )
    .unwrap();
    let new = m9::run(
        &m9::parse(new_config.clone()).unwrap(),
        &root.join("new.json"),
        false,
        &flag,
    )
    .unwrap();
    let repeated = m9::run(
        &m9::parse(new_config).unwrap(),
        &root.join("repeat.json"),
        false,
        &flag,
    )
    .unwrap();
    assert_eq!(new, repeated);
    assert_eq!(new["protocol"], "NERVA-M9-v2");
    let mut original_arms = new["arms"].clone();
    let mut branches = 0;
    for arm in ["before", "aged_frozen", "paired", "scrambled"] {
        for mapping in ["normal_mapping", "reversed_mapping"] {
            let model = &new["arms"][arm][mapping];
            for stage in ["checkpoint_probe", "rest_checkpoint_probe"] {
                let probes = &model["weight_reset_probes"]["stages"][stage];
                for intervention in ["sham", "restore_core", "restore_all_plastic"] {
                    for (index, trial) in
                        probes[intervention].as_array().unwrap().iter().enumerate()
                    {
                        branches += 1;
                        assert_eq!(trial["effect_not_presented"], true);
                        assert_eq!(trial["parent_unchanged"], true);
                        assert_eq!(trial["frozen_parameters_preserved"], true);
                        if intervention == "sham" {
                            assert_eq!(
                                trial["prediction_count"],
                                model[stage]["predictions_count"][index]
                            );
                        }
                        if arm == "before" || arm == "aged_frozen" {
                            assert!(trial["changed_weights"].as_array().unwrap().is_empty());
                            assert_eq!(trial["core_events"], probes["sham"][index]["core_events"]);
                        }
                    }
                }
            }
            original_arms[arm][mapping]
                .as_object_mut()
                .unwrap()
                .remove("weight_reset_probes");
        }
    }
    assert_eq!(branches, 96);
    assert_eq!(old["arms"], original_arms);
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
            fs::read(root.join(format!("new.json{suffix}"))).unwrap()
        );
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn strict_factorial_config_preserves_all_other_model_parameters() {
    for weight in [1, 4] {
        for tau in [20_000, 100_000] {
            let parsed = m9::parse(config(weight, tau)).unwrap();
            let model = &parsed.experiment.engine.base.model;
            assert_eq!(model.core_weight_scale, weight as f32);
            assert_eq!(model.stdp_tau_us, tau as f32);
            assert_eq!(model.core_membrane_tau_us, 20_000.0);
            assert_eq!(model.core_threshold, 1.0);
            assert_eq!(model.core_fanout, 4);
            assert_eq!(model.core_delay_us, None);
            assert_eq!(model.sensory_weight_scale, 1.0);
            assert!(!model.homeostasis && !model.development);
            assert!(model.learning && model.feedback);
        }
    }
    for condition in [
        json!({"core_weight_scale":2,"stdp_tau_us":100000}),
        json!({"core_weight_scale":4,"stdp_tau_us":50000}),
        json!({"core_weight_scale":true,"stdp_tau_us":100000}),
        json!({"core_weight_scale":4,"stdp_tau_us":"100000"}),
        json!({"core_weight_scale":4}),
        json!({"core_weight_scale":4,"stdp_tau_us":100000,"core_threshold":0.6}),
    ] {
        let mut bad = config(4, 100_000);
        bad["condition"] = condition;
        assert!(m9::parse(bad).is_err());
    }
    for (field, value) in [
        ("protocol", json!("NERVA-M8-v1")),
        ("future_label", json!(1)),
        ("test_trials", json!(10)),
        ("seed", json!(-1)),
    ] {
        let mut bad = config(4, 100_000);
        bad[field] = value;
        assert!(m9::parse(bad).is_err());
    }
    let mut old = config(4, 100_000);
    old["protocol"] = json!("NERVA-M8-v1");
    assert!(m8::parse(old).is_err(), "M8 must still reject combinations");
}

#[test]
fn combined_runs_are_reproducible_and_existing_outputs_are_preserved() {
    let root: PathBuf = std::env::temp_dir().join(format!(
        "nerva-m9-test-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&root).unwrap();
    let cfg = root.join("config.json");
    fs::write(&cfg, serde_json::to_vec(&config(4, 100_000)).unwrap()).unwrap();
    let launch = |name: &str| {
        Command::new(env!("CARGO_BIN_EXE_nerva-m9"))
            .arg("--config")
            .arg(&cfg)
            .arg("--output")
            .arg(root.join(name))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap()
    };
    let mut first = launch("a.json");
    let mut second = launch("b.json");
    assert!(first.wait().unwrap().success());
    assert!(second.wait().unwrap().success());
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
    let before = fs::read(root.join("a.json")).unwrap();
    assert!(!launch("a.json").wait().unwrap().success());
    assert_eq!(before, fs::read(root.join("a.json")).unwrap());
    let result: Value = serde_json::from_slice(&before).unwrap();
    assert_eq!(result["protocol"], "NERVA-M9-v1");
    assert_eq!(result["requested_config"], config(4, 100_000));
    assert_eq!(result["condition"], config(4, 100_000)["condition"]);
    assert_eq!(result["effective_parameters"]["core_weight_scale"], 4.0);
    assert_eq!(result["effective_parameters"]["stdp_tau_us"], 100_000.0);
    for arm in ["before", "aged_frozen", "paired", "scrambled"] {
        for mapping in ["normal_mapping", "reversed_mapping"] {
            let model = &result["arms"][arm][mapping];
            assert_eq!(model["checkpoint_us"], model["immediate_probe_start_us"]);
            assert_eq!(model["rest_checkpoint_probe"]["test_count"], 2);
            assert_eq!(model["rest_checkpoint_probe"]["rest_us"], 2_000_000);
            assert!(model["parameters_preserved"].as_bool().unwrap());
        }
    }
    fs::remove_dir_all(root).unwrap();
}
