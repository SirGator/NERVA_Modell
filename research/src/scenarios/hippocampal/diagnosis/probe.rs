//! Bounded, streamed electrical observations. No simulation or learning changes.
use super::super::{Config, cosine, fail, frame};
use crate::{Result, error, fingerprint_debug};
use nerva::{
    core::{NeuronRole, SynapseId},
    runtime::ObservationEvent,
};
use nerva_vm::model::{
    Model,
    audit::{AuditOptions, learned_parameters_equal},
};
use serde_json::{Value, json};
use std::collections::BTreeMap;

const FRAME_LIMIT: usize = 200_000;
const TOTAL_LIMIT: usize = 2_000_000;
#[derive(Default)]
struct Node {
    batches: usize,
    spikes: usize,
    refractory: usize,
    subthreshold: usize,
    cancelled: usize,
    positive: f64,
    negative: f64,
    ignored_net: f64,
    continuous: f64,
    reset: f64,
    accepted_net: f64,
    integration_rounding: f64,
    minimum_gap: Option<f64>,
}
impl Node {
    fn json(&self, id: u64) -> Value {
        json!({"id":id,"batches":self.batches,"spikes":self.spikes,"refractory_batches":self.refractory,
            "subthreshold_batches":self.subthreshold,"excitation_cancelled_batches":self.cancelled,
            "positive_input":self.positive,"negative_input":self.negative,"refractory_ignored_net":self.ignored_net,
            "continuous_potential_change":self.continuous,"reset_potential_change":self.reset,
            "accepted_net":self.accepted_net,"integration_rounding":self.integration_rounding,
            "minimum_eligible_threshold_gap":self.minimum_gap})
    }
}
#[derive(Default)]
struct Edge {
    arrivals: usize,
    positive: f64,
    negative: f64,
    refractory: usize,
    subthreshold: usize,
}
impl Edge {
    fn json(&self, id: u64) -> Value {
        json!({"id":id,"arrivals":self.arrivals,"positive_amplitude":self.positive,"negative_amplitude":self.negative,
            "refractory_arrivals":self.refractory,"subthreshold_arrivals":self.subthreshold})
    }
}
pub(in crate::scenarios::hippocampal) struct Probe {
    pub counts: Vec<Vec<usize>>,
    pub source_counts: Vec<BTreeMap<u64, usize>>,
    pub details: Value,
}
pub(in crate::scenarios::hippocampal) fn run(model: &mut Model, c: &Config) -> Result<Probe> {
    model.freeze().map_err(fail)?;
    let parameters = model.network().clone();
    model.enable_audit(AuditOptions {
        raw_events: true,
        input_spikes: false,
    });
    let mut counts = Vec::new();
    let mut source_counts = Vec::new();
    let mut frames = Vec::new();
    let mut total = 0usize;
    // Black first: cue-free activation control. Fixed targets precede the two cues.
    for image in [4, 0, 1, 2, 3] {
        model.set_dynamics_auditing(false);
        let mut remaining = c.rest_us;
        let mut resting = 0;
        while remaining > 0 {
            let duration = remaining.min(50_000);
            let step = model
                .step_for(&frame(4).map_err(fail)?, duration)
                .map_err(fail)?;
            resting += step.core_counts.iter().sum::<usize>();
            let audit = model
                .take_audit()
                .ok_or_else(|| error("missing rest audit"))?;
            if audit.raw_events.len() > FRAME_LIMIT {
                return Err(error("recall rest audit budget exceeded"));
            }
            remaining -= duration;
        }
        model.set_dynamics_auditing(true);
        let potentials_before: BTreeMap<_, _> = model
            .network()
            .neurons()
            .map(|n| (n.id().0, (n.membrane_potential(), n.last_update().0)))
            .collect();
        let step = model.step(&frame(image).map_err(fail)?).map_err(fail)?;
        let audit = model
            .take_audit()
            .ok_or_else(|| error("missing electrical audit"))?;
        total += audit.raw_events.len();
        if audit.raw_events.len() > FRAME_LIMIT || total > TOTAL_LIMIT {
            return Err(error("electrical audit budget exceeded"));
        }
        let mut nodes = BTreeMap::<u64, Node>::new();
        let mut edges = BTreeMap::<u64, Edge>::new();
        let mut sources = BTreeMap::<u64, usize>::new();
        let mut pending = BTreeMap::<(u64, u64), Vec<SynapseId>>::new();
        let mut examples = Vec::new();
        for event in &audit.raw_events {
            match *event {
                ObservationEvent::SpikeEmitted(s) => {
                    *sources.entry(s.neuron_id.0).or_default() += 1;
                }
                ObservationEvent::NeuronAdvanced {
                    target,
                    potential_before,
                    potential_after,
                    ..
                } => {
                    if matches!(
                        model.network().neuron(target).unwrap().role(),
                        Some(NeuronRole::Processing | NeuronRole::Motor)
                    ) {
                        nodes.entry(target.0).or_default().continuous +=
                            f64::from(potential_after) - f64::from(potential_before);
                    }
                }
                ObservationEvent::SynapticArrival {
                    time,
                    synapse_id,
                    target,
                    amplitude,
                    ..
                } => {
                    if matches!(
                        model.network().neuron(target).unwrap().role(),
                        Some(NeuronRole::Processing | NeuronRole::Motor)
                    ) {
                        let edge = edges.entry(synapse_id.0).or_default();
                        edge.arrivals += 1;
                        edge.positive += f64::from(amplitude.max(0.));
                        edge.negative += f64::from(amplitude.min(0.));
                        pending
                            .entry((time.0, target.0))
                            .or_default()
                            .push(synapse_id);
                    }
                }
                ObservationEvent::NeuronIntegrated {
                    time,
                    target,
                    potential_before,
                    potential_after,
                    threshold,
                    refractory_until,
                    summed_input,
                    positive_input,
                    negative_input,
                    spiked,
                } => {
                    if !matches!(
                        model.network().neuron(target).unwrap().role(),
                        Some(NeuronRole::Processing | NeuronRole::Motor)
                    ) {
                        continue;
                    }
                    let refractory = time < refractory_until;
                    let candidate = potential_before + summed_input; // same f32 addition as the native cell
                    let gap = f64::from(threshold) - f64::from(candidate);
                    if spiked != (!refractory && candidate >= threshold)
                        || (!spiked
                            && potential_after
                                != if refractory {
                                    potential_before
                                } else {
                                    candidate
                                })
                    {
                        return Err(error(
                            "electrical boundary disagrees with native integration",
                        ));
                    }
                    let subthreshold = !refractory && !spiked;
                    // One-batch counterfactual only; not a simulated inhibition knockout.
                    let cancelled = subthreshold
                        && negative_input < 0.
                        && f64::from(potential_before) + positive_input >= f64::from(threshold);
                    let node = nodes.entry(target.0).or_default();
                    node.batches += 1;
                    node.spikes += usize::from(spiked);
                    node.refractory += usize::from(refractory);
                    node.subthreshold += usize::from(subthreshold);
                    node.cancelled += usize::from(cancelled);
                    node.positive += positive_input;
                    node.negative += negative_input;
                    if refractory {
                        node.ignored_net += f64::from(summed_input);
                    } else {
                        node.minimum_gap = Some(node.minimum_gap.map_or(gap, |old| old.min(gap)));
                        node.accepted_net += f64::from(summed_input);
                        node.integration_rounding += f64::from(candidate)
                            - f64::from(potential_before)
                            - f64::from(summed_input);
                    }
                    if spiked {
                        node.reset += f64::from(potential_after) - f64::from(candidate);
                    }
                    for id in pending.remove(&(time.0, target.0)).unwrap_or_default() {
                        let edge = edges.get_mut(&id.0).unwrap();
                        edge.refractory += usize::from(refractory);
                        edge.subthreshold += usize::from(subthreshold);
                    }
                    if examples.len() < 128 {
                        examples.push(json!({"time_us":time.0,"neuron":target.0,"potential_before":potential_before,
                            "potential_after":potential_after,"threshold":threshold,"refractory_until_us":refractory_until.0,
                            "net":summed_input,"positive":positive_input,"negative":negative_input,
                            "gap":gap,"spiked":spiked,"refractory":refractory,"excitation_cancelled":cancelled}));
                    }
                }
                _ => {}
            }
        }
        if !pending.is_empty() {
            return Err(error("arrival lacks integration boundary"));
        }
        let core_batches: usize = nodes
            .iter()
            .filter(|(id, _)| model.processing_ids().iter().any(|n| n.0 == **id))
            .map(|(_, n)| n.spikes)
            .sum();
        if core_batches != step.core_counts.iter().sum::<usize>() {
            return Err(error("electrical audit missing native spikes"));
        }
        let mut node_rows = Vec::new();
        for (&id, node) in &nodes {
            let (before, from) = potentials_before[&id];
            let neuron = model.network().neuron(nerva::core::NeuronId(id)).unwrap();
            let error = f64::from(neuron.membrane_potential())
                - f64::from(before)
                - (node.continuous + node.accepted_net + node.integration_rounding + node.reset);
            if error.abs() > 1e-9 {
                return Err(crate::error("recall potential ledger does not balance"));
            }
            let mut row = node.json(id);
            row["stored_before"] = json!(before);
            row["stored_after"] = json!(neuron.membrane_potential());
            row["stored_before_time_us"] = json!(from);
            row["stored_after_time_us"] = json!(neuron.last_update().0);
            row["balance_error"] = json!(error);
            node_rows.push(row);
        }
        frames.push(json!({"image":image,"start_us":step.start_us,"end_us":step.end_us,
            "core_counts":step.core_counts,"motor_counts":step.motor_counts,
            "source_spike_counts":sources.iter().map(|(&id,&count)|(id,count)).collect::<Vec<_>>(),
            "core_event_digest":fingerprint_debug(&step.core_events),"motor_event_digest":fingerprint_debug(&step.motor_events),
            "rest_core_spikes":resting,"audit_rows":audit.raw_events.len(),
            "neurons":node_rows,
            "synapses":edges.iter().map(|(&id,e)|e.json(id)).collect::<Vec<_>>(),
            "boundary_examples":examples,"examples_truncated":nodes.values().map(|n|n.batches).sum::<usize>()>128}));
        counts.push(step.core_counts);
        source_counts.push(sources);
    }
    model.set_dynamics_auditing(false);
    if !learned_parameters_equal(&parameters, model.network()) {
        return Err(error("diagnostic probe changed learned parameters"));
    }
    Ok(Probe {
        counts,
        source_counts,
        details: json!({"frames":frames,"complete":true,"audit_rows":total,
        "frame_row_limit":FRAME_LIMIT,"total_row_limit":TOTAL_LIMIT,"parameters_preserved":true,
        "continuous_change_scope":"Native continuous evolution includes leak, intrinsic currents and modulation; not isolated passive leak.",
        "cancellation_scope":"Excitation-only comparison at the same pre-integration state; no causal trajectory knockout.",
        "examples_scope":"First 128 boundaries per image; aggregate counts cover every boundary. Rest events are discarded per frame."}),
    })
}
pub(in crate::scenarios::hippocampal) fn margins(
    probe: &Probe,
    targets: &[Vec<usize>],
) -> [f64; 2] {
    [
        cosine(&probe.counts[3], &targets[0]) - cosine(&probe.counts[3], &targets[1]),
        cosine(&probe.counts[4], &targets[1]) - cosine(&probe.counts[4], &targets[0]),
    ]
}
