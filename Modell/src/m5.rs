//! M5 isolates signal strength/integration and the population receiving the
//! existing homeostatic rule. No labels or scoring enter the neural model.
use crate::{
    Result, m4,
    model::{
        HomeostasisScope, Model, SensorMode, SpikeCoding,
        audit::{AuditOptions, learned_parameters_equal},
    },
};
use nerva::{
    core::{Network, SimTime},
    runtime::ObservationEvent,
};
use nerva_qemu::keyboard::Key;
use serde_json::{Value, json};
use std::{
    fs::OpenOptions,
    io::{BufWriter, Write},
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

pub struct Config {
    pub base: m4::config::M4Config,
    pub experiment: String,
    pub scope: String,
    pub requested: Value,
}
impl Config {
    pub fn parse(value: Value) -> Result<Self> {
        use m4::config::{fields, string};
        fields(
            &value,
            &[
                "protocol",
                "experiment",
                "base",
                "homeostasis_scope",
                "action_copy_weight",
                "core_membrane_tau_us",
            ],
        )?;
        if string(&value, "protocol")? != "NERVA-M5-v1" {
            return Err("protocol must be NERVA-M5-v1".into());
        }
        let experiment = string(&value, "experiment")?.to_owned();
        let scope = string(&value, "homeostasis_scope")?.to_owned();
        let mut base = m4::config::M4Config::parse(value["base"].clone())?;
        base.model.homeostasis_scope = match scope.as_str() {
            "none" | "all" => HomeostasisScope::All,
            "core" => HomeostasisScope::Core,
            "core_sensor" => HomeostasisScope::CoreSensor,
            _ => {
                return Err("unknown homeostasis_scope; use none, core, core_sensor or all".into());
            }
        };
        if base.model.homeostasis != (scope != "none") {
            return Err("base homeostasis switch disagrees with homeostasis_scope".into());
        }
        base.model.action_copy_weight = value["action_copy_weight"]
            .as_f64()
            .ok_or("action_copy_weight must be numeric")?
            as f32;
        base.model.core_membrane_tau_us = value["core_membrane_tau_us"]
            .as_f64()
            .ok_or("core_membrane_tau_us must be numeric")?
            as f32;
        base.model.validate()?;
        match experiment.as_str() {
            "motor_threshold" => {
                if base.sensor.mode != SensorMode::Effect2Ch
                    || base.sensor.coding != SpikeCoding::Deterministic
                    || base.model.processing_neurons != 32
                    || scope != "none"
                    || base.model.development
                    || !base.model.learning
                    || !base.model.feedback
                    || base.model.initial_structural_drive != 0.0
                {
                    return Err("motor_threshold requires deterministic effect_2ch, 32 Core, STDP/feedback on, homeostasis/development off, zero drive".into());
                }
            }
            "sensor_homeostasis" => {}
            _ => return Err("unknown M5 experiment".into()),
        }
        Ok(Self {
            base,
            experiment,
            scope,
            requested: value,
        })
    }
}

fn fingerprint(network: &Network) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in format!("{network:?}").bytes() {
        hash = (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

pub fn run(config: &Config, output: &Path, raw: bool, flag: &AtomicBool) -> Result<Value> {
    if config.experiment == "motor_threshold" {
        return motor(config, output, flag);
    }
    let initial = Model::with_sensor(config.base.model.clone(), config.base.sensor)?;
    let mut reference = config.base.model.clone();
    reference.homeostasis_scope = HomeostasisScope::All;
    let reference = Model::with_sensor(reference, config.base.sensor)?;
    if initial.network() != reference.network() {
        return Err("homeostasis selection changed the initial network".into());
    }
    let hash = fingerprint(initial.network());
    let mut result = m4::run_internal(&config.base, output, raw, flag, true)?;
    result["protocol"] = json!("NERVA-M5-v1");
    result["experiment"] = json!(config.experiment);
    result["homeostasis_scope"] = json!(config.scope);
    result["requested_config"] = config.requested.clone();
    result["initial_network_fingerprint"] = json!(hash);
    result["initial_equals_global_scope_network"] = json!(true);
    for arm in m4::config::ARMS {
        for mapping in ["normal_mapping", "reversed_mapping"] {
            for stats in result["arms"][arm][mapping]["m5_sensor"]["stages"]
                .as_object()
                .unwrap()
                .values()
            {
                let h = stats["homeostasis_updates_visual_core_motor_copy"]
                    .as_array()
                    .unwrap();
                let allowed = match config.scope.as_str() {
                    "none" => [false; 4],
                    "core" => [false, true, false, false],
                    "core_sensor" => [true, true, false, false],
                    _ => [true; 4],
                };
                if h.iter()
                    .zip(allowed)
                    .any(|(n, yes)| !yes && n.as_u64() != Some(0))
                {
                    return Err(
                        "homeostasis changed a neuron outside the selected population".into(),
                    );
                }
            }
        }
    }
    result["checks"]["homeostasis_population_respected"] = json!(true);
    Ok(result)
}

fn motor(config: &Config, output: &Path, flag: &AtomicBool) -> Result<Value> {
    let neutral = m4::world::image(3)?;
    let initial = Model::with_sensor(config.base.model.clone(), config.base.sensor)?
        .network()
        .clone();
    let mut path = output.as_os_str().to_owned();
    path.push(".events.csv");
    let mut log = BufWriter::new(
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(std::path::PathBuf::from(path))?,
    );
    writeln!(log, "learning,action,phase,event")?;
    let mut rows = Vec::new();
    for learning in [false, true] {
        for key in [None, Some(Key::A), Some(Key::D)] {
            if flag.load(Ordering::Relaxed) {
                return Err("M5 interrupted".into());
            }
            let mut settings = config.base.model.clone();
            settings.learning = learning;
            let mut model = Model::with_sensor(settings, config.base.sensor)?;
            if model.network() != &initial {
                return Err("motor intervention initial states differ".into());
            }
            model.enable_audit(AuditOptions {
                raw_events: true,
                input_spikes: true,
            });
            let mut core_spikes = Vec::new();
            let mut arrivals = Vec::new();
            let mut changes = Vec::new();
            let mut visual = 0;
            let mut copies = 0;
            let mut copy_arrivals = 0;
            // Three copies of the same held key, exactly 10 ms apart. All
            // conditions have the same pulse count/times and 50-ms horizon.
            for (phase, duration) in [10_000, 10_000, 30_000].into_iter().enumerate() {
                model.record_executed_keys(&key.into_iter().collect::<Vec<_>>())?;
                let step = model.step_for(&neutral, duration)?;
                core_spikes.extend(step.core_events);
                let audit = model.take_audit().ok_or("missing motor audit")?;
                visual += audit.visual_neuron_spikes + audit.emitted_sensor_spikes;
                copies += audit.action_copy_spikes;
                copy_arrivals += audit.action_copy_arrivals;
                for event in &audit.raw_events {
                    writeln!(
                        log,
                        "{learning},{},{phase},\"{}\"",
                        key.map(Key::qcode).unwrap_or("none"),
                        format!("{event:?}").replace('"', "\"\"")
                    )?;
                    match event {
                        ObservationEvent::SynapticArrival {
                            time,
                            synapse_id,
                            source,
                            target,
                            amplitude,
                        } => {
                            let edge = model.network().synapse(*synapse_id).unwrap();
                            if edge.is_plastic()
                                && model.processing_ids().contains(source)
                                && model.processing_ids().contains(target)
                            {
                                arrivals.push(json!({"time_us":time.0,"synapse_id":synapse_id.0,"source":source.0,"target":target.0,"amplitude":amplitude}));
                            }
                        }
                        ObservationEvent::WeightChanged {
                            time,
                            synapse_id,
                            old_weight,
                            new_weight,
                        } => {
                            let edge = model.network().synapse(*synapse_id).unwrap();
                            if model.processing_ids().contains(&edge.pre())
                                && model.processing_ids().contains(&edge.post())
                            {
                                changes.push(json!({"time_us":time.0,"synapse_id":synapse_id.0,"old":old_weight.get(),"new":new_weight.get()}));
                            }
                        }
                        _ => {}
                    }
                }
            }
            if visual != 0
                || copies != if key.is_some() { 3 } else { 0 }
                || copy_arrivals != if key.is_some() { 9 } else { 0 }
            {
                return Err("motor bench visual silence or fixed-copy timing failed".into());
            }
            if !learning && !learned_parameters_equal(&initial, model.network()) {
                return Err("frozen motor control changed parameters".into());
            }
            if key.is_none() && !core_spikes.is_empty() {
                return Err("no-copy neutral control emitted Core spikes".into());
            }
            let potentials: Result<Vec<_>> = model
                .processing_ids()
                .iter()
                .map(|&id| {
                    let mut neuron = model.network().neuron(id).unwrap().clone();
                    neuron.advance_to(SimTime(50_000))?;
                    Ok(neuron.membrane_potential())
                })
                .collect();
            let counts: Vec<_> = model
                .processing_ids()
                .iter()
                .map(|id| core_spikes.iter().filter(|(n, _)| *n == id.0).count())
                .collect();
            rows.push(json!({"learning":learning,"action":key.map(Key::qcode).unwrap_or("none"),"core_spikes":core_spikes,
                "core_spike_count":core_spikes.len(),"core_counts":counts,"core_potentials_at_50ms":potentials?,
                "generated_and_emitted_visual_spikes":visual,"copy_spikes":copies,"copy_arrivals":copy_arrivals,
                "plastic_core_arrivals":arrivals,"plastic_core_arrival_count":arrivals.len(),"core_weight_updates":changes,
                "core_weight_update_count":changes.len()}));
        }
    }
    log.flush()?;
    let a = rows
        .iter()
        .find(|r| r["learning"] == true && r["action"] == "a")
        .unwrap();
    let d = rows
        .iter()
        .find(|r| r["learning"] == true && r["action"] == "d")
        .unwrap();
    let distinct = a["core_counts"] != d["core_counts"];
    let paths = [a, d].iter().all(|r| {
        r["core_spike_count"].as_u64().unwrap() > 0
            && r["plastic_core_arrival_count"].as_u64().unwrap() > 0
    });
    Ok(
        json!({"protocol":"NERVA-M5-v1","experiment":"motor_threshold","case_id":config.base.case_id,"seed":config.base.seed,
        "requested_config":config.requested,"initial_network_fingerprint":fingerprint(&initial),"observations":rows,
        "checks":{"same_initial_network":true,"identical_neutral_images":true,"fixed_copy_pulse_count_and_timing":true,"frozen_control_preserved":true},
        "signal_criteria":{"both_actions_reach_plastic_core_paths":paths,"action_count_patterns_distinct":distinct,"both_pass":paths&&distinct},
        "protocol_details":{"copy_times_us":[1,10001,20001],"horizon_us":50000,"no_actions_executed_by_decoder":true,
            "interpretation":"Signal prerequisite only; distinct motor-induced activity and STDP changes are not learned action-effect prediction."}}),
    )
}
