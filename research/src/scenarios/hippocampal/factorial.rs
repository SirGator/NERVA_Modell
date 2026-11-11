//! Cross direction and L2 strength at matched checkpoints; search recurrent relay paths.
use super::{
    Config, HippocampalScenario,
    association::{Training, train_with_learning},
    causal::verify_only_weights,
    diagnosis::{
        self,
        probe::{self, Probe, margins},
    },
};
use crate::{
    Result, error,
    scenario::{Intervention, Scenario, TrialMetrics},
};
use nerva::core::SynapseId;
use nerva_vm::model::LearningRule;
use serde_json::{Value, json};
use std::collections::BTreeMap;
pub(super) mod search;
pub(super) mod weights;
use weights::{norm, plan};
pub const MAX_TRAINING_PASSES: usize = 25;
pub const METRICS: [&str; 8] = [
    "reference_verified",
    "recurrent_reference_verified",
    "requested_factorial_valid",
    "feasible_factorial_valid",
    "natural_gain_a",
    "natural_gain_b",
    "native_training_passes",
    "audit_rows",
];
pub struct FactorialScenario;

fn train(c: &Config, seed: u64, audit: bool) -> Result<Training> {
    train_with_learning(
        c,
        seed,
        "three_on",
        audit,
        Some(LearningRule::ThreeFactorAccumulated),
    )
}
struct Observations<'a> {
    reference: &'a Training,
    c: &'a Config,
    seed: u64,
    targets: Vec<Vec<usize>>,
    signatures: Vec<Vec<(SynapseId, f32)>>,
    rows: Vec<Value>,
    training_passes: usize,
}
impl<'a> Observations<'a> {
    fn new(reference: &'a Training, c: &'a Config, seed: u64, base: Probe) -> Self {
        let targets = vec![base.counts[1].clone(), base.counts[2].clone()];
        let margin = margins(&base, &targets);
        Self {
            reference,
            c,
            seed,
            targets,
            signatures: vec![reference.weights.clone()],
            rows: vec![
                json!({"weights":reference.weights.iter().map(|(id,w)|(id.0,f64::from(*w))).collect::<Vec<_>>(),
                "completion_margins":margin,"margin_gains":[0.,0.],"core_effect":false,"motor_effect":false,"probe":base.details}),
            ],
            training_passes: 2,
        }
    }
    fn observe(&mut self, desired: Vec<(SynapseId, f32)>) -> Result<usize> {
        // Exact f32 vectors are the cache key. Reuse includes the full fixed probe sequence.
        if let Some(index) = self.signatures.iter().position(|old| *old == desired) {
            return Ok(index);
        }
        let mut branch = train(self.c, self.seed, false)?;
        if !self.reference.model.same_replay_state(&branch.model) {
            return Err(error("factorial checkpoint differs"));
        }
        verify_only_weights(&self.reference.model, &mut branch.model, &desired)?;
        let response = probe::run(&mut branch.model, self.c)?;
        let margin = margins(&response, &self.targets);
        let baseline = &self.rows[0];
        let gain = [
            margin[0] - baseline["completion_margins"][0].as_f64().unwrap(),
            margin[1] - baseline["completion_margins"][1].as_f64().unwrap(),
        ];
        let differs = |field: &str| {
            response.details["frames"]
                .as_array()
                .unwrap()
                .iter()
                .zip(baseline["probe"]["frames"].as_array().unwrap())
                .any(|(a, b)| a[field] != b[field])
        };
        self.rows.push(json!({"weights":desired.iter().map(|(id,w)|(id.0,f64::from(*w))).collect::<Vec<_>>(),
            "completion_margins":margin,"margin_gains":gain,"core_effect":differs("core_event_digest"),
            "motor_effect":differs("motor_event_digest"),"probe":response.details}));
        self.signatures.push(desired);
        self.training_passes += 1;
        Ok(self.rows.len() - 1)
    }
    fn eligible(&self, index: usize, distinct: bool) -> bool {
        let row = &self.rows[index];
        distinct
            && (0..2).all(|cue| {
                row["completion_margins"][cue].as_f64().unwrap() > 1e-9
                    && row["margin_gains"][cue].as_f64().unwrap() > 1e-9
            })
            && row["probe"]["frames"].as_array().unwrap()[3..]
                .iter()
                .all(|f| {
                    f["core_counts"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|v| v.as_u64().unwrap())
                        .sum::<u64>()
                        > 0
                })
            && row["probe"]["frames"][0]["core_counts"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_u64().unwrap())
                .sum::<u64>()
                <= self.rows[0]["probe"]["frames"][0]["core_counts"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_u64().unwrap())
                    .sum::<u64>()
    }
    fn arm(&mut self, direction: &[f64], strength: f64) -> Result<Value> {
        let mut desired = plan(&self.reference.weights, direction, strength);
        let index = if let Some(w) = desired.weights.take() {
            Some(self.observe(w)?)
        } else {
            None
        };
        desired.details["observation"] = json!(index);
        desired.details["matched_checkpoint"] = json!(index.is_some());
        desired.details["only_weights_changed"] = json!(index.is_some());
        Ok(desired.details)
    }
}
fn matrix(
    observations: &mut Observations<'_>,
    natural: &[f64],
    artificial: &[f64],
    high: f64,
) -> Result<Value> {
    let low = norm(natural);
    let mut arms = BTreeMap::new();
    for (name, direction, strength) in [
        ("NN", natural, low),
        ("NR", natural, high),
        ("RN", artificial, low),
        ("RR", artificial, high),
    ] {
        arms.insert(name, observations.arm(direction, strength)?);
    }
    let valid = norm(natural) > 0.
        && norm(artificial) > 0.
        && (high - low).abs() > 1e-6 * low
        && arms
            .values()
            .all(|a| a["feasible"] == true && a["faithful"] == true);
    let contrasts = if valid {
        let response = |name: &str, cue: usize| {
            observations.rows[arms[name]["observation"].as_u64().unwrap() as usize]
            ["completion_margins"][cue].as_f64().unwrap()
        };
        Value::Array(
            (0..2)
                .map(|cue| {
                    let nn = response("NN", cue);
                    let nr = response("NR", cue);
                    let rn = response("RN", cue);
                    let rr = response("RR", cue);
                    json!({"direction_at_natural_strength":rn-nn,"direction_at_high_strength":rr-nr,
                "strength_on_natural_direction":nr-nn,"strength_on_reference_direction":rr-rn,
                "interaction":rr-nr-rn+nn})
                })
                .collect(),
        )
    } else {
        Value::Null
    };
    Ok(
        json!({"natural_strength":low,"high_strength":high,"arms":arms,"valid":valid,"contrasts":contrasts,
        "limits":"L2 strength and direction fidelity are checked after f32 publication. 1% norm and cosine>=0.999 tolerances. No mathematical clipping is permitted."}),
    )
}
impl Scenario for FactorialScenario {
    type Config = Config;
    fn name(&self) -> &'static str {
        "hippocampal_direction_strength"
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
        json!({"protocol":"NERVA-Direction-Strength/1","seed":seed,"experiment":i.name,
            "processing_neurons":c.processing_neurons,"training_repetitions":c.training_repetitions,"rest_us":c.rest_us,
            "training_passes_per_execution":MAX_TRAINING_PASSES,"training_budget_is_upper_bound":true,
            "search_methods":search::METHODS,"doses":diagnosis::DOSES,"families":["targeted","swapped"],
            "norm":"L2 of actual visible f32 deltas from initial weights","axes":["direction","strength"],
            "reference_priority":["recurrent_contrast","recurrent_two_hop","sensory_and_recurrent"],
            "normalization_tolerances":{"relative_norm":0.01,"direction_cosine":0.999},
            "limits":"In-sample artificial search. Fixed probes and frozen native dynamics; not independent learned memory confirmation."})
    }
    fn interventions(&self) -> Vec<Intervention> {
        vec![Intervention {
            name: "direction_strength".into(),
        }]
    }
    fn metric_names(&self) -> Vec<&'static str> {
        METRICS.to_vec()
    }
    fn run(&self, c: &Config, seed: u64, i: &Intervention) -> Result<TrialMetrics> {
        self.validate(c)?;
        if i.name != "direction_strength" {
            return Err(error("unknown direction/strength intervention"));
        }
        let reference = train(c, seed, true)?;
        let mut baseline = train(c, seed, false)?;
        if !reference.model.same_replay_state(&baseline.model) {
            return Err(error("factorial initial checkpoint differs"));
        }
        verify_only_weights(&reference.model, &mut baseline.model, &reference.weights)?;
        let base = probe::run(&mut baseline.model, c)?;
        let edges = diagnosis::build_edges(&reference, &base)?;
        let distinct = base.counts[1..3]
            .iter()
            .all(|v| v.iter().sum::<usize>() > 0)
            && super::cosine(&base.counts[1], &base.counts[2]) < 1. - 1e-9;
        let directions: BTreeMap<_, _> = search::METHODS
            .into_iter()
            .map(|method| (method, search::scores(method, &edges, &reference, &base)))
            .collect();
        let mut observations = Observations::new(&reference, c, seed, base);
        let mut candidates = Vec::new();
        for method in search::METHODS {
            for dose in diagnosis::DOSES {
                for (family, sign) in [("targeted", 1.), ("swapped", -1.)] {
                    let desired: Vec<_> = edges
                        .iter()
                        .zip(&directions[method])
                        .map(|(e, s)| {
                            (
                                e.id,
                                (f64::from(e.initial) + sign * dose * s).clamp(0., 2.) as f32,
                            )
                        })
                        .collect();
                    let clipped = edges
                        .iter()
                        .zip(&directions[method])
                        .filter(|(e, s)| {
                            !(0. ..=2.).contains(&(f64::from(e.initial) + sign * dose * *s))
                        })
                        .count();
                    let index = observations.observe(desired)?;
                    candidates.push(json!({"method":method,"dose":dose,"family":family,"observation":index,
                        "clipped_edges":clipped,"candidate_passed_without_swap_control":observations.eligible(index,distinct),
                        "verified_reference":false}));
                }
            }
        }
        let mut references = BTreeMap::new();
        for index in (0..candidates.len()).step_by(2) {
            let a = candidates[index]["observation"].as_u64().unwrap() as usize;
            let b = candidates[index + 1]["observation"].as_u64().unwrap() as usize;
            let passes = candidates[index]["candidate_passed_without_swap_control"] == true
                && (0..2).all(|cue| {
                    observations.rows[a]["margin_gains"][cue].as_f64().unwrap()
                        > observations.rows[b]["margin_gains"][cue].as_f64().unwrap() + 1e-9
                });
            candidates[index]["verified_reference"] = json!(passes);
            if passes {
                references
                    .entry(candidates[index]["method"].as_str().unwrap().to_owned())
                    .or_insert(index);
            }
        }
        let selected = [
            "recurrent_contrast",
            "recurrent_two_hop",
            "sensory_and_recurrent",
        ]
        .into_iter()
        .find_map(|m| references.get(m).copied());
        let natural: Vec<_> = edges
            .iter()
            .map(|e| f64::from(e.natural) - f64::from(e.initial))
            .collect();
        let natural_observation =
            observations.observe(edges.iter().map(|e| (e.id, e.natural)).collect())?;
        let recurrent = references.keys().any(|m| m.starts_with("recurrent_"));
        let mut requested = Value::Null;
        let mut feasible = Value::Null;
        let mut direction = Value::Null;
        if let Some(index) = selected {
            let observation = candidates[index]["observation"].as_u64().unwrap() as usize;
            let artificial: Vec<_> = observations.signatures[observation]
                .iter()
                .zip(&edges)
                .map(|((_, w), e)| f64::from(*w) - f64::from(e.initial))
                .collect();
            let reference_norm = norm(&artificial);
            requested = matrix(&mut observations, &natural, &artificial, reference_norm)?;
            if let (Some(n), Some(a)) = (
                weights::limit(&reference.weights, &natural),
                weights::limit(&reference.weights, &artificial),
            ) {
                let common = reference_norm.min(n).min(a);
                feasible = matrix(&mut observations, &natural, &artificial, common)?;
            }
            direction = json!({"candidate_index":index,"observation":observation,"method":candidates[index]["method"],
                "alignment":diagnosis::alignment(&edges,&observations.signatures[observation],None)});
        }
        if observations.training_passes > MAX_TRAINING_PASSES {
            return Err(error("factorial reconstruction budget exceeded"));
        }
        let rows: usize = observations
            .rows
            .iter()
            .map(|r| r["probe"]["audit_rows"].as_u64().unwrap() as usize)
            .sum();
        let natural_gains = observations.rows[natural_observation]["margin_gains"].clone();
        let values = [
            f64::from(selected.is_some()),
            f64::from(recurrent),
            f64::from(requested["valid"] == true),
            f64::from(feasible["valid"] == true),
            natural_gains[0].as_f64().unwrap(),
            natural_gains[1].as_f64().unwrap(),
            observations.training_passes as f64,
            rows as f64,
        ];
        Ok(TrialMetrics {
            values: METRICS.iter().map(|n| (*n).into()).zip(values).collect(),
            details: json!({"initial_network_digest":reference.initial_digest,"training":reference.details,
                "matched_checkpoint":true,"only_weights_changed":true,"fixed_targets_distinct":distinct,
                "edges":edges.iter().map(|e|json!([e.id.0,e.pre,e.post,e.path,f64::from(e.initial),f64::from(e.natural),e.score])).collect::<Vec<_>>(),
                "directions":directions,"candidates":candidates,"references_by_method":references,
                "direction_reference":direction,"natural_observation":natural_observation,
                "requested_factorial":requested,"feasible_factorial":feasible,
                "observations":observations.rows,"maximum_training_passes":MAX_TRAINING_PASSES,
                "functional_memory_demonstrated":false,"independent_confirmation":false,
                "limits":"Artificial calibration on these images/seeds; reference search failure is not proof of an impossible path. Whole fixed probe sequences share one initial checkpoint; later histories may diverge. Recurrent directions modify only existing core-to-core weights. Duplicate vectors reuse exact observations."}),
        })
    }
    fn evaluate(&self, records: &[Value], replay: bool) -> Result<Value> {
        Ok(
            json!({"status":"direction_strength_factorial","records":records.len(),"replay_checked":replay,
            "functional_memory_demonstrated":false,"independent_confirmation":false}),
        )
    }
}
