use nerva_experiments::{m9, m10, m10d};
use serde_json::json;
use std::{
    fs,
    sync::atomic::AtomicBool,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn recurrence_interventions_preserve_parent_sham_baseline_and_fixed_decoder() {
    let root = std::env::temp_dir().join(format!(
        "nerva-m10d-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&root).unwrap();
    let cfg = m9::parse(
        json!({"protocol":"NERVA-M9-v2","case_id":"weight4_tau100","seed":694,
        "train_trials":8,"test_trials":8,"calibration_trials":8,
        "condition":{"core_weight_scale":4,"stdp_tau_us":100000}}),
    )
    .unwrap();
    let flag = AtomicBool::new(false);
    let reference = m9::run(&cfg, &root.join("old.json"), false, &flag).unwrap();
    let wrapper = m10d::run(&reference, &root.join("new.json"), &flag).unwrap();
    assert!(m10::replay_equal(
        &m10d::without_recurrence(&wrapper["m9_result"]),
        &reference
    ));
    let mut suppressed = 0;
    for arm in ["before", "aged_frozen", "paired", "scrambled"] {
        for mapping in ["normal_mapping", "reversed_mapping"] {
            for stage in ["checkpoint_probe", "rest_checkpoint_probe"] {
                let variants = &wrapper["m9_result"]["arms"][arm][mapping]["recurrence_probes"]["stages"]
                    [stage];
                for variant in ["sham", "block_exc", "block_inh", "block_both"] {
                    for index in 0..2 {
                        let q = &variants[variant][index];
                        assert_eq!(q["trace"][0], variants["sham"][index]["trace"][0]);
                        assert_eq!(
                            q["trace"][1]["potentials_before"],
                            variants["sham"][index]["trace"][1]["potentials_before"]
                        );
                        let ids = q["recurrence_gate"]["blocked_synapses"].as_array().unwrap();
                        if variant != "sham" {
                            assert!(!ids.is_empty());
                        }
                        suppressed += q["recurrence_gate"]["cancelled_arrivals"].as_u64().unwrap();
                        for phase in q["trace"].as_array().unwrap().iter().skip(1) {
                            for e in phase["events"].as_array().unwrap() {
                                if e["kind"] == "arrival" {
                                    assert!(!ids.contains(&e["synapse"]));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
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
    // Depending on the checkpoint there need not be any travelling Core input.
    let _ = suppressed;
    fs::remove_dir_all(root).unwrap();
}
