//! Separate motor control and action/effect assays, without a goal or reward.
mod metrics;
use crate::{
    Result,
    model::{KeyboardLayout, Model, ModelConfig, ModelKind, Step},
    random::next_random,
    retention::learned_parameters_equal,
};
use metrics::{MotorMeasures, features, motion, pixel_features, score};
use nerva::core::Network;
use nerva_qemu::{
    capture::Frame,
    keyboard::Key,
    motor_decoder::{AdRule, decode_ad},
    test_world::{MotionWorld, PixelEnvironment},
};
use serde_json::{Value, json};
use std::{
    fs,
    io::Write,
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

#[derive(Clone, Debug)]
pub struct SensorimotorConfig {
    pub model: ModelConfig,
    pub seeds: Vec<u64>,
    pub explore_steps: usize,
    pub probe_steps: usize,
    pub jobs: usize,
}
impl Default for SensorimotorConfig {
    fn default() -> Self {
        Self {
            model: ModelConfig {
                processing_neurons: 32,
                keyboard: KeyboardLayout::Ad,
                ..ModelConfig::default()
            },
            seeds: (0..10).collect(),
            explore_steps: 64,
            probe_steps: 64,
            jobs: 4,
        }
    }
}
impl SensorimotorConfig {
    pub fn validate(&self) -> Result<()> {
        self.model.validate()?;
        if self.model.kind != ModelKind::Recurrent
            || self.model.keyboard != KeyboardLayout::Ad
            || self.model.motor_competition
            || !self.model.learning
            || !self.model.development
            || !self.model.homeostasis
            || !self.model.feedback
        {
            return Err("sensorimotor requires the recurrent A/D base with existing learning rules; variants are generated internally".into());
        }
        let mut seeds = self.seeds.clone();
        seeds.sort();
        seeds.dedup();
        if seeds.is_empty()
            || seeds.len() > 100
            || seeds.len() != self.seeds.len()
            || !(1..=32).contains(&self.jobs)
        {
            return Err("need 1..=100 distinct seeds and 1..=32 jobs".into());
        }
        if [self.explore_steps, self.probe_steps]
            .iter()
            .any(|&n| !(16..=2048).contains(&n) || !n.is_multiple_of(8))
        {
            return Err("exploration/probe steps must be multiples of eight in 16..=2048".into());
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Variant {
    Count,
    Timing,
    Inhibition,
    RandomTie,
}
impl Variant {
    pub const ALL: [Self; 4] = [Self::Count, Self::Timing, Self::Inhibition, Self::RandomTie];
    pub fn name(self) -> &'static str {
        match self {
            Self::Count => "count",
            Self::Timing => "count_then_first_spike",
            Self::Inhibition => "inhibitory_competition",
            Self::RandomTie => "random_tie_control",
        }
    }
    fn rule(self) -> AdRule {
        match self {
            Self::Timing => AdRule::CountThenFirstSpike,
            Self::RandomTie => AdRule::RandomTie,
            _ => AdRule::Count,
        }
    }
    fn config(self, base: &ModelConfig) -> ModelConfig {
        ModelConfig {
            motor_competition: self == Self::Inhibition,
            ..base.clone()
        }
    }
    fn choose(self, step: &Step, random: &mut u64) -> Result<Option<Key>> {
        decode_ad(
            &step.motor_events,
            step.start_us,
            step.end_us,
            self.rule(),
            next_random(random),
        )
    }
}

#[derive(Clone, Debug)]
struct Observation {
    frame: Frame,
    previous: Option<Key>,
    previous_motion: Option<i16>,
    next: Option<Key>,
    next_motion: Option<i16>,
    reset: bool,
}
struct Trajectory {
    observations: Vec<Observation>,
    measures: MotorMeasures,
}
fn cancelled(flag: &AtomicBool) -> Result<()> {
    if flag.load(Ordering::Relaxed) {
        return Err("sensorimotor interrupted; partial raw data remain".into());
    }
    Ok(())
}
fn set_copy(model: &mut Model, key: Option<Key>) -> Result<()> {
    model.record_executed_keys(&key.into_iter().collect::<Vec<_>>())
}

fn explore(
    model: &mut Model,
    variant: Variant,
    steps: usize,
    seed: u64,
    held_out: bool,
    flag: &AtomicBool,
) -> Result<Trajectory> {
    let mut observations = Vec::new();
    let mut measures = MotorMeasures::default();
    let mut random = seed
        ^ if held_out {
            0x83e97dfe51a1f203
        } else {
            0x291abc8730fd8c6e
        };
    for _ in 0..steps / 8 {
        let start = (next_random(&mut random) % 7) as u8;
        let mut world = MotionWorld::new(start, false)?;
        let mut frame = world.observe()?;
        let mut previous = None;
        let mut previous_motion = None;
        for index in 0..=8 {
            cancelled(flag)?;
            set_copy(model, previous)?;
            let neural = model.step(&frame)?;
            if index == 8 {
                observations.push(Observation {
                    frame,
                    previous,
                    previous_motion,
                    next: None,
                    next_motion: None,
                    reset: false,
                });
                break;
            }
            let key = variant.choose(&neural, &mut random)?;
            world.act(key)?;
            let after = world.observe()?;
            let effect = motion(&frame, &after);
            measures.observe(&neural, key, Some(effect));
            observations.push(Observation {
                frame,
                previous,
                previous_motion,
                next: key,
                next_motion: Some(effect),
                reset: index == 0,
            });
            frame = after;
            previous = key;
            previous_motion = Some(effect);
        }
    }
    Ok(Trajectory {
        observations,
        measures,
    })
}

fn replay(model: &mut Model, data: &Trajectory, flag: &AtomicBool) -> Result<Vec<Step>> {
    let mut steps = Vec::new();
    for observation in &data.observations {
        cancelled(flag)?;
        set_copy(model, observation.previous)?;
        steps.push(model.step(&observation.frame)?);
    }
    Ok(steps)
}

fn action_copy(data: &Trajectory, index: usize, control: &str) -> Option<Key> {
    let observation = &data.observations[index];
    if observation.reset {
        return None;
    }
    match control {
        "aligned" => observation.previous,
        "swapped" => observation
            .previous
            .map(|key| if key == Key::A { Key::D } else { Key::A }),
        "delayed" => index
            .checked_sub(1)
            .filter(|&i| !data.observations[i].reset)
            .and_then(|i| data.observations[i].previous),
        _ => None,
    }
}

struct ProbeRun {
    steps: Vec<Step>,
    readout: Value,
}
fn probe(
    model: &mut Model,
    data: &Trajectory,
    control: &str,
    flag: &AtomicBool,
) -> Result<ProbeRun> {
    let checkpoint = model.network().clone();
    let mut steps = Vec::new();
    let mut counts = Vec::new();
    let mut timing = Vec::new();
    let mut pixels = Vec::new();
    let mut actions = Vec::new();
    let mut labels = Vec::new();
    let mut past_counts = Vec::new();
    let mut past_labels = Vec::new();
    let mut copied = 0;
    for (index, observation) in data.observations.iter().enumerate() {
        cancelled(flag)?;
        set_copy(model, action_copy(data, index, control))?;
        let step = model.step(&observation.frame)?;
        if step.weight_changes != 0
            || step.homeostasis_changes != 0
            || !step.formed.is_empty()
            || !step.pruned.is_empty()
        {
            return Err("plasticity event during action-effect probe".into());
        }
        copied += step.action_copy_arrivals;
        let (count, time) = features(&step, model.processing_ids());
        if let Some(label) = observation.next_motion {
            counts.push(count.clone());
            timing.push(time);
            pixels.push(pixel_features(&observation.frame)?);
            actions.push(vec![
                f64::from(observation.next == Some(Key::A)),
                f64::from(observation.next == Some(Key::D)),
            ]);
            labels.push(label);
        }
        if let Some(label) = observation.previous_motion {
            past_counts.push(count);
            past_labels.push(label);
        }
        steps.push(step);
    }
    if !learned_parameters_equal(&checkpoint, model.network()) {
        return Err("probe altered frozen parameters".into());
    }
    Ok(ProbeRun {
        steps,
        readout: json!({"next_motion_from_core_counts":score(&counts,&labels),
        "next_motion_from_core_timing_5ms":score(&timing,&labels),"next_motion_from_current_pixels":score(&pixels,&labels),
        "next_motion_from_executed_action":score(&actions,&labels),"previous_motion_from_current_core":score(&past_counts,&past_labels),
        "executed_action_copy_arrivals":copied,"parameters_retained":true}),
    })
}

fn compare(a: &[Step], b: &[Step]) -> Value {
    json!({"frames_with_changed_core_spike_times":a.iter().zip(b).filter(|(a,b)|a.core_events!=b.core_events).count(),
        "core_count_l1":a.iter().zip(b).flat_map(|(a,b)|a.core_counts.iter().zip(&b.core_counts)).map(|(&a,&b)|a.abs_diff(b)).sum::<usize>()})
}

fn compatible_base(base: &Network, extra: &Network) -> bool {
    base.neurons().all(|n| extra.neuron(n.id()) == Some(n))
        && base.synapses().all(|s| extra.synapse(s.id()) == Some(s))
}

const CSV_FILES: [&str; 5] = [
    "motor-actuator.csv",
    "motor-windows.csv",
    "motor-spikes.csv",
    "effect-core.csv",
    "effect-streams.csv",
];
fn run_serial(config: &SensorimotorConfig, output: &Path, flag: &AtomicBool) -> Result<Value> {
    let mut actuator = fs::File::create_new(output.join(CSV_FILES[0]))?;
    writeln!(
        actuator,
        "seed,competition,stimulus,expected_key,decoded_key,a_spikes,d_spikes,pixel_motion,passed"
    )?;
    let mut windows = fs::File::create_new(output.join(CSV_FILES[1]))?;
    writeln!(
        windows,
        "seed,variant,stream,window,a_spikes,d_spikes,chosen_key,first_a_us,first_d_us,positive_arrivals,negative_arrivals,refractory_arrivals"
    )?;
    let mut motor_spikes = fs::File::create_new(output.join(CSV_FILES[2]))?;
    writeln!(
        motor_spikes,
        "seed,variant,stream,window,key,time_us,relative_us"
    )?;
    let mut core = fs::File::create_new(output.join(CSV_FILES[3]))?;
    writeln!(
        core,
        "seed,variant,state,copy_control,frame,neuron,relative_spike_us"
    )?;
    let mut streams = fs::File::create_new(output.join(CSV_FILES[4]))?;
    writeln!(
        streams,
        "seed,variant,phase,frame,previous_key,next_key,previous_motion,next_motion,reset,frame_fingerprint"
    )?;
    fs::create_dir(output.join("frames"))?;
    let black = Frame::new(64, 64, vec![0; 64 * 64 * 3])?;
    let mut results = Vec::new();
    let mut actuator_pass = true;
    for &seed in &config.seeds {
        cancelled(flag)?;
        eprintln!("sensorimotor: seed {seed}");
        let base = ModelConfig {
            seed,
            ..config.model.clone()
        };
        for competition in [false, true] {
            for stimulus in ["silent", "a", "d", "both"] {
                let mut isolated = Model::from_config(ModelConfig {
                    feedback: false,
                    motor_competition: competition,
                    ..base.clone().frozen()
                })?;
                for key in [Key::A, Key::D] {
                    if stimulus == key.qcode() || stimulus == "both" {
                        isolated.diagnostic_motor_input(key, 1000, 1.05)?;
                    }
                }
                let step = isolated.step(&black)?;
                let expected = match stimulus {
                    "a" => Some(Key::A),
                    "d" => Some(Key::D),
                    _ => None,
                };
                let mut world = MotionWorld::new(3, false)?;
                let before = world.observe()?;
                world.act(step.action)?;
                let effect = motion(&before, &world.observe()?);
                let passed = step.action == expected
                    && effect
                        == match expected {
                            Some(Key::A) => -1,
                            Some(Key::D) => 1,
                            _ => 0,
                        }
                    && step.motor_counts[Key::A as usize]
                        == usize::from(stimulus == "a" || stimulus == "both")
                    && step.motor_counts[Key::D as usize]
                        == usize::from(stimulus == "d" || stimulus == "both")
                    && step.core_events.is_empty();
                actuator_pass &= passed;
                writeln!(
                    actuator,
                    "{seed},{competition},{stimulus},{},{},{},{},{effect},{passed}",
                    label(expected),
                    label(step.action),
                    step.motor_counts[Key::A as usize],
                    step.motor_counts[Key::D as usize]
                )?;
            }
        }
        let initial = Model::from_config(base.clone())?.network().clone();
        let mut reference: Option<(Trajectory, Trajectory)> = None;
        let mut reference_steps: Option<Vec<Step>> = None;
        for variant in Variant::ALL {
            let setting = variant.config(&base);
            let mut model = Model::from_config(setting.clone())?;
            let initial_compatible = compatible_base(&initial, model.network());
            if !initial_compatible {
                return Err("competition changed the pre-existing scaffold".into());
            }
            let training = explore(&mut model, variant, config.explore_steps, seed, false, flag)?;
            let trained = model.network().clone();
            model.freeze()?;
            if model.network() != &trained {
                return Err("freeze altered trained network".into());
            }
            let testing = explore(&mut model, variant, config.probe_steps, seed, true, flag)?;
            if !learned_parameters_equal(&trained, model.network()) {
                return Err("own probe changed frozen parameters".into());
            }
            save_stream(
                seed,
                variant,
                "training",
                &training,
                &mut streams,
                output,
                seed == config.seeds[0],
            )?;
            save_stream(
                seed,
                variant,
                "held_out",
                &testing,
                &mut streams,
                output,
                seed == config.seeds[0],
            )?;
            let mut own = Some((training, testing));
            if variant == Variant::Count {
                reference = own.take();
            }
            let (train, test) = if variant == Variant::Count {
                let pair = reference.as_ref().unwrap();
                (&pair.0, &pair.1)
            } else {
                let pair = own.as_ref().unwrap();
                (&pair.0, &pair.1)
            };
            let common = reference.as_ref().expect("count reference");
            let mut bench = Model::from_config(setting.clone())?;
            replay(&mut bench, &common.0, flag)?;
            bench.freeze()?;
            let bench_steps = replay(&mut bench, &common.1, flag)?;
            let exact_motor_stream = if variant == Variant::Count {
                reference_steps = Some(bench_steps.clone());
                true
            } else if variant != Variant::Inhibition {
                bench_steps == *reference_steps.as_ref().unwrap()
            } else {
                false
            };
            if variant != Variant::Inhibition && !exact_motor_stream {
                return Err("decoder-only variants changed neural replay".into());
            }
            let mut random = seed ^ 0x178ead343a92f5ce;
            let mut bench_measures = MotorMeasures::default();
            for (index, step) in bench_steps.iter().enumerate() {
                let key = variant.choose(step, &mut random)?;
                bench_measures.observe(step, key, None);
                write_motor(
                    seed,
                    variant,
                    index,
                    step,
                    key,
                    &mut windows,
                    &mut motor_spikes,
                )?;
            }
            let mut probes = Vec::new();
            for state in ["before_training", "aged_frozen", "trained"] {
                let mut aligned_steps: Option<Vec<Step>> = None;
                for copy_control in ["aligned", "swapped", "delayed", "absent"] {
                    let mut model = Model::from_config(if state == "trained" {
                        setting.clone()
                    } else {
                        setting.clone().frozen()
                    })?;
                    if state != "before_training" {
                        replay(&mut model, train, flag)?;
                    }
                    if state == "trained" && model.network() != &trained {
                        return Err("own training prefix was not reproduced exactly".into());
                    }
                    model.freeze()?;
                    let run = probe(&mut model, test, copy_control, flag)?;
                    for (index, step) in run.steps.iter().enumerate() {
                        for &(neuron, time) in &step.core_events {
                            writeln!(
                                core,
                                "{seed},{},{state},{copy_control},{index},{neuron},{}",
                                variant.name(),
                                time - step.start_us
                            )?;
                        }
                    }
                    let difference = aligned_steps
                        .as_ref()
                        .map(|steps| compare(steps, &run.steps));
                    if copy_control == "aligned" {
                        aligned_steps = Some(run.steps);
                    }
                    probes.push(json!({"state":state,"copy_control":copy_control,"readout":run.readout,"difference_from_aligned":difference}));
                }
            }
            let mut common_report = bench_measures.report();
            common_report["actions_actuated"] = json!(false);
            common_report["visible_moves"] = json!(null);
            results.push(json!({"seed":seed,"variant":variant.name(),"pre_existing_scaffold_unchanged":initial_compatible,
                "decoder_only_common_neural_stream_exact":if variant==Variant::Inhibition {json!(null)}else{json!(exact_motor_stream)},
                "common_input_motor_test":common_report,"own_exploration":train.measures.report(),"own_frozen_probe":test.measures.report(),
                "effect_probes":probes}));
        }
    }
    let mut report = json!({"experiment":"separate-motor-control-and-action-effects","version":1,"seeds":config.seeds,
        "processing_neurons":config.model.processing_neurons,"exploration_actions":config.explore_steps,"probe_actions":config.probe_steps,
        "segment_actions":8,"initial_structural_drive":config.model.initial_structural_drive,"pruning_min_below_us":config.model.pruning_min_below_us,
        "encoding":format!("{:?}",config.model.encoding),"motor_inhibition_weight":config.model.motor_inhibition_weight,
        "isolated_motor_mapping_all_passed":actuator_pass,"isolated_cases":config.seeds.len()*8,"results":results,
        "input_boundary":"pixels and own previously executed key only; no goal, reward, positions, correct actions or motion labels",
        "readout_contract":"external chronological calibration on first half of held-out probe; next pixel motion scored on second half; no future frame, next action or global time in Core features",
        "controls":"identical held-out pixels, changed prior action copies; identical training pixels/actions for trained and aged-frozen states",
        "limits":["A decoder change improves selection, not necessarily action-effect learning",
            "Symmetric lateral inhibition is a tested motif, not a guaranteed winner-take-all mechanism",
            "Motor arrival counts during refractory are observations, not proof that every arrival was discarded",
            "Closed-loop exploration streams differ across decoder variants; comparisons within each variant share the stream",
            "Action-only readouts know the actual next executed action as an external reference; Core readouts do not",
            "Good retrospective motion decoding can come from current visual input alone",
            "No goal-pursuit experiment is rerun; this assay does not establish motivation or causal understanding"]});
    finish(&mut report, output)?;
    Ok(report)
}

fn label(key: Option<Key>) -> &'static str {
    key.map_or("none", |key| key.qcode())
}
fn write_motor(
    seed: u64,
    variant: Variant,
    index: usize,
    step: &Step,
    key: Option<Key>,
    windows: &mut fs::File,
    spikes: &mut fs::File,
) -> Result<()> {
    let stream = "common_replay";
    let first = |key: Key| {
        step.motor_events
            .iter()
            .find(|p| p.channel == key.channel())
            .map(|p| (p.at_us - step.start_us).to_string())
            .unwrap_or_default()
    };
    let a = &step.motor_inputs[Key::A as usize];
    let d = &step.motor_inputs[Key::D as usize];
    writeln!(
        windows,
        "{seed},{},{stream},{index},{},{},{},{},{},{},{},{}",
        variant.name(),
        step.motor_counts[Key::A as usize],
        step.motor_counts[Key::D as usize],
        label(key),
        first(Key::A),
        first(Key::D),
        a.positive_arrivals + d.positive_arrivals,
        a.negative_arrivals + d.negative_arrivals,
        a.arrivals_during_refractory + d.arrivals_during_refractory
    )?;
    for pulse in &step.motor_events {
        writeln!(
            spikes,
            "{seed},{},{stream},{index},{},{},{}",
            variant.name(),
            Key::from_channel(pulse.channel).unwrap().qcode(),
            pulse.at_us,
            pulse.at_us - step.start_us
        )?;
    }
    Ok(())
}
fn save_stream(
    seed: u64,
    variant: Variant,
    phase: &str,
    data: &Trajectory,
    csv: &mut fs::File,
    output: &Path,
    save: bool,
) -> Result<()> {
    for (index, observation) in data.observations.iter().enumerate() {
        let mut hash = 0xcbf29ce484222325u64;
        for &byte in observation.frame.rgb() {
            hash = (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3);
        }
        writeln!(
            csv,
            "{seed},{},{phase},{index},{},{},{},{},{},{hash:016x}",
            variant.name(),
            label(observation.previous),
            label(observation.next),
            observation
                .previous_motion
                .map(|x| x.to_string())
                .unwrap_or_default(),
            observation
                .next_motion
                .map(|x| x.to_string())
                .unwrap_or_default(),
            observation.reset
        )?;
        if save {
            let mut bytes = b"P6\n64 64\n255\n".to_vec();
            bytes.extend_from_slice(observation.frame.rgb());
            fs::write(
                output.join(format!("frames/{}-{phase}-{index:04}.ppm", variant.name())),
                bytes,
            )?;
        }
    }
    Ok(())
}

pub fn run_sensorimotor(
    config: &SensorimotorConfig,
    output: &Path,
    flag: &AtomicBool,
) -> Result<Value> {
    config.validate()?;
    if config.jobs == 1 || config.seeds.len() == 1 {
        return run_serial(config, output, flag);
    }
    let size = config
        .seeds
        .len()
        .div_ceil(config.jobs.min(config.seeds.len()));
    let groups: Result<Vec<_>> = std::thread::scope(|scope| {
        let mut handles = Vec::new();
        for (index, seeds) in config.seeds.chunks(size).enumerate() {
            let directory = output.join(format!("worker-{index:02}"));
            fs::create_dir(&directory)?;
            let mut group = config.clone();
            group.seeds = seeds.to_vec();
            group.jobs = 1;
            handles.push(scope.spawn(move || {
                Ok::<_, Box<dyn std::error::Error + Send + Sync>>((
                    directory.clone(),
                    run_serial(&group, &directory, flag)?,
                ))
            }));
        }
        handles
            .into_iter()
            .map(|handle| handle.join().map_err(|_| "sensorimotor worker panicked")?)
            .collect()
    });
    let groups = groups?;
    let mut report = groups[0].1.clone();
    report["seeds"] = json!(config.seeds);
    report["results"] = json!(
        groups
            .iter()
            .flat_map(|(_, r)| r["results"].as_array().unwrap().iter().cloned())
            .collect::<Vec<_>>()
    );
    report["isolated_motor_mapping_all_passed"] = json!(
        groups
            .iter()
            .all(|(_, r)| r["isolated_motor_mapping_all_passed"] == true)
    );
    report["isolated_cases"] = json!(config.seeds.len() * 8);
    for name in CSV_FILES {
        let mut file = fs::File::create_new(output.join(name))?;
        for (index, (directory, _)) in groups.iter().enumerate() {
            for (line_index, line) in fs::read_to_string(directory.join(name))?
                .lines()
                .enumerate()
            {
                if index == 0 || line_index > 0 {
                    writeln!(file, "{line}")?;
                }
            }
        }
    }
    fs::create_dir(output.join("frames"))?;
    for entry in fs::read_dir(groups[0].0.join("frames"))? {
        let entry = entry?;
        fs::copy(entry.path(), output.join("frames").join(entry.file_name()))?;
    }
    finish(&mut report, output)?;
    Ok(report)
}

fn effect_score(
    report: &Value,
    seed: u64,
    variant: &str,
    state: &str,
    control: &str,
    feature: &str,
) -> f64 {
    report["results"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["seed"] == seed && r["variant"] == variant)
        .unwrap()["effect_probes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["state"] == state && p["copy_control"] == control)
        .unwrap()["readout"][feature]["balanced_accuracy"]
        .as_f64()
        .unwrap()
}
fn paired(values: &[f64]) -> Value {
    let mut rng = 0x725cab67b3e13ab4;
    let mut sampled = Vec::new();
    for _ in 0..10_000 {
        sampled.push(
            (0..values.len())
                .map(|_| values[next_random(&mut rng) as usize % values.len()])
                .sum::<f64>()
                / values.len() as f64,
        );
    }
    sampled.sort_by(f64::total_cmp);
    json!({"mean_difference":values.iter().sum::<f64>()/values.len() as f64,"seed_differences":values,
        "bootstrap_95_percent_interval":[sampled[250],sampled[9749]]})
}
fn finish(report: &mut Value, output: &Path) -> Result<()> {
    let seeds: Vec<_> = report["seeds"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_u64().unwrap())
        .collect();
    let mut comparisons = Vec::new();
    for variant in Variant::ALL {
        for feature in [
            "next_motion_from_core_counts",
            "next_motion_from_core_timing_5ms",
        ] {
            let mut gains = serde_json::Map::new();
            for (name, state, control, reference_feature) in [
                ("vs_aged_frozen", "aged_frozen", "aligned", feature),
                ("vs_before", "before_training", "aligned", feature),
                (
                    "vs_pixels",
                    "trained",
                    "aligned",
                    "next_motion_from_current_pixels",
                ),
                ("vs_absent_copy", "trained", "absent", feature),
                ("vs_swapped_copy", "trained", "swapped", feature),
            ] {
                let differences: Vec<_> = seeds
                    .iter()
                    .map(|&seed| {
                        effect_score(report, seed, variant.name(), "trained", "aligned", feature)
                            - effect_score(
                                report,
                                seed,
                                variant.name(),
                                state,
                                control,
                                reference_feature,
                            )
                    })
                    .collect();
                gains.insert(name.into(), paired(&differences));
            }
            let all_positive = gains
                .values()
                .all(|g| g["bootstrap_95_percent_interval"][0].as_f64().unwrap() > 0.0);
            let coverage = report["results"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|r| r["variant"] == variant.name())
                .all(|r| {
                    r["effect_probes"][8]["readout"][feature]["test_classes"] == 3
                        && r["effect_probes"][8]["readout"][feature]["calibration_classes"] == 3
                });
            comparisons.push(json!({"variant":variant.name(),"features":feature,"paired_balanced_accuracy":gains,
                "all_three_motion_classes_covered":coverage,"controlled_learning_evidence_criteria_met":seeds.len()>=10&&coverage&&all_positive}));
        }
    }
    report["effect_comparisons"] = json!(comparisons);
    report["controlled_effect_learning_evidence"] = json!(
        comparisons
            .iter()
            .any(|c| c["controlled_learning_evidence_criteria_met"] == true)
    );
    fs::write(
        output.join("sensorimotor.json"),
        serde_json::to_vec_pretty(report)?,
    )?;
    let mut text = format!(
        "# Motorische Kontrolle und Wirkungslernen\n\n{} Seeds. Isolierte Motorprüfung: **{}** ({} Fälle).\n\n| Variante | Auswahl bei gleichem Replay | Zeitlich verschiedene Zählgleichstände | Eigene sichtbare Bewegung nach Training | Core-Vorhersage, balanciert |\n|---|---:|---:|---:|---:|\n",
        seeds.len(),
        report["isolated_motor_mapping_all_passed"],
        report["isolated_cases"]
    );
    for variant in Variant::ALL {
        let rows: Vec<_> = report["results"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["variant"] == variant.name())
            .collect();
        let sum = |group: &str, field: &str| {
            rows.iter()
                .map(|r| r[group][field].as_u64().unwrap())
                .sum::<u64>()
        };
        let window = sum("common_input_motor_test", "windows");
        let selected = sum("common_input_motor_test", "selected_actions");
        let rate = seeds
            .iter()
            .map(|&seed| {
                effect_score(
                    report,
                    seed,
                    variant.name(),
                    "trained",
                    "aligned",
                    "next_motion_from_core_counts",
                )
            })
            .sum::<f64>()
            / seeds.len() as f64;
        text.push_str(&format!(
            "| {} | {:.1}% | {}/{} | {}/{} | {:.1}% |\n",
            variant.name(),
            100.0 * selected as f64 / window as f64,
            sum(
                "common_input_motor_test",
                "ties_with_different_spike_trains"
            ),
            sum("common_input_motor_test", "count_ties"),
            sum("own_frozen_probe", "visible_moves"),
            sum("own_frozen_probe", "windows"),
            100.0 * rate
        ));
    }
    text.push_str(&format!("\nKontrollierte Lernkriterien erfüllt: **{}**. Die externe Vorhersage ist kein Nachweis von Motivation oder kausalem Verständnis. Zufälliger Tie-Break ist ausschließlich eine Kontrollvariante.\n\n[Messwerte](sensorimotor.json), [Motorfenster](motor-windows.csv), [Spikezeiten](motor-spikes.csv), [Aktuatorprüfung](motor-actuator.csv), [Core-Spikes](effect-core.csv), [Bild-/Aktionsfolgen](effect-streams.csv).\n",report["controlled_effect_learning_evidence"]));
    fs::write(output.join("sensorimotor-summary.md"), text)?;
    Ok(())
}
