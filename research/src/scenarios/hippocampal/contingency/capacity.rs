//! Bound recurrent cue-to-outcome capacity with a fixed diagnostic intervention.
use super::super::factorial::weights::{norm, plan};
use super::{
    CUES, Config, ContingencyScenario, FRAGMENTS, Model, OUTCOMES, Response, Weights, checkpoint,
    frames, get_weights, probe, schedule, serialized, stimuli, train, verify_only_weights,
};
use crate::{
    Result, error,
    scenario::{Intervention, Scenario, TrialMetrics},
};
use nerva::{core::Polarity, core::SynapseId};
use nerva_qemu::capture::Frame;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

mod eligibility;
mod homeostasis;
mod routes;
mod structural;
pub use eligibility::replay_eligibility;
pub use homeostasis::replay_homeostasis;
use routes::{Edge, Routes, routes};
pub use structural::{
    replay_grown_dose, replay_grown_residual, replay_structural, replay_structural_consolidation,
    replay_structural_decisions, replay_structural_exploration,
};

pub struct CapacityScenario;
const PROTOCOL: &str = "NERVA-Contingency-Capacity/1";
const MAPPINGS: [&str; 2] = ["forward", "reverse"];
const CASES: [(&str, bool, f32); 4] = [
    ("direct_half", false, 0.5),
    ("direct_full", false, 1.),
    ("relay_half", true, 0.5),
    ("relay_full", true, 1.),
];
const METRICS: [&str; 3] = [
    "best_reference_cases",
    "full_reference",
    "full_native_at_reference",
];
type Responses = BTreeMap<&'static str, Response>;
type Pair = [Weights; 2];

struct Assay<'a> {
    c: &'a Config,
    seed: u64,
    reference: Model,
    frames: BTreeMap<&'static str, Frame>,
    edges: Vec<Edge>,
    core: BTreeSet<u64>,
    initial: Weights,
}
impl Assay<'_> {
    fn topology(&self) -> Vec<(u64, u64, u64, f32, bool)> {
        self.edges
            .iter()
            .map(|e| (e.id, e.pre, e.post, e.initial, e.eligible))
            .collect()
    }

    fn core_weights(&self, weights: &Weights) -> Result<Weights> {
        if !self
            .initial
            .iter()
            .map(|x| x.0)
            .eq(weights.iter().map(|x| x.0))
        {
            return Err(error("capacity training topology differs"));
        }
        Ok(self
            .edges
            .iter()
            .zip(weights)
            .map(|(edge, &value)| {
                if self.core.contains(&edge.pre) && self.core.contains(&edge.post) {
                    value
                } else {
                    (value.0, edge.initial)
                }
            })
            .collect())
    }

    fn response(&self, weights: &Weights, name: &str) -> Result<Response> {
        let (mut branch, _) = checkpoint(self.c, self.seed)?;
        verify_only_weights(&self.reference, &mut branch, weights)?;
        if get_weights(&branch) != *weights {
            return Err(error("capacity arm installed different weights"));
        }
        probe(
            &mut branch,
            self.c,
            &self.frames[name],
            &self.frames["blank"],
            None,
        )
    }

    fn observe(&self, weights: &Weights, names: &[&'static str]) -> Result<Responses> {
        names
            .iter()
            .map(|&name| Ok((name, self.response(weights, name)?)))
            .collect()
    }

    fn compare(
        &self,
        weights: &Pair,
        reset: &Responses,
        targets: &[BTreeSet<u64>; 2],
        cues: &[&'static str],
    ) -> Result<(Value, usize)> {
        let names = cues.iter().copied().chain(["blank"]).collect::<Vec<_>>();
        let a = self.observe(&weights[0], &names)?;
        let b = self.observe(&weights[1], &names)?;
        for arm in [&a, &b] {
            if arm.iter().any(|(&name, p)| {
                p.root_spikes != reset[name].root_spikes || p.start_us != reset[name].start_us
            }) {
                return Err(error("capacity arm changed cue input"));
            }
        }
        let mut passes = 0;
        let mut arms = BTreeMap::new();
        for (flip, (arm, other)) in [(&a, &b), (&b, &a)].into_iter().enumerate() {
            let mut checks = BTreeMap::new();
            for (cue, &name) in cues.iter().enumerate() {
                let expected = (cue / (cues.len() / 2)) ^ flip;
                let correct = spike_count(&arm[name], &targets[expected]);
                let wrong = spike_count(&arm[name], &targets[1 - expected]);
                let base = spike_count(&reset[name], &targets[expected]);
                let opposite = spike_count(&other[name], &targets[expected]);
                let pass = correct > wrong.max(base).max(opposite)
                    && arm["blank"].core_spikes.len() <= reset["blank"].core_spikes.len()
                    && arm[name].rest_core_spikes <= reset[name].rest_core_spikes;
                passes += usize::from(pass);
                checks.insert(name, json!({"correct":correct,"incorrect":wrong,"reset":base,"opposite":opposite,"passed":pass}));
            }
            arms.insert(MAPPINGS[flip], json!({"weights":serialized(&weights[flip]),"responses":json_responses(arm),"checks":checks}));
        }
        Ok((json!({"arms":arms,"passed_cases":passes}), passes))
    }

    fn learned(&self, flip: usize) -> Result<(Weights, Value)> {
        let (mut branch, _) = checkpoint(self.c, self.seed)?;
        if !self.reference.same_replay_state(&branch) {
            return Err(error("capacity training checkpoint differs"));
        }
        let history = train(
            &mut branch,
            &schedule(self.c, self.seed, MAPPINGS[flip], 0),
            &self.frames,
            None,
        )?;
        let final_weights = get_weights(&branch);
        let raw = json!({"frames":history,"weights":serialized(&final_weights)});
        Ok((self.core_weights(&final_weights)?, raw))
    }

    fn normalize(
        &self,
        direction: &Pair,
        target_norm: &Pair,
        reset: &Responses,
        targets: &[BTreeSet<u64>; 2],
    ) -> Result<(Value, usize)> {
        let plans = std::array::from_fn::<_, 2, _>(|i| {
            plan(
                &self.initial,
                &delta(&self.initial, &direction[i]),
                norm(&delta(&self.initial, &target_norm[i])),
            )
        });
        let [a, b] = plans;
        let (score, passes) = match (a.weights, b.weights) {
            (Some(a_w), Some(b_w))
                if a.details["faithful"] == true && b.details["faithful"] == true =>
            {
                self.compare(&[a_w, b_w], reset, targets, &CUES)?
            }
            _ => (Value::Null, 0),
        };
        Ok((
            json!({"plans":{"forward":a.details,"reverse":b.details},"comparison":score}),
            passes,
        ))
    }
}

fn spike_count(response: &Response, group: &BTreeSet<u64>) -> usize {
    response
        .core_spikes
        .iter()
        .filter(|(id, _)| group.contains(id))
        .count()
}
fn json_responses(responses: &Responses) -> BTreeMap<&'static str, Value> {
    responses
        .iter()
        .map(|(&name, response)| (name, response.json()))
        .collect()
}
fn active(reset: &Responses, names: &[&str]) -> BTreeSet<u64> {
    names
        .iter()
        .flat_map(|name| reset[*name].core_spikes.iter().map(|&(id, _)| id))
        .collect()
}
fn delta(initial: &Weights, installed: &Weights) -> Vec<f64> {
    initial
        .iter()
        .zip(installed)
        .map(|(&(_, a), &(_, b))| f64::from(b) - f64::from(a))
        .collect()
}
fn reference_weights(edges: &[Edge], routes: &[BTreeSet<u64>; 2], strength: f32) -> Weights {
    edges
        .iter()
        .map(|edge| {
            let weight = if routes.iter().any(|path| path.contains(&edge.id)) {
                edge.initial + strength * (2. - edge.initial)
            } else {
                edge.initial
            };
            (SynapseId(edge.id), weight)
        })
        .collect()
}

fn build(c: &Config, seed: u64) -> Result<(Assay<'_>, Value, BTreeSet<u64>)> {
    let (reference, development) = checkpoint(c, seed)?;
    let core = reference
        .processing_ids()
        .iter()
        .map(|id| id.0)
        .collect::<BTreeSet<_>>();
    let excitatory = reference
        .network()
        .neurons()
        .filter(|n| core.contains(&n.id().0) && n.polarity() == Polarity::Excitatory)
        .map(|n| n.id().0)
        .collect::<BTreeSet<_>>();
    let edges = reference
        .network()
        .synapses()
        .map(|s| Edge {
            id: s.id().0,
            pre: s.pre().0,
            post: s.post().0,
            initial: s.weight().get(),
            eligible: excitatory.contains(&s.pre().0)
                && core.contains(&s.post().0)
                && s.is_plastic()
                && s.is_enabled()
                && s.weight().get() < 2.,
        })
        .collect();
    let patterns = stimuli();
    let initial = get_weights(&reference);
    let assay = Assay {
        c,
        seed,
        reference,
        frames: frames(&patterns)?,
        edges,
        core,
        initial,
    };
    Ok((assay, development, excitatory))
}

fn run(c: &Config, seed: u64) -> Result<TrialMetrics> {
    let (assay, development, excitatory) = build(c, seed)?;
    let reset = assay.observe(
        &assay.initial,
        &["blank", CUES[0], CUES[1], OUTCOMES[0], OUTCOMES[1]],
    )?;
    let nearby = active(&reset, &["blank", CUES[0], CUES[1]]);
    let outcome = OUTCOMES.map(|name| active(&reset, &[name]));
    let targets = std::array::from_fn::<_, 2, _>(|i| {
        outcome[i]
            .iter()
            .filter(|id| !nearby.contains(id) && !outcome[1 - i].contains(id))
            .copied()
            .collect::<BTreeSet<_>>()
    });
    let quiet = excitatory
        .iter()
        .filter(|id| !nearby.contains(id) && !targets.iter().any(|set| set.contains(id)))
        .copied()
        .collect();
    let arrivals = CUES.map(|name| reset[name].arrived_edges.clone());
    let route_sets = [false, true].map(|reverse| {
        routes(
            &assay.edges,
            &arrivals,
            &reset["blank"].arrived_edges,
            &quiet,
            &targets,
            reverse,
        )
    });
    let mut rows = BTreeMap::new();
    let mut best = None;
    let mut best_score = 0;
    for (name, relay, strength) in CASES {
        let selected = route_sets
            .iter()
            .map(|r| if relay { &r.via_relay } else { &r.direct })
            .collect::<Vec<_>>();
        let weights =
            std::array::from_fn(|i| reference_weights(&assay.edges, selected[i], strength));
        let (row, score) = assay.compare(&weights, &reset, &targets, &CUES)?;
        if score > best_score {
            best = Some((name, weights));
            best_score = score;
        }
        rows.insert(name, row);
    }
    let (followup, native_at_reference) = if let Some((name, weights)) = best {
        let (forward, forward_history) = assay.learned(0)?;
        let (reverse, reverse_history) = assay.learned(1)?;
        let native = [forward, reverse];
        let (native_cue, _) = assay.compare(&native, &reset, &targets, &CUES)?;
        let (strengthened, passes) = assay.normalize(&native, &weights, &reset, &targets)?;
        let (attenuated, _) = assay.normalize(&weights, &native, &reset, &targets)?;
        let fragment_order = FRAGMENTS;
        let fragment_names: Vec<_> = fragment_order.into_iter().chain(["blank"]).collect();
        let fragment_reset = assay.observe(&assay.initial, &fragment_names)?;
        let (fragments, _) = assay.compare(&weights, &fragment_reset, &targets, &fragment_order)?;
        let mut lesions = Vec::new();
        for (flip, mapping) in MAPPINGS.into_iter().enumerate() {
            let paths = if name.starts_with("relay") {
                &route_sets[flip].via_relay
            } else {
                &route_sets[flip].direct
            };
            for (index, cue) in CUES.into_iter().enumerate() {
                let check = &rows[name]["arms"][mapping]["checks"][cue];
                if check["passed"] != true {
                    continue;
                }
                let target = &targets[index ^ flip];
                let mut restored = Vec::new();
                let edited = weights[flip]
                    .iter()
                    .zip(&assay.edges)
                    .map(|(&current, edge)| {
                        if paths[index].contains(&edge.id) && target.contains(&edge.post) {
                            restored.push(edge.id);
                            (current.0, edge.initial)
                        } else {
                            current
                        }
                    })
                    .collect();
                let response = assay.response(&edited, cue)?;
                if response.root_spikes != reset[cue].root_spikes
                    || response.start_us != reset[cue].start_us
                {
                    return Err(error("capacity terminal ablation changed cue input"));
                }
                let after = spike_count(&response, target);
                lesions.push(
                    json!({"mapping":mapping,"cue":cue,"restored_edges":restored,
                    "intact_correct_count":check["correct"],"ablated_correct_count":after,
                    "response":response.json()}),
                );
            }
        }
        (
            json!({"choice":name,"native":native_cue,"native_at_reference":strengthened,
                "training":{"forward":forward_history,"reverse":reverse_history},
                "reference_at_native":attenuated,"fragment_order":fragment_order,
                "fragment_reset":json_responses(&fragment_reset),"fragments":fragments,
                "terminal_ablation":lesions}),
            passes,
        )
    } else {
        (Value::Null, 0)
    };
    let routes = route_sets
        .iter()
        .map(|Routes { direct, via_relay }| json!({"direct":direct,"via_relay":via_relay}))
        .collect::<Vec<_>>();
    Ok(TrialMetrics {
        values: METRICS
            .into_iter()
            .map(str::to_owned)
            .zip([
                best_score as f64,
                f64::from(best_score == 4),
                f64::from(native_at_reference == 4),
            ])
            .collect(),
        details: json!({"protocol":PROTOCOL,"seed":seed,"development":development,"stimuli":stimuli(),
            "topology":assay.topology(),"core_ids":assay.core,"excitatory_core":excitatory,"quiet_relays":quiet,
            "reset":json_responses(&reset),"target_groups":targets,"routes":routes,"references":rows,"followup":followup}),
    })
}

impl Scenario for CapacityScenario {
    type Config = Config;
    fn name(&self) -> &'static str {
        "hippocampal_contingency_capacity"
    }
    fn default_config(&self) -> Config {
        ContingencyScenario.default_config()
    }
    fn validate(&self, c: &Config) -> Result<()> {
        ContingencyScenario.validate(c)
    }
    fn set_parameter(&self, c: &mut Config, name: &str, v: &Value) -> Result<()> {
        ContingencyScenario.set_parameter(c, name, v)
    }
    fn interventions(&self) -> Vec<Intervention> {
        vec![Intervention {
            name: "fixed_routes".into(),
        }]
    }
    fn metric_names(&self) -> Vec<&'static str> {
        METRICS.to_vec()
    }
    fn snapshot(&self, c: &Config, seed: u64, i: &Intervention) -> Value {
        json!({"protocol":PROTOCOL,"seed":seed,"intervention":i.name,
            "processing_neurons":c.processing_neurons,"training_repetitions":c.training_repetitions,
            "rest_us":c.rest_us,"stimuli":stimuli(),"cases":CASES})
    }
    fn run(&self, c: &Config, seed: u64, i: &Intervention) -> Result<TrialMetrics> {
        self.validate(c)?;
        if i.name != "fixed_routes" {
            return Err(error("unknown contingency capacity intervention"));
        }
        run(c, seed)
    }
    fn evaluate(&self, records: &[Value], full_replay: bool) -> Result<Value> {
        Ok(
            json!({"status":"capacity_diagnostic","passed":null,"replay":full_replay,"records":records.len()}),
        )
    }
}
