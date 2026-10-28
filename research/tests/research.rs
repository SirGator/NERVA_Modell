use nerva::experiment::{ExperimentRunner, M0ExperimentConfig, M0StudyConfig};
use nerva_research::{
    Result,
    collector::ResultCollector,
    error,
    runner::ResearchRunner,
    scenario::{Intervention, Scenario, TrialMetrics},
    scenarios::m0::M0Scenario,
    spec::ExperimentSpec,
};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!(
                "nerva-research-test-{}-{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            )))
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn m0_spec() -> Value {
    json!({"experiment":{"name":"m0_test","scenario":"m0_sequence","seeds":[7,19,41,73,101,149]},"measurements":{"metrics":["sequence_score","stability","firing_rate"]}})
}

#[test]
fn m0_adapter_matches_native_study_and_its_existing_decision() {
    let root = Temp::new();
    let spec = ExperimentSpec::parse(m0_spec()).unwrap();
    let runner = ResearchRunner {
        scenario: M0Scenario,
    };
    let summary = runner.execute(&spec, &root.0, false).unwrap();
    let native = ExperimentRunner::new(M0ExperimentConfig::default())
        .unwrap()
        .run_study(&M0StudyConfig::default())
        .unwrap();
    let decision = &summary["variants"][0]["decision"];
    assert_eq!(decision["passed"], native.success.passed);
    assert_eq!(decision["wins_over_fixed"], native.success.wins_over_fixed);
    assert_eq!(
        decision["wins_over_random"],
        native.success.wins_over_random
    );
    for (name, value) in [
        (
            "mean_advantage_over_fixed",
            native.success.mean_advantage_over_fixed,
        ),
        (
            "mean_advantage_over_random",
            native.success.mean_advantage_over_random,
        ),
        (
            "sign_test_p_over_fixed",
            native.success.sign_test_p_over_fixed,
        ),
        (
            "sign_test_p_over_random",
            native.success.sign_test_p_over_random,
        ),
    ] {
        assert!((decision[name].as_f64().unwrap() - value as f64).abs() < 1e-6);
    }
    for (index, comparison) in native.comparisons.iter().enumerate() {
        for (group, result) in comparison.groups.iter().enumerate() {
            let path = root.0.join(format!(
                "records/v00000-s{}-i{group:03}.json",
                spec.seeds[index]
            ));
            let record: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
            assert_eq!(
                record["record"]["metrics"]["sequence_score"],
                result.metrics.sequence_score() as f64
            );
            assert_eq!(
                record["record"]["details"]["event_log_digest"],
                result.event_log_digest
            );
            assert_eq!(
                record["record"]["details"]["effective_config"],
                format!("{:?}", result.effective_config)
            );
        }
    }
}

#[test]
fn example_grid_counts_all_control_runs_and_rejects_invalid_input_before_execution() {
    let runner = ResearchRunner {
        scenario: M0Scenario,
    };
    let mut v = m0_spec();
    v["parameters"] = json!([
        {"name":"learning.a_plus","values":[0.01,0.02,0.04,0.08]},
        {"name":"learning.a_minus","values":[0.01,0.02,0.04]}]);
    let plan = runner
        .plan(&ExperimentSpec::parse(v.clone()).unwrap())
        .unwrap();
    assert_eq!(
        plan["counts"],
        json!({"variants":12,"seeds":6,"interventions":4,"trials":288,"full_executions":576})
    );
    for mutate in 0..7 {
        let mut bad = v.clone();
        match mutate {
            0 => bad["experiment"]["seeds"] = json!([7, 7]),
            1 => bad["parameters"][0]["values"] = json!([]),
            2 => bad["parameters"][0]["name"] = json!("learning.typo"),
            3 => bad["measurements"]["metrics"] = json!(["unknown_metric"]),
            4 => bad["experiment"]["interventions"] = json!(["block_synapses"]),
            5 => bad["experiment"]["max_runs"] = json!(10),
            _ => bad["typo"] = json!(true),
        }
        let result = ExperimentSpec::parse(bad).and_then(|s| runner.plan(&s));
        assert!(result.is_err(), "invalid case {mutate} accepted");
    }
    let mut rounded = m0_spec();
    rounded["parameters"] = json!([{ "name":"learning.a_plus","values":[0.04,0.04000000001]}]);
    assert!(
        runner
            .plan(&ExperimentSpec::parse(rounded).unwrap())
            .is_err()
    );
}

struct Toy {
    calls: AtomicUsize,
    varying: bool,
}
impl Scenario for Toy {
    type Config = f64;
    fn name(&self) -> &'static str {
        "toy_environment"
    }
    fn default_config(&self) -> f64 {
        1.0
    }
    fn set_parameter(&self, c: &mut f64, name: &str, v: &Value) -> Result<()> {
        if name != "x" {
            return Err(error("unknown toy parameter"));
        }
        *c = v.as_f64().ok_or_else(|| error("x must be numeric"))?;
        Ok(())
    }
    fn validate(&self, c: &f64) -> Result<()> {
        if c.is_finite() {
            Ok(())
        } else {
            Err(error("nonfinite toy config"))
        }
    }
    fn snapshot(&self, c: &f64, seed: u64, i: &Intervention) -> Value {
        json!({"x":c,"seed":seed,"mode":i.name})
    }
    fn interventions(&self) -> Vec<Intervention> {
        ["plain", "offset"]
            .iter()
            .map(|s| Intervention {
                name: (*s).to_owned(),
            })
            .collect()
    }
    fn metric_names(&self) -> Vec<&'static str> {
        vec!["square"]
    }
    fn run(&self, c: &f64, seed: u64, i: &Intervention) -> Result<TrialMetrics> {
        let call = self.calls.fetch_add(1, Ordering::Relaxed);
        Ok(TrialMetrics {
            values: BTreeMap::from([(
                "square".to_owned(),
                c * c
                    + seed as f64
                    + f64::from(i.name == "offset")
                    + if self.varying { call as f64 } else { 0.0 },
            )]),
            details: json!({"fixture":true}),
        })
    }
}
fn toy_spec() -> ExperimentSpec {
    ExperimentSpec::parse(json!({"experiment":{"name":"toy","scenario":"toy_environment","seeds":[1,2]},"parameters":[{"name":"x","values":[2,3]}],"measurements":{"metrics":["square"]}})).unwrap()
}

#[test]
fn generic_runner_supports_a_different_scenario_resumes_and_detects_corruption() {
    let root = Temp::new();
    let runner = ResearchRunner {
        scenario: Toy {
            calls: AtomicUsize::new(0),
            varying: false,
        },
    };
    let spec = toy_spec();
    let summary = runner.execute(&spec, &root.0, false).unwrap();
    assert_eq!(runner.scenario.calls.load(Ordering::Relaxed), 16);
    assert!(summary["variants"][0]["decision"]["passed"].is_null());
    assert_eq!(runner.execute(&spec, &root.0, false).unwrap(), summary);
    assert_eq!(runner.scenario.calls.load(Ordering::Relaxed), 16);
    let missing = root.0.join("records/v00000-s1-i000.json");
    fs::remove_file(&missing).unwrap();
    runner.execute(&spec, &root.0, false).unwrap();
    assert_eq!(runner.scenario.calls.load(Ordering::Relaxed), 18);
    let mut damaged: Value = serde_json::from_slice(&fs::read(&missing).unwrap()).unwrap();
    damaged["record"]["metrics"]["square"] = json!(999);
    fs::write(&missing, serde_json::to_vec(&damaged).unwrap()).unwrap();
    assert!(
        runner
            .execute(&spec, &root.0, false)
            .unwrap_err()
            .to_string()
            .contains("integrity")
    );
}

#[test]
fn replay_mismatch_is_not_cached_and_different_plans_cannot_resume() {
    let root = Temp::new();
    let runner = ResearchRunner {
        scenario: Toy {
            calls: AtomicUsize::new(0),
            varying: true,
        },
    };
    assert!(
        runner
            .execute(&toy_spec(), &root.0, false)
            .unwrap_err()
            .to_string()
            .contains("replay differs")
    );
    assert_eq!(fs::read_dir(root.0.join("records")).unwrap().count(), 0);
    let mut changed = toy_spec();
    changed.source["experiment"]["name"] = json!("another");
    assert!(
        runner
            .execute(&changed, &root.0, false)
            .unwrap_err()
            .to_string()
            .contains("manifest differs")
    );
}

#[test]
fn output_lock_and_nonempty_directories_are_protected() {
    let root = Temp::new();
    let plan = json!({"test":true});
    let first = ResultCollector::open(&root.0, &plan).unwrap();
    assert!(ResultCollector::open(&root.0, &plan).is_err());
    drop(first);
    assert!(ResultCollector::open(&root.0, &plan).is_ok());
    let other = Temp::new();
    fs::create_dir(&other.0).unwrap();
    fs::write(other.0.join("summary.json"), "user data").unwrap();
    assert!(ResultCollector::open(&other.0, &plan).is_err());
    assert_eq!(
        fs::read_to_string(other.0.join("summary.json")).unwrap(),
        "user data"
    );
}
