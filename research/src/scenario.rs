use crate::Result;
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq)]
pub struct Intervention {
    pub name: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TrialMetrics {
    pub values: BTreeMap<String, f64>,
    pub details: Value,
}

/// A scenario owns configuration, intervention timing and scientific interpretation.
/// The runner only expands variants, iterates seeds, checks replay and persists records.
pub trait Scenario {
    type Config: Clone;
    fn name(&self) -> &'static str;
    fn default_config(&self) -> Self::Config;
    fn set_parameter(&self, config: &mut Self::Config, name: &str, value: &Value) -> Result<()>;
    fn validate(&self, config: &Self::Config) -> Result<()>;
    fn snapshot(&self, config: &Self::Config, seed: u64, intervention: &Intervention) -> Value;
    fn interventions(&self) -> Vec<Intervention>;
    fn metric_names(&self) -> Vec<&'static str>;
    fn run(
        &self,
        config: &Self::Config,
        seed: u64,
        intervention: &Intervention,
    ) -> Result<TrialMetrics>;
    /// Default replay compares every returned measurement and deterministic detail.
    /// Adapters with richer native records can compare those before collecting metrics.
    fn run_verified(
        &self,
        config: &Self::Config,
        seed: u64,
        intervention: &Intervention,
    ) -> Result<TrialMetrics> {
        let first = self.run(config, seed, intervention)?;
        if first != self.run(config, seed, intervention)? {
            return Err(crate::error("deterministic replay differs"));
        }
        Ok(first)
    }
    fn evaluate(&self, _records: &[Value], _full_replay_checked: bool) -> Result<Value> {
        Ok(serde_json::json!({"status":"not_evaluated","passed":null}))
    }
}
