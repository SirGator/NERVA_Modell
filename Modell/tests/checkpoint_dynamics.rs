use nerva::core::SynapseId;
use nerva_qemu::capture::Frame;
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

#[test]
fn development_audit_samples_only_maintenance_and_preserves_native_replay() {
    let config = ModelConfig {
        processing_neurons: 32,
        development: true,
        ..ModelConfig::default()
    };
    let mut traced = Model::from_config(config.clone()).unwrap();
    let mut native = Model::from_config(config).unwrap();
    let target = traced.processing_ids()[0];
    let focus = traced
        .network()
        .synapses()
        .find(|edge| {
            [edge.pre(), edge.post()]
                .iter()
                .all(|cell| traced.processing_ids().contains(cell))
        })
        .unwrap()
        .id();
    assert!(
        traced
            .enable_development_audit([target].into(), [SynapseId(u64::MAX)].into())
            .is_err()
    );
    traced
        .enable_development_audit([target].into(), [focus].into())
        .unwrap();
    let white = Frame::new(64, 64, vec![255; 64 * 64 * 3]).unwrap();
    assert_eq!(traced.step(&white).unwrap(), native.step(&white).unwrap());
    assert!(traced.take_development_audit().is_none());
    let observed = traced.step(&white).unwrap();
    assert_eq!(observed, native.step(&white).unwrap());
    let audit = traced.take_development_audit().unwrap();
    assert_eq!(audit["time_us"], observed.end_us);
    assert_eq!(audit["formation"][0]["target"], target.0);
    assert_eq!(audit["pruning"][0]["edge"], focus.0);
    assert!(traced.take_development_audit().is_none());
    assert!(traced.same_replay_state(&native));
}
