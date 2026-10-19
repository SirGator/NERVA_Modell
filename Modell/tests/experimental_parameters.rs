use nerva_vm::model::{KeyboardLayout, Model, ModelConfig, SensorConfig, SensorMode, SpikeCoding};
fn build(config: ModelConfig) -> Model {
    Model::with_sensor(
        config,
        SensorConfig {
            mode: SensorMode::Effect2Ch,
            coding: SpikeCoding::Deterministic,
            ..SensorConfig::default()
        },
    )
    .unwrap()
}
fn config() -> ModelConfig {
    ModelConfig {
        processing_neurons: 32,
        keyboard: KeyboardLayout::Ad,
        initial_structural_drive: 0.0,
        ..ModelConfig::default().frozen()
    }
}
#[test]
fn increased_density_preserves_every_original_cell_and_edge_and_adds_only_local_core_edges() {
    let original = build(config());
    let mut dense_config = config();
    dense_config.core_fanout = 12;
    let dense = build(dense_config);
    assert!(original.network().neurons().eq(dense.network().neurons()));
    for edge in original.network().synapses() {
        assert_eq!(dense.network().synapse(edge.id()), Some(edge));
    }
    assert!(dense.network().synapse_count() > original.network().synapse_count());
    for edge in dense
        .network()
        .synapses()
        .filter(|s| original.network().synapse(s.id()).is_none())
    {
        assert!(
            dense.processing_ids().contains(&edge.pre())
                && dense.processing_ids().contains(&edge.post())
        );
        assert_ne!(edge.pre(), edge.post());
        let a = dense.network().neuron(edge.pre()).unwrap();
        let b = dense.network().neuron(edge.post()).unwrap();
        assert!(a.position().distance_to(b.position()) <= 0.25);
        assert_eq!(
            dense
                .network()
                .outgoing_synapses(edge.pre())
                .unwrap()
                .filter(|s| s.post() == edge.post())
                .count(),
            1
        );
    }
}
#[test]
fn experimental_strength_delay_threshold_and_tau_have_only_the_requested_initial_effects() {
    let base = build(config());
    for parameter in ["core_weight", "visual_weight", "delay", "threshold", "tau"] {
        let mut cfg = config();
        match parameter {
            "core_weight" => cfg.core_weight_scale = 4.0,
            "visual_weight" => cfg.sensory_weight_scale = 0.5,
            "delay" => cfg.core_delay_us = Some(5000),
            "threshold" => cfg.core_threshold = 0.6,
            "tau" => cfg.stdp_tau_us = 100000.0,
            _ => unreachable!(),
        }
        let changed = build(cfg);
        for edge in base.network().synapses() {
            let core = base.processing_ids().contains(&edge.pre())
                && base.processing_ids().contains(&edge.post());
            let actual = changed.network().synapse(edge.id()).unwrap();
            assert_eq!(
                (actual.pre(), actual.post(), actual.is_plastic()),
                (edge.pre(), edge.post(), edge.is_plastic())
            );
            let scale = if parameter == "core_weight" && core {
                4.0
            } else if parameter == "visual_weight" && edge.pre().0 < 2 {
                0.5
            } else {
                1.0
            };
            assert_eq!(actual.weight().get(), edge.weight().get() * scale);
            assert_eq!(
                actual.delay_us(),
                if parameter == "delay" && core {
                    5000
                } else {
                    edge.delay_us()
                }
            );
        }
        for n in base.network().neurons() {
            let actual = changed.network().neuron(n.id()).unwrap();
            assert_eq!(actual.position(), n.position());
            assert_eq!(actual.polarity(), n.polarity());
            assert_eq!(
                actual.threshold(),
                if parameter == "threshold" && base.processing_ids().contains(&n.id()) {
                    0.6
                } else {
                    n.threshold()
                }
            );
        }
        if parameter == "tau" {
            assert_eq!(base.network(), changed.network());
        }
    }
}
