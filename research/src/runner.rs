use crate::{
    Result,
    collector::{ResultCollector, csv_cell},
    error,
    scenario::{Scenario, TrialMetrics},
    spec::{ExperimentSpec, VariantGenerator},
};
use serde_json::{Value, json};
use std::{collections::BTreeSet, path::Path, time::Instant};

pub struct ResearchRunner<S: Scenario> {
    pub scenario: S,
}

impl<S: Scenario> ResearchRunner<S> {
    /// Validate every expanded configuration before creating output or running a trial.
    pub fn plan(&self, spec: &ExperimentSpec) -> Result<Value> {
        if spec.scenario != self.scenario.name() {
            return Err(error("scenario adapter does not match spec"));
        }
        let supported = self.scenario.interventions();
        if supported.is_empty() {
            return Err(error("scenario has no interventions"));
        }
        let interventions = spec
            .interventions
            .clone()
            .unwrap_or_else(|| supported.clone());
        if interventions.iter().any(|i| !supported.contains(i)) {
            return Err(error("unsupported intervention for scenario"));
        }
        let available = self.scenario.metric_names();
        if spec
            .metrics
            .iter()
            .any(|m| !available.contains(&m.as_str()))
        {
            return Err(error("unknown metric for scenario"));
        }
        let grid = VariantGenerator::generate(&spec.axes, spec.max_runs)?;
        let count = grid
            .len()
            .checked_mul(spec.seeds.len())
            .and_then(|n| n.checked_mul(interventions.len()))
            .filter(|&n| n <= spec.max_runs)
            .ok_or_else(|| error("expanded trial count exceeds max_runs"))?;
        let mut base = self.scenario.default_config();
        for (name, value) in &spec.base {
            self.scenario.set_parameter(&mut base, name, value)?;
        }
        let mut variants = Vec::new();
        let mut jobs = Vec::with_capacity(count);
        let mut unique = BTreeSet::new();
        for (index, parameters) in grid.iter().enumerate() {
            let mut config = base.clone();
            for (name, value) in parameters {
                self.scenario.set_parameter(&mut config, name, value)?;
            }
            self.scenario.validate(&config)?;
            let snapshot = self.scenario.snapshot(&config, 0, &interventions[0]);
            if !unique.insert(snapshot.to_string()) {
                return Err(error(
                    "grid contains duplicate effective configurations (including float rounding)",
                ));
            }
            let id = format!("v{index:05}");
            variants.push(json!({"id":id,"parameters":parameters}));
            for &seed in &spec.seeds {
                for (i, intervention) in interventions.iter().enumerate() {
                    jobs.push(
                        json!({"id":format!("{id}-s{seed}-i{i:03}"),"variant":id,"seed":seed,
                    "intervention":intervention.name,"parameters":parameters,
                    "effective_config":self.scenario.snapshot(&config, seed, intervention)}),
                    );
                }
            }
        }
        Ok(
            json!({"protocol":"NERVA-Research-v1","engine_version":env!("CARGO_PKG_VERSION"),
            "engine_build_fnv1a64":env!("NERVA_RESEARCH_BUILD"),"spec":spec.source,
            "variants":variants,"jobs":jobs,"counts":{"variants":grid.len(),"seeds":spec.seeds.len(),
            "interventions":interventions.len(),"trials":count,"full_executions":count*if spec.verify_replay {2} else {1}},
            "limits":["Configured sweep; no automatic selection or independent confirmation", "FNV fingerprints detect accidental corruption, not adversarial tampering"]}),
        )
    }
    pub fn execute(
        &self,
        spec: &ExperimentSpec,
        output: &Path,
        report_only: bool,
    ) -> Result<Value> {
        let plan = self.plan(spec)?;
        if report_only && !output.join("manifest.json").exists() {
            return Err(error("report requires an existing result directory"));
        }
        let store = ResultCollector::open(output, &plan)?;
        let mut records = Vec::new();
        for job in plan["jobs"].as_array().unwrap() {
            let id = job["id"].as_str().unwrap();
            let record = if let Some(record) = store.cached(id, job)? {
                self.validate_record(&record, spec)?;
                record
            } else {
                if report_only {
                    return Err(error(format!("missing result: {id}")));
                }
                let mut config = self.scenario.default_config();
                for (name, value) in &spec.base {
                    self.scenario.set_parameter(&mut config, name, value)?;
                }
                for (name, value) in job["parameters"].as_object().unwrap() {
                    self.scenario.set_parameter(&mut config, name, value)?;
                }
                let intervention = self
                    .scenario
                    .interventions()
                    .into_iter()
                    .find(|i| i.name == job["intervention"])
                    .unwrap();
                let start = Instant::now();
                let result = if spec.verify_replay {
                    self.scenario.run_verified(
                        &config,
                        job["seed"].as_u64().unwrap(),
                        &intervention,
                    )
                } else {
                    self.scenario
                        .run(&config, job["seed"].as_u64().unwrap(), &intervention)
                }
                .map_err(|e| error(format!("{id}: {e}")))?;
                self.validate_metrics(&result)?;
                let record = json!({"job":job,"metrics":result.values,"details":result.details,
                    "full_replay_checked":spec.verify_replay,"wall_time_seconds":start.elapsed().as_secs_f64()});
                store.save(id, &record)?;
                println!("COMPLETED {id}");
                record
            };
            records.push(record);
        }
        let mut variants = Vec::new();
        for variant in plan["variants"].as_array().unwrap() {
            let rows: Vec<_> = records
                .iter()
                .filter(|r| r["job"]["variant"] == variant["id"])
                .cloned()
                .collect();
            let mut summaries = Vec::new();
            for intervention in self.scenario.interventions() {
                let selected: Vec<_> = rows
                    .iter()
                    .filter(|r| r["job"]["intervention"] == intervention.name)
                    .collect();
                if selected.is_empty() {
                    continue;
                }
                for metric in &spec.metrics {
                    let values: Vec<_> = selected
                        .iter()
                        .map(|r| r["metrics"][metric].as_f64().unwrap())
                        .collect();
                    summaries.push(json!({"intervention":intervention.name,"metric":metric,"seeds":values.len(),
                        "mean":values.iter().sum::<f64>()/values.len() as f64,
                        "min":values.iter().copied().fold(f64::INFINITY,f64::min),
                        "max":values.iter().copied().fold(f64::NEG_INFINITY,f64::max)}));
                }
            }
            variants.push(
                json!({"id":variant["id"],"parameters":variant["parameters"],"metrics":summaries,
                "decision":self.scenario.evaluate(&rows, spec.verify_replay)?}),
            );
        }
        let report = json!({"protocol":"NERVA-Research-v1","experiment":spec.name,"counts":plan["counts"],"variants":variants});
        store.write_json("summary.json", &report)?;
        let mut csv = String::from("variant,seed,intervention,metric,value\n");
        for record in &records {
            for metric in &spec.metrics {
                csv.push_str(&format!(
                    "{},{},{},{},{}\n",
                    csv_cell(record["job"]["variant"].as_str().unwrap()),
                    record["job"]["seed"],
                    csv_cell(record["job"]["intervention"].as_str().unwrap()),
                    csv_cell(metric),
                    record["metrics"][metric]
                ));
            }
        }
        store.write("metrics.csv", csv.as_bytes())?;
        let mut markdown = format!(
            "# {}\n\n{} Varianten, {} Seeds, {} Kontrollgruppen; {} Versuchsläufe.\n\n| Variante | Entscheidung des Szenarios |\n|---|---|\n",
            spec.name,
            plan["counts"]["variants"],
            plan["counts"]["seeds"],
            plan["counts"]["interventions"],
            records.len()
        );
        for variant in report["variants"].as_array().unwrap() {
            markdown.push_str(&format!(
                "| {} | {} |\n",
                variant["id"].as_str().unwrap(),
                variant["decision"]["passed"]
            ));
        }
        markdown.push_str("\nEntscheidungen gelten nur für das jeweilige Szenario. Ein Parameterraster ist keine unabhängige Bestätigung; Mehrfachvergleiche sind nicht korrigiert. Laufzeiten sind Hostmessungen und kein deterministisches neuronales Ergebnis.\n");
        store.write("summary.md", markdown.as_bytes())?;
        Ok(report)
    }
    fn validate_metrics(&self, metrics: &TrialMetrics) -> Result<()> {
        let expected: BTreeSet<_> = self
            .scenario
            .metric_names()
            .into_iter()
            .map(str::to_owned)
            .collect();
        if metrics.values.keys().cloned().collect::<BTreeSet<_>>() != expected
            || metrics.values.values().any(|x| !x.is_finite())
        {
            return Err(error(
                "scenario returned missing, unknown or nonfinite metrics",
            ));
        }
        Ok(())
    }
    fn validate_record(&self, record: &Value, spec: &ExperimentSpec) -> Result<()> {
        let values = record["metrics"]
            .as_object()
            .ok_or_else(|| error("cached metrics missing"))?
            .iter()
            .map(|(k, v)| {
                v.as_f64()
                    .map(|v| (k.clone(), v))
                    .ok_or_else(|| error("cached metric is not finite numeric data"))
            })
            .collect::<Result<_>>()?;
        self.validate_metrics(&TrialMetrics {
            values,
            details: record["details"].clone(),
        })?;
        if record["full_replay_checked"] != spec.verify_replay {
            return Err(error("cached replay contract differs"));
        }
        Ok(())
    }
}
