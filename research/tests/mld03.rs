use nerva_research::{
    scenario::{Intervention, Scenario},
    scenarios::hippocampal::{Config, mld03::Mld03Scenario},
};
#[test]
fn complete_followup_replays_and_keeps_activity_and_weight_controls_separate() {
    let c = Config {
        processing_neurons: 16,
        training_repetitions: 1,
        rest_us: 100000,
    };
    for section in ["excitability", "population"] {
        let row = Mld03Scenario
            .run_verified(
                &c,
                7,
                &Intervention {
                    name: section.into(),
                },
            )
            .unwrap();
        assert_eq!(row.details["shared_checkpoint_verified"], true);
        if section == "excitability" {
            assert_eq!(row.details["conditions"].as_object().unwrap().len(), 6);
            for condition in row.details["conditions"].as_object().unwrap().values() {
                assert_eq!(condition["arms"].as_object().unwrap().len(), 4);
                for frame in condition["reset_probe"]["frames"].as_array().unwrap() {
                    assert!(frame["spike_times"].is_array());
                    assert!(frame["integration_boundaries"].is_array());
                }
            }
        } else {
            let arms = &row.details["arms"];
            for name in ["off", "sensor", "core", "both", "all_bridge"] {
                assert_eq!(
                    arms[name]["training"]["weights_before"],
                    arms["off"]["training"]["weights_before"]
                );
                assert_eq!(arms[name]["training"]["topology_preserved"], true);
            }
            assert!(
                arms["off"]["training"]["homeostasis_by_neuron"]
                    .as_array()
                    .unwrap()
                    .is_empty()
            );
            let sensor = row.details["populations"]["sensor"].as_array().unwrap();
            assert!(
                arms["sensor"]["training"]["homeostasis_by_neuron"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|n| sensor.contains(&n["neuron_id"]))
            );
        }
    }
}
