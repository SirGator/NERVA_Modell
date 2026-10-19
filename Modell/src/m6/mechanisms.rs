//! Exploratory single-cell intervention after the fixed M6 matrix. Replays
//! training, then applies actual cue arrivals to independent neuron copies.
//! The learned parent and all its network parameters remain untouched.
use super::*;
use nerva::{
    core::{Neuron, NeuronRole, SimTime},
    runtime::ObservationEvent,
};

fn response(
    mut neuron: Neuron,
    arrivals: &[(u64, f32)],
    zero_intrinsic_at: Option<u64>,
) -> Result<Value> {
    if let Some(time) = zero_intrinsic_at {
        neuron.advance_to(SimTime(time))?;
        neuron.set_intrinsic_current(0.0)?;
    }
    let mut spikes = Vec::new();
    let mut rows = Vec::new();
    for &(time, input) in arrivals {
        let mut projected = neuron.clone();
        projected.advance_to(SimTime(time))?;
        let potential_before = projected.membrane_potential();
        let threshold = projected.threshold();
        let spike = neuron.integrate_input(SimTime(time), input)?;
        if spike.is_some() {
            spikes.push(time);
        }
        rows.push(json!({"time_us":time,"input_amplitude":input,"potential_before":potential_before,
            "potential_plus_input":potential_before+input,"threshold":threshold,"emitted":spike.is_some(),"potential_after":neuron.membrane_potential()}));
    }
    Ok(json!({"spike_times_us":spikes,"spike_count":spikes.len(),"inputs":rows}))
}

pub fn run(config: &Config, flag: &AtomicBool) -> Result<Value> {
    ensure(
        config.variant == "D"
            && config.base.sensor.mode == SensorMode::Effect2Ch
            && config.base.train_trials >= 256,
        "intrinsic audit requires variant D, effect_2ch/32 and at least 256 training trials",
    )?;
    let frames: Result<Vec<_>> = (0..7).map(world::image).collect();
    let frames = frames?;
    let initial = Model::with_sensor(config.base.model.clone(), config.base.sensor)?;
    let (gain, _) = m4::normalization::gain(&initial, &config.base)?;
    let mut cells = Vec::new();
    let mut checkpoints = Vec::new();
    for reversed in [false, true] {
        for key in [Key::A, Key::D] {
            let mut actor = Actor::new(config, "paired", gain, true)?;
            for trial in schedule(
                config.base.seed ^ TRAIN_SEED,
                config.base.train_trials,
                reversed,
                false,
            ) {
                ensure(!flag.load(Ordering::Relaxed), "intrinsic audit interrupted")?;
                actor.observe(&frames[3], None, BASE)?;
                for duration in [10000, 10000, 30000] {
                    actor.observe(&frames[3], Some(trial.key), duration)?;
                }
                actor.observe(
                    &frames[if trial.shown_motion < 0 { 2 } else { 4 }],
                    None,
                    EFFECT,
                )?;
                actor.observe(&frames[3], None, 10000)?;
            }
            checkpoints.push(json!({"mapping":if reversed{"reversed_mapping"}else{"normal_mapping"},"key":key.qcode(),
                "trained_core_state":m4::membrane(&actor.model),"trained_sensor_state":m4::sensor_state(&actor.model)}));
            let checkpoint = actor.model.network().clone();
            actor.model.freeze()?;
            actor.observe(&frames[3], None, BASE)?;
            let action = actor.model.next_observation_us();
            let key_index = config
                .base
                .model
                .keyboard
                .keys()
                .iter()
                .position(|k| *k == key)
                .unwrap();
            let source = actor
                .model
                .network()
                .neurons()
                .filter(|n| {
                    n.role() == Some(NeuronRole::Sensory)
                        && n.id().0 >= config.base.sensor.channels() as u64
                })
                .nth(key_index)
                .unwrap()
                .id();
            let targets: Vec<_> = actor
                .model
                .network()
                .synapses()
                .filter(|s| s.pre() == source && actor.model.processing_ids().contains(&s.post()))
                .map(|s| s.post())
                .collect();
            let before: Vec<_> = targets
                .iter()
                .map(|&id| actor.model.network().neuron(id).unwrap().clone())
                .collect();
            let mut arrivals = BTreeMap::<_, Vec<(u64, f32)>>::new();
            let mut actual_spikes = Vec::new();
            for duration in [10000, 10000, 30000] {
                let (step, audit) = actor.observe(&frames[3], Some(key), duration)?;
                actual_spikes.extend(step.core_events);
                for event in audit.raw_events {
                    if let ObservationEvent::SynapticArrival {
                        source: from,
                        target,
                        time,
                        amplitude,
                        ..
                    } = event
                        && targets.contains(&target)
                    {
                        ensure(
                            from == source,
                            "single-cell audit received additional network input; isolation would be invalid",
                        )?;
                        arrivals
                            .entry(target)
                            .or_default()
                            .push((time.0, amplitude));
                    }
                }
            }
            let after = actor.model.network().clone();
            for neuron in before {
                let id = neuron.id();
                let inputs = arrivals.get(&id).ok_or("missing actual copy arrivals")?;
                ensure(
                    inputs.len() == 3 && neuron.intrinsic_current() <= 0.0,
                    "single-cell audit requires three inputs and nonpositive intrinsic drive",
                )?;
                let control = response(neuron.clone(), inputs, None)?;
                let zero = response(neuron.clone(), inputs, Some(action - 1))?;
                let expected: Vec<_> = actual_spikes
                    .iter()
                    .filter_map(|&(n, time)| (n == id.0).then_some(time))
                    .collect();
                ensure(
                    control["spike_times_us"] == json!(expected),
                    "isolated unchanged neuron failed to reproduce full-network spikes",
                )?;
                cells.push(json!({"mapping":if reversed{"reversed"}else{"normal"},"key":key.qcode(),"neuron":id.0,
                    "intrinsic_current":neuron.intrinsic_current(),"membrane_tau_us":neuron.params().membrane_tau_us,
                    "action_us":action,"actual_arrivals":inputs,"unchanged_cell":control,"zero_intrinsic_cell":zero,
                    "control_reproduces_network":true}));
            }
            ensure(
                actor.model.network() == &after && learned_parameters_equal(&checkpoint, &after),
                "cell diagnostic modified parent parameters",
            )?;
        }
    }
    Ok(
        json!({"protocol":"NERVA-M6-intrinsic-audit-v1","seed":config.base.seed,"requested_config":config.requested,
        "training_checkpoints":checkpoints,"cells":cells,"parent_parameters_preserved":true,
        "interpretation":"Exploratory single-neuron counterfactual: actual inputs, only constant intrinsic current set to zero on a clone. No whole-network retraining, no regenerated recurrent feedback, no prediction success claim."}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn isolated_intrinsic_intervention_can_restore_three_pulse_integration_without_changing_the_original()
     {
        let mut neuron = Neuron::new(
            nerva::core::NeuronId(0),
            nerva::math::Position3D::ORIGIN,
            nerva::core::Polarity::Excitatory,
            None,
            nerva::config::NeuronConfig {
                resting_potential: 0.0,
                reset_potential: 0.0,
                threshold: 1.0,
                membrane_tau_us: 20000.0,
                refractory_period_us: 5000,
                activity_trace_tau_us: 50000.0,
                ..Default::default()
            },
            SimTime::ZERO,
        )
        .unwrap();
        neuron.set_intrinsic_current(-10.0).unwrap();
        neuron.advance_to(SimTime(200000)).unwrap();
        let before = neuron.clone();
        let inputs = [(201500, 0.6), (211500, 0.6), (221500, 0.6)];
        assert_eq!(
            response(neuron.clone(), &inputs, None).unwrap()["spike_count"],
            0
        );
        assert_eq!(
            response(neuron.clone(), &inputs, Some(200000)).unwrap()["spike_count"],
            1
        );
        assert_eq!(neuron, before);
    }
}
