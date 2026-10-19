use std::{
    fs,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
#[ignore = "requires QEMU, binutils and local Unix sockets; run explicitly"]
fn real_vga_guest_responds_to_keys_and_neural_pixel_feedback() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let output_dir =
        std::env::temp_dir().join(format!("nerva-visual-test-{}-{unique}", std::process::id()));
    let output = Command::new(env!("CARGO_BIN_EXE_nerva-vm"))
        .args(["smoke", "--steps", "40", "--output"])
        .arg(&output_dir)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}\nartifacts: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
        output_dir.display()
    );
    let report: serde_json::Value =
        serde_json::from_slice(&fs::read(output_dir.join("report.json")).unwrap()).unwrap();
    for field in [
        "sensory_spikes",
        "motor_spikes",
        "key_frames",
        "weight_changes",
    ] {
        assert!(
            report[field].as_u64().unwrap() > 0,
            "missing {field}: {report}"
        );
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("QMP_KEYBOARD_PASS"));
    assert!(stdout.contains("QMP_FULL_KEYBOARD_PASS"));
    assert_eq!(report["motor_channels"], 105);
    assert_eq!(report["keyboard_layout"], "de-iso105");
    assert_eq!(report["keyboard_feedback_verified"], true);
    assert_eq!(report["full_layout_verified"], true);
    assert_eq!(report["processing_neurons"], 256);
    assert_eq!(report["model_kind"], "Recurrent");
    for field in [
        "core_spikes",
        "formed",
        "pruned",
        "raw_motor_feedback_arrivals",
        "executed_action_copy_arrivals",
    ] {
        assert!(
            report[field].as_u64().unwrap() > 0,
            "missing recurrent {field}: {report}"
        );
    }
    assert_eq!(
        fs::read_to_string(output_dir.join("keyboard-layout.csv"))
            .unwrap()
            .lines()
            .count(),
        106
    );
    assert!(stdout.contains("VISUAL_CONTROL_PASS"));
    fs::remove_dir_all(output_dir).unwrap();
}
