use super::*;

fn frozen(architecture: Architecture) -> Model {
    Model::with_sensor(
        ModelConfig {
            processing_neurons: 64,
            architecture,
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

fn missing_local_pairs(model: &Model) -> Vec<(NeuronId, NeuronId, f32)> {
    let radius = model.diagnostic_formation_geometry().0;
    model
        .processing_ids()
        .iter()
        .flat_map(|&pre| {
            model.processing_ids().iter().filter_map(move |&post| {
                (pre != post
                    && model
                        .network()
                        .neuron(pre)
                        .unwrap()
                        .position()
                        .distance_to(model.network().neuron(post).unwrap().position())
                        <= radius
                    && !model
                        .network()
                        .outgoing_synapses(pre)
                        .unwrap()
                        .any(|edge| edge.post() == post))
                .then_some((pre, post, 0.25))
            })
        })
        .take(3)
        .collect()
}

fn rejects_unchanged(model: &mut Model, edges: &[(NeuronId, NeuronId, f32)]) {
    let before = model.diagnostic_fork().unwrap();
    assert!(model.diagnostic_insert_formed_synapses(edges).is_err());
    assert!(model.same_replay_state(&before));
}

#[test]
fn formed_intervention_rejects_invalid_batches_and_preserves_checkpoint() {
    let mut model = frozen(Architecture::Cortical);
    let pairs = missing_local_pairs(&model);
    assert_eq!(pairs.len(), 3);
    let valid = pairs[0];
    let existing = model.network().synapses().next().unwrap();
    let existing = (existing.pre(), existing.post(), 0.25);
    let noncore = model
        .network()
        .neurons()
        .find(|cell| !model.is_core(cell.id()))
        .unwrap()
        .id();
    let far = model
        .processing_ids()
        .iter()
        .find_map(|&pre| {
            model.processing_ids().iter().find_map(|&post| {
                (model
                    .network()
                    .neuron(pre)
                    .unwrap()
                    .position()
                    .distance_to(model.network().neuron(post).unwrap().position())
                    > model.formation.candidate_search.radius)
                    .then_some((pre, post, 0.25))
            })
        })
        .unwrap();
    for invalid in [
        existing,
        valid,
        (valid.0, valid.0, 0.25),
        (noncore, valid.1, 0.25),
        (valid.0, noncore, 0.25),
        (NeuronId(u64::MAX), valid.1, 0.25),
        far,
        (pairs[1].0, pairs[1].1, -0.1),
        (pairs[1].0, pairs[1].1, 2.1),
        (pairs[1].0, pairs[1].1, f32::NAN),
        (pairs[1].0, pairs[1].1, f32::INFINITY),
        (pairs[1].0, pairs[1].1, f32::NEG_INFINITY),
    ] {
        rejects_unchanged(&mut model, &[valid, invalid]);
    }
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
        rejects_unchanged(&mut live, &[valid]);
    }
    let mut direct = Model::from_config(ModelConfig {
        kind: ModelKind::Direct,
        ..ModelConfig::default().frozen()
    })
    .unwrap();
    rejects_unchanged(&mut direct, &[valid]);
    let mut competition = model.diagnostic_fork().unwrap();
    competition.install_checkpoint_competition().unwrap();
    rejects_unchanged(&mut competition, &[valid]);
    let before = model.diagnostic_fork().unwrap();
    assert!(
        model
            .diagnostic_insert_formed_synapses(&[])
            .unwrap()
            .is_empty()
    );
    assert!(model.same_replay_state(&before));

    // Force failure after the first runtime insertion, beyond request validation.
    let occupied = SynapseId(model.next_synapse + 1);
    model
        .simulation
        .add_synapse(
            Synapse::new(
                occupied,
                pairs[2].0,
                pairs[2].1,
                Weight::new(0.25).unwrap(),
                1_000,
                true,
            )
            .unwrap(),
        )
        .unwrap();
    rejects_unchanged(&mut model, &pairs[..2]);
    model.next_synapse = u64::MAX;
    rejects_unchanged(&mut model, &[valid]);
}

#[test]
fn formed_intervention_preserves_native_rest_profiles_and_operative_replay() {
    for architecture in [Architecture::Cortical, Architecture::Hippocampal] {
        let mut model = frozen(architecture);
        let frame = Frame::new(64, 64, vec![0; 64 * 64 * 3]).unwrap();
        model
            .simulation
            .schedule_external_input(SimTime(1), model.processing_ids()[0], 10.0)
            .unwrap();
        model.step_for(&frame, 1_000).unwrap();
        let mut pairs = missing_local_pairs(&model);
        pairs.truncate(2);
        pairs[0].2 = 0.0;
        pairs[1].2 = 2.0;
        let source = pairs[1].0;
        model
            .simulation
            .schedule_external_input(SimTime(model.next_start + 5_500), source, 10.0)
            .unwrap();
        assert!(model.simulation.pending_event_count() > 0);
        let before = model.diagnostic_fork().unwrap();
        let mut expected = model.diagnostic_fork().unwrap();
        let first_id = model.next_synapse;
        let (radius, birth_weight, delay) = model.diagnostic_formation_geometry();
        assert_eq!((radius, birth_weight, delay), (0.25, 0.25, 1_000));
        let ids = model.diagnostic_insert_formed_synapses(&pairs).unwrap();
        assert_eq!(ids, vec![SynapseId(first_id), SynapseId(first_id + 1)]);
        assert!(model.network().neurons().eq(before.network().neurons()));
        for edge in before.network().synapses() {
            assert_eq!(model.network().synapse(edge.id()), Some(edge));
        }
        for (&id, &(pre, post, weight)) in ids.iter().zip(&pairs) {
            let mut edge =
                Synapse::new(id, pre, post, Weight::new(weight).unwrap(), delay, true).unwrap();
            if architecture == Architecture::Hippocampal {
                area::configure_synapse(&mut edge, before.network(), before.area_field).unwrap();
            }
            assert_eq!(model.network().synapse(id), Some(&edge));
            expected.simulation.add_synapse(edge).unwrap();
        }
        expected.next_synapse += 2;
        // Exact comparison includes queued inputs, rule traces, random streams,
        // exploration/formation history and all clocks, beyond the visible graph.
        assert!(model.same_replay_state(&expected));
        let mut fork = model.diagnostic_fork().unwrap();
        assert!(model.same_replay_state(&fork));
        for _ in 0..2 {
            assert_eq!(
                model.step_for(&frame, 4_000).unwrap(),
                fork.step_for(&frame, 4_000).unwrap()
            );
            assert!(model.same_replay_state(&fork));
        }
        assert!(
            model
                .network()
                .synapse(ids[1])
                .unwrap()
                .transmission_count()
                > 0
        );
        // Duplicate existing pairs are rejected after successful insertion too.
        rejects_unchanged(&mut model, &pairs);
    }
}
