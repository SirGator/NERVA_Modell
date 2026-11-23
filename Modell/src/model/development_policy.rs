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

#[derive(Clone, Default, PartialEq, Eq)]
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
    /// Switch formation at a synchronous observation boundary without resetting
    /// per-target exploration history or touching any plasticity clocks.
    pub fn diagnostic_switch_exploratory_formation(
        &mut self,
        formation: ExploratoryFormation,
    ) -> Result<()> {
        if self.config.kind != ModelKind::Recurrent || !self.config.development {
            return Err("formation switching requires active recurrent development".into());
        }
        self.development_policy.settings.formation = formation;
        Ok(())
    }

    /// Suspend graph development at a synchronous observation boundary while
    /// preserving learning, homeostasis and their existing scheduler bookings.
    pub fn diagnostic_pause_development(&mut self) -> Result<()> {
        if self.config.kind != ModelKind::Recurrent {
            return Err("development suspension requires a recurrent model".into());
        }
        self.config.development = false;
        Ok(())
    }

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ModelConfig;
    use nerva_qemu::capture::Frame;

    #[test]
    fn live_controls_preserve_history_and_all_other_replay_state() {
        let config = ModelConfig {
            processing_neurons: 16,
            ..ModelConfig::default()
        };
        let mut model = Model::from_config(config.clone()).unwrap();
        let mut native = Model::from_config(config).unwrap();
        let white = Frame::new(64, 64, vec![255; 64 * 64 * 3]).unwrap();
        assert_eq!(model.step(&white).unwrap(), native.step(&white).unwrap());
        let target = model.processing[0];
        for branch in [&mut model, &mut native] {
            branch.development_policy.used.insert(target);
            branch.development_policy.settings.pruning = false;
        }
        let plasticity = model.plasticity();
        model
            .diagnostic_switch_exploratory_formation(ExploratoryFormation::RecentExcitatory)
            .unwrap();
        assert!(model.development_policy.used.contains(&target));
        assert!(!model.development_policy.settings.pruning);
        assert_eq!(model.plasticity(), plasticity);
        model
            .diagnostic_switch_exploratory_formation(ExploratoryFormation::Off)
            .unwrap();
        assert!(model.same_replay_state(&native));
        model.diagnostic_pause_development().unwrap();
        model.diagnostic_pause_development().unwrap();
        assert_eq!(model.plasticity().learning, plasticity.learning);
        assert_eq!(model.plasticity().homeostasis, plasticity.homeostasis);
        assert!(!model.plasticity().development);
        assert!(
            model
                .diagnostic_switch_exploratory_formation(ExploratoryFormation::Off)
                .is_err()
        );
        model.config.development = true;
        assert!(model.same_replay_state(&native));
        assert_eq!(model.step(&white).unwrap(), native.step(&white).unwrap());
        assert!(model.same_replay_state(&native));
    }
}
