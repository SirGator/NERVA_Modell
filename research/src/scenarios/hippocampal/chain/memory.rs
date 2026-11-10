//! Guaranteed native A->B transfer; no teacher input ever reaches B.
use super::{Outcome, cell, flag};
use crate::Result;
use nerva::{
    config::RuntimeConfig,
    core::{Network, NeuronId, SimTime, Synapse, SynapseId},
    learning::NoPlasticity,
    primitives::Weight,
    runtime::{ObservationEvent, Simulation},
};
use serde_json::{Value, json};
pub(super) fn probe(weight: f32, enabled: bool, cue: Option<u64>, wait: u64) -> Result<Value> {
    let mut network = Network::new();
    for (id, x) in [(1, 0.), (2, 0.1), (3, 0.2)] {
        network.add_neuron(cell(id, x)?).map_err(super::fail)?;
    }
    let mut edge = Synapse::new(
        SynapseId(7),
        NeuronId(1),
        NeuronId(2),
        Weight::new(weight).map_err(super::fail)?,
        1000,
        false,
    )
    .map_err(super::fail)?;
    edge.set_enabled(enabled);
    network.add_synapse(edge).map_err(super::fail)?;
    let mut sim = Simulation::new(network, NoPlasticity, RuntimeConfig::default(), 1.)
        .map_err(super::fail)?;
    sim.run_until(SimTime(wait)).map_err(super::fail)?;
    let stored = sim.network().synapse(SynapseId(7)).unwrap().weight().get();
    if let Some(id) = cue {
        sim.schedule_external_input(SimTime(wait + 10000), NeuronId(id), 1.1)
            .map_err(super::fail)?;
    }
    sim.run_until(SimTime(wait + 20000)).map_err(super::fail)?;
    let spikes: Vec<_> = sim
        .event_log()
        .iter()
        .filter_map(|e| match e {
            ObservationEvent::SpikeEmitted(s) => Some((s.neuron_id.0, s.time.0)),
            _ => None,
        })
        .collect();
    let arrivals: Vec<_> = sim
        .event_log()
        .iter()
        .filter_map(|e| match e {
            ObservationEvent::SynapticArrival {
                time,
                synapse_id,
                target,
                amplitude,
                ..
            } => Some((time.0, synapse_id.0, target.0, f64::from(*amplitude))),
            _ => None,
        })
        .collect();
    Ok(
        json!({"weight":f64::from(weight),"enabled":enabled,"cue_neuron":cue,"wait_us":wait,
        "weight_retained":stored==weight && sim.network().synapse(SynapseId(7)).unwrap().weight().get()==weight,
        "spikes":spikes,"arrivals":arrivals,
        "target_fired":spikes.iter().any(|(id,_)|*id==2),"source_fired":spikes.iter().any(|(id,_)|*id==1),
        "distractor_fired":spikes.iter().any(|(id,_)|*id==3),"direct_target_input":false}),
    )
}
pub(super) fn run(wait: u64) -> Result<Outcome> {
    let mut cases = Vec::new();
    for (name, weight, enabled, cue) in [
        ("preset", 1.5, true, Some(1)),
        ("zero_weight", 0., true, Some(1)),
        ("blocked_path", 1.5, false, Some(1)),
        ("no_cue", 1.5, true, None),
        ("other_cue", 1.5, true, Some(3)),
    ] {
        let mut result = probe(weight, enabled, cue, wait)?;
        result["control"] = json!(name);
        cases.push(result);
    }
    let passed = cases[0]["target_fired"] == true
        && cases[1..].iter().all(|r| r["target_fired"] == false)
        && cases.iter().all(|r| r["weight_retained"] == true)
        && cases[4]["distractor_fired"] == true;
    Ok(Outcome {
        stages: [
            flag(cases[0]["source_fired"] == true && cases[4]["distractor_fired"] == true),
            flag(cases[0]["spikes"] != cases[4]["spikes"]),
            -1.,
            flag(cases.iter().all(|r| r["weight_retained"] == true)),
            -1.,
            flag(passed),
        ],
        passed,
        rows: cases.len(),
        training: 0,
        details: json!({"scope":"Artificial three-cell circuit, not the hippocampal actor","cases":cases,
            "preset_edge":[1,2,1.5,1000],"learning":false,
            "limits":"Demonstrates cue-selective propagation and retained preset weights in this circuit only; no learned association or generalization."}),
    })
}
