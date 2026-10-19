//! Balanced known-action -> delayed-image experiment, with visual-only readouts.
mod readout;
mod scan;
use crate::{
    Result,
    model::{KeyboardLayout, Model, ModelConfig, ModelKind, Step, WINDOW_US},
    retention::learned_parameters_equal,
};
use nerva_qemu::{
    capture::Frame,
    keyboard::Key,
    test_world::{MotionWorld, PixelEnvironment},
};
use readout::{Readout, chronological, core_features, paired, score};
use serde_json::{Value, json};
use std::{
    fs,
    io::Write,
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

#[derive(Clone, Debug)]
pub struct M3Config {
    pub model: ModelConfig,
    pub seeds: Vec<u64>,
    pub train_trials: usize,
    pub probe_trials: usize,
    pub calibration_trials: usize,
    pub phase_us: u64,
    pub retention_windows: usize,
    pub scan_repeats: usize,
    pub jobs: usize,
}
impl Default for M3Config {
    fn default() -> Self {
        Self {
            model: ModelConfig {
                processing_neurons: 32,
                keyboard: KeyboardLayout::Ad,
                ..ModelConfig::default()
            },
            seeds: (0..10).collect(),
            train_trials: 128,
            probe_trials: 64,
            calibration_trials: 32,
            phase_us: 8_000,
            retention_windows: 40,
            scan_repeats: 8,
            jobs: 4,
        }
    }
}
impl M3Config {
    pub fn validate(&self) -> Result<()> {
        self.model.validate()?;
        let mut seeds = self.seeds.clone();
        seeds.sort();
        seeds.dedup();
        if self.model.kind != ModelKind::Recurrent
            || self.model.keyboard != KeyboardLayout::Ad
            || !self.model.motor_timing
            || !self.model.learning
            || !self.model.development
            || !self.model.homeostasis
            || !self.model.feedback
            || self.model.motor_competition
        {
            return Err("M3 requires recurrent A/D with timing decoder and existing learning rules; controls are created internally".into());
        }
        if seeds.is_empty()
            || seeds.len() != self.seeds.len()
            || seeds.len() > 100
            || !(1..=32).contains(&self.jobs)
        {
            return Err("need 1..=100 distinct seeds and 1..=32 jobs".into());
        }
        if [
            self.train_trials,
            self.probe_trials,
            self.calibration_trials,
        ]
        .iter()
        .any(|&n| !(8..=2048).contains(&n) || !n.is_multiple_of(4))
            || !(1_000..=20_000).contains(&self.phase_us)
            || self.retention_windows > 1000
            || !(4..=32).contains(&self.scan_repeats)
            || !self.scan_repeats.is_multiple_of(2)
        {
            return Err("trial counts must be multiples of four in 8..=2048; phase 1000..=20000 us; retention <=1000; scan repeats even in 4..=32".into());
        }
        Ok(())
    }
}

pub use nerva_vm::m4::schedule::{Trial, schedule};

fn stop(flag: &AtomicBool) -> Result<()> {
    if flag.load(Ordering::Relaxed) {
        return Err("M3 interrupted; partial data remain".into());
    }
    Ok(())
}
fn copy(model: &mut Model, key: Option<Key>) -> Result<()> {
    model.record_executed_keys(&key.into_iter().collect::<Vec<_>>())
}
fn picture(label: i16) -> Result<Frame> {
    let mut world = MotionWorld::new(3, false)?;
    world.act(Some(if label < 0 { Key::A } else { Key::D }))?;
    world.observe()
}
fn motion(before: &Frame, after: &Frame) -> i16 {
    let moment = |f: &Frame| {
        f.rgb()
            .as_chunks::<3>()
            .0
            .iter()
            .enumerate()
            .map(|(i, rgb)| (i % f.width()) as i64 * rgb.iter().map(|&x| i64::from(x)).sum::<i64>())
            .sum::<i64>()
    };
    (moment(after) - moment(before)).signum() as i16
}
fn hash(frame: &Frame) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for &byte in frame.rgb() {
        hash = (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

struct Cue {
    world: MotionWorld,
    baseline: Step,
    step: Step,
}
fn begin(model: &mut Model, key: Key, reversed: bool, phase: u64, start: &Frame) -> Result<Cue> {
    copy(model, None)?;
    let baseline = model.step_for(start, WINDOW_US - 2 * phase)?;
    let mut world = MotionWorld::new(3, reversed)?;
    world.act(Some(key))?;
    // The host has executed the key. Vision is deliberately delayed by phase;
    // the Core receives its own action before the resulting image is acquired.
    copy(model, Some(key))?;
    let step = model.step_for(start, phase)?;
    Ok(Cue {
        world,
        baseline,
        step,
    })
}
fn finish_trial(
    model: &mut Model,
    cue: &Cue,
    trial: Trial,
    phase: u64,
    start: &Frame,
) -> Result<(Step, i16)> {
    let actual = cue.world.observe()?;
    let actual_motion = motion(start, &actual);
    let shown = picture(trial.shown_motion)?;
    copy(model, None)?;
    let outcome = model.step_for(&shown, phase)?;
    if cue.step.end_us >= outcome.start_us
        || cue
            .step
            .core_events
            .iter()
            .any(|&(_, time)| time > cue.step.end_us)
    {
        return Err("future sensory events leaked into action-time features".into());
    }
    Ok((outcome, actual_motion))
}
fn frozen_step(step: &Step) -> Result<()> {
    if step.weight_changes > 0
        || step.homeostasis_changes > 0
        || !step.formed.is_empty()
        || !step.pruned.is_empty()
    {
        return Err("plasticity during frozen M3 measurement".into());
    }
    Ok(())
}

struct Log {
    trials: fs::File,
    spikes: fs::File,
    seed: u64,
    dynamics: &'static str,
    mapping: &'static str,
    state: &'static str,
}
impl Log {
    fn spikes(&mut self, stage: &str, index: usize, phase: &str, step: &Step) -> Result<()> {
        for &(id, time) in &step.core_events {
            writeln!(
                self.spikes,
                "{},{},{},{},{stage},{index},{phase},{id},{time},{}",
                self.seed,
                self.dynamics,
                self.mapping,
                self.state,
                time - step.start_us
            )?;
        }
        Ok(())
    }
    fn trial(
        &mut self,
        stage: &str,
        index: usize,
        trial: Trial,
        observed: (&Cue, &Step, i16),
        pred: [Option<i16>; 2],
        start_hash: &str,
    ) -> Result<()> {
        let (cue, outcome, actual) = observed;
        writeln!(
            self.trials,
            "{},{},{},{},{stage},{index},{},{},{actual},{},{},{},{},{},{},{},{},{},{start_hash}",
            self.seed,
            self.dynamics,
            self.mapping,
            self.state,
            trial.key.qcode(),
            trial.shown_motion,
            cue.step.start_us,
            cue.step.end_us,
            outcome.start_us,
            outcome.end_us,
            cue.step.action_copy_arrivals,
            cue.step.core_events.len(),
            outcome.core_events.len(),
            pred[0].map(|x| x.to_string()).unwrap_or_default(),
            pred[1].map(|x| x.to_string()).unwrap_or_default()
        )?;
        self.spikes(stage, index, "cue", &cue.step)?;
        self.spikes(stage, index, "outcome", outcome)
    }
}

struct Calibration {
    readouts: [Readout; 2],
    validation: [Value; 2],
}
fn calibrate(
    model: &mut Model,
    config: &M3Config,
    start: &Frame,
    seed: u64,
    log: &mut Log,
    flag: &AtomicBool,
) -> Result<Calibration> {
    let mut rows = [Vec::new(), Vec::new()];
    let mut labels = Vec::new();
    for (index, trial) in schedule(seed, config.calibration_trials, false, false)
        .into_iter()
        .enumerate()
    {
        stop(flag)?;
        copy(model, None)?;
        let baseline = model.step_for(start, WINDOW_US - config.phase_us)?;
        let step = model.step_for(&picture(trial.shown_motion)?, config.phase_us)?;
        frozen_step(&baseline)?;
        frozen_step(&step)?;
        rows[0].push(core_features(&step, model.processing_ids(), false));
        rows[1].push(core_features(&step, model.processing_ids(), true));
        labels.push(trial.shown_motion);
        log.spikes("visual_calibration", index, "visual", &step)?;
    }
    Ok(Calibration {
        validation: [
            chronological(&rows[0], &labels),
            chronological(&rows[1], &labels),
        ],
        readouts: [
            Readout::fit(&rows[0], &labels),
            Readout::fit(&rows[1], &labels),
        ],
    })
}

struct ProbeSpec<'a> {
    seed: u64,
    reversed: bool,
    stage: &'a str,
}

fn probe(
    model: &mut Model,
    config: &M3Config,
    start: &Frame,
    spec: ProbeSpec<'_>,
    calibration: &Calibration,
    log: &mut Log,
    flag: &AtomicBool,
) -> Result<Value> {
    let ProbeSpec {
        seed,
        reversed,
        stage,
    } = spec;
    let mut labels = Vec::new();
    let mut predicted = [Vec::new(), Vec::new()];
    let mut post = [Vec::new(), Vec::new()];
    let mut cue_rows = [Vec::new(), Vec::new()];
    let mut copied = 0;
    let start_hash = hash(start);
    for (index, trial) in schedule(seed, config.probe_trials, reversed, false)
        .into_iter()
        .enumerate()
    {
        stop(flag)?;
        let cue = begin(model, trial.key, reversed, config.phase_us, start)?;
        frozen_step(&cue.baseline)?;
        frozen_step(&cue.step)?;
        copied += cue.step.action_copy_arrivals;
        // Forecast occurs here, before finish_trial observes/presents the image.
        let rows = [
            core_features(&cue.step, model.processing_ids(), false),
            core_features(&cue.step, model.processing_ids(), true),
        ];
        let guesses = [
            calibration.readouts[0].predict(&rows[0]),
            calibration.readouts[1].predict(&rows[1]),
        ];
        let (outcome, actual) = finish_trial(model, &cue, trial, config.phase_us, start)?;
        frozen_step(&outcome)?;
        if actual != trial.shown_motion {
            return Err("probe image does not match executed action".into());
        }
        for kind in 0..2 {
            predicted[kind].push(guesses[kind]);
            post[kind].push(calibration.readouts[kind].predict(&core_features(
                &outcome,
                model.processing_ids(),
                kind == 1,
            )));
            cue_rows[kind].push(rows[kind].clone());
        }
        labels.push(actual);
        log.trial(
            stage,
            index,
            trial,
            (&cue, &outcome, actual),
            [Some(guesses[0]), Some(guesses[1])],
            &start_hash,
        )?;
    }
    Ok(
        json!({"visual_to_action_transfer_counts":score(&labels,&predicted[0]),"visual_to_action_transfer_timing":score(&labels,&predicted[1]),
        "outcome_visual_decoding_counts":score(&labels,&post[0]),"outcome_visual_decoding_timing":score(&labels,&post[1]),
        "supervised_action_cue_reference_counts":chronological(&cue_rows[0],&labels),"supervised_action_cue_reference_timing":chronological(&cue_rows[1],&labels),
        "action_copy_arrivals_before_image":copied,"all_forecasts_before_effect_image":true}),
    )
}

const FILES: [&str; 3] = ["m3-trials.csv", "m3-spikes.csv", "motor-scan.csv"];
fn run_serial(config: &M3Config, output: &Path, flag: &AtomicBool) -> Result<Value> {
    let mut trial_file = fs::File::create_new(output.join(FILES[0]))?;
    writeln!(
        trial_file,
        "seed,dynamics,mapping,state,stage,trial,key,shown_motion,actual_motion,cue_start_us,forecast_end_us,effect_start_us,effect_end_us,copy_arrivals,cue_core_spikes,outcome_core_spikes,count_prediction,timing_prediction,start_image_fingerprint"
    )?;
    let mut spike_file = fs::File::create_new(output.join(FILES[1]))?;
    writeln!(
        spike_file,
        "seed,dynamics,mapping,state,stage,trial,phase,neuron,time_us,relative_us"
    )?;
    let start = MotionWorld::new(3, false)?.observe()?;
    let mut results = Vec::new();
    for &seed in &config.seeds {
        for dynamics in ["baseline", "reduced_motor_drive"] {
            for reversed in [false, true] {
                eprintln!(
                    "M3: seed {seed}, {dynamics}, {}",
                    if reversed { "reversed" } else { "normal" }
                );
                let base = ModelConfig {
                    seed,
                    motor_weight_scale: config.model.motor_weight_scale
                        * if dynamics == "baseline" { 1.0 } else { 0.5 },
                    ..config.model.clone()
                };
                let initial = Model::from_config(base.clone())?.network().clone();
                for state in ["before_training", "aged_frozen", "paired", "scrambled"] {
                    stop(flag)?;
                    let mut model = Model::from_config(
                        if matches!(state, "before_training" | "aged_frozen") {
                            base.clone().frozen()
                        } else {
                            base.clone()
                        },
                    )?;
                    if model.network() != &initial {
                        return Err("M3 initial networks differ".into());
                    }
                    let mut log = Log {
                        trials: trial_file.try_clone()?,
                        spikes: spike_file.try_clone()?,
                        seed,
                        dynamics,
                        mapping: if reversed { "reversed" } else { "normal" },
                        state,
                    };
                    let mut changes = 0;
                    let mut formed = 0;
                    let mut pruned = 0;
                    let mut contingency = [0usize; 4];
                    let mut train_copy = 0;
                    if state != "before_training" {
                        for (index, trial) in schedule(
                            seed ^ 0xa4b1c3,
                            config.train_trials,
                            reversed,
                            state == "scrambled",
                        )
                        .into_iter()
                        .enumerate()
                        {
                            stop(flag)?;
                            let cue =
                                begin(&mut model, trial.key, reversed, config.phase_us, &start)?;
                            let (outcome, actual) =
                                finish_trial(&mut model, &cue, trial, config.phase_us, &start)?;
                            for step in [&cue.baseline, &cue.step, &outcome] {
                                changes += step.weight_changes;
                                formed += step.formed.len();
                                pruned += step.pruned.len();
                            }
                            train_copy += cue.step.action_copy_arrivals;
                            contingency[usize::from(trial.key == Key::D) * 2
                                + usize::from(trial.shown_motion > 0)] += 1;
                            log.trial(
                                "training",
                                index,
                                trial,
                                (&cue, &outcome, actual),
                                [None, None],
                                &hash(&start),
                            )?;
                        }
                    }
                    let checkpoint = model.network().clone();
                    model.freeze()?;
                    if model.network() != &checkpoint {
                        return Err("freeze changed M3 trained state".into());
                    }
                    let calibration =
                        calibrate(&mut model, config, &start, seed ^ 0x65a0df, &mut log, flag)?;
                    let immediate = probe(
                        &mut model,
                        config,
                        &start,
                        ProbeSpec {
                            seed: seed ^ 0x892b61,
                            reversed,
                            stage: "probe",
                        },
                        &calibration,
                        &mut log,
                        flag,
                    )?;
                    for _ in 0..config.retention_windows {
                        stop(flag)?;
                        copy(&mut model, None)?;
                        frozen_step(&model.step(&start)?)?;
                    }
                    let retained = probe(
                        &mut model,
                        config,
                        &start,
                        ProbeSpec {
                            seed: seed ^ 0xf24c8d,
                            reversed,
                            stage: "retained_probe",
                        },
                        &calibration,
                        &mut log,
                        flag,
                    )?;
                    if !learned_parameters_equal(&checkpoint, model.network()) {
                        return Err("M3 frozen parameters changed".into());
                    }
                    results.push(json!({"seed":seed,"dynamics":dynamics,"mapping":if reversed{"reversed"}else{"normal"},"state":state,
                "same_initial_network":true,"parameters_retained":true,"training_weight_changes":changes,"formed":formed,"pruned":pruned,
                "training_action_copy_arrivals":train_copy,"training_contingency_a_left_a_right_d_left_d_right":contingency,
                "motor_weight_scale":base.motor_weight_scale,"visual_readout_validation_counts":calibration.validation[0],
                "visual_readout_validation_timing":calibration.validation[1],"probe":immediate,"retained_probe":retained}));
                }
            }
        }
    }
    let scan = scan::run(config, output, flag)?;
    let mut report = json!({"experiment":"M3-known-action-before-effect","version":1,"seeds":config.seeds,"processing_neurons":config.model.processing_neurons,
        "train_trials":config.train_trials,"probe_trials":config.probe_trials,"calibration_trials":config.calibration_trials,
        "action_phase_us":config.phase_us,"effect_phase_us":config.phase_us,"trial_us":WINDOW_US,"retention_delay_us":config.retention_windows as u64*WINDOW_US,
        "decoder":"count_then_first_spike","initial_structural_drive":config.model.initial_structural_drive,"pruning_min_below_us":config.model.pruning_min_below_us,
        "readout":"standardized nearest visual-only class centroids; primary forecast never fitted on action-labelled examples",
        "input_boundary":"pixels and already executed A/D copy only; no position, outcome class, reward, readout prediction or right action",
        "results":results,"motor_scan":scan,
        "limits":["Controlled balanced exploration is not an autonomous policy or goal task",
            "Visual-to-action transfer measures outcome-like activation, not a complete causal world model",
            "A supervised action-cue reference can learn A/D lookup outside NERVA; it is not the primary endpoint",
            "Short observation phases retain the 50-ms decoder lookback; M3 ignores proposed keys",
            "The eight-point motor scan is descriptive and never selects an M3 treatment after seeing its scores",
            "A negative simple readout does not exclude other internal representations or learning at other delays"]});
    finish(&mut report, output)?;
    Ok(report)
}

pub fn run_m3(config: &M3Config, output: &Path, flag: &AtomicBool) -> Result<Value> {
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
            let dir = output.join(format!("worker-{index:02}"));
            fs::create_dir(&dir)?;
            let mut part = config.clone();
            part.seeds = seeds.to_vec();
            part.jobs = 1;
            handles.push(scope.spawn(move || {
                Ok::<_, Box<dyn std::error::Error + Send + Sync>>((
                    dir.clone(),
                    run_serial(&part, &dir, flag)?,
                ))
            }));
        }
        handles
            .into_iter()
            .map(|handle| handle.join().map_err(|_| "M3 worker panicked")?)
            .collect()
    });
    let groups = groups?;
    let mut report = groups[0].1.clone();
    report["seeds"] = json!(config.seeds);
    for field in ["results", "motor_scan"] {
        report[field] = json!(
            groups
                .iter()
                .flat_map(|(_, r)| r[field].as_array().unwrap().iter().cloned())
                .collect::<Vec<_>>()
        );
    }
    for name in FILES {
        let mut file = fs::File::create_new(output.join(name))?;
        for (i, (dir, _)) in groups.iter().enumerate() {
            for (line_index, line) in fs::read_to_string(dir.join(name))?.lines().enumerate() {
                if i == 0 || line_index > 0 {
                    writeln!(file, "{line}")?;
                }
            }
        }
    }
    finish(&mut report, output)?;
    Ok(report)
}

fn value(
    report: &Value,
    seed: u64,
    dynamics: &str,
    mapping: &str,
    state: &str,
    stage: &str,
    feature: &str,
) -> f64 {
    report["results"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| {
            r["seed"] == seed
                && r["dynamics"] == dynamics
                && r["mapping"] == mapping
                && r["state"] == state
        })
        .unwrap()[stage][feature]["balanced_accuracy"]
        .as_f64()
        .unwrap()
}
fn finish(report: &mut Value, output: &Path) -> Result<()> {
    let seeds: Vec<_> = report["seeds"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_u64().unwrap())
        .collect();
    let mut evidence = Vec::new();
    for dynamics in ["baseline", "reduced_motor_drive"] {
        for feature in [
            "visual_to_action_transfer_counts",
            "visual_to_action_transfer_timing",
        ] {
            let mut tests = Vec::new();
            let mut success = seeds.len() >= 10;
            for mapping in ["normal", "reversed"] {
                for stage in ["probe", "retained_probe"] {
                    for reference in ["before_training", "aged_frozen", "scrambled"] {
                        let differences: Vec<_> = seeds
                            .iter()
                            .map(|&seed| {
                                value(report, seed, dynamics, mapping, "paired", stage, feature)
                                    - value(
                                        report, seed, dynamics, mapping, reference, stage, feature,
                                    )
                            })
                            .collect();
                        let comparison = paired(&differences);
                        success &= comparison["bootstrap_95_percent_interval"][0]
                            .as_f64()
                            .unwrap()
                            > 0.0;
                        tests.push(json!({"mapping":mapping,"stage":stage,"reference":reference,"paired_difference":comparison}));
                    }
                }
            }
            let validation = if feature.ends_with("counts") {
                "visual_readout_validation_counts"
            } else {
                "visual_readout_validation_timing"
            };
            let visual = report["results"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|r| r["dynamics"] == dynamics && r["state"] == "paired")
                .map(|r| r[validation]["balanced_accuracy"].as_f64().unwrap())
                .sum::<f64>()
                / (2 * seeds.len()) as f64;
            success &= visual >= 0.8;
            evidence.push(json!({"dynamics":dynamics,"features":feature,"visual_validation_mean":visual,"criteria_met":success,"comparisons":tests}));
        }
    }
    report["m3_evidence"] = json!(evidence);
    report["m3_criteria_met"] = json!(evidence.iter().any(|r| r["criteria_met"] == true));
    fs::write(output.join("m3.json"), serde_json::to_vec_pretty(&report)?)?;
    let mut text = format!(
        "# M3: bekannte Aktion vor dem Wirkungsbild\n\n{} Seeds, {} ausgeglichene Trainingstrials pro Modell. M3-Kriterien erfüllt: **{}**.\n\n| Dynamik | Belegung | Merkmale | Vor Training | Gleich alt, eingefroren | Gekoppelt gelernt | Zuordnung im Training zerstört |\n|---|---|---|---:|---:|---:|---:|\n",
        seeds.len(),
        report["train_trials"],
        report["m3_criteria_met"]
    );
    for dynamics in ["baseline", "reduced_motor_drive"] {
        for mapping in ["normal", "reversed"] {
            for feature in [
                "visual_to_action_transfer_counts",
                "visual_to_action_transfer_timing",
            ] {
                let rates: Vec<_> = ["before_training", "aged_frozen", "paired", "scrambled"]
                    .iter()
                    .map(|state| {
                        100.0
                            * seeds
                                .iter()
                                .map(|&seed| {
                                    value(report, seed, dynamics, mapping, state, "probe", feature)
                                })
                                .sum::<f64>()
                            / seeds.len() as f64
                    })
                    .collect();
                text.push_str(&format!(
                    "| {dynamics} | {mapping} | {} | {:.1}% | {:.1}% | {:.1}% | {:.1}% |\n",
                    if feature.ends_with("counts") {
                        "Zählung"
                    } else {
                        "1-ms-Bins bei Standardphase"
                    },
                    rates[0],
                    rates[1],
                    rates[2],
                    rates[3]
                ));
            }
        }
    }
    text.push_str("\nPrimäre Auswerter wurden ausschließlich auf visuellen Wirkungszuständen kalibriert. Die Vorhersage entsteht nach bekannter Aktion und vor dem neuen Bild. Zufallsniveau: 50 %. Eingefrorene Parameter werden erhalten.\n\n[Messwerte](m3.json), [Zeitfolge und Vorhersagen](m3-trials.csv), [Core-Spikes](m3-spikes.csv), [Motor-Parameterscan](motor-scan.csv).\n");
    fs::write(output.join("m3-summary.md"), text)?;
    Ok(())
}
