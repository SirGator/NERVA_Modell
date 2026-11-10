//! Local coavailability, with explicit pre/post, receptor, timing and spatial knockouts.
use super::{Outcome, cell, flag};
use crate::{Result, error};
use nerva::{
    config::{LearningConfig, RuntimeConfig},
    core::{Network, NeuronId, SimTime, Synapse, SynapseId},
    learning::{PairStdp, ThreeFactorConfig},
    neuromodulation::{ChemicalConfig, ChemicalSpecies, ReceptorProfile},
    primitives::Weight,
    runtime::{LearningAuditEvent, ObservationEvent, Simulation},
};
use serde_json::{Value, json};

pub(super) const CONTROLS: [&str; 7] = [
    "coincident",
    "no_pre",
    "no_post",
    "receptor_knockout",
    "late_modulator",
    "remote_modulator",
    "frozen",
];
fn condition(name: &str) -> Result<Value> {
    let mut net = Network::new();
    for (id, x) in [
        (1, 0.),
        (2, 0.1),
        (3, 0.1),
        (
            4,
            if name == "remote_modulator" {
                10.
            } else {
                0.15
            },
        ),
    ] {
        let mut n = cell(id, x)?;
        if id == 3 || (id == 2 && name != "receptor_knockout") {
            n.set_receptor_profile(ReceptorProfile {
                dopamine_d1_like: 1.,
                ..Default::default()
            })
            .map_err(super::fail)?;
        }
        net.add_neuron(n).map_err(super::fail)?;
    }
    for (edge, post) in [(7, 2), (8, 3)] {
        net.add_synapse(
            Synapse::new(
                SynapseId(edge),
                NeuronId(1),
                NeuronId(post),
                Weight::new(0.05).map_err(super::fail)?,
                1000,
                true,
            )
            .map_err(super::fail)?,
        )
        .map_err(super::fail)?;
    }
    let rule = PairStdp::new(&LearningConfig::default())
        .with_three_factor(ThreeFactorConfig::default())
        .map_err(super::fail)?;
    let mut sim = Simulation::new(net, rule, RuntimeConfig::default(), 1.)
        .map_err(super::fail)?
        .with_neuromodulation(ChemicalConfig::default())
        .map_err(super::fail)?;
    sim.set_learning_enabled(name != "frozen");
    sim.enable_learning_audit(100000).map_err(super::fail)?;
    if name != "no_pre" {
        sim.schedule_external_input(SimTime(1000), NeuronId(1), 1.1)
            .map_err(super::fail)?;
    }
    if name != "no_post" {
        sim.schedule_external_input(SimTime(10000), NeuronId(2), 1.1)
            .map_err(super::fail)?;
    }
    sim.run_until(SimTime(50000)).map_err(super::fail)?;
    let tag = sim
        .network()
        .synapse(SynapseId(7))
        .unwrap()
        .learning_eligibility();
    let mut logs = vec![sim.drain_learning_audit()];
    let release = if name == "late_modulator" {
        10_000_000
    } else {
        100_000
    };
    sim.schedule_chemical_release(
        SimTime(release),
        NeuronId(4),
        ChemicalSpecies::Dopamine,
        0.002,
    )
    .map_err(super::fail)?;
    sim.run_until(SimTime(release + 100000))
        .map_err(super::fail)?;
    logs.push(sim.disable_learning_audit());
    let mut traces = Vec::new();
    let mut chemical = Vec::new();
    let mut rows = 0;
    let mut requests = 0;
    let mut no_change = 0;
    let mut requested_mass = 0.;
    let mut first_signal = None;
    let mut last_signal = None;
    let mut zero_signal = 0;
    for log in logs {
        if !log.is_complete() {
            return Err(error("local-availability audit incomplete"));
        }
        rows += log.events.len() + log.delayed.len();
        if let Some(s) = log.delayed.get(&SynapseId(7)) {
            requests += s.nonzero_requested;
            no_change += s.nonzero_requested_without_weight_change;
            zero_signal += s.zero_signal_integrations;
            requested_mass += s.positive_requested - s.negative_requested;
            if let Some(time) = s.first_nonzero_signal_us {
                first_signal = Some(first_signal.map_or(time, |old: u64| old.min(time)));
            }
            if let Some(time) = s.last_nonzero_signal_us {
                last_signal = Some(last_signal.map_or(time, |old: u64| old.max(time)));
            }
        }
        for event in log.events {
            match event {
                LearningAuditEvent::Trace {
                    time,
                    synapse,
                    phase,
                    tag_before,
                    tag_after,
                    held_signal,
                    old_weight,
                    new_weight,
                    delayed,
                    ..
                } if synapse == SynapseId(7) => {
                    traces.push(json!({"time_us":time.0,"synapse_id":synapse.0,"phase":format!("{phase:?}"),
                            "tag_before":f64::from(tag_before),"tag_after":f64::from(tag_after),
                            "held_factor":f64::from(held_signal),"weight_before":f64::from(old_weight.get()),
                            "weight_after":f64::from(new_weight.get()),
                            "requested_delta":delayed.map(|d|d.unrounded_delta)}));
                }
                LearningAuditEvent::ChemicalSample {
                    time,
                    neuron,
                    concentrations,
                    response,
                } if neuron == NeuronId(2) => {
                    chemical.push(json!({"time_us":time.0,"neuron":neuron.0,"dopamine":f64::from(concentrations.dopamine),
                        "held_factor":f64::from(response.learning_signal)}));
                }
                _ => {}
            }
        }
    }
    let arrivals: Vec<_> = sim
        .event_log()
        .iter()
        .filter_map(|e| match e {
            ObservationEvent::SynapticArrival {
                time, synapse_id, ..
            } if *synapse_id == SynapseId(7) => Some(time.0),
            _ => None,
        })
        .collect();
    let posts: Vec<_> = sim
        .event_log()
        .iter()
        .filter_map(|e| match e {
            ObservationEvent::SpikeEmitted(s) if s.neuron_id == NeuronId(2) => Some(s.time.0),
            _ => None,
        })
        .collect();
    let weight = sim.network().synapse(SynapseId(7)).unwrap().weight().get();
    let neighbor = sim.network().synapse(SynapseId(8)).unwrap().weight().get();
    Ok(
        json!({"control":name,"synapse_id":7,"pre":1,"post":2,"source":4,
        "pre_arrivals_us":arrivals,"post_spikes_us":posts,"tag_before_release":f64::from(tag),
        "release_us":release,"initial_weight":f64::from(0.05f32),"final_weight":f64::from(weight),
        "weight_delta":f64::from(weight)-f64::from(0.05f32),"unmarked_neighbor_unchanged":neighbor==0.05,
        "nonzero_requests":requests,"requests_without_change":no_change,"requested_absolute_mass":requested_mass,
        "zero_factor_integrations":zero_signal,"first_nonzero_factor_us":first_signal,"last_nonzero_factor_us":last_signal,
        "trace_events":traces,"local_chemical_samples":chemical,"audit_rows":rows,"audit_complete":true,
        "local_factor_is_measured_at_postsynaptic_cell":true}),
    )
}
pub(super) fn run() -> Result<Outcome> {
    let mut cases = Vec::new();
    for name in CONTROLS {
        cases.push(condition(name)?);
    }
    let passed = cases[0]["weight_delta"].as_f64().unwrap() > 0.
        && cases[0]["tag_before_release"].as_f64().unwrap() > 0.
        && cases[0]["nonzero_requests"].as_u64().unwrap() > 0
        && cases[1..].iter().all(|r| r["weight_delta"] == 0.)
        && cases
            .iter()
            .all(|r| r["unmarked_neighbor_unchanged"] == true);
    let rows = cases
        .iter()
        .map(|r| r["audit_rows"].as_u64().unwrap() as usize)
        .sum();
    Ok(Outcome {
        stages: [
            flag(
                !cases[0]["pre_arrivals_us"].as_array().unwrap().is_empty()
                    && !cases[0]["post_spikes_us"].as_array().unwrap().is_empty(),
            ),
            -1.,
            flag(cases[0]["tag_before_release"].as_f64().unwrap() > 0.),
            flag(cases[0]["weight_delta"].as_f64().unwrap() > 0.),
            -1.,
            -1.,
        ],
        passed,
        rows,
        training: CONTROLS.len(),
        details: json!({"scope":"Controlled native synapse with known pre/post and artificial chemical release",
            "cases":cases,"limits":"Late pulses can overlap a very small nonzero tail; overlap, requested mass and actual f32 change are separate measurements. No physiological calibration or autonomous reward."}),
    })
}
