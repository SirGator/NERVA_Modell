//! Let the existing developmental rule respond to Core homeostatic demand during training.
use super::super::{Model, fail, probe};
use super::{
    Assay, CUES, CapacityScenario, Config, MAPPINGS, Scenario, build, checkpoint,
    eligibility::{checked_capacity, recorded_focus, u64_field},
    schedule,
};
use crate::{Result, error};
use nerva::{
    config::LearningConfig,
    core::SynapseId,
    runtime::{LearningAuditEvent, LearningPhase, ObservationEvent},
};
use nerva_vm::model::{HomeostasisScope, PlasticitySettings, audit::AuditOptions};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

const PROTOCOL: &str = "NERVA-Contingency-Structural/1";
const PROBES: [&str; 3] = ["blank", "cue0", "cue1"];
type Targets = [BTreeSet<u64>; 2];
type Endpoints = Vec<(u64, u64, u64, f32, u64, bool)>;

#[derive(Default)]
struct TrainingTrace {
    arrivals: Vec<(u64, u64, u64, u64, f32)>,
    integrals: Vec<(u64, u64, f32, f32, f32, u64, bool)>,
    tags: Vec<(u64, u64, &'static str, f32, f32)>,
    homeostatic_changes: usize,
}

fn topology(model: &Model) -> Endpoints {
    model
        .network()
        .synapses()
        .map(|s| {
            (
                s.id().0,
                s.pre().0,
                s.post().0,
                s.weight().get(),
                s.delay_us(),
                s.is_enabled(),
            )
        })
        .collect()
}

impl TrainingTrace {
    fn record(
        &mut self,
        model: &mut Model,
        focus: &BTreeSet<SynapseId>,
        quiet: &BTreeSet<u64>,
        core: &BTreeSet<u64>,
    ) -> Result<Vec<(u64, u64)>> {
        let audit = model
            .take_audit()
            .ok_or_else(|| error("missing structural training audit"))?;
        if audit.raw_events.len() > 200_000 {
            return Err(error("structural training exceeded frame event budget"));
        }
        for event in audit.raw_events {
            match event {
                ObservationEvent::SynapticArrival {
                    time,
                    synapse_id,
                    source,
                    target,
                    amplitude,
                } if focus.contains(&synapse_id) || quiet.contains(&target.0) => {
                    self.arrivals
                        .push((time.0, synapse_id.0, source.0, target.0, amplitude));
                }
                ObservationEvent::NeuronIntegrated {
                    time,
                    target,
                    potential_before,
                    summed_input,
                    threshold,
                    refractory_until,
                    spiked,
                    ..
                } if quiet.contains(&target.0) => {
                    self.integrals.push((
                        time.0,
                        target.0,
                        potential_before,
                        summed_input,
                        threshold,
                        refractory_until.0,
                        spiked,
                    ));
                }
                ObservationEvent::HomeostasisChanged { neuron_id, .. } => {
                    if !core.contains(&neuron_id.0) {
                        return Err(error("structural study changed homeostasis outside Core"));
                    }
                    self.homeostatic_changes += 1;
                }
                _ => {}
            }
        }
        let log = model.take_learning_audit().map_err(fail)?;
        for event in log.events {
            if let LearningAuditEvent::Trace {
                time,
                synapse,
                phase,
                tag_before,
                tag_after,
                ..
            } = event
                && focus.contains(&synapse)
                && phase != LearningPhase::Delayed
                && tag_before != tag_after
            {
                let name = if phase == LearningPhase::PreArrival {
                    "pre"
                } else {
                    "post"
                };
                self.tags
                    .push((time.0, synapse.0, name, tag_before, tag_after));
            }
        }
        Ok(audit
            .input_spikes
            .into_iter()
            .map(|s| (s.channel.0, s.at.0))
            .collect())
    }
}

fn train(
    assay: &Assay<'_>,
    mapping: &str,
    development: bool,
    diagnostics: Option<(&BTreeSet<SynapseId>, &BTreeSet<u64>)>,
) -> Result<(Model, Value)> {
    let mut model = checkpoint(assay.c, assay.seed)?.0;
    if !assay.reference.same_replay_state(&model) {
        return Err(error(
            "structural arm starts from a different developed state",
        ));
    }
    let removed = model
        .diagnostic_clear_learning_eligibility()
        .map_err(fail)?;
    model
        .set_checkpoint_homeostasis_scope(HomeostasisScope::Core)
        .map_err(fail)?;
    model
        .set_plasticity(PlasticitySettings {
            learning: true,
            development,
            homeostasis: true,
        })
        .map_err(fail)?;
    if let Some((_, quiet)) = diagnostics {
        model.enable_audit(AuditOptions {
            raw_events: true,
            input_spikes: true,
        });
        model.enable_learning_audit(100_000).map_err(fail)?;
        model.set_dynamics_auditing(!quiet.is_empty());
    }
    let mut trace = TrainingTrace::default();
    let mut frames = Vec::new();
    for name in schedule(assay.c, assay.seed, mapping, 0) {
        let step = model.step(&assay.frames[name]).map_err(fail)?;
        if !development && (!step.formed.is_empty() || !step.pruned.is_empty()) {
            return Err(error("frozen structural control changed topology"));
        }
        if let Some((focus, quiet)) = diagnostics {
            let roots = trace.record(&mut model, focus, quiet, &assay.core)?;
            frames.push(json!({"stimulus":name,"start_us":step.start_us,"end_us":step.end_us,
                "root_spikes":roots,"core_spikes":step.core_events,"weight_changes":step.weight_changes,
                "formed":step.formed,"pruned":step.pruned}));
        }
    }
    if diagnostics.is_some() {
        let pending = model.disable_learning_audit().map_err(fail)?;
        if !pending.events.is_empty()
            || !pending.delayed.is_empty()
            || !pending.boundaries.is_empty()
            || trace.homeostatic_changes == 0
        {
            return Err(error(
                "incomplete structural learning or maintenance observation",
            ));
        }
        model.set_dynamics_auditing(false);
    }
    model.freeze().map_err(fail)?;
    let observed = if diagnostics.is_some() {
        json!({"frames":frames,"removed_tags":removed,"end_topology":topology(&model),
            "homeostasis_changes":trace.homeostatic_changes,"arrivals":trace.arrivals,
            "quiet_integrations":trace.integrals,"tag_events":trace.tags})
    } else {
        Value::Null
    };
    Ok((model, observed))
}

fn response(
    assay: &Assay<'_>,
    trained: &Model,
    mapping: &str,
    growth: bool,
    name: &str,
    blocked: &[SynapseId],
) -> Result<(Value, usize)> {
    let (mut branch, _) = train(assay, mapping, growth, None)?;
    if !trained.same_replay_state(&branch) {
        return Err(error(
            "structural recall training fork differs from its observed parent",
        ));
    }
    let cancelled = if blocked.is_empty() {
        0
    } else {
        branch.diagnostic_block_synapses(blocked).map_err(fail)?
    };
    let result = probe(
        &mut branch,
        assay.c,
        &assay.frames[name],
        &assay.frames["blank"],
        None,
    )?;
    Ok((result.json(), cancelled))
}

fn arm(
    assay: &Assay<'_>,
    mapping: &str,
    growth: bool,
    focus: &BTreeSet<SynapseId>,
    quiet: &BTreeSet<u64>,
) -> Result<(Model, Value)> {
    let (parent, training) = train(assay, mapping, growth, Some((focus, quiet)))?;
    let probes = PROBES
        .into_iter()
        .map(|name| {
            Ok((
                name,
                response(assay, &parent, mapping, growth, name, &[])?.0,
            ))
        })
        .collect::<Result<BTreeMap<_, _>>>()?;
    Ok((parent, json!({"training":training,"probes":probes})))
}

fn count(probe: &Value, group: &BTreeSet<u64>) -> usize {
    probe["core_spikes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| group.contains(&row[0].as_u64().unwrap()))
        .count()
}

fn gates(
    frozen: &BTreeMap<&str, Value>,
    growing: &BTreeMap<&str, Value>,
    targets: &Targets,
    balanced: Option<&Value>,
) -> Result<Value> {
    let matched = |a: &Value, b: &Value| {
        a["root_spikes"] == b["root_spikes"] && a["start_us"] == b["start_us"]
    };
    let mut rows = BTreeMap::new();
    for (flip, mapping) in MAPPINGS.into_iter().enumerate() {
        let own = &growing[mapping]["probes"];
        let fixed = &frozen[mapping]["probes"];
        let opposite = &growing[MAPPINGS[1 ^ flip]]["probes"];
        if !matched(&own["blank"], &fixed["blank"])
            || !matched(&own["blank"], &opposite["blank"])
            || balanced.is_some_and(|b| !matched(&own["blank"], &b["probes"]["blank"]))
        {
            return Err(error(
                "structural recall blank controls received different inputs",
            ));
        }
        let quiet = own["blank"]["core_spikes"].as_array().unwrap().len()
            <= fixed["blank"]["core_spikes"].as_array().unwrap().len();
        let mut per_cue = BTreeMap::new();
        for (i, cue) in CUES.into_iter().enumerate() {
            let correct_group = &targets[i ^ flip];
            let correct = count(&own[cue], correct_group);
            let wrong = count(&own[cue], &targets[1 ^ i ^ flip]);
            let baseline = count(&fixed[cue], correct_group);
            let reversed = count(&opposite[cue], correct_group);
            let control = balanced.map(|b| count(&b["probes"][cue], correct_group));
            let inputs = matched(&own[cue], &fixed[cue])
                && matched(&own[cue], &opposite[cue])
                && balanced.is_none_or(|b| matched(&own[cue], &b["probes"][cue]));
            if !inputs {
                return Err(error("structural recall cues received different inputs"));
            }
            let passed = inputs
                && quiet
                && correct > wrong.max(baseline).max(reversed).max(control.unwrap_or(0))
                && own[cue]["rest_core_spikes"].as_u64().unwrap()
                    <= fixed[cue]["rest_core_spikes"].as_u64().unwrap();
            per_cue.insert(
                cue,
                json!({"correct":correct,"incorrect":wrong,"frozen":baseline,
                "opposite":reversed,"balanced":control,"same_root_input":inputs,"passed":passed}),
            );
        }
        rows.insert(mapping, per_cue);
    }
    Ok(json!(rows))
}

fn archived_frames(training: &Value) -> Value {
    json!(
        training["frames"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| json!({
                "stimulus":f["stimulus"],"start_us":f["start_us"],"root_spikes":f["root_spikes"],
                "core_spikes":f["core_spikes"],"weight_changes":f["weight_changes"]
            }))
            .collect::<Vec<_>>()
    )
}

fn run(source: &Value, homeostasis: &Value, checksum: &str) -> Result<Value> {
    let cfgrow = &source["job"]["effective_config"];
    let cfg = Config {
        processing_neurons: usize::try_from(u64_field(cfgrow, "processing_neurons")?)
            .map_err(fail)?,
        training_repetitions: usize::try_from(u64_field(cfgrow, "training_repetitions")?)
            .map_err(fail)?,
        rest_us: u64_field(cfgrow, "rest_us")?,
    };
    CapacityScenario.validate(&cfg)?;
    let seed = u64_field(&source["job"], "seed")?;
    let (assay, development, _) = build(&cfg, seed)?;
    let details = &source["details"];
    if homeostasis["protocol"] != "NERVA-Contingency-Core-Homeostasis/1"
        || homeostasis["archived_record_checksum"] != checksum
        || homeostasis["replay_verified"] != true
        || homeostasis["seed"] != seed
        || details["development"] != development
        || homeostasis["topology"] != json!(assay.topology())
        || details["topology"] != homeostasis["topology"]
        || homeostasis["target_groups"] != details["target_groups"]
    {
        return Err(error(
            "structural study inputs differ from the reconstructed checkpoint",
        ));
    }
    let targets: Targets =
        serde_json::from_value(details["target_groups"].clone()).map_err(fail)?;
    let mut fixed = BTreeMap::new();
    let mut grow = BTreeMap::new();
    let mut parents = BTreeMap::new();
    let mut focused = BTreeMap::new();
    let mut quiets = BTreeMap::new();
    let initial_ids = assay
        .initial
        .iter()
        .map(|&(id, _)| id)
        .collect::<BTreeSet<_>>();
    for mapping in MAPPINGS {
        let focus = recorded_focus(&assay, &details["followup"], mapping)?;
        let old = details["followup"]["training"][mapping]["frames"]
            .as_array()
            .unwrap();
        let active = old
            .iter()
            .flat_map(|frame| frame["core_spikes"].as_array().unwrap())
            .map(|row| row[0].as_u64().unwrap())
            .collect::<BTreeSet<_>>();
        let quiet = focus
            .iter()
            .map(|&id| assay.reference.network().synapse(id).unwrap().pre().0)
            .filter(|id| !active.contains(id))
            .collect::<BTreeSet<_>>();
        let (_, a) = arm(&assay, mapping, false, &focus, &quiet)?;
        let upstream = &homeostasis["arms"]["core_clear"]["training"][mapping];
        let weights = json!(
            a["training"]["end_topology"]
                .as_array()
                .unwrap()
                .iter()
                .map(|e| (&e[0], &e[3]))
                .collect::<Vec<_>>()
        );
        if archived_frames(&a["training"]) != upstream["frames"]
            || weights != upstream["weights"]
            || a["training"]["removed_tags"] != upstream["removed_tags"]
        {
            return Err(error(
                "frozen topology training differs from archived Core homeostasis",
            ));
        }
        let (parent, b) = arm(&assay, mapping, true, &focus, &quiet)?;
        if !a["training"]["frames"]
            .as_array()
            .unwrap()
            .iter()
            .zip(b["training"]["frames"].as_array().unwrap())
            .all(|(a, b)| a["root_spikes"] == b["root_spikes"] && a["start_us"] == b["start_us"])
        {
            return Err(error("growth changed the presented root input"));
        }
        let bindings = focus
            .iter()
            .map(|&id| {
                let s = assay.reference.network().synapse(id).unwrap();
                (id.0, s.pre().0, s.post().0, s.delay_us())
            })
            .collect::<Vec<_>>();
        focused.insert(mapping, bindings);
        quiets.insert(mapping, quiet);
        parents.insert(mapping, parent);
        fixed.insert(mapping, a);
        grow.insert(mapping, b);
    }
    let preliminary = gates(&fixed, &grow, &targets, None)?;
    let triggered = MAPPINGS
        .iter()
        .any(|&m| CUES.iter().any(|&c| preliminary[m][c]["passed"] == true));
    let balanced = if triggered {
        Some(arm(&assay, "balanced", true, &BTreeSet::new(), &BTreeSet::new())?.1)
    } else {
        None
    };
    let final_gates = gates(&fixed, &grow, &targets, balanced.as_ref())?;
    let mut ablations = Vec::new();
    for mapping in MAPPINGS {
        let parent = &parents[mapping];
        let new = parent
            .network()
            .synapses()
            .filter(|s| s.is_enabled() && !initial_ids.contains(&s.id()))
            .map(|s| s.id())
            .collect::<Vec<_>>();
        for cue in CUES {
            if final_gates[mapping][cue]["passed"] == true && !new.is_empty() {
                let (blocked, cancelled) = response(&assay, parent, mapping, true, cue, &new)?;
                if ["start_us", "root_spikes"]
                    .into_iter()
                    .any(|k| blocked[k] != grow[mapping]["probes"][cue][k])
                {
                    return Err(error("new-edge ablation changed recall input"));
                }
                ablations.push(json!({"mapping":mapping,"cue":cue,"blocked_edges":new.iter().map(|id|id.0).collect::<Vec<_>>(),
                    "queued_arrivals_cancelled":cancelled,"probe":blocked}));
            }
        }
    }
    Ok(
        json!({"protocol":PROTOCOL,"seed":seed,"archived_record_checksum":checksum,
        "checkpoint_time_us":development["checkpoint_time_us"],"initial_topology":topology(&assay.reference),
        "core_ids":assay.core,"target_groups":targets,"stdp_window_us":LearningConfig::default().stdp_window_us,
        "focus":focused,"quiet_sources":quiets,"arms":{"frozen":fixed,"growing":grow},"balanced":balanced,
        "preliminary_gates":preliminary,"gates":final_gates,"ablations":ablations}),
    )
}

pub fn replay_structural(capacity: &Value, homeostasis: &Value) -> Result<Value> {
    let (source, checksum) = checked_capacity(capacity)?;
    let mut output = run(source, homeostasis, &checksum)?;
    if output != run(source, homeostasis, &checksum)? {
        return Err(error("structural study replay differs"));
    }
    output["replay_verified"] = json!(true);
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_raw_selective_gain_must_also_exceed_balanced_training() {
        let result = |a: Option<u64>, b: Option<u64>| {
            let probe = |id: Option<u64>| {
                json!({"start_us":10,"root_spikes":[],"rest_core_spikes":0,
                    "core_spikes":id.map(|id| (id,11)).into_iter().collect::<Vec<_>>()})
            };
            json!({"probes":{"blank":probe(None),"cue0":probe(a),"cue1":probe(b)}})
        };
        let baseline = BTreeMap::from([
            ("forward", result(None, None)),
            ("reverse", result(None, None)),
        ]);
        let trained = BTreeMap::from([
            ("forward", result(Some(1), Some(2))),
            ("reverse", result(Some(2), Some(1))),
        ]);
        let targets = [BTreeSet::from([1]), BTreeSet::from([2])];
        let preliminary = gates(&baseline, &trained, &targets, None).unwrap();
        assert_eq!(preliminary["forward"]["cue0"]["passed"], true);
        assert_eq!(preliminary["reverse"]["cue0"]["passed"], true);
        let balanced = result(Some(1), Some(2));
        let final_gates = gates(&baseline, &trained, &targets, Some(&balanced)).unwrap();
        assert_eq!(final_gates["forward"]["cue0"]["passed"], false);
        assert_eq!(final_gates["reverse"]["cue0"]["passed"], true);
    }
}
