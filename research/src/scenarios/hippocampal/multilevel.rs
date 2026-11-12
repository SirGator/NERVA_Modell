//! One fixed diagnosis: true learning-only intervention, local replay and matched norm probes.
use super::{
    Config,
    association::{self, Observers, Training},
    causal::verify_only_weights,
    diagnosis::{
        self,
        probe::{self, Probe, margins},
    },
    factorial::{search, weights},
};
use crate::{
    Result, error,
    scenario::{Intervention, Scenario, TrialMetrics},
};
use nerva::core::SynapseId;
use nerva_vm::model::LearningRule;
use serde_json::{Value, json};
use std::collections::BTreeMap;
pub(super) mod tape;
pub struct MultilevelScenario;
const MAX_PASSES: usize = 30;
const METRICS: [&str; 8] = [
    "offline_replay_verified",
    "constant_core_effect",
    "constant_gain_a",
    "constant_gain_b",
    "reference_verified",
    "matched_norm_comparison_valid",
    "native_training_passes",
    "audit_rows",
];
fn settings(arm: &str) -> (&str, LearningRule) {
    match arm {
        "A_frozen" => ("frozen_on", LearningRule::ThreeFactorAccumulated),
        "C_constant" => ("three_on", LearningRule::ThreeFactorConstantPositive),
        "D_pair" => ("pair_on", LearningRule::PairStdp),
        _ => ("three_on", LearningRule::ThreeFactorAccumulated),
    }
}
fn train(c: &Config, seed: u64, arm: &str, record: bool) -> Result<(Training, tape::Tape)> {
    let (group, rule) = settings(arm);
    let mut tape = tape::Tape::default();
    let t = association::train_recorded(
        c,
        seed,
        group,
        record,
        Some(rule),
        Observers {
            history: None,
            tape: record.then_some(&mut tape),
            frame: None,
        },
    )?;
    Ok((t, tape))
}
fn weights_of(t: &Training) -> Vec<(SynapseId, f32)> {
    t.model
        .network()
        .synapses()
        .map(|s| (s.id(), s.weight().get()))
        .collect()
}
fn delta(initial: &[(SynapseId, f32)], current: &[(SynapseId, f32)]) -> Vec<f64> {
    initial
        .iter()
        .zip(current)
        .map(|((id, a), (other, b))| {
            assert_eq!(id, other);
            f64::from(*b) - f64::from(*a)
        })
        .collect()
}
fn response(p: Probe, base: &Probe, targets: &[Vec<usize>]) -> Value {
    let m = margins(&p, targets);
    let b = margins(base, targets);
    let gain = [m[0] - b[0], m[1] - b[1]];
    let distinct = super::cosine(&targets[0], &targets[1]) < 1. - 1e-9;
    json!({"completion_margins":m,"margin_gains":gain,"eligible":diagnosis::eligible(&p,base,gain,m,distinct),
        "core_effect":diagnosis::effect(&p,base,"core_event_digest"),"motor_effect":diagnosis::effect(&p,base,"motor_event_digest"),
        "potential_effect":diagnosis::effect(&p,base,"neurons"),"probe":p.details})
}
struct Matched<'a> {
    c: &'a Config,
    seed: u64,
    reference: &'a Training,
    base: Probe,
    targets: Vec<Vec<usize>>,
    cache: Vec<Vec<(SynapseId, f32)>>,
    rows: Vec<Value>,
    passes: usize,
}
impl Matched<'_> {
    fn observe(&mut self, name: &str, w: Vec<(SynapseId, f32)>) -> Result<usize> {
        if let Some(i) = self.cache.iter().position(|old| *old == w) {
            return Ok(i);
        }
        let (mut branch, _) = train(self.c, self.seed, "B_original", false)?;
        if !self.reference.model.same_replay_state(&branch.model) {
            return Err(error("multilevel matched checkpoint differs"));
        }
        verify_only_weights(&self.reference.model, &mut branch.model, &w)?;
        let p = probe::run(&mut branch.model, self.c)?;
        let mut row = response(p, &self.base, &self.targets);
        row["name"] = json!(name);
        row["weights"] = json!(w.iter().map(|(id, w)| (id.0, *w)).collect::<Vec<_>>());
        row["only_weights_changed"] = json!(true);
        self.cache.push(w);
        self.rows.push(row);
        self.passes += 1;
        Ok(self.rows.len() - 1)
    }
    fn normalized(&mut self, name: &str, d: &[f64], strength: f64) -> Result<Value> {
        let mut plan = weights::plan(&self.reference.weights, d, strength);
        let i = if plan.details["faithful"] == true {
            Some(self.observe(name, plan.weights.take().unwrap())?)
        } else {
            None
        };
        plan.details["observation"] = json!(i);
        Ok(plan.details)
    }
}
impl Scenario for MultilevelScenario {
    type Config = Config;
    fn name(&self) -> &'static str {
        "hippocampal_multilevel_diagnosis"
    }
    fn default_config(&self) -> Config {
        super::association::AssociationScenario.default_config()
    }
    fn set_parameter(&self, c: &mut Config, name: &str, v: &Value) -> Result<()> {
        super::HippocampalScenario.set_parameter(c, name, v)
    }
    fn validate(&self, c: &Config) -> Result<()> {
        super::association::AssociationScenario.validate(c)
    }
    fn interventions(&self) -> Vec<Intervention> {
        vec![Intervention {
            name: "diagnose".into(),
        }]
    }
    fn metric_names(&self) -> Vec<&'static str> {
        METRICS.to_vec()
    }
    fn snapshot(&self, c: &Config, seed: u64, i: &Intervention) -> Value {
        json!({"protocol":"NERVA-Multilevel-Diagnosis/1","seed":seed,"intervention":i.name,
        "processing_neurons":c.processing_neurons,"training_repetitions":c.training_repetitions,"rest_us":c.rest_us,
        "arms":["A_frozen","B_original","C_constant","D_pair","E_fixed_reference"],"constant_learning_factor":1.,"other_chemical_effects":"native, unchanged",
        "reference_method":"sensory_and_recurrent","reference_dose":1.,"swap_control":true,"reference_recipe_fixed_before_confirmation":true,
        "norm_tolerances":{"relative":0.01,"cosine":0.999},"training_passes_per_execution":MAX_PASSES,"training_budget_is_upper_bound":true,
        "offline_tape_limit":2000000,"history_selected_synapses":8})
    }
    fn run(&self, c: &Config, seed: u64, i: &Intervention) -> Result<TrialMetrics> {
        self.validate(c)?;
        if i.name != "diagnose" {
            return Err(error("unknown multilevel intervention"));
        }
        let (reference, tape) = train(c, seed, "B_original", true)?;
        let (mut baseline, _) = train(c, seed, "B_original", false)?;
        if !reference.model.same_replay_state(&baseline.model) {
            return Err(error(
                "recorded multilevel training changed full replay state",
            ));
        }
        let offline = tape.replay(&reference.model)?;
        let history = tape.history(
            c,
            seed,
            "three_on",
            LearningRule::ThreeFactorAccumulated,
            &reference.model,
        )?;
        verify_only_weights(&reference.model, &mut baseline.model, &reference.weights)?;
        let base = probe::run(&mut baseline.model, c)?;
        let targets = vec![base.counts[1].clone(), base.counts[2].clone()];
        let edges = diagnosis::build_edges(&reference, &base)?;
        let score = search::scores("sensory_and_recurrent", &edges, &reference, &base);
        let natural = delta(&reference.weights, &weights_of(&reference));
        let mut matched = Matched {
            c,
            seed,
            reference: &reference,
            base,
            targets,
            cache: Vec::new(),
            rows: Vec::new(),
            passes: 2,
        };
        // Cache the already measured initial-weight observation.
        let baseline_json = json!({"name":"initial_weights","completion_margins":margins(&matched.base,&matched.targets),"margin_gains":[0.,0.],"eligible":false,
            "core_effect":false,"motor_effect":false,"potential_effect":false,"probe":matched.base.details,"weights":reference.weights.iter().map(|(id,w)|(id.0,*w)).collect::<Vec<_>>(),"only_weights_changed":true});
        matched.cache.push(reference.weights.clone());
        matched.rows.push(baseline_json);
        let natural_i = matched.observe("B_original", weights_of(&reference))?;
        let constant_i = matched.observe("offline_constant", offline.constant.clone())?;
        let pair_i = matched.observe("offline_pair", offline.pair.clone())?;
        let mut live = BTreeMap::new();
        let mut directions = BTreeMap::from([
            ("B_original".to_string(), natural.clone()),
            (
                "offline_constant".into(),
                delta(&reference.weights, &offline.constant),
            ),
            (
                "offline_pair".into(),
                delta(&reference.weights, &offline.pair),
            ),
        ]);
        let mut actual_passes = 0;
        for arm in ["A_frozen", "C_constant", "D_pair"] {
            let (mut learned, tape) = train(c, seed, arm, true)?;
            let (mut reset, _) = train(c, seed, arm, false)?;
            actual_passes += 2;
            if !learned.model.same_replay_state(&reset.model) {
                return Err(error("live arm observation changed checkpoint"));
            }
            if learned.initial_digest != reference.initial_digest
                || learned.details["input_digest"] != reference.details["input_digest"]
            {
                return Err(error(
                    "live arms differ in initial network or input schedule",
                ));
            }
            let (group, rule) = settings(arm);
            let h = tape.history(c, seed, group, rule, &learned.model)?;
            let w = weights_of(&learned);
            directions.insert(arm.into(), delta(&reference.weights, &w));
            verify_only_weights(&learned.model, &mut reset.model, &learned.weights)?;
            let without = probe::run(&mut reset.model, c)?;
            let with = probe::run(&mut learned.model, c)?;
            let own = response(with, &without, &matched.targets);
            let transplanted = matched.observe(arm, w.clone())?;
            let predicted = if arm == "C_constant" {
                Some(&offline.constant)
            } else if arm == "D_pair" {
                Some(&offline.pair)
            } else {
                None
            };
            let (different, l2) = predicted.map_or((0, 0.), |p| {
                let differences = p
                    .iter()
                    .zip(&w)
                    .map(|((_, a), (_, b))| f64::from(*a) - f64::from(*b))
                    .collect::<Vec<_>>();
                (
                    differences.iter().filter(|d| **d != 0.).count(),
                    weights::norm(&differences),
                )
            });
            live.insert(arm,json!({"training":learned.details,"history":h,"own_checkpoint_response":own,"matched_B_checkpoint_observation":transplanted,
                "offline_vs_live_different_weights":different,"offline_vs_live_l2":l2,"initial_network_digest":learned.initial_digest}));
        }
        let artificial = edges
            .iter()
            .zip(&score)
            .map(|(e, s)| (e.id, (f64::from(e.initial) + s).clamp(0., 2.) as f32))
            .collect::<Vec<_>>();
        let swapped = edges
            .iter()
            .zip(&score)
            .map(|(e, s)| (e.id, (f64::from(e.initial) - s).clamp(0., 2.) as f32))
            .collect::<Vec<_>>();
        let artificial_delta = delta(&reference.weights, &artificial);
        let e_i = matched.observe("E_fixed_reference", artificial)?;
        let swap_i = matched.observe("E_swapped", swapped)?;
        let reference_verified = matched.rows[e_i]["eligible"] == true
            && (0..2).all(|cue| {
                matched.rows[e_i]["margin_gains"][cue].as_f64().unwrap()
                    > matched.rows[swap_i]["margin_gains"][cue].as_f64().unwrap() + 1e-9
            });
        directions.insert("E_reference".into(), artificial_delta.clone());
        let reference_norm = weights::norm(&artificial_delta);
        let common = if reference_verified {
            directions
                .values()
                .filter_map(|d| weights::limit(&reference.weights, d))
                .fold(reference_norm, f64::min)
        } else {
            0.
        };
        let mut normalized = BTreeMap::new();
        if reference_verified {
            for (name, d) in &directions {
                normalized.insert(name.clone(),json!({"at_natural_strength":matched.normalized(name,d,weights::norm(&natural))?,"at_common_high_strength":matched.normalized(name,d,common)?}));
            }
        }
        let direction_effect = common > weights::norm(&natural) * (1. + 1e-6)
            && reference_verified
            && normalized
                .get("B_original")
                .is_some_and(|n| n["at_common_high_strength"]["faithful"] == true)
            && normalized["E_reference"]["at_common_high_strength"]["faithful"] == true;
        let passes = matched.passes + actual_passes;
        if passes > MAX_PASSES {
            return Err(error("multilevel reconstruction budget exceeded"));
        }
        let rows: usize = matched
            .rows
            .iter()
            .map(|r| r["probe"]["audit_rows"].as_u64().unwrap() as usize)
            .sum();
        let const_response = &live["C_constant"]["own_checkpoint_response"];
        let values = [
            1.,
            f64::from(const_response["core_effect"] == true),
            const_response["margin_gains"][0].as_f64().unwrap(),
            const_response["margin_gains"][1].as_f64().unwrap(),
            f64::from(reference_verified),
            f64::from(direction_effect),
            passes as f64,
            rows as f64,
        ];
        Ok(TrialMetrics {
            values: METRICS.into_iter().map(str::to_owned).zip(values).collect(),
            details: json!({"initial_network_digest":reference.initial_digest,"B_training":reference.details,"B_history":history,"offline":offline.json,
            "B_natural_observation":natural_i,"offline_constant_observation":constant_i,"offline_pair_observation":pair_i,"live":live,"matched_observations":matched.rows,
            "reference":{"verified":reference_verified,"observation":e_i,"swap_observation":swap_i,"method":"sensory_and_recurrent","dose":1.,"norm":reference_norm},
            "normalized":normalized,"common_high_norm":common,"maximum_training_passes":MAX_PASSES,
            "limits":"Artificial image-specific reference; no learned-memory claim. Only B-checkpoint transplants isolate weight direction. Live C/D can alter later spikes. Undefined, bounded or quantization-unfaithful norm arms stay unexecuted. Offline local replay does not generate counterfactual spike trains."}),
        })
    }
    fn evaluate(&self, records: &[Value], replay: bool) -> Result<Value> {
        Ok(
            json!({"status":"multilevel_diagnosis","passed":null,"records":records.len(),"replay_checked":replay,"independent_confirmation":false}),
        )
    }
}
