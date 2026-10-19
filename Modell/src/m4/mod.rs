//! M4 JSON protocol. Labels and worlds stay outside the frame-only model port.
pub mod config;
pub(crate) mod logging;
pub mod mechanisms;
pub mod normalization;
pub mod schedule;
pub mod world;
use crate::{
    Result,
    model::{
        Model, Step, WINDOW_US,
        audit::{AuditOptions, NetworkSummary, learned_parameters_equal},
    },
    readout::{Readout, chronological, core_features, score},
};
use config::{ARMS, M4Config};
use logging::{Logs, Stats};
use nerva_qemu::{capture::Frame, keyboard::Key};
use schedule::{Trial, schedule};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    io::Write,
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

pub const TRIAL_US: u64 = 160_000;
const BASE_US: u64 = 50_000;
const EFFECT_US: u64 = 50_000;
const TRAIN_SEED: u64 = 0x518cfd42;
const CALIBRATION_SEED: u64 = 0x18253abe;
const PROBE_SEED: u64 = 0x7361adf0;
const RETENTION_SEED: u64 = 0x925d4e17;
const INTERFERENCE_SEED: u64 = 0xaba7c193;

fn cancelled(flag: &AtomicBool) -> Result<()> {
    if flag.load(Ordering::Relaxed) {
        Err("M4 interrupted; partial sidecars remain, no completed result published".into())
    } else {
        Ok(())
    }
}
fn balanced(labels: &[i16]) -> bool {
    let left = labels.iter().filter(|&&x| x == -1).count();
    left > 0 && left * 2 == labels.len() && labels.iter().all(|&x| x == 1 || x == -1)
}
fn confusion(labels: &[i16], predicted: &[i16]) -> [[usize; 2]; 2] {
    let mut matrix = [[0; 2]; 2];
    for (&truth, &guess) in labels.iter().zip(predicted) {
        matrix[usize::from(truth > 0)][usize::from(guess > 0)] += 1;
    }
    matrix
}
fn mean(a: f64, b: f64) -> f64 {
    (a + b) / 2.0
}
fn network_summary(model: &Model) -> Value {
    let s = NetworkSummary::read(model.network(), 2.0);
    json!({"neurons":s.neurons,"synapses":s.synapses,"plastic_synapses":s.plastic_synapses,"active_pre_traces":s.active_pre_traces,"pre_trace_sum":s.pre_trace_sum,"pre_trace_max":s.pre_trace_max,"weights_at_zero":s.weights_at_zero,"weights_at_upper_bound":s.weights_at_upper_bound,"weight_sum":s.weight_sum,"utility_sum":s.utility_sum,"eligibility_sum":s.eligibility_sum})
}
pub(crate) fn membrane(model: &Model) -> Value {
    let potentials: Vec<_> = model
        .processing_ids()
        .iter()
        .map(|&id| model.network().neuron(id).unwrap().membrane_potential())
        .collect();
    let values: Vec<_> = potentials.iter().map(|&x| f64::from(x)).collect();
    json!({"core_potential_min":values.iter().copied().fold(f64::INFINITY,f64::min),"core_potential_mean":values.iter().sum::<f64>()/values.len() as f64,"core_potential_max":values.iter().copied().fold(f64::NEG_INFINITY,f64::max),"core_potentials":values,
        "core_structural_drive":model.processing_ids().iter().map(|&id|model.network().neuron(id).unwrap().structural_drive()).collect::<Vec<_>>(),"core_intrinsic_current":model.processing_ids().iter().map(|&id|model.network().neuron(id).unwrap().intrinsic_current()).collect::<Vec<_>>()})
}

pub(crate) fn sensor_state(model: &Model) -> Value {
    let channels = model.sensor_config().channels() as u64;
    let currents: Vec<_> = model
        .network()
        .neurons()
        .filter(|n| n.id().0 < channels)
        .map(|n| f64::from(n.intrinsic_current()))
        .collect();
    json!({"sensor_intrinsic_min":currents.iter().copied().fold(f64::INFINITY,f64::min),
        "sensor_intrinsic_mean":currents.iter().sum::<f64>()/currents.len() as f64,
        "sensor_intrinsic_max":currents.iter().copied().fold(f64::NEG_INFINITY,f64::max),
        "sensor_intrinsic_currents":currents})
}

/// Owns only the neural model and read-only diagnostics. No world, label,
/// future frame or scoring function can be stored here.
struct Port {
    model: Model,
    stats: BTreeMap<&'static str, Stats>,
    last_copy: Option<u64>,
    last_copy_count: usize,
    experience: Vec<(u64, u64, Option<Key>)>,
    record_experience: bool,
}
#[derive(Clone, Copy)]
struct Context {
    stage: &'static str,
    trial: usize,
    phase: &'static str,
}
impl Port {
    fn observe(
        &mut self,
        frame: &Frame,
        duration: u64,
        key: Option<Key>,
        context: Context,
        logs: &mut Logs,
        flag: &AtomicBool,
    ) -> Result<Step> {
        cancelled(flag)?;
        if self.record_experience {
            self.experience
                .push((duration, world::fingerprint(frame), key));
        }
        self.model
            .record_executed_keys(&key.into_iter().collect::<Vec<_>>())?;
        let mut remaining = duration;
        let mut last = None;
        self.last_copy = None;
        self.last_copy_count = 0;
        while remaining > 0 {
            let step = self.model.step_for(frame, remaining.min(WINDOW_US))?;
            let audit = self.model.take_audit().ok_or("missing M4 model audit")?;
            if !self.model.config().learning && step.weight_changes != 0 {
                return Err("STDP-off model produced weight updates".into());
            }
            if !self.model.config().feedback
                && (step.action_copy_arrivals != 0 || step.feedback_arrivals != 0)
            {
                return Err("feedback-off model received feedback".into());
            }
            if !self.model.config().development
                && (!step.formed.is_empty() || !step.pruned.is_empty())
            {
                return Err("development-off model changed its structure".into());
            }
            if context.stage != "training"
                && (audit.weight_updates > 0
                    || audit.homeostasis_updates > 0
                    || audit.formed > 0
                    || audit.pruned > 0)
            {
                return Err("plasticity remained active during M4 evaluation".into());
            }
            self.last_copy = self.last_copy.or(audit.first_action_copy_arrival_us);
            self.last_copy_count += audit.action_copy_arrivals;
            let mut neural = membrane(&self.model);
            if logs.extended() {
                let state = sensor_state(&self.model);
                for (key, value) in state.as_object().unwrap() {
                    neural[key] = value.clone();
                }
            }
            let inhibitory = step
                .core_events
                .iter()
                .filter(|&&(id, _)| {
                    self.model
                        .network()
                        .neuron(nerva::core::NeuronId(id))
                        .unwrap()
                        .polarity()
                        == nerva::core::Polarity::Inhibitory
                })
                .count();
            neural["inhibitory_core_spikes"] = json!(inhibitory);
            logs.phase(
                (context.stage, context.trial, context.phase),
                &step,
                &audit,
                &neural,
            )?;
            let stats = self.stats.entry(context.stage).or_default();
            stats.add(&step, &audit);
            stats.inhibitory += inhibitory;
            remaining -= remaining.min(WINDOW_US);
            last = Some(step);
        }
        last.ok_or_else(|| "zero-duration M4 observation".into())
    }
}
struct Images {
    frames: Vec<Frame>,
    hashes: Vec<u64>,
}
impl Images {
    fn new() -> Result<Self> {
        let frames: Result<Vec<_>> = (0..7).map(world::image).collect();
        let frames = frames?;
        let hashes = frames.iter().map(world::fingerprint).collect();
        Ok(Self { frames, hashes })
    }
}

struct Calibration {
    readouts: [Readout; 2],
    validation: [Value; 2],
    labels: Vec<i16>,
    pixel_readout: Readout,
}
struct Session<'a> {
    config: &'a M4Config,
    images: &'a Images,
    logs: &'a mut Logs,
    flag: &'a AtomicBool,
    reversed: bool,
}
impl Session<'_> {
    fn calibration(&mut self, port: &mut Port, seed: u64) -> Result<Calibration> {
        let mut rows = [Vec::new(), Vec::new()];
        let mut labels = Vec::new();
        let mut pixel_rows = Vec::new();
        for (trial, item) in schedule(seed, self.config.calibration_trials, false, false)
            .into_iter()
            .enumerate()
        {
            port.observe(
                &self.images.frames[3],
                BASE_US,
                None,
                Context {
                    stage: "calibration",
                    trial,
                    phase: "baseline",
                },
                self.logs,
                self.flag,
            )?;
            let position = if item.shown_motion < 0 { 2 } else { 4 };
            let step = port.observe(
                &self.images.frames[position],
                self.config.delay_us,
                None,
                Context {
                    stage: "calibration",
                    trial,
                    phase: "visual",
                },
                self.logs,
                self.flag,
            )?;
            for (kind, row) in rows.iter_mut().enumerate() {
                row.push(core_features(&step, port.model.processing_ids(), kind == 1));
            }
            pixel_rows.push(
                nerva_qemu::vision_encoder::grayscale(&self.images.frames[position], 8, 8)?
                    .into_iter()
                    .map(f64::from)
                    .collect(),
            );
            labels.push(world::visible_effect(&self.images.frames[position])?);
            port.observe(
                &self.images.frames[3],
                TRIAL_US - BASE_US - self.config.delay_us,
                None,
                Context {
                    stage: "calibration",
                    trial,
                    phase: "padding",
                },
                self.logs,
                self.flag,
            )?;
        }
        if !balanced(&labels) {
            return Err("visual calibration is not balanced".into());
        }
        Ok(Calibration {
            readouts: [
                Readout::fit(&rows[0], &labels),
                Readout::fit(&rows[1], &labels),
            ],
            validation: [
                chronological(&rows[0], &labels),
                chronological(&rows[1], &labels),
            ],
            pixel_readout: Readout::fit(&pixel_rows, &labels),
            labels,
        })
    }
    fn action_trial(
        &mut self,
        port: &mut Port,
        trial: Trial,
        index: usize,
        stage: &'static str,
        readouts: Option<&Calibration>,
    ) -> Result<([i16; 2], i16, Option<Key>)> {
        let start = &self.images.frames[3];
        let trial_start = port.model.next_observation_us();
        port.observe(
            start,
            BASE_US,
            None,
            Context {
                stage,
                trial: index,
                phase: "baseline",
            },
            self.logs,
            self.flag,
        )?;
        let action_us = port.model.next_observation_us();
        let mut world = world::DelayedWorld::new(self.reversed)?;
        world.execute(trial.key, action_us, self.config.delay_us)?;
        let cue = port.observe(
            start,
            self.config.delay_us,
            Some(trial.key),
            Context {
                stage,
                trial: index,
                phase: "action",
            },
            self.logs,
            self.flag,
        )?;
        let first_copy = port.last_copy;
        let copies = port.last_copy_count;
        let forecast_us = cue.end_us;
        if world.motion_us.is_some()
            || cue.start_us != action_us
            || cue.core_events.iter().any(|&(_, time)| time > forecast_us)
        {
            return Err("action-time features contain a future event".into());
        }
        // Compute predictions before advancing the world or obtaining an effect image.
        let predictions = if let Some(calibration) = readouts {
            [
                calibration.readouts[0].predict(&core_features(
                    &cue,
                    port.model.processing_ids(),
                    false,
                )),
                calibration.readouts[1].predict(&core_features(
                    &cue,
                    port.model.processing_ids(),
                    true,
                )),
            ]
        } else {
            [0, 0]
        };
        let effect_us = port.model.next_observation_us();
        world.advance(effect_us)?;
        let actual = world::visible_effect(&world.observe_native()?)?;
        let motion_us = world
            .motion_us
            .ok_or("delayed physical action was not executed")?;
        if forecast_us >= motion_us
            || motion_us != action_us + self.config.delay_us
            || effect_us != motion_us
        {
            return Err("M4 action/motion/image causal ordering failed".into());
        }
        if stage != "training" && actual != trial.shown_motion {
            return Err("probe frame does not match real action effect".into());
        }
        let position = if trial.shown_motion < 0 { 2 } else { 4 };
        port.observe(
            &self.images.frames[position],
            EFFECT_US,
            None,
            Context {
                stage,
                trial: index,
                phase: "effect",
            },
            self.logs,
            self.flag,
        )?;
        port.observe(
            start,
            TRIAL_US - BASE_US - self.config.delay_us - EFFECT_US,
            None,
            Context {
                stage,
                trial: index,
                phase: "padding",
            },
            self.logs,
            self.flag,
        )?;
        if port.model.next_observation_us() - trial_start != TRIAL_US {
            return Err("M4 trial duration is not constant".into());
        }
        writeln!(
            self.logs.details,
            "{},{},{stage},{index},{},{action_us},{motion_us},{forecast_us},{effect_us},{},{actual},{},{},{copies},{},{},{},{},{:016x},{:016x}",
            self.logs.arm,
            self.logs.mapping,
            trial.key.qcode(),
            trial.shown_motion,
            if readouts.is_some() {
                predictions[0].to_string()
            } else {
                String::new()
            },
            if readouts.is_some() {
                predictions[1].to_string()
            } else {
                String::new()
            },
            first_copy.map(|x| x.to_string()).unwrap_or_default(),
            cue.core_events.len(),
            cue.core_events
                .first()
                .map(|x| x.1.to_string())
                .unwrap_or_default(),
            cue.core_events
                .last()
                .map(|x| x.1.to_string())
                .unwrap_or_default(),
            self.images.hashes[3],
            self.images.hashes[position]
        )?;
        Ok((predictions, actual, cue.action))
    }
    fn probe(
        &mut self,
        port: &mut Port,
        calibration: &Calibration,
        seed: u64,
        stage: &'static str,
    ) -> Result<Value> {
        let mut labels = Vec::new();
        let mut guesses = [Vec::new(), Vec::new()];
        let mut selected = 0;
        let mut pixel_predictions = Vec::new();
        let pixel_features: Vec<_> =
            nerva_qemu::vision_encoder::grayscale(&self.images.frames[3], 8, 8)?
                .into_iter()
                .map(f64::from)
                .collect();
        let mut a = 0;
        let mut d = 0;
        for (index, trial) in schedule(seed, self.config.test_trials, self.reversed, false)
            .into_iter()
            .enumerate()
        {
            let pixel_prediction = calibration.pixel_readout.predict(&pixel_features);
            let (predictions, actual, action) =
                self.action_trial(port, trial, index, stage, Some(calibration))?;
            pixel_predictions.push(pixel_prediction);
            labels.push(actual);
            guesses[0].push(predictions[0]);
            guesses[1].push(predictions[1]);
            selected += usize::from(action.is_some());
            a += usize::from(trial.key == Key::A);
            d += usize::from(trial.key == Key::D);
        }
        if !balanced(&labels) || a != d {
            return Err("M4 probe labels or actions are not balanced".into());
        }
        Ok(
            json!({"count":score(&labels,&guesses[0]),"time_bins":score(&labels,&guesses[1]),"prediction_confusion_matrix":confusion(&labels,&guesses[0]),"action_selection_rate":selected as f64/labels.len() as f64,"action_counts":{"a":a,"d":d},"test_count":labels.len(),"pixel_baseline":score(&labels,&pixel_predictions),"all_predictions_before_motion_and_image":true}),
        )
    }
}

fn anchors(config: &M4Config, images: &Images) -> Result<Value> {
    use crate::model::{SensorMode, sensory::SensorEncoder};
    let encoder = SensorEncoder::new(config.sensor)?;
    let features: Result<Vec<_>> = images
        .frames
        .iter()
        .map(|frame| encoder.features(frame))
        .collect();
    let features = features?;
    for i in 0..7 {
        for j in i + 1..7 {
            if images.hashes[i] == images.hashes[j] {
                return Err("canonical images alias different positions".into());
            }
            if config.sensor.mode != SensorMode::Effect2Ch && features[i] == features[j] {
                return Err("sensor aliases distinct positions".into());
            }
        }
    }
    if features[2] == features[4] {
        return Err("left and right sensor effects alias".into());
    }
    Ok(
        json!({"canonical_resolution":world::CANONICAL,"requested_resolution":config.requested["params"]["resolution"],"effective_sampling_resolution":config.sensor.resolution,"sensor_channels":config.sensor.channels(),"canonical_positions_distinct":true,"all_seven_sensor_positions_distinct":config.sensor.mode!=SensorMode::Effect2Ch,"left_right_sensor_effects_distinct":true,"diagnostic_features_from_current_frame_only":true,"sampling":"256-square canonical raster, 4x exact tiling of existing 64-square renderer; floating-point rectangle area averages before coding"}),
    )
}

pub fn run(config: &M4Config, output: &Path, raw: bool, flag: &AtomicBool) -> Result<Value> {
    run_internal(config, output, raw, flag, false)
}
pub(crate) fn run_internal(
    config: &M4Config,
    output: &Path,
    raw: bool,
    flag: &AtomicBool,
    extended: bool,
) -> Result<Value> {
    let images = Images::new()?;
    let anchor = anchors(config, &images)?;
    let initial_model = Model::with_sensor(config.model.clone(), config.sensor)?;
    let initial = initial_model.network().clone();
    let (gain, normalization) = normalization::gain(&initial_model, config)?;
    if initial_model.processing_ids().iter().any(|&id| {
        initial.neuron(id).unwrap().structural_drive() != config.model.initial_structural_drive
    }) {
        return Err("initial drive was overridden".into());
    }
    let mut logs = Logs::new(output, raw, extended)?;
    let mut mappings: BTreeMap<&str, BTreeMap<&str, Value>> = BTreeMap::new();
    for reversed in [false, true] {
        let mapping = if reversed { "reversed" } else { "normal" };
        let domain = if reversed { 0x3f28ae71 } else { 0 };
        let train_seed = config.seed ^ TRAIN_SEED ^ domain;
        let train = schedule(train_seed, config.train_trials, reversed, false);
        let shuffled = schedule(train_seed, config.train_trials, reversed, true);
        if train.iter().zip(&shuffled).any(|(a, b)| a.key != b.key) {
            return Err("scrambled action sequence changed".into());
        }
        let mut matched_training = None;
        let mut paired_labels = None;
        let mut results = BTreeMap::new();
        for arm in ARMS {
            cancelled(flag)?;
            logs.arm = arm;
            logs.mapping = mapping;
            let base = if matches!(arm, "before" | "aged_frozen") {
                config.model.clone().frozen()
            } else {
                config.model.clone()
            };
            let mut model = Model::with_sensor(base, config.sensor)?;
            if model.network() != &initial {
                return Err("M4 arms have different initial networks".into());
            }
            model.set_sensory_gain(gain)?;
            model.enable_audit(AuditOptions {
                raw_events: raw,
                input_spikes: true,
            });
            let mut port = Port {
                model,
                stats: BTreeMap::new(),
                last_copy: None,
                last_copy_count: 0,
                experience: Vec::new(),
                record_experience: true,
            };
            let before_summary = network_summary(&port.model);
            let before_sensor = if extended {
                sensor_state(&port.model)
            } else {
                Value::Null
            };
            let mut labels = Vec::new();
            let mut contingency = [0usize; 4];
            {
                let mut session = Session {
                    config,
                    images: &images,
                    logs: &mut logs,
                    flag,
                    reversed,
                };
                if arm != "before" {
                    for (index, &trial) in if arm == "scrambled" {
                        &shuffled
                    } else {
                        &train
                    }
                    .iter()
                    .enumerate()
                    {
                        session.action_trial(&mut port, trial, index, "training", None)?;
                        labels.push(trial.shown_motion);
                        contingency[usize::from(trial.key == Key::D) * 2
                            + usize::from(trial.shown_motion > 0)] += 1;
                    }
                    if !balanced(&labels) {
                        return Err("training effect labels are not balanced".into());
                    }
                    if arm == "aged_frozen" {
                        matched_training = Some(port.experience.clone());
                        paired_labels = Some(labels.clone());
                    }
                    if arm == "paired" && matched_training.as_ref() != Some(&port.experience) {
                        return Err("paired and frozen training experiences differ".into());
                    }
                    if arm == "scrambled" {
                        if contingency != [config.train_trials / 4; 4] {
                            return Err(
                                "scrambled action/effect contingencies are not independent".into(),
                            );
                        }
                        if labels.iter().filter(|&&x| x == -1).count()
                            != paired_labels
                                .as_ref()
                                .unwrap()
                                .iter()
                                .filter(|&&x| x == -1)
                                .count()
                        {
                            return Err("scrambled image marginals changed".into());
                        }
                    }
                }
            }
            port.record_experience = false;
            let trained_summary = network_summary(&port.model);
            let trained_membrane = membrane(&port.model);
            let trained_sensor = if extended {
                sensor_state(&port.model)
            } else {
                Value::Null
            };
            let checkpoint = port.model.network().clone();
            port.model.freeze()?;
            if port.model.network() != &checkpoint {
                return Err("freezing reset the running M4 model".into());
            }
            let mut session = Session {
                config,
                images: &images,
                logs: &mut logs,
                flag,
                reversed,
            };
            let calibration =
                session.calibration(&mut port, config.seed ^ CALIBRATION_SEED ^ domain)?;
            if !balanced(&calibration.labels) {
                return Err("calibration label audit failed".into());
            }
            let immediate = session.probe(
                &mut port,
                &calibration,
                config.seed ^ PROBE_SEED ^ domain,
                "probe",
            )?;
            for trial in 0..40 {
                port.observe(
                    &images.frames[3],
                    WINDOW_US,
                    None,
                    Context {
                        stage: "rest",
                        trial,
                        phase: "neutral",
                    },
                    session.logs,
                    flag,
                )?;
            }
            let retained = session.probe(
                &mut port,
                &calibration,
                config.seed ^ RETENTION_SEED ^ domain,
                "retained_probe",
            )?;
            for trial in 0..28 {
                port.observe(
                    &images.frames[trial % 7],
                    WINDOW_US,
                    None,
                    Context {
                        stage: "interference",
                        trial,
                        phase: "visual",
                    },
                    session.logs,
                    flag,
                )?;
            }
            let interfered = session.probe(
                &mut port,
                &calibration,
                config.seed ^ INTERFERENCE_SEED ^ domain,
                "interference_probe",
            )?;
            if !learned_parameters_equal(&checkpoint, port.model.network()) {
                return Err("frozen M4 parameters changed during measurement".into());
            }
            let diagnostics: serde_json::Map<String, Value> = port
                .stats
                .iter()
                .map(|(&stage, stats)| (stage.to_owned(), stats.value()))
                .collect();
            let train_stats = port
                .stats
                .get("training")
                .map(Stats::value)
                .unwrap_or_else(|| Stats::default().value());
            let mut result = json!({"balanced_accuracy":immediate["count"]["balanced_accuracy"],"visual_decodability":calibration.validation[0]["balanced_accuracy"],"visual_calibration_accuracy":calibration.validation[0],"visual_calibration_time_bins":calibration.validation[1],
                "test_count":config.test_trials,"action_selection_rate":immediate["action_selection_rate"],"action_counts":immediate["action_counts"],"prediction_confusion_matrix":immediate["prediction_confusion_matrix"],
                "spike_count":train_stats["spike_count"],"refractory_arrival_fraction":train_stats["refractory_arrival_fraction"],"formed":train_stats["formed"],"pruned":train_stats["pruned"],"weight_updates":train_stats["weight_updates"],"motor_feedback_arrivals":train_stats["motor_feedback_arrivals"],"synapse_count":port.model.network().synapse_count(),
                "probe":immediate,"retained_probe":retained,"interference_probe":interfered,"initial_network":before_summary,"trained_network":trained_summary,"trained_neural_state":trained_membrane,"final_neural_state":membrane(&port.model),"parameters_preserved":true,"training_contingency_a_left_a_right_d_left_d_right":contingency,"stage_diagnostics":diagnostics});
            if extended {
                let sensor_stats: BTreeMap<_, _> = port
                    .stats
                    .iter()
                    .map(|(stage, stats)| (*stage, stats.sensor_value()))
                    .collect();
                result["m5_sensor"] = json!({"initial":before_sensor,"trained":trained_sensor,
                    "final":sensor_state(&port.model),"stages":sensor_stats});
            }
            results.insert(arm, result);
        }
        mappings.insert(mapping, results);
    }
    let mut arms = serde_json::Map::new();
    for arm in ARMS {
        let a = &mappings["normal"][arm];
        let b = &mappings["reversed"][arm];
        let mut matrix = [[0usize; 2]; 2];
        for (i, row) in matrix.iter_mut().enumerate() {
            for (j, x) in row.iter_mut().enumerate() {
                *x = a["prediction_confusion_matrix"][i][j].as_u64().unwrap() as usize
                    + b["prediction_confusion_matrix"][i][j].as_u64().unwrap() as usize;
            }
        }
        let mut result = json!({"normal_mapping":a,"reversed_mapping":b,"test_count":2*config.test_trials,"prediction_confusion_matrix":matrix,"chance_level":0.5});
        for field in [
            "balanced_accuracy",
            "visual_decodability",
            "action_selection_rate",
            "refractory_arrival_fraction",
        ] {
            result[field] = json!(mean(a[field].as_f64().unwrap(), b[field].as_f64().unwrap()));
        }
        for field in [
            "spike_count",
            "formed",
            "pruned",
            "weight_updates",
            "motor_feedback_arrivals",
            "synapse_count",
        ] {
            result[field] = json!(a[field].as_u64().unwrap() + b[field].as_u64().unwrap());
        }
        result["visual_calibration_accuracy"] = result["visual_decodability"].clone();
        result["action_counts"] = json!({"a":config.test_trials,"d":config.test_trials});
        arms.insert(arm.to_owned(), result);
    }
    logs.flush()?;
    Ok(
        json!({"protocol":"NERVA-M4-v1","case_id":config.case_id,"seed":config.seed,"arms":arms,
        "checks":{"same_initial_network":true,"matched_experience":true,"no_leakage":true,"frozen_probe":true,"balanced_labels":true},
        "sensor_anchor":anchor,"normalization":normalization,"requested_config":config.requested,
        "protocol_details":{"trial_us":TRIAL_US,"baseline_us":BASE_US,"action_effect_delay_us":config.delay_us,"effect_observation_us":EFFECT_US,"readout_window_us":config.delay_us,"rest_us":2_000_000,"interference_us":1_400_000,
            "readout":"M3 standardized visual-only centroids, count and eight relative time bins; independent chronological visual validation",
            "data_split_seeds":{"training":config.seed^TRAIN_SEED,"calibration":config.seed^CALIBRATION_SEED,"probe":config.seed^PROBE_SEED,"retained":config.seed^RETENTION_SEED,"interference":config.seed^INTERFERENCE_SEED,"reversed_domain":0x3f28ae71u64},
            "model_input_fields":["current_frame","executed_key"],"matched_experience_definition":"paired equals aged_frozen in actual training frames/actions/timing; scrambled preserves executed keys/timing and frame marginals, with independent pairings",
            "limits":["Diagnostic sensors are hand-defined visible features, not autonomous vision","Short-delay visual validation can be weak; prediction negatives then do not exclude internal information","The fixed normalizer matches initial population-mean expected current, not every cell or learned current","Controlled exploration and visual-to-action transfer do not demonstrate autonomous goal pursuit"]}}),
    )
}
