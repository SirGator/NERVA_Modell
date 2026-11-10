//! One-mechanism comparison: historical publication vs retained small contributions.
use super::{
    Config, HippocampalScenario,
    association::{AssociationScenario, run_with_learning},
};
use crate::{
    Result, error,
    scenario::{Intervention, Scenario, TrialMetrics},
};
use nerva_vm::model::LearningRule;
use serde_json::{Value, json};
use std::collections::BTreeMap;
mod numeric;
pub const GROUPS: [&str; 6] = [
    "direct",
    "accumulated",
    "direct_frozen",
    "accumulated_frozen",
    "direct_unpaired",
    "accumulated_unpaired",
];
pub const NUMERIC: &str = "precision_numeric";
pub const EXTRA_METRICS: [&str; 9] = [
    "requested_mass",
    "published_mass",
    "pending_mass",
    "clipped_mass",
    "rounding_error_mass",
    "mass_balance_max_error",
    "signed_requested_mass",
    "signed_published_mass",
    "signed_pending_mass",
];
pub struct PrecisionScenario;
impl Scenario for PrecisionScenario {
    type Config = Config;
    fn name(&self) -> &'static str {
        "hippocampal_precision"
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
        json!({"protocol":"NERVA-Precision-Comparison/1","control":i.name,"seed":seed,
            "processing_neurons":c.processing_neurons,"training_repetitions":c.training_repetitions,"rest_us":c.rest_us,
            "training_passes_per_execution":if i.name==NUMERIC {0}else{2},
            "intervention":"Only weight publication policy; identical initial topology/cells/weights and input schedule",
            "numeric_partitions":[1,1000],"source_of_modulation":"Native model spikes; no artificial pulses in model arms"})
    }
    fn interventions(&self) -> Vec<Intervention> {
        std::iter::once(NUMERIC)
            .chain(GROUPS)
            .map(|name| Intervention { name: name.into() })
            .collect()
    }
    fn metric_names(&self) -> Vec<&'static str> {
        let mut names = AssociationScenario.metric_names();
        names.extend(EXTRA_METRICS);
        names
    }
    fn run(&self, c: &Config, seed: u64, i: &Intervention) -> Result<TrialMetrics> {
        self.validate(c)?;
        if i.name == NUMERIC {
            return Ok(TrialMetrics {
                values: self
                    .metric_names()
                    .into_iter()
                    .map(|name| (name.into(), 0.))
                    .collect(),
                details: numeric::run()?,
            });
        }
        if !GROUPS.contains(&i.name.as_str()) {
            return Err(error("unknown precision control"));
        }
        let rule = if i.name.starts_with("accumulated") {
            LearningRule::ThreeFactorAccumulated
        } else {
            LearningRule::ThreeFactor
        };
        let group = if i.name.ends_with("frozen") {
            "frozen_on"
        } else if i.name.ends_with("unpaired") {
            "three_unpaired"
        } else {
            "three_on"
        };
        let mut row = run_with_learning(c, seed, group, Some(rule))?;
        row.details["weight_update_mode"] = json!(if i.name.starts_with("accumulated") {
            "accumulated"
        } else {
            "direct_f32"
        });
        let training = &row.details["training"];
        let ledger = training["audit"]["mass_ledger"]
            .as_array()
            .ok_or_else(|| error("precision ledger missing"))?;
        let before = training["weights_before"].as_array().unwrap();
        let after = training["weights_after"].as_array().unwrap();
        let changes: BTreeMap<_, _> = before
            .iter()
            .zip(after)
            .map(|(a, b)| {
                (
                    a[0].as_u64().unwrap(),
                    b[3].as_f64().unwrap() - a[3].as_f64().unwrap(),
                )
            })
            .collect();
        let mut max_error = 0f64;
        for edge in ledger {
            let id = edge["synapse_id"].as_u64().unwrap();
            let actual = edge["actual"].as_f64().unwrap();
            max_error = max_error
                .max((changes[&id] - actual).abs())
                .max(edge["balance_error"].as_f64().unwrap().abs());
        }
        let residuals = training["weight_residuals_after"].as_array().unwrap();
        let sum = |name: &str| {
            ledger
                .iter()
                .map(|r| r[name].as_f64().unwrap())
                .sum::<f64>()
        };
        let numbers = [
            training["audit"]["requested_absolute_mass"]
                .as_f64()
                .unwrap(),
            training["audit"]["actual_absolute_mass"].as_f64().unwrap(),
            residuals.iter().map(|r| r[1].as_f64().unwrap().abs()).sum(),
            sum("clipped_l1"),
            sum("rounding_l1"),
            max_error,
            sum("requested"),
            sum("actual"),
            residuals.iter().map(|r| r[1].as_f64().unwrap()).sum(),
        ];
        if max_error > 1e-12 {
            return Err(error("precision weight/audit conservation differs"));
        }
        row.values
            .extend(EXTRA_METRICS.into_iter().map(str::to_owned).zip(numbers));
        row.details["functional_memory_demonstrated"] = json!(false);
        Ok(row)
    }
    fn evaluate(&self, records: &[Value], replay: bool) -> Result<Value> {
        Ok(
            json!({"status":"precision_publication_comparison","passed":null,"records":records.len(),
            "replay_checked":replay,"functional_memory_demonstrated":false}),
        )
    }
}
