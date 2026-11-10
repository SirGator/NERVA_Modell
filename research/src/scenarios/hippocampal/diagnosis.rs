//! Fixed, exploratory weight-direction search and exact recall-path accounting.
use super::{
    Config, HippocampalScenario,
    association::{Training, train_with_learning},
    causal::verify_only_weights,
};
use crate::{
    Result, error,
    scenario::{Intervention, Scenario, TrialMetrics},
};
use nerva::core::{NeuronRole, Polarity, SynapseId};
use nerva_vm::model::LearningRule;
use serde_json::{Value, json};
use std::collections::BTreeMap;
pub(super) mod probe;
use probe::{Probe, margins};
pub const DOSES: [f64; 3] = [0.25, 1., 2.];
pub const TRAINING_PASSES: usize = 14;
pub const METRICS: [&str; 8] = [
    "reference_verified",
    "natural_core_effect",
    "natural_motor_effect",
    "natural_margin_gain_a",
    "natural_margin_gain_b",
    "matched_checkpoints",
    "audit_rows",
    "native_training_passes",
];
pub struct DiagnosisScenario;
pub(super) struct Edge {
    pub id: SynapseId,
    pub pre: u64,
    pub post: u64,
    pub path: &'static str,
    pub initial: f32,
    pub natural: f32,
    pub score: f64,
}
pub(super) fn alignment(
    edges: &[Edge],
    installed: &[(SynapseId, f32)],
    path: Option<&str>,
) -> Value {
    let (mut dot, mut aa, mut bb, mut natural_l1, mut support_l1, mut agree, mut overlap) =
        (0., 0., 0., 0., 0., 0usize, 0usize);
    for (e, &(id, w)) in edges.iter().zip(installed) {
        assert_eq!(e.id, id);
        if path.is_some_and(|p| p != e.path) {
            continue;
        }
        let a = f64::from(e.natural) - f64::from(e.initial);
        let b = f64::from(w) - f64::from(e.initial);
        dot += a * b;
        aa += a * a;
        bb += b * b;
        natural_l1 += a.abs();
        if b != 0. {
            support_l1 += a.abs();
        }
        if a != 0. && b != 0. {
            overlap += 1;
            agree += usize::from(a.signum() == b.signum());
        }
    }
    json!({"cosine":if aa>0. && bb>0. {Some((dot/(aa*bb).sqrt()).clamp(-1.,1.))}else{None},
        "natural_l2":aa.sqrt(),"reference_l2":bb.sqrt(),"dot":dot,"natural_l1":natural_l1,
        "natural_l1_on_reference_support":support_l1,"overlapping_nonzero_edges":overlap,"same_sign_edges":agree})
}
pub(super) fn effect(a: &Probe, b: &Probe, field: &str) -> bool {
    a.details["frames"]
        .as_array()
        .unwrap()
        .iter()
        .zip(b.details["frames"].as_array().unwrap())
        .any(|(x, y)| x[field] != y[field])
}
pub(super) fn eligible(
    probe: &Probe,
    base: &Probe,
    gain: [f64; 2],
    margin: [f64; 2],
    distinct: bool,
) -> bool {
    distinct
        && gain.iter().all(|&x| x > 1e-9)
        && margin.iter().all(|&x| x > 1e-9)
        && probe.counts[3..]
            .iter()
            .all(|v| v.iter().sum::<usize>() > 0)
        && probe.counts[0].iter().sum::<usize>() <= base.counts[0].iter().sum::<usize>()
}
pub(super) fn build_edges(trained: &Training, base: &Probe) -> Result<Vec<Edge>> {
    let mut edges = Vec::new();
    for (s, &(id, initial)) in trained.model.network().synapses().zip(&trained.weights) {
        if id != s.id() {
            return Err(error("diagnosis edge identity differs"));
        }
        let pre = trained.model.network().neuron(s.pre()).unwrap();
        let post = trained.model.network().neuron(s.post()).unwrap();
        let path = match (pre.role(), post.role(), pre.polarity()) {
            (Some(NeuronRole::Sensory), Some(NeuronRole::Processing), _) => "sensory_to_core",
            (Some(NeuronRole::Processing), Some(NeuronRole::Processing), Polarity::Excitatory) => {
                "core_exc_to_core"
            }
            (Some(NeuronRole::Processing), Some(NeuronRole::Processing), Polarity::Inhibitory) => {
                "core_inh_to_core"
            }
            _ => "other",
        };
        let count =
            |image: usize, node: u64| *base.source_counts[image].get(&node).unwrap_or(&0) as f64;
        let cue = count(3, s.pre().0) - count(4, s.pre().0);
        let missing_a = (count(1, s.post().0) - count(3, s.post().0)).max(0.);
        let missing_b = (count(2, s.post().0) - count(4, s.post().0)).max(0.);
        let sign = if pre.polarity() == Polarity::Excitatory {
            1.
        } else {
            -1.
        };
        let score = if path != "other" && s.is_enabled() && s.is_plastic() {
            sign * cue * (missing_a - missing_b)
        } else {
            0.
        };
        edges.push(Edge {
            id,
            pre: s.pre().0,
            post: s.post().0,
            path,
            initial,
            natural: s.weight().get(),
            score,
        });
    }
    let maximum = edges.iter().map(|e| e.score.abs()).fold(0f64, f64::max);
    if maximum > 0. {
        for e in &mut edges {
            e.score /= maximum;
        }
    }
    Ok(edges)
}
impl Scenario for DiagnosisScenario {
    type Config = Config;
    fn name(&self) -> &'static str {
        "hippocampal_direction_path"
    }
    fn default_config(&self) -> Config {
        Config {
            processing_neurons: 256,
            training_repetitions: 8,
            rest_us: 11_000_000,
        }
    }
    fn set_parameter(&self, c: &mut Config, name: &str, value: &Value) -> Result<()> {
        HippocampalScenario.set_parameter(c, name, value)
    }
    fn validate(&self, c: &Config) -> Result<()> {
        super::association::AssociationScenario.validate(c)
    }
    fn snapshot(&self, c: &Config, seed: u64, i: &Intervention) -> Value {
        json!({"protocol":"NERVA-Direction-Path/2","seed":seed,"experiment":i.name,
            "processing_neurons":c.processing_neurons,"training_repetitions":c.training_repetitions,"rest_us":c.rest_us,
            "training_passes_per_execution":TRAINING_PASSES,"doses":DOSES,
            "learning_rule":"three_factor_accumulated","candidate_families":["targeted","swapped"],
            "candidate_scopes":["sensory_and_recurrent","recurrent_only"],
            "probe_images":["black","full_a","full_b","partial_a","partial_b"],
            "reference_readout":"Fixed full_a/full_b activity from the initial-weight branch at a matched trained checkpoint",
            "limits":"Bounded artificial search using calibration activity; in-sample capability only. No learning-rate or dynamics changes."})
    }
    fn interventions(&self) -> Vec<Intervention> {
        vec![Intervention {
            name: "direction_path".into(),
        }]
    }
    fn metric_names(&self) -> Vec<&'static str> {
        METRICS.to_vec()
    }
    fn run(&self, c: &Config, seed: u64, i: &Intervention) -> Result<TrialMetrics> {
        self.validate(c)?;
        if i.name != "direction_path" {
            return Err(error("unknown direction/path intervention"));
        }
        let mut trained = train_with_learning(
            c,
            seed,
            "three_on",
            true,
            Some(LearningRule::ThreeFactorAccumulated),
        )?;
        let mut baseline = train_with_learning(
            c,
            seed,
            "three_on",
            false,
            Some(LearningRule::ThreeFactorAccumulated),
        )?;
        if !trained.model.same_replay_state(&baseline.model) {
            return Err(error("diagnosis baseline checkpoint differs"));
        }
        verify_only_weights(&trained.model, &mut baseline.model, &trained.weights)?;
        let base = probe::run(&mut baseline.model, c)?;
        let targets = vec![base.counts[1].clone(), base.counts[2].clone()];
        let base_margins = margins(&base, &targets);
        let distinct = targets.iter().all(|v| v.iter().sum::<usize>() > 0)
            && super::cosine(&targets[0], &targets[1]) < 1. - 1e-9;
        let edges = build_edges(&trained, &base)?;
        let mut candidates = Vec::new();
        let mut candidate_weights = Vec::new();
        let mut total_rows = base.details["audit_rows"].as_u64().unwrap();
        // All branches are reconstructed before the trained reference is advanced.
        for scope in ["sensory_and_recurrent", "recurrent_only"] {
            let scores: Vec<_> = edges
                .iter()
                .map(|e| {
                    if scope == "recurrent_only" && !e.path.starts_with("core_") {
                        0.
                    } else {
                        e.score
                    }
                })
                .collect();
            let scale = scores.iter().map(|s| s.abs()).fold(0f64, f64::max);
            let scores: Vec<_> = scores
                .into_iter()
                .map(|s| if scale > 0. { s / scale } else { 0. })
                .collect();
            for dose in DOSES {
                for (family, sign) in [("targeted", 1.), ("swapped", -1.)] {
                    let mut branch = train_with_learning(
                        c,
                        seed,
                        "three_on",
                        false,
                        Some(LearningRule::ThreeFactorAccumulated),
                    )?;
                    if !trained.model.same_replay_state(&branch.model) {
                        return Err(error("candidate checkpoint differs"));
                    }
                    let desired: Vec<_> = edges
                        .iter()
                        .zip(&scores)
                        .map(|(e, score)| {
                            (
                                e.id,
                                (f64::from(e.initial) + sign * dose * score).clamp(0., 2.) as f32,
                            )
                        })
                        .collect();
                    verify_only_weights(&trained.model, &mut branch.model, &desired)?;
                    let response = probe::run(&mut branch.model, c)?;
                    let margin = margins(&response, &targets);
                    let gain = [margin[0] - base_margins[0], margin[1] - base_margins[1]];
                    let passes = eligible(&response, &base, gain, margin, distinct);
                    let clipped = edges
                        .iter()
                        .zip(&scores)
                        .filter(|(e, score)| {
                            !(0. ..=2.).contains(&(f64::from(e.initial) + sign * dose * *score))
                        })
                        .count();
                    let l1: f64 = edges
                        .iter()
                        .zip(&desired)
                        .map(|(e, (_, w))| (f64::from(*w) - f64::from(e.initial)).abs())
                        .sum();
                    let paths: BTreeMap<_, _> = [
                        "sensory_to_core",
                        "core_exc_to_core",
                        "core_inh_to_core",
                        "other",
                    ]
                    .into_iter()
                    .map(|p| (p, alignment(&edges, &desired, Some(p))))
                    .collect();
                    total_rows += response.details["audit_rows"].as_u64().unwrap();
                    candidates.push(json!({"scope":scope,"family":family,"dose":dose,"completion_margins":margin,"margin_gains":gain,
                    "candidate_passed_without_swap_control":passes,"verified_reference":false,
                    "clipped_edges":clipped,"installed_l1":l1,"alignment":alignment(&edges,&desired,None),
                    "alignment_by_path":paths,"weights":desired.iter().map(|(id,w)|(id.0,f64::from(*w))).collect::<Vec<_>>(),
                    "core_effect":effect(&response,&base,"core_event_digest"),
                    "motor_effect":effect(&response,&base,"motor_event_digest"),
                    "matched_checkpoint":true,"only_weights_changed":true,"probe":response.details}));
                    candidate_weights.push(desired);
                }
            }
        }
        // A candidate is valid only if BOTH cues improve and beat the swapped-label
        // control at the identical dose. Select the first passing dose, never the best score.
        let mut selected = None;
        for index in (0..candidates.len()).step_by(2) {
            let better = (0..2).all(|cue| {
                candidates[index]["margin_gains"][cue].as_f64().unwrap()
                    > candidates[index + 1]["margin_gains"][cue].as_f64().unwrap() + 1e-9
            });
            let passes =
                candidates[index]["candidate_passed_without_swap_control"] == true && better;
            candidates[index]["verified_reference"] = json!(passes);
            if passes && (selected.is_none() || (index >= 6 && selected.is_some_and(|old| old < 6)))
            {
                selected = Some(index);
            }
        }
        let natural = probe::run(&mut trained.model, c)?;
        total_rows += natural.details["audit_rows"].as_u64().unwrap();
        let natural_margins = margins(&natural, &targets);
        let natural_gains = [
            natural_margins[0] - base_margins[0],
            natural_margins[1] - base_margins[1],
        ];
        let core_effect = effect(&natural, &base, "core_event_digest");
        let motor_effect = effect(&natural, &base, "motor_event_digest");
        let verified = selected.is_some();
        let direction=selected.map(|index|json!({"candidate_index":index,"alignment":alignment(&edges,&candidate_weights[index],None),
            "scope":candidates[index]["scope"],"by_path":candidates[index]["alignment_by_path"]})).unwrap_or(Value::Null);
        let values = [
            f64::from(verified),
            f64::from(core_effect),
            f64::from(motor_effect),
            natural_gains[0],
            natural_gains[1],
            1.,
            total_rows as f64,
            TRAINING_PASSES as f64,
        ];
        Ok(TrialMetrics {
            values: METRICS.iter().map(|s| (*s).into()).zip(values).collect(),
            details: json!({"initial_network_digest":trained.initial_digest,"training":trained.details,
                "matched_checkpoint":true,"only_weights_changed":true,"fixed_targets_distinct":distinct,
                "baseline_margins":base_margins,"natural_margins":natural_margins,
                "baseline":base.details,"natural":natural.details,"candidates":candidates,
                "direction_reference":direction,"reference_verified":verified,
                "edges":edges.iter().map(|e|json!([e.id.0,e.pre,e.post,e.path,f64::from(e.initial),f64::from(e.natural),e.score])).collect::<Vec<_>>(),
                "functional_memory_demonstrated":false,"independent_confirmation":false,
                "diagnosis":if verified {"artificial_path_capability_demonstrated; compare natural direction with this reference"}else{
                    "unresolved: no verified reference in the bounded search; neither learning rule nor network declared faulty"},
                "limits":"Artificial weights are fit from baseline activity, not learned or independently confirmed. A failure of this finite search does not prove absence of a working path. Full probes form one fixed sequence; later responses can include earlier intervention effects. No new edges, stimuli, threshold or learning-rate edits."}),
        })
    }
    fn evaluate(&self, records: &[Value], replay: bool) -> Result<Value> {
        Ok(
            json!({"status":"direction_path_diagnosis","records":records.len(),"replay_checked":replay,
            "functional_memory_demonstrated":false,"independent_confirmation":false}),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn electrical_observation_preserves_the_full_chemical_model_replay_state() {
        let c = Config {
            processing_neurons: 16,
            training_repetitions: 1,
            rest_us: 100_000,
        };
        let mut observed = train_with_learning(
            &c,
            7,
            "three_on",
            false,
            Some(LearningRule::ThreeFactorAccumulated),
        )
        .unwrap();
        let mut plain = train_with_learning(
            &c,
            7,
            "three_on",
            false,
            Some(LearningRule::ThreeFactorAccumulated),
        )
        .unwrap();
        let result = probe::run(&mut observed.model, &c).unwrap();
        for (index, image) in [4, 0, 1, 2, 3].into_iter().enumerate() {
            let mut rest = c.rest_us;
            while rest > 0 {
                let duration = rest.min(50_000);
                plain
                    .model
                    .step_for(&super::super::frame(4).unwrap(), duration)
                    .unwrap();
                rest -= duration;
            }
            let step = plain
                .model
                .step(&super::super::frame(image).unwrap())
                .unwrap();
            assert_eq!(result.counts[index], step.core_counts);
        }
        assert!(observed.model.same_replay_state(&plain.model));
    }
    #[test]
    fn zero_norm_and_signed_directions_are_not_confused() {
        let edges = vec![Edge {
            id: SynapseId(1),
            pre: 1,
            post: 2,
            path: "test",
            initial: 1.,
            natural: 1.25,
            score: 1.,
        }];
        assert_eq!(
            alignment(&edges, &[(SynapseId(1), 1.)], None)["cosine"],
            Value::Null
        );
        assert_eq!(
            alignment(&edges, &[(SynapseId(1), 0.5)], None)["cosine"],
            -1.
        );
        assert_eq!(
            alignment(&edges, &[(SynapseId(1), 1.5)], None)["same_sign_edges"],
            1
        );
    }
}
