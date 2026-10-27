use nerva_experiments::{m9, m10, m10e};
use serde_json::json;
use std::{
    fs,
    sync::atomic::AtomicBool,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn crossed_reset_preserves_action_state_and_blocked_weight_changes_have_no_effect() {
    let root = std::env::temp_dir().join(format!(
        "nerva-m10e-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&root).unwrap();
    let cfg = m9::parse(
        json!({"protocol":"NERVA-M9-v2","case_id":"weight4_tau100","seed":695,
        "train_trials":8,"test_trials":8,"calibration_trials":8,
        "condition":{"core_weight_scale":4,"stdp_tau_us":100000}}),
    )
    .unwrap();
    let flag = AtomicBool::new(false);
    let reference = m9::run(&cfg, &root.join("old.json"), false, &flag).unwrap();
    let wrapper = m10e::run(&reference, &root.join("new.json"), &flag).unwrap();
    assert!(m10::replay_equal(
        &m10e::without_crossed(&wrapper["m9_result"]),
        &reference
    ));
    let mut changed = 0;
    for arm in ["before", "aged_frozen", "paired", "scrambled"] {
        for mapping in ["normal_mapping", "reversed_mapping"] {
            for stage in ["checkpoint_probe", "rest_checkpoint_probe"] {
                let variants =
                    &wrapper["m9_result"]["arms"][arm][mapping]["crossed_probes"]["stages"][stage];
                for gate in ["sham", "block_exc", "block_inh", "block_both"] {
                    for index in 0..2 {
                        let trained = &variants[format!("trained__{gate}")][index];
                        let reset = &variants[format!("reset_core__{gate}")][index];
                        assert_eq!(trained["trace"][0], reset["trace"][0]);
                        for field in ["potentials_before", "refractory_until_before"] {
                            assert_eq!(trained["trace"][1][field], reset["trace"][1][field]);
                        }
                        assert_eq!(
                            reset["weight_intervention"]["activation_us"],
                            reset["action_us"]
                        );
                        let edits = reset["changed_weights"].as_array().unwrap();
                        changed += edits.len();
                        let blocked = reset["recurrence_gate"]["blocked_synapses"]
                            .as_array()
                            .unwrap();
                        if edits.iter().all(|e| blocked.contains(&e[0])) {
                            assert_eq!(trained["trace"], reset["trace"]);
                        }
                    }
                }
            }
        }
    }
    assert!(changed > 0);
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
