use crate::{Result, error, scenario::Intervention};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

pub fn fields(value: &Value, allowed: &[&str]) -> Result<()> {
    let map = value
        .as_object()
        .ok_or_else(|| error("expected an object"))?;
    if let Some(key) = map.keys().find(|k| !allowed.contains(&k.as_str())) {
        return Err(error(format!("unknown field: {key}")));
    }
    Ok(())
}
pub fn string<'a>(value: &'a Value, field: &str) -> Result<&'a str> {
    value[field]
        .as_str()
        .ok_or_else(|| error(format!("{field} must be a string")))
}
pub fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 80
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

#[derive(Clone, Debug)]
pub struct ExperimentSpec {
    pub source: Value,
    pub name: String,
    pub scenario: String,
    pub seeds: Vec<u64>,
    pub base: BTreeMap<String, Value>,
    pub axes: Vec<(String, Vec<Value>)>,
    pub metrics: Vec<String>,
    pub interventions: Option<Vec<Intervention>>,
    pub verify_replay: bool,
    pub max_runs: usize,
}

fn flatten(prefix: &str, value: &Value, out: &mut BTreeMap<String, Value>) -> Result<()> {
    for (name, value) in value
        .as_object()
        .ok_or_else(|| error("base must be an object"))?
    {
        let key = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{prefix}.{name}")
        };
        if value.is_object() {
            flatten(&key, value, out)?;
        } else if out.insert(key.clone(), value.clone()).is_some() {
            return Err(error(format!("duplicate base parameter: {key}")));
        }
    }
    Ok(())
}
impl ExperimentSpec {
    pub fn parse(source: Value) -> Result<Self> {
        fields(
            &source,
            &["experiment", "base", "parameters", "measurements"],
        )?;
        let experiment = &source["experiment"];
        fields(
            experiment,
            &[
                "name",
                "scenario",
                "seeds",
                "interventions",
                "verify_replay",
                "max_runs",
            ],
        )?;
        let name = string(experiment, "name")?.to_owned();
        if !identifier(&name) {
            return Err(error(
                "experiment name must contain 1..80 ASCII letters, digits, '-' or '_'",
            ));
        }
        let scenario = string(experiment, "scenario")?.to_owned();
        let seeds: Vec<_> = experiment["seeds"]
            .as_array()
            .ok_or_else(|| error("seeds must be an array"))?
            .iter()
            .map(|v| {
                v.as_u64()
                    .ok_or_else(|| error("seeds must be nonnegative u64 integers"))
            })
            .collect::<Result<_>>()?;
        if seeds.is_empty() || seeds.iter().collect::<BTreeSet<_>>().len() != seeds.len() {
            return Err(error("seeds must be nonempty and unique"));
        }
        let mut base = BTreeMap::new();
        if let Some(value) = source.get("base") {
            flatten("", value, &mut base)?;
        }
        let mut axes = Vec::new();
        let mut names = BTreeSet::new();
        if let Some(parameters) = source.get("parameters") {
            for axis in parameters
                .as_array()
                .ok_or_else(|| error("parameters must be an array"))?
            {
                fields(axis, &["name", "values"])?;
                let name = string(axis, "name")?.to_owned();
                if !names.insert(name.clone()) {
                    return Err(error("duplicate parameter axis"));
                }
                let values = axis["values"]
                    .as_array()
                    .ok_or_else(|| error("axis values must be an array"))?
                    .clone();
                if values.is_empty()
                    || values
                        .iter()
                        .map(Value::to_string)
                        .collect::<BTreeSet<_>>()
                        .len()
                        != values.len()
                {
                    return Err(error("axis values must be nonempty and unique"));
                }
                axes.push((name, values));
            }
        }
        fields(&source["measurements"], &["metrics"])?;
        let metrics: Vec<_> = source["measurements"]["metrics"]
            .as_array()
            .ok_or_else(|| error("metrics must be an array"))?
            .iter()
            .map(|v| {
                v.as_str()
                    .map(str::to_owned)
                    .ok_or_else(|| error("metric names must be strings"))
            })
            .collect::<Result<_>>()?;
        if metrics.is_empty() || metrics.iter().collect::<BTreeSet<_>>().len() != metrics.len() {
            return Err(error("metrics must be nonempty and unique"));
        }
        let interventions = experiment
            .get("interventions")
            .map(|v| -> Result<Vec<_>> {
                let list = v
                    .as_array()
                    .ok_or_else(|| error("interventions must be an array"))?
                    .iter()
                    .map(|v| {
                        v.as_str()
                            .map(|s| Intervention { name: s.to_owned() })
                            .ok_or_else(|| error("interventions must be names"))
                    })
                    .collect::<Result<Vec<_>>>()?;
                if list.is_empty()
                    || list.iter().map(|v| &v.name).collect::<BTreeSet<_>>().len() != list.len()
                {
                    return Err(error("interventions must be nonempty and unique"));
                }
                Ok(list)
            })
            .transpose()?;
        let verify_replay = experiment
            .get("verify_replay")
            .map(|v| {
                v.as_bool()
                    .ok_or_else(|| error("verify_replay must be boolean"))
            })
            .transpose()?
            .unwrap_or(true);
        let max_runs = experiment
            .get("max_runs")
            .map(|v| {
                v.as_u64()
                    .filter(|&n| n > 0 && n <= 100_000)
                    .ok_or_else(|| error("max_runs must be 1..100000"))
            })
            .transpose()?
            .unwrap_or(10_000) as usize;
        Ok(Self {
            source,
            name,
            scenario,
            seeds,
            base,
            axes,
            metrics,
            interventions,
            verify_replay,
            max_runs,
        })
    }
}

pub struct VariantGenerator;
impl VariantGenerator {
    pub fn generate(
        axes: &[(String, Vec<Value>)],
        limit: usize,
    ) -> Result<Vec<BTreeMap<String, Value>>> {
        let mut variants = vec![BTreeMap::new()];
        for (name, values) in axes {
            let size = variants
                .len()
                .checked_mul(values.len())
                .filter(|&n| n <= limit)
                .ok_or_else(|| error("parameter grid exceeds max_runs"))?;
            let mut next = Vec::with_capacity(size);
            for variant in &variants {
                for value in values {
                    let mut copy = variant.clone();
                    copy.insert(name.clone(), value.clone());
                    next.push(copy);
                }
            }
            variants = next;
        }
        Ok(variants)
    }
}
