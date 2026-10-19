//! Isolated diagnostic stimulation, never fed into an M4 learning run.
use super::{config::M4Config, normalization, world};
use crate::{
    Result,
    model::{
        KeyboardLayout, Model, ModelConfig, SensorConfig, SensorMode, SpikeCoding,
        audit::{AuditOptions, learned_parameters_equal},
    },
};
use nerva::{
    config::{LearningConfig, NeuronConfig, RuntimeConfig},
    core::{Network, Neuron, NeuronId, NeuronRole, Polarity, SimTime, Synapse, SynapseId},
    learning::PairStdp,
    math::Position3D,
    primitives::Weight,
    runtime::{ObservationEvent, Simulation},
};
use nerva_qemu::keyboard::Key;
use serde_json::{Value, json};
use std::{collections::BTreeSet, fs::File, io::Write, path::Path};

fn pair(delta: i64, enabled: bool) -> Result<Value> {
    let post = 200_000u64;
    let arrival = (i128::from(post) - i128::from(delta)) as u64;
    let source = arrival - 1_000;
    let mut network = Network::new();
    for id in 0..2 {
        network.add_neuron(Neuron::new(
            NeuronId(id),
            Position3D::ORIGIN,
            Polarity::Excitatory,
            Some(NeuronRole::Processing),
            NeuronConfig {
                resting_potential: 0.0,
                reset_potential: 0.0,
                threshold: 1.0,
                membrane_tau_us: 20_000.0,
                refractory_period_us: 1000,
                activity_trace_tau_us: 50_000.0,
                ..NeuronConfig::default()
            },
            SimTime::ZERO,
        )?)?;
    }
    network.add_synapse(Synapse::new(
        SynapseId(0),
        NeuronId(0),
        NeuronId(1),
        Weight::new(0.5)?,
        1000,
        true,
    )?)?;
    let rule = PairStdp::new(&LearningConfig {
        a_plus: 0.0001,
        a_minus: 0.00012,
        max_weight: 2.0,
        ..LearningConfig::default()
    });
    let mut simulation = Simulation::new(network, rule, RuntimeConfig::default(), 3.0)?;
    simulation.set_learning_enabled(enabled);
    simulation.schedule_external_input(SimTime(source), NeuronId(0), 1.05)?;
    simulation.schedule_external_input(SimTime(post), NeuronId(1), 1.05)?;
    simulation.run_until(SimTime(arrival.max(post) + 50_000))?;
    let events = simulation.drain_event_log().into_events();
    let actual_arrival = events
        .iter()
        .find_map(|event| {
            if let ObservationEvent::SynapticArrival { time, .. } = event {
                Some(time.0)
            } else {
                None
            }
        })
        .ok_or("single-synapse arrival missing")?;
    let actual_post = events
        .iter()
        .find_map(|event| {
            if let ObservationEvent::SpikeEmitted(spike) = event {
                (spike.neuron_id == NeuronId(1)).then_some(spike.time.0)
            } else {
                None
            }
        })
        .ok_or("single-synapse post spike missing")?;
    let weight = simulation
        .network()
        .synapse(SynapseId(0))
        .unwrap()
        .weight()
        .get();
    let expected = if !enabled || delta.unsigned_abs() > 100_000 {
        0.5
    } else if delta > 0 {
        0.5 + 0.0001 * (-(delta as f32) / 20_000.0).exp()
    } else {
        0.5 - 0.00012 * ((delta as f32) / 20_000.0).exp()
    };
    let error = (weight - expected).abs();
    if error > 1e-7 || i128::from(actual_post) - i128::from(actual_arrival) != i128::from(delta) {
        return Err(
            "single-synapse STDP disagrees with actual arrival timing or theoretical rule".into(),
        );
    }
    Ok(
        json!({"requested_post_minus_arrival_us":delta,"source_spike_us":source,"actual_synaptic_arrival_us":actual_arrival,"actual_post_spike_us":actual_post,"learning_enabled":enabled,"initial_weight":0.5,"final_weight":weight,"expected_weight":expected,"absolute_error":error,"pre_trace":simulation.network().synapse(SynapseId(0)).unwrap().pre_trace(),"weight_updates":events.iter().filter(|e|matches!(e,ObservationEvent::WeightChanged{..})).count(),"passed":true}),
    )
}
fn base(seed: u64) -> ModelConfig {
    ModelConfig {
        seed,
        processing_neurons: 32,
        keyboard: KeyboardLayout::Ad,
        ..ModelConfig::default().frozen()
    }
}
fn sensor() -> SensorConfig {
    SensorConfig {
        mode: SensorMode::Effect2Ch,
        coding: SpikeCoding::Deterministic,
        ..SensorConfig::default()
    }
}
fn potential(model: &Model, until: u64) -> Result<Vec<f32>> {
    model
        .processing_ids()
        .iter()
        .map(|&id| {
            let mut copy = model.network().neuron(id).unwrap().clone();
            copy.advance_to(SimTime(until))?;
            Ok(copy.membrane_potential())
        })
        .collect()
}
fn feedback(seed: u64, action: Key, copy: Option<Key>, delay: u64) -> Result<Value> {
    let mut model = Model::with_sensor(base(seed), sensor())?;
    model.enable_audit(AuditOptions {
        raw_events: true,
        input_spikes: true,
    });
    let before = model.network().clone();
    let mut spikes = Vec::new();
    let mut arrivals = Vec::new();
    let mut observed = 0;
    if delay > 0 {
        let step = model.step_for(&world::image(3)?, delay)?;
        spikes.extend(step.core_events);
        model.take_audit();
    }
    model.record_executed_keys(&copy.into_iter().collect::<Vec<_>>())?;
    let step = model.step_for(&world::image(3)?, 20_000 - delay)?;
    spikes.extend(step.core_events);
    let audit = model.take_audit().unwrap();
    for event in &audit.raw_events {
        if let ObservationEvent::SynapticArrival {
            time,
            source,
            target,
            amplitude,
            ..
        } = event
            && source.0 >= model.processing_ids().last().unwrap().0 + 3
            && model.processing_ids().contains(target)
        {
            arrivals.push(
                json!({"time_us":time.0,"source":source.0,"target":target.0,"amplitude":amplitude}),
            );
            observed += 1;
        }
    }
    if !learned_parameters_equal(&before, model.network()) {
        return Err("feedback bench changed frozen parameters".into());
    }
    let potentials = potential(&model, 20_000)?;
    Ok(
        json!({"seed":seed,"executed_action":action.qcode(),"copy":copy.map(Key::qcode),"copy_delay_us":delay,"core_spikes":spikes,"core_spike_count":spikes.len(),"copy_arrivals":audit.action_copy_arrivals,"first_copy_arrival_us":audit.first_action_copy_arrival_us,"arrivals":arrivals,"observed_copy_paths":observed,"core_potentials_at_20ms":potentials,"frozen":true}),
    )
}

pub fn run(output: &Path) -> Result<Value> {
    let mut stdp = Vec::new();
    for delta in [
        -120_000, -20_000, -8_000, -2_000, 2_000, 8_000, 20_000, 120_000,
    ] {
        stdp.push(pair(delta, true)?);
    }
    stdp.push(pair(2_000, false)?);
    let mut feedback_rows = Vec::new();
    let mut signal_rows = Vec::new();
    let mut actuator = Vec::new();
    let mut raw = File::create(super::logging::sidecar(output, ".signal-events.csv"))?;
    writeln!(raw, "seed,image,event")?;
    for seed in 0..3 {
        for action in [Key::A, Key::D] {
            for intervention in ["correct", "none", "swapped", "delayed"] {
                let copy = match intervention {
                    "none" => None,
                    "swapped" => Some(if action == Key::A { Key::D } else { Key::A }),
                    _ => Some(action),
                };
                let mut result = feedback(
                    seed,
                    action,
                    copy,
                    if intervention == "delayed" { 4_000 } else { 0 },
                )?;
                result["intervention"] = json!(intervention);
                feedback_rows.push(result);
            }
        }
        let params = json!({"protocol":"NERVA-M4-v1","case_id":"signal_audit","seed":seed,"params":{"sensor":"effect_2ch","resolution":0,"encoding":"deterministic","processing":32,"action_effect_delay_ms":8,"motor_feedback":false,"stdp":true,"homeostasis":true,"development":false,"initial_drive":0,"pruning_min_us":5000000,"normalization":"equal_input_current"},"arms":super::config::ARMS,"train_trials":8,"test_trials":8,"calibration_trials":8});
        let cfg = M4Config::parse(params)?;
        for image in [2, 3, 4] {
            let mut model = Model::with_sensor(
                ModelConfig {
                    feedback: false,
                    ..base(seed)
                },
                sensor(),
            )?;
            let (gain, _) = normalization::gain(&model, &cfg)?;
            model.set_sensory_gain(gain)?;
            model.enable_audit(AuditOptions {
                raw_events: true,
                input_spikes: true,
            });
            let before = model.network().clone();
            let mut core = 0;
            let mut sensory = 0;
            let mut motors = 0;
            let mut positive = 0;
            let mut negative = 0;
            let mut paths = BTreeSet::new();
            let mut first_core = None;
            let mut first_motor = None;
            for _ in 0..5 {
                let step = model.step(&world::image(image)?)?;
                core += step.core_events.len();
                sensory += step.sensory_spikes;
                motors += step.motor_spikes;
                first_core = first_core.or(step.core_events.first().map(|e| e.1));
                first_motor = first_motor.or(step.motor_events.first().map(|e| e.at_us));
                for event in model.take_audit().unwrap().raw_events {
                    if let ObservationEvent::SynapticArrival {
                        synapse_id,
                        amplitude,
                        ..
                    } = &event
                    {
                        paths.insert(synapse_id.0);
                        positive += usize::from(*amplitude > 0.0);
                        negative += usize::from(*amplitude < 0.0);
                    }
                    writeln!(
                        raw,
                        "{seed},{image},\"{}\"",
                        format!("{event:?}").replace('"', "\"\"")
                    )?;
                }
            }
            if !learned_parameters_equal(&before, model.network()) {
                return Err("signal bench changed frozen parameters".into());
            }
            signal_rows.push(json!({"seed":seed,"visible_position":image,"sensory_spikes":sensory,"core_spikes":core,"motor_spikes":motors,"positive_arrivals":positive,"negative_arrivals":negative,"unique_active_synapses":paths.len(),"active_synapse_ids":paths,"first_core_spike_us":first_core,"first_motor_spike_us":first_motor,"core_potentials_at_250ms":potential(&model,250_000)?,"input_gain":gain}));
        }
        for keys in [vec![], vec![Key::A], vec![Key::D], vec![Key::A, Key::D]] {
            let mut model = Model::with_sensor(
                ModelConfig {
                    feedback: false,
                    ..base(seed)
                },
                sensor(),
            )?;
            for &key in &keys {
                model.diagnostic_motor_input(key, 1000, 1.05)?;
            }
            let step = model.step(&world::image(3)?)?;
            let expected = if keys.len() == 1 { Some(keys[0]) } else { None };
            if step.action != expected {
                return Err("isolated motor mapping failed".into());
            }
            let mut world = world::DelayedWorld::new(false)?;
            let effect = if let Some(key) = step.action {
                world.execute(key, 50_001, 8_000)?;
                world.advance(58_001)?;
                Some(world::visible_effect(&world.observe_native()?)?)
            } else {
                None
            };
            actuator.push(json!({"seed":seed,"keys":keys.iter().map(|key|key.qcode()).collect::<Vec<_>>(),"motor_pulses":step.motor_events.len(),"selected":step.action.map(Key::qcode),"visible_effect":effect,"passed":true}));
        }
    }
    let mut limits = vec![
        "Bench stimuli are isolated diagnostics and never training targets for M4",
        "Synaptic arrival deltas, not source emission deltas, define the STDP comparisons",
        "Feedback tests use identical neutral images and preserve the same scaffold",
        "Zero spike response does not mean zero subthreshold information",
    ];
    limits.push("These three-seed bench tests are not a ten-seed learning confirmation");
    Ok(
        json!({"protocol":"NERVA-M4-mechanisms-v1","stdp_pairs":stdp,"feedback_interventions":feedback_rows,"signal_paths":signal_rows,"actuator_cases":actuator,"stdp_theory_all_passed":true,"actuator_all_passed":true,"limits":limits}),
    )
}
