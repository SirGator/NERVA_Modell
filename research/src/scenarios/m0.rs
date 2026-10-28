//! Adapter for the unchanged native M0 sequence and its four controlled groups.
use crate::{
    Result, error,
    scenario::{Intervention, Scenario, TrialMetrics},
};
use nerva::experiment::{M0Experiment, M0ExperimentConfig, M0Group, M0GroupResult};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

pub struct M0Scenario;
const GROUPS: [&str; 4] = [
    "none",
    "disable_stdp",
    "shuffle_input",
    "enable_homeostasis",
];
const PARAMETERS: &[&str] = &[
    "learning.a_plus",
    "learning.a_minus",
    "learning.tau_plus_us",
    "learning.tau_minus_us",
    "learning.stdp_window_us",
    "learning.homeostasis.target_rate_hz",
    "learning.homeostasis.target_input_rate",
    "learning.homeostasis.update_interval_us",
    "learning.homeostasis.intrinsic_adjustment_rate",
    "learning.homeostasis.structural_adjustment_rate",
    "neuron.threshold",
    "neuron.membrane_tau_us",
    "neuron.refractory_period_us",
    "training_repetitions",
    "symbol_interval_us",
    "pause_us",
    "sequence_delay_us",
    "sensory_weight",
    "recurrent_weight",
    "recurrent_weight_jitter",
    "inhibitory_drive_weight",
    "inhibitory_feedback_weight",
    "inhibitory_delay_us",
    "probe_duration_us",
    "probe_tolerance_us",
    "quiet_window_us",
];

fn group(name: &str) -> Result<M0Group> {
    Ok(match name {
        "none" => M0Group::OrderedLearning,
        "disable_stdp" => M0Group::OrderedFixed,
        "shuffle_input" => M0Group::RandomLearning,
        "enable_homeostasis" => M0Group::OrderedHomeostasis,
        _ => return Err(error(format!("unsupported M0 intervention: {name}"))),
    })
}
fn effective(base: &M0ExperimentConfig, seed: u64, name: &str) -> M0ExperimentConfig {
    let mut c = base.clone();
    c.nerva.network.seed = seed;
    c.nerva.learning.enabled = name != "disable_stdp";
    c.nerva.learning.homeostasis.enabled = name == "enable_homeostasis";
    c
}
fn float(v: &Value) -> Result<f32> {
    v.as_f64()
        .map(|x| x as f32)
        .filter(|x| x.is_finite())
        .ok_or_else(|| error("parameter requires a finite f32 number"))
}
fn integer(v: &Value) -> Result<u64> {
    v.as_u64()
        .ok_or_else(|| error("parameter requires a nonnegative integer"))
}

impl Scenario for M0Scenario {
    type Config = M0ExperimentConfig;
    fn name(&self) -> &'static str {
        "m0_sequence"
    }
    fn default_config(&self) -> Self::Config {
        M0ExperimentConfig::default()
    }
    fn set_parameter(&self, c: &mut Self::Config, name: &str, value: &Value) -> Result<()> {
        match name {
            "learning.a_plus" => c.nerva.learning.a_plus = float(value)?,
            "learning.a_minus" => c.nerva.learning.a_minus = float(value)?,
            "learning.tau_plus_us" => c.nerva.learning.tau_plus_us = float(value)?,
            "learning.tau_minus_us" => c.nerva.learning.tau_minus_us = float(value)?,
            "learning.stdp_window_us" => c.nerva.learning.stdp_window_us = integer(value)?,
            "learning.homeostasis.target_rate_hz" => {
                c.nerva.learning.homeostasis.target_rate_hz = float(value)?
            }
            "learning.homeostasis.target_input_rate" => {
                c.nerva.learning.homeostasis.target_input_rate = float(value)?
            }
            "learning.homeostasis.update_interval_us" => {
                c.nerva.learning.homeostasis.update_interval_us = integer(value)?
            }
            "learning.homeostasis.intrinsic_adjustment_rate" => {
                c.nerva.learning.homeostasis.intrinsic_adjustment_rate = float(value)?
            }
            "learning.homeostasis.structural_adjustment_rate" => {
                c.nerva.learning.homeostasis.structural_adjustment_rate = float(value)?
            }
            "neuron.threshold" => c.nerva.neuron.threshold = float(value)?,
            "neuron.membrane_tau_us" => c.nerva.neuron.membrane_tau_us = float(value)?,
            "neuron.refractory_period_us" => c.nerva.neuron.refractory_period_us = integer(value)?,
            "training_repetitions" => {
                c.training_repetitions =
                    usize::try_from(integer(value)?).map_err(|e| error(e.to_string()))?
            }
            "symbol_interval_us" => c.symbol_interval_us = integer(value)?,
            "pause_us" => c.pause_us = integer(value)?,
            "sequence_delay_us" => c.sequence_delay_us = integer(value)?,
            "sensory_weight" => c.sensory_weight = float(value)?,
            "recurrent_weight" => c.recurrent_weight = float(value)?,
            "recurrent_weight_jitter" => c.recurrent_weight_jitter = float(value)?,
            "inhibitory_drive_weight" => c.inhibitory_drive_weight = float(value)?,
            "inhibitory_feedback_weight" => c.inhibitory_feedback_weight = float(value)?,
            "inhibitory_delay_us" => c.inhibitory_delay_us = integer(value)?,
            "probe_duration_us" => c.probe_duration_us = integer(value)?,
            "probe_tolerance_us" => c.probe_tolerance_us = integer(value)?,
            "quiet_window_us" => c.quiet_window_us = integer(value)?,
            _ => return Err(error(format!("unknown M0 parameter: {name}"))),
        }
        Ok(())
    }
    fn validate(&self, c: &Self::Config) -> Result<()> {
        c.validate()?;
        if c.recurrent_weight_jitter <= 0.0 {
            return Err(error(
                "M0 studies require positive recurrent_weight_jitter for seeded independent states",
            ));
        }
        Ok(())
    }
    fn snapshot(&self, base: &Self::Config, seed: u64, intervention: &Intervention) -> Value {
        let c = effective(base, seed, &intervention.name);
        json!({"native_type":"M0ExperimentConfig","seed":seed,"intervention":intervention.name,
            "native_config":format!("{c:?}"),"learning_enabled":c.nerva.learning.enabled,
            "homeostasis_enabled":c.nerva.learning.homeostasis.enabled})
    }
    fn interventions(&self) -> Vec<Intervention> {
        GROUPS
            .iter()
            .map(|s| Intervention {
                name: (*s).to_owned(),
            })
            .collect()
    }
    fn metric_names(&self) -> Vec<&'static str> {
        vec![
            "sequence_score",
            "stability",
            "firing_rate",
            "training_rate_hz",
            "transition_hits",
            "false_transitions",
            "silent_fraction",
            "persistent_fraction",
            "frozen_replay_identical",
            "weights_frozen",
            "latency_hits",
            "rates_finite",
        ]
    }
    fn run(
        &self,
        base: &Self::Config,
        seed: u64,
        intervention: &Intervention,
    ) -> Result<TrialMetrics> {
        collect_native(run_native(base, seed, intervention)?)
    }
    fn run_verified(
        &self,
        base: &Self::Config,
        seed: u64,
        intervention: &Intervention,
    ) -> Result<TrialMetrics> {
        let first = run_native(base, seed, intervention)?;
        let second = run_native(base, seed, intervention)?;
        if first != second {
            return Err(error(
                "native M0 full replay differs, including event streams",
            ));
        }
        collect_native(first)
    }
    fn evaluate(&self, records: &[Value], full_replay: bool) -> Result<Value> {
        let mut indexed = BTreeMap::new();
        let mut seeds = BTreeSet::new();
        for r in records {
            let seed = r["job"]["seed"].as_u64().unwrap();
            seeds.insert(seed);
            indexed.insert((seed, r["job"]["intervention"].as_str().unwrap()), r);
        }
        if !full_replay
            || seeds
                .iter()
                .any(|&s| GROUPS.iter().any(|g| !indexed.contains_key(&(s, *g))))
        {
            return Ok(json!({"status":"missing_native_controls_or_full_replay","passed":null}));
        }
        let value = |seed, group, metric: &str| {
            indexed[&(seed, group)]["metrics"][metric].as_f64().unwrap()
        };
        let fixed: Vec<_> = seeds
            .iter()
            .map(|&s| {
                value(s, "none", "sequence_score") - value(s, "disable_stdp", "sequence_score")
            })
            .collect();
        let random: Vec<_> = seeds
            .iter()
            .map(|&s| {
                value(s, "none", "sequence_score") - value(s, "shuffle_input", "sequence_score")
            })
            .collect();
        let all_stable = records.iter().all(|r| r["metrics"]["stability"] == 1.0);
        let all_replays = records.iter().all(|r| {
            r["full_replay_checked"] == true
                && r["metrics"]["frozen_replay_identical"] == 1.0
                && r["metrics"]["weights_frozen"] == 1.0
        });
        let complete = seeds.iter().all(|&s| {
            value(s, "none", "transition_hits") == 3.0
                && value(s, "none", "false_transitions") == 0.0
                && value(s, "none", "latency_hits") == 3.0
        });
        let noncollapse = records.iter().all(|r| {
            r["metrics"]["persistent_fraction"] == 0.0
                && r["metrics"]["rates_finite"] == 1.0
                && r["metrics"]["silent_fraction"].as_f64().unwrap() < 1.0
        });
        let mean = |v: &[f64]| v.iter().sum::<f64>() / v.len() as f64;
        let p_fixed = sign_p(&fixed);
        let p_random = sign_p(&random);
        Ok(
            json!({"status":"native_m0_criterion","seeds":seeds.len(),"wins_over_fixed":fixed.iter().filter(|&&v|v>0.0).count(),
            "wins_over_random":random.iter().filter(|&&v|v>0.0).count(),"mean_advantage_over_fixed":mean(&fixed),
            "mean_advantage_over_random":mean(&random),"sign_test_p_over_fixed":p_fixed,"sign_test_p_over_random":p_random,
            "all_runs_stable":all_stable,"all_replays_identical":all_replays,
            "passed":complete && noncollapse && all_stable && all_replays && mean(&fixed)>0.0 && mean(&random)>0.0 && p_fixed<0.05 && p_random<0.05,
            "limits":"Native M0 criterion per variant, uncorrected across parameter grid; not an independent confirmation or M9 action-effect result"}),
        )
    }
}

pub fn parameter_names() -> &'static [&'static str] {
    PARAMETERS
}

fn sign_p(values: &[f64]) -> f64 {
    let n = values.iter().filter(|&&v| v != 0.0).count();
    let wins = values.iter().filter(|&&v| v > 0.0).count();
    if n == 0 {
        return 1.0;
    }
    let mut log_p = -(n as f64) * std::f64::consts::LN_2;
    let mut total = 0.0;
    for k in 0..=n {
        if k >= wins {
            total += log_p.exp();
        }
        if k < n {
            log_p += ((n - k) as f64).ln() - ((k + 1) as f64).ln();
        }
    }
    total.min(1.0)
}

fn run_native(
    base: &M0ExperimentConfig,
    seed: u64,
    intervention: &Intervention,
) -> Result<M0GroupResult> {
    let mut c = base.clone();
    c.nerva.network.seed = seed;
    M0Experiment::new(c, group(&intervention.name)?)?.run()
}
pub(super) fn collect_native(result: M0GroupResult) -> Result<TrialMetrics> {
    let m = &result.metrics;
    let values = BTreeMap::from([
        ("sequence_score".to_owned(), m.sequence_score() as f64),
        ("stability".to_owned(), f64::from(m.stable_after_learning)),
        ("firing_rate".to_owned(), m.probe_mean_rate_hz as f64),
        (
            "training_rate_hz".to_owned(),
            m.training_mean_rate_hz as f64,
        ),
        ("transition_hits".to_owned(), m.transition_hits as f64),
        ("false_transitions".to_owned(), m.false_transitions as f64),
        (
            "silent_fraction".to_owned(),
            m.silent_neuron_fraction as f64,
        ),
        (
            "persistent_fraction".to_owned(),
            m.persistently_active_neuron_fraction as f64,
        ),
        (
            "frozen_replay_identical".to_owned(),
            f64::from(m.frozen_probe_replay_identical),
        ),
        (
            "weights_frozen".to_owned(),
            f64::from(m.frozen_weights_unchanged),
        ),
        (
            "latency_hits".to_owned(),
            m.transition_latency_errors_us.len() as f64,
        ),
        (
            "rates_finite".to_owned(),
            f64::from(m.probe_neuron_rates_hz.iter().all(|(_, r)| r.is_finite())),
        ),
    ]);
    Ok(TrialMetrics {
        values,
        details: json!({"native_group":result.group.label(),
            "effective_config":format!("{:?}",result.effective_config),"event_log_digest":result.event_log_digest,
            "initial_weights":result.initial_weights.iter().map(|w|w.get()).collect::<Vec<_>>(),
            "final_weights":result.final_weights.iter().map(|w|w.get()).collect::<Vec<_>>(),
            "training_patterns":result.training_patterns.iter().map(|p|format!("{p:?}")).collect::<Vec<_>>(),
            "predicted_patterns":result.predicted_patterns.iter().map(|(t,p)|json!([t.0,format!("{p:?}")])).collect::<Vec<_>>(),
            "transition_latency_errors_us":m.transition_latency_errors_us,
            "probe_neuron_rates_hz":m.probe_neuron_rates_hz.iter().map(|(id,r)|json!([id.0,r])).collect::<Vec<_>>()}),
    })
}
