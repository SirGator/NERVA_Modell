//! Prospective A/B acquisition with local lead-based formation and causal lag control.
use super::{
    Assay, Config, ContingencyScenario, MAPPINGS, OUTCOMES, OUTCOMES_B, TrainingAudit, checkpoint,
    decisions, exact_fork, fail, frames, patterns, stage, templates, topology,
};
use crate::{
    Result, error,
    scenario::{Intervention, Scenario, TrialMetrics},
};
use nerva_vm::model::ExploratoryFormation;
use serde_json::{Value, json};
use std::collections::BTreeMap;

pub struct LeadGrowthScenario;
const PROTOCOL: &str = "NERVA-Contingency-Lead-Growth/1";
const INTERVENTION: &str = "crossed_lead_growth";
const POLICIES: [(&str, ExploratoryFormation); 3] = [
    ("recent", ExploratoryFormation::RecentExcitatory),
    ("lead", ExploratoryFormation::LeadingExcitatory),
    ("lagged", ExploratoryFormation::LaggedLeadingExcitatory),
];
const METRICS: [&str; 12] = [
    "recent_a_acquisition",
    "recent_b_acquisition",
    "recent_a_retention",
    "recent_joint_paths",
    "lead_a_acquisition",
    "lead_b_acquisition",
    "lead_a_retention",
    "lead_joint_paths",
    "lagged_a_acquisition",
    "lagged_b_acquisition",
    "lagged_a_retention",
    "lagged_joint_paths",
];

fn layout(seed: u64) -> (&'static str, BTreeMap<&'static str, Vec<usize>>) {
    let interleave = seed % 4 == 3;
    let rows = [0, 2, 4, 6, 1, 3, 5, 7];
    let mapped = patterns()
        .into_iter()
        .map(|(name, pixels)| {
            let mut pixels: Vec<_> = pixels
                .into_iter()
                .map(|pixel| {
                    let (row, col) = (pixel / 8, pixel % 8);
                    if interleave {
                        8 * rows[row] + 7 - col
                    } else {
                        8 * col + 7 - row
                    }
                })
                .collect();
            pixels.sort_unstable();
            (name, pixels)
        })
        .collect();
    (if interleave { "interleave" } else { "rotate" }, mapped)
}

fn lead_rule() -> Value {
    json!({"sample_interval_us":10000,"min_lead_us":1000,"max_lead_us":75000,
        "lag_us":157000,"decay_us":2000000,"age_tiers_us":[15000,40000,75000],"age_scores":[3,2,1]})
}

fn run(config: &Config, seed: u64) -> Result<TrialMetrics> {
    let reference = checkpoint(config, seed)?.0;
    let checkpoint_time_us = reference.next_observation_us();
    let initial_topology = topology(&reference);
    let core_ids: Vec<_> = reference.processing_ids().iter().map(|id| id.0).collect();
    let (layout_name, stimuli) = layout(seed);
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
            .map(|(name, r)| (name, r.json()))
            .collect::<BTreeMap<_, _>>()
    );
    let start = assay.start_training("native")?;
    let mut trials = BTreeMap::new();
    let mut checks = BTreeMap::new();
    let mut values = BTreeMap::new();
    for (policy, mode) in POLICIES {
        let mut policy_start = exact_fork(&start)?;
        policy_start
            .diagnostic_switch_exploratory_formation(mode)
            .map_err(fail)?;
        let mut a = BTreeMap::new();
        let mut b = BTreeMap::new();
        for a_mapping in MAPPINGS {
            let mut after_a = exact_fork(&policy_start)?;
            let rows = assay.train(&mut after_a, a_mapping, 0, TrainingAudit::Lead)?;
            a.insert(a_mapping, stage(&assay, &after_a, rows, policy)?);
            let mut branches = BTreeMap::new();
            for b_mapping in MAPPINGS {
                let mut branch = exact_fork(&after_a)?;
                let rows = assay.train(&mut branch, b_mapping, 1, TrainingAudit::Lead)?;
                branches.insert(b_mapping, stage(&assay, &branch, rows, policy)?);
            }
            b.insert(a_mapping, branches);
        }
        let a = json!(a);
        let wrapped_b = json!({policy:b});
        let (mut scores, scoring) = decisions(&a, &wrapped_b, &reset, &targets, &[policy]);
        let first = scores
            .remove("a_acquisition")
            .expect("shared A acquisition score");
        values.insert(format!("{policy}_a_acquisition"), first);
        values.append(&mut scores);
        checks.insert(policy,json!({"a_acquisition":scoring["a_acquisition"],
            "b_acquisition":scoring["b_acquisition"][policy],"a_retention":scoring["a_retention"][policy]}));
        trials.insert(policy, json!({"a":a,"b":wrapped_b[policy]}));
    }
    let details = json!({"protocol":PROTOCOL,"seed":seed,"checkpoint_time_us":checkpoint_time_us,
        "core_ids":core_ids,"initial_topology":initial_topology,"stimuli":stimuli,"layout":layout_name,
        "reset":reset,"targets":{"a":targets[0],"b":targets[1]},"trials":trials,"checks":checks});
    Ok(TrialMetrics { values, details })
}

impl Scenario for LeadGrowthScenario {
    type Config = Config;
    fn name(&self) -> &'static str {
        "hippocampal_contingency_lead_growth"
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
        let (layout_name, stimuli) = layout(seed);
        json!({"protocol":PROTOCOL,"seed":seed,"intervention":intervention.name,
            "processing_neurons":config.processing_neurons,"training_repetitions":config.training_repetitions,
            "rest_us":config.rest_us,"stimuli":stimuli,"layout":layout_name,
            "formation_policies":POLICIES.map(|(name,_)|name),"lead_rule":lead_rule()})
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
            return Err(error("unknown lead growth intervention"));
        }
        run(config, seed)
    }
}
