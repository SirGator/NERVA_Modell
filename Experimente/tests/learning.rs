use nerva_experiments::serial_model::VmModel;

#[test]
fn guest_supplied_associations_are_learned_and_inference_is_frozen() {
    // Different tasks prove that the model does not contain the guest's XOR rule.
    for targets in [[2, 3, 0, 1], [1, 0, 3, 2], [3, 1, 0, 2]] {
        let mut model = VmModel::new().unwrap();
        for input in 0..4 {
            assert_eq!(model.step(input, None).unwrap(), None);
        }
        let initial = model.weights();
        for _ in 0..32 {
            for (input, target) in targets.iter().enumerate() {
                model.step(input as u8, Some(*target)).unwrap();
            }
        }
        let trained = model.weights();
        assert_ne!(initial, trained);
        for _ in 0..4 {
            for (input, target) in targets.iter().enumerate() {
                assert_eq!(model.step(input as u8, None).unwrap(), Some(*target));
            }
        }
        assert_eq!(trained, model.weights());
    }
}

#[test]
fn weights_survive_restart_and_invalid_files_do_not_mutate_the_model() {
    let path = std::env::temp_dir().join(format!("nerva-vm-test-{}.weights", std::process::id()));
    let mut model = VmModel::new().unwrap();
    for _ in 0..32 {
        model.step(2, Some(1)).unwrap();
    }
    model.save(&path).unwrap();
    let mut restored = VmModel::new().unwrap();
    restored.load(&path).unwrap();
    assert_eq!(restored.weights(), model.weights());
    assert_eq!(restored.step(2, None).unwrap(), Some(1));
    let before = restored.weights();
    std::fs::write(&path, "NERVA_VM_WEIGHTS_V1\nNaN\n").unwrap();
    assert!(restored.load(&path).is_err());
    assert_eq!(before, restored.weights());
    std::fs::remove_file(path).unwrap();
}

#[test]
fn out_of_range_inputs_do_not_change_weights() {
    let mut model = VmModel::new().unwrap();
    let before = model.weights();
    assert!(model.step(4, None).is_err());
    assert!(model.step(0, Some(255)).is_err());
    assert_eq!(before, model.weights());
}
