use nerva::experiment::{M0Experiment, M0ExperimentConfig, M0Group};
use nerva_research::{
    scenario::{Intervention, Scenario},
    scenarios::m0_selection::M0SelectionScenario,
};
use serde_json::json;

#[test]
fn explicit_reference_controls_preserve_all_legacy_group_policies() {
    for (group, stdp, homeostasis) in [
        (M0Group::OrderedLearning, true, false),
        (M0Group::OrderedFixed, false, false),
        (M0Group::RandomLearning, true, false),
        (M0Group::OrderedHomeostasis, true, true),
    ] {
        let experiment = M0Experiment::new(M0ExperimentConfig::default(), group).unwrap();
        assert_eq!(
            experiment.run().unwrap(),
            experiment.run_controlled(stdp, homeostasis, 0).unwrap()
        );
    }
}

#[test]
fn controlled_adapter_switches_real_mechanisms_and_retains_matched_controls() {
    let scenario = M0SelectionScenario;
    let mut config = scenario.default_config();
    scenario
        .set_parameter(&mut config, "mechanisms.stdp", &json!(false))
        .unwrap();
    scenario
        .set_parameter(&mut config, "mechanisms.homeostasis", &json!(true))
        .unwrap();
    for name in [
        "none",
        "disable_stdp",
        "shuffle_input",
        "disable_homeostasis",
    ] {
        let intervention = Intervention {
            name: name.to_owned(),
        };
        let snapshot = scenario.snapshot(&config, 7, &intervention);
        assert_eq!(snapshot["learning_enabled"], false);
        assert_eq!(
            snapshot["homeostasis_enabled"],
            name != "disable_homeostasis"
        );
        let metrics = scenario.run_verified(&config, 7, &intervention).unwrap();
        for endpoint in ["immediate", "retained"] {
            assert_eq!(
                metrics.details[endpoint]["initial_weights"],
                metrics.details[endpoint]["final_weights"]
            );
        }
    }
    let config = M0ExperimentConfig::default();
    let assay = M0Experiment::new(config, M0Group::OrderedLearning).unwrap();
    for enabled in [false, true] {
        let result = assay.run_controlled(false, enabled, 0).unwrap();
        assert_eq!(
            result.event_log.iter().any(|e| matches!(
                e,
                nerva::runtime::ObservationEvent::HomeostasisChanged { .. }
            )),
            enabled
        );
    }
}

#[test]
fn retention_adds_a_real_quiet_interval_before_the_cue() {
    let native =
        M0Experiment::new(M0ExperimentConfig::default(), M0Group::OrderedLearning).unwrap();
    let before = native.run_controlled(true, false, 0).unwrap();
    let after = native.run_controlled(true, false, 200_000).unwrap();
    assert_eq!(before.initial_weights, after.initial_weights);
    assert_eq!(before.final_weights, after.final_weights);
    assert_eq!(
        after.predicted_patterns[0].0.0 - before.predicted_patterns[0].0.0,
        200_000
    );
}
