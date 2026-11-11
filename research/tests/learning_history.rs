use nerva_research::{
    scenario::{Intervention, Scenario},
    scenarios::hippocampal::history::HistoryScenario,
};
use serde_json::json;
fn config() -> <HistoryScenario as Scenario>::Config {
    let mut c = HistoryScenario.default_config();
    for (name, value) in [
        ("processing_neurons", json!(16)),
        ("training_repetitions", json!(1)),
        ("rest_us", json!(100000)),
    ] {
        HistoryScenario.set_parameter(&mut c, name, &value).unwrap();
    }
    c
}
#[test]
fn histories_are_chronological_indexed_and_preserve_full_native_replay() {
    let mut c = config();
    HistoryScenario
        .set_parameter(&mut c, "history.neurons", &json!([64]))
        .unwrap();
    let row = HistoryScenario
        .run_verified(
            &c,
            7,
            &Intervention {
                name: "three_on".into(),
            },
        )
        .unwrap();
    let h = &row.details["history"];
    assert_eq!(h["complete"], true);
    assert_eq!(row.details["observation_preserves_full_state"], true);
    let events = h["events"].as_array().unwrap();
    assert!(!events.is_empty());
    assert!(
        events
            .iter()
            .any(|e| e["branch"] == "learned" && e["phase"] == "recall")
    );
    assert!(
        events
            .iter()
            .any(|e| e["branch"] == "weight_reset" && e["phase"] == "recall")
    );
    assert!(
        events
            .windows(2)
            .all(|w| w[0]["time_us"].as_u64() <= w[1]["time_us"].as_u64())
    );
    for (id, indices) in h["synapse_index"].as_object().unwrap() {
        for index in indices.as_array().unwrap() {
            assert_eq!(
                events[index.as_u64().unwrap() as usize]["synapse_id"]
                    .as_u64()
                    .unwrap()
                    .to_string(),
                *id
            );
        }
    }
    assert_eq!(
        h["summary"]["weight_events"],
        row.details["training"]["audit"]["weight_events"]
    );
    assert_eq!(
        h["summary"]["requests_without_change"],
        row.details["training"]["audit"]["requests_without_change"]
    );
    for edge in h["synapse_summary"].as_array().unwrap() {
        let net =
            edge["positive_actual"].as_f64().unwrap() + edge["negative_actual"].as_f64().unwrap();
        let difference =
            edge["weight_after"].as_f64().unwrap() - edge["weight_before"].as_f64().unwrap();
        assert!((net - difference).abs() < 1e-12);
    }
    let by_id = events
        .iter()
        .map(|e| (e["id"].as_u64().unwrap(), e))
        .collect::<std::collections::BTreeMap<_, _>>();
    for e in events {
        if let Some(update) = e["data"]["training_update_id"].as_u64() {
            let prior = by_id[&update];
            assert_eq!(prior["synapse_id"], e["synapse_id"]);
            assert_eq!(prior["phase"], "training");
            assert!(prior["time_us"].as_u64() <= e["time_us"].as_u64());
        }
    }
}
#[test]
fn summary_only_and_frozen_controls_have_identical_metrics_and_no_spurious_learning() {
    let c = config();
    let mut detailed = c.clone();
    HistoryScenario
        .set_parameter(&mut detailed, "history.neurons", &json!([64]))
        .unwrap();
    for group in ["frozen_on", "three_off", "pair_on"] {
        let i = Intervention { name: group.into() };
        let a = HistoryScenario.run(&c, 7, &i).unwrap();
        let b = HistoryScenario.run(&detailed, 7, &i).unwrap();
        assert_eq!(
            a.details["history"]["summary"],
            b.details["history"]["summary"]
        );
        assert!(
            a.details["history"]["events"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        if group != "pair_on" {
            assert_eq!(a.values["weight_events"], 0.);
            assert_eq!(a.values["recall_used_changed_synapses"], 0.);
            assert_eq!(a.values["recall_core_effect"], 0.);
            assert_eq!(a.values["weight_gain_a"], 0.);
        }
    }
}
#[test]
fn invalid_selection_and_overflow_fail_instead_of_claiming_complete_history() {
    let mut c = config();
    assert!(
        HistoryScenario
            .set_parameter(&mut c, "history.neurons", &json!([64, 64]))
            .is_err()
    );
    assert!(
        HistoryScenario
            .set_parameter(&mut c, "history.synapses", &json!([-1]))
            .is_err()
    );
    assert!(
        HistoryScenario
            .set_parameter(&mut c, "history.max_rows", &json!(0))
            .is_err()
    );
    HistoryScenario
        .set_parameter(&mut c, "history.neurons", &json!([u64::MAX]))
        .unwrap();
    assert!(HistoryScenario.validate(&c).is_err());
    HistoryScenario
        .set_parameter(&mut c, "history.neurons", &json!([64]))
        .unwrap();
    HistoryScenario
        .set_parameter(&mut c, "history.max_rows", &json!(1))
        .unwrap();
    assert!(
        HistoryScenario
            .run(
                &c,
                7,
                &Intervention {
                    name: "three_on".into()
                }
            )
            .unwrap_err()
            .to_string()
            .contains("budget exceeded")
    );
}

#[test]
fn a_synapse_selection_does_not_expand_to_neighboring_connections() {
    let c = config();
    let i = Intervention {
        name: "pair_on".into(),
    };
    let summary = HistoryScenario.run(&c, 7, &i).unwrap();
    let id = summary.details["history"]["synapse_summary"][0]["synapse_id"]
        .as_u64()
        .unwrap();
    let mut selected = c;
    HistoryScenario
        .set_parameter(&mut selected, "history.synapses", &json!([id]))
        .unwrap();
    let row = HistoryScenario.run(&selected, 7, &i).unwrap();
    let h = &row.details["history"];
    assert_eq!(h["selection"]["synapses"], json!([id]));
    for e in h["events"].as_array().unwrap() {
        if let Some(synapse) = e["synapse_id"].as_u64() {
            assert_eq!(synapse, id);
        }
    }
    assert_eq!(h["summary"], summary.details["history"]["summary"]);
}

#[test]
fn source_processing_budget_is_enforced_even_without_retained_events() {
    let mut c = config();
    HistoryScenario
        .set_parameter(&mut c, "history.max_source_rows", &json!(1))
        .unwrap();
    assert!(
        HistoryScenario
            .run(
                &c,
                7,
                &Intervention {
                    name: "three_on".into()
                }
            )
            .unwrap_err()
            .to_string()
            .contains("source audit budget exceeded")
    );
    assert!(
        HistoryScenario
            .set_parameter(&mut c, "history.max_source_rows", &json!(100000001))
            .is_err()
    );
}
