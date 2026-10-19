use crate::{
    Result,
    model::{KeyboardLayout, ModelConfig, SensorConfig, SensorMode, SpikeCoding},
};
use serde_json::Value;

pub const ARMS: [&str; 4] = ["before", "aged_frozen", "paired", "scrambled"];
#[derive(Clone, Debug)]
pub struct M4Config {
    pub case_id: String,
    pub seed: u64,
    pub model: ModelConfig,
    pub sensor: SensorConfig,
    pub delay_us: u64,
    pub train_trials: usize,
    pub test_trials: usize,
    pub calibration_trials: usize,
    pub requested: Value,
}
pub(crate) fn fields(value: &Value, allowed: &[&str]) -> Result<()> {
    let object = value
        .as_object()
        .ok_or("configuration section must be an object")?;
    for key in object.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(format!("unknown configuration field {key:?}").into());
        }
    }
    for &key in allowed {
        if !object.contains_key(key) {
            return Err(format!("missing configuration field {key:?}").into());
        }
    }
    Ok(())
}
fn number(value: &Value, key: &str) -> Result<u64> {
    value[key]
        .as_u64()
        .ok_or_else(|| format!("{key} must be a nonnegative integer").into())
}
fn boolean(value: &Value, key: &str) -> Result<bool> {
    value[key]
        .as_bool()
        .ok_or_else(|| format!("{key} must be boolean").into())
}
pub(crate) fn string<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    value[key]
        .as_str()
        .ok_or_else(|| format!("{key} must be a string").into())
}
impl M4Config {
    pub fn parse(value: Value) -> Result<Self> {
        fields(
            &value,
            &[
                "protocol",
                "case_id",
                "seed",
                "params",
                "arms",
                "train_trials",
                "test_trials",
                "calibration_trials",
            ],
        )?;
        if string(&value, "protocol")? != "NERVA-M4-v1" {
            return Err("protocol must be NERVA-M4-v1".into());
        }
        if value["arms"] != serde_json::json!(ARMS) {
            return Err(
                "arms must be [before, aged_frozen, paired, scrambled] in that order".into(),
            );
        }
        let case_id = string(&value, "case_id")?.to_owned();
        if case_id.is_empty()
            || !case_id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            return Err("case_id must contain only letters, digits or underscores".into());
        }
        let p = &value["params"];
        fields(
            p,
            &[
                "sensor",
                "resolution",
                "encoding",
                "processing",
                "action_effect_delay_ms",
                "motor_feedback",
                "stdp",
                "homeostasis",
                "development",
                "initial_drive",
                "pruning_min_us",
                "normalization",
            ],
        )?;
        if string(p, "normalization")? != "equal_input_current" {
            return Err("normalization must be equal_input_current".into());
        }
        let mode: SensorMode = string(p, "sensor")?.parse()?;
        let coding: SpikeCoding = string(p, "encoding")?.parse()?;
        let resolution = usize::try_from(number(p, "resolution")?)?;
        // Suite sentinel 0 explicitly means a diagnostic feature sensor on the
        // canonical 64-grid; it is never accepted for a pixel sensor.
        let effective = if resolution == 0 && mode != SensorMode::Pixels {
            64
        } else {
            resolution
        };
        let sensor = SensorConfig {
            mode,
            coding,
            resolution: effective,
            ..SensorConfig::default()
        };
        sensor.validate()?;
        let seed = number(&value, "seed")?;
        let initial = p["initial_drive"]
            .as_f64()
            .ok_or("initial_drive must be numeric")? as f32;
        let model = ModelConfig {
            seed,
            keyboard: KeyboardLayout::Ad,
            processing_neurons: usize::try_from(number(p, "processing")?)?,
            learning: boolean(p, "stdp")?,
            development: boolean(p, "development")?,
            homeostasis: boolean(p, "homeostasis")?,
            feedback: boolean(p, "motor_feedback")?,
            initial_structural_drive: initial,
            pruning_min_below_us: number(p, "pruning_min_us")?,
            ..ModelConfig::default()
        };
        model.validate()?;
        let delay_us = number(p, "action_effect_delay_ms")?
            .checked_mul(1000)
            .ok_or("delay overflow")?;
        if ![2_000, 8_000, 20_000, 50_000].contains(&delay_us) {
            return Err("action_effect_delay_ms must be 2, 8, 20 or 50".into());
        }
        let train_trials = usize::try_from(number(&value, "train_trials")?)?;
        let test_trials = usize::try_from(number(&value, "test_trials")?)?;
        let calibration_trials = usize::try_from(number(&value, "calibration_trials")?)?;
        for (name, n) in [
            ("train_trials", train_trials),
            ("test_trials", test_trials),
            ("calibration_trials", calibration_trials),
        ] {
            if !(8..=4096).contains(&n) || !n.is_multiple_of(4) {
                return Err(format!("{name} must be a multiple of four in 8..=4096").into());
            }
        }
        Ok(Self {
            case_id,
            seed,
            model,
            sensor,
            delay_us,
            train_trials,
            test_trials,
            calibration_trials,
            requested: value,
        })
    }
}
