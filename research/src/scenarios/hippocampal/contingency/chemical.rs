//! Isolate the local chemical factor on the identical recorded training history.
use super::super::factorial::weights::{norm, plan};
use super::{
    CUES, MAPPINGS, Tape, Weights, checkpoint,
    electrical::{Archive, checked_record},
    get_weights, schedule, serialized, train,
};
use crate::{Result, error};
use serde_json::{Value, json};
use std::collections::BTreeMap;

const PROTOCOL: &str = "NERVA-Contingency-Chemical/1";
const CONTROLS: [&str; 4] = ["reset", "forward_core", "reverse_core", "balanced_core"];
type Arms = BTreeMap<String, Weights>;

fn validate_electrical(
    archive: &Archive,
    details: &Value,
    electrical: &Value,
    checksum: &str,
) -> Result<()> {
    if electrical["replay_verified"] != true
        || electrical["archived_record_checksum"] != checksum
        || archive
            .metadata()
            .as_object()
            .unwrap()
            .iter()
            .any(|(key, value)| electrical[key] != *value)
        || electrical["arms"]
            .as_object()
            .is_none_or(|arms| arms.len() != CONTROLS.len())
    {
        return Err(error(
            "electrical controls do not belong to archived training",
        ));
    }
    for name in CONTROLS {
        let (arm, stored) = (&electrical["arms"][name], &details["probes"][name]);
        if stored["weights"].is_null() || arm["weights"] != stored["weights"] {
            return Err(error("electrical control weights differ from archive"));
        }
        for cue in CUES {
            let mut comparison = arm["cues"][cue].clone();
            let entries = comparison
                .as_object_mut()
                .ok_or_else(|| error("missing archived cue response"))?;
            entries.remove("boundaries");
            entries.remove("arrivals");
            if comparison != stored["responses"][cue] {
                return Err(error("electrical cue control differs from archive"));
            }
        }
    }
    Ok(())
}

fn core_delta(archive: &Archive, weights: &Weights) -> Result<Vec<f64>> {
    if weights.len() != archive.edges.len()
        || weights
            .iter()
            .zip(&archive.edges)
            .any(|(&(id, _), e)| id.0 != e.0)
    {
        return Err(error("chemical replay topology differs"));
    }
    Ok(archive
        .edges
        .iter()
        .zip(weights)
        .map(|(&(_, pre, post, old), &(_, new))| {
            if archive.core.contains(&pre) && archive.core.contains(&post) {
                f64::from(new) - f64::from(old)
            } else {
                0.
            }
        })
        .collect())
}

fn blank_count(response: &Value) -> Result<usize> {
    response["core_spikes"]
        .as_array()
        .map(Vec::len)
        .ok_or_else(|| error("missing blank spike control"))
}

fn observe(
    archive: &Archive,
    electrical: &Value,
    blank_reset: &Value,
    weights: &Arms,
) -> Result<(Value, Value)> {
    let mut arms = BTreeMap::from([("reset".to_owned(), electrical["arms"]["reset"].clone())]);
    let mut blank = BTreeMap::from([("reset".to_owned(), blank_count(blank_reset)?)]);
    for (name, vector) in weights {
        let mut cues = BTreeMap::new();
        for (cue, control) in CUES
            .into_iter()
            .map(|cue| (cue, &electrical["arms"]["reset"]["cues"][cue]))
            .chain([("blank", blank_reset)])
        {
            let observed = archive.observe(vector, cue)?;
            if ["start_us", "root_spikes"]
                .into_iter()
                .any(|field| observed[field] != control[field])
            {
                return Err(error("counterfactual root input differs from reset"));
            }
            if cue == "blank" {
                blank.insert(name.clone(), blank_count(&observed)?);
            } else {
                cues.insert(cue, observed);
            }
        }
        arms.insert(
            name.clone(),
            json!({"weights":serialized(vector),"cues":cues}),
        );
    }
    let mut result = archive.metadata();
    result["arms"] = json!(arms);
    result["archived_record_checksum"] = electrical["archived_record_checksum"].clone();
    result["replay_verified"] = json!(true); // published only after full replay
    Ok((result, json!(blank)))
}

fn run(record: &Value, electrical: &Value, checksum: &str) -> Result<Value> {
    let archive = Archive::load(record)?;
    let details = &record["details"];
    validate_electrical(&archive, details, electrical, checksum)?;
    let initial = get_weights(&archive.reference);
    let mut raw = Arms::new();
    let mut matched = Arms::new();
    let mut norms = BTreeMap::new();
    let mut histories = BTreeMap::new();
    for mapping in MAPPINGS {
        let (mut model, _) = checkpoint(&archive.c, archive.seed)?;
        if !archive.reference.same_replay_state(&model) {
            return Err(error("chemical training checkpoints differ"));
        }
        let mut tape = Tape::default();
        let order = schedule(&archive.c, archive.seed, mapping, archive.gap);
        let windows = train(&mut model, &order, &archive.frames, Some(&mut tape))?;
        let natural = get_weights(&model);
        if windows != details["training"][mapping]["frames"]
            || json!(serialized(&natural)) != details["training"][mapping]["weights"]
        {
            return Err(error(format!(
                "{mapping} does not reproduce archived training"
            )));
        }
        let (history, constant) = tape.replay_checkpoint_chemistry(&model)?;
        histories.insert(mapping, history);
        let native_delta = core_delta(&archive, &natural)?;
        let const_delta = core_delta(&archive, &constant)?;
        let candidate = plan(&initial, &const_delta, norm(&native_delta));
        let arm = format!("{mapping}_core");
        if candidate.details["faithful"] == true
            && let Some(vector) = candidate.weights
        {
            matched.insert(arm.clone(), vector);
        }
        norms.insert(mapping, candidate.details);
        let const_core = initial
            .iter()
            .zip(constant)
            .zip(const_delta)
            .map(|((&old, new), delta)| if delta == 0. { old } else { new })
            .collect();
        raw.insert(arm, const_core);
    }
    let native_blank = CONTROLS
        .into_iter()
        .map(|arm| {
            Ok((
                arm,
                blank_count(&details["probes"][arm]["responses"]["blank"])?,
            ))
        })
        .collect::<Result<BTreeMap<_, _>>>()?;
    let blank_reset = &details["probes"]["reset"]["responses"]["blank"];
    let (counterfactual, raw_blank) = observe(&archive, electrical, blank_reset, &raw)?;
    let (equal_size, matched_blank) = if matched.len() == MAPPINGS.len() {
        observe(&archive, electrical, blank_reset, &matched)?
    } else {
        (Value::Null, Value::Null)
    };
    Ok(
        json!({"protocol":PROTOCOL,"seed":archive.seed,"gap_frames":archive.gap,
        "archived_record_checksum":checksum,"training":histories,"norm":norms,
        "probes":{"native":electrical,"constant":counterfactual,"matched":equal_size},
        "blank_core_spikes":{"native":native_blank,"constant":raw_blank,"matched":matched_blank}}),
    )
}

/// Counterfactual weights from identical local events, evaluated in a native frozen network.
pub fn replay_chemical(envelope: &Value, electrical: &Value) -> Result<Value> {
    let (record, checksum) = checked_record(envelope)?;
    let mut observed = run(record, electrical, &checksum)?;
    if observed != run(record, electrical, &checksum)? {
        return Err(error("contingency chemical replay differs"));
    }
    observed["replay_verified"] = json!(true);
    Ok(observed)
}
