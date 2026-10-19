use nerva::core::SynapseId;
use nerva_vm::model::{Model, ModelConfig};
#[test]
fn frozen_weight_interventions_reject_invalid_requests_without_changing_replay_state() {
    let config = ModelConfig {
        processing_neurons: 32,
        ..ModelConfig::default()
    };
    let mut model = Model::from_config(config.clone()).unwrap();
    let id = model.network().synapses().next().unwrap().id();
    assert!(model.diagnostic_synapse_weight(id, 0.0).is_err());
    model.freeze().unwrap();
    let mut reference = Model::from_config(config).unwrap();
    reference.freeze().unwrap();
    assert!(model.same_replay_state(&reference));
    for (target, weight) in [
        (id, -1.0),
        (id, f32::NAN),
        (id, f32::INFINITY),
        (id, 2.1),
        (SynapseId(u64::MAX), 0.0),
    ] {
        assert!(model.diagnostic_synapse_weight(target, weight).is_err());
        assert!(model.same_replay_state(&reference));
    }
    let before = model.network().clone();
    model.diagnostic_synapse_weight(id, 0.0).unwrap();
    assert_eq!(model.network().synapse(id).unwrap().weight().get(), 0.0);
    assert!(before.neurons().eq(model.network().neurons()));
    for edge in before.synapses().filter(|edge| edge.id() != id) {
        assert_eq!(model.network().synapse(edge.id()), Some(edge));
    }
}
