//! Preserved direct sensor-to-motor interface control.
use crate::model::random::next_random;
use nerva::{
    config::NeuronConfig,
    core::{Network, Neuron, NeuronId, NeuronRole, Polarity, SimTime, Synapse, SynapseId},
    io::ChannelId,
    math::Position3D,
    nerves::{Fiber, FiberDirection, FiberId, Mapping},
    primitives::Weight,
    roots::{MotorRoot, RootChannel, RootId, SensoryRoot},
};
use nerva_qemu::{Result, keyboard::Key};

pub(crate) fn build_direct(
    seed: u64,
    count: u64,
    motor_keys: &[Key],
    motor_weight_scale: f32,
    motor_refractory_us: u64,
) -> Result<(Network, Mapping, SensoryRoot, MotorRoot)> {
    let motor_count = motor_keys.len() as u64;
    let weight_scale = (motor_count as f32 / 5.0).sqrt();
    let sensory_root = SensoryRoot::new(
        RootId(1),
        "visual pixels",
        (0..count)
            .map(|channel| RootChannel {
                channel: ChannelId(channel),
            })
            .collect(),
    )?;
    let motor_root = MotorRoot::new(
        RootId(2),
        "keyboard",
        motor_keys
            .iter()
            .map(|key| RootChannel {
                channel: ChannelId(key.channel()),
            })
            .collect(),
    )?;
    let mut network = Network::new();
    let mut mapping = Mapping::new();
    for id in 0..count + motor_count {
        let sensory = id < count;
        network.add_neuron(Neuron::new(
            NeuronId(id),
            Position3D::ORIGIN,
            Polarity::Excitatory,
            Some(if sensory {
                NeuronRole::Sensory
            } else {
                NeuronRole::Motor
            }),
            NeuronConfig {
                resting_potential: 0.0,
                reset_potential: 0.0,
                threshold: 1.0,
                membrane_tau_us: 20_000.0,
                refractory_period_us: if sensory { 1_000 } else { motor_refractory_us },
                activity_trace_tau_us: 20_000.0,
                intrinsic: Default::default(),
            },
            SimTime::ZERO,
        )?)?;
        mapping
            .add_fiber(Fiber::new(
                FiberId(id),
                if sensory {
                    FiberDirection::Sensory
                } else {
                    FiberDirection::Motor
                },
                NeuronId(id),
                1,
                1.05,
            )?)
            .map_err(|error| format!("fiber: {error:?}"))?;
        if sensory {
            mapping.map_sensory(sensory_root.root(), ChannelId(id), FiberId(id))
        } else {
            mapping.map_motor(
                motor_root.root(),
                ChannelId(motor_keys[(id - count) as usize].channel()),
                FiberId(id),
            )
        }
        .map_err(|error| format!("mapping: {error:?}"))?;
    }
    let mut random = seed;
    for pixel in 0..count {
        let first = next_random(&mut random) % motor_count;
        let second = (first + 1 + next_random(&mut random) % (motor_count - 1)) % motor_count;
        for (offset, motor) in [first, second].into_iter().enumerate() {
            let weight = 0.04 + (next_random(&mut random) % 10_000) as f32 * 0.000004;
            network.add_synapse(Synapse::new(
                SynapseId(pixel * 2 + offset as u64),
                NeuronId(pixel),
                NeuronId(count + motor),
                Weight::new(weight * weight_scale * motor_weight_scale)?,
                1_000,
                true,
            )?)?;
        }
    }
    Ok((network, mapping, sensory_root, motor_root))
}
