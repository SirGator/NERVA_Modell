//! Read-only inputs and decisions at a native pre-commit maintenance boundary.
use super::{AreaWeights, DevelopmentPlan, Model, Neuron, NeuronId, Result, SynapseId, area};
use serde_json::{Value, json};
use std::collections::BTreeSet;

pub(super) struct Collector {
    pub(super) targets: BTreeSet<NeuronId>,
    edges: BTreeSet<SynapseId>,
    pub(super) last: Option<Value>,
}

impl Model {
    /// Observe selected Core cells and existing Core-to-Core synapses.
    /// Drain after each step. Only steps with a maintenance decision yield a record.
    pub fn enable_development_audit(
        &mut self,
        targets: BTreeSet<NeuronId>,
        edges: BTreeSet<SynapseId>,
    ) -> Result<()> {
        if targets.iter().any(|&cell| !self.is_core(cell))
            || edges.iter().any(|&id| {
                self.network()
                    .synapse(id)
                    .is_none_or(|edge| !self.is_core(edge.pre()) || !self.is_core(edge.post()))
            })
        {
            return Err("development audit requires existing Core cells and Core edges".into());
        }
        self.development_audit = Some(Collector {
            targets,
            edges,
            last: None,
        });
        Ok(())
    }

    pub fn take_development_audit(&mut self) -> Option<Value> {
        self.development_audit.as_mut()?.last.take()
    }

    pub fn development_audit_rules(&self) -> Value {
        let search = &self.formation.candidate_search;
        let pruning = self.pruning.config();
        json!({
            "growth_threshold": self.formation.growth_threshold,
            "radius": search.radius,
            "activity_weight": search.activity_weight,
            "temporal_weight": search.temporal_weight,
            "distance_weight": search.distance_weight,
            "temporal_tau_us": search.temporal_tau_us,
            "recency_tau_us": search.recency_tau_us,
            "min_candidate_score": search.min_candidate_score,
            "pruning_threshold": pruning.pruning_threshold,
            "min_below_duration_us": pruning.min_below_duration_us,
            "utility_tau_us": self.simulation.utility_dynamics().utility_tau_us,
        })
    }

    pub(super) fn development_affinity(&self, source: NeuronId, post: AreaWeights) -> f32 {
        let pre = self.network().neuron(source).expect("Core candidate");
        area::connection_affinity(
            self.area_field
                .at(pre.position())
                .expect("validated position"),
            post,
            pre.polarity(),
        )
        .expect("convex influences")
    }

    pub(super) fn observe_formation(
        &self,
        target: &Neuron,
        demand: f32,
        nearby: &[NeuronId],
        post: Option<AreaWeights>,
        eligible: Option<usize>,
        selected: Option<NeuronId>,
    ) -> Value {
        let now = self.simulation.current_time();
        let network = self.network();
        let incoming: BTreeSet<_> = network
            .incoming_synapse_ids(target.id())
            .iter()
            .map(|&edge| network.synapse(edge).expect("adjacency edge").pre())
            .collect();
        let neighbors: Vec<_> = nearby
            .iter()
            .copied()
            .filter(|&source| source != target.id() && !incoming.contains(&source))
            .map(|source| {
                let cell = network.neuron(source).expect("Core candidate");
                (
                    source.0,
                    target.position().distance_to(cell.position()),
                    cell.activity_trace_at(now),
                    cell.last_spike().map(|time| time.0),
                    post.map_or(1.0, |post| self.development_affinity(source, post)),
                )
            })
            .collect();
        json!({
            "target": target.id().0,
            "drive": demand,
            "target_trace": target.activity_trace_at(now),
            "target_last_spike_us": target.last_spike().map(|time| time.0),
            "neighbors": neighbors,
            "native_eligible": eligible,
            "selected": selected.map(|source| source.0),
        })
    }

    pub(super) fn observe_development(
        &self,
        formation: Vec<Value>,
        plan: &DevelopmentPlan,
    ) -> Result<Option<Value>> {
        let Some(audit) = &self.development_audit else {
            return Ok(None);
        };
        let now = self.simulation.current_time();
        let network = self.network();
        let tau = self.simulation.utility_dynamics().utility_tau_us;
        let pruning = audit
            .edges
            .iter()
            .filter_map(|id| network.synapse(*id))
            .map(|edge| {
                let polarity = network.neuron(edge.pre()).expect("presynaptic cell").polarity();
                let supported = (edge.is_plastic() || self.pruning.config().prune_non_plastic)
                    && self.simulation.utility_rule().supports(polarity);
                let selected = plan
                    .prunings
                    .binary_search_by_key(&edge.id(), |candidate| candidate.synapse_id)
                    .is_ok();
                Ok(json!({
                    "edge": edge.id().0,
                    "decayed_utility": edge.utility_at(now, tau)?,
                    "stored_utility": edge.utility(),
                    "updated_us": edge.utility_updated_at().map(|time| time.0),
                    "below_since_us": self.pruning.below_threshold_since(edge.id()).map(|time| time.0),
                    "supported": supported,
                    "selected": selected,
                }))
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Some(json!({
            "time_us": now.0,
            "formation": formation,
            "pruning": pruning,
        })))
    }
}
