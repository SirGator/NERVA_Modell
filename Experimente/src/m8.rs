//! Six one-factor scans using existing model dynamics and the paired replay protocol.
use crate::{
    Result,
    model::{Model, ModelConfig},
};
use nerva_vm::{
    m4::normalization,
    m6::{self, RunOptions},
};
use serde_json::{Value, json};
use std::{path::Path, sync::atomic::AtomicBool};
pub struct Config {
    pub engine: m6::Config,
    pub requested: Value,
    pub parameter: String,
    pub value: f64,
}
fn fields(value: &Value, names: &[&str]) -> Result<()> {
    let object = value.as_object().ok_or("expected configuration object")?;
    if object.len() != names.len() || names.iter().any(|k| !object.contains_key(*k)) {
        return Err("unknown or missing M8 configuration field".into());
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
    fields(&value["condition"], &["parameter", "value"])?;
    if value["protocol"] != "NERVA-M8-v1" {
        return Err("protocol must be NERVA-M8-v1".into());
    }
    let parameter = value["condition"]["parameter"]
        .as_str()
        .ok_or("parameter must be string")?
        .to_owned();
    let level = value["condition"]["value"]
        .as_f64()
        .ok_or("value must be numeric")?;
    let allowed = match parameter.as_str() {
        "reference" => vec![1.0],
        "core_weight_scale" => vec![2.0, 4.0],
        "core_threshold" => vec![0.8, 0.6],
        "stdp_tau_us" => vec![50_000.0, 100_000.0],
        "core_delay_us" => vec![500.0, 5000.0],
        "core_fanout" => vec![8.0, 12.0],
        "sensory_weight_scale" => vec![0.5, 1.5],
        _ => return Err("unknown M8 parameter".into()),
    };
    if !allowed.contains(&level) {
        return Err("unsupported M8 level".into());
    }
    let base = json!({"protocol":"NERVA-M4-v1","case_id":value["case_id"],"seed":value["seed"],
        "train_trials":value["train_trials"],"test_trials":value["test_trials"],"calibration_trials":value["calibration_trials"],
        "arms":["before","aged_frozen","paired","scrambled"],"params":{"sensor":"effect_2ch","resolution":0,"encoding":"deterministic",
        "processing":32,"action_effect_delay_ms":50,"motor_feedback":true,"stdp":true,"homeostasis":false,"development":false,
        "initial_drive":0,"pruning_min_us":5000000,"normalization":"equal_input_current"}});
    let mut engine =
        m6::Config::parse(json!({"protocol":"NERVA-M6-v1","variant":"D","base":base}))?;
    let model = &mut engine.base.model;
    match parameter.as_str() {
        "core_weight_scale" => model.core_weight_scale = level as f32,
        "core_threshold" => model.core_threshold = level as f32,
        "stdp_tau_us" => model.stdp_tau_us = level as f32,
        "core_delay_us" => model.core_delay_us = Some(level as u64),
        "core_fanout" => model.core_fanout = level as usize,
        "sensory_weight_scale" => model.sensory_weight_scale = level as f32,
        _ => {}
    }
    model.validate()?;
    Ok(Config {
        engine,
        requested: value,
        parameter,
        value: level,
    })
}
pub fn parameters(model: &ModelConfig) -> Value {
    json!({"core_weight_scale":model.core_weight_scale,"core_threshold":model.core_threshold,"core_fanout":model.core_fanout,
        "core_delay_us":model.core_delay_us,"stdp_tau_us":model.stdp_tau_us,"sensory_weight_scale":model.sensory_weight_scale,
        "core_membrane_tau_us":model.core_membrane_tau_us,"action_copy_weight":model.action_copy_weight,"homeostasis":model.homeostasis,"development":model.development})
}
pub fn run(config: &Config, output: &Path, raw: bool, flag: &AtomicBool) -> Result<Value> {
    let mut base = config.engine.base.clone();
    let defaults = ModelConfig::default();
    base.model.core_threshold = defaults.core_threshold;
    base.model.core_weight_scale = defaults.core_weight_scale;
    base.model.core_fanout = defaults.core_fanout;
    base.model.core_delay_us = defaults.core_delay_us;
    base.model.stdp_tau_us = defaults.stdp_tau_us;
    base.model.sensory_weight_scale = defaults.sensory_weight_scale;
    let baseline = Model::with_sensor(base.model.clone(), base.sensor)?;
    let (gain, budget) = normalization::gain(&baseline, &base)?;
    let initial = Model::with_sensor(config.engine.base.model.clone(), base.sensor)?;
    let mut result = m6::run_with_options(
        &config.engine,
        output,
        raw,
        flag,
        &RunOptions {
            reference_gain: Some(gain),
            checkpoint_rest_us: Some(2_000_000),
            propagation: true,
        },
    )?;
    result["protocol"] = json!("NERVA-M8-v1");
    result["requested_config"] = config.requested.clone();
    result["condition"] = json!({"parameter":config.parameter,"value":config.value});
    result["effective_parameters"] = parameters(&config.engine.base.model);
    result["reference_normalization"] = budget;
    result["initial_neurons"] = json!(
        initial
            .network()
            .neurons()
            .map(
                |n| json!({"id":n.id().0,"polarity":format!("{:?}",n.polarity()),
        "position":[n.position().x,n.position().y,n.position().z],"threshold":n.threshold()})
            )
            .collect::<Vec<_>>()
    );
    result["initial_synapses"] = json!(
        initial
            .network()
            .synapses()
            .map(|s| json!({"id":s.id().0,"pre":s.pre().0,"post":s.post().0,
        "weight":s.weight().get(),"delay_us":s.delay_us(),"plastic":s.is_plastic()}))
            .collect::<Vec<_>>()
    );
    result["protocol_details"]["rest_checkpoint_us"] = json!(2_000_000);
    result["protocol_details"]["limits"] = json!([
        "M8 one-factor scan; fixed baseline rate gain so visual weight scaling is not normalized away",
        "Two action probes per mapping at each identical training checkpoint, immediate and after independent 2s rest",
        "Online probes advance parent state; original sequential retention/interference also reported separately",
        "No action-specific wiring; extra local edges appended using independent RNG after unchanged scaffold",
        "Core homeostasis and development off; additional spikes alone are not learning",
        "Fresh confirmation seeds and fixed controls/mapping criteria; no parameter tuning on confirmation results"
    ]);
    Ok(result)
}
