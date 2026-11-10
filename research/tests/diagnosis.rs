use nerva_research::{
    scenario::{Intervention, Scenario},
    scenarios::hippocampal::{Config, diagnosis::DiagnosisScenario},
};
#[test]
fn native_diagnosis_replays_and_cannot_promote_an_unverified_search_to_a_direction() {
    let row = DiagnosisScenario
        .run_verified(
            &Config {
                processing_neurons: 16,
                training_repetitions: 1,
                rest_us: 100000,
            },
            7,
            &Intervention {
                name: "direction_path".into(),
            },
        )
        .unwrap();
    assert_eq!(row.values["matched_checkpoints"], 1.);
    assert_eq!(row.values["native_training_passes"], 14.);
    assert_eq!(row.details["candidates"].as_array().unwrap().len(), 12);
    assert_eq!(row.details["functional_memory_demonstrated"], false);
    if row.values["reference_verified"] == 0. {
        assert!(row.details["direction_reference"].is_null());
    }
    for branch in ["baseline", "natural"] {
        assert_eq!(row.details[branch]["complete"], true);
        for f in row.details[branch]["frames"].as_array().unwrap() {
            for n in f["neurons"].as_array().unwrap() {
                assert!(n["balance_error"].as_f64().unwrap().abs() < 1e-9);
            }
        }
    }
}
