use nerva_research::{
    scenario::{Intervention, Scenario},
    scenarios::continual::ContinualScenario,
};

#[test]
fn frozen_probes_replay_without_learning_and_controls_preserve_weights() {
    let scenario = ContinualScenario;
    let config = scenario.default_config();
    scenario.validate(&config).unwrap();
    for name in ["no_b", "freeze_after_a"] {
        let result = scenario
            .run_verified(&config, 900_000_001, &Intervention { name: name.into() })
            .unwrap();
        assert_eq!(result.values["weights_frozen"], 1.);
        assert_eq!(result.values["probe_replay_identical"], 1.);
        assert_eq!(result.values["untrained_a_score"], 0.);
        assert_eq!(result.values["untrained_b_score"], 0.);
        assert_eq!(
            result.details["weights_after_a"],
            result.details["weights_after_b"]
        );
        assert_eq!(result.values["interference_assessable"], 0.);
    }
}

#[test]
fn unlearned_tasks_cannot_be_classified_as_forgetting() {
    let scenario = ContinualScenario;
    let mut config = scenario.default_config();
    scenario
        .set_parameter(&mut config, "training_repetitions", &serde_json::json!(1))
        .unwrap();
    let result = scenario
        .run_verified(
            &config,
            900_104_730,
            &Intervention {
                name: "none".into(),
            },
        )
        .unwrap();
    assert_eq!(result.values["a_acquired"], 0.);
    assert_eq!(result.values["interference_assessable"], 0.);
    assert_eq!(result.values["interference_observed"], 0.);
}

#[test]
fn all_conditions_share_initial_network_and_replay_training_is_deterministic() {
    let scenario = ContinualScenario;
    let config = scenario.default_config();
    let mut initial = None;
    for name in ["none", "no_b", "freeze_after_a", "disjoint_b"] {
        let result = scenario
            .run_verified(&config, 900_000_001, &Intervention { name: name.into() })
            .unwrap();
        if let Some(expected) = &initial {
            assert_eq!(&result.details["weights_after_a"], expected);
        } else {
            initial = Some(result.details["weights_after_a"].clone());
        }
        assert_eq!(result.values["weights_frozen"], 1.);
        assert_eq!(result.values["probe_replay_identical"], 1.);
    }
}
