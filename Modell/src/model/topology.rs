//! Minimal spatial scaffold. No task-specific action wiring or internal layers.
use super::random::{next_random, shuffle};
use super::*;
use std::collections::BTreeSet;

pub(crate) struct Topology {
    pub network: Network,
    pub mapping: Mapping,
    pub sensory_root: SensoryRoot,
    pub motor_root: MotorRoot,
    pub processing: Vec<NeuronId>,
    pub action_copies: Vec<NeuronId>,
    pub next_synapse: u64,
}

pub(crate) fn build(config: &ModelConfig, sensor: &SensorConfig) -> Result<Topology> {
    let sensors = sensor.channels() as u64;
    let mut network = Network::new();
    let mut mapping = Mapping::new();
    let keys = config.keyboard.keys();
    let core_count = config.processing_neurons as u64;
    let motors = sensors + core_count;
    let copies = motors + keys.len() as u64;
    let sensory_root = SensoryRoot::new(
        RootId(1),
        "visual pixels",
        (0..sensors)
            .map(|id| RootChannel {
                channel: ChannelId(id),
            })
            .collect(),
    )?;
    let motor_root = MotorRoot::new(
        RootId(2),
        "keyboard",
        keys.iter()
            .map(|key| RootChannel {
                channel: ChannelId(key.channel()),
            })
            .collect(),
    )?;
    let mut rng = config.seed;
    let mut identities: Vec<_> = (0..core_count).collect();
    shuffle(&mut identities, &mut rng);
    let inhibitory: BTreeSet<_> = identities[..(core_count as usize / 5)]
        .iter()
        .copied()
        .collect();
    let side = (core_count as f64).cbrt().ceil() as u64;
    for id in 0..sensors + core_count + keys.len() as u64 * 2 {
        let (position, role, polarity, refractory) = if id < sensors {
            (
                sensor.position(id),
                NeuronRole::Sensory,
                Polarity::Excitatory,
                1_000,
            )
        } else if id < motors {
            let index = id - sensors;
            let center = (side - 1) as f32 / 2.0;
            let mut jitter = || (next_random(&mut rng) % 10_000) as f32 * 0.000002 - 0.01;
            (
                Position3D::new(
                    (index % side) as f32 * 0.12 - center * 0.12 + jitter(),
                    ((index / side) % side) as f32 * 0.12 - center * 0.12 + jitter(),
                    (index / (side * side)) as f32 * 0.12 - center * 0.12 + jitter(),
                ),
                NeuronRole::Processing,
                if inhibitory.contains(&index) {
                    Polarity::Inhibitory
                } else {
                    Polarity::Excitatory
                },
                5_000,
            )
        } else {
            let (index, role, z) = if id < copies {
                (id - motors, NeuronRole::Motor, 1.5)
            } else {
                (id - copies, NeuronRole::Sensory, 1.2)
            };
            (
                Position3D::new(index as f32 / keys.len().max(2) as f32 - 0.5, 0.0, z),
                role,
                Polarity::Excitatory,
                if role == NeuronRole::Motor {
                    config.motor_refractory_us
                } else {
                    10_000
                },
            )
        };
        let mut neuron = Neuron::new(
            NeuronId(id),
            position,
            polarity,
            Some(role),
            NeuronConfig {
                resting_potential: 0.0,
                reset_potential: 0.0,
                threshold: if role == NeuronRole::Processing {
                    config.core_threshold
                } else {
                    1.0
                },
                membrane_tau_us: if role == NeuronRole::Processing {
                    config.core_membrane_tau_us
                } else {
                    20_000.0
                },
                refractory_period_us: refractory,
                activity_trace_tau_us: 50_000.0,
                intrinsic: Default::default(),
            },
            SimTime::ZERO,
        )?;
        if role == NeuronRole::Processing {
            // Uniform local startup demand, no preselected future endpoints.
            neuron.set_structural_drive_clamped(config.initial_structural_drive, 0.0, 3.0)?;
        }
        network.add_neuron(neuron)?;
        if id < sensors || (motors..copies).contains(&id) {
            let sensory = id < sensors;
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
                    ChannelId(keys[(id - motors) as usize].channel()),
                    FiberId(id),
                )
            }
            .map_err(|error| format!("mapping: {error:?}"))?;
        }
    }
    let processing: Vec<_> = (sensors..motors).map(NeuronId).collect();
    let mut next_id = 0;
    // Four sparse local visual projections per input; no direct sensor→motor shortcut.
    for sensor in 0..sensors {
        let position = network.neuron(NeuronId(sensor)).expect("sensor").position();
        let mut nearby = processing.clone();
        nearby.sort_by(|a, b| {
            network
                .neuron(*a)
                .unwrap()
                .position()
                .distance_to(position)
                .total_cmp(&network.neuron(*b).unwrap().position().distance_to(position))
                .then(a.cmp(b))
        });
        nearby.truncate(24.min(nearby.len()));
        shuffle(&mut nearby, &mut rng);
        for &target in nearby.iter().take(4) {
            let weight = 0.55 + (next_random(&mut rng) % 10_000) as f32 * 0.00002;
            connect(
                &mut network,
                &mut next_id,
                NeuronId(sensor),
                target,
                weight * config.sensory_weight_scale,
                1_000,
                true,
            )?;
        }
    }
    // At most four initial recurrent edges per cell, selected locally at random.
    for &source in &processing {
        let pos = network.neuron(source).unwrap().position();
        let mut nearby: Vec<_> = processing
            .iter()
            .copied()
            .filter(|target| {
                *target != source
                    && network.neuron(*target).unwrap().position().distance_to(pos) <= 0.25
            })
            .collect();
        shuffle(&mut nearby, &mut rng);
        for &target in nearby.iter().take(4) {
            let weight = 0.18 + (next_random(&mut rng) % 10_000) as f32 * 0.00001;
            let delay = 500 + next_random(&mut rng) % 1501;
            connect(
                &mut network,
                &mut next_id,
                source,
                target,
                weight * config.core_weight_scale,
                config.core_delay_us.unwrap_or(delay),
                true,
            )?;
        }
    }
    let excitatory: Vec<_> = processing
        .iter()
        .copied()
        .filter(|id| network.neuron(*id).unwrap().polarity() == Polarity::Excitatory)
        .collect();
    for (index, _) in keys.iter().enumerate() {
        let motor = NeuronId(motors + index as u64);
        let mut sources = excitatory.clone();
        shuffle(&mut sources, &mut rng);
        for &source in sources.iter().take(8) {
            let weight = 1.0 + (next_random(&mut rng) % 10_000) as f32 * 0.00004;
            connect(
                &mut network,
                &mut next_id,
                source,
                motor,
                weight * config.motor_weight_scale,
                1_000,
                true,
            )?;
        }
        let mut targets = processing.clone();
        shuffle(&mut targets, &mut rng);
        for &target in targets.iter().take(3) {
            if config.feedback {
                connect(
                    &mut network,
                    &mut next_id,
                    motor,
                    target,
                    0.35,
                    1_500,
                    false,
                )?;
                connect(
                    &mut network,
                    &mut next_id,
                    NeuronId(copies + index as u64),
                    target,
                    config.action_copy_weight,
                    1_500,
                    false,
                )?;
            }
        }
    }
    if config.motor_competition {
        // Dale-consistent competition: each excitatory motor recruits a local
        // inhibitory cell targeting the other motor. Existing cells/edges/RNG
        // are unchanged; no goal or preferred action enters this symmetric motif.
        for index in 0..2u64 {
            let inhibitory = NeuronId(copies + keys.len() as u64 + index);
            let motor = NeuronId(motors + index);
            let opposite = NeuronId(motors + (1 - index));
            let position = network.neuron(motor).expect("motor").position();
            network.add_neuron(Neuron::new(
                inhibitory,
                Position3D::new(position.x, position.y, position.z + 0.1),
                Polarity::Inhibitory,
                None,
                NeuronConfig {
                    resting_potential: 0.0,
                    reset_potential: 0.0,
                    threshold: 1.0,
                    membrane_tau_us: 20_000.0,
                    refractory_period_us: 1_000,
                    activity_trace_tau_us: 50_000.0,
                    intrinsic: Default::default(),
                },
                SimTime::ZERO,
            )?)?;
            connect(
                &mut network,
                &mut next_id,
                motor,
                inhibitory,
                1.2,
                500,
                false,
            )?;
            connect(
                &mut network,
                &mut next_id,
                inhibitory,
                opposite,
                config.motor_inhibition_weight,
                500,
                false,
            )?;
        }
    }
    // Additional local edges use their own random stream after the original
    // scaffold. Existing cells, visual/motor/copy wiring and RNG draws stay
    // identical when only recurrent density changes. No action/outcome labels.
    if config.core_fanout > 4 {
        let mut extra_rng = config.seed ^ 0xa68b712e5c934df0;
        for &source in &processing {
            let position = network.neuron(source).unwrap().position();
            let mut nearby: Vec<_> = processing
                .iter()
                .copied()
                .filter(|&target| {
                    target != source
                        && network
                            .neuron(target)
                            .unwrap()
                            .position()
                            .distance_to(position)
                            <= 0.25
                        && !network
                            .outgoing_synapses(source)
                            .unwrap()
                            .any(|s| s.post() == target)
                })
                .collect();
            shuffle(&mut nearby, &mut extra_rng);
            for target in nearby.into_iter().take(config.core_fanout - 4) {
                let weight = (0.18 + (next_random(&mut extra_rng) % 10_000) as f32 * 0.00001)
                    * config.core_weight_scale;
                let delay = 500 + next_random(&mut extra_rng) % 1501;
                connect(
                    &mut network,
                    &mut next_id,
                    source,
                    target,
                    weight,
                    config.core_delay_us.unwrap_or(delay),
                    true,
                )?;
            }
        }
    }
    Ok(Topology {
        network,
        mapping,
        sensory_root,
        motor_root,
        processing,
        action_copies: (copies..copies + keys.len() as u64).map(NeuronId).collect(),
        next_synapse: next_id,
    })
}

fn connect(
    network: &mut Network,
    cursor: &mut u64,
    source: NeuronId,
    target: NeuronId,
    weight: f32,
    delay: u64,
    plastic: bool,
) -> Result<()> {
    network.add_synapse(Synapse::new(
        SynapseId(*cursor),
        source,
        target,
        Weight::new(weight)?,
        delay,
        plastic,
    )?)?;
    *cursor += 1;
    Ok(())
}
