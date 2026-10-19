use nerva_qemu::{
    keyboard::Key,
    motor_decoder::{AdRule, decode_ad},
    signals::MotorPulse,
    test_world::{MotionWorld, PixelEnvironment},
};
fn pulse(key: Key, time: u64) -> MotorPulse {
    MotorPulse {
        channel: key.channel(),
        at_us: time,
        amplitude: 1.0,
    }
}

#[test]
fn timing_breaks_only_nonempty_count_ties_without_channel_order_bias() {
    let events = vec![
        pulse(Key::A, 40),
        pulse(Key::D, 20),
        pulse(Key::A, 80),
        pulse(Key::D, 90),
    ];
    assert_eq!(decode_ad(&events, 1, 100, AdRule::Count, 0).unwrap(), None);
    assert_eq!(
        decode_ad(&events, 1, 100, AdRule::CountThenFirstSpike, 0).unwrap(),
        Some(Key::D)
    );
    let swapped: Vec<_> = events
        .iter()
        .map(|p| {
            pulse(
                if p.channel == Key::A.channel() {
                    Key::D
                } else {
                    Key::A
                },
                p.at_us,
            )
        })
        .collect();
    assert_eq!(
        decode_ad(&swapped, 1, 100, AdRule::CountThenFirstSpike, 0).unwrap(),
        Some(Key::A)
    );
    let simultaneous = vec![pulse(Key::A, 20), pulse(Key::D, 20)];
    assert_eq!(
        decode_ad(&simultaneous, 1, 100, AdRule::CountThenFirstSpike, 0).unwrap(),
        None
    );
    let more_a = vec![pulse(Key::D, 20), pulse(Key::A, 30), pulse(Key::A, 40)];
    assert_eq!(
        decode_ad(&more_a, 1, 100, AdRule::CountThenFirstSpike, 0).unwrap(),
        Some(Key::A)
    );
}

#[test]
fn random_tie_control_is_balanced_and_never_creates_activity_from_silence() {
    let events = vec![pulse(Key::A, 20), pulse(Key::D, 20)];
    let mut a = 0;
    let mut d = 0;
    for draw in 0..100 {
        match decode_ad(&events, 1, 100, AdRule::RandomTie, draw).unwrap() {
            Some(Key::A) => a += 1,
            Some(Key::D) => d += 1,
            _ => panic!("tie was unresolved"),
        };
        assert_eq!(
            decode_ad(&[], 1, 100, AdRule::RandomTie, draw).unwrap(),
            None
        );
    }
    assert_eq!((a, d), (50, 50));
    assert!(decode_ad(&[pulse(Key::W, 20)], 1, 100, AdRule::Count, 0).is_err());
    assert!(decode_ad(&[pulse(Key::A, 0)], 1, 100, AdRule::Count, 0).is_err());
    assert!(decode_ad(&[pulse(Key::A, 101)], 1, 100, AdRule::Count, 0).is_err());
    assert!(
        decode_ad(
            &[MotorPulse {
                amplitude: f32::NAN,
                ..pulse(Key::A, 20)
            }],
            1,
            100,
            AdRule::Count,
            0
        )
        .is_err()
    );
}

#[test]
fn goal_free_world_allows_indefinite_motion_and_reversal_has_only_visual_effects() {
    let mut normal = MotionWorld::new(3, false).unwrap();
    let mut reversed = MotionWorld::new(3, true).unwrap();
    assert_eq!(normal.observe().unwrap(), reversed.observe().unwrap());
    normal.act(Some(Key::A)).unwrap();
    reversed.act(Some(Key::A)).unwrap();
    assert_ne!(normal.observe().unwrap(), reversed.observe().unwrap());
    for _ in 0..30 {
        normal.act(Some(Key::D)).unwrap();
    }
    let boundary = normal.observe().unwrap();
    normal.act(Some(Key::D)).unwrap();
    assert_eq!(boundary, normal.observe().unwrap());
    assert!(normal.act(Some(Key::Space)).is_err());
    assert!(MotionWorld::new(7, false).is_err());
}
