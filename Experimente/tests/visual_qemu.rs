use std::{
    fs,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
#[ignore = "requires QEMU, binutils and local Unix sockets; run explicitly"]
fn arbitrary_ad_recording_supports_a_reproducible_train_freeze_probe_command() {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory = std::env::temp_dir().join(format!(
        "nerva-retention-qemu-{}-{stamp}",
        std::process::id()
    ));
    let train = directory.join("training");
    let test = directory.join("probe");
    let binary = env!("CARGO_BIN_EXE_nerva-experiments");
    for (actions, steps, output) in [("A,A,D,D", "8", &train), ("A,D,D,D,A,A,D,A", "16", &test)] {
        let result = Command::new(binary)
            .args(["record", "--actions", actions, "--steps", steps, "--output"])
            .arg(output)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    for name in ["first", "replay"] {
        let result = Command::new(binary)
            .arg("retention")
            .arg(&train)
            .arg(&test)
            .args([
                "--train-frames",
                "12",
                "--processing",
                "16",
                "--seeds",
                "7",
                "--output",
            ])
            .arg(directory.join(name))
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(String::from_utf8_lossy(&result.stdout).contains("RETENTION_PASS"));
    }
    for file in [
        "retention.json",
        "retention-core.csv",
        "retention-spikes.csv",
        "retention-training.csv",
        "retention-topology.csv",
        "retention-neurons.csv",
        "retention-summary.md",
    ] {
        assert_eq!(
            fs::read(directory.join("first").join(file)).unwrap(),
            fs::read(directory.join("replay").join(file)).unwrap()
        );
    }
    fs::remove_dir_all(directory).unwrap();
}

#[test]
#[ignore = "requires QEMU, binutils and local Unix sockets; run explicitly"]
fn recorded_ad_study_replays_identically_in_a_separate_process() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory =
        std::env::temp_dir().join(format!("nerva-qemu-study-{}-{unique}", std::process::id()));
    let source = directory.join("recorded");
    let replay = directory.join("replayed");
    let binary = env!("CARGO_BIN_EXE_nerva-experiments");
    let captured = Command::new(binary)
        .args(["study", "--steps", "8", "--seeds", "7", "--output"])
        .arg(&source)
        .output()
        .unwrap();
    assert!(
        captured.status.success(),
        "{} {}",
        String::from_utf8_lossy(&captured.stdout),
        String::from_utf8_lossy(&captured.stderr)
    );
    let repeated = Command::new(binary)
        .arg("study-replay")
        .arg(&source)
        .args(["--seeds", "7", "--output"])
        .arg(&replay)
        .output()
        .unwrap();
    assert!(
        repeated.status.success(),
        "{} {}",
        String::from_utf8_lossy(&repeated.stdout),
        String::from_utf8_lossy(&repeated.stderr)
    );
    assert_eq!(
        fs::read(source.join("study.json")).unwrap(),
        fs::read(replay.join("study.json")).unwrap()
    );
    assert_eq!(
        fs::read(source.join("study-core.csv")).unwrap(),
        fs::read(replay.join("study-core.csv")).unwrap()
    );
    assert_eq!(
        fs::read(source.join("study-spikes.csv")).unwrap(),
        fs::read(replay.join("study-spikes.csv")).unwrap()
    );
    fs::remove_dir_all(directory).unwrap();
}
