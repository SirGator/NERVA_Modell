use nerva::core::NeuronRole;
use nerva_qemu::{
    keyboard::Key,
    test_world::{MotionWorld, PixelEnvironment},
    vision_encoder::{Encoding, VisionEncoder},
};
use nerva_vm::model::{
    KeyboardLayout, Model, ModelConfig, SensorConfig, SensorMode, SpikeCoding,
    audit::{AuditOptions, learned_parameters_equal},
    sensory::SensorEncoder,
};

fn image(position: u8) -> nerva_qemu::capture::Frame {
    MotionWorld::new(position, false)
        .unwrap()
        .observe()
        .unwrap()
}
fn model_config() -> ModelConfig {
    ModelConfig {
        processing_neurons: 16,
        keyboard: KeyboardLayout::Ad,
        ..ModelConfig::default()
    }
}

#[test]
fn diagnostic_sensors_are_current_frame_anchors_and_area_sampling_preserves_all_seven_positions() {
    for resolution in [8, 16, 64] {
        for mode in [SensorMode::Pixels, SensorMode::Position7Ch] {
            let sensor = SensorEncoder::new(SensorConfig {
                mode,
                resolution,
                ..SensorConfig::default()
            })
            .unwrap();
            let features: Vec<_> = (0..7)
                .map(|p| sensor.features(&image(p)).unwrap())
                .collect();
            for a in 0..7 {
                for b in a + 1..7 {
                    assert_ne!(
                        features[a], features[b],
                        "{mode:?}/{resolution}: {a} aliases {b}"
                    );
                }
            }
            if mode == SensorMode::Position7Ch {
                for (position, feature) in features.iter().enumerate() {
                    assert_eq!(feature.iter().sum::<f32>(), 1.0);
                    assert_eq!(feature[position], 1.0);
                }
            }
        }
        let sensor = SensorEncoder::new(SensorConfig {
            mode: SensorMode::Effect2Ch,
            resolution,
            ..SensorConfig::default()
        })
        .unwrap();
        assert_eq!(sensor.features(&image(3)).unwrap(), vec![0.0, 0.0]);
        assert_eq!(sensor.features(&image(2)).unwrap(), vec![1.0, 0.0]);
        assert_eq!(sensor.features(&image(4)).unwrap(), vec![0.0, 1.0]);
    }
}

#[test]
fn diagnostic_and_pixel_populations_use_correct_ids_without_overriding_zero_start_drive() {
    for mode in [
        SensorMode::Pixels,
        SensorMode::Effect2Ch,
        SensorMode::Position7Ch,
    ] {
        for coding in [
            SpikeCoding::Deterministic,
            SpikeCoding::Poisson,
            SpikeCoding::OnOff,
        ] {
            let sensor = SensorConfig {
                mode,
                coding,
                resolution: 8,
                ..SensorConfig::default()
            };
            let mut model = Model::with_sensor(
                ModelConfig {
                    initial_structural_drive: 0.0,
                    homeostasis: false,
                    ..model_config()
                },
                sensor,
            )
            .unwrap();
            assert_eq!(model.network().neuron_count(), sensor.channels() + 16 + 4);
            assert_eq!(model.processing_ids()[0].0, sensor.channels() as u64);
            for neuron in model
                .network()
                .neurons()
                .filter(|n| n.role() == Some(NeuronRole::Processing))
            {
                assert_eq!(neuron.structural_drive(), 0.0);
            }
            for _ in 0..4 {
                let step = model.step(&image(4)).unwrap();
                assert!(step.formed.is_empty() && step.pruned.is_empty());
            }
        }
    }
}

#[test]
fn legacy_pixel_rates_and_initial_network_are_preserved_and_unknown_resolutions_are_rejected() {
    assert_eq!(
        "effect_2ch".parse::<SensorMode>().unwrap(),
        SensorMode::Effect2Ch
    );
    assert_eq!(
        "deterministic".parse::<SpikeCoding>().unwrap(),
        SpikeCoding::Deterministic
    );
    assert!("position_8ch".parse::<SensorMode>().is_err());
    assert!("poission".parse::<SpikeCoding>().is_err());
    for mode in [Encoding::Brightness, Encoding::Change] {
        let coding = if mode == Encoding::Brightness {
            SpikeCoding::Poisson
        } else {
            SpikeCoding::OnOff
        };
        let sensor = SensorConfig {
            coding,
            ..SensorConfig::default()
        };
        let mut previous = VisionEncoder::new(64, 64, 120.0, mode).unwrap();
        let mut new = SensorEncoder::new(sensor).unwrap();
        for position in [3, 2, 2, 4, 3] {
            assert_eq!(
                new.encode(&image(position)).unwrap(),
                previous.encode(&image(position)).unwrap()
            );
        }
        let old_model = Model::from_config(ModelConfig {
            encoding: mode,
            ..model_config()
        })
        .unwrap();
        let explicit = Model::with_sensor(model_config(), sensor).unwrap();
        assert_eq!(old_model.network(), explicit.network());
    }
    for resolution in [0, 7, 32, 65] {
        assert!(
            SensorEncoder::new(SensorConfig {
                resolution,
                ..SensorConfig::default()
            })
            .is_err()
        );
    }
}

#[test]
fn deterministic_and_poisson_have_identical_rate_integrals_and_on_off_logs_its_actual_silent_baseline()
 {
    let mut expected = None;
    for coding in [
        SpikeCoding::Deterministic,
        SpikeCoding::Poisson,
        SpikeCoding::OnOff,
    ] {
        let sensor = SensorConfig {
            mode: SensorMode::Position7Ch,
            coding,
            resolution: 8,
            ..SensorConfig::default()
        };
        let mut model = Model::with_sensor(model_config().frozen(), sensor).unwrap();
        model.enable_audit(AuditOptions {
            input_spikes: true,
            ..AuditOptions::default()
        });
        model.step(&image(3)).unwrap();
        let first = model.take_audit().unwrap();
        if coding == SpikeCoding::OnOff {
            assert_eq!(first.expected_sensor_spikes, 0.0);
            assert_eq!(first.emitted_sensor_spikes, 0);
            model.step(&image(2)).unwrap();
            let change = model.take_audit().unwrap();
            assert_eq!(
                change
                    .input_rates
                    .iter()
                    .filter(|r| r.rate_hz > 0.0)
                    .count(),
                2
            );
            assert!(change.expected_sensor_spikes > first.expected_sensor_spikes);
        } else {
            if let Some(expected) = expected {
                assert_eq!(first.expected_sensor_spikes, expected);
            } else {
                expected = Some(first.expected_sensor_spikes);
            }
            assert!(first.expected_sensor_spikes > 5.9 && first.expected_sensor_spikes < 6.0);
            assert_eq!(first.emitted_sensor_spikes, first.input_spikes.len());
        }
    }
}

#[test]
fn audit_is_read_only_deterministic_and_exactly_matches_input_feedback_and_weight_counters() {
    let sensor = SensorConfig {
        mode: SensorMode::Position7Ch,
        coding: SpikeCoding::Deterministic,
        ..SensorConfig::default()
    };
    let mut observed = Model::with_sensor(model_config(), sensor).unwrap();
    let mut plain = Model::with_sensor(model_config(), sensor).unwrap();
    observed.enable_audit(AuditOptions {
        raw_events: true,
        input_spikes: true,
    });
    for position in [3, 2, 4, 3, 4, 2] {
        observed.record_executed_keys(&[Key::A]).unwrap();
        plain.record_executed_keys(&[Key::A]).unwrap();
        let a = observed.step(&image(position)).unwrap();
        let b = plain.step(&image(position)).unwrap();
        assert_eq!(a, b);
        assert_eq!(observed.network(), plain.network());
        let audit = observed.take_audit().unwrap();
        assert_eq!(audit.weight_updates, a.weight_changes);
        assert_eq!(audit.emitted_sensor_spikes, a.sensory_spikes);
        assert_eq!(audit.action_copy_arrivals, a.action_copy_arrivals);
        assert_eq!(audit.motor_feedback_arrivals, a.feedback_arrivals);
        assert_eq!(audit.formed, a.formed.len());
        assert_eq!(audit.pruned, a.pruned.len());
        assert!(!audit.raw_events.is_empty());
        assert!(
            audit
                .input_spikes
                .iter()
                .all(|s| s.at.0 >= a.start_us && s.at.0 <= a.end_us)
        );
        assert_eq!(observed.next_observation_us(), a.end_us + 1);
        assert!(observed.take_audit().is_none());
    }
}

#[test]
fn existing_rule_ablations_and_freeze_work_with_small_sensors() {
    let sensor = SensorConfig {
        mode: SensorMode::Effect2Ch,
        coding: SpikeCoding::Deterministic,
        ..SensorConfig::default()
    };
    let mut model = Model::with_sensor(
        ModelConfig {
            learning: false,
            feedback: false,
            development: false,
            ..model_config()
        },
        sensor,
    )
    .unwrap();
    model.enable_audit(AuditOptions::default());
    for position in [3, 2, 4, 3, 2, 4] {
        model.record_executed_keys(&[Key::D]).unwrap();
        let step = model.step(&image(position)).unwrap();
        assert_eq!(step.weight_changes, 0);
        assert_eq!(step.action_copy_arrivals, 0);
        assert_eq!(step.feedback_arrivals, 0);
        assert!(step.formed.is_empty() && step.pruned.is_empty());
    }
    model.freeze().unwrap();
    let frozen = model.network().clone();
    for position in [2, 4, 3] {
        model.step(&image(position)).unwrap();
        assert!(learned_parameters_equal(&frozen, model.network()));
    }
}
