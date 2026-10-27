//! Fixed 2×2 test of recurrent strength and the existing STDP time constants.
//!
//! Reuses the M8 execution and replay measurements without changing its
//! one-factor configuration contract or any neural learning rule.
use crate::{Result, m8};
use serde_json::{Value, json};
use std::{path::Path, sync::atomic::AtomicBool};

pub struct Config {
    pub experiment: m8::Config,
    pub requested: Value,
}

fn fields(value: &Value, expected: &[&str]) -> Result<()> {
    let object = value
        .as_object()
        .ok_or("expected M9 configuration object")?;
    if object.len() != expected.len() || expected.iter().any(|key| !object.contains_key(*key)) {
        return Err("unknown or missing M9 configuration field".into());
    }
    Ok(())
}

pub fn parse(value: Value) -> Result<Config> {
    fields(
        &value,
        &[
            "protocol",
            "case_id",
            "seed",
            "train_trials",
            "test_trials",
            "calibration_trials",
            "condition",
        ],
    )?;
    if value["protocol"] != "NERVA-M9-v1" && value["protocol"] != "NERVA-M9-v2" {
        return Err("protocol must be NERVA-M9-v1 or NERVA-M9-v2".into());
    }
    fields(&value["condition"], &["core_weight_scale", "stdp_tau_us"])?;
    let weight = value["condition"]["core_weight_scale"]
        .as_f64()
        .filter(|x| [1.0, 4.0].contains(x))
        .ok_or("M9 core_weight_scale must be 1 or 4")?;
    let tau = value["condition"]["stdp_tau_us"]
        .as_f64()
        .filter(|x| [20_000.0, 100_000.0].contains(x))
        .ok_or("M9 stdp_tau_us must be 20000 or 100000")?;

    let mut reference = value.clone();
    reference["protocol"] = json!("NERVA-M8-v1");
    reference["condition"] = json!({"parameter":"reference", "value":1});
    let mut experiment = m8::parse(reference)?;
    experiment.engine.base.model.core_weight_scale = weight as f32;
    experiment.engine.base.model.stdp_tau_us = tau as f32;
    experiment.engine.base.model.validate()?;
    Ok(Config {
        experiment,
        requested: value,
    })
}

pub fn run(config: &Config, output: &Path, raw: bool, flag: &AtomicBool) -> Result<Value> {
    run_with_trace(config, output, raw, flag, false)
}
pub fn run_with_trace(
    config: &Config,
    output: &Path,
    raw: bool,
    flag: &AtomicBool,
    trace: bool,
) -> Result<Value> {
    run_with_diagnostics(config, output, raw, flag, trace, false)
}
pub fn run_with_diagnostics(
    config: &Config,
    output: &Path,
    raw: bool,
    flag: &AtomicBool,
    trace: bool,
    recurrence: bool,
) -> Result<Value> {
    run_extended(config, output, raw, flag, trace, recurrence, false)
}

pub fn run_with_crossed_weights(
    config: &Config,
    output: &Path,
    raw: bool,
    flag: &AtomicBool,
) -> Result<Value> {
    run_extended(config, output, raw, flag, false, false, true)
}

fn run_extended(
    config: &Config,
    output: &Path,
    raw: bool,
    flag: &AtomicBool,
    trace: bool,
    recurrence: bool,
    crossed: bool,
) -> Result<Value> {
    let causal = config.requested["protocol"] == "NERVA-M9-v2";
    let mut result = if crossed {
        m8::run_with_crossed_weights(&config.experiment, output, raw, flag)?
    } else {
        m8::run_with_diagnostics(
            &config.experiment,
            output,
            raw,
            flag,
            causal,
            trace,
            recurrence,
        )?
    };
    result["protocol"] = config.requested["protocol"].clone();
    result["requested_config"] = config.requested.clone();
    result["condition"] = config.requested["condition"].clone();
    result["protocol_details"]["limits"] = json!([
        "M9 fixed 2x2: Core weights 1/4 and both existing STDP time constants 20/100ms",
        "Core weights scale excitation and inhibition together; no new neural rule or task-specific wiring",
        "Reference rate gain and all initial edges, cells, delays and peripheral weights held fixed within seed",
        "Two action probes per mapping at exact training checkpoints, immediate and after independent 2s rest; no-copy controls",
        "Core homeostasis and development off; weights frozen during evaluation; extra spikes alone are not learning",
        "Fresh pilot and confirmation seeds; no selection or tuning between stages; factorial contrasts descriptive"
    ]);
    if causal {
        result["protocol_details"]["weight_resets"] = json!({
            "interventions":["sham","restore_core","restore_all_plastic"],
            "restoration":"condition-specific initial weights, after independent rest and before baseline",
            "readout":"same training-checkpoint count decoder; no recalibration after intervention",
            "limitations":["Pending inputs retain emission-time amplitudes", "Membrane and trace state preserved; tests necessity under this state and fixed decoder, not sufficiency or exclusivity of weight storage"],
            "minimum_confirmation_seeds":20
        });
    }
    Ok(result)
}
