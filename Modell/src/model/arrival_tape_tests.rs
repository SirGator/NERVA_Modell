use super::*;

fn frozen() -> Model {
    Model::with_sensor(
        ModelConfig {
            processing_neurons: 32,
            neuromodulation: true,
            ..ModelConfig::default()
        }
        .frozen(),
        SensorConfig {
            resolution: 8,
            ..SensorConfig::default()
        },
    )
    .unwrap()
}

fn rejects_unchanged(model: &mut Model, target: NeuronId, arrivals: &[(u64, f32)]) {
    let before = model.diagnostic_fork().unwrap();
    assert!(
        model
            .diagnostic_replace_core_inputs(target, arrivals)
            .is_err()
    );
    assert!(model.same_replay_state(&before));
}

#[test]
fn arrival_tape_rejections_are_independent_and_atomic() {
    let mut model = frozen();
    let target = model.processing_ids()[0];
    let end = model.next_start + 4 * WINDOW_US - 1;
    for tape in [
        vec![(0, 0.5)],
        vec![(end + 1, 0.5)],
        vec![(2, 0.5), (1, 0.5)],
        vec![(1, 0.5), (2, f32::NAN)],
        vec![(1, 0.5), (2, f32::INFINITY)],
        vec![(1, 0.5), (2, f32::NEG_INFINITY)],
    ] {
        rejects_unchanged(&mut model, target, &tape);
    }
    let noncore = model
        .network()
        .neurons()
        .find(|cell| !model.is_core(cell.id()))
        .unwrap()
        .id();
    rejects_unchanged(&mut model, noncore, &[(1, 0.5)]);
    rejects_unchanged(&mut model, NeuronId(u64::MAX), &[(1, 0.5)]);
    for settings in [
        PlasticitySettings {
            learning: true,
            development: false,
            homeostasis: false,
        },
        PlasticitySettings {
            learning: false,
            development: true,
            homeostasis: false,
        },
        PlasticitySettings {
            learning: false,
            development: false,
            homeostasis: true,
        },
    ] {
        let mut live = model.diagnostic_fork().unwrap();
        live.set_plasticity(settings).unwrap();
        rejects_unchanged(&mut live, target, &[(1, 0.5)]);
    }
    let mut competition = model.diagnostic_fork().unwrap();
    competition.install_checkpoint_competition().unwrap();
    rejects_unchanged(&mut competition, target, &[]);
    let mut direct = Model::from_config(ModelConfig {
        kind: ModelKind::Direct,
        ..ModelConfig::default().frozen()
    })
    .unwrap();
    rejects_unchanged(&mut direct, target, &[]);
    // Force a runtime scheduling rejection after the fork has disabled inputs:
    // the model's requested tape horizon trails the runtime scheduler boundary.
    let mut stale = model.diagnostic_fork().unwrap();
    stale
        .simulation
        .schedule_external_input(SimTime(10), target, 0.0)
        .unwrap();
    stale.simulation.run_until(SimTime(10)).unwrap();
    rejects_unchanged(&mut stale, target, &[(1, 0.5)]);
    model.next_start = u64::MAX - WINDOW_US;
    rejects_unchanged(&mut model, target, &[]);
}

#[test]
fn arrival_tape_blocks_native_inputs_and_delivers_ordered_external_events() {
    let mut model = frozen();
    let (source, target) = model
        .network()
        .synapses()
        .find(|edge| model.is_core(edge.pre()) && model.is_core(edge.post()))
        .map(|edge| (edge.pre(), edge.post()))
        .unwrap();
    model
        .simulation
        .schedule_external_input(SimTime(1), source, 10.0)
        .unwrap();
    let frame = Frame::new(64, 64, vec![0; 64 * 64 * 3]).unwrap();
    model.step_for(&frame, 1).unwrap();
    let start = model.next_start;
    let end = start + 4 * WINDOW_US - 1;
    let receiver = model
        .processing_ids()
        .iter()
        .copied()
        .find(|&id| id != target)
        .unwrap();
    model
        .simulation
        .schedule_external_input(SimTime(start + 3), target, 0.125)
        .unwrap();
    model
        .simulation
        .schedule_external_input(SimTime(start + 4), receiver, 0.25)
        .unwrap();
    let mut expected = model.diagnostic_fork().unwrap();
    let incoming: Vec<_> = model
        .network()
        .synapses()
        .filter(|edge| edge.post() == target)
        .map(|edge| edge.id())
        .collect();
    let mut ids = incoming.clone();
    ids.sort_unstable();
    let cancelled = expected
        .simulation
        .disable_synapses_and_cancel_arrivals(&incoming)
        .unwrap();
    assert!(
        cancelled > 0,
        "native target arrivals must actually be queued"
    );
    let tape = [
        (start, 0.0),
        (start + 1, -0.5),
        (start + 1, 0.75),
        (end, 0.25),
    ];
    for &(time, amplitude) in &tape {
        expected
            .simulation
            .schedule_external_input(SimTime(time), target, amplitude)
            .unwrap();
    }
    let before = model.diagnostic_fork().unwrap();
    assert_eq!(
        model.diagnostic_replace_core_inputs(target, &tape).unwrap(),
        (ids, cancelled)
    );
    assert!(model.same_replay_state(&expected));
    assert!(model.network().neurons().eq(before.network().neurons()));
    assert_eq!(
        model.simulation.chemical_field(),
        before.simulation.chemical_field()
    );
    model.simulation.run_until(SimTime(end)).unwrap();
    expected.simulation.run_until(SimTime(end)).unwrap();
    assert!(model.same_replay_state(&expected));
    let events = model.simulation.drain_event_log().into_events();
    let target_inputs: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            ObservationEvent::ExternalInput {
                target: id,
                time,
                amplitude,
            } if *id == target => Some((time.0, *amplitude)),
            _ => None,
        })
        .collect();
    assert_eq!(
        target_inputs,
        vec![
            (start, 0.0),
            (start + 1, -0.5),
            (start + 1, 0.75),
            (start + 3, 0.125),
            (end, 0.25)
        ]
    );
    assert!(!events.iter().any(|event| matches!(event, ObservationEvent::SynapticArrival { target: id, .. } if *id == target)));
    assert!(events.iter().any(|event| matches!(event, ObservationEvent::ExternalInput { target: id, .. } if *id == receiver)));
}
