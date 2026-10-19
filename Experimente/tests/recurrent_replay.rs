use nerva_experiments::{
    experiment::{RecordedFrame, Recording, replay},
    model::{KeyboardLayout, Model, ModelConfig},
};
use nerva_qemu::{capture::Frame, keyboard::Key};
use std::sync::atomic::AtomicBool;
fn config() -> ModelConfig {
    ModelConfig {
        processing_neurons: 32,
        keyboard: KeyboardLayout::Wasd,
        ..ModelConfig::default()
    }
}
#[test]
fn frozen_core_preserves_weights_and_topology_and_replay_is_exact() {
    let frozen = config().frozen();
    let model = Model::from_config(frozen.clone()).unwrap();
    let initial: Vec<_> = model
        .network()
        .synapses()
        .map(|synapse| {
            (
                synapse.id(),
                synapse.pre(),
                synapse.post(),
                synapse.weight(),
            )
        })
        .collect();
    let recording = Recording {
        frames: (0..4)
            .map(|index| RecordedFrame {
                frame: Frame::new(64, 64, vec![255; 64 * 64 * 3]).unwrap(),
                executed_key: [Key::A, Key::D][index % 2],
            })
            .collect(),
    };
    let stop = AtomicBool::new(false);
    let a = replay(model, &recording, &stop).unwrap();
    let b = replay(Model::from_config(frozen).unwrap(), &recording, &stop).unwrap();
    assert_eq!(a, b);
    assert!(a.steps.iter().all(|step| step.weight_changes == 0
        && step.homeostasis_changes == 0
        && step.formed.is_empty()
        && step.pruned.is_empty()));
    assert_eq!(
        initial,
        a.final_network
            .synapses()
            .map(|synapse| (
                synapse.id(),
                synapse.pre(),
                synapse.post(),
                synapse.weight()
            ))
            .collect::<Vec<_>>()
    );
}
