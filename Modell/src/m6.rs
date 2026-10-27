//! Combined M6 experiment. Calibration and checkpoint probes use deterministic
//! replay branches; the running parent is never calibrated or reset.
use crate::{
    Result,
    m4::{
        self,
        config::{ARMS, M4Config},
        logging::{Logs, Stats},
        schedule::{Trial, schedule},
        world,
    },
    model::{
        HomeostasisScope, Model, SensorMode, SpikeCoding, Step,
        audit::{AuditOptions, InputAudit, learned_parameters_equal},
    },
    readout::{Readout, chronological, core_features, score},
};
use nerva_qemu::{capture::Frame, keyboard::Key};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs::OpenOptions,
    io::{BufWriter, Write},
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

const BASE: u64 = 50_000;
const CUE: u64 = 50_000;
const EFFECT: u64 = 50_000;
const TRIAL: u64 = 160_000;
const TRAIN_SEED: u64 = 0x518cfd42;
const CAL_SEED: u64 = 0x18253abe;
const PROBE_SEED: u64 = 0x7361adf0;
const REST_SEED: u64 = 0x925d4e17;
const INTER_SEED: u64 = 0xaba7c193;

pub mod mechanisms;
mod weight_resets;

/// Optional measurements for subsequent experiments. The default preserves the
/// historical M6 protocol and file contract exactly.
#[derive(Default)]
pub struct RunOptions {
    pub reference_gain: Option<f32>,
    pub checkpoint_rest_us: Option<u64>,
    pub propagation: bool,
    pub weight_resets: bool,
    pub weight_reset_trace: bool,
    pub recurrence_probes: bool,
    pub crossed_weight_probes: bool,
}

pub struct Config {
    pub base: M4Config,
    pub variant: String,
    pub copies: usize,
    pub requested: Value,
}
impl Config {
    pub fn parse(value: Value) -> Result<Self> {
        use m4::config::{fields, string};
        fields(&value, &["protocol", "variant", "base"])?;
        if string(&value, "protocol")? != "NERVA-M6-v1" {
            return Err("protocol must be NERVA-M6-v1".into());
        }
        let variant = string(&value, "variant")?.to_owned();
        let (copies, sensor_homeostasis) = match variant.as_str() {
            "A" => (1, true),
            "B" => (3, true),
            "C" => (1, false),
            "D" => (3, false),
            _ => return Err("variant must be A, B, C or D".into()),
        };
        let mut base = M4Config::parse(value["base"].clone())?;
        let valid_sensor = (base.sensor.mode == SensorMode::Effect2Ch
            && base.model.processing_neurons == 32)
            || (base.sensor.mode == SensorMode::Pixels
                && base.sensor.resolution == 16
                && base.model.processing_neurons == 128);
        if !valid_sensor
            || base.sensor.coding != SpikeCoding::Deterministic
            || base.delay_us != CUE
            || !base.model.learning
            || (!base.model.homeostasis && sensor_homeostasis)
            || !base.model.feedback
            || base.model.development
            || base.model.initial_structural_drive != 0.0
        {
            return Err("M6 requires effect_2ch/32 or pixels16/128, deterministic, delay50ms, STDP/feedback on, development off and zero drive; Core-homeostasis off is supported only with visual homeostasis off (C/D)".into());
        }
        base.model.homeostasis_scope = if sensor_homeostasis {
            HomeostasisScope::CoreSensor
        } else {
            HomeostasisScope::Core
        };
        base.model.action_copy_weight = 0.9;
        base.model.core_membrane_tau_us = 20_000.0;
        Ok(Self {
            base,
            variant,
            copies,
            requested: value,
        })
    }
}
fn ensure(ok: bool, message: &str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(message.to_owned().into())
    }
}
fn balanced(labels: &[i16]) -> bool {
    labels.iter().filter(|&&x| x < 0).count() * 2 == labels.len()
        && labels.iter().all(|&x| x == -1 || x == 1)
}
fn fingerprint(model: &Model) -> String {
    format!(
        "{:016x}",
        format!("{:?}", model.network())
            .bytes()
            .fold(0xcbf29ce484222325u64, |h, b| (h ^ u64::from(b))
                .wrapping_mul(0x100000001b3))
    )
}
fn writer(output: &Path, suffix: &str) -> Result<BufWriter<std::fs::File>> {
    let mut path = output.as_os_str().to_owned();
    path.push(suffix);
    Ok(BufWriter::new(
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(std::path::PathBuf::from(path))?,
    ))
}

/// Neural input boundary: no world, positions, labels or future frames.
struct Actor {
    model: Model,
}
impl Actor {
    fn new(config: &Config, arm: &str, gain: f32, raw: bool) -> Result<Self> {
        let settings = if matches!(arm, "before" | "aged_frozen") {
            config.base.model.clone().frozen()
        } else {
            config.base.model.clone()
        };
        let mut model = Model::with_sensor(settings, config.base.sensor)?;
        model.set_sensory_gain(gain)?;
        model.enable_audit(AuditOptions {
            raw_events: raw,
            input_spikes: true,
        });
        Ok(Self { model })
    }
    fn observe(
        &mut self,
        frame: &Frame,
        key: Option<Key>,
        duration: u64,
    ) -> Result<(Step, InputAudit)> {
        self.model
            .record_executed_keys(&key.into_iter().collect::<Vec<_>>())?;
        let step = self.model.step_for(frame, duration)?;
        let audit = self.model.take_audit().ok_or("missing M6 audit")?;
        ensure(
            self.model.config().learning || audit.weight_updates == 0,
            "frozen STDP update",
        )?;
        ensure(
            self.model.config().homeostasis || audit.homeostasis_updates == 0,
            "frozen homeostasis update",
        )?;
        ensure(
            audit.formed == 0 && audit.pruned == 0,
            "unexpected M6 structure change",
        )?;
        let h = audit.homeostasis_by_population;
        ensure(
            h[2] == 0 && h[3] == 0,
            "M6 motor/copy homeostasis was enabled",
        )?;
        if self.model.config().homeostasis_scope == HomeostasisScope::Core {
            ensure(h[0] == 0, "visual homeostasis in Core-only variant")?;
        }
        Ok((step, audit))
    }
}
struct Op {
    frame: Arc<Frame>,
    key: Option<Key>,
    duration: u64,
    expected: Step,
}
struct Calibration {
    readouts: [Readout; 2],
    visual: [Value; 2],
    pixel: Readout,
}
struct Prediction {
    guesses: [i16; 2],
    effect: i16,
    pixel: i16,
    selected: bool,
}
struct Evaluation {
    value: Value,
}
struct Session<'a> {
    config: &'a Config,
    frames: &'a [Arc<Frame>],
    logs: &'a mut Logs,
    copies: &'a mut BufWriter<std::fs::File>,
    flag: &'a AtomicBool,
    stats: BTreeMap<&'static str, Stats>,
    tape: Vec<Op>,
    record: bool,
    reversed: bool,
    propagation: &'a mut Option<BufWriter<std::fs::File>>,
}
impl Session<'_> {
    fn observe(
        &mut self,
        actor: &mut Actor,
        image: usize,
        key: Option<Key>,
        duration: u64,
        meta: (&'static str, usize, &'static str),
    ) -> Result<(Step, InputAudit)> {
        ensure(!self.flag.load(Ordering::Relaxed), "M6 interrupted")?;
        let (step, audit) = actor.observe(&self.frames[image], key, duration)?;
        if self.record {
            self.tape.push(Op {
                frame: self.frames[image].clone(),
                key,
                duration,
                expected: step.clone(),
            });
        }
        let mut state = m4::membrane(&actor.model);
        for (key, value) in m4::sensor_state(&actor.model).as_object().unwrap() {
            state[key] = value.clone();
        }
        let inhibitory = step
            .core_events
            .iter()
            .filter(|&&(id, _)| {
                actor
                    .model
                    .network()
                    .neuron(nerva::core::NeuronId(id))
                    .unwrap()
                    .polarity()
                    == nerva::core::Polarity::Inhibitory
            })
            .count();
        state["inhibitory_core_spikes"] = json!(inhibitory);
        self.logs.phase(meta, &step, &audit, &state)?;
        let stats = self.stats.entry(meta.0).or_default();
        stats.add(&step, &audit);
        stats.inhibitory += inhibitory;
        Ok((step, audit))
    }
    fn replay(&self, parent: &Actor, arm: &str, gain: f32, raw: bool) -> Result<Actor> {
        let mut fork = Actor::new(self.config, arm, gain, raw)?;
        for op in &self.tape {
            ensure(!self.flag.load(Ordering::Relaxed), "M6 replay interrupted")?;
            let (step, _) = fork.observe(&op.frame, op.key, op.duration)?;
            ensure(step == op.expected, "replayed neural trajectory differs")?;
        }
        fork.model.freeze()?;
        ensure(
            parent.model.same_replay_state(&fork.model),
            "replayed checkpoint differs: neural state, STDP traces, queue, generator or decoder",
        )?;
        Ok(fork)
    }
    fn calibrate(&mut self, actor: &mut Actor) -> Result<Calibration> {
        let mut rows = [Vec::new(), Vec::new()];
        let mut pixels = Vec::new();
        let mut labels = Vec::new();
        for (index, trial) in schedule(
            self.config.base.seed ^ CAL_SEED,
            self.config.base.calibration_trials,
            false,
            false,
        )
        .into_iter()
        .enumerate()
        {
            self.observe(
                actor,
                3,
                None,
                BASE,
                ("calibration_fork", index, "baseline"),
            )?;
            let image = if trial.shown_motion < 0 { 2 } else { 4 };
            let (step, _) = self.observe(
                actor,
                image,
                None,
                CUE,
                ("calibration_fork", index, "visual"),
            )?;
            for (kind, rows) in rows.iter_mut().enumerate() {
                rows.push(core_features(
                    &step,
                    actor.model.processing_ids(),
                    kind == 1,
                ));
            }
            pixels.push(
                nerva_qemu::vision_encoder::grayscale(&self.frames[image], 8, 8)?
                    .into_iter()
                    .map(f64::from)
                    .collect(),
            );
            labels.push(world::visible_effect(&self.frames[image])?);
            self.observe(
                actor,
                3,
                None,
                50_000,
                ("calibration_fork", index, "padding"),
            )?;
            self.observe(
                actor,
                3,
                None,
                10_000,
                ("calibration_fork", index, "padding"),
            )?;
        }
        ensure(balanced(&labels), "unbalanced visual calibration")?;
        Ok(Calibration {
            readouts: [
                Readout::fit(&rows[0], &labels),
                Readout::fit(&rows[1], &labels),
            ],
            visual: [
                chronological(&rows[0], &labels),
                chronological(&rows[1], &labels),
            ],
            pixel: Readout::fit(&pixels, &labels),
        })
    }
    fn action(
        &mut self,
        actor: &mut Actor,
        trial: Trial,
        index: usize,
        stage: &'static str,
        cal: Option<&Calibration>,
    ) -> Result<Prediction> {
        let start = actor.model.next_observation_us();
        self.observe(actor, 3, None, BASE, (stage, index, "baseline"))?;
        let action = actor.model.next_observation_us();
        let mut world = world::DelayedWorld::new(self.reversed)?;
        world.execute(trial.key, action, CUE)?;
        let mut cue: Option<Step> = None;
        let mut times = Vec::new();
        let mut emitted = 0;
        for (part, duration) in [10_000, 10_000, 30_000].into_iter().enumerate() {
            let key = if part == 0 || self.config.copies == 3 {
                Some(trial.key)
            } else {
                None
            };
            let (step, audit) = self.observe(actor, 3, key, duration, (stage, index, "action"))?;
            times.extend(audit.action_copy_arrival_times_us);
            emitted += audit.action_copy_spikes;
            if let Some(total) = &mut cue {
                total.end_us = step.end_us;
                total.action = step.action;
                for (count, new) in total.core_counts.iter_mut().zip(step.core_counts) {
                    *count += new;
                }
                total.core_events.extend(step.core_events);
            } else {
                cue = Some(step);
            }
        }
        let cue = cue.unwrap();
        let forecast = cue.end_us;
        if let Some(file) = self.propagation.as_mut() {
            let key_index = self
                .config
                .base
                .model
                .keyboard
                .keys()
                .iter()
                .position(|k| *k == trial.key)
                .unwrap();
            let source = actor
                .model
                .network()
                .neurons()
                .filter(|n| {
                    n.role() == Some(nerva::core::NeuronRole::Sensory)
                        && n.id().0 >= self.config.base.sensor.channels() as u64
                })
                .nth(key_index)
                .ok_or("missing copy source")?
                .id();
            let direct: Vec<_> = actor
                .model
                .network()
                .outgoing_synapses(source)?
                .filter(|s| actor.model.processing_ids().contains(&s.post()))
                .map(|s| s.post().0)
                .collect();
            let extra = cue
                .core_events
                .iter()
                .filter(|(id, _)| !direct.contains(id))
                .count();
            writeln!(
                file,
                "{},{},{stage},{index},{},{action},{forecast},\"{}\",\"{}\",{}",
                self.logs.arm,
                self.logs.mapping,
                trial.key.qcode(),
                serde_json::to_string(&direct)?,
                serde_json::to_string(&cue.core_events)?,
                extra
            )?;
        }
        let expected: Vec<_> = (0..self.config.copies)
            .flat_map(|pulse| [action + pulse as u64 * 10_000 + 1500; 3])
            .collect();
        ensure(
            times == expected && emitted == self.config.copies,
            "wrong real action-copy arrivals/emissions",
        )?;
        ensure(
            times.iter().all(|&t| t <= forecast)
                && cue.core_events.iter().all(|&(_, t)| t <= forecast),
            "future event in prediction",
        )?;
        // Predictions happen before advancing the world or reading its effect.
        let guesses = cal.map_or([0, 0], |c| {
            [
                c.readouts[0].predict(&core_features(&cue, actor.model.processing_ids(), false)),
                c.readouts[1].predict(&core_features(&cue, actor.model.processing_ids(), true)),
            ]
        });
        let pixel = if let Some(c) = cal {
            let features = nerva_qemu::vision_encoder::grayscale(&self.frames[3], 8, 8)?
                .into_iter()
                .map(f64::from)
                .collect::<Vec<_>>();
            c.pixel.predict(&features)
        } else {
            0
        };
        ensure(
            world.motion_us.is_none(),
            "physical action moved before forecast",
        )?;
        let effect_time = actor.model.next_observation_us();
        world.advance(effect_time)?;
        let actual = world::visible_effect(&world.observe_native()?)?;
        ensure(
            world.motion_us == Some(action + CUE)
                && forecast < effect_time
                && effect_time == action + CUE,
            "real action/motion/image boundary failed",
        )?;
        if stage != "training" {
            ensure(
                actual == trial.shown_motion,
                "probe effect differs from physical mapping",
            )?;
        }
        let image = if trial.shown_motion < 0 { 2 } else { 4 };
        self.observe(actor, image, None, EFFECT, (stage, index, "effect"))?;
        self.observe(
            actor,
            3,
            None,
            TRIAL - BASE - CUE - EFFECT,
            (stage, index, "padding"),
        )?;
        ensure(
            actor.model.next_observation_us() - start == TRIAL,
            "trial duration drift",
        )?;
        writeln!(
            self.logs.details,
            "{},{},{stage},{index},{},{action},{effect_time},{forecast},{effect_time},{},{actual},{},{},{},{},{},{},{},{:016x},{:016x}",
            self.logs.arm,
            self.logs.mapping,
            trial.key.qcode(),
            trial.shown_motion,
            if cal.is_some() {
                guesses[0].to_string()
            } else {
                String::new()
            },
            if cal.is_some() {
                guesses[1].to_string()
            } else {
                String::new()
            },
            times.len(),
            times.first().map(u64::to_string).unwrap_or_default(),
            cue.core_events.len(),
            cue.core_events
                .first()
                .map(|(_, t)| t.to_string())
                .unwrap_or_default(),
            cue.core_events
                .last()
                .map(|(_, t)| t.to_string())
                .unwrap_or_default(),
            world::fingerprint(&self.frames[3]),
            world::fingerprint(&self.frames[image])
        )?;
        writeln!(
            self.copies,
            "{},{},{stage},{index},{},{action},{forecast},{effect_time},{emitted},\"{}\"",
            self.logs.arm,
            self.logs.mapping,
            trial.key.qcode(),
            serde_json::to_string(&times)?
        )?;
        Ok(Prediction {
            guesses,
            effect: actual,
            pixel,
            selected: cue.action.is_some(),
        })
    }
    fn probe(
        &mut self,
        actor: &mut Actor,
        cal: &Calibration,
        seed: u64,
        stage: &'static str,
    ) -> Result<Evaluation> {
        let trials = schedule(seed, self.config.base.test_trials, self.reversed, false);
        self.evaluate(actor, cal, &trials, stage)
    }
    fn evaluate(
        &mut self,
        actor: &mut Actor,
        cal: &Calibration,
        trials: &[Trial],
        stage: &'static str,
    ) -> Result<Evaluation> {
        let mut labels = Vec::new();
        let mut predictions = [Vec::new(), Vec::new()];
        let mut pixels = Vec::new();
        let mut selected = 0;
        for (index, &trial) in trials.iter().enumerate() {
            let result = self.action(actor, trial, index, stage, Some(cal))?;
            labels.push(result.effect);
            pixels.push(result.pixel);
            selected += usize::from(result.selected);
            for (rows, guess) in predictions.iter_mut().zip(result.guesses) {
                rows.push(guess);
            }
        }
        ensure(balanced(&labels), "unbalanced probe labels")?;
        let early = 16.min(labels.len());
        let value = json!({"count":score(&labels,&predictions[0]),"time_bins":score(&labels,&predictions[1]),
            "early_count":score(&labels[..early],&predictions[0][..early]),"early_time_bins":score(&labels[..early],&predictions[1][..early]),
            "labels":labels,"predictions_count":predictions[0],"predictions_time_bins":predictions[1],"pixel_baseline":score(&labels,&pixels),
            "test_count":labels.len(),"action_selection_rate":selected as f64/labels.len() as f64,"all_predictions_before_motion_and_image":true});
        Ok(Evaluation { value })
    }
}

pub fn run(config: &Config, output: &Path, raw: bool, flag: &AtomicBool) -> Result<Value> {
    run_with_options(config, output, raw, flag, &RunOptions::default())
}
pub fn run_with_options(
    config: &Config,
    output: &Path,
    raw: bool,
    flag: &AtomicBool,
    options: &RunOptions,
) -> Result<Value> {
    let frames: Result<Vec<_>> = (0..7)
        .map(|position| Ok(Arc::new(world::image(position)?)))
        .collect();
    let frames = frames?;
    let initial = Model::with_sensor(config.base.model.clone(), config.base.sensor)?;
    let initial_network = initial.network().clone();
    let initial_hash = fingerprint(&initial);
    let (mut gain, mut normalization) = m4::normalization::gain(&initial, &config.base)?;
    if let Some(fixed) = options.reference_gain {
        ensure(
            fixed.is_finite() && fixed > 0.0,
            "invalid fixed reference gain",
        )?;
        let ratio = f64::from(fixed) / f64::from(gain);
        for value in normalization["expected_current_per_core_second"]
            .as_array_mut()
            .unwrap()
        {
            *value = json!(value.as_f64().unwrap() * ratio);
        }
        gain = fixed;
        normalization["mode"] = json!("fixed_reference_gain");
        normalization["fixed_rate_gain"] = json!(fixed);
        normalization["initial_mean_current_per_core_second"] = json!(
            normalization["expected_current_per_core_second"]
                .as_array()
                .unwrap()
                .iter()
                .map(|x| x.as_f64().unwrap())
                .sum::<f64>()
                / initial.processing_ids().len() as f64
        );
    }
    let mut propagation = if options.propagation {
        let mut file = writer(output, ".propagation.csv")?;
        writeln!(
            file,
            "arm,mapping,stage,trial,key,action_us,forecast_us,direct_targets,core_events,extra_core_spikes"
        )?;
        Some(file)
    } else {
        None
    };
    let mut logs = Logs::new(output, raw, true)?;
    let mut copies = writer(output, ".copies.csv")?;
    writeln!(
        copies,
        "arm,mapping,stage,trial,key,action_us,forecast_us,effect_image_us,copy_emissions,arrival_times_us"
    )?;
    let mut branches = writer(output, ".branches.csv")?;
    writeln!(
        branches,
        "arm,mapping,branch,checkpoint_us,branch_end_us,parent_after_us,replayed_observations,exact_replay_state,parent_unchanged"
    )?;
    let mut mappings = BTreeMap::<&str, BTreeMap<&str, Value>>::new();
    for reversed in [false, true] {
        let mapping = if reversed { "reversed" } else { "normal" };
        let train = schedule(
            config.base.seed ^ TRAIN_SEED,
            config.base.train_trials,
            reversed,
            false,
        );
        let scrambled = schedule(
            config.base.seed ^ TRAIN_SEED,
            config.base.train_trials,
            reversed,
            true,
        );
        ensure(
            train.iter().zip(&scrambled).all(|(a, b)| a.key == b.key),
            "scrambled action stream changed",
        )?;
        let mut matched = None;
        let mut results = BTreeMap::new();
        for arm in ARMS {
            logs.arm = arm;
            logs.mapping = mapping;
            let mut actor = Actor::new(config, arm, gain, raw)?;
            ensure(
                actor.model.network() == &initial_network,
                "initial neural networks differ",
            )?;
            let mut session = Session {
                config,
                frames: &frames,
                logs: &mut logs,
                copies: &mut copies,
                flag,
                stats: BTreeMap::new(),
                tape: Vec::new(),
                record: true,
                reversed,
                propagation: &mut propagation,
            };
            let mut contingency = [0usize; 4];
            if arm != "before" {
                for (index, &trial) in (if arm == "scrambled" {
                    &scrambled
                } else {
                    &train
                })
                .iter()
                .enumerate()
                {
                    session.action(&mut actor, trial, index, "training", None)?;
                    contingency[usize::from(trial.key == Key::D) * 2
                        + usize::from(trial.shown_motion > 0)] += 1;
                }
                let inputs: Vec<_> = session
                    .tape
                    .iter()
                    .map(|op| (world::fingerprint(&op.frame), op.key, op.duration))
                    .collect();
                if arm == "aged_frozen" {
                    matched = Some(inputs.clone());
                }
                if arm == "paired" {
                    ensure(
                        matched.as_ref() == Some(&inputs),
                        "paired/frozen actual inputs differ",
                    )?;
                }
                ensure(
                    contingency[0] + contingency[2] == config.base.train_trials / 2
                        && contingency[0] + contingency[1] == config.base.train_trials / 2,
                    "unbalanced training",
                )?;
                if arm == "scrambled" {
                    ensure(
                        contingency == [config.base.train_trials / 4; 4],
                        "scrambled contingency is not independent",
                    )?;
                }
            }
            session.record = false;
            let checkpoint = actor.model.network().clone();
            let checkpoint_time = actor.model.next_observation_us();
            let trained_state = m4::membrane(&actor.model);
            let trained_sensor = m4::sensor_state(&actor.model);
            actor.model.freeze()?;
            ensure(
                actor.model.network() == &checkpoint,
                "freezing reset the checkpoint",
            )?;
            // Parent is borrowed immutably for replay. Calibration cannot touch it.
            let mut calibration_actor = session.replay(&actor, arm, gain, raw)?;
            let calibration = session.calibrate(&mut calibration_actor)?;
            ensure(
                actor.model.network() == &checkpoint
                    && actor.model.next_observation_us() == checkpoint_time,
                "calibration modified parent",
            )?;
            writeln!(
                branches,
                "{arm},{mapping},calibration,{checkpoint_time},{},{checkpoint_time},{},true,true",
                calibration_actor.model.next_observation_us(),
                session.tape.len()
            )?;
            ensure(
                learned_parameters_equal(&checkpoint, calibration_actor.model.network()),
                "calibration changed learned parameters",
            )?;
            // Two independent checkpoint branches share the complete initial
            // runtime state, so action-history effects cannot masquerade as a
            // transient immediately after training. They are not 128 duplicates.
            let mut snapshot_labels = Vec::new();
            let mut snapshot_predictions = [Vec::new(), Vec::new()];
            for (index, key) in [Key::A, Key::D].into_iter().enumerate() {
                let mut branch = session.replay(&actor, arm, gain, raw)?;
                let motion = if (key == Key::D) ^ reversed { 1 } else { -1 };
                let result = session.action(
                    &mut branch,
                    Trial {
                        key,
                        shown_motion: motion,
                    },
                    index,
                    "checkpoint_probe",
                    Some(&calibration),
                )?;
                snapshot_labels.push(result.effect);
                for (rows, guess) in snapshot_predictions.iter_mut().zip(result.guesses) {
                    rows.push(guess);
                }
                ensure(
                    learned_parameters_equal(&checkpoint, branch.model.network()),
                    "checkpoint probe changed learned parameters",
                )?;
                writeln!(
                    branches,
                    "{arm},{mapping},checkpoint_{},{checkpoint_time},{},{checkpoint_time},{},true,true",
                    key.qcode(),
                    branch.model.next_observation_us(),
                    session.tape.len()
                )?;
            }
            ensure(
                actor.model.network() == &checkpoint
                    && actor.model.next_observation_us() == checkpoint_time,
                "checkpoint branch modified parent",
            )?;
            let snapshot = json!({"count":score(&snapshot_labels,&snapshot_predictions[0]),"time_bins":score(&snapshot_labels,&snapshot_predictions[1]),
                "labels":snapshot_labels,"predictions_count":snapshot_predictions[0],"predictions_time_bins":snapshot_predictions[1],"test_count":2});
            let mut rest_snapshot = Value::Null;
            if let Some(rest) = options.checkpoint_rest_us {
                ensure(
                    rest > 0 && rest <= 10_000_000,
                    "invalid checkpoint rest duration",
                )?;
                let mut labels = Vec::new();
                let mut predictions = [Vec::new(), Vec::new()];
                for (index, key) in [Key::A, Key::D].into_iter().enumerate() {
                    let mut branch = session.replay(&actor, arm, gain, raw)?;
                    let mut remaining = rest;
                    while remaining > 0 {
                        let duration = remaining.min(50_000);
                        branch.observe(&frames[3], None, duration)?;
                        remaining -= duration;
                    }
                    let motion = if (key == Key::D) ^ reversed { 1 } else { -1 };
                    let result = session.action(
                        &mut branch,
                        Trial {
                            key,
                            shown_motion: motion,
                        },
                        index,
                        "rest_checkpoint_probe",
                        Some(&calibration),
                    )?;
                    labels.push(result.effect);
                    for (rows, guess) in predictions.iter_mut().zip(result.guesses) {
                        rows.push(guess);
                    }
                    ensure(
                        learned_parameters_equal(&checkpoint, branch.model.network()),
                        "rest checkpoint changed learned parameters",
                    )?;
                    writeln!(
                        branches,
                        "{arm},{mapping},rest_checkpoint_{},{checkpoint_time},{},{checkpoint_time},{},true,true",
                        key.qcode(),
                        branch.model.next_observation_us(),
                        session.tape.len()
                    )?;
                }
                rest_snapshot = json!({"count":score(&labels,&predictions[0]),"time_bins":score(&labels,&predictions[1]),"labels":labels,
                    "predictions_count":predictions[0],"predictions_time_bins":predictions[1],"test_count":2,"rest_us":rest});
            }
            let mut no_copy = serde_json::Map::new();
            if options.propagation {
                for (stage, rest) in [
                    ("checkpoint_probe", 0),
                    (
                        "rest_checkpoint_probe",
                        options.checkpoint_rest_us.unwrap_or(0),
                    ),
                ] {
                    let mut branch = session.replay(&actor, arm, gain, raw)?;
                    let mut remaining = rest;
                    while remaining > 0 {
                        let duration = remaining.min(50_000);
                        branch.observe(&frames[3], None, duration)?;
                        remaining -= duration;
                    }
                    branch.observe(&frames[3], None, BASE)?;
                    let start = branch.model.next_observation_us();
                    let mut events = Vec::new();
                    for duration in [10_000, 10_000, 30_000] {
                        let (step, audit) = branch.observe(&frames[3], None, duration)?;
                        ensure(
                            audit.action_copy_arrivals == 0,
                            "no-copy control received copy",
                        )?;
                        events.extend(step.core_events);
                    }
                    ensure(
                        learned_parameters_equal(&checkpoint, branch.model.network()),
                        "no-copy probe changed learned parameters",
                    )?;
                    no_copy.insert(stage.to_owned(),json!({"start_us":start,"end_us":branch.model.next_observation_us()-1,"core_events":events,
                        "exact_replay_state_before_rest":true,"parent_unchanged":actor.model.next_observation_us()==checkpoint_time}));
                    writeln!(
                        branches,
                        "{arm},{mapping},no_copy_{stage},{checkpoint_time},{},{checkpoint_time},{},true,true",
                        branch.model.next_observation_us(),
                        session.tape.len()
                    )?;
                }
            }
            let weight_resets = if options.weight_resets {
                Some(weight_resets::probe(
                    &session,
                    &actor,
                    arm,
                    gain,
                    raw,
                    &initial_network,
                    &calibration,
                    options
                        .checkpoint_rest_us
                        .ok_or("weight resets require independent rest checkpoint")?,
                    options.weight_reset_trace,
                )?)
            } else {
                None
            };
            let immediate_start = actor.model.next_observation_us();
            let recurrence = if options.recurrence_probes {
                Some(weight_resets::recurrence_probe(
                    &session,
                    &actor,
                    arm,
                    gain,
                    raw,
                    &initial_network,
                    &calibration,
                    options
                        .checkpoint_rest_us
                        .ok_or("recurrence probes require rest checkpoint")?,
                )?)
            } else {
                None
            };
            let crossed = if options.crossed_weight_probes {
                Some(weight_resets::crossed_probe(
                    &session,
                    &actor,
                    arm,
                    gain,
                    raw,
                    &initial_network,
                    &calibration,
                    options
                        .checkpoint_rest_us
                        .ok_or("crossed probes require rest checkpoint")?,
                )?)
            } else {
                None
            };
            ensure(
                immediate_start == checkpoint_time,
                "gap before immediate parent probe",
            )?;
            let immediate = session.probe(
                &mut actor,
                &calibration,
                config.base.seed ^ PROBE_SEED,
                "probe",
            )?;
            for index in 0..40 {
                session.observe(&mut actor, 3, None, 50_000, ("rest", index, "neutral"))?;
            }
            let retained = session.probe(
                &mut actor,
                &calibration,
                config.base.seed ^ REST_SEED,
                "retained_probe",
            )?;
            for index in 0..28 {
                session.observe(
                    &mut actor,
                    index % 7,
                    None,
                    50_000,
                    ("interference", index, "visual"),
                )?;
            }
            let interfered = session.probe(
                &mut actor,
                &calibration,
                config.base.seed ^ INTER_SEED,
                "interference_probe",
            )?;
            ensure(
                learned_parameters_equal(&checkpoint, actor.model.network()),
                "parent frozen parameters changed",
            )?;
            let stages: BTreeMap<_, _> = session
                .stats
                .iter()
                .map(|(stage, stats)| {
                    (
                        *stage,
                        json!({"neural":stats.value(),"sensor":stats.sensor_value()}),
                    )
                })
                .collect();
            results.insert(arm,json!({"balanced_accuracy":immediate.value["count"]["balanced_accuracy"],"visual_decodability":calibration.visual[0]["balanced_accuracy"],
                "visual_time_bins":calibration.visual[1],"checkpoint_probe":snapshot,"probe":immediate.value,"retained_probe":retained.value,"interference_probe":interfered.value,
                "checkpoint_us":checkpoint_time,"immediate_probe_start_us":immediate_start,"calibration_end_on_fork_us":calibration_actor.model.next_observation_us(),
                "parameters_preserved":true,"trained_core_state":trained_state,"trained_sensor_state":trained_sensor,"stage_diagnostics":stages,
                "training_contingency_a_left_a_right_d_left_d_right":contingency,"replayed_observations":session.tape.len()}));
            if options.checkpoint_rest_us.is_some() {
                results.get_mut(arm).unwrap()["rest_checkpoint_probe"] = rest_snapshot;
                results.get_mut(arm).unwrap()["trained_weights"] = json!(
                    checkpoint
                        .synapses()
                        .map(|s| (s.id().0, s.weight().get()))
                        .collect::<Vec<_>>()
                );
                results.get_mut(arm).unwrap()["no_copy_probes"] = Value::Object(no_copy);
            }
            if let Some(probes) = weight_resets {
                results.get_mut(arm).unwrap()["weight_reset_probes"] = probes;
            }
            if let Some(probes) = recurrence {
                results.get_mut(arm).unwrap()["recurrence_probes"] = probes;
            }
            if let Some(probes) = crossed {
                results.get_mut(arm).unwrap()["crossed_probes"] = probes;
            }
        }
        mappings.insert(mapping, results);
    }
    let mut arms = BTreeMap::new();
    for arm in ARMS {
        let normal = &mappings["normal"][arm];
        let reversed = &mappings["reversed"][arm];
        let mut comparisons = BTreeMap::new();
        let mut comparison_stages = vec![
            "checkpoint_probe",
            "probe",
            "retained_probe",
            "interference_probe",
        ];
        if options.checkpoint_rest_us.is_some() {
            comparison_stages.push("rest_checkpoint_probe");
        }
        for stage in comparison_stages {
            let mut features = BTreeMap::new();
            for feature in ["count", "time_bins"] {
                let key = if feature == "count" {
                    "predictions_count"
                } else {
                    "predictions_time_bins"
                };
                let a = normal[stage][key].as_array().unwrap();
                let b = reversed[stage][key].as_array().unwrap();
                ensure(a.len() == b.len(), "mapping probe length differs")?;
                let labels = normal[stage]["labels"].as_array().unwrap();
                let inverse = reversed[stage]["labels"].as_array().unwrap();
                ensure(
                    labels
                        .iter()
                        .zip(inverse)
                        .all(|(a, b)| a.as_i64() == b.as_i64().map(|x| -x)),
                    "actions differ between mapping probes",
                )?;
                let flip = a
                    .iter()
                    .zip(b)
                    .filter(|(a, b)| a.as_i64() == b.as_i64().map(|x| -x))
                    .count();
                let dual = a
                    .iter()
                    .zip(b)
                    .zip(labels.iter().zip(inverse))
                    .filter(|((a, b), (x, y))| a == x && b == y)
                    .count();
                features.insert(feature,json!({"flip_rate":flip as f64/a.len() as f64,"dual_correct_rate":dual as f64/a.len() as f64,"test_count":a.len()}));
            }
            comparisons.insert(stage, features);
        }
        arms.insert(arm,json!({"normal_mapping":normal,"reversed_mapping":reversed,"mapping_comparison":comparisons,
            "balanced_accuracy":(normal["balanced_accuracy"].as_f64().unwrap()+reversed["balanced_accuracy"].as_f64().unwrap())/2.0,
            "visual_decodability":(normal["visual_decodability"].as_f64().unwrap()+reversed["visual_decodability"].as_f64().unwrap())/2.0,"test_count":2*config.base.test_trials}));
    }
    logs.flush()?;
    copies.flush()?;
    branches.flush()?;
    if let Some(file) = &mut propagation {
        file.flush()?;
    }
    Ok(
        json!({"protocol":"NERVA-M6-v1","case_id":config.base.case_id,"seed":config.base.seed,"variant":config.variant,"requested_config":config.requested,
        "initial_network_fingerprint":initial_hash,"normalization":normalization,"arms":arms,
        "checks":{"same_initial_network":true,"matched_experience":true,"no_leakage":true,"frozen_probe":true,"balanced_labels":true,
            "same_actions_across_mappings":true,"calibration_replay_exact":true,"parent_not_calibrated_or_reset":true,"zero_calibration_gap_on_parent":true,"all_copies_before_image":true},
        "protocol_details":{"trial_us":TRIAL,"baseline_us":BASE,"action_effect_delay_us":CUE,"effect_observation_us":EFFECT,
            "copies_per_action":config.copies,"relative_copy_input_us":if config.copies==3{vec![0,10000,20000]}else{vec![0]},
            "core_tau_us":20000,"action_copy_weight":0.9,"core_homeostasis":config.base.model.homeostasis,"visual_homeostasis":matches!(config.variant.as_str(),"A"|"B"),
            "motor_and_copy_homeostasis":false,"calibration_on_replay_fork_only":true,"parent_calibration_gap_us":0,
            "first_forecast_after_checkpoint_us":BASE+CUE-1,"checkpoint_probes_per_mapping":2,"online_probes_per_mapping":config.base.test_trials,
            "early_online_trials":16.min(config.base.test_trials),"rest_us":2000000,"interference_us":1400000,
            "model_input_fields":["current_frame","executed_key"],"calibration_branch_uses_same_checkpoint_including_pending_events":true,
            "limits":["Checkpoint A/D predictions share the same starting state and are two examples per seed, not 128 independent copies","Parent online probing itself advances dynamic state; early16 results are reported separately","First forecast includes 50ms baseline and 50ms action integration",if config.base.model.homeostasis {"Core-homeostasis is held on; no claim that it is optimal"} else {"Exploratory Core-homeostasis-off control; separate from the original fixed-core M6 matrix"},"Reference A also uses the new timing and replay-calibration protocol, so it is not a direct replication of old M4 timing"]}}),
    )
}
