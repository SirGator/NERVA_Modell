use nerva::{
    cell_profiles,
    core::{NeuronRole, Polarity},
};
use nerva_qemu::capture::Frame;
use nerva_vm::model::{
    Architecture, KeyboardLayout, Model, ModelConfig, ModelKind, SensorConfig, SpikeCoding,
    audit::learned_parameters_equal,
};
fn config(chemistry: bool) -> ModelConfig {
    ModelConfig {
        processing_neurons: 128,
        keyboard: KeyboardLayout::Wasd,
        architecture: Architecture::Hippocampal,
        neuromodulation: chemistry,
        development: false,
        homeostasis: false,
        feedback: false,
        ..Default::default()
    }
}
fn build(config: ModelConfig) -> Model {
    Model::with_sensor(
        config,
        SensorConfig {
            resolution: 8,
            coding: SpikeCoding::Deterministic,
            ..Default::default()
        },
    )
    .unwrap()
}
fn white() -> Frame {
    Frame::new(8, 8, vec![255; 8 * 8 * 3]).unwrap()
}
#[test]
fn explicit_learning_rule_preserves_network_and_automatic_defaults() {
    use nerva_vm::model::LearningRule;
    let sensor = SensorConfig {
        resolution: 8,
        coding: SpikeCoding::Deterministic,
        ..Default::default()
    };
    let automatic = Model::with_sensor(config(true), sensor).unwrap();
    let three =
        Model::with_sensor_and_learning(config(true), sensor, LearningRule::ThreeFactor).unwrap();
    let pair =
        Model::with_sensor_and_learning(config(true), sensor, LearningRule::PairStdp).unwrap();
    assert!(automatic.same_replay_state(&three));
    let accumulated =
        Model::with_sensor_and_learning(config(true), sensor, LearningRule::ThreeFactorAccumulated)
            .unwrap();
    assert_eq!(three.network(), accumulated.network());
    assert!(!three.same_replay_state(&accumulated));
    assert_eq!(pair.network(), three.network());
    assert!(!pair.same_replay_state(&three));
    let cortical = ModelConfig {
        architecture: Architecture::Cortical,
        neuromodulation: false,
        ..config(false)
    };
    let automatic = Model::with_sensor(cortical.clone(), sensor).unwrap();
    let pair = Model::with_sensor_and_learning(cortical, sensor, LearningRule::PairStdp).unwrap();
    assert!(automatic.same_replay_state(&pair));
}
#[test]
fn shared_tissue_has_graded_profiles_cross_area_edges_and_real_release_classes() {
    let model = build(config(true));
    let summary = model.hippocampal_summary();
    assert!(
        summary["dominant_influence_counts"]
            .as_array()
            .unwrap()
            .iter()
            .all(|v| v.as_u64().unwrap() > 0)
    );
    assert!(summary["short_term_synapses"].as_u64().unwrap() > 0);
    assert!(
        summary["release_source_counts"]
            .as_array()
            .unwrap()
            .iter()
            .all(|v| v.as_u64().unwrap() > 0)
    );
    let mut crosses = 0;
    let mut thresholds = std::collections::BTreeSet::new();
    for &id in model.processing_ids() {
        let cell = model.network().neuron(id).unwrap();
        thresholds.insert(cell.params().threshold.to_bits());
        assert_eq!(cell.role(), Some(NeuronRole::Processing));
        let weights = model.area_weights(id).unwrap();
        weights.validate().unwrap();
        assert!(
            weights.as_array().iter().filter(|&&v| v > 0.01).count() > 1,
            "soft boundary became an exclusive region"
        );
    }
    assert!(thresholds.len() > 4);
    for edge in model.network().synapses() {
        if let (Some(pre), Some(post)) = (
            model.area_weights(edge.pre()),
            model.area_weights(edge.post()),
        ) {
            let source = model.network().neuron(edge.pre()).unwrap();
            assert_eq!(
                edge.short_term_config(),
                cell_profiles::short_term_profile(pre, post, source.polarity()).unwrap()
            );
            if pre
                .as_array()
                .iter()
                .zip(post.as_array())
                .any(|(a, b)| (a - b).abs() > 0.2)
            {
                crosses += 1;
            }
        }
    }
    assert!(crosses > 0);
}
#[test]
fn chemical_ablation_preserves_initial_tissue_and_native_activity_replays() {
    let mut active = build(config(true));
    let mut repeated = build(config(true));
    let mut no_chem = build(config(false));
    assert_eq!(
        active.network(),
        no_chem.network(),
        "chemical knockout changed the initial anatomy"
    );
    assert!(active.same_replay_state(&repeated));
    let mut spikes = 0;
    for _ in 0..6 {
        let a = active.step(&white()).unwrap();
        let b = repeated.step(&white()).unwrap();
        no_chem.step(&white()).unwrap();
        assert_eq!(format!("{a:?}"), format!("{b:?}"));
        assert!(active.same_replay_state(&repeated));
        spikes += a.core_counts.iter().sum::<usize>();
    }
    assert!(spikes > 0);
    assert!(
        active.hippocampal_summary()["active_chemical_pulses"]
            .as_u64()
            .unwrap()
            > 0
    );
    assert!(
        active.hippocampal_summary()["modulated_processing_neurons"]
            .as_u64()
            .unwrap()
            > 0
    );
    assert_eq!(no_chem.hippocampal_summary()["active_chemical_pulses"], 0);
    assert_eq!(
        no_chem.hippocampal_summary()["modulated_processing_neurons"],
        0
    );
    active.freeze().unwrap();
    let before = active.network().clone();
    for _ in 0..2 {
        active.step(&white()).unwrap();
    }
    assert!(learned_parameters_equal(&before, active.network()));
}
#[test]
fn structural_growth_uses_the_same_library_profiles_as_initial_connections() {
    let mut c = config(false);
    c.development = true;
    let mut model = build(c);
    let mut formed = 0;
    for _ in 0..8 {
        let step = model.step(&white()).unwrap();
        formed += step.formed.len();
        for &(edge_id, _, _) in &step.formed {
            let edge = model
                .network()
                .synapse(nerva::core::SynapseId(edge_id))
                .unwrap();
            let pre = model.area_weights(edge.pre()).unwrap();
            let post = model.area_weights(edge.post()).unwrap();
            let polarity = model.network().neuron(edge.pre()).unwrap().polarity();
            assert_eq!(
                edge.short_term_config(),
                cell_profiles::short_term_profile(pre, post, polarity).unwrap()
            );
        }
    }
    assert!(formed > 0);
}
#[test]
fn controls_and_invalid_profiles_are_explicit() {
    let mut c = config(false);
    c.area_transition_width = 0.;
    let sharp = build(c);
    for &id in sharp.processing_ids() {
        assert_eq!(
            sharp
                .area_weights(id)
                .unwrap()
                .as_array()
                .iter()
                .filter(|&&v| v == 1.)
                .count(),
            1
        );
    }
    let baseline = build(ModelConfig {
        processing_neurons: 128,
        keyboard: KeyboardLayout::Wasd,
        ..Default::default()
    });
    assert!(
        baseline
            .network()
            .neurons()
            .all(|n| n.release_profile() == Default::default()
                && n.receptor_profile() == Default::default())
    );
    assert!(
        baseline
            .network()
            .synapses()
            .all(|s| s.short_term_config().is_none())
    );
    assert_eq!(baseline.processing_ids().iter().filter(|&&id|baseline.network().neuron(id).unwrap().polarity()==Polarity::Inhibitory).count(),128/5);
    for width in [f32::NAN, -0.1, 1.1] {
        let mut c = config(false);
        c.area_transition_width = width;
        assert!(c.validate().is_err());
    }
    let mut direct = config(false);
    direct.kind = ModelKind::Direct;
    assert!(direct.validate().is_err());
}

#[test]
fn learning_audit_preserves_full_model_replay_and_reports_limits() {
    let mut plain = build(config(true));
    let mut audited = build(config(true));
    audited.enable_learning_audit(100_000).unwrap();
    for _ in 0..4 {
        let a = plain.step(&white()).unwrap();
        let b = audited.step(&white()).unwrap();
        assert_eq!(format!("{a:?}"), format!("{b:?}"));
        assert!(plain.same_replay_state(&audited));
        let log = audited.take_learning_audit().unwrap();
        assert!(log.is_complete() && !log.events.is_empty());
    }
    audited.disable_learning_audit().unwrap();
    audited.enable_learning_audit(1).unwrap();
    audited.step(&white()).unwrap();
    assert!(audited.take_learning_audit().is_err());
}

#[test]
fn constant_learning_factor_preserves_all_frozen_chemistry_and_electrical_events() {
    use nerva_vm::model::{LearningRule, audit::AuditOptions};
    let sensor = SensorConfig {
        resolution: 8,
        coding: SpikeCoding::Deterministic,
        ..Default::default()
    };
    let mut c = config(true);
    c.processing_neurons = 16;
    let mut original =
        Model::with_sensor_and_learning(c.clone(), sensor, LearningRule::ThreeFactorAccumulated)
            .unwrap();
    let mut constant =
        Model::with_sensor_and_learning(c, sensor, LearningRule::ThreeFactorConstantPositive)
            .unwrap();
    for model in [&mut original, &mut constant] {
        model.freeze().unwrap();
        model.enable_audit(AuditOptions {
            raw_events: true,
            input_spikes: true,
        });
        model.enable_learning_replay_audit(100000).unwrap();
        model.set_dynamics_auditing(true);
    }
    for _ in 0..4 {
        let a = original.step(&white()).unwrap();
        let b = constant.step(&white()).unwrap();
        assert_eq!(a.core_events, b.core_events);
        assert_eq!(a.motor_events, b.motor_events);
        assert_eq!(original.network(), constant.network());
        assert_eq!(original.take_audit(), constant.take_audit());
        assert_eq!(
            original.take_learning_audit().unwrap(),
            constant.take_learning_audit().unwrap()
        );
    }
}

#[test]
fn checkpoint_competition_is_explicit_and_keeps_the_developed_graph_fixed() {
    use nerva_vm::model::PlasticitySettings;
    let mut c = config(true);
    c.processing_neurons = 16;
    let mut model = build(c);
    assert!(model.install_checkpoint_competition().is_err());
    model.freeze().unwrap();
    let before = model.network().clone();
    model.install_checkpoint_competition().unwrap();
    assert_eq!(model.network(), &before);
    assert_eq!(model.competition_pools().len(), 16);
    model
        .set_plasticity(PlasticitySettings {
            learning: true,
            development: false,
            homeostasis: true,
        })
        .unwrap();
    assert!(
        model
            .set_plasticity(PlasticitySettings {
                learning: true,
                development: true,
                homeostasis: true
            })
            .is_err()
    );
    assert!(!model.plasticity().development);
    model.step(&white()).unwrap();
}
