//! Follow observed pairing into transmitted weights on retained terminal edges.
use super::{
    LearningConfig, MAPPINGS, Structure, SynapseId, Tape, build, checked_capacity, error,
    exploration::PROTOCOL as EXPLORATION, recorded_config, recorded_focus, recorded_relays,
    topology, train,
};
use crate::Result;
use nerva::learning::WeightUpdateMode;
use nerva_vm::model::{DevelopmentPolicy, ExploratoryFormation};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

pub(super) const PROTOCOL: &str = "NERVA-Contingency-Consolidation/1";
pub(super) const STRUCTURE: Structure = Structure::Experimental(DevelopmentPolicy {
    formation: ExploratoryFormation::RecentExcitatory,
    pruning: false,
});

fn archive_projection(
    mut training: Value,
    focus: &BTreeSet<SynapseId>,
    quiet: &BTreeSet<u64>,
) -> Value {
    let target = |row: &Value, index| quiet.contains(&row[index].as_u64().unwrap());
    let edge = |row: &Value| focus.contains(&SynapseId(row[1].as_u64().unwrap()));
    training["arrivals"]
        .as_array_mut()
        .unwrap()
        .retain(|row| edge(row) || target(row, 3usize));
    training["tag_events"].as_array_mut().unwrap().retain(edge);
    training["quiet_integrations"]
        .as_array_mut()
        .unwrap()
        .retain(|row| target(row, 1usize));
    training
}

fn root_counts(training: &Value) -> BTreeMap<u64, usize> {
    let mut counts = BTreeMap::new();
    for frame in training["frames"].as_array().unwrap() {
        for row in frame["root_spikes"].as_array().unwrap() {
            *counts.entry(row[0].as_u64().unwrap()).or_default() += 1;
        }
    }
    counts
}

fn run(source: &Value, archived: &Value, checksum: &str) -> Result<Value> {
    let (config, seed) = recorded_config(source)?;
    let (assay, development, _) = build(&config, seed)?;
    let details = &source["details"];
    if archived["protocol"] != EXPLORATION
        || archived["replay_verified"] != true
        || archived["archived_record_checksum"] != checksum
        || archived["seed"] != seed
        || archived["checkpoint_time_us"] != development["checkpoint_time_us"]
        || archived["initial_topology"] != json!(topology(&assay.reference))
        || archived["core_ids"] != json!(assay.core)
        || details["development"] != development
    {
        return Err(error(
            "consolidation input differs from archived exploration",
        ));
    }
    let rule = assay.reference.diagnostic_learning_rule();
    let chemistry = rule
        .three_factor_config()
        .ok_or_else(|| error("consolidation replay requires delayed learning"))?;
    if rule.weight_update_mode() != WeightUpdateMode::Accumulated {
        return Err(error(
            "consolidation reference must preserve rounding carry",
        ));
    }
    let mut recorded = BTreeMap::new();
    let mut focus = BTreeSet::new();
    let mut quiet = BTreeSet::new();
    for mapping in MAPPINGS {
        let path = recorded_focus(&assay, &details["followup"], mapping)?;
        let relays = recorded_relays(&assay, &details["followup"], mapping, &path);
        let bindings: Vec<_> = path
            .iter()
            .map(|&id| {
                let edge = assay.reference.network().synapse(id).unwrap();
                (id.0, edge.pre().0, edge.post().0, edge.delay_us())
            })
            .collect();
        if archived["focus"][mapping] != json!(bindings)
            || archived["quiet_sources"][mapping] != json!(relays)
        {
            return Err(error(
                "consolidation focus differs from archived terminal edges",
            ));
        }
        focus.extend(&path);
        quiet.extend(&relays);
        recorded.insert(mapping, (path, relays));
    }
    if focus.is_empty() {
        return Err(error(
            "consolidation analysis has no retained terminal edges",
        ));
    }
    recorded.insert("balanced", (BTreeSet::new(), BTreeSet::new()));

    let mut result = BTreeMap::new();
    let mut marginal = None;
    for mapping in MAPPINGS.into_iter().chain(["balanced"]) {
        let archive = if mapping == "balanced" {
            &archived["balanced"]["recent_retain"]["training"]
        } else {
            &archived["arms"]["recent_retain"][mapping]["training"]
        };
        let mut tape = Tape::default();
        let (parent, training) = train(
            &assay,
            mapping,
            STRUCTURE,
            Some((&focus, &quiet)),
            None,
            Some(&mut tape),
        )?;
        let (path, relays) = &recorded[mapping];
        if (mapping != "balanced" || !archive.is_null())
            && archive_projection(training.clone(), path, relays) != *archive
        {
            return Err(error(format!(
                "{mapping}: audited training differs from archived exploration"
            )));
        }
        let roots = root_counts(&training);
        if marginal.get_or_insert_with(|| roots.clone()) != &roots {
            return Err(error(
                "consolidation histories differ in marginal root inputs",
            ));
        }
        let history = tape.replay_retained(&parent)?;
        result.insert(mapping, json!({"training":training, "history":history}));
    }
    Ok(json!({
        "protocol": PROTOCOL, "seed": seed, "archived_record_checksum": checksum,
        "checkpoint_time_us": development["checkpoint_time_us"], "core_ids": assay.core,
        "stdp_window_us": LearningConfig::default().stdp_window_us,
        "focus": archived["focus"], "quiet_sources": archived["quiet_sources"],
        "rules": {
            "eligibility_tau_us": chemistry.eligibility_tau_us, "learning_rate": chemistry.learning_rate,
            "max_abs_eligibility": chemistry.max_abs_eligibility, "weight_update_mode": "accumulated",
        },
        "mappings": result,
    }))
}

pub fn replay_structural_consolidation(capacity: &Value, exploration: &Value) -> Result<Value> {
    let (source, checksum) = checked_capacity(capacity)?;
    let mut result = run(source, exploration, &checksum)?;
    if result != run(source, exploration, &checksum)? {
        return Err(error("structural consolidation replay differs"));
    }
    result["replay_verified"] = json!(true);
    Ok(result)
}
