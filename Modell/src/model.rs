//! Spatial recurrent model assembled from NERVA rules; direct wiring remains a control.
//! Diagnostic sensory modes derive features from the current frame only.
//! No task policy, reward or teaching pulse.

use nerva::{
    config::{HomeostasisConfig, LearningConfig, NeuronConfig, RuntimeConfig},
    core::{Network, Neuron, NeuronId, NeuronRole, Polarity, SimTime, Synapse, SynapseId},
    development::{
        CandidateSearchConfig, DevelopmentPlan, FormationConfig, PlannedFormation, PruningConfig,
        PruningController, local_candidates,
    },
    io::{ChannelId, ReceptorSignal},
    learning::{ExcitatoryCausalUtility, LocalHomeostasis, PairStdp, UtilityDynamicsConfig},
    math::Position3D,
    nerves::{Fiber, FiberDirection, FiberId, Mapping, Routing},
    primitives::Weight,
    roots::{MotorOutput, MotorRoot, RootChannel, RootId, SensoryRoot},
    runtime::{ObservationEvent, Simulation},
};
use nerva_qemu::{
    Result, capture::Frame, keyboard::Key, motor_decoder::MotorDecoder, signals::MotorPulse,
    vision_encoder::Encoding,
};

pub mod audit;
pub mod config;
pub(crate) mod random;
pub mod sensory;
mod topology;
pub use config::{HomeostasisScope, ModelConfig, ModelKind};
pub use sensory::{SensorConfig, SensorMode, SpikeCoding};
use sensory::{SensorEncoder, SpikeSource};

pub const WINDOW_US: u64 = 50_000;

pub type SynapseFormation = (u64, u64, u64);

#[derive(Default)]
struct TopologyChanges {
    formed: Vec<SynapseFormation>,
    pruned: Vec<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyboardLayout {
    Full,
    Wasd,
    Ad,
}

impl KeyboardLayout {
    pub fn keys(self) -> &'static [Key] {
        match self {
            Self::Full => Key::ALL,
            Self::Wasd => &Key::ALL[..5],
            Self::Ad => &[Key::A, Key::D],
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Full => "de-iso105",
            Self::Wasd => "wasd",
            Self::Ad => "ad",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlasticitySettings {
    pub learning: bool,
    pub development: bool,
    pub homeostasis: bool,
}

pub struct Model {
    simulation: Simulation<PairStdp>,
    sensory_root: SensoryRoot,
    motor_root: MotorRoot,
    mapping: Mapping,
    encoder: SensorEncoder,
    spike_source: SpikeSource,
    sensory_gain: f32,
    decoder: MotorDecoder,
    next_start: u64,
    pending_motor: Vec<MotorOutput>,
    config: ModelConfig,
    processing: Vec<NeuronId>,
    action_copies: Vec<NeuronId>,
    executed_keys: Vec<Key>,
    pruning: PruningController,
    formation: FormationConfig,
    next_synapse: u64,
    motor_neurons: Vec<NeuronId>,
    next_development_us: u64,
    audit: Option<audit::Collector>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MotorInputStats {
    pub positive_arrivals: usize,
    pub negative_arrivals: usize,
    pub positive_amplitude: f64,
    pub negative_amplitude: f64,
    /// Nonzero arrivals, matching the positive/negative arrival counters.
    pub arrivals_during_refractory: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Step {
    pub action: Option<Key>,
    pub keys: Vec<Key>,
    pub motor_counts: Vec<usize>,
    pub motor_events: Vec<MotorPulse>,
    pub motor_inputs: Vec<MotorInputStats>,
    pub start_us: u64,
    pub end_us: u64,
    pub sensory_spikes: usize,
    pub motor_spikes: usize,
    pub weight_changes: usize,
    pub core_counts: Vec<usize>,
    pub core_events: Vec<(u64, u64)>,
    pub feedback_arrivals: usize,
    pub action_copy_arrivals: usize,
    pub homeostasis_changes: usize,
    pub formed: Vec<SynapseFormation>,
    pub pruned: Vec<u64>,
    pub core_synapses: usize,
}

impl Model {
    pub fn new(seed: u64, mode: Encoding, learning_enabled: bool) -> Result<Self> {
        Self::with_keyboard(seed, mode, learning_enabled, KeyboardLayout::Full)
    }

    pub fn with_keyboard(
        seed: u64,
        mode: Encoding,
        learning_enabled: bool,
        layout: KeyboardLayout,
    ) -> Result<Self> {
        let mut config = ModelConfig {
            seed,
            encoding: mode,
            keyboard: layout,
            ..ModelConfig::default()
        };
        if !learning_enabled {
            config = config.frozen();
        }
        Self::from_config(config)
    }

    pub fn from_config(config: ModelConfig) -> Result<Self> {
        let sensor = SensorConfig {
            coding: if config.encoding == Encoding::Change {
                SpikeCoding::OnOff
            } else {
                SpikeCoding::Poisson
            },
            ..SensorConfig::default()
        };
        Self::with_sensor(config, sensor)
    }

    /// Assemble the same neural mechanisms with an explicitly selected,
    /// frame-only sensory front end. No world state or labels enter this API.
    pub fn with_sensor(mut config: ModelConfig, sensor: SensorConfig) -> Result<Self> {
        config.validate()?;
        sensor.validate()?;
        config.encoding = if sensor.coding == SpikeCoding::OnOff {
            Encoding::Change
        } else {
            Encoding::Brightness
        };
        if config.kind == ModelKind::Direct {
            config.development = false;
            config.homeostasis = false;
            config.feedback = false;
        }
        let encoder = SensorEncoder::new(sensor)?;
        let count = sensor.channels() as u64;
        let (network, mapping, sensory_root, motor_root, processing, action_copies, next_synapse) =
            if config.kind == ModelKind::Recurrent {
                let topology = topology::build(&config, &sensor)?;
                (
                    topology.network,
                    topology.mapping,
                    topology.sensory_root,
                    topology.motor_root,
                    topology.processing,
                    topology.action_copies,
                    topology.next_synapse,
                )
            } else {
                let (network, mapping, sensory_root, motor_root) = crate::direct::build_direct(
                    config.seed,
                    count,
                    config.keyboard.keys(),
                    config.motor_weight_scale,
                    config.motor_refractory_us,
                )?;
                let cursor = network.synapse_count() as u64;
                (
                    network,
                    mapping,
                    sensory_root,
                    motor_root,
                    Vec::new(),
                    Vec::new(),
                    cursor,
                )
            };
        let motor_neurons = network
            .neurons()
            .filter(|neuron| neuron.role() == Some(NeuronRole::Motor))
            .map(|neuron| neuron.id())
            .collect();
        let rule = PairStdp::try_from_config(&LearningConfig {
            // Runtime owns the on/off switch, including freeze and resume.
            enabled: true,
            a_plus: 0.0001,
            a_minus: 0.00012,
            tau_plus_us: config.stdp_tau_us,
            tau_minus_us: config.stdp_tau_us,
            max_weight: if config.kind == ModelKind::Recurrent {
                2.0
            } else {
                0.2 * (config.keyboard.keys().len() as f32 / 5.0).sqrt()
            },
            ..LearningConfig::default()
        })
        .map_err(|error| format!("STDP: {error:?}"))?;
        let mut simulation = if config.kind == ModelKind::Recurrent {
            Simulation::with_utility_rule(
                network,
                rule,
                Box::new(ExcitatoryCausalUtility),
                RuntimeConfig::default(),
                3.0,
            )?
            .with_utility_dynamics(UtilityDynamicsConfig {
                eligibility_tau_us: 20_000.0,
                eta: 0.1,
                utility_tau_us: 500_000.0,
            })?
        } else {
            Simulation::new(network, rule, RuntimeConfig::default(), 1.0)?
        };
        {
            let homeostasis = HomeostasisConfig {
                enabled: config.homeostasis,
                update_interval_us: 100_000,
                target_rate_hz: 8.0,
                target_input_rate: 50.0,
                intrinsic_adjustment_rate: 0.5,
                structural_adjustment_rate: 0.002,
                min_intrinsic_current: -10.0,
                max_intrinsic_current: 20.0,
                min_structural_drive: 0.0,
                max_structural_drive: 3.0,
            };
            homeostasis.validate()?;
            if config.homeostasis_scope != HomeostasisScope::All {
                let population = simulation
                    .network()
                    .neuron_ids()
                    .filter(|id| {
                        processing.contains(id)
                            || (config.homeostasis_scope == HomeostasisScope::CoreSensor
                                && id.0 < count)
                    })
                    .collect();
                simulation.set_homeostasis_population(Some(population))?;
            }
            simulation = simulation.with_homeostasis(LocalHomeostasis::new(&homeostasis))?;
        }
        simulation.set_learning_enabled(config.learning);
        let formation = FormationConfig {
            growth_threshold: 0.5,
            initial_weight: Weight::new(0.25)?,
            delay_us: 1_000,
            plastic: true,
            drive_consumption: 0.5,
            candidate_search: CandidateSearchConfig {
                radius: 0.25,
                min_candidate_score: 0.01,
                temporal_tau_us: 20_000.0,
                recency_tau_us: 100_000.0,
                ..CandidateSearchConfig::default()
            },
        };
        let pruning = PruningController::new(PruningConfig {
            pruning_threshold: 0.01,
            min_below_duration_us: config.pruning_min_below_us,
            prune_non_plastic: false,
        })?;
        Ok(Self {
            simulation,
            sensory_root,
            motor_root,
            mapping,
            encoder,
            spike_source: SpikeSource::new(sensor.coding, config.seed),
            sensory_gain: 1.0,
            decoder: MotorDecoder::new(WINDOW_US, 1)?.with_timing_ties(config.motor_timing),
            next_start: 1,
            pending_motor: Vec::new(),
            config,
            processing,
            action_copies,
            executed_keys: Vec::new(),
            pruning,
            formation,
            next_synapse,
            motor_neurons,
            next_development_us: 100_000,
            audit: None,
        })
    }

    /// Enable read-only audits without changing model dynamics. Drain after
    /// each step; an undrained record is replaced by the next observation.
    pub fn enable_audit(&mut self, options: audit::AuditOptions) {
        self.audit = Some(audit::Collector::new(
            options,
            self.processing.clone(),
            self.action_copies.clone(),
            self.motor_neurons.clone(),
            self.encoder.config().channels() as u64,
        ));
    }

    pub fn take_audit(&mut self) -> Option<audit::InputAudit> {
        self.audit
            .as_mut()
            .and_then(|collector| collector.last.take())
    }

    /// Fixed sensory rate gain, chosen before any neural observation. It does
    /// not change topology, weights or learning rules.
    pub fn set_sensory_gain(&mut self, gain: f32) -> Result<()> {
        if self.next_start != 1 || !gain.is_finite() || gain <= 0.0 {
            return Err(
                "sensory gain must be finite and positive, before the first observation".into(),
            );
        }
        self.sensory_gain = gain;
        Ok(())
    }

    pub fn next_observation_us(&self) -> u64 {
        self.next_start
    }

    /// Exact replay comparison for this model's fixed CPU/stateless utility
    /// implementations. Includes queued neural inputs, STDP-owned traces,
    /// sensory generator phase, decoder and execution clocks. Read-only audit
    /// collectors are excluded. Diagnostic string comparisons may conservatively
    /// reject equal hash-map states; M6 uses the ordered deterministic generator.
    pub fn same_replay_state(&self, other: &Self) -> bool {
        self.config == other.config
            && self.simulation.same_replay_state(&other.simulation)
            && self.next_start == other.next_start
            && self.sensory_gain == other.sensory_gain
            && self.pending_motor == other.pending_motor
            && self.executed_keys == other.executed_keys
            && self.next_synapse == other.next_synapse
            && self.next_development_us == other.next_development_us
            && self.processing == other.processing
            && self.action_copies == other.action_copies
            && self.motor_neurons == other.motor_neurons
            && format!(
                "{:?}",
                (
                    &self.encoder,
                    &self.spike_source,
                    &self.decoder,
                    &self.pruning,
                    &self.formation
                )
            ) == format!(
                "{:?}",
                (
                    &other.encoder,
                    &other.spike_source,
                    &other.decoder,
                    &other.pruning,
                    &other.formation
                )
            )
            && self.sensory_root == other.sensory_root
            && self.motor_root == other.motor_root
            && self.mapping == other.mapping
    }

    pub fn sensor_config(&self) -> SensorConfig {
        self.encoder.config()
    }

    pub fn config(&self) -> &ModelConfig {
        &self.config
    }
    pub fn network(&self) -> &Network {
        self.simulation.network()
    }

    /// Controlled counterfactual on a frozen experimental instance. No clocks,
    /// topology or queued arrivals are reset; travelling inputs retain their
    /// emission-time amplitudes. This is not a learning rule.
    pub fn diagnostic_synapse_weight(&mut self, id: SynapseId, weight: f32) -> Result<()> {
        if self.config.learning || self.config.development || self.config.homeostasis {
            return Err(
                "weight intervention requires frozen learning, development and homeostasis".into(),
            );
        }
        let weight = nerva::primitives::Weight::new(weight)?;
        if weight.get() > 2.0 {
            return Err("diagnostic weight must be within 0..=2".into());
        }
        self.simulation
            .update_synapse(id, |edge| edge.set_weight(weight))?;
        Ok(())
    }
    pub fn processing_ids(&self) -> &[NeuronId] {
        &self.processing
    }

    /// Isolated bench stimulus, never used by autonomous runs or training.
    /// Requires frozen rules and no motor feedback; it cannot serve as a teacher.
    pub fn diagnostic_motor_input(
        &mut self,
        key: Key,
        offset_us: u64,
        amplitude: f32,
    ) -> Result<()> {
        if self.config.learning
            || self.config.development
            || self.config.homeostasis
            || self.config.feedback
        {
            return Err(
                "isolated motor stimulus requires frozen rules and disabled feedback".into(),
            );
        }
        let index = self
            .config
            .keyboard
            .keys()
            .iter()
            .position(|candidate| *candidate == key)
            .ok_or("motor key is outside the configured layout")?;
        if offset_us >= WINDOW_US - 1 || !amplitude.is_finite() || amplitude <= 0.0 {
            return Err("invalid isolated motor stimulus".into());
        }
        self.simulation.schedule_external_input(
            SimTime(
                self.next_start
                    .checked_add(offset_us)
                    .ok_or("stimulus time overflow")?,
            ),
            self.motor_neurons[index],
            amplitude,
        )?;
        Ok(())
    }

    /// Disable STDP, utility updates, homeostasis and graph development on this
    /// running instance. Preserve learned parameters, membrane state, clocks,
    /// random streams, queued arrivals and pending action copies. Idempotent.
    pub fn freeze(&mut self) -> Result<()> {
        self.set_plasticity(PlasticitySettings {
            learning: false,
            development: false,
            homeostasis: false,
        })
    }

    pub fn plasticity(&self) -> PlasticitySettings {
        PlasticitySettings {
            learning: self.config.learning,
            development: self.config.development,
            homeostasis: self.config.homeostasis,
        }
    }

    /// Resume existing rules after a frozen evaluation, preserving learned state.
    pub fn set_plasticity(&mut self, settings: PlasticitySettings) -> Result<()> {
        if self.config.kind == ModelKind::Direct && (settings.development || settings.homeostasis) {
            return Err("direct control supports weight learning only".into());
        }
        self.simulation
            .set_homeostasis_enabled(settings.homeostasis)?;
        self.simulation.set_learning_enabled(settings.learning);
        self.config.learning = settings.learning;
        self.config.homeostasis = settings.homeostasis;
        self.config.development = settings.development;
        Ok(())
    }

    /// Feedback about the host's successfully applied/held keys, not a teaching
    /// target. It enters dedicated action-copy cells during the next frame.
    pub fn record_executed_keys(&mut self, keys: &[Key]) -> Result<()> {
        if keys
            .iter()
            .any(|key| !self.config.keyboard.keys().contains(key))
        {
            return Err("executed key is outside the configured motor layout".into());
        }
        self.executed_keys = keys.to_vec();
        self.executed_keys.sort();
        self.executed_keys.dedup();
        Ok(())
    }

    fn is_core(&self, id: NeuronId) -> bool {
        self.processing.binary_search(&id).is_ok()
    }

    pub fn core_synapse_count(&self) -> usize {
        self.network()
            .synapses()
            .filter(|synapse| self.is_core(synapse.pre()) && self.is_core(synapse.post()))
            .count()
    }

    fn develop(&mut self) -> Result<TopologyChanges> {
        // Scope topology changes to Processing cells. Growth demand, candidate
        // scoring, utility/pruning decisions and atomic commit all come from NERVA.
        self.pruning.observe(&self.simulation)?;
        let mut plan = DevelopmentPlan::default();
        let now = self.simulation.current_time();
        for &target in &self.processing {
            let demand = nerva::development::IncomingGrowthDrive::from_neuron(
                self.network().neuron(target).expect("processing neuron"),
            )?
            .demand;
            if demand <= self.formation.growth_threshold {
                continue;
            }
            if let Some(candidate) = local_candidates(
                self.network(),
                target,
                now,
                &self.formation.candidate_search,
            )?
            .into_iter()
            .find(|candidate| self.is_core(candidate.source))
            {
                plan.formations.push(PlannedFormation {
                    target,
                    source: candidate.source,
                    score: candidate.score,
                });
            }
        }
        self.pruning.populate_plan(&mut plan, &self.simulation)?;
        plan.prunings.retain(|pruning| {
            let synapse = self
                .network()
                .synapse(pruning.synapse_id)
                .expect("pruning candidate");
            self.is_core(synapse.pre()) && self.is_core(synapse.post())
        });
        let created = if plan.is_empty() {
            Vec::new()
        } else {
            plan.commit(
                &mut self.simulation,
                &self.formation,
                SynapseId(self.next_synapse),
            )?
        };
        self.pruning.forget_plan(&plan);
        if let Some(last) = created.last() {
            self.next_synapse = last
                .synapse_id
                .0
                .checked_add(1)
                .ok_or("synapse ID overflow")?;
        }
        Ok(TopologyChanges {
            formed: created
                .into_iter()
                .map(|synapse| (synapse.synapse_id.0, synapse.source.0, synapse.target.0))
                .collect(),
            pruned: plan
                .prunings
                .iter()
                .map(|pruning| pruning.synapse_id.0)
                .collect(),
        })
    }

    pub fn step(&mut self, frame: &Frame) -> Result<Step> {
        self.step_for(frame, WINDOW_US)
    }

    /// A bounded observation interval, retaining all dynamics and pending events.
    /// Motor decoding retains its 50-ms lookback; short M3 phases use Core state.
    pub fn step_for(&mut self, frame: &Frame, duration_us: u64) -> Result<Step> {
        if duration_us == 0 || duration_us > WINDOW_US {
            return Err("observation interval must be 1..=50000 us".into());
        }
        let end = self
            .next_start
            .checked_add(duration_us - 1)
            .ok_or("simulation time overflow")?;
        if self.config.feedback {
            for key in std::mem::take(&mut self.executed_keys) {
                let index = self
                    .config
                    .keyboard
                    .keys()
                    .iter()
                    .position(|candidate| *candidate == key)
                    .expect("validated action");
                self.simulation.schedule_external_input(
                    SimTime(self.next_start),
                    self.action_copies[index],
                    1.5,
                )?;
            }
        }
        let mut rates = self.encoder.encode(frame)?;
        for rate in &mut rates {
            rate.rate_hz *= self.sensory_gain;
        }
        if rates
            .iter()
            .any(|rate| !rate.rate_hz.is_finite() || rate.rate_hz > 1_000_000.0)
        {
            return Err("normalized sensory rate exceeds NERVA's input bounds".into());
        }

        let mut observation = self.audit.as_mut().map(|collector| {
            collector.begin(
                self.next_start,
                end,
                &rates,
                self.simulation.network(),
                if self.config.kind == ModelKind::Recurrent {
                    2.0
                } else {
                    0.2 * (self.config.keyboard.keys().len() as f32 / 5.0).sqrt()
                },
            )
        });
        for rate in rates {
            self.spike_source.push(ReceptorSignal {
                channel: ChannelId(rate.channel),
                at: SimTime(self.next_start),
                value: rate.rate_hz,
            })?;
        }
        let mut spikes = Vec::new();
        self.spike_source.advance_until(SimTime(end), &mut spikes)?;
        let sensory_spikes = spikes.len();
        if let (Some(collector), Some(observation)) = (&self.audit, &mut observation) {
            collector.input(observation, &spikes);
        }
        for spike in spikes {
            let input = Routing::sensory(&self.mapping, self.sensory_root.root().id, spike)?
                .ok_or("unmapped visual channel")?;
            self.simulation.schedule_external_input(
                input.arrives_at,
                input.neuron,
                input.amplitude,
            )?;
        }
        let motor_base = self.motor_neurons[0].0;
        let motor_index = |id: NeuronId| {
            id.0.checked_sub(motor_base)
                .map(|index| index as usize)
                .filter(|&index| index < self.motor_neurons.len())
        };
        let mut refractory: Vec<_> = self
            .motor_neurons
            .iter()
            .map(|id| self.network().neuron(*id).unwrap().refractory_until().0)
            .collect();
        let mut motor_inputs = vec![MotorInputStats::default(); Key::COUNT];
        self.simulation.run_until(SimTime(end))?;
        let mut weight_changes = 0;
        let mut core_counts = vec![0; self.processing.len()];
        let mut core_events = Vec::new();
        let mut feedback_arrivals = 0;
        let mut action_copy_arrivals = 0;
        let mut homeostasis_changes = 0;
        for event in self.simulation.drain_event_log().into_events() {
            if let (Some(collector), Some(observation)) = (&mut self.audit, &mut observation) {
                collector.event(observation, &event, self.simulation.network());
            }
            match event {
                ObservationEvent::SpikeEmitted(spike) => {
                    if let Some(index) = motor_index(spike.neuron_id) {
                        refractory[index] = spike.time.0.saturating_add(
                            self.network()
                                .neuron(spike.neuron_id)
                                .unwrap()
                                .params()
                                .refractory_period_us,
                        );
                    }
                    if self.is_core(spike.neuron_id) {
                        let index = (spike.neuron_id.0 - self.processing[0].0) as usize;
                        core_counts[index] += 1;
                        core_events.push((spike.neuron_id.0, spike.time.0));
                    }
                    for output in Routing::motor(&self.mapping, spike.neuron_id, spike.time)? {
                        self.motor_root.observe(MotorOutput {
                            channel: output.channel,
                            at: output.arrives_at,
                            amplitude: output.amplitude,
                        });
                    }
                }
                ObservationEvent::WeightChanged { .. } => weight_changes += 1,
                ObservationEvent::SynapticArrival {
                    source,
                    target,
                    time,
                    amplitude,
                    ..
                } => {
                    if let Some(index) = motor_index(target) {
                        record_motor_input(
                            &mut motor_inputs
                                [self.config.keyboard.keys()[index].channel() as usize],
                            time.0,
                            amplitude,
                            refractory[index],
                        );
                    }
                    if !self.is_core(target) {
                        continue;
                    }
                    if self
                        .network()
                        .neuron(source)
                        .is_some_and(|neuron| neuron.role() == Some(NeuronRole::Motor))
                    {
                        feedback_arrivals += 1;
                    }
                    if self.action_copies.contains(&source) {
                        action_copy_arrivals += 1;
                    }
                }
                ObservationEvent::ExternalInput {
                    target,
                    time,
                    amplitude,
                } => {
                    if let Some(index) = motor_index(target) {
                        record_motor_input(
                            &mut motor_inputs
                                [self.config.keyboard.keys()[index].channel() as usize],
                            time.0,
                            amplitude,
                            refractory[index],
                        );
                    }
                }
                ObservationEvent::HomeostasisChanged { neuron_id, .. }
                    if self.is_core(neuron_id) =>
                {
                    homeostasis_changes += 1;
                }
                _ => {}
            }
        }
        self.pending_motor.extend(self.motor_root.drain_outputs());
        self.pending_motor
            .sort_by_key(|event| (event.at, event.channel));
        let due = self
            .pending_motor
            .partition_point(|event| event.at <= SimTime(end));
        let outputs: Vec<_> = self.pending_motor.drain(..due).collect();
        let pulses: Vec<_> = outputs
            .iter()
            .map(|output| MotorPulse {
                channel: output.channel.0,
                at_us: output.at.0,
                amplitude: output.amplitude,
            })
            .collect();
        let mut motor_counts = vec![0; Key::COUNT];
        for pulse in &pulses {
            motor_counts[pulse.channel as usize] += 1;
        }
        let keys = self.decoder.decode_keys(&pulses, end)?;
        let action = keys.iter().copied().find(|key| !key.is_modifier());
        let development_due = end >= self.next_development_us;
        if development_due {
            self.next_development_us = (end / 100_000)
                .checked_add(1)
                .and_then(|t| t.checked_mul(100_000))
                .ok_or("development clock overflow")?;
        }
        let TopologyChanges { formed, pruned } = if self.config.development && development_due {
            self.develop()?
        } else {
            TopologyChanges::default()
        };
        if let (Some(collector), Some(mut observation)) = (&mut self.audit, observation) {
            observation.after = audit::NetworkSummary::read(
                self.simulation.network(),
                if self.config.kind == ModelKind::Recurrent {
                    2.0
                } else {
                    0.2 * (self.config.keyboard.keys().len() as f32 / 5.0).sqrt()
                },
            );
            observation.formed = formed.len();
            observation.pruned = pruned.len();
            collector.forget_pruned(&pruned);
            collector.finish(observation);
        }
        let start_us = self.next_start;
        self.next_start = end.checked_add(1).ok_or("simulation time overflow")?;
        Ok(Step {
            action,
            keys,
            motor_counts,
            motor_events: pulses,
            motor_inputs,
            start_us,
            end_us: end,
            sensory_spikes,
            motor_spikes: outputs.len(),
            weight_changes,
            core_counts,
            core_events,
            feedback_arrivals,
            action_copy_arrivals,
            homeostasis_changes,
            formed,
            pruned,
            core_synapses: self.core_synapse_count(),
        })
    }
}

fn record_motor_input(
    stats: &mut MotorInputStats,
    time: u64,
    amplitude: f32,
    refractory_until: u64,
) {
    if amplitude > 0.0 {
        stats.positive_arrivals += 1;
        stats.positive_amplitude += f64::from(amplitude);
    }
    if amplitude < 0.0 {
        stats.negative_arrivals += 1;
        stats.negative_amplitude += f64::from(amplitude);
    }
    stats.arrivals_during_refractory += usize::from(amplitude != 0.0 && time < refractory_until);
}
