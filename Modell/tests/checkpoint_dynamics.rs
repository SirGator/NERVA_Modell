use nerva_vm::model::{HomeostasisScope, Model, ModelConfig};
#[test]
fn checkpoint_dynamics_reject_live_and_invalid_changes_and_identity_preserves_replay() {
    let cfg = ModelConfig {
        processing_neurons: 16,
        ..ModelConfig::default()
    };
    let mut model = Model::from_config(cfg.clone()).unwrap();
    assert!(model.diagnostic_core_dynamics(0.9, 1.).is_err());
    assert!(
        model
            .set_checkpoint_homeostasis_scope(HomeostasisScope::Sensor)
            .is_err()
    );
    model.freeze().unwrap();
    let mut same = Model::from_config(cfg).unwrap();
    same.freeze().unwrap();
    for (theta, tau) in [(f32::NAN, 1.), (0., 1.), (1., 3.)] {
        assert!(model.diagnostic_core_dynamics(theta, tau).is_err());
        assert!(model.same_replay_state(&same));
    }
    model.diagnostic_core_dynamics(1., 1.).unwrap();
    assert!(model.same_replay_state(&same));
    let weights = model.network().synapses().cloned().collect::<Vec<_>>();
    let sensor = model
        .network()
        .neurons()
        .filter(|n| !model.processing_ids().contains(&n.id()))
        .cloned()
        .collect::<Vec<_>>();
    model.diagnostic_core_dynamics(0.9, 2.).unwrap();
    assert_eq!(
        model.network().synapses().cloned().collect::<Vec<_>>(),
        weights
    );
    for n in sensor {
        assert_eq!(model.network().neuron(n.id()), Some(&n));
    }
    for id in model.processing_ids() {
        let n = model.network().neuron(*id).unwrap();
        let old = same.network().neuron(*id).unwrap();
        assert_eq!(n.base_threshold(), 0.9 * old.base_threshold());
        assert_eq!(
            n.params().membrane_tau_us,
            2. * old.params().membrane_tau_us
        );
    }
}
