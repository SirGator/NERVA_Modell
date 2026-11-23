//! Remove the balanced component and test the residual at a matched dose.
use super::{
    Directions, MAPPINGS, MARGIN, PROTOCOL as GROWN_DOSE, SynapseId, Targets, Weights, build,
    checked_capacity, error, fail, limit, make_carrier, norm, permutation, plan_at,
    recorded_config, topology, verify_checkpoint,
};
use crate::Result;
use serde_json::{Value, json};
use std::collections::BTreeMap;

const PROTOCOL: &str = "NERVA-Contingency-Grown-Residual/1";
const CONTROLS: [&str; 5] = [
    "balanced",
    "forward",
    "reference_forward",
    "reference_reverse",
    "reverse",
];

fn projection(pattern: &[f64], nuisance: &[f64]) -> Result<(f64, Vec<f64>)> {
    let energy: f64 = nuisance.iter().map(|x| x * x).sum();
    if energy == 0. || pattern.len() != nuisance.len() {
        return Err(error("invalid nuisance direction for residual weights"));
    }
    let coefficient = pattern
        .iter()
        .zip(nuisance)
        .map(|(x, y)| x * y)
        .sum::<f64>()
        / energy;
    let residual: Vec<_> = pattern
        .iter()
        .zip(nuisance)
        .map(|(x, y)| x - coefficient * y)
        .collect();
    if norm(&residual) <= norm(pattern) * 1e-12 {
        return Err(error("residual weight direction is numerically zero"));
    }
    Ok((coefficient, residual))
}

fn requested(archive: &Value, tier: &str) -> Result<f64> {
    archive["tiers"][tier]["requested_norm"]
        .as_f64()
        .filter(|x| x.is_finite() && *x > 0.)
        .ok_or_else(|| error("missing positive archived norm"))
}

fn run(source: &Value, consolidation: &Value, archive: &Value, checksum: &str) -> Result<Value> {
    let (configuration, seed) = recorded_config(source)?;
    let (assay, development, _) = build(&configuration, seed)?;
    let details = &source["details"];
    verify_checkpoint(&assay, details, &development, consolidation, checksum)?;
    let targets: Targets =
        serde_json::from_value(details["target_groups"].clone()).map_err(fail)?;
    let checkpoint = development["checkpoint_time_us"].clone();
    if archive["protocol"] != GROWN_DOSE
        || archive["replay_verified"] != true
        || archive["archived_record_checksum"] != checksum
        || archive["seed"] != seed
        || archive["checkpoint_time_us"] != checkpoint
        || archive["target_groups"] != json!(targets)
        || archive["core_ids"] != json!(assay.core)
    {
        return Err(error(
            "grown residual inputs differ from archived dose replay",
        ));
    }
    let carrier = make_carrier(&assay, consolidation, &targets)?;
    if json!(topology(&carrier.parent)) != archive["carrier_topology"] {
        return Err(error(
            "grown residual carrier topology differs from archived dose",
        ));
    }
    let support: Vec<_> = assay.edges.iter().filter(|edge| edge.eligible).collect();
    let base: Weights = support
        .iter()
        .map(|edge| {
            let id = SynapseId(edge.id);
            (
                id,
                carrier.parent.network().synapse(id).unwrap().weight().get(),
            )
        })
        .collect();
    if archive["support"] != json!(support.iter().map(|edge| edge.id).collect::<Vec<_>>()) {
        return Err(error("grown residual support differs from archived dose"));
    }
    let baseline = carrier.observe(&base, None)?;
    if baseline != archive["baseline"] {
        return Err(error("grown residual baseline does not replay exactly"));
    }

    let old: BTreeMap<String, Vec<f64>> =
        serde_json::from_value(archive["directions"].clone()).map_err(fail)?;
    if CONTROLS
        .iter()
        .any(|name| old.get(*name).is_none_or(|v| v.len() != support.len()))
    {
        return Err(error("invalid archived grown-dose directions"));
    }
    let controls: Vec<_> = CONTROLS
        .into_iter()
        .filter(|&name| norm(&old[name]) > 0.)
        .collect();
    let order = permutation(support.iter().map(|edge| edge.post), seed);
    let permute = |values: &[f64]| order.iter().map(|&i| values[i]).collect::<Vec<_>>();
    let nuisance = &old["balanced"];
    let sham_nuisance = permute(nuisance);
    let mut directions = Directions::new();
    let mut coefficients = BTreeMap::new();
    for ((mapping, residual_key), (sham_key, shuffled_key)) in MAPPINGS
        .into_iter()
        .zip(["residual_forward", "residual_reverse"])
        .zip([
            ("sham_forward", "shuffle_residual_forward"),
            ("sham_reverse", "shuffle_residual_reverse"),
        ])
    {
        let (balanced, residual) = projection(&old[mapping], nuisance)?;
        let (sham, sham_residual) = projection(&old[mapping], &sham_nuisance)?;
        directions.insert(shuffled_key, permute(&residual));
        directions.insert(residual_key, residual);
        directions.insert(sham_key, sham_residual);
        coefficients.insert(
            mapping,
            json!({"balanced_coefficient": balanced, "sham_coefficient": sham}),
        );
    }

    let old_ray = requested(archive, "ray")?;
    let new_ray = directions
        .values()
        .filter_map(|vector| limit(&base, vector))
        .fold(f64::INFINITY, f64::min)
        * MARGIN;
    let ray = old_ray.min(new_ray);
    if ray <= 0. {
        return Err(error("no positive common ray for residual comparisons"));
    }
    let mut tiers = BTreeMap::new();
    for (tier, dose, reuse) in [
        ("ray", ray, ray == old_ray),
        ("bounded", requested(archive, "bounded")?, true),
    ] {
        let mut arms = BTreeMap::new();
        let observe = |direction: &[f64]| -> Result<Value> {
            let (weights, plan) = plan_at(tier, &base, direction, dose)?;
            let mut observation = carrier.observe(&weights, Some(&baseline))?;
            observation["plan"] = plan;
            Ok(observation)
        };
        for (&name, direction) in &directions {
            arms.insert(name, observe(direction)?);
        }
        if !reuse {
            for &name in &controls {
                arms.insert(name, observe(&old[name])?);
            }
        }
        let reused: &[_] = if reuse { &controls } else { &[] };
        tiers.insert(
            tier,
            json!({"requested_norm":dose, "reused_controls":reused, "arms":arms}),
        );
    }
    Ok(json!({
        "protocol": PROTOCOL, "seed": seed, "archived_record_checksum": checksum,
        "checkpoint_time_us": checkpoint, "projection": coefficients,
        "directions": directions, "tiers": tiers,
    }))
}

pub fn replay_grown_residual(
    capacity: &Value,
    consolidation: &Value,
    archive: &Value,
) -> Result<Value> {
    let (source, checksum) = checked_capacity(capacity)?;
    let mut result = run(source, consolidation, archive, &checksum)?;
    if result != run(source, consolidation, archive, &checksum)? {
        return Err(error("grown residual replay differs"));
    }
    result["replay_verified"] = json!(true);
    Ok(result)
}
