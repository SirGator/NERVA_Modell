//! External, fixed pixel probe. Labels and scores never enter neural learning.
use crate::{
    Result, error, fingerprint_debug,
    scenario::{Intervention, Scenario, TrialMetrics},
};
use nerva_qemu::{capture::Frame, vision_encoder::Encoding};
use nerva_vm::model::{
    Architecture, KeyboardLayout, Model, ModelConfig, PlasticitySettings, SensorConfig, SensorMode,
    SpikeCoding, audit::learned_parameters_equal,
};
use serde_json::{Value, json};
use std::collections::BTreeMap;

pub mod association;
pub mod causal;
pub mod chain;
pub mod precision;
mod weights;
use weights::{snapshot_json, weight_changes, weight_snapshot};

pub const GROUPS: [&str; 5] = ["cortical", "smooth", "sharp", "no_chemistry", "frozen"];
pub const PARAMETERS: [&str; 3] = ["processing_neurons", "training_repetitions", "rest_us"];
pub struct HippocampalScenario;
#[derive(Clone, Debug)]
pub struct Config {
    pub processing_neurons: usize,
    pub training_repetitions: usize,
    pub rest_us: u64,
}
fn fail(e: impl std::fmt::Display) -> crate::ExperimentError {
    error(e.to_string())
}
fn model_config(c: &Config, seed: u64, group: &str) -> ModelConfig {
    ModelConfig {
        seed,
        processing_neurons: c.processing_neurons,
        keyboard: KeyboardLayout::Wasd,
        encoding: Encoding::Brightness,
        development: false,
        homeostasis: false,
        feedback: false,
        architecture: if group == "cortical" {
            Architecture::Cortical
        } else {
            Architecture::Hippocampal
        },
        neuromodulation: !matches!(group, "cortical" | "no_chemistry"),
        area_transition_width: if group == "sharp" { 0. } else { 0.18 },
        ..ModelConfig::default()
    }
}
fn pixels(kind: usize) -> Vec<u8> {
    let common = [(1, 1), (2, 1), (1, 2), (2, 2), (1, 3), (2, 3)];
    let a = [(5, 5), (6, 5)];
    let b = [(5, 1), (6, 1)];
    let mut values = vec![0u8; 64];
    let points: Vec<_> = match kind {
        0 => common.into_iter().chain(a).collect(),
        1 => common.into_iter().chain(b).collect(),
        2 => a.to_vec(),
        3 => b.to_vec(),
        _ => Vec::new(),
    };
    for (x, y) in points {
        values[y * 8 + x] = 255;
    }
    values
}
fn frame(kind: usize) -> nerva_qemu::Result<Frame> {
    Frame::new(
        8,
        8,
        pixels(kind).into_iter().flat_map(|v| [v; 3]).collect(),
    )
}
fn cosine(a: &[usize], b: &[usize]) -> f64 {
    let dot: f64 = a.iter().zip(b).map(|(&x, &y)| x as f64 * y as f64).sum();
    let aa: f64 = a.iter().map(|&x| (x as f64).powi(2)).sum();
    let bb: f64 = b.iter().map(|&x| (x as f64).powi(2)).sum();
    // Silence receives no similarity credit. It is also explicitly gated below.
    if aa == 0. || bb == 0. {
        0.
    } else {
        (dot / (aa * bb).sqrt()).clamp(0., 1.)
    }
}
fn summarize(responses: &[Vec<usize>]) -> Value {
    let margins = [
        cosine(&responses[2], &responses[0]) - cosine(&responses[2], &responses[1]),
        cosine(&responses[3], &responses[1]) - cosine(&responses[3], &responses[0]),
    ];
    json!({"core_counts":responses, "completion_margins":margins,
        "separation":1.-cosine(&responses[0], &responses[1]),
        "all_responses_active":responses.iter().all(|v| v.iter().sum::<usize>()>0)})
}
fn probes(model: &mut Model, c: &Config) -> Result<Value> {
    model
        .set_plasticity(PlasticitySettings {
            learning: false,
            development: false,
            homeostasis: false,
        })
        .map_err(fail)?;
    let before = model.network().clone();
    let black = frame(4).map_err(fail)?;
    let mut responses = Vec::new();
    let mut rest_spikes = Vec::new();
    for kind in 0..4 {
        let mut remaining = c.rest_us;
        let mut spikes = 0;
        while remaining > 0 {
            let duration = remaining.min(nerva_vm::model::WINDOW_US);
            let rest = model.step_for(&black, duration).map_err(fail)?;
            spikes += rest.core_counts.iter().sum::<usize>();
            remaining -= duration;
        }
        rest_spikes.push(spikes);
        responses.push(
            model
                .step(&frame(kind).map_err(fail)?)
                .map_err(fail)?
                .core_counts,
        );
    }
    let mut result = summarize(&responses);
    result["rest_core_spikes"] = json!(rest_spikes);
    result["learned_parameters_preserved"] =
        json!(learned_parameters_equal(&before, model.network()));
    if result["learned_parameters_preserved"] != true {
        return Err(error("probe changed learned parameters"));
    }
    Ok(result)
}
impl Scenario for HippocampalScenario {
    type Config = Config;
    fn name(&self) -> &'static str {
        "hippocampal_probe"
    }
    fn default_config(&self) -> Config {
        Config {
            processing_neurons: 128,
            training_repetitions: 8,
            rest_us: 11_000_000,
        }
    }
    fn set_parameter(&self, c: &mut Config, name: &str, v: &Value) -> Result<()> {
        let n = v
            .as_u64()
            .ok_or_else(|| error("parameter must be an unsigned integer"))?;
        match name {
            "processing_neurons" => c.processing_neurons = usize::try_from(n).map_err(fail)?,
            "training_repetitions" => c.training_repetitions = usize::try_from(n).map_err(fail)?,
            "rest_us" => c.rest_us = n,
            _ => return Err(error(format!("unknown hippocampal parameter: {name}"))),
        }
        Ok(())
    }
    fn validate(&self, c: &Config) -> Result<()> {
        if !(16..=1024).contains(&c.processing_neurons)
            || !(1..=128).contains(&c.training_repetitions)
            || !(100_000..=20_000_000).contains(&c.rest_us)
        {
            return Err(error(
                "hippocampal probe budget: 16..1024 cells, 1..128 repetitions, 0.1..20s rest",
            ));
        }
        for group in GROUPS {
            model_config(c, 42, group).validate().map_err(fail)?;
        }
        Ok(())
    }
    fn snapshot(&self, c: &Config, seed: u64, i: &Intervention) -> Value {
        json!({"protocol":"NERVA-Hippocampal-Probe/2", "model":format!("{:?}",model_config(c,seed,&i.name)),
            "processing_neurons":c.processing_neurons,"training_repetitions":c.training_repetitions,
            "rest_us":c.rest_us,"input":"8x8 brightness, deterministic120Hz, 50ms",
            "pattern_full_a":pixels(0),"pattern_full_b":pixels(1),
            "pattern_partial_a":pixels(2),"pattern_partial_b":pixels(3),
            "probe_order":["full_a","full_b","partial_a","partial_b"]})
    }
    fn interventions(&self) -> Vec<Intervention> {
        GROUPS
            .iter()
            .map(|&name| Intervention { name: name.into() })
            .collect()
    }
    fn metric_names(&self) -> Vec<&'static str> {
        vec![
            "completion_a",
            "completion_b",
            "acquisition_a",
            "acquisition_b",
            "separation",
            "all_responses_active",
            "weight_updates",
            "training_weight_l1",
            "training_weight_max_delta",
            "training_changed_synapses",
            "training_core_spikes",
            "parameters_preserved",
        ]
    }
    fn run(&self, c: &Config, seed: u64, i: &Intervention) -> Result<TrialMetrics> {
        self.validate(c)?;
        if !GROUPS.contains(&i.name.as_str()) {
            return Err(error("unknown hippocampal control"));
        }
        let mut model = Model::with_sensor(
            model_config(c, seed, &i.name),
            SensorConfig {
                mode: SensorMode::Pixels,
                resolution: 8,
                coding: SpikeCoding::Deterministic,
                max_rate_hz: 120.,
            },
        )
        .map_err(fail)?;
        let initial = fingerprint_debug(model.network());
        let before = probes(&mut model, c)?;
        let training_weights_before = weight_snapshot(&model);
        let training_start_us = model.next_observation_us();
        model
            .set_plasticity(PlasticitySettings {
                learning: i.name != "frozen",
                development: false,
                homeostasis: false,
            })
            .map_err(fail)?;
        let mut updates = 0usize;
        let mut spikes = 0usize;
        let mut schedule = Vec::new();
        let black = frame(4).map_err(fail)?;
        for trial in 0..c.training_repetitions {
            let order = if (seed ^ trial as u64) & 1 == 0 {
                [0, 1]
            } else {
                [1, 0]
            };
            for kind in order {
                schedule.push(kind);
                for image in [&frame(kind).map_err(fail)?, &black] {
                    let step = model.step(image).map_err(fail)?;
                    updates += step.weight_changes;
                    spikes += step.core_counts.iter().sum::<usize>();
                }
            }
        }
        let training_end_us = model.next_observation_us();
        let training_weights_after = weight_snapshot(&model);
        let weight_changes = weight_changes(
            &training_weights_before,
            &training_weights_after,
            model.network(),
        )?;
        let local_factors: Vec<_> = model
            .network()
            .neurons()
            .map(|cell| (cell.id().0, f64::from(cell.modulation().learning_signal)))
            .collect();
        let after = probes(&mut model, c)?;
        let get = |phase: &Value, cue: usize| phase["completion_margins"][cue].as_f64().unwrap();
        let a = get(&after, 0);
        let b = get(&after, 1);
        let values = [
            a,
            b,
            a - get(&before, 0),
            b - get(&before, 1),
            after["separation"].as_f64().unwrap(),
            if after["all_responses_active"] == true {
                1.
            } else {
                0.
            },
            updates as f64,
            weight_changes["all"]["l1"].as_f64().unwrap(),
            weight_changes["all"]["maximum"].as_f64().unwrap(),
            weight_changes["all"]["changed"].as_u64().unwrap() as f64,
            spikes as f64,
            1.,
        ];
        Ok(TrialMetrics {
            values: self
                .metric_names()
                .into_iter()
                .map(str::to_owned)
                .zip(values)
                .collect(),
            details: json!({"initial_network_digest":initial,"final_network_digest":fingerprint_debug(model.network()),
                "before":before,"after":after,"training_schedule":schedule,
                "training_weights":{"row_fields":["synapse_id","pre","post","weight_f64_from_f32","stored_learning_tag_f64_from_f32","tag_updated_at_us"],
                    "before":snapshot_json(&training_weights_before),"after":snapshot_json(&training_weights_after),"changes":weight_changes,
                    "start_us":training_start_us,"end_us":training_end_us,"end_local_learning_factors":local_factors},
                "limits":"Fixed external native-count similarity measurement; no labels or score reach the model. STP/chemical states are not transplanted/reset. A candidate advantage is not isolated weight memory or validated cognition."}),
        })
    }
    fn evaluate(&self, records: &[Value], replay: bool) -> Result<Value> {
        let mut paired = BTreeMap::<u64, BTreeMap<String, &Value>>::new();
        for row in records {
            paired
                .entry(
                    row["job"]["seed"]
                        .as_u64()
                        .ok_or_else(|| error("missing seed"))?,
                )
                .or_default()
                .insert(
                    row["job"]["intervention"]
                        .as_str()
                        .ok_or_else(|| error("missing control"))?
                        .into(),
                    row,
                );
        }
        let mut candidates = Vec::new();
        for (&seed, groups) in &paired {
            let complete = GROUPS.iter().all(|g| groups.contains_key(*g));
            let passed = complete && {
                let smooth = &groups["smooth"]["metrics"];
                smooth["all_responses_active"] == 1.
                    && ["a", "b"].iter().all(|cue| {
                        let m = format!("completion_{cue}");
                        let a = format!("acquisition_{cue}");
                        smooth[&m].as_f64().unwrap() > 0.
                            && smooth[&a].as_f64().unwrap() > 0.
                            && smooth[&m].as_f64().unwrap()
                                >= groups["sharp"]["metrics"][&m].as_f64().unwrap()
                            && ["cortical", "frozen", "no_chemistry"].iter().all(|g| {
                                smooth[&a].as_f64().unwrap()
                                    > groups[*g]["metrics"][&a].as_f64().unwrap()
                            })
                    })
                    && smooth["separation"].as_f64().unwrap()
                        >= groups["cortical"]["metrics"]["separation"]
                            .as_f64()
                            .unwrap()
                    && smooth["separation"].as_f64().unwrap()
                        >= groups["sharp"]["metrics"]["separation"].as_f64().unwrap()
            };
            candidates
                .push(json!({"seed":seed,"all_controls":complete,"candidate_gate_passed":passed}));
        }
        Ok(
            json!({"status":"exploratory_native_pattern_probe","passed":null,
            "candidate_gate_passed":replay && paired.len()>=2 && candidates.iter().all(|v|v["candidate_gate_passed"]==true),
            "functional_memory_demonstrated":false,"per_seed":candidates,
            "limits":"No independent confirmation, weight-storage intervention or external-world memory task; The candidate gate also requires advantage over no-chemistry and no worse completion/separation than sharp profiles; this is not established biological function."}),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn partial_images_keep_only_their_own_unique_pixels() {
        let images: Vec<_> = (0..4).map(pixels).collect();
        assert_eq!(images[0].iter().filter(|&&v| v > 0).count(), 8);
        assert_eq!(images[1].iter().filter(|&&v| v > 0).count(), 8);
        for partial in 2..4 {
            assert_eq!(images[partial].iter().filter(|&&v| v > 0).count(), 2);
            for (p, &value) in images[partial].iter().enumerate() {
                if value > 0 {
                    assert_eq!(images[partial - 2][p], 255);
                    assert_eq!(images[3 - partial][p], 0);
                }
            }
        }
    }
    #[test]
    fn silent_or_one_sided_responses_cannot_pass_candidate_gate() {
        let s = HippocampalScenario;
        assert_eq!(
            s.evaluate(&[], true).unwrap()["candidate_gate_passed"],
            false
        );
        assert_eq!(cosine(&[0, 0], &[0, 0]), 0.);
        assert_eq!(
            summarize(&vec![vec![0, 0]; 4])["all_responses_active"],
            false
        );
    }
    #[test]
    fn candidate_gate_requires_both_cues_chemical_control_and_sharp_noninferiority() {
        let scenario = HippocampalScenario;
        let mut rows = Vec::new();
        for seed in [7, 11] {
            for group in GROUPS {
                let (completion, acquisition, separation) = match group {
                    "smooth" => (0.5, 0.2, 0.6),
                    "sharp" => (0.4, 0.1, 0.5),
                    _ => (0.1, 0.0, 0.2),
                };
                rows.push(json!({"job":{"seed":seed,"intervention":group},"metrics":{
                    "completion_a":completion,"completion_b":completion,"acquisition_a":acquisition,
                    "acquisition_b":acquisition,"separation":separation,"all_responses_active":1.}}));
            }
        }
        assert_eq!(
            scenario.evaluate(&rows, true).unwrap()["candidate_gate_passed"],
            true
        );
        assert_eq!(
            scenario.evaluate(&rows, false).unwrap()["candidate_gate_passed"],
            false
        );
        for (group, metric, value) in [
            ("smooth", "acquisition_b", -0.1),
            ("smooth", "all_responses_active", 0.),
            ("no_chemistry", "acquisition_a", 0.3),
            ("sharp", "completion_a", 0.6),
            ("sharp", "separation", 0.7),
        ] {
            let mut broken = rows.clone();
            let row = broken
                .iter_mut()
                .find(|r| r["job"]["intervention"] == group)
                .unwrap();
            row["metrics"][metric] = json!(value);
            assert_eq!(
                scenario.evaluate(&broken, true).unwrap()["candidate_gate_passed"],
                false,
                "{group}/{metric}"
            );
        }
    }
    #[test]
    fn full_producer_splits_long_rest_into_valid_model_steps_and_replays() {
        let scenario = HippocampalScenario;
        let config = Config {
            processing_neurons: 16,
            training_repetitions: 1,
            rest_us: 100_000,
        };
        for name in ["cortical", "smooth"] {
            let result = scenario
                .run_verified(&config, 17, &Intervention { name: name.into() })
                .unwrap();
            assert_eq!(result.values["parameters_preserved"], 1.);
            for phase in ["before", "after"] {
                assert_eq!(
                    result.details[phase]["rest_core_spikes"]
                        .as_array()
                        .unwrap()
                        .len(),
                    4
                );
                assert_eq!(result.details[phase]["learned_parameters_preserved"], true);
            }
        }
    }
}
