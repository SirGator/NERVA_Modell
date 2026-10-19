use nerva_qemu::{
    keyboard::Key,
    test_world::{EpisodeSpec, MiniWorld, PixelEnvironment},
};

fn world(start: u8, target: u8, reversed: bool) -> MiniWorld {
    MiniWorld::new(EpisodeSpec {
        start,
        target,
        reversed,
    })
    .unwrap()
}

#[test]
fn seven_cells_bound_actions_and_reversal_changes_the_visible_effect() {
    let mut normal = world(3, 6, false);
    let mut reversed = world(3, 6, true);
    assert_eq!(normal.observe().unwrap(), reversed.observe().unwrap());
    normal.act(Some(Key::A)).unwrap();
    reversed.act(Some(Key::A)).unwrap();
    assert_eq!(normal.assess().distance, 4);
    assert_eq!(reversed.assess().distance, 2);
    assert_ne!(normal.observe().unwrap(), reversed.observe().unwrap());
    let mut left = world(0, 6, false);
    let first = left.observe().unwrap();
    for _ in 0..20 {
        left.act(Some(Key::A)).unwrap();
    }
    assert_eq!(left.observe().unwrap(), first);
    left.act(None).unwrap();
    assert_eq!(left.observe().unwrap(), first);
    assert!(left.act(Some(Key::Space)).is_err());
    assert_eq!(left.observe().unwrap(), first);
    for _ in 0..6 {
        left.act(Some(Key::D)).unwrap();
    }
    assert!(left.assess().reached);
    assert_eq!(left.assess().distance, 0);
    assert!(left.act(Some(Key::D)).is_err());
}

#[test]
fn goal_is_visible_in_grayscale_and_invalid_or_already_solved_tasks_are_rejected() {
    use nerva_qemu::vision_encoder::grayscale;
    let a = world(3, 0, false).observe().unwrap();
    let b = world(3, 6, false).observe().unwrap();
    assert_eq!((a.width(), a.height()), (64, 64));
    assert_ne!(
        grayscale(&a, 64, 64).unwrap(),
        grayscale(&b, 64, 64).unwrap()
    );
    for (start, target) in [(7, 0), (0, 7), (255, 2), (3, 3)] {
        assert!(
            MiniWorld::new(EpisodeSpec {
                start,
                target,
                reversed: false
            })
            .is_err()
        );
    }
}
