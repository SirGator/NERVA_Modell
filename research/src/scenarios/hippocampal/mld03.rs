//! Fixed paired follow-up: excitability versus learned weights, and homeostasis populations.
use super::{
    Config, HippocampalScenario,
    association::AssociationScenario,
    causal::verify_only_weights,
    diagnosis::probe::{self, Probe},
    fail,
    mld02::{
        atlas::{get_weights, train},
        images, interaction,
    },
};
use crate::{
    Result, error,
    scenario::{Intervention, Scenario, TrialMetrics},
};
use nerva::core::NeuronRole;
use nerva_vm::model::{HomeostasisScope, Model};
use serde_json::{Value, json};
use std::collections::BTreeMap;

pub struct Mld03Scenario;
pub const DYNAMICS: [(&str, f32, f32); 6] = [
    ("native", 1., 1.),
    ("threshold_09999", 0.9999, 1.),
    ("threshold_099", 0.99, 1.),
    ("threshold_090", 0.9, 1.),
    ("tau_half", 1., 0.5),
    ("tau_double", 1., 2.),
];
const METRICS: [&str; 4] = [
    "native_training_passes",
    "spike_effects",
    "functional_candidates",
    "population_interaction_a",
];
fn observe(model: &mut Model, c: &Config) -> Result<Probe> {
    probe::run_images_timed(model, c, &images::PROBES, images::probe, true)
}
fn margins(p: &Probe, targets: &[Vec<usize>]) -> Vec<f64> {
    (3..9)
        .map(|i| {
            let k = (i - 3) % 2;
            super::cosine(&p.counts[i], &targets[k]) - super::cosine(&p.counts[i], &targets[1 - k])
        })
        .collect()
}
fn measured(p: Probe, reset: &Probe, targets: &[Vec<usize>]) -> Value {
    let m = margins(&p, targets);
    let b = margins(reset, targets);
    let gains = m.iter().zip(b).map(|(a, b)| a - b).collect::<Vec<_>>();
    let active = targets.iter().all(|t| t.iter().sum::<usize>() > 0)
        && super::cosine(&targets[0], &targets[1]) < 1. - 1e-9;
    let quiet = p.counts[0].iter().sum::<usize>() <= reset.counts[0].iter().sum::<usize>();
    let gate = |start: usize, end: usize| {
        active
            && quiet
            && (start..end).all(|i| {
                m[i] > 1e-9 && gains[i] > 1e-9 && p.counts[i + 3].iter().sum::<usize>() > 0
            })
    };
    json!({"margins":m,"gains":gains,"original_success":gate(0,2),"novel_success":gate(2,6),"all_cues_success":gate(0,6),
        "core_effect":super::diagnosis::effect(&p,reset,"core_event_digest"),"motor_effect":super::diagnosis::effect(&p,reset,"motor_event_digest"),"probe":p.details})
}
fn populations(model: &Model) -> Value {
    json!({"sensor":model.network().neurons().filter(|n|n.role()==Some(NeuronRole::Sensory) && n.id().0 < model.processing_ids()[0].0).map(|n|n.id().0).collect::<Vec<_>>(),
        "core":model.processing_ids().iter().map(|n|n.0).collect::<Vec<_>>(),
        "motor":model.network().neurons().filter(|n|n.role()==Some(NeuronRole::Motor)).map(|n|n.id().0).collect::<Vec<_>>(),
        "other":model.network().neurons().filter(|n|n.role()==Some(NeuronRole::Sensory) && n.id().0 >= model.processing_ids()[0].0).map(|n|n.id().0).collect::<Vec<_>>()})
}
fn excitability(c: &Config, seed: u64) -> Result<TrialMetrics> {
    let (shared, _) = train(c, seed, "B", false)?;
    let initial = shared.weights.clone();
    let natural = get_weights(&shared.model);
    let (g, _) = train(c, seed, "G", false)?;
    let (d, _) = train(c, seed, "D", false)?;
    let core = initial
        .iter()
        .zip(&natural)
        .map(|((id, w), (_, learned))| {
            let edge = shared.model.network().synapse(*id).unwrap();
            let internal = shared.model.processing_ids().contains(&edge.pre())
                && shared.model.processing_ids().contains(&edge.post());
            (*id, if internal { *learned } else { *w })
        })
        .collect::<Vec<_>>();
    let vectors = BTreeMap::from([
        ("reset", initial.clone()),
        ("B", natural),
        ("B_core", core),
        ("G", get_weights(&g.model)),
        ("D", get_weights(&d.model)),
    ]);
    let (mut target, _) = train(c, seed, "B", false)?;
    if !shared.model.same_replay_state(&target.model) {
        return Err(error("MLD03 target checkpoint differs"));
    }
    verify_only_weights(&shared.model, &mut target.model, &initial)?;
    let reference = observe(&mut target.model, c)?;
    let targets = vec![reference.counts[1].clone(), reference.counts[2].clone()];
    let mut rows = BTreeMap::new();
    let mut effects = 0.;
    let mut candidates = 0.;
    for (name, theta, tau) in DYNAMICS {
        let mut probes = BTreeMap::new();
        let mut applied_weights = BTreeMap::new();
        let mut parameters = Value::Null;
        for (arm, weights) in &vectors {
            let (mut branch, _) = train(c, seed, "B", false)?;
            if !shared.model.same_replay_state(&branch.model) {
                return Err(error("MLD03 weight fork checkpoint differs"));
            }
            verify_only_weights(&shared.model, &mut branch.model, weights)?;
            branch
                .model
                .diagnostic_core_dynamics(theta, tau)
                .map_err(fail)?;
            let current = json!(
                branch
                    .model
                    .processing_ids()
                    .iter()
                    .map(|id| {
                        let n = branch.model.network().neuron(*id).unwrap();
                        json!([
                            id.0,
                            n.base_threshold(),
                            n.threshold(),
                            n.params().membrane_tau_us
                        ])
                    })
                    .collect::<Vec<_>>()
            );
            if parameters != Value::Null && parameters != current {
                return Err(error("MLD03 Core parameters differ across weight arms"));
            }
            parameters = current;
            applied_weights.insert(
                *arm,
                get_weights(&branch.model)
                    .into_iter()
                    .map(|(id, w)| (id.0, w))
                    .collect::<Vec<_>>(),
            );
            probes.insert(*arm, observe(&mut branch.model, c)?);
        }
        let reset = probes.remove("reset").unwrap();
        let mut arms = BTreeMap::new();
        for (arm, p) in probes {
            let row = measured(p, &reset, &targets);
            if row["core_effect"] == true {
                effects += 1.;
            }
            if row["all_cues_success"] == true {
                candidates += 1.;
            }
            arms.insert(arm, row);
        }
        rows.insert(name,json!({"threshold_scale":theta,"tau_scale":tau,"core_parameters":parameters,"applied_weights":applied_weights,"reset_probe":reset.details,"arms":arms}));
    }
    finish(
        34.,
        effects,
        candidates,
        0.,
        json!({"protocol":"NERVA-MLD-03/1","section":"excitability","shared_checkpoint_verified":true,
        "populations":populations(&shared.model),"weights_initial":initial.iter().map(|(i,w)|(i.0,*w)).collect::<Vec<_>>(),
        "edges":shared.model.network().synapses().map(|s|(s.id().0,s.pre().0,s.post().0)).collect::<Vec<_>>(),
        "weight_vectors":vectors.into_iter().map(|(name,w)|(name,w.into_iter().map(|(i,w)|(i.0,w)).collect::<Vec<_>>())).collect::<BTreeMap<_,_>>(),
        "fixed_targets":targets,"reference_probe":reference.details,"conditions":rows,
        "limits":"Diagnostic Core perturbations, not learned excitability. First effect only on the fixed grid, not a minimum threshold estimate. Same B cellular state for all weight vectors; travelling amplitudes persist. Novel cues are fragments of the same task."}),
    )
}
fn population(c: &Config, seed: u64) -> Result<TrialMetrics> {
    let (shared, development) = interaction::checkpoint(c, seed)?;
    let initial = get_weights(&shared);
    let (mut before, _) = interaction::checkpoint(c, seed)?;
    if !shared.same_replay_state(&before) {
        return Err(error("MLD03 developed baseline differs"));
    }
    let baseline = observe(&mut before, c)?;
    let targets = vec![baseline.counts[1].clone(), baseline.counts[2].clone()];
    let mut arms = BTreeMap::new();
    for (name, scope, enabled) in [
        ("off", HomeostasisScope::CoreSensor, false),
        ("sensor", HomeostasisScope::Sensor, true),
        ("core", HomeostasisScope::Core, true),
        ("both", HomeostasisScope::CoreSensor, true),
        ("all_bridge", HomeostasisScope::All, true),
    ] {
        let (mut learned, _) = interaction::checkpoint(c, seed)?;
        let (mut reset, _) = interaction::checkpoint(c, seed)?;
        if !shared.same_replay_state(&learned) || !shared.same_replay_state(&reset) {
            return Err(error("MLD03 population checkpoints differ"));
        }
        learned
            .set_checkpoint_homeostasis_scope(scope)
            .map_err(fail)?;
        reset
            .set_checkpoint_homeostasis_scope(scope)
            .map_err(fail)?;
        let training = interaction::training(&mut learned, c, seed, enabled, false, true)?;
        interaction::training(&mut reset, c, seed, enabled, false, false)?;
        if !learned.same_replay_state(&reset) {
            return Err(error("MLD03 population observer changed state"));
        }
        verify_only_weights(&learned, &mut reset, &initial)?;
        let without = observe(&mut reset, c)?;
        let with = observe(&mut learned, c)?;
        let m = margins(&with, &targets);
        let own = measured(with, &without, &targets);
        arms.insert(name,json!({"scope":format!("{scope:?}"),"homeostasis":enabled,"common_checkpoint_verified":true,
            "training":training,"margins":m,"own_weight_effect":own,"reset_probe":without.details}));
    }
    let contrast = (0..6)
        .map(|i| {
            arms["both"]["margins"][i].as_f64().unwrap()
                - arms["sensor"]["margins"][i].as_f64().unwrap()
                - arms["core"]["margins"][i].as_f64().unwrap()
                + arms["off"]["margins"][i].as_f64().unwrap()
        })
        .collect::<Vec<_>>();
    finish(
        22.,
        0.,
        0.,
        contrast[0],
        json!({"protocol":"NERVA-MLD-03/1","section":"population","shared_checkpoint_verified":true,
        "development":development,"populations":populations(&shared),"baseline_probe":baseline.details,"fixed_targets":targets,"arms":arms,"interaction":contrast,
        "limits":"Total population-controller effects include learning feedback and cellular adaptation. Both means Sensor+Core; all_bridge additionally includes other roles. No competition after development. Shared developed state already contains development-phase All homeostasis; this diagnoses subsequent training, not development origins."}),
    )
}
fn finish(
    passes: f64,
    effects: f64,
    candidates: f64,
    contrast: f64,
    details: Value,
) -> Result<TrialMetrics> {
    Ok(TrialMetrics {
        values: METRICS
            .into_iter()
            .map(str::to_owned)
            .zip([passes, effects, candidates, contrast])
            .collect(),
        details,
    })
}
impl Scenario for Mld03Scenario {
    type Config = Config;
    fn name(&self) -> &'static str {
        "hippocampal_mld03"
    }
    fn default_config(&self) -> Config {
        AssociationScenario.default_config()
    }
    fn set_parameter(&self, c: &mut Config, n: &str, v: &Value) -> Result<()> {
        HippocampalScenario.set_parameter(c, n, v)
    }
    fn validate(&self, c: &Config) -> Result<()> {
        AssociationScenario.validate(c)
    }
    fn interventions(&self) -> Vec<Intervention> {
        ["excitability", "population"]
            .into_iter()
            .map(|name| Intervention { name: name.into() })
            .collect()
    }
    fn metric_names(&self) -> Vec<&'static str> {
        METRICS.to_vec()
    }
    fn snapshot(&self, c: &Config, seed: u64, i: &Intervention) -> Value {
        json!({"protocol":"NERVA-MLD-03/1","seed":seed,"section":i.name,
        "processing_neurons":c.processing_neurons,"training_repetitions":c.training_repetitions,"rest_us":c.rest_us,"dynamics":DYNAMICS,
        "weight_arms":["reset","B","B_core","G","D"],"population_arms":["off","sensor","core","both","all_bridge"],"probes":images::PROBES,
        "epsilon":1e-9,"training_passes_per_execution":if i.name=="population"{22}else{34}})
    }
    fn run(&self, c: &Config, seed: u64, i: &Intervention) -> Result<TrialMetrics> {
        self.validate(c)?;
        match i.name.as_str() {
            "excitability" => excitability(c, seed),
            "population" => population(c, seed),
            _ => Err(error("unknown MLD03 section")),
        }
    }
    fn evaluate(&self, rows: &[Value], replay: bool) -> Result<Value> {
        Ok(
            json!({"status":"mld03_paired_followup","passed":null,"records":rows.len(),"replay_checked":replay}),
        )
    }
}
