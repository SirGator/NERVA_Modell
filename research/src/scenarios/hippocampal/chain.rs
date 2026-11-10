//! Four bounded causal diagnostics. Artificial capability is never task learning.
use super::{Config, HippocampalScenario, fail};
use crate::{
    Result, error,
    scenario::{Intervention, Scenario, TrialMetrics},
};
use nerva::{
    config::NeuronConfig,
    core::{Neuron, NeuronId, Polarity, SimTime},
    math::Position3D,
};
use serde_json::{Value, json};
use std::collections::BTreeMap;
mod gain;
mod local;
mod memory;
mod numeric;
pub const GROUPS: [&str; 4] = ["A_numeric", "B_memory", "C_gain", "D_local"];
pub const GAINS: [f64; 5] = [0., 1., 10., 100., 1000.];
pub const METRICS: [&str; 10] = [
    "input_discriminable",
    "representation_discriminable",
    "local_tag_available",
    "persistent_weight_change",
    "native_recall_effect",
    "artificial_recall_effect",
    "diagnostic_controls_passed",
    "audit_complete",
    "native_training_passes",
    "audit_rows",
];
pub struct ChainScenario;
struct Outcome {
    // -1 is untested; it is never silently treated as a failure or a pass.
    stages: [f64; 6],
    passed: bool,
    rows: usize,
    training: usize,
    details: Value,
}
fn flag(value: bool) -> f64 {
    if value { 1. } else { 0. }
}
fn cell(id: u64, x: f32) -> Result<Neuron> {
    Neuron::new(
        NeuronId(id),
        Position3D::new(x, 0., 0.),
        Polarity::Excitatory,
        None,
        NeuronConfig {
            resting_potential: 0.,
            reset_potential: 0.,
            threshold: 1.,
            refractory_period_us: 1000,
            ..Default::default()
        },
        SimTime::ZERO,
    )
    .map_err(fail)
}
impl Scenario for ChainScenario {
    type Config = Config;
    fn name(&self) -> &'static str {
        "hippocampal_causal_chain"
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
        HippocampalScenario.validate(c)?;
        if c.processing_neurons > 256 || c.training_repetitions > 16 {
            return Err(error(
                "chain budget: at most 256 model cells and 16 repetitions",
            ));
        }
        Ok(())
    }
    fn snapshot(&self, c: &Config, seed: u64, i: &Intervention) -> Value {
        json!({"protocol":"NERVA-Causal-Chain/1","experiment":i.name,"seed":seed,
            "processing_neurons":c.processing_neurons,"training_repetitions":c.training_repetitions,
            "rest_us":c.rest_us,"gains":GAINS,"numeric_partitions":[1,1000],
            "training_passes_per_execution":match i.name.as_str() {"C_gain"=>6,"D_local"=>7,_=>0},
            "microcircuit":"unit-threshold LIF; A->B delay1000us; artificial stimuli/weights explicitly labelled",
            "scope":if i.name=="C_gain" {"hippocampal_model_and_calibrated_microcircuit"} else {"isolated_native_mechanism"},
            "limits":"A/B/D are deterministic calibrations, not independent seed samples. -1 metrics mean untested."})
    }
    fn interventions(&self) -> Vec<Intervention> {
        GROUPS
            .iter()
            .map(|&name| Intervention { name: name.into() })
            .collect()
    }
    fn metric_names(&self) -> Vec<&'static str> {
        METRICS.to_vec()
    }
    fn run(&self, c: &Config, seed: u64, i: &Intervention) -> Result<TrialMetrics> {
        self.validate(c)?;
        let result = match i.name.as_str() {
            "A_numeric" => numeric::run()?,
            "B_memory" => memory::run(c.rest_us)?,
            "C_gain" => gain::run(c, seed)?,
            "D_local" => local::run()?,
            _ => return Err(error("unknown causal-chain experiment")),
        };
        let mut values = result.stages.to_vec();
        values.extend([
            flag(result.passed),
            1.,
            result.training as f64,
            result.rows as f64,
        ]);
        Ok(TrialMetrics {
            values: METRICS.iter().map(|s| (*s).into()).zip(values).collect(),
            details: json!({"experiment":i.name,"data":result.details,"functional_memory_demonstrated":false,
                "artificial_intervention":true,"limits":"Capability of a diagnostic circuit is not a demonstrated hippocampal memory. Fixed grids give sampled brackets, not a universal threshold."}),
        })
    }
    fn evaluate(&self, records: &[Value], replay: bool) -> Result<Value> {
        let mut groups = BTreeMap::<u64, Vec<&str>>::new();
        for row in records {
            groups
                .entry(
                    row["job"]["seed"]
                        .as_u64()
                        .ok_or_else(|| error("missing chain seed"))?,
                )
                .or_default()
                .push(
                    row["job"]["intervention"]
                        .as_str()
                        .ok_or_else(|| error("missing chain group"))?,
                );
        }
        Ok(
            json!({"status":"causal_diagnosis_not_learning_confirmation","passed":null,
            "complete_per_seed":groups.values().all(|g|GROUPS.iter().all(|name|g.contains(name))),
            "replay_checked":replay,"functional_memory_demonstrated":false}),
        )
    }
}
