//! Twelve LIF neurons: four observations, four teaching inputs, four motor outputs.
//! Sixteen initially identical plastic connections learn the guest's association.
//! Four fixed teaching connections trigger causal postsynaptic spikes during training.

use std::{fs, path::Path};

use nerva::{
    config::{LearningConfig, NeuronConfig, RuntimeConfig},
    core::{Network, Neuron, NeuronId, NeuronRole, Polarity, SimTime, Synapse, SynapseId},
    io::{ChannelId, EffectorSignal, ReceptorSignal},
    learning::PairStdp,
    math::Position3D,
    nerves::{Fiber, FiberDirection, FiberId, Mapping, Routing},
    primitives::Weight,
    roots::{MotorOutput, MotorRoot, RootChannel, RootId, SensoryRoot},
    runtime::{ObservationEvent, Simulation},
    transduction::{
        DirectMotorTransducer, DirectSensoryTransducer, MotorTransducer, SensoryTransducer,
    },
};

use crate::Result;

pub struct VmModel {
    simulation: Simulation<PairStdp>,
    sensory_root: SensoryRoot,
    motor_root: MotorRoot,
    mapping: Mapping,
    sensory: DirectSensoryTransducer,
    motor: DirectMotorTransducer,
    next_time: u64,
}

impl VmModel {
    pub fn new() -> Result<Self> {
        let sensory_root = SensoryRoot::new(
            RootId(1),
            "VM observations and teaching inputs",
            (0..8)
                .map(|id| RootChannel {
                    channel: ChannelId(id),
                })
                .collect(),
        )?;
        let motor_root = MotorRoot::new(
            RootId(2),
            "VM decisions",
            (0..4)
                .map(|id| RootChannel {
                    channel: ChannelId(id),
                })
                .collect(),
        )?;
        let mut network = Network::new();
        let mut mapping = Mapping::new();
        for id in 0..12 {
            let role = if id < 8 {
                NeuronRole::Sensory
            } else {
                NeuronRole::Motor
            };
            network.add_neuron(Neuron::new(
                NeuronId(id),
                Position3D::ORIGIN,
                Polarity::Excitatory,
                Some(role),
                NeuronConfig {
                    resting_potential: 0.0,
                    reset_potential: 0.0,
                    threshold: 1.0,
                    membrane_tau_us: 500.0,
                    refractory_period_us: 5_000,
                    activity_trace_tau_us: 2_000.0,
                    intrinsic: Default::default(),
                },
                SimTime::ZERO,
            )?)?;
            let (direction, channel) = if id < 8 {
                (FiberDirection::Sensory, ChannelId(id))
            } else {
                (FiberDirection::Motor, ChannelId(id - 8))
            };
            mapping
                .add_fiber(Fiber::new(FiberId(id), direction, NeuronId(id), 1, 1.0)?)
                .map_err(|error| format!("fiber: {error:?}"))?;
            if id < 8 {
                mapping.map_sensory(sensory_root.root(), channel, FiberId(id))
            } else {
                mapping.map_motor(motor_root.root(), channel, FiberId(id))
            }
            .map_err(|error| format!("mapping: {error:?}"))?;
        }
        for observation in 0..4 {
            for action in 0..4 {
                network.add_synapse(Synapse::new(
                    SynapseId(observation * 4 + action),
                    NeuronId(observation),
                    NeuronId(8 + action),
                    Weight::new(0.05)?,
                    1,
                    true,
                )?)?;
            }
            network.add_synapse(Synapse::new(
                SynapseId(16 + observation),
                NeuronId(4 + observation),
                NeuronId(8 + observation),
                Weight::new(1.0)?,
                1,
                false,
            )?)?;
        }
        let learning = PairStdp::try_from_config(&LearningConfig {
            a_plus: 0.08,
            a_minus: 0.0,
            tau_plus_us: 2_000.0,
            tau_minus_us: 2_000.0,
            stdp_window_us: 5_000,
            max_weight: 1.5,
            ..LearningConfig::default()
        })
        .map_err(|error| format!("STDP: {error:?}"))?;
        Ok(Self {
            simulation: Simulation::new(network, learning, RuntimeConfig::default(), 1.0)?,
            sensory_root,
            motor_root,
            mapping,
            sensory: DirectSensoryTransducer::new(),
            motor: DirectMotorTransducer::new(),
            next_time: 20_000,
        })
    }

    /// Advance a complete, isolated 20 ms trial. Only motor spikes before the
    /// teaching pulse count as predictions. Inference freezes all weights.
    pub fn step(&mut self, observation: u8, teacher: Option<u8>) -> Result<Option<u8>> {
        if observation >= 4 || teacher.is_some_and(|value| value >= 4) {
            return Err("observation and teacher must be in 0..4".into());
        }
        let start = self.next_time;
        let end = start
            .checked_add(20_000)
            .ok_or("simulation time overflow")?;
        self.simulation.set_learning_enabled(teacher.is_some());
        self.sensory.push(ReceptorSignal {
            channel: ChannelId(u64::from(observation)),
            at: SimTime(start),
            value: 1.0,
        })?;
        if let Some(teacher) = teacher {
            self.sensory.push(ReceptorSignal {
                channel: ChannelId(4 + u64::from(teacher)),
                at: SimTime(start + 1_000),
                value: 1.0,
            })?;
        }
        let mut spikes = Vec::new();
        self.sensory.advance_until(SimTime(end - 2), &mut spikes)?;
        for spike in spikes {
            let impulse = Routing::sensory(&self.mapping, self.sensory_root.root().id, spike)?
                .ok_or("missing sensory mapping")?;
            self.simulation.schedule_external_input(
                impulse.arrives_at,
                impulse.neuron,
                impulse.amplitude,
            )?;
        }
        self.simulation.run_until(SimTime(end - 1))?;
        for event in self.simulation.drain_event_log().into_events() {
            if let ObservationEvent::SpikeEmitted(spike) = event {
                for impulse in Routing::motor(&self.mapping, spike.neuron_id, spike.time)? {
                    self.motor_root.observe(MotorOutput {
                        channel: impulse.channel,
                        at: impulse.arrives_at,
                        amplitude: impulse.amplitude,
                    });
                }
            }
        }
        for output in self.motor_root.drain_outputs() {
            self.motor.push(output)?;
        }
        let mut signals: Vec<EffectorSignal> = Vec::new();
        self.motor.advance_until(SimTime(end), &mut signals)?;
        self.next_time = end;
        let mut decisions = signals
            .iter()
            .filter(|signal| signal.at < SimTime(start + 1_000));
        let first = decisions.next().map(|signal| signal.channel.get() as u8);
        // Conflicting learned outputs are reported as abstention.
        Ok(if decisions.next().is_none() {
            first
        } else {
            None
        })
    }

    pub fn weights(&self) -> Vec<f32> {
        self.simulation
            .network()
            .synapses()
            .filter(|synapse| synapse.is_plastic())
            .map(|synapse| synapse.weight().get())
            .collect()
    }

    /// Weights only: neural membrane state and STDP traces start fresh on load.
    pub fn load(&mut self, path: &Path) -> Result<()> {
        let text = fs::read_to_string(path)?;
        let mut lines = text.lines();
        if lines.next() != Some("NERVA_VM_WEIGHTS_V1") {
            return Err("invalid weights header".into());
        }
        let values: Vec<f32> = lines
            .map(str::parse)
            .collect::<std::result::Result<_, _>>()?;
        if values.len() != 16
            || values
                .iter()
                .any(|value| !value.is_finite() || !(0.0..=1.5).contains(value))
        {
            return Err("expected 16 finite weights in 0..=1.5".into());
        }
        for (id, value) in values.into_iter().enumerate() {
            self.simulation
                .update_synapse(SynapseId(id as u64), |synapse| {
                    synapse.set_weight(Weight::new(value).expect("validated weight"))
                })?;
        }
        Ok(())
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        use std::io::Write;
        let temporary = path.with_extension(format!("weights-{}.tmp", std::process::id()));
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        let result = (|| -> Result<()> {
            writeln!(file, "NERVA_VM_WEIGHTS_V1")?;
            for value in self.weights() {
                writeln!(file, "{value}")?;
            }
            file.sync_all()?;
            fs::rename(&temporary, path)?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(temporary);
        }
        result
    }
}
