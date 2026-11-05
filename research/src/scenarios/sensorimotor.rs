//! Size/feedback assay using the existing model and frame-only sensory port.
//! The evaluator owns the persistent world, prescribed exploration and decoder.
use crate::{
    Result, error, fingerprint, fingerprint_debug,
    scenario::{Intervention, Scenario, TrialMetrics},
};
use nerva::core::{NeuronRole, Polarity};
use nerva_qemu::{
    capture::Frame,
    keyboard::Key,
    test_world::{MotionWorld, PixelEnvironment},
};
use nerva_vm::{
    model::{
        HomeostasisScope, KeyboardLayout, Model, ModelConfig, SensorConfig, SensorMode,
        SpikeCoding,
        audit::{AuditOptions, learned_parameters_equal},
    },
    readout::Readout,
};
use serde_json::{Value, json};
use std::collections::BTreeMap;

pub const GROUPS: [&str; 6] = [
    "active",
    "active_frozen",
    "passive",
    "passive_frozen",
    "decoupled",
    "decoupled_frozen",
];
pub const PARAMETERS: [&str; 9] = [
    "processing_neurons",
    "train_trials",
    "test_trials",
    "action_delay_us",
    "max_observed_events",
    "max_total_neurons",
    "mechanisms.development",
    "mechanisms.homeostasis",
    "homeostasis.scope",
];
pub struct SensorimotorScenario;

#[derive(Clone, Debug)]
pub struct Config {
    pub processing_neurons: usize,
    pub train_trials: usize,
    pub test_trials: usize,
    pub action_delay_us: u64,
    pub max_observed_events: usize,
    pub max_total_neurons: usize,
    pub development: bool,
    pub homeostasis: bool,
    pub homeostasis_scope: HomeostasisScope,
}

fn fail(e: impl std::fmt::Display) -> nerva::experiment::ExperimentError {
    error(e.to_string())
}

fn model_config(c: &Config, seed: u64) -> ModelConfig {
    ModelConfig {
        seed,
        keyboard: KeyboardLayout::Ad,
        processing_neurons: c.processing_neurons,
        development: c.development,
        homeostasis: c.homeostasis,
        homeostasis_scope: c.homeostasis_scope,
        ..ModelConfig::default()
    }
}
fn sensor() -> SensorConfig {
    SensorConfig {
        mode: SensorMode::Position7Ch,
        coding: SpikeCoding::Deterministic,
        ..SensorConfig::default()
    }
}

fn scope_name(c: &Config) -> &'static str {
    if !c.homeostasis {
        return "off";
    }
    match c.homeostasis_scope {
        HomeostasisScope::All => "all",
        HomeostasisScope::Core => "core",
        HomeostasisScope::CoreSensor => "core_sensor",
    }
}

/// Read-only homeostatic-current summaries. These never advance neuron state.
fn intrinsic_summary(model: &Model) -> Value {
    let mut counts = [0usize; 4];
    let mut sums = [0f64; 4];
    let mut minima = [f64::INFINITY; 4];
    let mut maxima = [f64::NEG_INFINITY; 4];
    for neuron in model.network().neurons() {
        let population = if neuron.id().0 < model.sensor_config().channels() as u64 {
            0
        } else if neuron.role() == Some(NeuronRole::Processing) {
            1
        } else if neuron.role() == Some(NeuronRole::Motor) {
            2
        } else {
            3
        };
        let value = f64::from(neuron.intrinsic_current());
        counts[population] += 1;
        sums[population] += value;
        minima[population] = minima[population].min(value);
        maxima[population] = maxima[population].max(value);
    }
    let mut populations = serde_json::Map::new();
    for (i, name) in ["sensor", "core", "motor", "copy"].iter().enumerate() {
        populations.insert(
            (*name).into(),
            json!({"count":counts[i],"mean":sums[i]/counts[i] as f64,
            "min":minima[i],"max":maxima[i]}),
        );
    }
    Value::Object(populations)
}

/// Blocks return the world to position 3. All keys and directions are balanced.
/// Active/passive share the exact image sequence. Decoupled exploration crosses
/// every key with both directions once per block, preserving action marginals.
fn trajectory(seed: u64, trials: usize, decoupled: bool) -> Vec<(Key, Key)> {
    let mut state = seed;
    let mut rows = Vec::new();
    for _ in 0..trials / 4 {
        state = state.wrapping_add(0x9e3779b97f4a7c15);
        let first = if (state ^ (state >> 31)) & 1 == 0 {
            Key::A
        } else {
            Key::D
        };
        let other = if first == Key::A { Key::D } else { Key::A };
        let physical = [first, other, other, first];
        let keys = if decoupled {
            [first, first, other, other]
        } else {
            physical
        };
        rows.extend(keys.into_iter().zip(physical));
    }
    rows
}

#[derive(Default)]
struct Stats {
    events: usize,
    core_spikes: usize,
    sensor_spikes: usize,
    root_sensor_spikes: usize,
    homeostasis_by_population: [usize; 4],
    duration_us: u64,
    peak_rate_hz: f64,
    active_cells: Vec<bool>,
    copies: usize,
    weight_updates: usize,
    homeostasis_updates: usize,
    formed: usize,
    pruned: usize,
    maximum_synapses: usize,
    windows: Vec<Value>,
}

fn observe(
    model: &mut Model,
    c: &Config,
    stats: &mut Stats,
    frame: &Frame,
    duration: u64,
    key: Option<Key>,
    phase: &str,
) -> Result<Vec<f64>> {
    model
        .record_executed_keys(&key.into_iter().collect::<Vec<_>>())
        .map_err(fail)?;
    let step = model.step_for(frame, duration).map_err(fail)?;
    let audit = model
        .take_audit()
        .ok_or_else(|| error("missing sensorimotor audit"))?;
    let spikes: usize = step.core_counts.iter().sum();
    let rate = spikes as f64 * 1_000_000. / duration as f64 / c.processing_neurons as f64;
    stats.events += audit.raw_events.len();
    if stats.events > c.max_observed_events {
        return Err(error("sensorimotor event budget exhausted"));
    }
    stats.core_spikes += spikes;
    stats.sensor_spikes += audit.visual_neuron_spikes;
    stats.root_sensor_spikes += audit.emitted_sensor_spikes;
    for (total, &count) in stats
        .homeostasis_by_population
        .iter_mut()
        .zip(&audit.homeostasis_by_population)
    {
        *total += count;
    }
    stats.duration_us += duration;
    stats.peak_rate_hz = stats.peak_rate_hz.max(rate);
    if stats.active_cells.is_empty() {
        stats.active_cells.resize(c.processing_neurons, false);
    }
    for (active, &n) in stats.active_cells.iter_mut().zip(&step.core_counts) {
        *active |= n > 0;
    }
    stats.copies += step.action_copy_arrivals;
    stats.weight_updates += step.weight_changes;
    stats.homeostasis_updates += audit.homeostasis_updates;
    stats.formed += step.formed.len();
    stats.pruned += step.pruned.len();
    stats.maximum_synapses = stats.maximum_synapses.max(model.network().synapse_count());
    if (!model.config().development && (!step.formed.is_empty() || !step.pruned.is_empty()))
        || (!model.config().homeostasis && audit.homeostasis_updates > 0)
    {
        return Err(error("disabled mechanism produced updates"));
    }
    let allowed = if !model.config().homeostasis {
        [false; 4]
    } else {
        match model.config().homeostasis_scope {
            HomeostasisScope::All => [true; 4],
            HomeostasisScope::Core => [false, true, false, false],
            HomeostasisScope::CoreSensor => [true, true, false, false],
        }
    };
    if audit
        .homeostasis_by_population
        .iter()
        .zip(allowed)
        .any(|(&n, enabled)| !enabled && n > 0)
    {
        return Err(error(
            "homeostasis updated a population outside the declared scope",
        ));
    }
    if !model.config().learning
        && (step.weight_changes > 0
            || audit.homeostasis_updates > 0
            || !step.formed.is_empty()
            || !step.pruned.is_empty())
    {
        return Err(error("frozen model changed learned parameters"));
    }
    stats.windows.push(
        json!({"phase":phase,"start_us":step.start_us,"end_us":step.end_us,
        "core_spikes":spikes,"core_rate_hz":rate,"synapses":model.network().synapse_count(),
        "sensor_spikes":audit.visual_neuron_spikes,"root_sensor_spikes":audit.emitted_sensor_spikes,
        "sensor_rate_hz":audit.visual_neuron_spikes as f64*1_000_000./duration as f64/model.sensor_config().channels() as f64,
        "root_sensor_rate_hz":audit.emitted_sensor_spikes as f64*1_000_000./duration as f64/model.sensor_config().channels() as f64,
        "homeostasis_by_population":audit.homeostasis_by_population,"intrinsic_currents":intrinsic_summary(model),
        "weight_updates":step.weight_changes,"homeostasis_updates":audit.homeostasis_updates,
        "formed":step.formed.len(),"pruned":step.pruned.len(),
        "events":audit.raw_events.len(),"events_digest":fingerprint_debug(&audit.raw_events)}),
    );
    Ok(step.core_counts.iter().map(|&x| x as f64).collect())
}

fn visible_position(frame: &Frame) -> Result<i16> {
    let features = nerva_vm::model::sensory::SensorEncoder::new(sensor())
        .map_err(fail)?
        .features(frame)
        .map_err(fail)?;
    features
        .iter()
        .position(|&x| x == 1.)
        .map(|p| p as i16)
        .ok_or_else(|| error("no visible position"))
}

struct TrialContext<'a> {
    c: &'a Config,
    model: &'a mut Model,
    world: &'a mut MotionWorld,
    stats: &'a mut Stats,
    passive: bool,
}
impl TrialContext<'_> {
    fn trial(&mut self, key: Key, physical: Key, readout: Option<&Readout>) -> Result<Value> {
        let before = self.world.observe().map_err(fail)?;
        let position = visible_position(&before)?;
        observe(
            self.model, self.c, self.stats, &before, 50_000, None, "context",
        )?;
        let action_us = self.model.next_observation_us();
        let features = observe(
            self.model,
            self.c,
            self.stats,
            &before,
            self.c.action_delay_us,
            if self.passive { None } else { Some(key) },
            "pre_effect",
        )?;
        // Forecast is computed before world.act and before reading the future frame.
        let prediction = readout.map(|r| r.predict(&features));
        let forecast_us = self.model.next_observation_us() - 1;
        let effect_us = self.model.next_observation_us();
        if forecast_us >= effect_us || effect_us != action_us + self.c.action_delay_us {
            return Err(error("future image leaked into prediction"));
        }
        self.world.act(Some(physical)).map_err(fail)?;
        let after = self.world.observe().map_err(fail)?;
        let actual = visible_position(&after)?;
        observe(
            self.model, self.c, self.stats, &after, 50_000, None, "effect",
        )?;
        Ok(
            json!({"key":format!("{key:?}"),"physical_key":format!("{physical:?}"),
            "position_before":position,"actual_position":actual,"predicted_position":prediction,
            "action_us":action_us,"forecast_us":forecast_us,"effect_us":effect_us,
            "before_frame":fingerprint(before.rgb()),"after_frame":fingerprint(after.rgb())}),
        )
    }
}

impl Scenario for SensorimotorScenario {
    type Config = Config;
    fn name(&self) -> &'static str {
        "sensorimotor_size"
    }
    fn default_config(&self) -> Config {
        Config {
            processing_neurons: 256,
            train_trials: 8,
            test_trials: 8,
            action_delay_us: 20_000,
            max_observed_events: 2_000_000,
            max_total_neurons: 100_000,
            development: true,
            homeostasis: true,
            homeostasis_scope: HomeostasisScope::All,
        }
    }
    fn set_parameter(&self, c: &mut Config, name: &str, value: &Value) -> Result<()> {
        if name == "homeostasis.scope" {
            let scope = value
                .as_str()
                .ok_or_else(|| error("homeostasis.scope requires off/core/core_sensor/all"))?;
            c.homeostasis_scope = match scope {
                "off" | "all" => HomeostasisScope::All,
                "core" => HomeostasisScope::Core,
                "core_sensor" => HomeostasisScope::CoreSensor,
                _ => return Err(error("homeostasis.scope requires off/core/core_sensor/all")),
            };
            c.homeostasis = scope != "off";
            return Ok(());
        }
        if matches!(name, "mechanisms.development" | "mechanisms.homeostasis") {
            let enabled = value
                .as_bool()
                .ok_or_else(|| error("mechanism switches must be boolean"))?;
            if name == "mechanisms.development" {
                c.development = enabled;
            } else {
                c.homeostasis = enabled;
            }
            return Ok(());
        }
        let n = value
            .as_u64()
            .ok_or_else(|| error("sensorimotor parameters require unsigned integers"))?;
        let size = usize::try_from(n).map_err(fail)?;
        match name {
            "processing_neurons" => c.processing_neurons = size,
            "train_trials" => c.train_trials = size,
            "test_trials" => c.test_trials = size,
            "action_delay_us" => c.action_delay_us = n,
            "max_observed_events" => c.max_observed_events = size,
            "max_total_neurons" => c.max_total_neurons = size,
            _ => return Err(error(format!("unknown sensorimotor parameter: {name}"))),
        }
        Ok(())
    }
    fn validate(&self, c: &Config) -> Result<()> {
        model_config(c, 0).validate().map_err(fail)?;
        let total = c
            .processing_neurons
            .checked_add(sensor().channels() + KeyboardLayout::Ad.keys().len() * 2)
            .ok_or_else(|| error("total neuron count overflow"))?;
        if total > c.max_total_neurons {
            return Err(error("total neuron count exceeds max_total_neurons budget"));
        }
        for n in [c.train_trials, c.test_trials] {
            if !(4..=256).contains(&n) || !n.is_multiple_of(4) {
                return Err(error("trial counts must be multiples of four in 4..256"));
            }
        }
        if !(2_000..=50_000).contains(&c.action_delay_us) {
            return Err(error("action_delay_us must be 2000..50000"));
        }
        if !(1..=10_000_000).contains(&c.max_observed_events) {
            return Err(error("max_observed_events must be 1..10000000"));
        }
        Ok(())
    }
    fn snapshot(&self, c: &Config, seed: u64, i: &Intervention) -> Value {
        json!({"config":format!("{c:?}"),"model_config":format!("{:?}",model_config(c,seed)),
            "sensor_config":format!("{:?}",sensor()),"seed":seed,"intervention":i.name,
            "learning_enabled":!i.name.ends_with("_frozen"),
            "development_enabled":!i.name.ends_with("_frozen") && c.development,
            "homeostasis_enabled":!i.name.ends_with("_frozen") && c.homeostasis,
            "homeostasis_scope":scope_name(c),
            "total_initial_neurons":c.processing_neurons+sensor().channels()+KeyboardLayout::Ad.keys().len()*2,
            "world":"persistent seven-position MotionWorld; no reset within training/probe",
            "exploration":"prescribed balanced A/D actions, not an autonomous policy"})
    }
    fn interventions(&self) -> Vec<Intervention> {
        GROUPS
            .iter()
            .map(|s| Intervention { name: (*s).into() })
            .collect()
    }
    fn metric_names(&self) -> Vec<&'static str> {
        vec![
            "prediction_accuracy",
            "position_mae",
            "persistence_mae",
            "majority_accuracy",
            "visual_accuracy",
            "core_rate_hz",
            "peak_core_rate_hz",
            "silent_fraction",
            "observed_events",
            "processing_neurons",
            "total_neurons",
            "initial_synapses",
            "final_synapses",
            "maximum_synapses",
            "core_inhibitory_fraction",
            "weight_updates",
            "homeostasis_updates",
            "formed",
            "pruned",
            "action_copy_arrivals",
            "parameters_preserved",
            "sensor_rate_hz",
            "root_sensor_rate_hz",
            "training_sensor_rate_hz",
            "training_core_rate_hz",
            "measurement_sensor_rate_hz",
            "measurement_core_rate_hz",
            "sensor_intrinsic_current",
            "core_intrinsic_current",
            "motor_intrinsic_current",
            "copy_intrinsic_current",
            "sensor_homeostasis_updates",
            "core_homeostasis_updates",
            "motor_homeostasis_updates",
            "copy_homeostasis_updates",
        ]
    }
    fn run(&self, c: &Config, seed: u64, i: &Intervention) -> Result<TrialMetrics> {
        self.validate(c)?;
        if !GROUPS.contains(&i.name.as_str()) {
            return Err(error("unknown sensorimotor intervention"));
        }
        let mut model = Model::with_sensor(model_config(c, seed), sensor()).map_err(fail)?;
        let initial_hash = fingerprint_debug(model.network());
        let initial_currents = intrinsic_summary(&model);
        let initial_synapses = model.network().synapse_count();
        let inhibitory = model
            .processing_ids()
            .iter()
            .filter(|&&id| model.network().neuron(id).unwrap().polarity() == Polarity::Inhibitory)
            .count();
        model.enable_audit(AuditOptions {
            raw_events: true,
            input_spikes: false,
        });
        if i.name.ends_with("_frozen") {
            model.freeze().map_err(fail)?;
        }
        let initial = model.network().clone();
        let passive = i.name.starts_with("passive");
        let mut world = MotionWorld::new(3, false).map_err(fail)?;
        let mut stats = Stats {
            maximum_synapses: initial_synapses,
            ..Stats::default()
        };
        let mut training = Vec::new();
        for (key, physical) in trajectory(
            seed ^ 0x518cfd42,
            c.train_trials,
            i.name.starts_with("decoupled"),
        ) {
            training.push(
                TrialContext {
                    c,
                    model: &mut model,
                    world: &mut world,
                    stats: &mut stats,
                    passive,
                }
                .trial(key, physical, None)?,
            );
        }
        if i.name.ends_with("_frozen") && !learned_parameters_equal(&initial, model.network()) {
            return Err(error(
                "no-learning control changed parameters during training",
            ));
        }
        let training_windows = stats.windows.clone();
        let training_sensor_spikes = stats.sensor_spikes;
        let training_core_spikes = stats.core_spikes;
        let training_duration_us = stats.duration_us;
        let trained_currents = intrinsic_summary(&model);
        let trained_hash = fingerprint_debug(model.network());
        model.freeze().map_err(fail)?;
        let checkpoint = model.network().clone();
        // Visual-only external calibration. Labels never enter Model or its rules.
        let mut rows = Vec::new();
        let mut labels = Vec::new();
        let mut readout: Option<Readout> = None;
        let mut validation_correct = 0;
        for repeat in 0..2 {
            for offset in 0..7 {
                let position = ((offset * 3 + (seed % 7) as usize + repeat) % 7) as u8;
                let frame = MotionWorld::new(position, false)
                    .map_err(fail)?
                    .observe()
                    .map_err(fail)?;
                observe(
                    &mut model,
                    c,
                    &mut stats,
                    &frame,
                    50_000,
                    None,
                    "visual_context",
                )?;
                let feature = observe(
                    &mut model,
                    c,
                    &mut stats,
                    &frame,
                    c.action_delay_us,
                    None,
                    "visual_calibration",
                )?;
                if repeat == 0 {
                    rows.push(feature);
                    labels.push(position as i16);
                } else if readout.as_ref().unwrap().predict(&feature) == position as i16 {
                    validation_correct += 1;
                }
            }
            if repeat == 0 {
                readout = Some(Readout::fit(&rows, &labels));
            }
        }
        let readout = readout.unwrap();
        let mut probes = Vec::new();
        // Probe always uses the same coupled dynamics; only prior training differs.
        for (key, physical) in trajectory(seed ^ 0x7361adf0, c.test_trials, false) {
            probes.push(
                TrialContext {
                    c,
                    model: &mut model,
                    world: &mut world,
                    stats: &mut stats,
                    passive,
                }
                .trial(key, physical, Some(&readout))?,
            );
        }
        if !learned_parameters_equal(&checkpoint, model.network()) {
            return Err(error("measurement changed frozen parameters"));
        }
        let correct = probes
            .iter()
            .filter(|p| p["predicted_position"] == p["actual_position"])
            .count();
        let mae: f64 = probes
            .iter()
            .map(|p| {
                (p["predicted_position"].as_i64().unwrap() - p["actual_position"].as_i64().unwrap())
                    .abs() as f64
            })
            .sum::<f64>()
            / probes.len() as f64;
        let persistence = probes
            .iter()
            .map(|p| {
                (p["position_before"].as_i64().unwrap() - p["actual_position"].as_i64().unwrap())
                    .abs() as f64
            })
            .sum::<f64>()
            / probes.len() as f64;
        let mut classes = BTreeMap::new();
        for p in &probes {
            *classes
                .entry(p["actual_position"].as_i64().unwrap())
                .or_insert(0usize) += 1;
        }
        let numbers = [
            correct as f64 / probes.len() as f64,
            mae,
            persistence,
            *classes.values().max().unwrap() as f64 / probes.len() as f64,
            validation_correct as f64 / 7.,
            stats.core_spikes as f64 * 1_000_000.
                / stats.duration_us as f64
                / c.processing_neurons as f64,
            stats.peak_rate_hz,
            stats.active_cells.iter().filter(|&&x| !x).count() as f64 / c.processing_neurons as f64,
            stats.events as f64,
            c.processing_neurons as f64,
            model.network().neuron_count() as f64,
            initial_synapses as f64,
            model.network().synapse_count() as f64,
            stats.maximum_synapses as f64,
            inhibitory as f64 / c.processing_neurons as f64,
            stats.weight_updates as f64,
            stats.homeostasis_updates as f64,
            stats.formed as f64,
            stats.pruned as f64,
            stats.copies as f64,
            1.,
            stats.sensor_spikes as f64 * 1_000_000.
                / stats.duration_us as f64
                / sensor().channels() as f64,
            stats.root_sensor_spikes as f64 * 1_000_000.
                / stats.duration_us as f64
                / sensor().channels() as f64,
            training_sensor_spikes as f64 * 1_000_000.
                / training_duration_us as f64
                / sensor().channels() as f64,
            training_core_spikes as f64 * 1_000_000.
                / training_duration_us as f64
                / c.processing_neurons as f64,
            (stats.sensor_spikes - training_sensor_spikes) as f64 * 1_000_000.
                / (stats.duration_us - training_duration_us) as f64
                / sensor().channels() as f64,
            (stats.core_spikes - training_core_spikes) as f64 * 1_000_000.
                / (stats.duration_us - training_duration_us) as f64
                / c.processing_neurons as f64,
            trained_currents["sensor"]["mean"].as_f64().unwrap(),
            trained_currents["core"]["mean"].as_f64().unwrap(),
            trained_currents["motor"]["mean"].as_f64().unwrap(),
            trained_currents["copy"]["mean"].as_f64().unwrap(),
            stats.homeostasis_by_population[0] as f64,
            stats.homeostasis_by_population[1] as f64,
            stats.homeostasis_by_population[2] as f64,
            stats.homeostasis_by_population[3] as f64,
        ];
        Ok(TrialMetrics {
            values: self
                .metric_names()
                .into_iter()
                .map(String::from)
                .zip(numbers)
                .collect(),
            details: json!({"initial_network_digest":initial_hash,"trained_network_digest":trained_hash,
                "intrinsic_currents":{"initial":initial_currents,"trained":trained_currents,"final":intrinsic_summary(&model)},
                "final_network_digest":fingerprint_debug(model.network()),
                "training":training,"probes":probes,"training_curve":training_windows,"measurement_windows":stats.windows[training_windows.len()..],
                "checks":{"no_future_image":true,"frozen_measurement":true,"persistent_world":true},
                "limits":"Visual-only external position decoder; diagnostic frame features and prescribed exploration. No labels reach neural learning. No autonomous action-selection, memory or general cognition claim. Decoupled training is tested on coupled dynamics; visual calibration validation is only seven observations."}),
        })
    }
    fn evaluate(&self, _records: &[Value], _full_replay_checked: bool) -> Result<Value> {
        Ok(json!({"status":"sensorimotor_assay_evaluated_externally","passed":null}))
    }
}
