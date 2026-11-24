//! Cross independent B histories on live, exactly forked A states.
use super::{
    Assay, CUES_B, Config, ContingencyScenario, MAPPINGS, Model, OUTCOMES, OUTCOMES_B, PROBES,
    Templates, TrainingAudit, checkpoint, fail, frames, passes, patterns, score_mappings,
    templates, topology,
};
use crate::{
    Result, error,
    scenario::{Intervention, Scenario, TrialMetrics},
};
use nerva_vm::model::ExploratoryFormation;
use serde_json::{Value, json};
use std::collections::BTreeMap;

mod transfer;
pub use transfer::{replay_a_source, replay_a_timing, replay_a_transfer, replay_b_source};
pub mod lead;

pub struct LiveGrowthScenario;
const PROTOCOL: &str = "NERVA-Contingency-Live-Growth/1";
const POLICIES: [&str; 3] = ["recent", "nearest", "off"];
const INTERVENTION: &str = "crossed_b_formation";
const METRICS: [&str; 10] = [
    "a_acquisition",
    "recent_b_acquisition",
    "nearest_b_acquisition",
    "off_b_acquisition",
    "recent_a_retention",
    "nearest_a_retention",
    "off_a_retention",
    "recent_joint_paths",
    "nearest_joint_paths",
    "off_joint_paths",
];

fn exact_fork(model: &Model) -> Result<Model> {
    let fork = model.diagnostic_fork().map_err(fail)?;
    if !model.same_replay_state(&fork) {
        return Err(error("growth checkpoint fork differs in replay state"));
    }
    Ok(fork)
}

fn probes(assay: &Assay<'_>, model: &Model) -> Result<Value> {
    let mut cohort = BTreeMap::new();
    let mut frozen = exact_fork(model)?;
    frozen.freeze().map_err(fail)?;
    for cue in PROBES {
        let mut branch = exact_fork(&frozen)?;
        cohort.insert(cue, assay.read(&mut branch, cue)?.json());
    }
    Ok(json!(cohort))
}

fn stage(assay: &Assay<'_>, model: &Model, frames: Value, policy: &str) -> Result<Value> {
    Ok(
        json!({"frames":frames,"end_topology":topology(model),"probes":probes(assay,model)?,
        "formation_policy":policy}),
    )
}

fn follow_b(assay: &Assay<'_>, after_a: &Model, policy: &str, mapping: &str) -> Result<Value> {
    let mut branch = exact_fork(after_a)?;
    match policy {
        "recent" => {}
        "nearest" => branch
            .diagnostic_switch_exploratory_formation(ExploratoryFormation::NearestExcitatory)
            .map_err(fail)?,
        "off" => branch.diagnostic_pause_development().map_err(fail)?,
        _ => return Err(error("unknown phase-B formation policy")),
    }
    let plasticity = branch.plasticity();
    if !plasticity.learning
        || !plasticity.homeostasis
        || plasticity.development != (policy != "off")
    {
        return Err(error(
            "phase-B formation switch changed learning or homeostasis",
        ));
    }
    let rows = assay.train(&mut branch, mapping, 1, TrainingAudit::Formation)?;
    stage(assay, &branch, rows, policy)
}

fn decisions(
    stages_a: &Value,
    branches_b: &Value,
    reset: &Value,
    targets: &[Templates; 2],
    policies: &[&str],
) -> (BTreeMap<String, f64>, Value) {
    let a = MAPPINGS.map(|m| &stages_a[m]["probes"]);
    let a_acquisition = score_mappings(a, reset, &targets[0], 0, None, None);
    let mut metrics = BTreeMap::from([("a_acquisition".into(), passes(&a_acquisition) as f64)]);
    let mut b_gates = BTreeMap::new();
    let mut retention_gates = BTreeMap::new();
    for &policy in policies {
        let arms = &branches_b[policy];
        let mut b = BTreeMap::new();
        let mut retain = BTreeMap::new();
        for history in MAPPINGS {
            let after_b = MAPPINGS.map(|m| &arms[history][m]["probes"]);
            let fixed_a = &stages_a[history]["probes"];
            b.insert(
                history,
                score_mappings(after_b, reset, &targets[1], 1, Some([fixed_a; 2]), None),
            );
            let matched_b = MAPPINGS.map(|m| &arms[m][history]["probes"]);
            retain.insert(
                history,
                score_mappings(matched_b, reset, &targets[0], 0, None, Some(&a_acquisition)),
            );
        }
        let mut joint_paths = 0;
        for a_map in &MAPPINGS[..2] {
            for b_map in &MAPPINGS[..2] {
                let retained = super::CUES
                    .iter()
                    .all(|cue| retain[b_map][a_map][cue]["passed"] == true);
                let acquired = CUES_B
                    .iter()
                    .all(|cue| b[a_map][b_map][cue]["passed"] == true);
                joint_paths += usize::from(retained && acquired);
            }
        }
        metrics.insert(
            format!("{policy}_b_acquisition"),
            b.values().map(passes).sum::<usize>() as f64,
        );
        metrics.insert(
            format!("{policy}_a_retention"),
            retain.values().map(passes).sum::<usize>() as f64,
        );
        metrics.insert(format!("{policy}_joint_paths"), joint_paths as f64);
        b_gates.insert(policy, b);
        retention_gates.insert(policy, retain);
    }
    (
        metrics,
        json!({"a_acquisition":a_acquisition,"b_acquisition":b_gates,"a_retention":retention_gates}),
    )
}

fn run(config: &Config, seed: u64) -> Result<TrialMetrics> {
    let reference = checkpoint(config, seed)?.0;
    let initial_topology = topology(&reference);
    let checkpoint_time_us = reference.next_observation_us();
    let core_ids: Vec<_> = reference.processing_ids().iter().map(|id| id.0).collect();
    let stimuli = patterns();
    let assay = Assay {
        config,
        seed,
        reference,
        images: frames(&stimuli)?,
    };
    let reset = assay.reset()?;
    let targets = [templates(&reset, OUTCOMES), templates(&reset, OUTCOMES_B)];
    let reset = json!(
        reset
            .into_iter()
            .map(|(name, response)| (name, response.json()))
            .collect::<BTreeMap<_, _>>()
    );
    let developed = assay.start_training("native")?;
    let mut stages_a = BTreeMap::new();
    let mut stages_b = BTreeMap::new();
    for a_mapping in MAPPINGS {
        let mut after_a = exact_fork(&developed)?;
        let rows = assay.train(&mut after_a, a_mapping, 0, TrainingAudit::Formation)?;
        stages_a.insert(a_mapping, stage(&assay, &after_a, rows, "recent")?);
        for policy in POLICIES {
            let mapping = MAPPINGS
                .into_iter()
                .map(|b_mapping| Ok((b_mapping, follow_b(&assay, &after_a, policy, b_mapping)?)))
                .collect::<Result<BTreeMap<_, _>>>()?;
            stages_b
                .entry(policy)
                .or_insert_with(BTreeMap::new)
                .insert(a_mapping, mapping);
        }
    }
    let stages_a = json!(stages_a);
    let branches_b = json!(stages_b);
    let (values, checks) = decisions(&stages_a, &branches_b, &reset, &targets, &POLICIES);
    let details = json!({"protocol":PROTOCOL,"seed":seed,"core_ids":core_ids,
        "checkpoint_time_us":checkpoint_time_us,"stimuli":stimuli,"initial_topology":initial_topology,
        "reset":reset,"targets":{"a":targets[0],"b":targets[1]}, "stages_a":stages_a,
        "branches_b":branches_b,"checks":checks});
    Ok(TrialMetrics { values, details })
}

impl Scenario for LiveGrowthScenario {
    type Config = Config;
    fn name(&self) -> &'static str {
        "hippocampal_contingency_live_growth"
    }
    fn default_config(&self) -> Config {
        ContingencyScenario.default_config()
    }
    fn validate(&self, config: &Config) -> Result<()> {
        ContingencyScenario.validate(config)
    }
    fn set_parameter(&self, config: &mut Config, name: &str, value: &Value) -> Result<()> {
        ContingencyScenario.set_parameter(config, name, value)
    }
    fn snapshot(&self, config: &Config, seed: u64, intervention: &Intervention) -> Value {
        json!({"protocol":PROTOCOL,"seed":seed,"intervention":intervention.name,
            "processing_neurons":config.processing_neurons,"training_repetitions":config.training_repetitions,
            "rest_us":config.rest_us,"stimuli":patterns(),"formation_policies":POLICIES,
            "a_formation":"recent","pruning":false,"homeostasis_scope":"core"})
    }
    fn interventions(&self) -> Vec<Intervention> {
        vec![Intervention {
            name: INTERVENTION.into(),
        }]
    }
    fn metric_names(&self) -> Vec<&'static str> {
        METRICS.to_vec()
    }
    fn run(&self, config: &Config, seed: u64, intervention: &Intervention) -> Result<TrialMetrics> {
        self.validate(config)?;
        if intervention.name != INTERVENTION {
            return Err(error("unknown live growth intervention"));
        }
        run(config, seed)
    }
}
