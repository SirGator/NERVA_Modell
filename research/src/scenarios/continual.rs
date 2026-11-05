//! Small controlled A/B acquisition and interference assay, using existing STDP.
use crate::{
    Result, error,
    scenario::{Intervention, Scenario, TrialMetrics},
};
use nerva::{
    config::NervaConfig,
    core::{Network, Neuron, NeuronId, NeuronRole, Polarity, SimTime, Synapse, SynapseId},
    experiment::M0ExperimentConfig,
    learning::{NoPlasticity, PairStdp},
    math::Position3D,
    primitives::Weight,
    runtime::{ObservationEvent, Simulation},
};
use serde_json::{Value, json};
use std::collections::BTreeMap;

pub struct ContinualScenario;
#[derive(Clone)]
pub struct Config {
    pub native: NervaConfig,
    pub repetitions: u64,
    pub interval_us: u64,
}
const A: [u64; 3] = [0, 1, 2];
const B: [u64; 3] = [3, 1, 5];
const DISJOINT_B: [u64; 3] = [3, 4, 5];
const PAUSE: u64 = 40_000;
const PROBE: u64 = 80_000;

fn fail<E: std::fmt::Display>(e: E) -> nerva::experiment::ExperimentError {
    error(e.to_string())
}
fn next(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9e3779b97f4a7c15);
    let mut x = *state;
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d049bb133111eb);
    x ^ (x >> 31)
}
fn network(c: &Config, seed: u64) -> Result<Network> {
    let mut n = Network::new();
    for id in 0..7 {
        n.add_neuron(
            Neuron::new(
                NeuronId(id),
                Position3D::new(id as f32 * 0.01, 0., 0.),
                if id == 6 {
                    Polarity::Inhibitory
                } else {
                    Polarity::Excitatory
                },
                Some(NeuronRole::Processing),
                c.native.neuron,
                SimTime::ZERO,
            )
            .map_err(fail)?,
        )
        .map_err(fail)?;
    }
    let mut offsets: Vec<f32> = (0..30).map(|i| (i % 3) as f32 * 0.005 - 0.005).collect();
    let mut rng = seed;
    for i in (1..offsets.len()).rev() {
        let j = (next(&mut rng) % (i as u64 + 1)) as usize;
        offsets.swap(i, j);
    }
    let mut edge = 0;
    for pre in 0..6 {
        for post in 0..6 {
            if pre == post {
                continue;
            }
            n.add_synapse(
                Synapse::new(
                    SynapseId(edge),
                    NeuronId(pre),
                    NeuronId(post),
                    Weight::new(0.12 + offsets[edge as usize]).map_err(fail)?,
                    10_000,
                    true,
                )
                .map_err(fail)?,
            )
            .map_err(fail)?;
            edge += 1;
        }
    }
    for id in 0..6 {
        for (pre, post, weight) in [(id, 6, 1.1), (6, id, 0.05)] {
            n.add_synapse(
                Synapse::new(
                    SynapseId(edge),
                    NeuronId(pre),
                    NeuronId(post),
                    Weight::new(weight).map_err(fail)?,
                    1_000,
                    false,
                )
                .map_err(fail)?,
            )
            .map_err(fail)?;
            edge += 1;
        }
    }
    Ok(n)
}
fn weights(n: &Network) -> Vec<f32> {
    n.synapses().map(|s| s.weight().get()).collect()
}
fn train(
    sim: &mut Simulation<PairStdp>,
    c: &Config,
    sequence: Option<[u64; 3]>,
    start: u64,
) -> Result<u64> {
    let duration = 3 * c.interval_us + PAUSE;
    if let Some(sequence) = sequence {
        for repeat in 0..c.repetitions {
            for (i, id) in sequence.iter().enumerate() {
                sim.schedule_external_input(
                    SimTime(start + repeat * duration + i as u64 * c.interval_us),
                    NeuronId(*id),
                    1.2,
                )
                .map_err(fail)?;
            }
        }
    }
    let end = start + c.repetitions * duration + PAUSE;
    sim.run_until(SimTime(end)).map_err(fail)?;
    Ok(end)
}
fn probe(n: &Network, c: &Config, sequence: [u64; 3], at: u64) -> Result<Value> {
    let before = weights(n);
    let mut sim = Simulation::new(
        n.clone(),
        NoPlasticity,
        c.native.runtime,
        c.native.network.distance_decay_length,
    )
    .map_err(fail)?;
    sim.schedule_external_input(SimTime(at), NeuronId(sequence[0]), 1.2)
        .map_err(fail)?;
    sim.run_until(SimTime(at + PROBE)).map_err(fail)?;
    let spikes: Vec<_> = sim
        .event_log()
        .iter()
        .filter_map(|event| match event {
            ObservationEvent::SpikeEmitted(s) if s.neuron_id.0 < 6 => {
                Some((s.neuron_id.0, s.time.0))
            }
            _ => None,
        })
        .collect();
    let matched: Vec<_> = sequence
        .iter()
        .enumerate()
        .map(|(i, id)| {
            spikes
                .iter()
                .any(|(nid, t)| nid == id && t.abs_diff(at + i as u64 * 10_000) <= 2)
        })
        .collect();
    let hits = matched.iter().skip(1).filter(|x| **x).count();
    let extras = spikes
        .len()
        .saturating_sub(matched.iter().filter(|x| **x).count());
    let stable = !sim.has_pending_non_homeostasis_events();
    let score = hits as i64 - extras as i64;
    Ok(
        json!({"sequence":sequence,"spikes":spikes,"score":score,"hits":hits,"extra_spikes":extras,
        "stable":stable,"weights_frozen":before == weights(sim.network()),
        "cue_present":matched[0],"events":format!("{:?}",sim.event_log())}),
    )
}
fn probes(n: &Network, c: &Config, b: [u64; 3], at: u64) -> Result<Value> {
    let first = json!({"a":probe(n,c,A,at)?,"b":probe(n,c,b,at)?});
    let replay = json!({"a":probe(n,c,A,at)?,"b":probe(n,c,b,at)?});
    if first != replay {
        return Err(error("continual frozen probe replay differs"));
    }
    Ok(first)
}
impl Scenario for ContinualScenario {
    type Config = Config;
    fn name(&self) -> &'static str {
        "continual_sequences"
    }
    fn default_config(&self) -> Config {
        let mut native = M0ExperimentConfig::default().nerva;
        native.network.excitatory_neurons = 6;
        native.network.inhibitory_neurons = 1;
        native.learning.homeostasis.enabled = false;
        Config {
            native,
            repetitions: 24,
            interval_us: 10_000,
        }
    }
    fn set_parameter(&self, c: &mut Config, name: &str, v: &Value) -> Result<()> {
        match name {
            "training_repetitions" => {
                c.repetitions = v
                    .as_u64()
                    .ok_or_else(|| error("integer repetitions required"))?
            }
            "symbol_interval_us" => {
                c.interval_us = v
                    .as_u64()
                    .ok_or_else(|| error("integer interval required"))?
            }
            "learning.a_plus" | "learning.a_minus" => {
                let value = v
                    .as_f64()
                    .filter(|x| x.is_finite() && *x >= 0. && *x <= 1.)
                    .ok_or_else(|| error("learning rate must be 0..1"))?
                    as f32;
                if name.ends_with("a_plus") {
                    c.native.learning.a_plus = value;
                } else {
                    c.native.learning.a_minus = value;
                }
            }
            _ => return Err(error("unknown continual-assay parameter")),
        }
        Ok(())
    }
    fn validate(&self, c: &Config) -> Result<()> {
        c.native.validate().map_err(fail)?;
        if !(1..=64).contains(&c.repetitions) || !(5_000..=30_000).contains(&c.interval_us) {
            return Err(error(
                "continual assay requires 1..64 repetitions and 5000..30000us intervals",
            ));
        }
        Ok(())
    }
    fn snapshot(&self, c: &Config, seed: u64, i: &Intervention) -> Value {
        let mut native = c.native.clone();
        native.network.seed = seed;
        json!({"native":format!("{native:?}"),"seed":seed,"intervention":i.name,
            "repetitions":c.repetitions,"interval_us":c.interval_us,"probe_expected_delay_us":10000,
            "a":A,"b":if i.name == "disjoint_b" { DISJOINT_B } else { B },
            "input":"External current pulses to processing cells; sensor coding intentionally bypassed",
            "topology":"All directed processing edges, equal target-blind weight pool; no task-specific edges"})
    }
    fn interventions(&self) -> Vec<Intervention> {
        ["none", "no_b", "freeze_after_a", "disjoint_b"]
            .iter()
            .map(|s| Intervention { name: (*s).into() })
            .collect()
    }
    fn metric_names(&self) -> Vec<&'static str> {
        vec![
            "untrained_a_score",
            "untrained_b_score",
            "after_a_score_a",
            "after_a_score_b",
            "after_b_score_a",
            "after_b_score_b",
            "a_loss",
            "a_acquired",
            "b_acquired",
            "interference_assessable",
            "interference_observed",
            "stability",
            "weights_frozen",
            "probe_replay_identical",
            "event_count",
        ]
    }
    fn run(&self, c: &Config, seed: u64, i: &Intervention) -> Result<TrialMetrics> {
        let b = if i.name == "disjoint_b" {
            DISJOINT_B
        } else {
            B
        };
        let n = network(c, seed)?;
        let untrained = probes(&n, c, b, 1)?;
        let mut native = c.native.clone();
        native.network.seed = seed;
        let rule =
            PairStdp::try_from_config(&native.learning).map_err(|e| error(format!("{e:?}")))?;
        let mut sim = Simulation::from_config(n, rule, &native).map_err(fail)?;
        let end_a = train(&mut sim, c, Some(A), 1)?;
        let stable_a = !sim.has_pending_non_homeostasis_events();
        let weights_a = weights(sim.network());
        let after_a = probes(sim.network(), c, b, end_a + PAUSE)?;
        if i.name == "freeze_after_a" {
            sim.freeze_learning();
        }
        let end_b = train(
            &mut sim,
            c,
            if i.name == "no_b" { None } else { Some(b) },
            end_a + PAUSE,
        )?;
        let stable_b = !sim.has_pending_non_homeostasis_events();
        let after_b = probes(sim.network(), c, b, end_b + PAUSE)?;
        let weights_b = weights(sim.network());
        let sets = [&untrained, &after_a, &after_b];
        let stable = stable_a
            && stable_b
            && sets
                .iter()
                .all(|v| v["a"]["stable"] == true && v["b"]["stable"] == true);
        let frozen = sets
            .iter()
            .all(|v| v["a"]["weights_frozen"] == true && v["b"]["weights_frozen"] == true);
        let score = |v: &Value, task: &str| v[task]["score"].as_f64().unwrap();
        let acquired_a = score(&after_a, "a") == 2. && score(&untrained, "a") < 2.;
        let acquired_b = score(&after_b, "b") == 2. && score(&untrained, "b") < 2.;
        let loss = score(&after_a, "a") - score(&after_b, "a");
        let assessable = stable && frozen && acquired_a && acquired_b && i.name == "none";
        let numbers = [
            score(&untrained, "a"),
            score(&untrained, "b"),
            score(&after_a, "a"),
            score(&after_a, "b"),
            score(&after_b, "a"),
            score(&after_b, "b"),
            loss,
            f64::from(acquired_a),
            f64::from(acquired_b),
            f64::from(assessable),
            f64::from(assessable && loss > 0.),
            f64::from(stable),
            f64::from(frozen),
            1.,
            sim.event_log().len() as f64,
        ];
        Ok(TrialMetrics {
            values: self
                .metric_names()
                .into_iter()
                .map(String::from)
                .zip(numbers)
                .collect::<BTreeMap<_, _>>(),
            details: json!({"untrained":untrained,"after_a":after_a,"after_b":after_b,
                "weights_after_a":weights_a,"weights_after_b":weights_b,
                "training_a_stable":stable_a,"training_b_stable":stable_b,
                "training_events":format!("{:?}",sim.event_log()),
                "limits":"Technical acquisition/interference assay. Frozen fork probes never train the parent. Overlap shares one internal pattern, distinct cues. No action-effect or independent-task claim."}),
        })
    }
    fn evaluate(&self, records: &[Value], full_replay: bool) -> Result<Value> {
        let tested: Vec<_> = records
            .iter()
            .filter(|r| r["job"]["intervention"] == "none")
            .collect();
        Ok(
            json!({"status":"technical_acquisition_assay","passed":null,"full_replay":full_replay,
            "assessable_seeds":tested.iter().filter(|r|r["metrics"]["interference_assessable"] == 1.).count(),
            "note":"Do not diagnose forgetting unless both tasks are acquired and matched controls support loss beyond aging."}),
        )
    }
}
