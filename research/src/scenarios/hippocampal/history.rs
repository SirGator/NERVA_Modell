//! Optional event history; native rules and checkpoint interventions stay unchanged.
use super::{
    Config as ModelConfig,
    association::{self, AssociationScenario},
    causal::verify_only_weights,
};
use crate::{
    Result, error,
    scenario::{Intervention, Scenario, TrialMetrics},
};
use nerva_vm::model::LearningRule;
use serde_json::{Value, json};
use std::collections::BTreeSet;
mod collect;
pub(super) use collect::History;

pub const PARAMETERS: [&str; 7] = [
    "processing_neurons",
    "training_repetitions",
    "rest_us",
    "history.neurons",
    "history.synapses",
    "history.max_rows",
    "history.max_source_rows",
];
#[derive(Clone, Debug)]
pub struct Config {
    model: ModelConfig,
    neurons: BTreeSet<u64>,
    synapses: BTreeSet<u64>,
    max_rows: usize,
    max_source_rows: usize,
}
pub struct HistoryScenario;
fn ids(value: &Value) -> Result<BTreeSet<u64>> {
    let values = value
        .as_array()
        .ok_or_else(|| error("history IDs must be an array"))?;
    let ids = values
        .iter()
        .map(|v| {
            v.as_u64()
                .ok_or_else(|| error("history IDs must be u64 integers"))
        })
        .collect::<Result<BTreeSet<_>>>()?;
    if ids.len() != values.len() {
        return Err(error("duplicate history ID"));
    }
    Ok(ids)
}
impl Scenario for HistoryScenario {
    type Config = Config;
    fn name(&self) -> &'static str {
        "hippocampal_learning_history"
    }
    fn default_config(&self) -> Config {
        Config {
            model: AssociationScenario.default_config(),
            neurons: BTreeSet::new(),
            synapses: BTreeSet::new(),
            max_rows: 100_000,
            max_source_rows: 20_000_000,
        }
    }
    fn set_parameter(&self, c: &mut Config, name: &str, v: &Value) -> Result<()> {
        match name {
            "history.neurons" => c.neurons = ids(v)?,
            "history.synapses" => c.synapses = ids(v)?,
            "history.max_source_rows" => {
                c.max_source_rows = v
                    .as_u64()
                    .filter(|n| (1..=100_000_000).contains(n))
                    .ok_or_else(|| error("history.max_source_rows must be 1..100000000"))?
                    as usize;
            }
            "history.max_rows" => {
                c.max_rows = v
                    .as_u64()
                    .filter(|n| (1..=1_000_000).contains(n))
                    .ok_or_else(|| error("history.max_rows must be 1..1000000"))?
                    as usize
            }
            _ => AssociationScenario.set_parameter(&mut c.model, name, v)?,
        }
        Ok(())
    }
    fn validate(&self, c: &Config) -> Result<()> {
        AssociationScenario.validate(&c.model)?;
        // Topology and IDs are fixed across the paired controls and seeds.
        let model = association::build(&c.model, 0, "three_on", None)?;
        History::new(c).initialize(&model)
    }
    fn snapshot(&self, c: &Config, seed: u64, i: &Intervention) -> Value {
        let mut base = AssociationScenario.snapshot(&c.model, seed, i);
        base["learning_rule"] = json!(if i.name.starts_with("pair") {
            "PairStdp"
        } else {
            "ThreeFactorAccumulated"
        });
        base["training_passes_per_execution"] = json!(3);
        json!({"protocol":"NERVA-Learning-History/1", "base":base,
            "rule":"ThreeFactorAccumulated except pair controls", "neurons":c.neurons,"synapses":c.synapses,"max_rows":c.max_rows,
            "frame_audit_limit":collect::FRAME_LIMIT,"total_audit_limit":c.max_source_rows,
            "training_passes_per_execution":3,"development":false,"homeostasis":false,"competition":false})
    }
    fn interventions(&self) -> Vec<Intervention> {
        AssociationScenario.interventions()
    }
    fn metric_names(&self) -> Vec<&'static str> {
        vec![
            "weight_events",
            "requests_without_change",
            "trace_rows",
            "recall_used_changed_synapses",
            "recall_core_effect",
            "recall_motor_effect",
            "weight_gain_a",
            "weight_gain_b",
            "matched_checkpoints",
        ]
    }
    fn run(&self, c: &Config, seed: u64, i: &Intervention) -> Result<TrialMetrics> {
        self.validate(c)?;
        if !association::GROUPS.contains(&i.name.as_str()) {
            return Err(error("unknown learning history control"));
        }
        let rule = if i.name.starts_with("pair") {
            LearningRule::PairStdp
        } else {
            LearningRule::ThreeFactorAccumulated
        };
        let mut history = History::new(c);
        let mut trained = association::train_observed(
            &c.model,
            seed,
            &i.name,
            true,
            Some(rule),
            Some(&mut history),
        )?;
        let mut reset =
            association::train_with_learning(&c.model, seed, &i.name, false, Some(rule))?;
        if !trained.model.same_replay_state(&reset.model) {
            return Err(error("history changed full training replay state"));
        }
        history.checkpoint(&trained.model);
        verify_only_weights(&trained.model, &mut reset.model, &trained.weights)?;
        // Independently check observing every recall/rest event preserves the full state.
        let mut plain =
            association::train_with_learning(&c.model, seed, &i.name, false, Some(rule))?;
        let observed =
            association::recall_observed(&mut trained.model, &c.model, Some(&mut history))?;
        let unaudited = association::recall(&mut plain.model, &c.model)?;
        if observed != unaudited || !trained.model.same_replay_state(&plain.model) {
            return Err(error("history changed full recall replay state"));
        }
        history.observe_weight_reset();
        let without = association::recall_observed(&mut reset.model, &c.model, Some(&mut history))?;
        let differs = |key: &str| {
            observed["observations"]
                .as_array()
                .unwrap()
                .iter()
                .zip(without["observations"].as_array().unwrap())
                .any(|(a, b)| a[key] != b[key])
        };
        let core_effect = differs("core_event_digest");
        let motor_effect = differs("motor_event_digest");
        let margin = |v: &Value, k: usize| v["completion_margins"][k].as_f64().unwrap();
        let history = history.finish();
        let values = [
            history["summary"]["weight_events"].as_f64().unwrap(),
            history["summary"]["requests_without_change"]
                .as_f64()
                .unwrap(),
            history["events"].as_array().unwrap().len() as f64,
            history["summary"]["recall_used_changed_synapses"]
                .as_f64()
                .unwrap(),
            f64::from(core_effect),
            f64::from(motor_effect),
            margin(&observed, 0) - margin(&without, 0),
            margin(&observed, 1) - margin(&without, 1),
            1.,
        ];
        Ok(TrialMetrics {
            values: self
                .metric_names()
                .into_iter()
                .map(str::to_owned)
                .zip(values)
                .collect(),
            details: json!({"history":history,"training":trained.details,"learned_recall":observed,"reset_recall":without,
                "matched_checkpoints":true,"observation_preserves_full_state":true,
                "limits":"Weight reset isolates the joint weight effect. Temporal links and used changed synapses do not prove a single update caused a spike, or that a mechanism is helpful/harmful. Development, homeostasis and competition are disabled by this assay."}),
        })
    }
    fn evaluate(&self, records: &[Value], replay: bool) -> Result<Value> {
        Ok(
            json!({"status":"learning_history_observation","passed":null,"records":records.len(),"replay_checked":replay,"functional_memory_demonstrated":false}),
        )
    }
}
