//! Checkpoint-only controls for experiments on incoming structural growth.
use super::{Model, ModelKind, Neuron, NeuronId, Polarity, Result, SimTime};
use std::collections::BTreeSet;

pub const EXPLORATION_WINDOW_US: u64 = 100_000;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ExploratoryFormation {
    #[default]
    Off,
    RecentExcitatory,
    NearestExcitatory,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DevelopmentPolicy {
    pub formation: ExploratoryFormation,
    pub pruning: bool,
}

impl Default for DevelopmentPolicy {
    fn default() -> Self {
        Self {
            formation: ExploratoryFormation::Off,
            pruning: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Exploration {
    pub edge: u64,
    pub time_us: u64,
    pub source_last_us: Option<u64>,
    pub target_last_us: Option<u64>,
    pub recent_candidates: usize,
    pub drive: f32,
}

#[derive(Default, PartialEq, Eq)]
pub(super) struct PolicyState {
    pub(super) settings: DevelopmentPolicy,
    pub(super) used: BTreeSet<NeuronId>,
}

pub(super) struct Choice {
    pub(super) source: NeuronId,
    pub(super) source_last: Option<SimTime>,
    pub(super) target_last: Option<SimTime>,
    pub(super) recent_candidates: usize,
}

impl Model {
    /// Configure an experimental policy at a frozen checkpoint, starting a
    /// new interval with one exploratory formation available per target.
    pub fn diagnostic_development_policy(&mut self, policy: DevelopmentPolicy) -> Result<()> {
        let settings = self.plasticity();
        if settings.learning
            || settings.development
            || settings.homeostasis
            || self.config.kind != ModelKind::Recurrent
        {
            return Err("development policy selection requires a frozen recurrent model".into());
        }
        self.development_policy = PolicyState {
            settings: policy,
            used: BTreeSet::new(),
        };
        Ok(())
    }

    pub(super) fn exploratory_source(
        &self,
        target: &Neuron,
        nearby: &[NeuronId],
        now: SimTime,
    ) -> Option<Choice> {
        let rule = self.development_policy.settings.formation;
        let recent = |last: Option<SimTime>| {
            last.and_then(|time| now.duration_since(time))
                .is_some_and(|age| age <= EXPLORATION_WINDOW_US)
        };
        if rule == ExploratoryFormation::Off
            || self.development_policy.used.contains(&target.id())
            || recent(target.last_spike())
        {
            return None;
        }
        let network = self.network();
        let incoming = network.incoming_synapse_ids(target.id());
        let sources: Vec<_> = nearby
            .iter()
            .filter_map(|&id| {
                let source = network.neuron(id).expect("local Core source");
                (id != target.id()
                    && source.polarity() == Polarity::Excitatory
                    && incoming
                        .iter()
                        .all(|&edge| network.synapse(edge).expect("adjacency").pre() != id))
                .then_some(source)
            })
            .collect();
        let active = sources.iter().filter(|cell| recent(cell.last_spike()));
        let recent_candidates = active.clone().count();
        if recent_candidates == 0 {
            return None;
        }
        let nearest = |cell: &Neuron| target.position().distance_to(cell.position());
        let source = match rule {
            ExploratoryFormation::RecentExcitatory => active.max_by(|a, b| {
                a.last_spike()
                    .cmp(&b.last_spike())
                    .then_with(|| nearest(b).total_cmp(&nearest(a)))
                    .then_with(|| b.id().cmp(&a.id()))
            }),
            ExploratoryFormation::NearestExcitatory => sources.iter().min_by(|a, b| {
                nearest(a)
                    .total_cmp(&nearest(b))
                    .then_with(|| a.id().cmp(&b.id()))
            }),
            ExploratoryFormation::Off => None,
        }?;
        Some(Choice {
            source: source.id(),
            source_last: source.last_spike(),
            target_last: target.last_spike(),
            recent_candidates,
        })
    }
}
