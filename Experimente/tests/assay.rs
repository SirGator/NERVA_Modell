use nerva_experiments::{
    experiment::{RecordedFrame, Recording, run_study},
    model::{KeyboardLayout, ModelConfig},
};
use nerva_qemu::{capture::Frame, keyboard::Key};
use std::{
    fs,
    sync::atomic::AtomicBool,
    time::{SystemTime, UNIX_EPOCH},
};

fn recording() -> Recording {
    Recording {
        frames: (0..8)
            .map(|index| RecordedFrame {
                frame: Frame::new(
                    64,
                    64,
                    vec![if index % 4 < 2 { 255 } else { 0 }; 64 * 64 * 3],
                )
                .unwrap(),
                executed_key: [Key::A, Key::A, Key::D, Key::D][index % 4],
            })
            .collect(),
    }
}

#[test]
fn saved_recording_replays_with_a_matched_topology_control() {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory =
        std::env::temp_dir().join(format!("nerva-assay-{}-{stamp}", std::process::id()));
    fs::create_dir_all(directory.join("frames")).unwrap();
    let recording = recording();
    for (index, frame) in recording.frames.iter().enumerate() {
        let mut bytes = b"P6\n64 64\n255\n".to_vec();
        bytes.extend_from_slice(frame.frame.rgb());
        fs::write(directory.join(format!("frames/{index:06}.ppm")), bytes).unwrap();
    }
    recording.write_manifest(&directory).unwrap();
    let loaded = Recording::load(&directory).unwrap();
    assert_eq!(recording, loaded);
    let config = ModelConfig {
        processing_neurons: 16,
        keyboard: KeyboardLayout::Wasd,
        ..ModelConfig::default()
    };
    let report = run_study(&loaded, &config, &[11], &directory, &AtomicBool::new(false)).unwrap();
    assert_eq!(report["all_exact_replays"], true);
    assert_eq!(report["results"][0]["same_initial_network"], true);
    assert_eq!(report["results"][0]["fixed_topology"]["formed"], 0);
    assert_eq!(report["results"][0]["fixed_topology"]["pruned"], 0);
    assert_eq!(
        fs::read_to_string(directory.join("study-core.csv"))
            .unwrap()
            .lines()
            .count(),
        1 + 2 * 8 * 16
    );
    assert!(
        fs::read_to_string(directory.join("study-spikes.csv"))
            .unwrap()
            .lines()
            .count()
            > 1
    );
    assert!(directory.join("study-summary.md").exists());
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn incomplete_or_mislabeled_action_sequences_are_rejected() {
    let mut data = recording();
    data.validate().unwrap();
    data.frames[0].executed_key = Key::D;
    assert!(data.validate().is_err());
    data.frames.clear();
    assert!(data.validate().is_err());
}
