use nerva::core::{NeuronRole, Polarity};
use nerva_vm::model::{KeyboardLayout, Model, ModelConfig, SensorConfig, SensorMode, SpikeCoding};

fn sensor() -> SensorConfig {
    SensorConfig {
        mode: SensorMode::Position7Ch,
        coding: SpikeCoding::Deterministic,
        ..SensorConfig::default()
    }
}

#[test]
fn sparse_topology_invariants_hold_at_arbitrary_sizes() {
    let sizes = std::env::var("NERVA_TEST_SIZES").unwrap_or_else(|_| "1,3,17,65,1025,4097".into());
    for size in sizes
        .split(',')
        .map(|s| s.trim().parse::<usize>().expect("positive integer sizes"))
    {
        let model = Model::with_sensor(
            ModelConfig {
                processing_neurons: size,
                keyboard: KeyboardLayout::Ad,
                ..ModelConfig::default().frozen()
            },
            sensor(),
        )
        .unwrap();
        assert_eq!(model.processing_ids().len(), size);
        assert_eq!(
            model.network().neuron_count(),
            size + sensor().channels() + 4
        );
        assert_eq!(
            model
                .processing_ids()
                .iter()
                .filter(
                    |&&id| model.network().neuron(id).unwrap().polarity() == Polarity::Inhibitory
                )
                .count(),
            size / 5
        );
        assert!(model.core_synapse_count() <= size * 4);
        for edge in model.network().synapses() {
            let pre = model.network().neuron(edge.pre()).unwrap();
            let post = model.network().neuron(edge.post()).unwrap();
            assert_ne!(edge.pre(), edge.post());
            assert!(
                !(pre.role() == Some(NeuronRole::Sensory)
                    && post.role() == Some(NeuronRole::Motor))
            );
            if pre.role() == Some(NeuronRole::Processing)
                && post.role() == Some(NeuronRole::Processing)
            {
                assert!(pre.position().distance_to(post.position()) <= 0.25);
            }
        }
    }
}

#[test]
fn empty_networks_and_identifier_overflow_are_rejected_before_allocation() {
    for size in [0, usize::MAX] {
        assert!(
            Model::with_sensor(
                ModelConfig {
                    processing_neurons: size,
                    ..ModelConfig::default()
                },
                sensor()
            )
            .is_err()
        );
    }
}
