//! Checkpoint-only controls for experiments on incoming structural growth.
use super::{Model, ModelKind, Neuron, NeuronId, Polarity, Result, SimTime};
use nerva::runtime::ObservationEvent;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub const EXPLORATION_WINDOW_US: u64 = 100_000;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ExploratoryFormation {
    #[default]
    Off,
    RecentExcitatory,
    NearestExcitatory,
    /// Rank recent candidates by decaying local causal lead evidence.
    LeadingExcitatory,
    /// Same evidence rule with source histories shifted later by 157 ms.
    LaggedLeadingExcitatory,
    /// Rank recent candidates by observed lead evidence minus lagged evidence.
    ContrastLeadingExcitatory,
}

impl ExploratoryFormation {
    fn uses_evidence(self) -> bool {
        matches!(
            self,
            Self::LeadingExcitatory
                | Self::LaggedLeadingExcitatory
                | Self::ContrastLeadingExcitatory
        )
    }
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
    /// Candidate RecentExcitatory would choose from the same eligible set.
    pub recent_source: u64,
    /// Decayed ranking score at formation, or zero for rules without evidence.
    pub chosen_evidence: f64,
    pub recent_evidence: f64,
    /// Decayed channel scores at formation. An unsampled channel contributes zero.
    pub chosen_observed: f64,
    pub chosen_lagged: f64,
    pub recent_observed: f64,
    pub recent_lagged: f64,
}

#[derive(Clone, Default, PartialEq)]
pub(super) struct PolicyState {
    pub(super) settings: DevelopmentPolicy,
    pub(super) used: BTreeSet<NeuronId>,
    evidence: Option<LeadEvidence>,
}

pub(super) struct Choice {
    pub(super) source: NeuronId,
    pub(super) source_last: Option<SimTime>,
    pub(super) target_last: Option<SimTime>,
    pub(super) recent_candidates: usize,
    pub(super) recent_source: u64,
    pub(super) chosen_evidence: f64,
    pub(super) recent_evidence: f64,
    pub(super) chosen_observed: f64,
    pub(super) chosen_lagged: f64,
    pub(super) recent_observed: f64,
    pub(super) recent_lagged: f64,
}

const CONTROL_SHIFT_US: u64 = 157_000;
const MAX_LEAD_US: u64 = 75_000;
const SAMPLE_INTERVAL_US: u64 = 10_000;
const EVIDENCE_TAU_US: f64 = 2_000_000.;

#[derive(Clone, Default, PartialEq)]
struct LeadEvidence {
    activated_us: u64,
    histories: BTreeMap<NeuronId, VecDeque<u64>>,
    last_sample: BTreeMap<NeuronId, u64>,
    observed: LeadScores,
    lagged: LeadScores,
}

#[derive(Clone, Default, PartialEq)]
struct LeadScores(BTreeMap<(NeuronId, NeuronId), (f64, u64)>);

impl LeadScores {
    fn score(&self, source: NeuronId, target: NeuronId, now: u64) -> f64 {
        self.0.get(&(source, target)).map_or(0., |&(value, at)| {
            value * (-(now.saturating_sub(at) as f64) / EVIDENCE_TAU_US).exp()
        })
    }

    fn sample(
        &mut self,
        target: NeuronId,
        sources: &[NeuronId],
        now: u64,
        lookup: u64,
        histories: &BTreeMap<NeuronId, VecDeque<u64>>,
    ) {
        for &source in sources {
            let spike = histories.get(&source).and_then(|history| {
                history
                    .iter()
                    .rev()
                    .copied()
                    .find(|&at| at <= lookup.saturating_sub(1_000))
            });
            let Some(age) = spike.and_then(|at| lookup.checked_sub(at)) else {
                continue;
            };
            let contribution = match age {
                1_000..=15_000 => 3.,
                15_001..=40_000 => 2.,
                40_001..=75_000 => 1.,
                _ => continue,
            };
            let value = self.score(source, target, now) + contribution;
            self.0.insert((source, target), (value, now));
        }
    }
}

impl LeadEvidence {
    fn spike(&mut self, source: NeuronId, now: u64) {
        let history = self.histories.entry(source).or_default();
        history.push_back(now);
        while history
            .front()
            .is_some_and(|&at| now.saturating_sub(at) > CONTROL_SHIFT_US + MAX_LEAD_US)
        {
            history.pop_front();
        }
    }

    fn sample_due(&self, target: NeuronId, now: u64) -> bool {
        now.saturating_sub(self.activated_us) >= CONTROL_SHIFT_US
            && !self
                .last_sample
                .get(&target)
                .is_some_and(|&at| now.saturating_sub(at) < SAMPLE_INTERVAL_US)
    }

    fn sample(
        &mut self,
        target: NeuronId,
        sources: &[NeuronId],
        now: u64,
        formation: ExploratoryFormation,
    ) {
        if !self.sample_due(target, now) {
            return;
        }
        self.last_sample.insert(target, now);
        use ExploratoryFormation::{
            ContrastLeadingExcitatory, LaggedLeadingExcitatory, LeadingExcitatory,
        };
        if matches!(formation, LeadingExcitatory | ContrastLeadingExcitatory) {
            self.observed
                .sample(target, sources, now, now, &self.histories);
        }
        if matches!(
            formation,
            LaggedLeadingExcitatory | ContrastLeadingExcitatory
        ) {
            self.lagged.sample(
                target,
                sources,
                now,
                now.saturating_sub(CONTROL_SHIFT_US),
                &self.histories,
            );
        }
    }

    fn score(
        &self,
        source: NeuronId,
        target: NeuronId,
        now: u64,
        formation: ExploratoryFormation,
    ) -> f64 {
        match formation {
            ExploratoryFormation::LeadingExcitatory => self.observed.score(source, target, now),
            ExploratoryFormation::LaggedLeadingExcitatory => self.lagged.score(source, target, now),
            ExploratoryFormation::ContrastLeadingExcitatory => {
                self.observed.score(source, target, now) - self.lagged.score(source, target, now)
            }
            _ => 0.,
        }
    }
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
        if formation != self.development_policy.settings.formation {
            // A new temporal relation starts a fresh evidence interval. Neural
            // state and the per-target formation allowance remain unchanged.
            self.development_policy.evidence = formation.uses_evidence().then(|| LeadEvidence {
                activated_us: self.next_start,
                ..LeadEvidence::default()
            });
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
            evidence: policy.formation.uses_evidence().then(|| LeadEvidence {
                activated_us: self.next_start,
                ..LeadEvidence::default()
            }),
        };
        Ok(())
    }

    pub(super) fn lead_evidence_active(&self) -> bool {
        self.config.development && self.development_policy.settings.formation.uses_evidence()
    }

    /// Consume the runtime's ordered event clock, never the frame-end neuron state.
    pub(super) fn observe_lead_evidence(&mut self, event: &ObservationEvent) {
        if !self.lead_evidence_active() {
            return;
        }
        match *event {
            ObservationEvent::SpikeEmitted(spike)
                if self.is_core(spike.neuron_id)
                    && self.network().neuron(spike.neuron_id).unwrap().polarity()
                        == Polarity::Excitatory =>
            {
                self.development_policy
                    .evidence
                    .as_mut()
                    .expect("active evidence")
                    .spike(spike.neuron_id, spike.time.0);
            }
            ObservationEvent::NeuronIntegrated {
                time,
                target,
                summed_input,
                refractory_until,
                ..
            } if self.is_core(target) && summed_input > 0. && time >= refractory_until => {
                if !self
                    .development_policy
                    .evidence
                    .as_ref()
                    .expect("active evidence")
                    .sample_due(target, time.0)
                {
                    return;
                }
                let cell = self.network().neuron(target).expect("integration target");
                let sources: Vec<_> = self
                    .core_index
                    .within(cell.position(), self.formation.candidate_search.radius)
                    .into_iter()
                    .filter(|&id| {
                        id != target
                            && self.network().neuron(id).unwrap().polarity() == Polarity::Excitatory
                    })
                    .collect();
                let formation = self.development_policy.settings.formation;
                self.development_policy
                    .evidence
                    .as_mut()
                    .expect("active evidence")
                    .sample(target, &sources, time.0, formation);
            }
            _ => {}
        }
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
        let recency = |a: &Neuron, b: &Neuron| {
            a.last_spike()
                .cmp(&b.last_spike())
                .then_with(|| nearest(b).total_cmp(&nearest(a)))
                .then_with(|| b.id().cmp(&a.id()))
        };
        let recent_source = active
            .clone()
            .max_by(|a, b| recency(a, b))
            .expect("recent candidates")
            .id();
        let evidence = self.development_policy.evidence.as_ref();
        let score = |source| evidence.map_or(0., |e| e.score(source, target.id(), now.0, rule));
        let source = match rule {
            ExploratoryFormation::LeadingExcitatory
            | ExploratoryFormation::LaggedLeadingExcitatory
            | ExploratoryFormation::ContrastLeadingExcitatory => active.max_by(|a, b| {
                score(a.id())
                    .total_cmp(&score(b.id()))
                    .then_with(|| recency(a, b))
            }),
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
        let channels = |source| {
            evidence.map_or((0., 0.), |e| {
                (
                    e.observed.score(source, target.id(), now.0),
                    e.lagged.score(source, target.id(), now.0),
                )
            })
        };
        let (chosen_observed, chosen_lagged) = channels(source.id());
        let (recent_observed, recent_lagged) = channels(recent_source);
        Some(Choice {
            source: source.id(),
            source_last: source.last_spike(),
            target_last: target.last_spike(),
            recent_candidates,
            recent_source: recent_source.0,
            chosen_evidence: score(source.id()),
            recent_evidence: score(recent_source),
            chosen_observed,
            chosen_lagged,
            recent_observed,
            recent_lagged,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ModelConfig;
    use nerva_qemu::capture::Frame;

    #[test]
    fn evidence_is_causal_bounded_and_samples_once_per_target_interval() {
        let source = NeuronId(1);
        let target = NeuronId(2);
        let mut evidence = LeadEvidence::default();
        let observed = ExploratoryFormation::LeadingExcitatory;
        // Warmup is identical for the treatment and delayed control.
        evidence.spike(source, 140_000);
        evidence.sample(target, &[source], 150_000, observed);
        assert_eq!(evidence.score(source, target, 150_000, observed), 0.);
        evidence.spike(source, 159_500); // Too recent at the first sample.
        evidence.sample(target, &[source], 160_000, observed);
        assert_eq!(evidence.score(source, target, 160_000, observed), 2.);
        evidence.sample(target, &[source], 169_999, observed);
        assert_eq!(evidence.last_sample[&target], 160_000);
        evidence.sample(target, &[source], 170_000, observed);
        let expected = 2. * (-10_000. / EVIDENCE_TAU_US).exp() + 3.;
        assert_eq!(evidence.score(source, target, 170_000, observed), expected);
        assert_eq!(
            evidence.score(source, target, 2_170_000, observed),
            expected * (-1_f64).exp()
        );
        // Future-only spikes cannot create causal evidence.
        let mut future = LeadEvidence::default();
        future.spike(source, 160_001);
        future.sample(target, &[source], 160_000, observed);
        assert_eq!(future.score(source, target, 160_000, observed), 0.);
        future.spike(source, 400_000);
        assert_eq!(future.histories[&source].len(), 1);
    }

    #[test]
    fn lagged_control_uses_identical_history_shifted_later() {
        let source = NeuronId(1);
        let target = NeuronId(2);
        let observed = ExploratoryFormation::LeadingExcitatory;
        let lagged = ExploratoryFormation::LaggedLeadingExcitatory;
        let mut actual = LeadEvidence::default();
        for time in [4_000, 144_000, 155_000] {
            actual.spike(source, time);
        }
        let mut delayed = actual.clone();
        actual.sample(target, &[source], 161_000, observed);
        delayed.sample(target, &[source], 161_000, lagged);
        assert_eq!(actual.score(source, target, 161_000, observed), 3.);
        // 4ms lookup clock, source at 4ms is simultaneous and ineligible.
        assert_eq!(delayed.score(source, target, 161_000, lagged), 0.);
        delayed.sample(target, &[source], 171_000, lagged);
        assert_eq!(delayed.score(source, target, 171_000, lagged), 3.);
        let mut literal_shift = LeadEvidence::default();
        for time in [161_000, 301_000, 312_000] {
            literal_shift.spike(source, time);
        }
        literal_shift.sample(target, &[source], 171_000, observed);
        assert!(delayed.lagged == literal_shift.observed);
    }

    #[test]
    fn contrast_uses_both_channels_on_the_same_samples_and_keeps_negative_scores() {
        use ExploratoryFormation::{
            ContrastLeadingExcitatory, LaggedLeadingExcitatory, LeadingExcitatory,
        };
        let target = NeuronId(1);
        let both = NeuronId(2);
        let older = NeuronId(3);
        let silent = NeuronId(4);
        let sources = [both, older, silent];
        let mut history = LeadEvidence::default();
        history.spike(both, 30_000);
        history.spike(older, 30_000);
        history.spike(both, 160_000);
        let mut observed = history.clone();
        let mut lagged = history.clone();
        let mut contrast = history;
        for sample in [156_999, 200_000, 209_999, 210_000] {
            observed.sample(target, &sources, sample, LeadingExcitatory);
            lagged.sample(target, &sources, sample, LaggedLeadingExcitatory);
            contrast.sample(target, &sources, sample, ContrastLeadingExcitatory);
        }
        assert_eq!(contrast.last_sample[&target], 210_000);
        assert_eq!(contrast.last_sample, observed.last_sample);
        assert_eq!(contrast.last_sample, lagged.last_sample);
        assert!(contrast.observed == observed.observed);
        assert!(contrast.lagged == lagged.lagged);
        let now = 2_210_000;
        for source in sources {
            assert_eq!(
                contrast.score(source, target, now, ContrastLeadingExcitatory),
                observed.score(source, target, now, LeadingExcitatory)
                    - lagged.score(source, target, now, LaggedLeadingExcitatory),
            );
        }
        assert!(contrast.score(both, target, now, ContrastLeadingExcitatory) < 0.);
        assert!(
            contrast.score(older, target, now, ContrastLeadingExcitatory)
                < contrast.score(both, target, now, ContrastLeadingExcitatory)
        );
        assert_eq!(
            contrast.score(silent, target, now, ContrastLeadingExcitatory),
            0.
        );
    }

    #[test]
    fn frozen_configuration_and_live_mode_switch_preserve_neural_state_and_cap() {
        let mut model = Model::with_sensor(
            ModelConfig {
                processing_neurons: 16,
                learning: false,
                development: false,
                homeostasis: false,
                ..ModelConfig::default()
            },
            super::super::SensorConfig {
                resolution: 8,
                ..super::super::SensorConfig::default()
            },
        )
        .unwrap();
        model
            .diagnostic_development_policy(DevelopmentPolicy {
                formation: ExploratoryFormation::LeadingExcitatory,
                pruning: false,
            })
            .unwrap();
        assert_eq!(
            model
                .development_policy
                .evidence
                .as_ref()
                .unwrap()
                .activated_us,
            1
        );
        model
            .set_plasticity(super::super::PlasticitySettings {
                learning: true,
                development: true,
                homeostasis: false,
            })
            .unwrap();
        let target = model.processing[0];
        model.development_policy.used.insert(target);
        model
            .development_policy
            .evidence
            .as_mut()
            .unwrap()
            .observed
            .0
            .insert((target, target), (4., 1));
        let before = model.diagnostic_fork().unwrap();
        model
            .diagnostic_switch_exploratory_formation(ExploratoryFormation::LeadingExcitatory)
            .unwrap();
        assert!(model.same_replay_state(&before));
        model
            .diagnostic_switch_exploratory_formation(ExploratoryFormation::LaggedLeadingExcitatory)
            .unwrap();
        assert!(
            model
                .development_policy
                .evidence
                .as_ref()
                .unwrap()
                .observed
                .0
                .is_empty()
        );
        assert!(model.development_policy.used.contains(&target));
        assert!(!model.development_policy.settings.pruning);
        assert!(model.simulation.same_replay_state(&before.simulation));
        model
            .diagnostic_switch_exploratory_formation(ExploratoryFormation::Off)
            .unwrap();
        assert!(model.development_policy.evidence.is_none());
    }

    #[test]
    fn only_positive_nonrefractory_integration_batches_sample_evidence() {
        let mut model = Model::with_sensor(
            ModelConfig {
                processing_neurons: 16,
                ..ModelConfig::default()
            },
            super::super::SensorConfig {
                resolution: 8,
                ..super::super::SensorConfig::default()
            },
        )
        .unwrap();
        model
            .diagnostic_switch_exploratory_formation(ExploratoryFormation::LeadingExcitatory)
            .unwrap();
        model.formation.candidate_search.radius = 10.;
        let target = model.processing[0];
        let source = model
            .processing
            .iter()
            .copied()
            .find(|&id| {
                id != target
                    && model.network().neuron(id).unwrap().polarity() == Polarity::Excitatory
            })
            .unwrap();
        model
            .development_policy
            .evidence
            .as_mut()
            .unwrap()
            .spike(source, 190_000);
        let event = |summed_input, refractory_until| ObservationEvent::NeuronIntegrated {
            time: SimTime(200_000),
            target,
            potential_before: 0.,
            potential_after: 0.,
            threshold: 1.,
            refractory_until: SimTime(refractory_until),
            summed_input,
            positive_input: 1.,
            negative_input: -2.,
            spiked: false,
        };
        model.observe_lead_evidence(&event(-1., 0));
        model.observe_lead_evidence(&event(0., 0));
        model.observe_lead_evidence(&event(1., 200_001));
        assert!(
            model
                .development_policy
                .evidence
                .as_ref()
                .unwrap()
                .last_sample
                .is_empty()
        );
        model.observe_lead_evidence(&event(1., 200_000));
        assert_eq!(
            model.development_policy.evidence.as_ref().unwrap().score(
                source,
                target,
                200_000,
                ExploratoryFormation::LeadingExcitatory
            ),
            3.
        );
    }

    #[test]
    fn leading_forks_replay_exactly_and_frozen_probes_do_not_learn() {
        for formation in [
            ExploratoryFormation::LeadingExcitatory,
            ExploratoryFormation::LaggedLeadingExcitatory,
            ExploratoryFormation::ContrastLeadingExcitatory,
        ] {
            let mut model = Model::with_sensor(
                ModelConfig {
                    processing_neurons: 16,
                    ..ModelConfig::default()
                },
                super::super::SensorConfig {
                    resolution: 8,
                    ..super::super::SensorConfig::default()
                },
            )
            .unwrap();
            model
                .diagnostic_switch_exploratory_formation(formation)
                .unwrap();
            let white = Frame::new(64, 64, vec![255; 64 * 64 * 3]).unwrap();
            for _ in 0..4 {
                model.step(&white).unwrap();
            }
            let mut fork = model.diagnostic_fork().unwrap();
            fork.diagnostic_switch_exploratory_formation(formation)
                .unwrap();
            assert!(model.same_replay_state(&fork));
            assert_eq!(model.step(&white).unwrap(), fork.step(&white).unwrap());
            assert!(model.same_replay_state(&fork));
            // The evidence state participates in replay equality.
            fork.development_policy
                .evidence
                .as_mut()
                .unwrap()
                .spike(NeuronId(999), 300_000);
            assert!(!model.same_replay_state(&fork));
            model
                .set_plasticity(super::super::PlasticitySettings {
                    learning: false,
                    development: false,
                    homeostasis: false,
                })
                .unwrap();
            let evidence = model.development_policy.evidence.clone();
            model.set_dynamics_auditing(true);
            model.step(&white).unwrap();
            assert!(model.development_policy.evidence == evidence);
            assert!(model.dynamics_auditing_requested);
        }
    }

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
