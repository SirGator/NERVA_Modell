use nerva_experiments::{
    agency::{AgencyConfig, run_agency, scenarios},
    model::{KeyboardLayout, Model, ModelConfig, PlasticitySettings},
    retention::learned_parameters_equal,
};
use nerva_qemu::capture::Frame;
use std::{
    fs,
    process::Command,
    sync::atomic::AtomicBool,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn schedules_hold_out_goal_positions_balance_directions_and_only_reverse_the_rules() {
    let train = scenarios(7, 24, true, false);
    let normal = scenarios(7, 18, false, false);
    let reversed = scenarios(7, 18, false, true);
    assert_eq!(normal, scenarios(7, 18, false, false));
    assert!(train.iter().all(|s| [1, 3, 5].contains(&s.target)));
    assert!(
        normal
            .iter()
            .all(|s| [0, 2, 4, 6].contains(&s.target) && s.start.abs_diff(s.target) >= 2)
    );
    assert_eq!(normal.iter().filter(|s| s.start < s.target).count(), 9);
    for (a, b) in normal.iter().zip(&reversed) {
        assert_eq!((a.start, a.target), (b.start, b.target));
        assert!(!a.reversed && b.reversed);
    }
}

#[test]
fn frozen_model_resumes_existing_plasticity_without_rebuilding_its_network() {
    let config = ModelConfig {
        processing_neurons: 16,
        keyboard: KeyboardLayout::Ad,
        ..ModelConfig::default()
    };
    let mut model = Model::from_config(config.clone()).unwrap();
    let settings = model.plasticity();
    let white = Frame::new(64, 64, vec![255; 64 * 64 * 3]).unwrap();
    for _ in 0..4 {
        model.step(&white).unwrap();
    }
    let trained = model.network().clone();
    model.freeze().unwrap();
    assert_eq!(model.network(), &trained);
    for _ in 0..4 {
        assert_eq!(model.step(&white).unwrap().weight_changes, 0);
    }
    assert!(learned_parameters_equal(&trained, model.network()));
    model.set_plasticity(settings).unwrap();
    let changes: usize = (0..4)
        .map(|_| model.step(&white).unwrap().weight_changes)
        .sum();
    assert!(changes > 0);
    assert_eq!(model.plasticity(), settings);
    assert!(!learned_parameters_equal(&trained, model.network()));
    // A model initially frozen can also enable the same existing STDP rule.
    let mut initially_frozen = Model::from_config(config.frozen()).unwrap();
    initially_frozen
        .set_plasticity(PlasticitySettings {
            learning: true,
            development: false,
            homeostasis: false,
        })
        .unwrap();
    assert!(initially_frozen.step(&white).unwrap().weight_changes > 0);
}

#[test]
fn closed_loop_results_replay_exactly_and_neural_actions_follow_motor_activity() {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory =
        std::env::temp_dir().join(format!("nerva-agency-{}-{stamp}", std::process::id()));
    let a = directory.join("a");
    let b = directory.join("b");
    fs::create_dir_all(&a).unwrap();
    fs::create_dir(&b).unwrap();
    let config = AgencyConfig {
        model: ModelConfig {
            processing_neurons: 16,
            keyboard: KeyboardLayout::Ad,
            ..ModelConfig::default()
        },
        seeds: vec![7],
        train_episodes: 2,
        adapt_episodes: 2,
        eval_episodes: 2,
        episode_steps: 3,
        jobs: 1,
    };
    let first = run_agency(&config, &a, &AtomicBool::new(false)).unwrap();
    let second = run_agency(&config, &b, &AtomicBool::new(false)).unwrap();
    assert_eq!(first, second);
    assert_eq!(first["episodes_total"], 60);
    assert_eq!(first["criteria"]["feasibility_demonstrated"], false);
    for result in first["results"].as_array().unwrap() {
        for phase in [
            "before",
            "after_frozen",
            "reversed_before",
            "reversed_after",
        ] {
            let p = &result["phases"][phase];
            assert_eq!(p["plasticity_enabled"], false);
            assert_eq!(p["weight_changes"], 0);
            assert_eq!(p["formed"], 0);
            assert_eq!(p["pruned"], 0);
            if result["condition"] != "random" {
                assert_eq!(p["parameters_retained"], true);
            }
            assert!(p["steps"].as_u64().unwrap() <= 6);
        }
        if result["condition"] == "weight_only" {
            assert_eq!(result["phases"]["training"]["homeostasis_changes"], 0);
            assert_eq!(result["phases"]["training"]["formed"], 0);
        }
        if result["condition"] == "frozen" {
            assert_eq!(result["phases"]["training"]["weight_changes"], 0);
        }
    }
    for line in fs::read_to_string(a.join("agency-actions.csv"))
        .unwrap()
        .lines()
        .skip(1)
    {
        let c: Vec<_> = line.split(',').collect();
        if c[1] == "random" {
            continue;
        }
        let motor_a: usize = c[10].parse().unwrap();
        let motor_d: usize = c[11].parse().unwrap();
        let expected = if motor_a == motor_d && motor_a > 0 && c[12] != c[13] {
            if c[12].parse::<u64>().unwrap() < c[13].parse::<u64>().unwrap() {
                "a"
            } else {
                "d"
            }
        } else if motor_a == motor_d {
            "none"
        } else if motor_a > motor_d {
            "a"
        } else {
            "d"
        };
        assert_eq!(
            c[5], expected,
            "action was not selected by its neural motor output"
        );
    }
    for file in [
        "agency.json",
        "agency-summary.md",
        "agency-actions.csv",
        "agency-episodes.csv",
        "agency-learning.csv",
    ] {
        assert_eq!(
            fs::read(a.join(file)).unwrap(),
            fs::read(b.join(file)).unwrap()
        );
    }
    // Exercise CLI dispatch without QEMU or sockets.
    let output = Command::new(env!("CARGO_BIN_EXE_nerva-experiments"))
        .args([
            "agency",
            "--seeds",
            "7",
            "--processing",
            "16",
            "--train-episodes",
            "2",
            "--adapt-episodes",
            "2",
            "--eval-episodes",
            "2",
            "--episode-steps",
            "3",
            "--output",
        ])
        .arg(directory.join("cli"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read(a.join("agency.json")).unwrap(),
        fs::read(directory.join("cli/agency.json")).unwrap()
    );
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn invalid_experiment_configurations_are_rejected() {
    let mut config = AgencyConfig {
        seeds: vec![7, 7],
        ..AgencyConfig::default()
    };
    assert!(config.validate().is_err());
    config.seeds = vec![7];
    config.episode_steps = 0;
    assert!(config.validate().is_err());
    config.episode_steps = 6;
    config.train_episodes = 0;
    assert!(config.validate().is_err());
    config.train_episodes = 1;
    config.model.keyboard = KeyboardLayout::Full;
    assert!(config.validate().is_err());
}

#[test]
fn parallel_seeds_match_serial_measurements_exactly() {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory = std::env::temp_dir().join(format!(
        "nerva-agency-parallel-{}-{stamp}",
        std::process::id()
    ));
    let serial = directory.join("serial");
    let parallel = directory.join("parallel");
    fs::create_dir_all(&serial).unwrap();
    fs::create_dir(&parallel).unwrap();
    let mut config = AgencyConfig {
        seeds: vec![11, 7],
        train_episodes: 1,
        adapt_episodes: 1,
        eval_episodes: 1,
        episode_steps: 2,
        jobs: 1,
        model: ModelConfig {
            processing_neurons: 16,
            keyboard: KeyboardLayout::Ad,
            ..ModelConfig::default()
        },
    };
    let a = run_agency(&config, &serial, &AtomicBool::new(false)).unwrap();
    config.jobs = 2;
    let b = run_agency(&config, &parallel, &AtomicBool::new(false)).unwrap();
    assert_eq!(a, b);
    for name in [
        "agency.json",
        "agency-summary.md",
        "agency-actions.csv",
        "agency-episodes.csv",
        "agency-learning.csv",
    ] {
        assert_eq!(
            fs::read(serial.join(name)).unwrap(),
            fs::read(parallel.join(name)).unwrap()
        );
    }
    fs::remove_dir_all(directory).unwrap();
}
