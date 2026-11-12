use nerva_research::{
    scenario::{Intervention, Scenario},
    scenarios::hippocampal::{Config, mld02::Mld02Scenario},
};
#[test]
fn atlas_keeps_novel_cues_and_distractor_controls_separate_and_replays() {
    let c = Config {
        processing_neurons: 16,
        training_repetitions: 1,
        rest_us: 100000,
    };
    let row = Mld02Scenario
        .run_verified(
            &c,
            7,
            &Intervention {
                name: "atlas".into(),
            },
        )
        .unwrap();
    assert_eq!(row.values["offline_verified"], 1.);
    assert!(row.values["native_training_passes"] <= 80.);
    let d = &row.details;
    assert_ne!(
        d["cases"]["D"]["training"]["input_digest"],
        d["B_training"]["input_digest"]
    );
    for case in ["F", "G", "C", "P", "L_plus", "L_minus"] {
        assert_eq!(
            d["cases"][case]["training"]["input_digest"],
            d["B_training"]["input_digest"]
        );
    }
    for row in d["observations"].as_array().unwrap() {
        assert_eq!(row["gains"].as_array().unwrap().len(), 6);
        assert_eq!(row["probe"]["frames"].as_array().unwrap().len(), 9);
    }
    assert_eq!(
        d["cases"]["F"]["training"]["weight_changes"]["all"]["changed"],
        0
    );
    let neg = d["cases"]["L_plus"]["training"]["audit"]["blocked_negative"]
        .as_f64()
        .unwrap();
    assert!(neg < 0.);
    assert!(
        d["cases"]["L_minus"]["training"]["audit"]["blocked_positive"]
            .as_f64()
            .unwrap()
            > 0.
    );
}
#[test]
fn developed_structure_is_shared_in_the_complete_two_by_two() {
    let c = Config {
        processing_neurons: 16,
        training_repetitions: 1,
        rest_us: 100000,
    };
    let row = Mld02Scenario
        .run_verified(
            &c,
            7,
            &Intervention {
                name: "interaction".into(),
            },
        )
        .unwrap();
    let d = &row.details;
    assert_eq!(d["shared_checkpoint_verified"], true);
    assert!(
        d["development"]["formed"].as_u64().unwrap() + d["development"]["pruned"].as_u64().unwrap()
            > 0
    );
    let arms = &d["arms"];
    for name in ["H0_K0", "H0_K1", "H1_K0", "H1_K1"] {
        assert_eq!(arms[name]["common_checkpoint_verified"], true);
        assert_eq!(arms[name]["training"]["topology_preserved"], true);
        assert_eq!(
            arms[name]["training"]["weights_before"],
            arms["H0_K0"]["training"]["weights_before"]
        );
    }
    for i in 0..6 {
        let v = |n: &str| arms[n]["margins"][i].as_f64().unwrap();
        assert_eq!(
            d["interaction"][i].as_f64().unwrap(),
            v("H1_K1") - v("H1_K0") - v("H0_K1") + v("H0_K0")
        );
    }
}
