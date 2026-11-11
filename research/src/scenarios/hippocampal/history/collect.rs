use super::Config;
use crate::{Result, error};
use nerva::{
    core::{NeuronId, SynapseId},
    runtime::{LearningAuditEvent, LearningAuditLog, LearningPhase, ObservationEvent},
};
use nerva_vm::model::{Model, audit::InputAudit};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

pub(super) const FRAME_LIMIT: usize = 200_000;
#[derive(Default)]
struct Edge {
    pre: u64,
    post: u64,
    initial: f32,
    changed: bool,
    final_weight: f32,
    weight_events: usize,
    positive: f64,
    negative: f64,
    requests: usize,
    no_change: usize,
    zero_signal: usize,
    recall_arrivals: usize,
    last_update: Option<usize>,
}
pub(in crate::scenarios::hippocampal) struct History {
    branch: &'static str,
    neurons: BTreeSet<u64>,
    selected_edges: BTreeSet<u64>,
    edges: BTreeMap<u64, Edge>,
    neuron_metadata: Vec<Value>,
    max_rows: usize,
    observed: usize,
    max_source_rows: usize,
    events: Vec<Value>,
    windows: Vec<Value>,
    counts: BTreeMap<String, usize>,
    mechanisms: BTreeMap<String, (usize, f64, f64)>,
    weight_events: usize,
    no_change: usize,
    reversed_changes: usize,
}
impl History {
    pub fn new(c: &Config) -> Self {
        Self {
            branch: "learned",
            neurons: c.neurons.clone(),
            selected_edges: c.synapses.clone(),
            edges: BTreeMap::new(),
            neuron_metadata: Vec::new(),
            max_rows: c.max_rows,
            observed: 0,
            max_source_rows: c.max_source_rows,
            events: Vec::new(),
            windows: Vec::new(),
            counts: BTreeMap::new(),
            mechanisms: BTreeMap::new(),
            weight_events: 0,
            no_change: 0,
            reversed_changes: 0,
        }
    }
    pub fn initialize(&mut self, model: &Model) -> Result<()> {
        for &id in &self.neurons {
            if model.network().neuron(NeuronId(id)).is_none() {
                return Err(error(format!("unknown history neuron {id}")));
            }
        }
        let watched_neurons = self.neurons.clone();
        for &id in &self.selected_edges {
            let s = model
                .network()
                .synapse(SynapseId(id))
                .ok_or_else(|| error(format!("unknown history synapse {id}")))?;
            self.neurons.extend([s.pre().0, s.post().0]);
        }
        for s in model.network().synapses() {
            self.edges.insert(
                s.id().0,
                Edge {
                    pre: s.pre().0,
                    post: s.post().0,
                    initial: s.weight().get(),
                    ..Default::default()
                },
            );
            if watched_neurons.contains(&s.pre().0) || watched_neurons.contains(&s.post().0) {
                self.selected_edges.insert(s.id().0);
            }
        }
        self.neuron_metadata = model.network().neurons().filter(|n| self.neurons.contains(&n.id().0)).map(|n| json!({
            "neuron_id":n.id().0,"role":format!("{:?}",n.role()),"base_threshold":n.base_threshold(),
            "intrinsic_current":n.intrinsic_current(),"structural_drive":n.structural_drive(),
            "receptors":{"dopamine_d1_like":n.receptor_profile().dopamine_d1_like,"dopamine_d2_like":n.receptor_profile().dopamine_d2_like,
            "acetylcholine":n.receptor_profile().acetylcholine,"noradrenaline":n.receptor_profile().noradrenaline}})).collect();
        Ok(())
    }
    pub fn checkpoint(&mut self, model: &Model) {
        for s in model.network().synapses() {
            let edge = self.edges.get_mut(&s.id().0).unwrap();
            edge.changed = edge.initial != s.weight().get();
            edge.final_weight = s.weight().get();
        }
    }
    pub fn observe_weight_reset(&mut self) {
        self.branch = "weight_reset";
    }
    fn row(
        &mut self,
        time: u64,
        phase: &str,
        mechanism: &str,
        neuron: Option<u64>,
        synapse: Option<u64>,
        data: Value,
    ) -> Result<Option<usize>> {
        *self
            .counts
            .entry(format!("{}.{phase}.{mechanism}", self.branch))
            .or_default() += 1;
        let keep = match synapse {
            Some(s) => self.selected_edges.contains(&s),
            None => neuron.is_some_and(|n| self.neurons.contains(&n)),
        };
        if !keep {
            return Ok(None);
        }
        if self.events.len() + self.windows.len() >= self.max_rows {
            return Err(error(
                "learning history retained row budget exceeded; narrow selection or increase history.max_rows",
            ));
        }
        let stream = if matches!(
            mechanism,
            "stdp_pre" | "stdp_post" | "three_factor" | "neuromodulation"
        ) {
            "learning"
        } else {
            "activity"
        };
        let id = self.events.len();
        let edge = synapse.map(|s| &self.edges[&s]);
        self.events.push(json!({"id":id,"branch":self.branch,"time_us":time,"phase":phase,"mechanism":mechanism,"neuron_id":neuron,
            "synapse_id":synapse,"source_id":edge.map(|s|s.pre),"target_id":edge.map(|s|s.post),"stream":stream,"data":data}));
        Ok(Some(id))
    }
    pub fn add(
        &mut self,
        phase: &str,
        audit: &InputAudit,
        log: Option<&LearningAuditLog>,
        model: &Model,
    ) -> Result<()> {
        let rows = audit.raw_events.len() + log.map_or(0, |l| l.events.len() + l.delayed.len());
        self.observed += rows;
        if rows > FRAME_LIMIT || self.observed > self.max_source_rows {
            return Err(error(format!(
                "learning history source audit budget exceeded (frame {rows}/{FRAME_LIMIT}, total {}/{})",
                self.observed, self.max_source_rows
            )));
        }
        if log.is_some_and(|l| !l.is_complete()) {
            return Err(error("incomplete learning history audit"));
        }
        if let Some(log) = log {
            let mut changes = 0;
            for event in &log.events {
                match *event {
                    LearningAuditEvent::Trace {
                        time,
                        synapse,
                        phase: hook,
                        tag_before,
                        tag_after,
                        tag_anchor_before,
                        held_signal,
                        old_weight,
                        new_weight,
                        residual_before,
                        residual_after,
                        delayed,
                    } => {
                        let mechanism = match hook {
                            LearningPhase::Delayed => "three_factor",
                            LearningPhase::PreArrival => "stdp_pre",
                            LearningPhase::PostSpike => "stdp_post",
                        };
                        let delta = f64::from(new_weight.get()) - f64::from(old_weight.get());
                        let stats = self.mechanisms.entry(mechanism.into()).or_default();
                        stats.0 += usize::from(delta != 0.);
                        stats.1 += delta.max(0.);
                        stats.2 += delta.min(0.);
                        let edge = self.edges.get_mut(&synapse.0).unwrap();
                        edge.weight_events += usize::from(delta != 0.);
                        edge.positive += delta.max(0.);
                        edge.negative += delta.min(0.);
                        changes += usize::from(delta != 0.);
                        self.reversed_changes += usize::from(
                            hook == LearningPhase::Delayed
                                && tag_before > 0.
                                && held_signal < 0.
                                && delta != 0.,
                        );
                        let post = edge.post;
                        let id = self.row(time.0,phase,mechanism,Some(post),Some(synapse.0),json!({"tag_before":tag_before,"tag_after":tag_after,
                            "tag_anchor_before_us":tag_anchor_before.map(|t|t.0),"held_learning_signal":held_signal,
                            "weight_before":old_weight.get(),"weight_after":new_weight.get(),"actual_delta":delta,"residual_before":residual_before,"residual_after":residual_after,
                            "requested_delta":delayed.map(|d|d.unrounded_delta),"tag_integral_seconds":delayed.map(|d|d.tag_integral_seconds),
                            "apply_weight":delayed.map(|d|d.apply_weight),"clipped_delta":delayed.map(|d|d.clipped_delta),"rounding_error":delayed.map(|d|d.rounding_error)}))?;
                        if delta != 0. {
                            self.edges.get_mut(&synapse.0).unwrap().last_update = id;
                        }
                    }
                    LearningAuditEvent::ChemicalSample {
                        time,
                        neuron,
                        concentrations,
                        response,
                    } => {
                        self.row(time.0,phase,"neuromodulation",Some(neuron.0),None,json!({"dopamine":concentrations.dopamine,"acetylcholine":concentrations.acetylcholine,
                            "noradrenaline":concentrations.noradrenaline,"learning_signal":response.learning_signal,"threshold_shift":response.threshold_shift,
                            "intrinsic_drive":response.intrinsic_drive,"transmission_gain":response.transmission_gain}))?;
                    }
                }
            }
            if changes != audit.weight_updates {
                return Err(error("history/native weight event mismatch"));
            }
            self.weight_events += changes;
            for (id, s) in &log.delayed {
                let edge = self.edges.get_mut(&id.0).unwrap();
                edge.requests += s.nonzero_requested;
                edge.no_change += s.nonzero_requested_without_weight_change;
                edge.zero_signal += s.zero_signal_integrations;
                self.no_change += s.nonzero_requested_without_weight_change;
                if self.selected_edges.contains(&id.0) {
                    if self.events.len() + self.windows.len() >= self.max_rows {
                        return Err(error("learning history retained row budget exceeded"));
                    }
                    self.windows.push(json!({"start_us":audit.start_us,"end_us":audit.end_us,"synapse_id":id.0,"source_id":edge.pre,"target_id":edge.post,
                        "integrations":s.integrations,"first_us":s.first_us,"last_us":s.last_us,"positive_requested":s.positive_requested,"negative_requested":s.negative_requested,
                        "positive_requested_no_change":s.positive_requested_no_change,"negative_requested_no_change":s.negative_requested_no_change,
                        "nonzero_requested":s.nonzero_requested,"requests_without_change":s.nonzero_requested_without_weight_change,"zero_signal_integrations":s.zero_signal_integrations,
                        "positive_tag_integral":s.positive_tag_integral,"negative_tag_integral":s.negative_tag_integral,"peak_abs_tag":s.peak_abs_tag,
                        "min_signal":s.min_signal,"max_signal":s.max_signal,"residual_before":s.first_residual,"residual_after":s.last_residual,
                        "positive_actual":s.positive_actual,"negative_actual":s.negative_actual,"actual_changes":s.actual_changes,
                        "clipped":s.positive_clipped+s.negative_clipped,"rounding_error":s.positive_rounding_error+s.negative_rounding_error}));
                }
            }
        }
        for event in &audit.raw_events {
            match *event {
                ObservationEvent::SynapticArrival {
                    time,
                    synapse_id,
                    source,
                    target,
                    amplitude,
                } => {
                    let edge = self.edges.get_mut(&synapse_id.0).unwrap();
                    if phase == "recall" && self.branch == "learned" {
                        edge.recall_arrivals += 1;
                    }
                    // last_update is only linked during recall, after the training checkpoint.
                    // Runtime amplitudes are captured at emission; queued arrivals can precede learning.
                    let data = json!({"source":source.0,"target":target.0,"amplitude":amplitude,"changed_at_checkpoint":edge.changed,
                        "training_update_id":if phase.starts_with("recall") { edge.last_update } else { None }});
                    self.row(
                        time.0,
                        phase,
                        "transmission",
                        Some(target.0),
                        Some(synapse_id.0),
                        data,
                    )?;
                }
                ObservationEvent::SpikeEmitted(s) => {
                    self.row(
                        s.time.0,
                        phase,
                        "spike",
                        Some(s.neuron_id.0),
                        None,
                        json!({}),
                    )?;
                }
                ObservationEvent::NeuronAdvanced {
                    time,
                    target,
                    from,
                    potential_before,
                    potential_after,
                } => {
                    if potential_before != potential_after {
                        self.row(time.0,phase,"continuous_dynamics",Some(target.0),None,json!({"from_us":from.0,"potential_before":potential_before,"potential_after":potential_after}))?;
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
                    self.row(time.0,phase,"integration",Some(target.0),None,json!({"potential_before":potential_before,"potential_after":potential_after,"threshold":threshold,
                        "threshold_gap_before":f64::from(threshold)-f64::from(potential_before),
                        "threshold_gap_after_input":f64::from(threshold)-f64::from(potential_before+summed_input),"refractory":time<refractory_until,"refractory_until_us":refractory_until.0,"summed_input":summed_input,
                        "positive_input":positive_input,"negative_input":negative_input,"spiked":spiked}))?;
                }
                ObservationEvent::ExternalInput {
                    time,
                    target,
                    amplitude,
                } => {
                    self.row(
                        time.0,
                        phase,
                        "external_input",
                        Some(target.0),
                        None,
                        json!({"amplitude":amplitude}),
                    )?;
                }
                ObservationEvent::WeightChanged { .. } => {} // Already attributed to its learning hook above.
                ObservationEvent::HomeostasisChanged { .. } => {
                    return Err(error("unexpected homeostasis in fixed history assay"));
                }
            }
        }
        // Detect any future change to this assay's fixed topology contract.
        if model.network().synapse_count() != self.edges.len() {
            return Err(error("history assay changed topology"));
        }
        Ok(())
    }
    pub fn finish(mut self) -> Value {
        self.events
            .sort_by_key(|e| (e["time_us"].as_u64().unwrap(), e["id"].as_u64().unwrap()));
        let mut neurons = BTreeMap::<u64, Vec<usize>>::new();
        let mut synapses = BTreeMap::<u64, Vec<usize>>::new();
        for (index, event) in self.events.iter().enumerate() {
            let ids = [
                event["neuron_id"].as_u64(),
                event["source_id"].as_u64(),
                event["target_id"].as_u64(),
            ]
            .into_iter()
            .flatten()
            .collect::<BTreeSet<_>>();
            for id in ids {
                neurons.entry(id).or_default().push(index);
            }
            if let Some(id) = event["synapse_id"].as_u64() {
                synapses.entry(id).or_default().push(index);
            }
        }
        let used = self
            .edges
            .values()
            .filter(|s| s.changed && s.recall_arrivals > 0)
            .count();
        let edges = self.edges.into_iter().map(|(id,s)|json!({"synapse_id":id,"source_id":s.pre,"target_id":s.post,"weight_before":s.initial,"weight_after":s.final_weight,"weight_events":s.weight_events,
            "positive_actual":s.positive,"negative_actual":s.negative,"opposing_update_mass":s.positive.min(-s.negative),
            "requests":s.requests,"requests_without_change":s.no_change,"zero_signal_integrations":s.zero_signal,
            "changed_at_checkpoint":s.changed,"recall_arrivals":s.recall_arrivals})).collect::<Vec<_>>();
        let mechanisms = self.mechanisms.into_iter().map(|(name,(changes,positive,negative))|(name,json!({"weight_changes":changes,"positive_actual":positive,"negative_actual":negative}))).collect::<BTreeMap<_,_>>();
        json!({"protocol":"NERVA-Learning-History/1","complete":true,"selection":{"neurons":self.neurons,"synapses":self.selected_edges},
            "summary":{"observed_rows":self.observed,"weight_events":self.weight_events,"requests_without_change":self.no_change,
                "negative_factor_positive_tag_weight_changes":self.reversed_changes,"recall_used_changed_synapses":used,"event_counts":self.counts,"mechanisms":mechanisms},
            "neurons":self.neuron_metadata,"synapse_summary":edges,"events":self.events,"delayed_windows":self.windows,"neuron_index":neurons,"synapse_index":synapses,
            "ordering":"Chronological; equal-time events retain each stream's order only. Cross-stream order is not a causal ordering. IDs are stable identifiers, indices address sorted events.",
            "limits":"No-change delayed integrations are exact per-frame aggregates, not individual timestamped events. Pair-STDP requested delta is unavailable (null). training_update_id is a temporal link, never an emission-weight or spike-causality claim. Rest arrivals are distinguished from scored recall arrivals. Summary covers the whole network; detail covers selection only."})
    }
}
