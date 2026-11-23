//! Separate inherited tag and pairing state from task experience.
use super::super::{Tape, fail};
use super::{
    Assay, CUES, CapacityScenario, Config, MAPPINGS, PROTOCOL, Scenario, Weights, build,
    checkpoint, get_weights, json_responses, schedule, serialized, stimuli, train,
};
use crate::{Result, error, fingerprint};
use nerva::core::SynapseId;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn u64_field(row: &Value, name: &str) -> Result<u64> {
    row[name]
        .as_u64()
        .ok_or_else(|| error(format!("invalid capacity field {name}")))
}

pub(super) fn recorded_focus(
    assay: &Assay<'_>,
    followup: &Value,
    mapping: &str,
) -> Result<BTreeSet<SynapseId>> {
    let rows = followup["terminal_ablation"]
        .as_array()
        .ok_or_else(|| error("missing capacity ablations"))?;
    let mut focus = BTreeSet::new();
    for row in rows.iter().filter(|row| row["mapping"] == mapping) {
        let ids: Vec<u64> = serde_json::from_value(row["restored_edges"].clone()).map_err(fail)?;
        for id in ids {
            if !assay
                .edges
                .iter()
                .any(|edge| edge.id == id && edge.eligible)
            {
                return Err(error("capacity focus includes an ineligible edge"));
            }
            focus.insert(SynapseId(id));
        }
    }
    Ok(focus)
}

fn mapping_run(
    assay: &Assay<'_>,
    followup: &Value,
    mapping: &str,
) -> Result<(Value, Weights, Weights)> {
    let focus = recorded_focus(assay, followup, mapping)?;
    let recurrent = assay.reference.network();
    let initial_focus = focus.iter().map(|&id| {
        let edge = recurrent.synapse(id).unwrap();
        let post = recurrent.neuron(edge.post()).unwrap();
        json!({"id":id.0,"pre":edge.pre().0,"post":edge.post().0,"delay_us":edge.delay_us(),
            "initial_tag":edge.learning_eligibility(),"initial_tag_anchor":edge.learning_eligibility_updated_at().map(|t| t.0),
            "initial_residual":edge.learning_weight_residual(),"initial_factor":post.modulation().learning_signal})
    }).collect::<Vec<_>>();
    let initial_nonzero = recurrent
        .synapses()
        .filter(|e| e.learning_eligibility() != 0.)
        .count();
    let mut branches = [
        checkpoint(assay.c, assay.seed)?.0,
        checkpoint(assay.c, assay.seed)?.0,
    ];
    if branches
        .iter()
        .any(|b| !assay.reference.same_replay_state(b))
    {
        return Err(error("inherited eligibility checkpoints differ"));
    }
    let cleared = branches[1]
        .diagnostic_clear_learning_eligibility()
        .map_err(fail)?;
    if cleared != initial_nonzero
        || branches[1]
            .network()
            .synapses()
            .any(|e| e.learning_eligibility() != 0.)
    {
        return Err(error("incorrect inherited eligibility reset"));
    }
    let order = schedule(assay.c, assay.seed, mapping, 0);
    let mut native_tape = Tape::default();
    let native_windows = train(
        &mut branches[0],
        &order,
        &assay.frames,
        Some(&mut native_tape),
    )?;
    if native_windows != followup["training"][mapping]["frames"]
        || json!(serialized(&get_weights(&branches[0]))) != followup["training"][mapping]["weights"]
    {
        return Err(error(format!(
            "{mapping} no longer reproduces archived capacity training"
        )));
    }
    let (fixed, fixed_weights) = native_tape.replay_inherited_eligibility(&branches[0], &focus)?;
    let mut live_tape = Tape::default();
    let live_windows = train(
        &mut branches[1],
        &order,
        &assay.frames,
        Some(&mut live_tape),
    )?;
    let live_weights = get_weights(&branches[1]);
    let same_history = native_tape.same_learning_history(&live_tape);
    let same_weights = live_weights == fixed_weights["clear_eligibility"];
    if same_history && !same_weights {
        return Err(error(
            "identical live history did not reproduce the fixed eligibility intervention",
        ));
    }
    let live_core = assay.core_weights(&live_weights)?;
    let fixed_core = assay.core_weights(&fixed_weights["clear_eligibility"])?;
    let all_weights = fixed_weights
        .iter()
        .map(|(&name, weights)| (name, serialized(weights)))
        .collect::<BTreeMap<_, _>>();
    let tau = assay
        .reference
        .diagnostic_learning_rule()
        .three_factor_config()
        .ok_or_else(|| error("expected three-factor learning in checkpoint"))?
        .eligibility_tau_us;
    Ok((
        json!({
            "focus":initial_focus,"eligibility_tau_us":tau,"fixed_history":fixed,"fixed_weights":all_weights,
            "live":{"frames":live_windows,"weights":serialized(&live_weights),"removed_tags":cleared,
                    "same_learning_history":same_history,"same_fixed_weights":same_weights}
        }),
        live_core,
        fixed_core,
    ))
}

fn run(source: &Value, checksum: &str) -> Result<Value> {
    let details = &source["details"];
    let settings = &source["job"]["effective_config"];
    let cfg = Config {
        processing_neurons: usize::try_from(u64_field(settings, "processing_neurons")?)
            .map_err(fail)?,
        training_repetitions: usize::try_from(u64_field(settings, "training_repetitions")?)
            .map_err(fail)?,
        rest_us: u64_field(settings, "rest_us")?,
    };
    CapacityScenario.validate(&cfg)?;
    let seed = u64_field(&source["job"], "seed")?;
    let (assay, development, _) = build(&cfg, seed)?;
    if details["seed"] != seed
        || details["development"] != development
        || details["stimuli"] != json!(stimuli())
        || details["topology"] != json!(assay.topology())
        || details["core_ids"] != json!(assay.core)
    {
        return Err(error(
            "capacity record is incompatible with the reconstructed checkpoint",
        ));
    }
    let targets: [BTreeSet<u64>; 2] =
        serde_json::from_value(details["target_groups"].clone()).map_err(fail)?;
    if !targets[0].is_disjoint(&targets[1])
        || targets.iter().any(|group| !group.is_subset(&assay.core))
    {
        return Err(error("invalid archived capacity target groups"));
    }
    let followup = &details["followup"];
    let (a, live_a, fixed_a) = mapping_run(&assay, followup, MAPPINGS[0])?;
    let (b, live_b, fixed_b) = mapping_run(&assay, followup, MAPPINGS[1])?;
    let reset = assay.observe(&assay.initial, &["blank", "cue0", "cue1"])?;
    let reset_json = json_responses(&reset);
    if reset_json
        .iter()
        .any(|(key, row)| details["reset"][*key] != *row)
    {
        return Err(error("archived capacity baseline differs"));
    }
    let (live, _) = assay.compare(&[live_a.clone(), live_b.clone()], &reset, &targets, &CUES)?;
    let shared = live_a == fixed_a && live_b == fixed_b;
    let fixed = if shared {
        live.clone()
    } else {
        assay
            .compare(&[fixed_a, fixed_b], &reset, &targets, &CUES)?
            .0
    };
    Ok(
        json!({"protocol":"NERVA-Contingency-Eligibility/1","seed":seed,"archived_record_checksum":checksum,
        "checkpoint_time_us":development["checkpoint_time_us"],"initial_weights":serialized(&assay.initial),
        "core_ids":assay.core,"topology":assay.topology(),"target_groups":targets,"mappings":{"forward":a,"reverse":b},
        "probes":{"reset":reset_json,"native":followup["native"],"live_clear":live,"offline_clear":fixed,
            "identical_live_and_offline_core_weights":shared}}),
    )
}

pub(super) fn checked_capacity(envelope: &Value) -> Result<(&Value, String)> {
    let source = &envelope["record"];
    let checksum = fingerprint(&serde_json::to_vec(source).map_err(fail)?);
    if source["details"]["protocol"] != PROTOCOL
        || source["full_replay_checked"] != true
        || envelope["checksum_fnv1a64"] != checksum
        || source["details"]["followup"].is_null()
    {
        return Err(error(
            "intact replay-verified capacity record with a followup required",
        ));
    }
    Ok((source, checksum))
}

/// Reconstruct the fixed-history and live contrasts on one checked capacity record.
pub fn replay_eligibility(envelope: &Value) -> Result<Value> {
    let (source, checksum) = checked_capacity(envelope)?;
    let mut output = run(source, &checksum)?;
    if output != run(source, &checksum)? {
        return Err(error("inherited eligibility diagnostic replay differs"));
    }
    output["replay_verified"] = json!(true);
    Ok(output)
}
