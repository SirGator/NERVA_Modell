//! Passive synapse audit and frozen weight counterfactuals. No learning rule.
use crate::{
    Result,
    model::{
        Model, SensorMode, Step,
        audit::{AuditOptions, InputAudit, learned_parameters_equal},
    },
};
use nerva::{
    core::{Network, NeuronId, Polarity, SynapseId},
    runtime::ObservationEvent,
};
use nerva_qemu::{capture::Frame, keyboard::Key};
use nerva_vm::{
    m4::{
        normalization,
        schedule::{Trial, schedule},
        world,
    },
    m6,
};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::{BufWriter, Write},
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

const TRAIN_SEED: u64 = 0x518cfd42;
const ARMS: [&str; 3] = ["aged_frozen", "paired", "scrambled"];
const CONDITIONS: [&str; 5] = [
    "sham",
    "restore_core",
    "restore_all_plastic",
    "block_core",
    "no_copy",
];
fn ensure(ok: bool, message: &str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(message.to_owned().into())
    }
}
pub fn validate_reference(reference: &Value) -> Result<m6::Config> {
    let config = m6::Config::parse(reference["requested_config"].clone())?;
    ensure(
        reference["protocol"] == "NERVA-M6-v1"
            && config.variant == "D"
            && config.base.sensor.mode == SensorMode::Effect2Ch
            && !config.base.model.homeostasis
            && reference["seed"] == config.base.seed
            && reference["case_id"] == config.base.case_id,
        "M7 requires a matching completed M6 D / effect_2ch32 / Core-homeostasis-off reference",
    )?;
    for name in [
        "same_initial_network",
        "matched_experience",
        "no_leakage",
        "frozen_probe",
        "balanced_labels",
        "same_actions_across_mappings",
        "calibration_replay_exact",
        "parent_not_calibrated_or_reset",
        "zero_calibration_gap_on_parent",
        "all_copies_before_image",
    ] {
        ensure(
            reference["checks"][name] == true,
            "missing/failed mandatory M6 check",
        )?;
    }
    ensure(
        reference["checks"]
            .as_object()
            .is_some_and(|checks| checks.values().all(|v| v == true)),
        "M6 reference checks failed",
    )?;
    for arm in ARMS {
        for mapping in ["normal_mapping", "reversed_mapping"] {
            ensure(
                reference["arms"][arm][mapping]["parameters_preserved"] == true,
                "M6 reference arm missing/not frozen",
            )?;
        }
    }
    Ok(config)
}
fn file(root: &Path, name: &str) -> Result<BufWriter<File>> {
    Ok(BufWriter::new(
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(root.join(name))?,
    ))
}
struct Actor {
    model: Model,
}
impl Actor {
    fn new(config: &m6::Config, arm: &str, gain: f32, raw: bool) -> Result<Self> {
        let settings = if arm == "aged_frozen" {
            config.base.model.clone().frozen()
        } else {
            config.base.model.clone()
        };
        let mut model = Model::with_sensor(settings, config.base.sensor)?;
        model.set_sensory_gain(gain)?;
        model.enable_audit(AuditOptions {
            raw_events: raw,
            input_spikes: false,
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
        let audit = self.model.take_audit().ok_or("missing M7 audit")?;
        ensure(
            audit.homeostasis_updates == 0 && audit.formed == 0 && audit.pruned == 0,
            "unexpected M7 homeostasis/development",
        )?;
        ensure(
            self.model.config().learning || audit.weight_updates == 0,
            "frozen weight update",
        )?;
        Ok((step, audit))
    }
}
#[derive(Clone, Copy)]
struct Context {
    trial: usize,
    phase: &'static str,
    key: Key,
    effect: i16,
}
#[derive(Clone, Copy)]
struct Mark {
    time: u64,
    context: Context,
}
struct Recorder {
    events: BufWriter<File>,
    pairs: BufWriter<File>,
    weights: BufWriter<File>,
    arm: &'static str,
    mapping: &'static str,
    seq: u64,
    last_pre: BTreeMap<SynapseId, Mark>,
    last_post: BTreeMap<NeuronId, Mark>,
    ledger: BTreeMap<SynapseId, f32>,
    updates: usize,
    core_spikes: usize,
    sensory_spikes: usize,
    copy_arrivals: usize,
}
impl Recorder {
    fn new(root: &Path) -> Result<Self> {
        let mut events = file(root, "events.csv")?;
        let mut pairs = file(root, "pairs.csv")?;
        let mut weights = file(root, "weights.csv")?;
        writeln!(
            events,
            "arm,mapping,trial,phase,key,effect,seq,time_us,event,synapse,source,target,amplitude,old_weight,new_weight"
        )?;
        writeln!(
            pairs,
            "arm,mapping,trial,phase,key,effect,seq,synapse,kind,pre_us,post_us,delta_us,pre_trial,pre_phase,pre_key,pre_effect,post_trial,post_phase,post_key,post_effect,old_weight,new_weight,delta_weight,expected_weight"
        )?;
        writeln!(
            weights,
            "arm,mapping,checkpoint,time_us,synapse,pre,post,weight,transmissions"
        )?;
        Ok(Self {
            events,
            pairs,
            weights,
            arm: "",
            mapping: "",
            seq: 0,
            last_pre: BTreeMap::new(),
            last_post: BTreeMap::new(),
            ledger: BTreeMap::new(),
            updates: 0,
            core_spikes: 0,
            sensory_spikes: 0,
            copy_arrivals: 0,
        })
    }
    fn begin(&mut self, arm: &'static str, mapping: &'static str, initial: &Network) {
        self.arm = arm;
        self.mapping = mapping;
        self.seq = 0;
        self.last_pre.clear();
        self.last_post.clear();
        self.ledger = initial
            .synapses()
            .map(|s| (s.id(), s.weight().get()))
            .collect();
        self.updates = 0;
        self.core_spikes = 0;
        self.sensory_spikes = 0;
        self.copy_arrivals = 0;
    }
    fn phase(&mut self, actor: &Actor, audit: InputAudit, context: Context) -> Result<()> {
        self.sensory_spikes += audit.emitted_sensor_spikes;
        self.copy_arrivals += audit.action_copy_arrivals;
        for event in audit.raw_events {
            let (time, kind, id, source, target, amplitude, old, new) = match event {
                ObservationEvent::SynapticArrival {
                    time,
                    synapse_id,
                    source,
                    target,
                    amplitude,
                } => {
                    self.last_pre.insert(
                        synapse_id,
                        Mark {
                            time: time.0,
                            context,
                        },
                    );
                    (
                        time.0,
                        "arrival",
                        Some(synapse_id.0),
                        Some(source.0),
                        Some(target.0),
                        Some(amplitude),
                        None,
                        None,
                    )
                }
                ObservationEvent::SpikeEmitted(spike) => {
                    self.last_post.insert(
                        spike.neuron_id,
                        Mark {
                            time: spike.time.0,
                            context,
                        },
                    );
                    self.core_spikes +=
                        usize::from(actor.model.processing_ids().contains(&spike.neuron_id));
                    (
                        spike.time.0,
                        "spike",
                        None,
                        Some(spike.neuron_id.0),
                        None,
                        None,
                        None,
                        None,
                    )
                }
                ObservationEvent::WeightChanged {
                    time,
                    synapse_id,
                    old_weight,
                    new_weight,
                } => {
                    ensure(
                        self.arm != "aged_frozen",
                        "frozen reference changed weights",
                    )?;
                    let edge = actor
                        .model
                        .network()
                        .synapse(synapse_id)
                        .ok_or("unknown audit synapse")?;
                    ensure(
                        edge.is_plastic()
                            && actor.model.network().neuron(edge.pre()).unwrap().polarity()
                                == Polarity::Excitatory,
                        "STDP updated ineligible edge",
                    )?;
                    ensure(
                        self.ledger[&synapse_id] == old_weight.get(),
                        "weight ledger mismatch",
                    )?;
                    let pre = *self
                        .last_pre
                        .get(&synapse_id)
                        .ok_or("update without presynaptic arrival")?;
                    let post = *self
                        .last_post
                        .get(&edge.post())
                        .ok_or("update without postsynaptic spike")?;
                    let ltp = new_weight > old_weight;
                    ensure(
                        (ltp && post.time == time.0 && pre.time <= post.time)
                            || (!ltp && pre.time == time.0 && post.time <= pre.time),
                        "wrong causal STDP partner",
                    )?;
                    let elapsed = pre.time.abs_diff(post.time);
                    ensure(elapsed <= 100_000, "STDP update outside window")?;
                    let delta =
                        if ltp { 0.0001 } else { -0.00012 } * (-(elapsed as f32) / 20_000.0).exp();
                    let expected = (old_weight.get() + delta).clamp(0.0, 2.0);
                    ensure(
                        expected == new_weight.get(),
                        "observed STDP update disagrees with nearest-pair equation",
                    )?;
                    writeln!(
                        self.pairs,
                        "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
                        self.arm,
                        self.mapping,
                        context.trial,
                        context.phase,
                        context.key.qcode(),
                        context.effect,
                        self.seq,
                        synapse_id.0,
                        if ltp { "ltp" } else { "ltd" },
                        pre.time,
                        post.time,
                        i128::from(post.time) - i128::from(pre.time),
                        pre.context.trial,
                        pre.context.phase,
                        pre.context.key.qcode(),
                        pre.context.effect,
                        post.context.trial,
                        post.context.phase,
                        post.context.key.qcode(),
                        post.context.effect,
                        old_weight.get(),
                        new_weight.get(),
                        f64::from(new_weight.get()) - f64::from(old_weight.get()),
                        expected
                    )?;
                    self.ledger.insert(synapse_id, new_weight.get());
                    self.updates += 1;
                    (
                        time.0,
                        "weight",
                        Some(synapse_id.0),
                        Some(edge.pre().0),
                        Some(edge.post().0),
                        None,
                        Some(old_weight.get()),
                        Some(new_weight.get()),
                    )
                }
                ObservationEvent::ExternalInput {
                    time,
                    target,
                    amplitude,
                } => (
                    time.0,
                    "external",
                    None,
                    None,
                    Some(target.0),
                    Some(amplitude),
                    None,
                    None,
                ),
                ObservationEvent::HomeostasisChanged { .. } => {
                    return Err("unexpected homeostasis event".into());
                }
            };
            let number = |x: Option<u64>| x.map(|v| v.to_string()).unwrap_or_default();
            let float = |x: Option<f32>| x.map(|v| v.to_string()).unwrap_or_default();
            writeln!(
                self.events,
                "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
                self.arm,
                self.mapping,
                context.trial,
                context.phase,
                context.key.qcode(),
                context.effect,
                self.seq,
                time,
                kind,
                number(id),
                number(source),
                number(target),
                float(amplitude),
                float(old),
                float(new)
            )?;
            self.seq += 1;
        }
        for edge in actor.model.network().synapses() {
            ensure(
                self.ledger[&edge.id()] == edge.weight().get(),
                "unlogged weight mutation",
            )?;
        }
        Ok(())
    }
    fn snapshot(&mut self, actor: &Actor, checkpoint: &str) -> Result<()> {
        for edge in actor.model.network().synapses() {
            ensure(
                self.ledger[&edge.id()] == edge.weight().get(),
                "snapshot weight differs from event ledger",
            )?;
            writeln!(
                self.weights,
                "{},{},{},{},{},{},{},{},{}",
                self.arm,
                self.mapping,
                checkpoint,
                actor.model.next_observation_us(),
                edge.id().0,
                edge.pre().0,
                edge.post().0,
                edge.weight().get(),
                edge.transmission_count()
            )?;
        }
        Ok(())
    }
}
fn cancelled(flag: &AtomicBool) -> Result<()> {
    ensure(
        !flag.load(Ordering::Relaxed),
        "M7 interrupted; output remains incomplete",
    )
}
fn train(
    actor: &mut Actor,
    frames: &[Frame],
    trials: &[Trial],
    flag: &AtomicBool,
    mut recorder: Option<&mut Recorder>,
) -> Result<()> {
    for (index, trial) in trials.iter().enumerate() {
        cancelled(flag)?;
        let action = actor.model.next_observation_us() + 50_000;
        let phases = [
            ("baseline", 3, None, 50_000),
            ("action", 3, Some(trial.key), 10_000),
            ("action", 3, Some(trial.key), 10_000),
            ("action", 3, Some(trial.key), 30_000),
            (
                "effect",
                if trial.shown_motion < 0 { 2 } else { 4 },
                None,
                50_000,
            ),
            ("padding", 3, None, 10_000),
        ];
        let mut arrivals: Vec<u64> = Vec::new();
        for (phase, image, key, duration) in phases {
            let (step, audit) = actor.observe(&frames[image], key, duration)?;
            if phase == "action" {
                arrivals.extend(audit.action_copy_arrival_times_us.iter().copied());
                ensure(step.end_us < action + 50_000, "forecast not before image")?;
            }
            if let Some(r) = recorder.as_deref_mut() {
                r.phase(
                    actor,
                    audit,
                    Context {
                        trial: index,
                        phase,
                        key: trial.key,
                        effect: trial.shown_motion,
                    },
                )?;
            }
        }
        ensure(
            arrivals
                == (0..3)
                    .flat_map(|pulse| [action + pulse * 10_000 + 1500; 3])
                    .collect::<Vec<_>>(),
            "wrong M7 copy arrival timing",
        )?;
        if let Some(r) = recorder.as_deref_mut()
            && ((index + 1).is_multiple_of(64) || index + 1 == trials.len())
        {
            r.snapshot(actor, &format!("train_{}", index + 1))?;
        }
    }
    Ok(())
}
fn neutral(actor: &mut Actor, frame: &Frame, windows: usize) -> Result<()> {
    for _ in 0..windows {
        actor.observe(frame, None, 50_000)?;
    }
    Ok(())
}
fn intervention(actor: &mut Actor, initial: &Network, condition: &str) -> Result<usize> {
    let before = actor.model.network().clone();
    let mut edits = Vec::new();
    for edge in initial.synapses() {
        let core = actor.model.processing_ids().contains(&edge.pre())
            && actor.model.processing_ids().contains(&edge.post());
        let value = match condition {
            "restore_core" if core => Some(edge.weight().get()),
            "restore_all_plastic" if edge.is_plastic() => Some(edge.weight().get()),
            "block_core" if core => Some(0.0),
            _ => None,
        };
        if let Some(value) = value
            && before.synapse(edge.id()).unwrap().weight().get() != value
        {
            edits.push((edge.id(), value));
        }
    }
    for &(id, value) in &edits {
        actor.model.diagnostic_synapse_weight(id, value)?;
    }
    ensure(
        before.neurons().eq(actor.model.network().neurons()),
        "weight intervention changed cells",
    )?;
    Ok(edits.len())
}
fn cue(actor: &mut Actor, frame: &Frame, key: Key, copies: bool) -> Result<Value> {
    actor.observe(frame, None, 50_000)?;
    let action = actor.model.next_observation_us();
    let mut counts = vec![0; actor.model.processing_ids().len()];
    let mut events = Vec::new();
    let mut arrivals = 0;
    for duration in [10_000, 10_000, 30_000] {
        let (step, audit) = actor.observe(frame, copies.then_some(key), duration)?;
        for (a, b) in counts.iter_mut().zip(step.core_counts) {
            *a += b;
        }
        events.extend(step.core_events);
        arrivals += audit.action_copy_arrivals;
    }
    ensure(
        arrivals == if copies { 9 } else { 0 },
        "counterfactual copy count",
    )?;
    let potentials: Vec<_> = actor
        .model
        .processing_ids()
        .iter()
        .map(|id| {
            actor
                .model
                .network()
                .neuron(*id)
                .unwrap()
                .membrane_potential()
        })
        .collect();
    Ok(
        json!({"key":key.qcode(),"action_us":action,"forecast_us":actor.model.next_observation_us()-1,
        "effect_not_presented":true,"core_counts":counts,"core_events":events,"core_potentials":potentials,"copy_arrivals":arrivals}),
    )
}
fn hash(network: &Network) -> String {
    format!(
        "{:016x}",
        format!("{network:?}")
            .bytes()
            .fold(0xcbf29ce484222325u64, |h, b| (h ^ u64::from(b))
                .wrapping_mul(0x100000001b3))
    )
}
fn topology(model: &Model) -> Value {
    let neurons:Vec<_>=model.network().neurons().map(|n|json!({"id":n.id().0,"role":format!("{:?}",n.role()),"polarity":format!("{:?}",n.polarity()),"position":[n.position().x,n.position().y,n.position().z]})).collect();
    let edges:Vec<_>=model.network().synapses().map(|s|json!({"id":s.id().0,"pre":s.pre().0,"post":s.post().0,"weight":s.weight().get(),"delay_us":s.delay_us(),"plastic":s.is_plastic(),"enabled":s.is_enabled(),
        "accepted_by_stdp":s.is_plastic() && model.network().neuron(s.pre()).unwrap().polarity()==Polarity::Excitatory})).collect();
    json!({"neurons":neurons,"synapses":edges,"processing_ids":model.processing_ids().iter().map(|id|id.0).collect::<Vec<_>>()})
}
pub fn run(reference: &Value, root: &Path, flag: &AtomicBool) -> Result<Value> {
    let config = validate_reference(reference)?;
    let frames = (0..7).map(world::image).collect::<Result<Vec<_>>>()?;
    let initial_model = Model::with_sensor(config.base.model.clone(), config.base.sensor)?;
    let initial = initial_model.network().clone();
    let (gain, budget) = normalization::gain(&initial_model, &config.base)?;
    ensure(
        hash(&initial) == reference["initial_network_fingerprint"],
        "initial network differs from M6",
    )?;
    ensure(
        budget["mode"] == reference["normalization"]["mode"]
            && reference["normalization"]["fixed_rate_gain"]
                .as_f64()
                .map(|x| x as f32)
                == Some(gain),
        "normalization differs from M6",
    )?;
    let mut recorder = Recorder::new(root)?;
    let mut runs = Vec::new();
    for reversed in [false, true] {
        let mapping = if reversed {
            "reversed_mapping"
        } else {
            "normal_mapping"
        };
        for arm in ARMS {
            cancelled(flag)?;
            let trials = schedule(
                config.base.seed ^ TRAIN_SEED,
                config.base.train_trials,
                reversed,
                arm == "scrambled",
            );
            let mut parent = Actor::new(&config, arm, gain, true)?;
            ensure(
                parent.model.network() == &initial,
                "different initial model",
            )?;
            recorder.begin(arm, mapping, &initial);
            recorder.snapshot(&parent, "initial")?;
            train(&mut parent, &frames, &trials, flag, Some(&mut recorder))?;
            let old = &reference["arms"][arm][mapping];
            for (key, measured) in [
                ("core_spikes", recorder.core_spikes),
                ("sensory_spikes", recorder.sensory_spikes),
                ("action_copy_arrivals", recorder.copy_arrivals),
                ("weight_updates", recorder.updates),
            ] {
                ensure(
                    old["stage_diagnostics"]["training"]["neural"][key] == measured,
                    "training trajectory differs from M6",
                )?;
            }
            let potentials: Vec<_> = parent
                .model
                .processing_ids()
                .iter()
                .map(|id| {
                    f64::from(
                        parent
                            .model
                            .network()
                            .neuron(*id)
                            .unwrap()
                            .membrane_potential(),
                    )
                })
                .collect();
            ensure(
                old["trained_core_state"]["core_potentials"]
                    .as_array()
                    .is_some_and(|expected| {
                        expected.len() == potentials.len()
                            && expected
                                .iter()
                                .zip(&potentials)
                                .all(|(a, b)| a.as_f64().map(|x| x as f32) == Some(*b as f32))
                    }),
                "training-end membrane state differs from M6 at the stored f32 precision",
            )?;
            let checkpoint = parent.model.network().clone();
            parent.model.freeze()?;
            ensure(
                parent.model.network() == &checkpoint,
                "freeze changed network",
            )?;
            recorder.snapshot(&parent, "frozen")?;
            neutral(&mut parent, &frames[3], 40)?;
            recorder.snapshot(&parent, "rest_2s")?;
            ensure(
                learned_parameters_equal(&checkpoint, parent.model.network()),
                "retention changed learned parameters",
            )?;
            let mut probes = Vec::new();
            for key in [Key::A, Key::D] {
                for condition in CONDITIONS {
                    cancelled(flag)?;
                    let mut branch = Actor::new(&config, arm, gain, false)?;
                    train(&mut branch, &frames, &trials, flag, None)?;
                    branch.model.freeze()?;
                    neutral(&mut branch, &frames[3], 40)?;
                    ensure(
                        branch.model.same_replay_state(&parent.model),
                        "counterfactual replay state differs",
                    )?;
                    let changed = intervention(&mut branch, &initial, condition)?;
                    let edited = branch.model.network().clone();
                    let response = cue(&mut branch, &frames[3], key, condition != "no_copy")?;
                    ensure(
                        learned_parameters_equal(&edited, branch.model.network()),
                        "counterfactual resumed learning",
                    )?;
                    probes.push(json!({"condition":condition,"weights_changed":changed,"same_replay_state_before_intervention":true,"response":response}));
                }
            }
            for index in 0..28 {
                parent.observe(&frames[index % 7], None, 50_000)?;
            }
            recorder.snapshot(&parent, "interference_1_4s")?;
            ensure(
                learned_parameters_equal(&checkpoint, parent.model.network()),
                "interference changed frozen parameters",
            )?;
            runs.push(json!({"arm":arm,"mapping":mapping,"training_matches_m6":true,"events":recorder.seq,"weight_updates":recorder.updates,
                "core_spikes":recorder.core_spikes,"sensory_spikes":recorder.sensory_spikes,"copy_arrivals":recorder.copy_arrivals,
                "frozen_parameters_preserved":true,"counterfactuals":probes}));
        }
    }
    recorder.events.flush()?;
    recorder.pairs.flush()?;
    recorder.weights.flush()?;
    let report = json!({"protocol":"NERVA-M7-synapse-audit-v1","seed":config.base.seed,"requested_config":config.requested,
        "initial_network_fingerprint":hash(&initial),"normalization":budget,"initial_topology":topology(&initial_model),"runs":runs,
        "checks":{"m6_recorded_training_diagnostics_reproduced":true,"every_weight_update_matches_nearest_pair_equation":true,"all_weight_changes_logged":true,
            "all_counterfactuals_begin_at_identical_executable_state":true,"parent_frozen_parameters_preserved":true},
        "limits":["Exploratory mechanism audit of existing M6 seeds, not new confirmation of learning",
            "Counterfactuals after 2s neutral rest, one A and one D from separate exact replay branches; no effect image or external readout",
            "Frozen weight retention is by construction, not a demonstration of consolidation or continuing-plasticity memory",
            "Structural development and homeostasis remain off; no new learning rule or task-specific wiring",
            "Weight-restoration changes only selected weights; travelling arrivals retain emission-time amplitudes",
            "Timing labels and outcome classes are external audit metadata; actor receives current frame and executed key only"]});
    let mut out = file(root, "result.json.partial")?;
    out.write_all(&serde_json::to_vec_pretty(&report)?)?;
    out.flush()?;
    drop(out);
    fs::rename(root.join("result.json.partial"), root.join("result.json"))?;
    Ok(report)
}
