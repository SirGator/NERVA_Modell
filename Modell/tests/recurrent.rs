use nerva::core::{NeuronRole, Polarity};
use nerva_qemu::{capture::Frame, keyboard::Key, vision_encoder::Encoding};
use nerva_vm::model::{KeyboardLayout, Model, ModelConfig, ModelKind};

fn config() -> ModelConfig {
    ModelConfig {
        processing_neurons: 32,
        keyboard: KeyboardLayout::Wasd,
        ..ModelConfig::default()
    }
}

#[test]
fn default_core_is_sparse_spatial_and_has_no_sensor_to_motor_shortcut() {
    let model = Model::from_config(ModelConfig::default()).unwrap();
    let network = model.network();
    assert_eq!(model.processing_ids().len(), 256);
    assert_eq!(
        network
            .neurons()
            .filter(|neuron| neuron.role() == Some(NeuronRole::Motor))
            .count(),
        105
    );
    assert_eq!(
        model
            .processing_ids()
            .iter()
            .filter(|id| network.neuron(**id).unwrap().polarity() == Polarity::Inhibitory)
            .count(),
        51
    );
    assert!(model.core_synapse_count() <= 256 * 4);
    assert!(model.core_synapse_count() > 0);
    for synapse in network.synapses() {
        let source = network.neuron(synapse.pre()).unwrap();
        let target = network.neuron(synapse.post()).unwrap();
        assert!(
            !(source.role() == Some(NeuronRole::Sensory)
                && target.role() == Some(NeuronRole::Motor))
        );
        if source.role() == Some(NeuronRole::Processing)
            && target.role() == Some(NeuronRole::Processing)
        {
            assert!(source.position().distance_to(target.position()) <= 0.25);
            assert!(synapse.is_plastic());
        }
    }
    let core = network.neuron(model.processing_ids()[0]).unwrap();
    assert!(
        network
            .neurons()
            .filter(|neuron| neuron.role() == Some(NeuronRole::Motor))
            .all(|neuron| neuron.position().distance_to(core.position()) > 0.25)
    );
}

#[test]
fn development_changes_only_core_edges_and_preserves_a_paired_fixed_control() {
    let mut control_config = config();
    control_config.development = false;
    let mut developing = Model::from_config(config()).unwrap();
    let mut fixed = Model::from_config(control_config).unwrap();
    assert_eq!(developing.network(), fixed.network());
    let initial_edges: Vec<_> = fixed
        .network()
        .synapses()
        .map(|synapse| (synapse.id(), synapse.pre(), synapse.post()))
        .collect();
    let initial_core = developing.core_synapse_count();
    let white = Frame::new(64, 64, vec![255; 64 * 64 * 3]).unwrap();
    let mut formations = 0;
    for _ in 0..8 {
        let step = developing.step(&white).unwrap();
        let fixed_step = fixed.step(&white).unwrap();
        assert!(fixed_step.formed.is_empty() && fixed_step.pruned.is_empty());
        for &(_, source, target) in &step.formed {
            let source = developing
                .network()
                .neuron(nerva::core::NeuronId(source))
                .unwrap();
            let target = developing
                .network()
                .neuron(nerva::core::NeuronId(target))
                .unwrap();
            assert_eq!(source.role(), Some(NeuronRole::Processing));
            assert_eq!(target.role(), Some(NeuronRole::Processing));
            assert!(source.position().distance_to(target.position()) <= 0.25);
        }
        formations += step.formed.len();
    }
    assert!(formations > 0);
    assert_eq!(fixed.core_synapse_count(), initial_core);
    assert_eq!(
        initial_edges,
        fixed
            .network()
            .synapses()
            .map(|synapse| (synapse.id(), synapse.pre(), synapse.post()))
            .collect::<Vec<_>>()
    );
}

#[test]
fn feedback_copies_are_self_action_observations_and_can_be_disabled() {
    let black = Frame::new(64, 64, vec![0; 64 * 64 * 3]).unwrap();
    let mut enabled = Model::from_config(config()).unwrap();
    let mut disabled_config = config();
    disabled_config.feedback = false;
    let mut disabled = Model::from_config(disabled_config).unwrap();
    for _ in 0..8 {
        enabled.record_executed_keys(&[Key::A]).unwrap();
        disabled.record_executed_keys(&[Key::A]).unwrap();
        assert_eq!(enabled.step(&black).unwrap().action_copy_arrivals, 3);
        let no_feedback = disabled.step(&black).unwrap();
        assert_eq!(no_feedback.action_copy_arrivals, 0);
        assert_eq!(no_feedback.feedback_arrivals, 0);
    }
    assert!(enabled.record_executed_keys(&[Key::F12]).is_err());
}

#[test]
fn direct_control_retains_original_wasd_behavior() {
    let mut model = Model::from_config(ModelConfig {
        kind: ModelKind::Direct,
        keyboard: KeyboardLayout::Wasd,
        encoding: Encoding::Brightness,
        ..ModelConfig::default()
    })
    .unwrap();
    assert!(model.processing_ids().is_empty());
    assert!(!model.config().development && !model.config().homeostasis && !model.config().feedback);
    assert_eq!(model.network().neuron_count(), 4096 + 5);
    assert_eq!(model.network().synapse_count(), 8192);
    assert!(
        model
            .step(&Frame::new(64, 64, vec![255; 64 * 64 * 3]).unwrap())
            .unwrap()
            .motor_spikes
            > 0
    );
}
