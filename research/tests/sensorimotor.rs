use nerva_research::{
    scenario::{Intervention, Scenario},
    scenarios::sensorimotor::{GROUPS, SensorimotorScenario},
};
use serde_json::json;

fn sizes() -> Vec<usize> {
    std::env::var("NERVA_TEST_SIZES")
        .unwrap_or_else(|_| "1,17,65,257,1025".into())
        .split(',')
        .map(|s| {
            s.trim()
                .parse::<usize>()
                .expect("NERVA_TEST_SIZES requires positive integer sizes")
        })
        .inspect(|&n| assert!(n > 0, "network size must be positive"))
        .collect()
}

#[test]
fn paired_worlds_frozen_controls_and_forecasts_replay_exactly() {
    let scenario = SensorimotorScenario;
    for size in sizes() {
        let mut config = scenario.default_config();
        config.processing_neurons = size;
        config.max_total_neurons = size.checked_add(11).unwrap();
        config.max_observed_events = 10_000_000;
        config.train_trials = 4;
        config.test_trials = 4;
        scenario.validate(&config).unwrap();
        let mut initial = None;
        let mut images = None;
        for name in GROUPS {
            let result = scenario
                .run_verified(&config, 17, &Intervention { name: name.into() })
                .unwrap();
            assert_eq!(result.values["processing_neurons"], size as f64);
            assert_eq!(result.values["total_neurons"], (size + 11) as f64);
            assert_eq!(
                result.values["core_inhibitory_fraction"],
                (size / 5) as f64 / size as f64
            );
            assert!(
                result.values["initial_synapses"]
                    <= (size * 4 + 7 * size.min(4) + 2 * size.min(8) + 4 * size.min(3)) as f64
            );
            assert_eq!(result.values["parameters_preserved"], 1.);
            let digest = result.details["initial_network_digest"].clone();
            if let Some(expected) = &initial {
                assert_eq!(&digest, expected);
            } else {
                initial = Some(digest);
            }
            let rows: Vec<_> = result.details["training"]
                .as_array()
                .unwrap()
                .iter()
                .map(|r| (r["before_frame"].clone(), r["after_frame"].clone()))
                .collect();
            if let Some(expected) = &images {
                assert_eq!(&rows, expected);
            } else {
                images = Some(rows);
            }
            if name.ends_with("_frozen") {
                for metric in ["weight_updates", "homeostasis_updates", "formed", "pruned"] {
                    assert_eq!(result.values[metric], 0.);
                }
            }
            if name.starts_with("passive") {
                assert_eq!(result.values["action_copy_arrivals"], 0.);
            } else {
                assert!(result.values["action_copy_arrivals"] > 0.);
            }
            for row in result.details["probes"].as_array().unwrap() {
                assert!(row["forecast_us"].as_u64().unwrap() < row["effect_us"].as_u64().unwrap());
                assert_eq!(
                    row["effect_us"].as_u64().unwrap() - row["action_us"].as_u64().unwrap(),
                    config.action_delay_us
                );
                assert!((0..7).contains(&row["predicted_position"].as_i64().unwrap()));
            }
            if name.starts_with("decoupled") {
                let mut pairings = std::collections::BTreeMap::new();
                for row in result.details["training"].as_array().unwrap() {
                    *pairings
                        .entry((row["key"].to_string(), row["physical_key"].to_string()))
                        .or_insert(0) += 1;
                }
                assert_eq!(pairings.len(), 4);
                assert!(pairings.values().all(|&n| n == 1));
            }
        }
    }
}

#[test]
fn diagnosis_switches_apply_identically_at_every_size() {
    let scenario = SensorimotorScenario;
    for size in sizes() {
        let mut initial = None;
        for development in [false, true] {
            for homeostasis in [false, true] {
                let mut c = scenario.default_config();
                c.processing_neurons = size;
                c.max_total_neurons = size + 11;
                c.max_observed_events = 10_000_000;
                c.train_trials = 4;
                c.test_trials = 4;
                scenario
                    .set_parameter(&mut c, "mechanisms.development", &json!(development))
                    .unwrap();
                scenario
                    .set_parameter(&mut c, "mechanisms.homeostasis", &json!(homeostasis))
                    .unwrap();
                let r = scenario
                    .run_verified(
                        &c,
                        17,
                        &Intervention {
                            name: "active".into(),
                        },
                    )
                    .unwrap();
                if !development {
                    assert_eq!(r.values["formed"], 0.);
                    assert_eq!(r.values["pruned"], 0.);
                }
                if !homeostasis {
                    assert_eq!(r.values["homeostasis_updates"], 0.);
                }
                let digest = r.details["initial_network_digest"].clone();
                if let Some(expected) = &initial {
                    assert_eq!(&digest, expected);
                } else {
                    initial = Some(digest);
                }
            }
        }
    }
}

#[test]
fn indexed_scaling_preserves_the_pre_change_reference_dynamics() {
    let scenario = SensorimotorScenario;
    for (size, initial, trained, final_digest) in [
        (
            17,
            "8fd22becc219d623",
            "f3d8367268cc2dcc",
            "979aa15c66f44f2e",
        ),
        (
            256,
            "07f78159dc37360f",
            "457c991f767554c3",
            "b8f0e55f4c8d420d",
        ),
    ] {
        let mut c = scenario.default_config();
        c.processing_neurons = size;
        c.train_trials = 4;
        c.test_trials = 4;
        let r = scenario
            .run(
                &c,
                17,
                &Intervention {
                    name: "active".into(),
                },
            )
            .unwrap();
        assert_eq!(r.details["initial_network_digest"], initial);
        assert_eq!(r.details["trained_network_digest"], trained);
        assert_eq!(r.details["final_network_digest"], final_digest);
    }
}

#[test]
fn sizes_parameters_and_event_budget_are_enforced() {
    let scenario = SensorimotorScenario;
    for (name, value) in [
        ("processing_neurons", json!(0)),
        ("train_trials", json!(5)),
        ("action_delay_us", json!(0)),
    ] {
        let mut c = scenario.default_config();
        scenario.set_parameter(&mut c, name, &value).unwrap();
        assert!(scenario.validate(&c).is_err());
    }
    let mut c = scenario.default_config();
    assert!(
        scenario
            .set_parameter(&mut c, "processing_neurons", &json!(256.0))
            .is_err()
    );
    assert!(
        scenario
            .set_parameter(&mut c, "mechanisms.stdp", &json!(true))
            .is_err()
    );
    c.processing_neurons = 1024;
    c.train_trials = 4;
    c.test_trials = 4;
    scenario.validate(&c).unwrap();
    let result = scenario
        .run_verified(
            &c,
            29,
            &Intervention {
                name: "active_frozen".into(),
            },
        )
        .unwrap();
    assert_eq!(result.values["total_neurons"], 1035.);
    assert!(result.values["initial_synapses"] > 4096.);
    c.max_observed_events = 1;
    let error = scenario
        .run(
            &c,
            29,
            &Intervention {
                name: "active".into(),
            },
        )
        .unwrap_err();
    assert!(error.to_string().contains("event budget"));
    c.max_observed_events = 2_000_000;
    c.max_total_neurons = 100;
    let error = scenario
        .run(
            &c,
            29,
            &Intervention {
                name: "active".into(),
            },
        )
        .unwrap_err();
    assert!(error.to_string().contains("max_total_neurons"));
    scenario
        .set_parameter(&mut c, "mechanisms.homeostasis", &json!(1))
        .unwrap_err();
}

#[test]
fn homeostasis_populations_are_audited_at_every_requested_size() {
    let scenario = SensorimotorScenario;
    for size in sizes() {
        let mut initial = None;
        let mut root_rate = None;
        let mut frozen_endpoints = None;
        for scope in ["off", "core", "core_sensor", "all"] {
            for group in ["active", "active_frozen"] {
                let mut c = scenario.default_config();
                c.processing_neurons = size;
                c.max_total_neurons = size + 11;
                c.max_observed_events = 10_000_000;
                c.train_trials = 4;
                c.test_trials = 4;
                c.development = false;
                scenario
                    .set_parameter(&mut c, "homeostasis.scope", &json!(scope))
                    .unwrap();
                let r = scenario
                    .run_verified(&c, 17, &Intervention { name: group.into() })
                    .unwrap();
                let frozen = group.ends_with("_frozen");
                let allowed = if frozen {
                    [false; 4]
                } else {
                    match scope {
                        "off" => [false; 4],
                        "core" => [false, true, false, false],
                        "core_sensor" => [true, true, false, false],
                        _ => [true; 4],
                    }
                };
                let mut total = 0.;
                for (i, population) in ["sensor", "core", "motor", "copy"].iter().enumerate() {
                    let updates = r.values[&format!("{population}_homeostasis_updates")];
                    total += updates;
                    if allowed[i] {
                        assert!(updates > 0.);
                    } else {
                        assert_eq!(updates, 0.);
                        assert_eq!(
                            r.details["intrinsic_currents"]["trained"][population]["min"],
                            0.
                        );
                        assert_eq!(
                            r.details["intrinsic_currents"]["trained"][population]["max"],
                            0.
                        );
                    }
                }
                assert_eq!(total, r.values["homeostasis_updates"]);
                assert_eq!(
                    r.details["intrinsic_currents"]["trained"],
                    r.details["intrinsic_currents"]["final"]
                );
                let digest = r.details["initial_network_digest"].clone();
                if let Some(expected) = &initial {
                    assert_eq!(&digest, expected);
                } else {
                    initial = Some(digest);
                }
                let rate = r.values["root_sensor_rate_hz"];
                if let Some(expected) = root_rate {
                    assert_eq!(rate, expected);
                } else {
                    root_rate = Some(rate);
                }
                if frozen {
                    let endpoints = (
                        r.values["visual_accuracy"],
                        r.values["position_mae"],
                        r.values["sensor_rate_hz"],
                        r.values["core_rate_hz"],
                    );
                    if let Some(expected) = frozen_endpoints {
                        assert_eq!(endpoints, expected);
                    } else {
                        frozen_endpoints = Some(endpoints);
                    }
                }
            }
        }
    }
}

#[test]
fn scope_values_are_explicit_and_do_not_accept_invalid_types() {
    let scenario = SensorimotorScenario;
    let mut c = scenario.default_config();
    for value in [
        json!(true),
        json!(1),
        json!("sensors_only"),
        json!("Core"),
        json!(""),
    ] {
        assert!(
            scenario
                .set_parameter(&mut c, "homeostasis.scope", &value)
                .is_err()
        );
    }
}
