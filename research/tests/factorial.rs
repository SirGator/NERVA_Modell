use nerva_research::{
    scenario::{Intervention, Scenario},
    scenarios::hippocampal::{Config, factorial::FactorialScenario},
};
#[test]
fn factorial_replays_uses_distinct_observations_and_retains_infeasible_arms() {
    let row = FactorialScenario
        .run_verified(
            &Config {
                processing_neurons: 16,
                training_repetitions: 1,
                rest_us: 100000,
            },
            7,
            &Intervention {
                name: "direction_strength".into(),
            },
        )
        .unwrap();
    let d = &row.details;
    assert_eq!(d["candidates"].as_array().unwrap().len(), 18);
    assert_eq!(
        d["observations"].as_array().unwrap().len() + 1,
        row.values["native_training_passes"] as usize
    );
    assert!(row.values["native_training_passes"] <= 25.);
    assert!(d["requested_factorial"]["arms"]["NR"]["observation"].is_null());
    assert_eq!(
        d["requested_factorial"]["arms"]["NR"]["reason"],
        "weight_bounds"
    );
    assert_eq!(d["functional_memory_demonstrated"], false);
    let weak = &d["feasible_factorial"]["arms"]["RN"];
    if weak["faithful"] == false {
        assert_eq!(d["feasible_factorial"]["valid"], false);
    }
}
