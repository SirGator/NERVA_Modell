//! Passive replay diagnosis of an existing M9 result; no new neural rule.
use crate::{Result, m9};
use serde_json::{Value, json};
use std::{path::Path, sync::atomic::AtomicBool};

/// JSON float parsing can differ in the final f64 bit; integers remain exact.
pub fn replay_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) if x.is_f64() || y.is_f64() => {
            let (x, y) = (x.as_f64().unwrap(), y.as_f64().unwrap());
            (x - y).abs() <= 8.0 * f64::EPSILON * x.abs().max(y.abs()).max(1.0)
        }
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(x, y)| replay_equal(x, y))
        }
        (Value::Object(x), Value::Object(y)) => {
            x.len() == y.len()
                && x.iter()
                    .all(|(k, v)| y.get(k).is_some_and(|w| replay_equal(v, w)))
        }
        _ => a == b,
    }
}

pub fn without_trace(result: &Value) -> Value {
    let mut copy = result.clone();
    for arm in ["before", "aged_frozen", "paired", "scrambled"] {
        for mapping in ["normal_mapping", "reversed_mapping"] {
            for stage in ["checkpoint_probe", "rest_checkpoint_probe"] {
                for intervention in ["sham", "restore_core", "restore_all_plastic"] {
                    if let Some(trials) =
                        copy["arms"][arm][mapping]["weight_reset_probes"]["stages"][stage]
                            [intervention]
                            .as_array_mut()
                    {
                        for trial in trials {
                            trial.as_object_mut().unwrap().remove("trace");
                        }
                    }
                }
            }
        }
    }
    copy
}

pub fn run(reference: &Value, output: &Path, flag: &AtomicBool) -> Result<Value> {
    if reference["protocol"] != "NERVA-M9-v2" {
        return Err("M10 requires an existing M9-v2 result".into());
    }
    let config = m9::parse(reference["requested_config"].clone())?;
    let result = m9::run_with_trace(&config, output, false, flag, true)?;
    if !replay_equal(&without_trace(&result), reference) {
        return Err("M10 replay differs from original M9 result".into());
    }
    Ok(
        json!({"protocol":"NERVA-M10-v1","seed":reference["seed"],"case_id":reference["case_id"],
        "reference_reproduced":true,"m9_result":result,
        "limits":["Retrospective diagnosis on existing M9 seeds, not fresh confirmation", "Voltages projected on cloned neurons to common phase times; original M9 potentials remain stored last-update states", "Signed synaptic impulses, not biophysical conductance currents", "Frozen rules and fixed count decoder"]}),
    )
}
