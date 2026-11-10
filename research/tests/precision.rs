use nerva_research::{
    scenario::{Intervention, Scenario},
    scenarios::hippocampal::{Config, precision::PrecisionScenario},
};
#[test]
fn native_numeric_calibration_and_paired_precision_preserve_all_controls() {
    let c = Config {
        processing_neurons: 16,
        training_repetitions: 1,
        rest_us: 100000,
    };
    let numeric = PrecisionScenario
        .run_verified(
            &c,
            7,
            &Intervention {
                name: "precision_numeric".into(),
            },
        )
        .unwrap();
    assert_eq!(numeric.details["controls_passed"], true);
    let direct = PrecisionScenario
        .run_verified(
            &c,
            7,
            &Intervention {
                name: "direct".into(),
            },
        )
        .unwrap();
    let accumulated = PrecisionScenario
        .run_verified(
            &c,
            7,
            &Intervention {
                name: "accumulated".into(),
            },
        )
        .unwrap();
    assert_eq!(
        direct.details["initial_network_digest"],
        accumulated.details["initial_network_digest"]
    );
    assert_eq!(
        direct.details["training"]["input_digest"],
        accumulated.details["training"]["input_digest"]
    );
    assert!(direct.values["rounding_error_mass"] > 0.);
    assert!(accumulated.values["rounding_error_mass"] < 1e-15);
    assert!(accumulated.values["pending_mass"] > 0.);
    assert!(accumulated.values["mass_balance_max_error"] < 1e-15);
    assert_eq!(accumulated.details["only_weights_changed"], true);
    let frozen = PrecisionScenario
        .run_verified(
            &c,
            7,
            &Intervention {
                name: "accumulated_frozen".into(),
            },
        )
        .unwrap();
    assert_eq!(frozen.values["weight_l1"], 0.);
    assert_eq!(frozen.values["pending_mass"], 0.);
    assert_eq!(frozen.values["weight_gain_a"], 0.);
}
