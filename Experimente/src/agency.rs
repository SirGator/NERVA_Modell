//! Closed-loop, bounded goal task. The actor receives pixels and its own actions;
//! scenario coordinates, success and scoring never enter the neural model.

use crate::{
    Result,
    model::{KeyboardLayout, Model, ModelConfig, ModelKind, Step},
    random::{next_random, shuffle},
    retention::{centroid_score, learned_parameters_equal},
};
use nerva_qemu::{
    capture::Frame,
    keyboard::Key,
    test_world::{EpisodeSpec, MiniWorld, POSITIONS, PixelEnvironment},
};
use serde_json::{Value, json};
use std::{
    fs,
    io::Write,
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

#[derive(Clone, Debug)]
pub struct AgencyConfig {
    pub model: ModelConfig,
    pub seeds: Vec<u64>,
    pub train_episodes: usize,
    pub adapt_episodes: usize,
    pub eval_episodes: usize,
    pub episode_steps: usize,
    pub jobs: usize,
}

impl Default for AgencyConfig {
    fn default() -> Self {
        Self {
            model: ModelConfig {
                keyboard: KeyboardLayout::Ad,
                processing_neurons: 32,
                ..ModelConfig::default()
            },
            seeds: (0..10).collect(),
            train_episodes: 32,
            adapt_episodes: 32,
            eval_episodes: 18,
            episode_steps: 6,
            jobs: 4,
        }
    }
}

impl AgencyConfig {
    pub fn validate(&self) -> Result<()> {
        self.model.validate()?;
        if !(1..=32).contains(&self.jobs) {
            return Err("jobs must be in 1..=32".into());
        }
        if self.model.kind != ModelKind::Recurrent
            || self.model.keyboard != KeyboardLayout::Ad
            || !self.model.learning
            || !self.model.development
            || !self.model.homeostasis
            || !self.model.feedback
        {
            return Err(
                "agency builds its own controls from recurrent A/D with all existing rules enabled"
                    .into(),
            );
        }
        let mut seeds = self.seeds.clone();
        seeds.sort();
        seeds.dedup();
        if seeds.is_empty() || seeds.len() > 100 || seeds.len() != self.seeds.len() {
            return Err("agency requires 1..=100 distinct seeds; the feasibility criterion needs at least ten".into());
        }
        if [self.train_episodes, self.adapt_episodes, self.eval_episodes]
            .iter()
            .any(|&n| !(1..=10_000).contains(&n))
            || !(1..=32).contains(&self.episode_steps)
        {
            return Err("episode counts must be 1..=10000 and step budget 1..=32".into());
        }
        Ok(())
    }
}

const CONDITIONS: [&str; 5] = [
    "development",
    "fixed_topology",
    "weight_only",
    "frozen",
    "random",
];
const PHASES: [&str; 6] = [
    "before",
    "training",
    "after_frozen",
    "reversed_before",
    "adaptation",
    "reversed_after",
];

struct Decision {
    key: Option<Key>,
    neural: Option<Step>,
}

/// This type has no access to environment state, episode specifications or reward.
struct Actor {
    model: Option<Box<Model>>,
    random: u64,
}

impl Actor {
    fn observe(&mut self, frame: &Frame) -> Result<Decision> {
        if let Some(model) = &mut self.model {
            let step = model.step(frame)?;
            if step
                .action
                .is_some_and(|key| !matches!(key, Key::A | Key::D))
            {
                return Err("A/D model proposed an out-of-layout action".into());
            }
            Ok(Decision {
                key: step.action,
                neural: Some(step),
            })
        } else {
            Ok(Decision {
                key: Some(if next_random(&mut self.random).is_multiple_of(2) {
                    Key::A
                } else {
                    Key::D
                }),
                neural: None,
            })
        }
    }

    fn executed(&mut self, key: Option<Key>) -> Result<()> {
        if let Some(model) = &mut self.model {
            model.record_executed_keys(&key.into_iter().collect::<Vec<_>>())?;
        }
        Ok(())
    }
}

#[derive(Default)]
struct Measures {
    successes: usize,
    steps: usize,
    a: usize,
    d: usize,
    idle: usize,
    idle_ties: usize,
    idle_silence: usize,
    blocked: usize,
    toward: usize,
    away: usize,
    productive: usize,
    sensory: usize,
    core: usize,
    motor: usize,
    copied: usize,
    weight_changes: usize,
    homeostasis: usize,
    formed: usize,
    pruned: usize,
    effect_core: Vec<Vec<f64>>,
    effect_action: Vec<Vec<f64>>,
    effects: Vec<Vec<i16>>,
}

impl Measures {
    fn observed(&mut self, decision: &Decision, frozen: bool) -> Result<()> {
        if let Some(step) = &decision.neural {
            self.sensory += step.sensory_spikes;
            self.core += step.core_events.len();
            self.motor += step.motor_spikes;
            self.copied += step.action_copy_arrivals;
            self.weight_changes += step.weight_changes;
            self.homeostasis += step.homeostasis_changes;
            self.formed += step.formed.len();
            self.pruned += step.pruned.len();
            if frozen
                && (step.weight_changes != 0
                    || step.homeostasis_changes != 0
                    || !step.formed.is_empty()
                    || !step.pruned.is_empty())
            {
                return Err("plasticity event during frozen agency evaluation".into());
            }
        }
        Ok(())
    }

    fn report(&self, episodes: usize) -> Value {
        let readout = if self.effects.len() >= 8 {
            let split = self.effects.len() / 2;
            json!({"from_current_core": if self.effect_core[0].is_empty() {json!(null)} else {
                centroid_score(&self.effect_core,&self.effects,split)},
                "from_own_selected_action": centroid_score(&self.effect_action,&self.effects,split),
                "meaning": "External chronological readout of next pixel-motion sign; no labels feed the actor"})
        } else {
            json!(null)
        };
        json!({"episodes": episodes,"successes": self.successes,
            "success_rate": self.successes as f64/episodes as f64,
            "steps": self.steps,"mean_steps": self.steps as f64/episodes as f64,
            "a": self.a,"d": self.d,"idle": self.idle,"blocked": self.blocked,
            "idle_from_ties":self.idle_ties,"idle_from_silence":self.idle_silence,
            "moves_toward_goal": self.toward,"moves_away_from_goal": self.away,
            "visible_moves": self.productive,"sensory_spikes": self.sensory,"core_spikes": self.core,
            "motor_spikes": self.motor,"action_copy_arrivals": self.copied,
            "weight_changes": self.weight_changes,"homeostasis_changes": self.homeostasis,
            "formed": self.formed,"pruned": self.pruned,"effect_readout": readout})
    }
}

/// Test episodes enumerate shuffled start/goal pairs, independent of the actor.
/// Training goals are odd; evaluation goals even. Distance >= 2 prevents free wins.
pub fn scenarios(seed: u64, episodes: usize, training: bool, reversed: bool) -> Vec<EpisodeSpec> {
    let targets: &[u8] = if training { &[1, 3, 5] } else { &[0, 2, 4, 6] };
    let mut pairs = Vec::new();
    for &target in targets {
        for start in 0..POSITIONS {
            if start.abs_diff(target) >= 2 {
                pairs.push((start, target));
            }
        }
    }
    let mut random = seed
        ^ if training {
            0x73d945ef98a3b3a7
        } else {
            0x28fd9e13d8a2d063
        };
    let mut result = Vec::with_capacity(episodes);
    while result.len() < episodes {
        shuffle(&mut pairs, &mut random);
        for &(start, target) in &pairs {
            if result.len() == episodes {
                break;
            }
            result.push(EpisodeSpec {
                start,
                target,
                reversed,
            });
        }
    }
    result
}

fn stop(cancelled: &AtomicBool) -> Result<()> {
    if cancelled.load(Ordering::Relaxed) {
        return Err("agency interrupted; partial traces remain in the output directory".into());
    }
    Ok(())
}

/// Image-derived measurement: the goal and track remain stationary within an
/// episode; only the solid agent contributes to changes in horizontal luminance.
fn pixel_motion(before: &Frame, after: &Frame) -> i16 {
    let moment = |frame: &Frame| -> i64 {
        frame
            .rgb()
            .as_chunks::<3>()
            .0
            .iter()
            .enumerate()
            .map(|(i, rgb)| {
                (i % frame.width()) as i64
                    * (i64::from(rgb[0]) + i64::from(rgb[1]) + i64::from(rgb[2]))
            })
            .sum()
    };
    (moment(after) - moment(before)).signum() as i16
}

fn write_frame(path: &Path, frame: &Frame) -> Result<()> {
    let mut bytes = format!("P6\n{} {}\n255\n", frame.width(), frame.height()).into_bytes();
    bytes.extend_from_slice(frame.rgb());
    fs::write(path, bytes)?;
    Ok(())
}

pub fn run_agency(config: &AgencyConfig, output: &Path, cancelled: &AtomicBool) -> Result<Value> {
    config.validate()?;
    if config.jobs == 1 || config.seeds.len() == 1 {
        return run_serial(config, output, cancelled);
    }
    // Independent seeds share no model, RNG or files. Merge in seed order so
    // serial and parallel execution produce exactly the same measurements.
    let workers = config.jobs.min(config.seeds.len());
    let group_size = config.seeds.len().div_ceil(workers);
    let groups: Result<Vec<_>> = std::thread::scope(|scope| {
        let mut handles = Vec::new();
        for (index, seeds) in config.seeds.chunks(group_size).enumerate() {
            let directory = output.join(format!("worker-{index:02}"));
            fs::create_dir(&directory)?;
            let mut group = config.clone();
            group.seeds = seeds.to_vec();
            group.jobs = 1;
            handles.push(scope.spawn(move || {
                let report = run_serial(&group, &directory, cancelled)?;
                Ok::<_, Box<dyn std::error::Error + Send + Sync>>((directory, report))
            }));
        }
        handles
            .into_iter()
            .map(|handle| handle.join().map_err(|_| "agency worker panicked")?)
            .collect()
    });
    let groups = groups?;
    let mut report = groups[0].1.clone();
    report["seeds"] = json!(config.seeds);
    report["episodes_total"] = json!(
        groups
            .iter()
            .map(|(_, r)| r["episodes_total"].as_u64().unwrap())
            .sum::<u64>()
    );
    report["results"] = json!(
        groups
            .iter()
            .flat_map(|(_, r)| r["results"].as_array().unwrap().iter().cloned())
            .collect::<Vec<_>>()
    );
    report["comparisons"] = comparisons(&report);
    report["criteria"] = criteria(&report);
    for name in [
        "agency-episodes.csv",
        "agency-actions.csv",
        "agency-learning.csv",
    ] {
        let mut combined = fs::File::create_new(output.join(name))?;
        for (index, (directory, _)) in groups.iter().enumerate() {
            let data = fs::read_to_string(directory.join(name))?;
            for (line_index, line) in data.lines().enumerate() {
                if index == 0 || line_index > 0 {
                    writeln!(combined, "{line}")?;
                }
            }
        }
    }
    fs::create_dir(output.join("frames"))?;
    for entry in fs::read_dir(groups[0].0.join("frames"))? {
        let entry = entry?;
        fs::copy(entry.path(), output.join("frames").join(entry.file_name()))?;
    }
    fs::write(
        output.join("agency.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    fs::write(output.join("agency-summary.md"), summary(&report))?;
    Ok(report)
}

fn run_serial(config: &AgencyConfig, output: &Path, cancelled: &AtomicBool) -> Result<Value> {
    config.validate()?;
    let mut episodes_csv = fs::File::create_new(output.join("agency-episodes.csv"))?;
    writeln!(
        episodes_csv,
        "seed,condition,phase,episode,start,target,reversed,steps,success,final_distance"
    )?;
    let mut actions_csv = fs::File::create_new(output.join("agency-actions.csv"))?;
    writeln!(
        actions_csv,
        "seed,condition,phase,episode,step,key,pixel_motion,distance_before,distance_after,core_spikes,motor_a,motor_d,first_a_us,first_d_us"
    )?;
    let mut progress_csv = fs::File::create_new(output.join("agency-learning.csv"))?;
    writeln!(
        progress_csv,
        "seed,condition,phase,episode,weight_changes,homeostasis_changes,formed,pruned,core_synapses"
    )?;
    fs::create_dir(output.join("frames"))?;
    let mut results = Vec::new();
    for &seed in &config.seeds {
        let base = ModelConfig {
            seed,
            ..config.model.clone()
        };
        let initial = Model::from_config(base.clone())?.network().clone();
        for condition in CONDITIONS {
            stop(cancelled)?;
            eprintln!("agency: seed {seed}, {condition}");
            let mut model_config = base.clone();
            match condition {
                "fixed_topology" => model_config.development = false,
                "weight_only" => {
                    model_config.development = false;
                    model_config.homeostasis = false;
                }
                "frozen" => model_config = model_config.frozen(),
                _ => {}
            }
            let model = if condition == "random" {
                None
            } else {
                Some(Box::new(Model::from_config(model_config)?))
            };
            if model
                .as_ref()
                .is_some_and(|model| model.network() != &initial)
            {
                return Err("neural control initial networks differ".into());
            }
            let settings = model.as_ref().map(|model| model.plasticity());
            let mut actor = Actor {
                model,
                random: seed ^ 0xa4bcef17693d28f5,
            };
            let mut phases = serde_json::Map::new();
            for phase in PHASES {
                let training = matches!(phase, "training" | "adaptation");
                let reversed = matches!(phase, "reversed_before" | "adaptation" | "reversed_after");
                let count = match phase {
                    "training" => config.train_episodes,
                    "adaptation" => config.adapt_episodes,
                    _ => config.eval_episodes,
                };
                if let Some(model) = &mut actor.model {
                    if training {
                        model.set_plasticity(settings.expect("neural settings"))?;
                    } else {
                        model.freeze()?;
                    }
                }
                let frozen = !training || condition == "frozen";
                let checkpoint = if frozen {
                    actor.model.as_ref().map(|model| model.network().clone())
                } else {
                    None
                };
                let mut measures = Measures::default();
                for (episode, spec) in scenarios(seed, count, training, reversed)
                    .into_iter()
                    .enumerate()
                {
                    stop(cancelled)?;
                    let mut world = MiniWorld::new(spec)?;
                    actor.executed(None)?; // Environment reset is not an executed agent action.
                    let mut frame = world.observe()?;
                    let save_frame =
                        seed == config.seeds[0] && condition == "development" && episode == 0;
                    if save_frame {
                        write_frame(&output.join(format!("frames/{phase}-start.ppm")), &frame)?;
                    }
                    let previous_updates = (
                        measures.weight_changes,
                        measures.homeostasis,
                        measures.formed,
                        measures.pruned,
                    );
                    let mut decision = actor.observe(&frame)?;
                    measures.observed(&decision, frozen)?;
                    let mut used_steps = 0;
                    for step_index in 0..config.episode_steps {
                        stop(cancelled)?;
                        let key = decision.key;
                        let before = world.assess();
                        world.act(key)?;
                        actor.executed(key)?;
                        let after_frame = world.observe()?;
                        let motion = pixel_motion(&frame, &after_frame);
                        let after = world.assess();
                        used_steps += 1;
                        measures.steps += 1;
                        match key {
                            Some(Key::A) => measures.a += 1,
                            Some(Key::D) => measures.d += 1,
                            _ => measures.idle += 1,
                        };
                        if key.is_none()
                            && let Some(step) = &decision.neural
                        {
                            if step.motor_counts[Key::A as usize] == 0
                                && step.motor_counts[Key::D as usize] == 0
                            {
                                measures.idle_silence += 1;
                            } else {
                                measures.idle_ties += 1;
                            }
                        }
                        measures.blocked += usize::from(key.is_some() && motion == 0);
                        measures.productive += usize::from(motion != 0);
                        measures.toward += usize::from(after.distance < before.distance);
                        measures.away += usize::from(after.distance > before.distance);
                        measures.effect_core.push(
                            decision.neural.as_ref().map_or_else(Vec::new, |step| {
                                step.core_counts.iter().map(|&count| count as f64).collect()
                            }),
                        );
                        measures.effect_action.push(vec![
                            f64::from(key == Some(Key::A)),
                            f64::from(key == Some(Key::D)),
                        ]);
                        measures.effects.push(vec![motion]);
                        let (core, a, d) = decision.neural.as_ref().map_or((0, 0, 0), |step| {
                            (
                                step.core_events.len(),
                                step.motor_counts[Key::A as usize],
                                step.motor_counts[Key::D as usize],
                            )
                        });
                        let first = |key: Key| {
                            decision
                                .neural
                                .as_ref()
                                .and_then(|step| {
                                    step.motor_events
                                        .iter()
                                        .find(|pulse| pulse.channel == key.channel())
                                        .map(|pulse| pulse.at_us)
                                })
                                .map(|t| t.to_string())
                                .unwrap_or_default()
                        };
                        writeln!(
                            actions_csv,
                            "{seed},{condition},{phase},{episode},{step_index},{},{motion},{},{},{core},{a},{d},{},{}",
                            key.map_or("none", |key| key.qcode()),
                            before.distance,
                            after.distance,
                            first(Key::A),
                            first(Key::D)
                        )?;
                        // Even the final action is followed by an observation. The model
                        // sees its consequence/goal image, never an assessment or reward.
                        decision = actor.observe(&after_frame)?;
                        measures.observed(&decision, frozen)?;
                        frame = after_frame;
                        if after.reached {
                            measures.successes += 1;
                            break;
                        }
                    }
                    if save_frame {
                        write_frame(&output.join(format!("frames/{phase}-end.ppm")), &frame)?;
                    }
                    let assessment = world.assess();
                    writeln!(
                        episodes_csv,
                        "{seed},{condition},{phase},{episode},{},{},{reversed},{used_steps},{},{}",
                        spec.start, spec.target, assessment.reached, assessment.distance
                    )?;
                    writeln!(
                        progress_csv,
                        "{seed},{condition},{phase},{episode},{},{},{},{},{}",
                        measures.weight_changes - previous_updates.0,
                        measures.homeostasis - previous_updates.1,
                        measures.formed - previous_updates.2,
                        measures.pruned - previous_updates.3,
                        actor
                            .model
                            .as_ref()
                            .map_or(0, |model| model.core_synapse_count())
                    )?;
                    if let (Some(checkpoint), Some(model)) = (&checkpoint, &actor.model)
                        && !learned_parameters_equal(checkpoint, model.network())
                    {
                        return Err("learned parameters changed in frozen agency evaluation".into());
                    }
                }
                let mut report = measures.report(count);
                report["plasticity_enabled"] = json!(
                    if training { settings } else { None }
                        .is_some_and(|settings| settings.learning
                            || settings.development
                            || settings.homeostasis)
                );
                report["parameters_retained"] = if checkpoint.is_some() {
                    json!(true)
                } else {
                    json!(null)
                };
                report["core_synapses"] = json!(
                    actor
                        .model
                        .as_ref()
                        .map_or(0, |model| model.core_synapse_count())
                );
                phases.insert(phase.to_string(), report);
            }
            results.push(json!({"seed":seed,"condition":condition,"same_initial_neural_network":condition!="random","phases":phases}));
        }
    }
    let mut report = json!({"experiment":"bounded-seven-position-closed-loop-agency", "version":1,
        "seeds":config.seeds,"processing_neurons":config.model.processing_neurons,
        "encoding":format!("{:?}",config.model.encoding),"train_episodes":config.train_episodes,
        "adapt_episodes":config.adapt_episodes,"eval_episodes":config.eval_episodes,
        "episode_step_limit":config.episode_steps,"initial_structural_drive":config.model.initial_structural_drive,
        "pruning_min_below_us":config.model.pruning_min_below_us,
        "motor_decoder":if config.model.motor_timing {"count_then_first_spike"}else{"count"},
        "training_goal_positions":[1,3,5],"evaluation_goal_positions":[0,2,4,6],
        "input_boundary":"Frame and own executed key only; no reward, distance, position, target coordinate or correct action",
        "goal_representation":"visible hollow white rectangle; solid white agent; both survive grayscale",
        "exploration":"only neural action selection; ties/silence idle; no random fallback in neural conditions",
        "episodes_total":config.seeds.len()*CONDITIONS.len()*(config.train_episodes+config.adapt_episodes+4*config.eval_episodes),
        "results":results,
        "limits":["Visible goal alone supplies no motivation or reinforcement rule",
            "External motion readouts measure correlation and do not guide action selection",
            "Closed-loop conditions generate different experience and elapsed neural times",
            "Frozen evaluation preserves learned parameters, while electrical state continues evolving",
            "Resumed homeostasis uses existing NERVA clock handling; projected utility may decay while learning is paused",
            "Only the one-dimensional Rust environment is tested; QEMU transfer and 2D require a successful first result"]});
    report["comparisons"] = comparisons(&report);
    report["criteria"] = criteria(&report);
    fs::write(
        output.join("agency.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    fs::write(output.join("agency-summary.md"), summary(&report))?;
    Ok(report)
}

fn rate(report: &Value, seed: u64, condition: &str, phase: &str) -> f64 {
    report["results"]
        .as_array()
        .expect("results")
        .iter()
        .find(|r| r["seed"] == seed && r["condition"] == condition)
        .expect("condition")["phases"][phase]["success_rate"]
        .as_f64()
        .expect("rate")
}

fn paired(report: &Value, a: (&str, &str), b: (&str, &str)) -> Value {
    let differences: Vec<_> = report["seeds"]
        .as_array()
        .expect("seeds")
        .iter()
        .map(|seed| {
            let seed = seed.as_u64().expect("seed");
            rate(report, seed, a.0, a.1) - rate(report, seed, b.0, b.1)
        })
        .collect();
    let mean = differences.iter().sum::<f64>() / differences.len() as f64;
    let mut random = 0xcb156da48e3b9d21;
    let mut bootstrap = Vec::with_capacity(10_000);
    for _ in 0..10_000 {
        bootstrap.push(
            (0..differences.len())
                .map(|_| differences[(next_random(&mut random) as usize) % differences.len()])
                .sum::<f64>()
                / differences.len() as f64,
        );
    }
    bootstrap.sort_by(f64::total_cmp);
    json!({"a":format!("{}:{}",a.0,a.1),"b":format!("{}:{}",b.0,b.1),
        "paired_seed_mean_difference":mean,"seed_differences":differences,
        "bootstrap_95_percent_interval":[bootstrap[250],bootstrap[9749]],
        "method":"10000 paired seed bootstrap resamples; descriptive small-sample interval"})
}

fn comparisons(report: &Value) -> Value {
    json!({"training_gain":paired(report,("development","after_frozen"),("development","before")),
        "vs_frozen":paired(report,("development","after_frozen"),("frozen","after_frozen")),
        "vs_weight_only":paired(report,("development","after_frozen"),("weight_only","after_frozen")),
        "vs_fixed_topology":paired(report,("development","after_frozen"),("fixed_topology","after_frozen")),
        "vs_random":paired(report,("development","after_frozen"),("random","after_frozen")),
        "reversal_drop":paired(report,("development","after_frozen"),("development","reversed_before")),
        "adaptation_gain":paired(report,("development","reversed_after"),("development","reversed_before"))})
}

fn mean_rate(report: &Value, condition: &str, phase: &str) -> f64 {
    let seeds = report["seeds"].as_array().expect("seeds");
    seeds
        .iter()
        .map(|seed| rate(report, seed.as_u64().unwrap(), condition, phase))
        .sum::<f64>()
        / seeds.len() as f64
}

fn criteria(report: &Value) -> Value {
    let advantages = ["training_gain", "vs_frozen", "vs_random"]
        .iter()
        .all(|name| {
            report["comparisons"][name]["bootstrap_95_percent_interval"][0]
                .as_f64()
                .unwrap()
                > 0.0
        });
    let drop = report["comparisons"]["reversal_drop"]["bootstrap_95_percent_interval"][0]
        .as_f64()
        .unwrap()
        > 0.0;
    let recovery = report["comparisons"]["adaptation_gain"]["bootstrap_95_percent_interval"][0]
        .as_f64()
        .unwrap()
        > 0.0;
    let enough = report["seeds"].as_array().unwrap().len() >= 10;
    let goal = mean_rate(report, "development", "after_frozen") >= 0.8;
    let adapted = mean_rate(report, "development", "reversed_after") >= 0.8;
    let structure = ["vs_weight_only", "vs_fixed_topology"].iter().all(|name| {
        report["comparisons"][name]["bootstrap_95_percent_interval"][0]
            .as_f64()
            .unwrap()
            > 0.0
    });
    json!({"threshold":0.8,"at_least_ten_seeds":enough,
        "held_out_goals_after_freeze_at_least_80_percent":goal,"positive_paired_advantages":advantages,
        "additional_structural_advantage":structure,
        "reversal_causes_drop":drop,"learning_causes_recovery":recovery,
        "adapted_success_at_least_80_percent":adapted,
        "feasibility_demonstrated":enough&&goal&&advantages&&drop&&recovery&&adapted})
}

fn summary(report: &Value) -> String {
    let mut text = format!(
        "# Minimaltest für Handlungsfähigkeit\n\n{} Seeds, {} Episoden insgesamt, maximal {} Aktionen pro Episode.\n\n| Variante | Vorher | Nach Training, eingefroren | Tasten vertauscht, vor Anpassung | Nach Anpassung, eingefroren |\n|---|---:|---:|---:|---:|\n",
        report["seeds"].as_array().unwrap().len(),
        report["episodes_total"],
        report["episode_step_limit"]
    );
    for condition in CONDITIONS {
        text.push_str(&format!(
            "| {condition} | {:.1}% | {:.1}% | {:.1}% | {:.1}% |\n",
            100.0 * mean_rate(report, condition, "before"),
            100.0 * mean_rate(report, condition, "after_frozen"),
            100.0 * mean_rate(report, condition, "reversed_before"),
            100.0 * mean_rate(report, condition, "reversed_after")
        ));
    }
    text.push_str(&format!(
        "\nMachbarkeitskriterien erfüllt: **{}**.\n\n",
        report["criteria"]["feasibility_demonstrated"]
    ));
    text.push_str("Zielkoordinaten, Abstände, Erfolg und korrekte Aktionen gelangen nicht ins Modell. Der sichtbare Zielrahmen ist die einzige Zielrepräsentation. Es gibt keine neue Motivation oder Belohnungsregel und keine Ersatzpolitik bei neuronaler Stille. Der Zufallsagent dient als Kontrolle. Eingefrorene Prüfphasen behalten gelernte Parameter; Membranen und Eingangshistorie entwickeln sich weiter.\n\n[Messwerte und Kriterien](agency.json), [Episoden](agency-episodes.csv), [Aktionen](agency-actions.csv), [Lernverlauf](agency-learning.csv).\n");
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pixel_motion_is_measured_from_the_actual_next_image() {
        let mut world = MiniWorld::new(EpisodeSpec {
            start: 3,
            target: 6,
            reversed: false,
        })
        .unwrap();
        let first = world.observe().unwrap();
        world.act(Some(Key::A)).unwrap();
        let second = world.observe().unwrap();
        assert_eq!(pixel_motion(&first, &second), -1);
        world.act(Some(Key::D)).unwrap();
        let third = world.observe().unwrap();
        assert_eq!(pixel_motion(&second, &third), 1);
        world.act(None).unwrap();
        assert_eq!(pixel_motion(&third, &world.observe().unwrap()), 0);
    }

    #[test]
    fn agency_criteria_allow_equal_weight_learning_and_require_an_advantage_over_no_learning() {
        let mut results = Vec::new();
        for seed in 0..10 {
            for condition in CONDITIONS {
                let mut phases = serde_json::Map::new();
                for phase in PHASES {
                    let value = if matches!(condition, "frozen" | "random") {
                        0.1
                    } else if phase == "before" || phase == "reversed_before" {
                        0.2
                    } else {
                        0.9
                    };
                    phases.insert(phase.into(), json!({"success_rate":value}));
                }
                results.push(json!({"seed":seed,"condition":condition,"phases":phases}));
            }
        }
        let mut report = json!({"seeds":(0..10).collect::<Vec<_>>(),"results":results});
        report["comparisons"] = comparisons(&report);
        let result = criteria(&report);
        assert_eq!(result["feasibility_demonstrated"], true);
        assert_eq!(result["additional_structural_advantage"], false);
        for result in report["results"].as_array_mut().unwrap() {
            if result["condition"] == "frozen" {
                result["phases"]["after_frozen"]["success_rate"] = json!(0.9);
            }
        }
        report["comparisons"] = comparisons(&report);
        assert_eq!(criteria(&report)["feasibility_demonstrated"], false);
    }
}
