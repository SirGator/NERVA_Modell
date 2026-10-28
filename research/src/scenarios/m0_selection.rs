//! Controlled mechanism switches and delayed probes for external selection.
use super::m0::{M0Scenario, collect_native, parameter_names};
use crate::{
    Result, error,
    scenario::{Intervention, Scenario, TrialMetrics},
};
use nerva::experiment::{M0Experiment, M0ExperimentConfig, M0Group, M0GroupResult};
use serde_json::{Value, json};

pub struct M0SelectionScenario;
#[derive(Clone)]
pub struct Config {
    pub native: M0ExperimentConfig,
    pub stdp: bool,
    pub homeostasis: bool,
    pub retention_delay_us: u64,
}

fn flags(c: &Config, name: &str) -> Result<(M0Group, bool, bool)> {
    Ok(match name {
        "none" => (M0Group::OrderedLearning, c.stdp, c.homeostasis),
        "disable_stdp" => (M0Group::OrderedFixed, false, c.homeostasis),
        "shuffle_input" => (M0Group::RandomLearning, c.stdp, c.homeostasis),
        "disable_homeostasis" => (M0Group::OrderedLearning, c.stdp, false),
        _ => return Err(error("unsupported selection intervention")),
    })
}
fn native(c: &Config, seed: u64, i: &Intervention, delay: u64) -> Result<M0GroupResult> {
    let (group, stdp, homeostasis) = flags(c, &i.name)?;
    let mut n = c.native.clone();
    n.nerva.network.seed = seed;
    M0Experiment::new(n, group)?.run_controlled(stdp, homeostasis, delay)
}
fn pair(c: &Config, seed: u64, i: &Intervention) -> Result<(M0GroupResult, M0GroupResult)> {
    Ok((
        native(c, seed, i, 0)?,
        native(c, seed, i, c.retention_delay_us)?,
    ))
}
fn metrics(a: M0GroupResult, b: M0GroupResult) -> Result<TrialMetrics> {
    let count = a.event_log.len() + b.event_log.len();
    let mut first = collect_native(a)?;
    let retained = collect_native(b)?;
    for (key, value) in retained.values {
        first.values.insert(format!("retained_{key}"), value);
    }
    first.values.insert("event_count".to_owned(), count as f64);
    first.details = json!({"immediate":first.details,"retained":retained.details,
        "retention_scope":"Frozen learned weights and existing cellular state after additional quiet delay; no continuing plasticity or interference"});
    Ok(first)
}
impl Scenario for M0SelectionScenario {
    type Config = Config;
    fn name(&self) -> &'static str {
        "m0_selection"
    }
    fn default_config(&self) -> Config {
        Config {
            native: M0ExperimentConfig::default(),
            stdp: true,
            homeostasis: false,
            retention_delay_us: 2_000_000,
        }
    }
    fn set_parameter(&self, c: &mut Config, name: &str, value: &Value) -> Result<()> {
        match name {
            "mechanisms.stdp" => {
                c.stdp = value
                    .as_bool()
                    .ok_or_else(|| error("mechanisms.stdp must be boolean"))?
            }
            "mechanisms.homeostasis" => {
                c.homeostasis = value
                    .as_bool()
                    .ok_or_else(|| error("mechanisms.homeostasis must be boolean"))?
            }
            "retention_delay_us" => {
                c.retention_delay_us = value
                    .as_u64()
                    .filter(|&n| n > 0 && n <= 10_000_000)
                    .ok_or_else(|| error("retention_delay_us must be 1..10000000"))?
            }
            _ => M0Scenario.set_parameter(&mut c.native, name, value)?,
        }
        Ok(())
    }
    fn validate(&self, c: &Config) -> Result<()> {
        if c.retention_delay_us == 0 || c.retention_delay_us > 10_000_000 {
            return Err(error("retention_delay_us must be 1..10000000"));
        }
        M0Scenario.validate(&c.native)
    }
    fn snapshot(&self, c: &Config, seed: u64, i: &Intervention) -> Value {
        let (_, stdp, homeostasis) = flags(c, &i.name).expect("validated intervention");
        let mut n = c.native.clone();
        n.nerva.network.seed = seed;
        n.nerva.learning.enabled = stdp;
        n.nerva.learning.homeostasis.enabled = homeostasis;
        json!({"native_type":"M0ExperimentConfig","native_config":format!("{n:?}"),"seed":seed,
            "intervention":i.name,"learning_enabled":stdp,"homeostasis_enabled":homeostasis,
            "retention_delay_us":c.retention_delay_us})
    }
    fn interventions(&self) -> Vec<Intervention> {
        [
            "none",
            "disable_stdp",
            "shuffle_input",
            "disable_homeostasis",
        ]
        .iter()
        .map(|s| Intervention {
            name: (*s).to_owned(),
        })
        .collect()
    }
    fn metric_names(&self) -> Vec<&'static str> {
        let mut names = M0Scenario.metric_names();
        names.extend([
            "retained_sequence_score",
            "retained_stability",
            "retained_firing_rate",
            "retained_training_rate_hz",
            "retained_transition_hits",
            "retained_false_transitions",
            "retained_silent_fraction",
            "retained_persistent_fraction",
            "retained_frozen_replay_identical",
            "retained_weights_frozen",
            "retained_latency_hits",
            "retained_rates_finite",
            "event_count",
        ]);
        names
    }
    fn run(&self, c: &Config, seed: u64, i: &Intervention) -> Result<TrialMetrics> {
        let (a, b) = pair(c, seed, i)?;
        metrics(a, b)
    }
    fn run_verified(&self, c: &Config, seed: u64, i: &Intervention) -> Result<TrialMetrics> {
        let first = pair(c, seed, i)?;
        let second = pair(c, seed, i)?;
        if first != second {
            return Err(error("controlled M0 full replay differs"));
        }
        metrics(first.0, first.1)
    }
    fn evaluate(&self, _records: &[Value], _full_replay: bool) -> Result<Value> {
        Ok(
            json!({"status":"selection_policy_evaluated_externally","passed":null,
            "limits":"Controlled mechanism profile; distinct from the fixed G1-G4 native M0 study"}),
        )
    }
}
pub fn selection_parameter_names() -> Vec<&'static str> {
    let mut names = parameter_names().to_vec();
    names.extend([
        "mechanisms.stdp",
        "mechanisms.homeostasis",
        "retention_delay_us",
    ]);
    names
}
