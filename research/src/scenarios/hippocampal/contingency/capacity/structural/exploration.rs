//! Paired tests of a local alternative for targets with unmet growth demand.
use super::{
    CUES, LearningConfig, MAPPINGS, Model, NeuronId, PROTOCOL as STRUCTURAL, Polarity, Structure,
    SynapseId, Targets, build, checked_capacity, count, error, fail, gates, observed_arm,
    recorded_config, recorded_focus, recorded_relays, response, topology,
};
use crate::Result;
use nerva_vm::model::{DevelopmentPolicy, ExploratoryFormation};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

pub(super) const PROTOCOL: &str = "NERVA-Contingency-Structural-Exploration/1";
const ARMS: [(&str, bool, ExploratoryFormation); 6] = [
    ("native_prune", true, ExploratoryFormation::Off),
    ("native_retain", false, ExploratoryFormation::Off),
    ("recent_prune", true, ExploratoryFormation::RecentExcitatory),
    (
        "recent_retain",
        false,
        ExploratoryFormation::RecentExcitatory,
    ),
    (
        "nearest_prune",
        true,
        ExploratoryFormation::NearestExcitatory,
    ),
    (
        "nearest_retain",
        false,
        ExploratoryFormation::NearestExcitatory,
    ),
];

fn passed(results: &Value) -> bool {
    MAPPINGS
        .iter()
        .any(|&m| CUES.iter().any(|&cue| results[m][cue]["passed"] == true))
}

fn native_matches(record: &Value, archive: &Value) -> bool {
    let mut native = record.clone();
    let training = native["training"].as_object_mut().unwrap();
    [
        training.remove("exploratory"),
        training.remove("exploratory_arrivals"),
    ]
    .into_iter()
    .all(|rows| rows == Some(json!([])))
        && native == *archive
}

fn run(source: &Value, archived: &Value, checksum: &str) -> Result<Value> {
    let (config, seed) = recorded_config(source)?;
    let (assay, development, _) = build(&config, seed)?;
    let details = &source["details"];
    let initial = json!(topology(&assay.reference));
    if archived["protocol"] != STRUCTURAL
        || archived["replay_verified"] != true
        || archived["archived_record_checksum"] != checksum
        || archived["seed"] != seed
        || archived["checkpoint_time_us"] != development["checkpoint_time_us"]
        || archived["initial_topology"] != initial
        || archived["core_ids"] != json!(assay.core)
        || archived["target_groups"] != details["target_groups"]
        || details["development"] != development
    {
        return Err(error(
            "exploration inputs differ from archived structural training",
        ));
    }
    let targets: Targets =
        serde_json::from_value(details["target_groups"].clone()).map_err(fail)?;
    let mut geometry = Vec::new();
    let mut initial_last_spikes = Vec::new();
    for &id in &assay.core {
        let cell = assay.reference.network().neuron(NeuronId(id)).unwrap();
        let p = cell.position();
        geometry.push((id, p.x, p.y, p.z, cell.polarity() == Polarity::Excitatory));
        initial_last_spikes.push((id, cell.last_spike().map(|time| time.0)));
    }
    let mut fixed = BTreeMap::new();
    let mut focus_by_mapping = BTreeMap::new();
    for mapping in MAPPINGS {
        let focus = recorded_focus(&assay, &details["followup"], mapping)?;
        let quiet = recorded_relays(&assay, &details["followup"], mapping, &focus);
        if archived["quiet_sources"][mapping] != json!(quiet) {
            return Err(error(
                "exploration quiet relays differ from archived capacity focus",
            ));
        }
        focus_by_mapping.insert(mapping, (focus, quiet));
        fixed.insert(mapping, archived["arms"]["frozen"][mapping].clone());
    }

    let mut arms = BTreeMap::new();
    let mut preliminary = BTreeMap::new();
    let mut final_gates = BTreeMap::new();
    let mut balanced = BTreeMap::new();
    let mut recent_parents = BTreeMap::new();
    for (name, pruning, formation) in ARMS {
        let structure = Structure::Experimental(DevelopmentPolicy { formation, pruning });
        let mut mappings = BTreeMap::new();
        for mapping in MAPPINGS {
            let (focus, quiet) = &focus_by_mapping[mapping];
            let (parent, record) = observed_arm(&assay, mapping, structure, focus, quiet)?;
            if name == "native_prune"
                && !native_matches(&record, &archived["arms"]["growing"][mapping])
            {
                return Err(error(
                    "native exploration control differs from archived replay",
                ));
            }
            if !record["training"]["frames"]
                .as_array()
                .unwrap()
                .iter()
                .map(|frame| &frame["root_spikes"])
                .eq(fixed[mapping]["training"]["frames"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|frame| &frame["root_spikes"]))
            {
                return Err(error("exploration changed the presented training input"));
            }
            if formation == ExploratoryFormation::RecentExcitatory {
                recent_parents.insert((name, mapping), (structure, parent));
            }
            mappings.insert(mapping, record);
        }
        let first = gates(&fixed, &mappings, &targets, None)?;
        let final_rows = if passed(&first) {
            let (_, control) = observed_arm(
                &assay,
                "balanced",
                structure,
                &BTreeSet::new(),
                &BTreeSet::new(),
            )?;
            let final_rows = gates(&fixed, &mappings, &targets, Some(&control))?;
            balanced.insert(name, control);
            final_rows
        } else {
            first.clone()
        };
        preliminary.insert(name, first);
        final_gates.insert(name, final_rows);
        arms.insert(name, mappings);
    }

    let mut ablations = Vec::new();
    for (name, pruning, formation) in ARMS {
        if formation != ExploratoryFormation::RecentExcitatory {
            continue;
        }
        let (native, nearest) = if pruning {
            ("native_prune", "nearest_prune")
        } else {
            ("native_retain", "nearest_retain")
        };
        for (flip, mapping) in MAPPINGS.into_iter().enumerate() {
            let (structure, parent): &(Structure, Model) = &recent_parents[&(name, mapping)];
            let new = arms[name][mapping]["training"]["exploratory"]
                .as_array()
                .unwrap()
                .iter()
                .map(|row| SynapseId(row[0].as_u64().unwrap()))
                .filter(|&edge| {
                    parent
                        .network()
                        .synapse(edge)
                        .is_some_and(|s| s.is_enabled())
                })
                .collect::<Vec<_>>();
            for (i, cue) in CUES.into_iter().enumerate() {
                let group = &targets[i ^ flip];
                let original = &arms[name][mapping]["probes"][cue];
                if new.is_empty()
                    || final_gates[name][mapping][cue]["passed"] != true
                    || [native, nearest].into_iter().any(|other| {
                        count(original, group) <= count(&arms[other][mapping]["probes"][cue], group)
                    })
                {
                    continue;
                }
                let (probe, cancelled) = response(&assay, parent, mapping, *structure, cue, &new)?;
                if probe["start_us"] != original["start_us"]
                    || probe["root_spikes"] != original["root_spikes"]
                {
                    return Err(error("exploratory-edge ablation changed recall input"));
                }
                ablations.push(json!({
                    "arm": name, "mapping": mapping, "cue": cue, "probe": probe,
                    "blocked_edges": new.iter().map(|edge| edge.0).collect::<Vec<_>>(),
                    "queued_arrivals_cancelled": cancelled,
                }));
            }
        }
    }
    Ok(json!({
        "protocol": PROTOCOL, "seed": seed, "archived_record_checksum": checksum,
        "checkpoint_time_us": development["checkpoint_time_us"],
        "initial_topology": initial, "stdp_window_us": LearningConfig::default().stdp_window_us,
        "core_ids": assay.core, "core_geometry": geometry, "initial_last_spikes": initial_last_spikes,
        "target_groups": targets, "focus": archived["focus"], "quiet_sources": archived["quiet_sources"],
        "arms": arms, "preliminary_gates": preliminary, "gates": final_gates,
        "balanced": balanced, "ablations": ablations,
    }))
}

pub fn replay_structural_exploration(capacity: &Value, structural: &Value) -> Result<Value> {
    let (source, checksum) = checked_capacity(capacity)?;
    let mut result = run(source, structural, &checksum)?;
    if result != run(source, structural, &checksum)? {
        return Err(error("structural exploration replay differs"));
    }
    result["replay_verified"] = json!(true);
    Ok(result)
}
