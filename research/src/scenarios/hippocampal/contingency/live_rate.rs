//! Compare a local rate correction during uninterrupted A and B acquisition.
use super::{
    CUES, Config, ContingencyScenario, MAPPINGS, Model, OUTCOMES, Response, checkpoint, fail,
    frames, probe, schedule, stimuli,
};
use crate::{
    Result, error,
    scenario::{Intervention, Scenario, TrialMetrics},
};
use nerva_qemu::capture::Frame;
use nerva_vm::model::{
    DevelopmentPolicy, ExploratoryFormation, HomeostasisScope, PlasticitySettings,
    audit::AuditOptions,
};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

mod b_factor;
pub use b_factor::replay_b_factor;
pub mod growth;

pub struct LiveRateScenario;
const PROTOCOL: &str = "NERVA-Contingency-Live-Rate/1";
const RATE_TAU_US: f32 = 2_000_000.;
const POLICIES: [&str; 2] = ["native", "corrected"];
const TASKS: [&str; 2] = ["a", "b"];
const CUES_B: [&str; 2] = ["cue_b0", "cue_b1"];
const OUTCOMES_B: [&str; 2] = ["outcome_b0", "outcome_b1"];
const PROBES: [&str; 5] = ["blank", CUES[0], CUES[1], CUES_B[0], CUES_B[1]];
pub const PARAMETERS: [&str; 3] = ["processing_neurons", "training_repetitions", "rest_us"];
const METRICS: [&str; 8] = [
    "native_a",
    "corrected_a",
    "native_b",
    "corrected_b",
    "native_retained_a",
    "corrected_retained_a",
    "native_joint",
    "corrected_joint",
];

type Templates = [BTreeSet<u64>; 2];
type Responses = BTreeMap<&'static str, Response>;

fn patterns() -> BTreeMap<&'static str, Vec<usize>> {
    let mut images = stimuli();
    for fragment in super::FRAGMENTS {
        images.remove(fragment);
    }
    for (name, x, rows) in [
        (CUES_B[0], 0, [2, 3]),
        (CUES_B[1], 4, [2, 3]),
        (OUTCOMES_B[0], 0, [5, 6]),
        (OUTCOMES_B[1], 4, [6, 7]),
    ] {
        images.insert(
            name,
            rows.into_iter()
                .flat_map(|y| (x..x + 4).map(move |i| 8 * y + i))
                .collect(),
        );
    }
    images
}

fn in_task(name: &'static str, task: usize) -> &'static str {
    if task == 0 {
        return name;
    }
    match name {
        "cue0" => CUES_B[0],
        "cue1" => CUES_B[1],
        "outcome0" => OUTCOMES_B[0],
        "outcome1" => OUTCOMES_B[1],
        _ => name,
    }
}

fn topology(model: &Model) -> Value {
    json!(
        model
            .network()
            .synapses()
            .map(|s| (
                s.id().0,
                s.pre().0,
                s.post().0,
                s.weight().get(),
                s.delay_us(),
                s.is_enabled()
            ))
            .collect::<Vec<_>>()
    )
}

struct Assay<'a> {
    config: &'a Config,
    seed: u64,
    reference: Model,
    images: BTreeMap<&'static str, Frame>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TrainingAudit {
    Off,
    Standard,
    Formation,
    Lead,
    Contrast,
}

impl Assay<'_> {
    fn restore(&self) -> Result<Model> {
        let branch = checkpoint(self.config, self.seed)?.0;
        if !self.reference.same_replay_state(&branch) {
            return Err(error("live rate checkpoint differs before treatment"));
        }
        Ok(branch)
    }

    fn start_training(&self, policy: &str) -> Result<Model> {
        let mut model = self.restore()?;
        if policy == "corrected" {
            model
                .diagnostic_pair_rate_correction(Some(RATE_TAU_US))
                .map_err(fail)?;
        }
        model
            .set_checkpoint_homeostasis_scope(HomeostasisScope::Core)
            .map_err(fail)?;
        model
            .diagnostic_development_policy(DevelopmentPolicy {
                formation: ExploratoryFormation::RecentExcitatory,
                pruning: false,
            })
            .map_err(fail)?;
        model
            .set_plasticity(PlasticitySettings {
                learning: true,
                development: true,
                homeostasis: true,
            })
            .map_err(fail)?;
        Ok(model)
    }

    fn train(
        &self,
        model: &mut Model,
        mapping: &str,
        task: usize,
        mode: TrainingAudit,
    ) -> Result<Value> {
        if mode != TrainingAudit::Off {
            model.enable_audit(AuditOptions {
                raw_events: false,
                input_spikes: true,
            });
        }
        let mut rows = Vec::new();
        for name in schedule(self.config, self.seed, mapping, 0) {
            let stimulus = in_task(name, task);
            let step = model.step(&self.images[stimulus]).map_err(fail)?;
            if !step.pruned.is_empty() {
                return Err(error(
                    "live rate comparison removed edges under retention policy",
                ));
            }
            if mode != TrainingAudit::Off {
                let audit = model
                    .take_audit()
                    .ok_or_else(|| error("live rate training input missing"))?;
                let roots: Vec<_> = audit
                    .input_spikes
                    .iter()
                    .map(|s| (s.channel.0, s.at.0))
                    .collect();
                if roots.len() != step.sensory_spikes {
                    return Err(error("live rate detectors differ from audited root input"));
                }
                let mut row = json!({
                    "stimulus":stimulus,"start_us":step.start_us,"end_us":step.end_us,
                    "root_spikes":roots,"core_spikes":step.core_events,"formed":step.formed,"pruned":step.pruned,
                    "weight_changes":step.weight_changes, "homeostasis_changes":step.homeostasis_changes,
                });
                if matches!(
                    mode,
                    TrainingAudit::Formation | TrainingAudit::Lead | TrainingAudit::Contrast
                ) {
                    row["exploratory"] = json!(
                        step.exploratory
                            .iter()
                            .map(|decision| {
                                (
                                    decision.edge,
                                    decision.time_us,
                                    decision.source_last_us,
                                    decision.target_last_us,
                                    decision.recent_candidates,
                                    decision.drive,
                                )
                            })
                            .collect::<Vec<_>>()
                    );
                }
                if matches!(mode, TrainingAudit::Lead | TrainingAudit::Contrast) {
                    row["lead_choices"] = json!(
                        step.exploratory
                            .iter()
                            .map(|decision| (
                                decision.edge,
                                decision.recent_source,
                                decision.chosen_evidence,
                                decision.recent_evidence,
                            ))
                            .collect::<Vec<_>>()
                    );
                }
                if mode == TrainingAudit::Contrast {
                    row["contrast_components"] = json!(
                        step.exploratory
                            .iter()
                            .map(|decision| (
                                decision.edge,
                                decision.chosen_observed,
                                decision.chosen_lagged,
                                decision.recent_observed,
                                decision.recent_lagged,
                            ))
                            .collect::<Vec<_>>()
                    );
                }
                rows.push(row);
            }
        }
        Ok(json!(rows))
    }

    fn read(&self, branch: &mut Model, name: &str) -> Result<Response> {
        probe(
            branch,
            self.config,
            &self.images[name],
            &self.images["blank"],
            None,
        )
    }

    fn reset(&self) -> Result<Responses> {
        PROBES
            .into_iter()
            .chain(OUTCOMES)
            .chain(OUTCOMES_B)
            .map(|name| Ok((name, self.read(&mut self.restore()?, name)?)))
            .collect()
    }

    fn probe_stage(
        &self,
        parent: &Model,
        policy: &str,
        mapping: &str,
        task: usize,
    ) -> Result<Value> {
        let mut probes = BTreeMap::new();
        for name in PROBES {
            let mut fork = self.start_training(policy)?;
            for m in 0..=task {
                self.train(&mut fork, mapping, m, TrainingAudit::Off)?;
            }
            if !parent.same_replay_state(&fork) {
                return Err(error(
                    "live rate probe differs from its uninterrupted parent",
                ));
            }
            fork.freeze().map_err(fail)?;
            probes.insert(name, self.read(&mut fork, name)?.json());
        }
        Ok(json!(probes))
    }
}

fn templates(reset: &Responses, outcomes: [&str; 2]) -> Templates {
    let excluded: BTreeSet<_> = PROBES
        .iter()
        .flat_map(|&cue| reset[cue].core_spikes.iter().map(|&(id, _)| id))
        .collect();
    let active = outcomes.map(|name| {
        reset[name]
            .core_spikes
            .iter()
            .map(|&(id, _)| id)
            .collect::<BTreeSet<_>>()
    });
    std::array::from_fn(|i| {
        active[i]
            .iter()
            .filter(|&id| !excluded.contains(id) && !active[1 - i].contains(id))
            .copied()
            .collect()
    })
}

fn spikes(probe: &Value) -> &Vec<Value> {
    probe["core_spikes"].as_array().unwrap()
}

fn count(probe: &Value, group: &BTreeSet<u64>) -> usize {
    spikes(probe)
        .iter()
        .filter(|row| group.contains(&row[0].as_u64().unwrap()))
        .count()
}

fn score_mappings(
    probes: [&Value; 3],
    reset: &Value,
    targets: &Templates,
    task: usize,
    prior_b: Option<[&Value; 2]>,
    initial_a: Option<&Value>,
) -> Value {
    let mut family = BTreeMap::new();
    for (flip, mapping) in MAPPINGS[..2].iter().copied().enumerate() {
        let own = probes[flip];
        let reverse = probes[1 ^ flip];
        let balanced = probes[2];
        let mut rows = BTreeMap::new();
        for (i, name) in CUES.into_iter().enumerate() {
            let name = in_task(name, task);
            let group = &targets[i ^ flip];
            let correct = count(&own[name], group);
            let wrong = count(&own[name], &targets[1 ^ i ^ flip]);
            let reset_count = count(&reset[name], group);
            let opposite = count(&reverse[name], group);
            let control = count(&balanced[name], group);
            let before_b = prior_b.map(|previous| count(&previous[flip][name], group));
            let eligible = targets.iter().all(|g| !g.is_empty())
                && initial_a.is_none_or(|hits| hits[mapping][name]["passed"] == true);
            let quiet = spikes(&own["blank"]).len() <= spikes(&reset["blank"]).len()
                && own[name]["rest_core_spikes"].as_u64().unwrap()
                    <= reset[name]["rest_core_spikes"].as_u64().unwrap();
            let passed = quiet
                && eligible
                && correct
                    > wrong
                        .max(reset_count)
                        .max(opposite)
                        .max(control)
                        .max(before_b.unwrap_or(0));
            rows.insert(
                name,
                json!({"correct":correct,"wrong":wrong,"reset":reset_count,
                "opposite":opposite,"balanced":control,"before_b":before_b,"quiet":quiet,
                "eligible":eligible,"passed":passed}),
            );
        }
        family.insert(mapping, rows);
    }
    json!(family)
}

fn score(branches: &Value, reset: &Value, target_groups: &[Templates; 2]) -> Value {
    let mut all = BTreeMap::new();
    for policy in POLICIES {
        let arms = &branches[policy];
        let a = MAPPINGS.map(|mapping| &arms[mapping]["a"]["probes"]);
        let b = MAPPINGS.map(|mapping| &arms[mapping]["b"]["probes"]);
        let acquired = score_mappings(a, reset, &target_groups[0], 0, None, None);
        let learned_b = score_mappings(b, reset, &target_groups[1], 1, Some([a[0], a[1]]), None);
        let retained = score_mappings(b, reset, &target_groups[0], 0, None, Some(&acquired));
        all.insert(
            policy,
            json!({"a_acquisition":acquired,"b_acquisition":learned_b,
            "a_retention":retained}),
        );
    }
    json!(all)
}

fn passes(family: &Value) -> usize {
    family
        .as_object()
        .unwrap()
        .values()
        .flat_map(|m| m.as_object().unwrap().values())
        .filter(|cue| cue["passed"] == true)
        .count()
}

fn run(config: &Config, seed: u64) -> Result<TrialMetrics> {
    let reference = checkpoint(config, seed)?.0;
    let initial = topology(&reference);
    let checkpoint_time_us = reference.next_observation_us();
    let core: Vec<_> = reference.processing_ids().iter().map(|id| id.0).collect();
    let stimuli = patterns();
    let assay = Assay {
        config,
        seed,
        reference,
        images: frames(&stimuli)?,
    };
    let reset = assay.reset()?;
    let targets = [templates(&reset, OUTCOMES), templates(&reset, OUTCOMES_B)];
    let reset = json!(
        reset
            .into_iter()
            .map(|(name, response)| (name, response.json()))
            .collect::<BTreeMap<_, _>>()
    );
    let mut branches = BTreeMap::new();
    for policy in POLICIES {
        let mut mappings = BTreeMap::new();
        for mapping in MAPPINGS {
            let mut parent = assay.start_training(policy)?;
            let mut stages = BTreeMap::new();
            for (task, stage) in TASKS.into_iter().enumerate() {
                let frames = assay.train(&mut parent, mapping, task, TrainingAudit::Standard)?;
                let probes = assay.probe_stage(&parent, policy, mapping, task)?;
                stages.insert(
                    stage,
                    json!({"frames":frames,"end_topology":topology(&parent),"probes":probes}),
                );
            }
            mappings.insert(mapping, stages);
        }
        branches.insert(policy, mappings);
    }
    let branches = json!(branches);
    let checks = score(&branches, &reset, &targets);
    let mut values = BTreeMap::new();
    for policy in POLICIES {
        let mut joint = true;
        for (family, metric) in [
            ("a_acquisition", "a"),
            ("b_acquisition", "b"),
            ("a_retention", "retained_a"),
        ] {
            let count = passes(&checks[policy][family]);
            joint &= count == 4;
            values.insert(format!("{policy}_{metric}"), count as f64);
        }
        values.insert(format!("{policy}_joint"), f64::from(joint));
    }
    Ok(TrialMetrics {
        values,
        details: json!({
            "protocol":PROTOCOL,"seed":seed,"checkpoint_time_us":checkpoint_time_us,"core_ids":core,
            "stimuli":stimuli,"initial_topology":initial,"targets":{"a":targets[0],"b":targets[1]},
            "reset":reset,"rate_tau_us":RATE_TAU_US,"branches":branches,"checks":checks,
        }),
    })
}

impl Scenario for LiveRateScenario {
    type Config = Config;
    fn name(&self) -> &'static str {
        "hippocampal_contingency_live_rate"
    }
    fn default_config(&self) -> Config {
        ContingencyScenario.default_config()
    }
    fn validate(&self, config: &Config) -> Result<()> {
        ContingencyScenario.validate(config)
    }
    fn set_parameter(&self, config: &mut Config, name: &str, value: &Value) -> Result<()> {
        ContingencyScenario.set_parameter(config, name, value)
    }
    fn snapshot(&self, config: &Config, seed: u64, intervention: &Intervention) -> Value {
        json!({"protocol":PROTOCOL,"seed":seed,"intervention":intervention.name,
            "processing_neurons":config.processing_neurons,"training_repetitions":config.training_repetitions,
            "rest_us":config.rest_us,"stimuli":patterns(),"rate_tau_us":RATE_TAU_US})
    }
    fn interventions(&self) -> Vec<Intervention> {
        vec![Intervention {
            name: "paired_rate_rules".into(),
        }]
    }
    fn metric_names(&self) -> Vec<&'static str> {
        METRICS.to_vec()
    }
    fn run(&self, config: &Config, seed: u64, intervention: &Intervention) -> Result<TrialMetrics> {
        self.validate(config)?;
        if intervention.name != "paired_rate_rules" {
            return Err(error("unknown live rate intervention"));
        }
        run(config, seed)
    }
}
