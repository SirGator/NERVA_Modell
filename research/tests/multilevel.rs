use nerva_research::{
    scenario::{Intervention, Scenario},
    scenarios::hippocampal::{Config, multilevel::MultilevelScenario},
};
#[test]
fn native_local_replay_matches_and_all_diagnostic_arms_replay() {
    let c = Config {
        processing_neurons: 16,
        training_repetitions: 1,
        rest_us: 100000,
    };
    let row = MultilevelScenario
        .run_verified(
            &c,
            7,
            &Intervention {
                name: "diagnose".into(),
            },
        )
        .unwrap();
    assert_eq!(
        row.details["offline"]["verified_original_local_state"],
        true
    );
    assert!(row.values["native_training_passes"] <= 30.);
    assert_eq!(
        row.details["live"]["A_frozen"]["training"]["weight_changes"]["all"]["changed"],
        0
    );
    assert_eq!(
        row.details["live"]["C_constant"]["history"]["complete"],
        true
    );
    for event in row.details["live"]["C_constant"]["history"]["events"]
        .as_array()
        .unwrap()
    {
        if event["stream"] == "learning" && event["mechanism"] != "neuromodulation" {
            assert_eq!(event["data"]["held_learning_signal"], 1.);
        }
    }
    for variants in row.details["normalized"].as_object().unwrap().values() {
        for arm in variants.as_object().unwrap().values() {
            if arm["observation"].is_number() {
                assert_eq!(arm["faithful"], true);
            }
        }
    }
}
