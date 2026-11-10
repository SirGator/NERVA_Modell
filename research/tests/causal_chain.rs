use nerva_research::{
    scenario::{Intervention, Scenario},
    scenarios::hippocampal::{Config, chain::ChainScenario},
};
use serde_json::Value;
fn run(name: &str) -> nerva_research::scenario::TrialMetrics {
    ChainScenario
        .run_verified(
            &Config {
                processing_neurons: 16,
                training_repetitions: 1,
                rest_us: 100000,
            },
            7,
            &Intervention { name: name.into() },
        )
        .unwrap()
}
#[test]
fn numerical_controls_isolate_rounding_bounds_null_signal_and_partition_loss() {
    let row = run("A_numeric");
    let data = &row.details["data"];
    assert_eq!(row.values["diagnostic_controls_passed"], 1.);
    assert_eq!(row.values["native_training_passes"], 0.);
    let cases = data["cases"].as_array().unwrap();
    let get = |name: &str| cases.iter().find(|r| r["case"] == name).unwrap();
    assert!(get("representable")["actual_signed_sum"].as_f64().unwrap() > 0.);
    assert_eq!(get("sub_ulp_interval")["actual_signed_sum"], 0.);
    assert!(
        get("sub_ulp_interval")["requested_signed_sum"]
            .as_f64()
            .unwrap()
            > 0.
    );
    assert_eq!(get("sub_ulp_interval")["clipped_steps"], 0);
    assert_eq!(get("upper_bound")["clipped_steps"], 1);
    assert_eq!(get("upper_bound")["rounded_away_steps"], 0);
    assert_eq!(get("zero_factor")["requested_signed_sum"], 0.);
    assert!(get("zero_factor")["tag_after_pair"].as_f64().unwrap() > 0.);
    assert_eq!(get("no_tag")["tag_after_pair"], 0.);
    let partitions = &data["partition_comparison"];
    assert!(partitions[0]["actual_signed_sum"].as_f64().unwrap() > 0.);
    assert_eq!(partitions[1]["actual_signed_sum"], 0.);
    assert_eq!(partitions[1]["rounded_away_steps"], 1000);
    assert!(
        partitions[1]["diagnostic_f64_accumulator"]
            .as_f64()
            .unwrap()
            > 0.5
    );
}
#[test]
fn artificial_memory_requires_the_cue_an_enabled_path_and_sufficient_weight() {
    let row = run("B_memory");
    let cases = row.details["data"]["cases"].as_array().unwrap();
    assert_eq!(row.values["diagnostic_controls_passed"], 1.);
    assert_eq!(row.values["local_tag_available"], -1.);
    for (index, case) in cases.iter().enumerate() {
        assert_eq!(case["target_fired"], index == 0);
        assert_eq!(case["weight_retained"], true);
        assert_eq!(case["direct_target_input"], false);
    }
    let spikes = cases[0]["spikes"].as_array().unwrap();
    let time = |id: u64| {
        spikes.iter().find(|s| s[0] == id).unwrap()[1]
            .as_u64()
            .unwrap()
    };
    assert_eq!(time(2) - time(1), 1000);
    assert_eq!(row.details["functional_memory_demonstrated"], false);
}
#[test]
fn local_controls_preserve_pairing_but_remove_factor_or_temporal_overlap() {
    let row = run("D_local");
    let cases = row.details["data"]["cases"].as_array().unwrap();
    assert_eq!(row.values["diagnostic_controls_passed"], 1.);
    assert_eq!(row.values["native_training_passes"], 7.);
    let get = |name: &str| cases.iter().find(|r| r["control"] == name).unwrap();
    let coupled = get("coincident");
    assert_eq!(coupled["pre_arrivals_us"], serde_json::json!([2000]));
    assert_eq!(coupled["post_spikes_us"], serde_json::json!([10000]));
    assert!(coupled["weight_delta"].as_f64().unwrap() > 0.);
    for control in cases.iter().skip(1) {
        assert_eq!(control["weight_delta"], 0.);
        assert_eq!(control["unmarked_neighbor_unchanged"], true);
    }
    assert_eq!(get("receptor_knockout")["nonzero_requests"], 0);
    assert_eq!(get("remote_modulator")["nonzero_requests"], 0);
    assert!(get("late_modulator")["nonzero_requests"].as_u64().unwrap() > 0);
    assert_eq!(get("late_modulator")["weight_delta"], 0.);
}
#[test]
fn fixed_gain_curve_reconstructs_matched_states_without_claiming_a_threshold() {
    let row = run("C_gain");
    let data = &row.details["data"];
    assert_eq!(row.values["diagnostic_controls_passed"], 1.);
    assert_eq!(row.values["native_training_passes"], 6.);
    let mut patterns = std::collections::BTreeMap::<u64, std::collections::BTreeSet<String>>::new();
    for window in data["training"]["windows"].as_array().unwrap() {
        patterns
            .entry(window["image"].as_u64().unwrap())
            .or_default()
            .insert(window["input_channel_counts"].to_string());
    }
    let disjoint = patterns[&0].is_disjoint(&patterns[&1]);
    assert_eq!(data["training"]["input_patterns_disjoint"], disjoint);
    assert_eq!(
        row.values["input_discriminable"],
        if disjoint { 1. } else { 0. }
    );
    let doses = data["doses"].as_array().unwrap();
    assert_eq!(doses.len(), 5);
    assert_eq!(
        doses
            .iter()
            .map(|r| r["gain"].as_f64().unwrap())
            .collect::<Vec<_>>(),
        vec![0., 1., 10., 100., 1000.]
    );
    assert_eq!(doses[0]["installed_l1_from_initial"], 0.);
    assert_eq!(doses[0]["core_effect"], false);
    assert!(
        doses
            .iter()
            .all(|d| d["matched_checkpoint"] == true && d["only_weights_changed"] == true)
    );
    let bench = data["bench_weight_curve"].as_array().unwrap();
    assert_eq!(bench[0]["target_fired"], false);
    assert_eq!(bench[3]["target_fired"], true);
    assert_eq!(row.details["functional_memory_demonstrated"], false);
    assert!(data.get("first_sampled_core_effect_gain").is_some());
    // Only sampled gains are permitted; no extrapolated or unbounded estimate.
    if data["first_sampled_core_effect_gain"] != Value::Null {
        assert!(doses.iter().any(
            |d| d["gain"] == data["first_sampled_core_effect_gain"] && d["core_effect"] == true
        ));
    }
}
