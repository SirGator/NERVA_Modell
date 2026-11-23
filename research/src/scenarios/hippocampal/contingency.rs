//! Test whether learned recurrent weights carry a counterbalanced visual association.
use super::{
    Config, HippocampalScenario,
    association::AssociationScenario,
    causal::verify_only_weights,
    fail,
    mld02::{atlas::get_weights, interaction::checkpoint},
    multilevel::tape::Tape,
};
use crate::{
    Result, error,
    scenario::{Intervention, Scenario, TrialMetrics},
};
use nerva::core::SynapseId;
use nerva::runtime::ObservationEvent;
use nerva_qemu::capture::Frame;
use nerva_vm::model::{
    Model, PlasticitySettings, Step, WINDOW_US,
    audit::{AuditOptions, InputAudit, learned_parameters_equal},
};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

pub struct ContingencyScenario;
pub mod capacity;
mod chemical;
mod electrical;
pub mod live_rate;
pub use chemical::replay_chemical;
pub use electrical::replay_electrical;
const PROTOCOL: &str = "NERVA-Visual-Contingency/1";
const CUES: [&str; 2] = ["cue0", "cue1"];
const FRAGMENTS: [&str; 4] = ["cue0a", "cue0b", "cue1a", "cue1b"];
const OUTCOMES: [&str; 2] = ["outcome0", "outcome1"];
const MAPPINGS: [&str; 3] = ["forward", "reverse", "balanced"];
const METRICS: [&str; 6] = [
    "original_core",
    "fragments_core",
    "original_all",
    "fragments_all",
    "templates_available",
    "core_spike_effects",
];
const AUDIT: AuditOptions = AuditOptions {
    raw_events: true,
    input_spikes: true,
};
type Spikes = Vec<(u64, u64)>;
type Weights = Vec<(SynapseId, f32)>;
type Responses = BTreeMap<&'static str, Response>;

fn stimuli() -> BTreeMap<&'static str, Vec<usize>> {
    let line = |x, y: usize| (0..4).map(move |dx| 8 * y + x + dx).collect::<Vec<_>>();
    let combine = |a: Vec<_>, b: Vec<_>| a.into_iter().chain(b).collect::<Vec<_>>();
    let (a, b, c, d) = (line(0, 0), line(0, 1), line(4, 0), line(4, 1));
    BTreeMap::from([
        ("blank", vec![]),
        (CUES[0], combine(a.clone(), b.clone())),
        (CUES[1], combine(c.clone(), d.clone())),
        (FRAGMENTS[0], a),
        (FRAGMENTS[1], b),
        (FRAGMENTS[2], c),
        (FRAGMENTS[3], d),
        (OUTCOMES[0], combine(line(0, 4), line(0, 7))),
        (OUTCOMES[1], combine(line(4, 4), line(4, 5))),
    ])
}
fn frames(patterns: &BTreeMap<&'static str, Vec<usize>>) -> Result<BTreeMap<&'static str, Frame>> {
    patterns
        .iter()
        .map(|(&name, pixels)| {
            let mut rgb = vec![0; 64 * 3];
            for &pixel in pixels {
                rgb[3 * pixel..3 * pixel + 3].fill(255);
            }
            Ok((name, Frame::new(8, 8, rgb).map_err(fail)?))
        })
        .collect()
}
fn schedule(c: &Config, seed: u64, mapping: &str, gap: usize) -> Vec<&'static str> {
    let mut frames = Vec::with_capacity(c.training_repetitions * 10);
    for repetition in 0..c.training_repetitions {
        let first = ((seed & 1) as usize + repetition) % 2;
        let reverse = mapping == "reverse" || (mapping == "balanced" && repetition % 4 >= 2);
        for cue in [first, 1 - first] {
            let mut trial = ["blank"; 5];
            trial[0] = CUES[cue];
            trial[1 + gap] = OUTCOMES[cue ^ usize::from(reverse)];
            frames.extend(trial);
        }
    }
    frames
}

fn observe(
    model: &mut Model,
    frame: &Frame,
    homeostasis: bool,
) -> Result<(Step, Spikes, InputAudit)> {
    let step = model.step(frame).map_err(fail)?;
    let audit = model
        .take_audit()
        .ok_or_else(|| error("missing contingency audit"))?;
    if audit.raw_events.len() > 200_000
        || !step.formed.is_empty()
        || !step.pruned.is_empty()
        || (!homeostasis && step.homeostasis_changes != 0)
    {
        return Err(error(
            "contingency observation exceeded its budget or changed maintenance state",
        ));
    }
    let root: Vec<_> = audit
        .input_spikes
        .iter()
        .map(|s| (s.channel.0, s.at.0))
        .collect();
    if root.len() != step.sensory_spikes {
        return Err(error("contingency input spike counts disagree"));
    }
    Ok((step, root, audit))
}

fn train(
    model: &mut Model,
    order: &[&str],
    frames: &BTreeMap<&str, Frame>,
    tape: Option<&mut Tape>,
) -> Result<Value> {
    train_with_homeostasis(model, order, frames, tape, false)
}
fn train_with_homeostasis(
    model: &mut Model,
    order: &[&str],
    frames: &BTreeMap<&str, Frame>,
    mut tape: Option<&mut Tape>,
    homeostasis: bool,
) -> Result<Value> {
    if let Some(tape) = &mut tape {
        tape.initialize_from_checkpoint(model);
        model.enable_learning_replay_audit(100_000).map_err(fail)?;
    }
    model
        .set_plasticity(PlasticitySettings {
            learning: true,
            development: false,
            homeostasis,
        })
        .map_err(fail)?;
    model.enable_audit(AuditOptions {
        raw_events: tape.is_some(),
        input_spikes: true,
    });
    let mut rows = Vec::with_capacity(order.len());
    for &stimulus in order {
        let (step, root_spikes, audit) = observe(model, &frames[stimulus], homeostasis)?;
        if let Some(tape) = &mut tape {
            tape.add(&audit, &model.take_learning_audit().map_err(fail)?)?;
        }
        rows.push(
            json!({"stimulus":stimulus,"start_us":step.start_us,"root_spikes":root_spikes,
            "core_spikes":step.core_events,"weight_changes":step.weight_changes}),
        );
    }
    if tape.is_some() {
        let pending = model.disable_learning_audit().map_err(fail)?;
        if !pending.events.is_empty()
            || !pending.boundaries.is_empty()
            || !pending.delayed.is_empty()
        {
            return Err(error("undrained contingency training audit"));
        }
    }
    model.freeze().map_err(fail)?;
    Ok(json!(rows))
}

struct Response {
    start_us: u64,
    rest_core_spikes: usize,
    root_spikes: Spikes,
    core_spikes: Spikes,
    arrived_edges: BTreeSet<u64>,
}
impl Response {
    fn json(&self) -> Value {
        json!({"start_us":self.start_us,"rest_core_spikes":self.rest_core_spikes,
            "root_spikes":self.root_spikes,"core_spikes":self.core_spikes,
            "arrived_edges":self.arrived_edges})
    }
}
fn probe(
    model: &mut Model,
    c: &Config,
    stimulus: &Frame,
    blank: &Frame,
    mut trace: Option<&mut electrical::Trace<'_>>,
) -> Result<Response> {
    let before = model.network().clone();
    let mut remaining = c.rest_us;
    let mut rest_core_spikes = 0;
    while remaining != 0 {
        let duration = remaining.min(WINDOW_US);
        let step = model.step_for(blank, duration).map_err(fail)?;
        rest_core_spikes += step.core_events.len();
        remaining -= duration;
    }
    let mut response = Response {
        start_us: model.next_observation_us(),
        rest_core_spikes,
        root_spikes: Vec::new(),
        core_spikes: Vec::new(),
        arrived_edges: BTreeSet::new(),
    };
    model.enable_audit(AUDIT);
    model.set_dynamics_auditing(trace.is_some());
    for frame in [stimulus, blank, blank, blank] {
        let (step, root, audit) = observe(model, frame, false)?;
        response.root_spikes.extend(root);
        response.core_spikes.extend(step.core_events);
        response
            .arrived_edges
            .extend(audit.raw_events.iter().filter_map(|event| match event {
                ObservationEvent::SynapticArrival {
                    synapse_id,
                    amplitude,
                    ..
                } if *amplitude != 0. => Some(synapse_id.0),
                _ => None,
            }));
        if let Some(trace) = &mut trace {
            trace.record(&audit.raw_events);
        }
    }
    model.set_dynamics_auditing(false);
    if !learned_parameters_equal(&before, model.network()) {
        return Err(error("contingency probe changed learned parameters"));
    }
    Ok(response)
}

fn templates(reset: &Responses) -> [BTreeSet<u64>; 2] {
    let active = |name| reset[name].core_spikes.iter().map(|&(id, _)| id);
    let mut excluded = BTreeSet::new();
    for cue in CUES.into_iter().chain(FRAGMENTS).chain(["blank"]) {
        excluded.extend(active(cue));
    }
    OUTCOMES.map(|name| {
        let other = if name == OUTCOMES[0] {
            OUTCOMES[1]
        } else {
            OUTCOMES[0]
        };
        let other: BTreeSet<_> = active(other).collect();
        active(name)
            .filter(|id| !excluded.contains(id) && !other.contains(id))
            .collect()
    })
}
type Counts = BTreeMap<&'static str, [usize; 2]>;
fn counts(rows: &Responses, targets: &[BTreeSet<u64>; 2]) -> Counts {
    rows.iter()
        .map(|(&name, row)| {
            (
                name,
                std::array::from_fn(|i| {
                    row.core_spikes
                        .iter()
                        .filter(|(id, _)| targets[i].contains(id))
                        .count()
                }),
            )
        })
        .collect()
}
fn gate(
    probes: &BTreeMap<String, Responses>,
    counts: &BTreeMap<String, Counts>,
    suffix: &str,
    cues: &[&str],
) -> bool {
    let reset = &probes["reset"];
    let forward = format!("forward_{suffix}");
    let reverse = format!("reverse_{suffix}");
    let balanced = format!("balanced_{suffix}");
    [(&forward, &reverse), (&reverse, &forward)]
        .into_iter()
        .enumerate()
        .all(|(flip, (arm, opposite))| {
            probes[arm]["blank"].core_spikes.len() <= reset["blank"].core_spikes.len()
                && cues.iter().all(|&cue| {
                    let correct = usize::from(cue.starts_with("cue1")) ^ flip;
                    let measured = counts[arm][cue][correct];
                    probes[arm][cue].rest_core_spikes <= reset[cue].rest_core_spikes
                        && measured > counts[arm][cue][1 - correct]
                        && ["reset", balanced.as_str(), opposite]
                            .into_iter()
                            .all(|control| measured > counts[control][cue][correct])
                })
        })
}

fn serialized(weights: &Weights) -> Vec<(u64, f32)> {
    weights.iter().map(|&(id, w)| (id.0, w)).collect()
}
fn run(c: &Config, seed: u64, gap: usize) -> Result<TrialMetrics> {
    let patterns = stimuli();
    let frames = frames(&patterns)?;
    let (reference, development) = checkpoint(c, seed)?;
    let initial = get_weights(&reference);
    let core_ids: BTreeSet<_> = reference.processing_ids().iter().map(|id| id.0).collect();
    let edges: Vec<_> = reference
        .network()
        .synapses()
        .map(|s| (s.id().0, s.pre().0, s.post().0, s.weight().get()))
        .collect();
    let internal = |pre, post| core_ids.contains(&pre) && core_ids.contains(&post);
    let mut training = BTreeMap::new();
    let mut arms = BTreeMap::from([("reset".into(), initial.clone())]);
    for mapping in MAPPINGS {
        let (mut model, _) = checkpoint(c, seed)?;
        if !reference.same_replay_state(&model) {
            return Err(error("contingency training checkpoints differ"));
        }
        let rows = train(&mut model, &schedule(c, seed, mapping, gap), &frames, None)?;
        let learned = get_weights(&model);
        if !initial.iter().map(|x| x.0).eq(learned.iter().map(|x| x.0)) {
            return Err(error("contingency training topology differs"));
        }
        let core: Weights = edges
            .iter()
            .zip(&learned)
            .map(|(&(_, pre, post, original), &(id, w))| {
                (id, if internal(pre, post) { w } else { original })
            })
            .collect();
        training.insert(
            mapping,
            json!({"frames":rows,"weights":serialized(&learned)}),
        );
        arms.insert(format!("{mapping}_all"), learned);
        arms.insert(format!("{mapping}_core"), core);
    }

    let mut probes = BTreeMap::new();
    for (arm, weights) in &arms {
        let mut responses = BTreeMap::new();
        for &name in frames.keys() {
            let (mut branch, _) = checkpoint(c, seed)?;
            verify_only_weights(&reference, &mut branch, weights)?;
            if get_weights(&branch) != *weights {
                return Err(error("contingency probe weights differ from intervention"));
            }
            responses.insert(
                name,
                probe(&mut branch, c, &frames[name], &frames["blank"], None)?,
            );
        }
        probes.insert(arm.clone(), responses);
    }
    let reset = &probes["reset"];
    for arm in probes.values() {
        if arm.iter().any(|(&name, p)| {
            p.start_us != reset[name].start_us || p.root_spikes != reset[name].root_spikes
        }) {
            return Err(error("contingency probe root inputs differ"));
        }
    }
    let targets = templates(reset);
    let available = targets.iter().all(|t| !t.is_empty());
    let scores = probes
        .iter()
        .map(|(name, rows)| (name.clone(), counts(rows, &targets)))
        .collect();
    let test = |suffix, cues| available && gate(&probes, &scores, suffix, cues);
    let (original_core, fragments_core) = (test("core", &CUES[..]), test("core", &FRAGMENTS[..]));
    let (original_all, fragments_all) = (test("all", &CUES[..]), test("all", &FRAGMENTS[..]));
    let effects = probes
        .iter()
        .filter(|(arm, _)| arm.ends_with("_core"))
        .map(|(_, rows)| {
            rows.iter()
                .filter(|&(&name, p)| p.core_spikes != reset[name].core_spikes)
                .count()
        })
        .sum::<usize>();
    let probes = probes
        .iter()
        .map(|(arm, rows)| {
            let responses = rows
                .iter()
                .map(|(&name, p)| (name, p.json()))
                .collect::<BTreeMap<_, _>>();
            (
                arm,
                json!({"weights":serialized(&arms[arm]),"responses":responses}),
            )
        })
        .collect::<BTreeMap<_, _>>();
    Ok(TrialMetrics {
        values: METRICS
            .into_iter()
            .map(str::to_owned)
            .zip([
                f64::from(original_core),
                f64::from(fragments_core),
                f64::from(original_all),
                f64::from(fragments_all),
                f64::from(available),
                effects as f64,
            ])
            .collect(),
        details: json!({"protocol":PROTOCOL,"gap_frames":gap,"core_ids":core_ids,"stimuli":patterns,
            "development":development,"edges":edges,"training":training,"probes":probes,
            "templates":targets,"counts":scores,"gates":{
                "core":{"original":original_core,"fragments":fragments_core},
                "all":{"original":original_all,"fragments":fragments_all}}}),
    })
}

impl Scenario for ContingencyScenario {
    type Config = Config;
    fn name(&self) -> &'static str {
        "hippocampal_contingency"
    }
    fn default_config(&self) -> Config {
        AssociationScenario.default_config()
    }
    fn set_parameter(&self, c: &mut Config, name: &str, v: &Value) -> Result<()> {
        HippocampalScenario.set_parameter(c, name, v)
    }
    fn validate(&self, c: &Config) -> Result<()> {
        AssociationScenario.validate(c)?;
        if !c.training_repetitions.is_multiple_of(4) || c.training_repetitions > 32 {
            return Err(error(
                "contingency requires 4..32 repetitions in multiples of four",
            ));
        }
        Ok(())
    }
    fn interventions(&self) -> Vec<Intervention> {
        ["immediate", "gap"]
            .into_iter()
            .map(|name| Intervention { name: name.into() })
            .collect()
    }
    fn metric_names(&self) -> Vec<&'static str> {
        METRICS.to_vec()
    }
    fn snapshot(&self, c: &Config, seed: u64, i: &Intervention) -> Value {
        json!({"protocol":PROTOCOL,"seed":seed,"section":i.name,
            "processing_neurons":c.processing_neurons,"training_repetitions":c.training_repetitions,
            "rest_us":c.rest_us,"stimuli":stimuli(),"gap_frames":if i.name=="gap"{2}else{0},
            "training_trials_per_repetition":2,"frames_per_trial":5,"frames_per_probe":4,
            "mappings":MAPPINGS,"learning":"ThreeFactorAccumulated",
            "checkpoint":"MLD02 native development and All homeostasis for 1 s; freeze maintenance after development"})
    }
    fn run(&self, c: &Config, seed: u64, i: &Intervention) -> Result<TrialMetrics> {
        self.validate(c)?;
        let gap = match i.name.as_str() {
            "immediate" => 0,
            "gap" => 2,
            _ => return Err(error("unknown contingency intervention")),
        };
        run(c, seed, gap)
    }
    fn evaluate(&self, rows: &[Value], replay: bool) -> Result<Value> {
        Ok(json!({"status":"visual_contingency_pilot","passed":null,
            "records":rows.len(),"replay_checked":replay}))
    }
}
