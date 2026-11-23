//! Equal-dose probes of learned and control weight vectors on a common grown graph.
use super::{
    Assay, Endpoints, MAPPINGS, Model, PROBES, SynapseId, Targets, build, checked_capacity,
    consolidation::{PROTOCOL as CONSOLIDATION, STRUCTURE},
    error, fail, probe, recorded_config, topology, train,
};
use crate::Result;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

use super::super::super::{
    super::factorial::weights::{limit, norm, plan},
    Weights,
    electrical::Trace,
    verify_only_weights,
};

const PROTOCOL: &str = "NERVA-Contingency-Grown-Dose/1";
const MARGIN: f64 = 0.90;
const REFERENCE_NAMES: [&str; 2] = ["reference_forward", "reference_reverse"];
type Directions = BTreeMap<&'static str, Vec<f64>>;

mod residual;
pub use residual::replay_grown_residual;

fn mixed(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let value = (*state ^ (*state >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    let value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

fn permutation(targets: impl Iterator<Item = u64>, seed: u64) -> Vec<usize> {
    let mut groups = BTreeMap::<u64, Vec<usize>>::new();
    let mut permutation = Vec::new();
    for (index, target) in targets.enumerate() {
        groups.entry(target).or_default().push(index);
        permutation.push(index);
    }
    let mut state = seed ^ 0xd6e8_feb8_6659_fd93;
    for indices in groups.values() {
        for i in (1..indices.len()).rev() {
            let j = (mixed(&mut state) % (i as u64 + 1)) as usize;
            permutation.swap(indices[i], indices[j]);
        }
    }
    permutation
}

fn headroom(weight: f32, delta: f64) -> f64 {
    let weight = f64::from(weight);
    if delta > 0. {
        2. - weight
    } else if delta < 0. {
        weight
    } else {
        0.
    }
}

fn saturation_limit(base: &Weights, direction: &[f64]) -> f64 {
    base.iter()
        .zip(direction)
        .map(|(&(_, weight), &delta)| headroom(weight, delta).powi(2))
        .sum::<f64>()
        .sqrt()
}

fn bounded_plan(base: &Weights, direction: &[f64], requested: f64) -> Result<(Weights, Value)> {
    let ceiling = saturation_limit(base, direction);
    if !requested.is_finite() || requested <= 0. || requested > ceiling {
        return Err(error("matched bounded weight dose is infeasible"));
    }
    let mut high = base
        .iter()
        .zip(direction)
        .filter(|&(_, &delta)| delta != 0.)
        .map(|(&(_, weight), &delta)| headroom(weight, delta) / delta.abs())
        .fold(0., f64::max);
    let mut low = 0.;
    let mut best_scale = 0.;
    let mut best = base.clone();
    let mut best_norm = 0.;
    let actual_delta = |weights: &Weights| {
        weights
            .iter()
            .zip(base)
            .map(|(&(_, new), &(_, old))| f64::from(new) - f64::from(old))
            .collect::<Vec<_>>()
    };
    for _ in 0..64 {
        let scale = (low + high) * 0.5;
        let weights: Weights = base
            .iter()
            .zip(direction)
            .map(|(&(id, weight), &delta)| {
                (id, (f64::from(weight) + scale * delta).clamp(0., 2.) as f32)
            })
            .collect();
        let actual = norm(&actual_delta(&weights));
        if (actual - requested).abs() < (best_norm - requested).abs() {
            best = weights;
            best_scale = scale;
            best_norm = actual;
        }
        if actual < requested {
            low = scale;
        } else {
            high = scale;
        }
    }
    let error_fraction = (best_norm - requested).abs() / requested;
    if error_fraction > 0.01 || best_norm == 0. {
        return Err(error(
            "bounded publication failed the matched dose tolerance",
        ));
    }
    let cosine = actual_delta(&best)
        .iter()
        .zip(direction)
        .map(|(actual, requested)| actual * requested)
        .sum::<f64>()
        / (best_norm * norm(direction));
    let clipped = base
        .iter()
        .zip(direction)
        .filter(|&(&(_, w), &d)| !(0. ..=2.).contains(&(f64::from(w) + best_scale * d)))
        .count();
    Ok((
        best,
        json!({
            "scale": best_scale, "actual_norm": best_norm,
            "relative_norm_error": error_fraction, "direction_cosine": cosine.clamp(-1., 1.),
            "clipped_coordinates": clipped,
        }),
    ))
}

fn archived_weights(topology: &Value) -> Result<BTreeMap<u64, f32>> {
    let rows: Endpoints = serde_json::from_value(topology.clone()).map_err(fail)?;
    let mut weights = BTreeMap::new();
    for (id, _, _, weight, _, enabled) in rows {
        if !enabled || !(0. ..=2.).contains(&weight) || weights.insert(id, weight).is_some() {
            return Err(error("invalid grown weight table"));
        }
    }
    Ok(weights)
}

struct Carrier<'a> {
    assay: &'a Assay<'a>,
    parent: Model,
    targets: BTreeSet<u64>,
}

impl Carrier<'_> {
    fn observe(&self, weights: &Weights, baseline: Option<&Value>) -> Result<Value> {
        let mut probes = BTreeMap::new();
        for cue in PROBES {
            let (mut branch, _) = train(self.assay, "balanced", STRUCTURE, None, None, None)?;
            verify_only_weights(&self.parent, &mut branch, weights)?;
            if weights
                .iter()
                .any(|&(id, weight)| branch.network().synapse(id).unwrap().weight().get() != weight)
            {
                return Err(error("grown dose installed different weights"));
            }
            let mut trace = Trace::new(&self.targets);
            let mut result = probe(
                &mut branch,
                self.assay.c,
                &self.assay.frames[cue],
                &self.assay.frames["blank"],
                (cue != "blank").then_some(&mut trace),
            )?
            .json();
            if cue != "blank" {
                trace.attach(&mut result);
            }
            if let Some(baseline) = baseline
                && ["start_us", "root_spikes"]
                    .into_iter()
                    .any(|key| result[key] != baseline["probes"][cue][key])
            {
                return Err(error("grown dose changed probe root input"));
            }
            probes.insert(cue, result);
        }
        let visible: Vec<_> = weights.iter().map(|&(id, w)| (id.0, w)).collect();
        Ok(json!({"weights": visible, "probes": probes}))
    }
}

fn verify_checkpoint(
    assay: &Assay<'_>,
    details: &Value,
    development: &Value,
    archive: &Value,
    checksum: &str,
) -> Result<()> {
    if archive["protocol"] != CONSOLIDATION
        || archive["replay_verified"] != true
        || archive["archived_record_checksum"] != checksum
        || archive["seed"] != assay.seed
        || archive["checkpoint_time_us"] != development["checkpoint_time_us"]
        || archive["core_ids"] != json!(assay.core)
        || details["development"] != *development
        || details["topology"] != json!(assay.topology())
    {
        return Err(error(
            "grown dose inputs differ from archived consolidation",
        ));
    }
    Ok(())
}

fn make_carrier<'a>(
    assay: &'a Assay<'a>,
    archive: &Value,
    targets: &Targets,
) -> Result<Carrier<'a>> {
    let mut focus = BTreeSet::new();
    let mut quiet = BTreeSet::new();
    for mapping in MAPPINGS {
        let edges: Vec<(u64, u64, u64, u64)> =
            serde_json::from_value(archive["focus"][mapping].clone()).map_err(fail)?;
        focus.extend(edges.into_iter().map(|(id, _, _, _)| SynapseId(id)));
        quiet.extend(
            serde_json::from_value::<Vec<u64>>(archive["quiet_sources"][mapping].clone())
                .map_err(fail)?,
        );
    }
    let (parent, training) = train(
        assay,
        "balanced",
        STRUCTURE,
        Some((&focus, &quiet)),
        None,
        None,
    )?;
    if training != archive["mappings"]["balanced"]["training"] {
        return Err(error(
            "grown carrier differs from archived balanced training",
        ));
    }
    Ok(Carrier {
        assay,
        parent,
        targets: targets.iter().flatten().copied().collect(),
    })
}

fn plan_at(tier: &str, base: &Weights, direction: &[f64], dose: f64) -> Result<(Weights, Value)> {
    if tier == "ray" {
        let normalized = plan(base, direction, dose);
        if normalized.details["faithful"] != true {
            return Err(error("published ray is not direction-faithful"));
        }
        Ok((normalized.weights.unwrap(), normalized.details))
    } else {
        bounded_plan(base, direction, dose)
    }
}

fn run(source: &Value, archive: &Value, checksum: &str) -> Result<Value> {
    let (config, seed) = recorded_config(source)?;
    let (assay, development, _) = build(&config, seed)?;
    let details = &source["details"];
    verify_checkpoint(&assay, details, &development, archive, checksum)?;
    let targets: Targets =
        serde_json::from_value(details["target_groups"].clone()).map_err(fail)?;
    let carrier = make_carrier(&assay, archive, &targets)?;
    let grown = json!(topology(&carrier.parent));
    let support: Vec<_> = assay.edges.iter().filter(|edge| edge.eligible).collect();
    let base: Weights = support
        .iter()
        .map(|edge| {
            let id = SynapseId(edge.id);
            let synapse = carrier
                .parent
                .network()
                .synapse(id)
                .expect("initial edge retained");
            (id, synapse.weight().get())
        })
        .collect();

    let mut directions = Directions::new();
    for mapping in MAPPINGS.into_iter().chain(["balanced"]) {
        let end = archived_weights(&archive["mappings"][mapping]["training"]["end_topology"])?;
        let vector = support
            .iter()
            .map(|edge| {
                end.get(&edge.id)
                    .map(|&new| f64::from(new) - f64::from(edge.initial))
                    .ok_or_else(|| error("learned direction lost a supported edge"))
            })
            .collect::<Result<Vec<_>>>()?;
        directions.insert(mapping, vector);
    }
    let order = permutation(support.iter().map(|edge| edge.post), seed);
    for (mapping, name) in MAPPINGS
        .into_iter()
        .zip(["shuffle_forward", "shuffle_reverse"])
    {
        directions.insert(
            name,
            order
                .iter()
                .map(|&index| directions[mapping][index])
                .collect(),
        );
    }
    let choice = details["followup"]["choice"]
        .as_str()
        .ok_or_else(|| error("capacity reference choice missing"))?;
    let mut full_reference = BTreeMap::new();
    for (mapping, name) in MAPPINGS.into_iter().zip(REFERENCE_NAMES) {
        let rows: Vec<(u64, f32)> = serde_json::from_value(
            details["references"][choice]["arms"][mapping]["weights"].clone(),
        )
        .map_err(fail)?;
        let archived: BTreeMap<_, _> = rows.into_iter().collect();
        let weights = support
            .iter()
            .zip(&base)
            .map(|(edge, &(id, anchor))| {
                let selected = archived
                    .get(&edge.id)
                    .copied()
                    .ok_or_else(|| error("reference lost a supported edge"))?;
                Ok((
                    id,
                    if selected == edge.initial {
                        anchor
                    } else {
                        selected
                    },
                ))
            })
            .collect::<Result<Weights>>()?;
        directions.insert(
            name,
            weights
                .iter()
                .zip(&base)
                .map(|(&(_, w), &(_, b))| f64::from(w) - f64::from(b))
                .collect(),
        );
        full_reference.insert(mapping, weights);
    }

    let unavailable: BTreeSet<_> = directions
        .iter()
        .filter(|(_, vector)| vector.iter().all(|&value| value == 0.))
        .map(|(&name, _)| name)
        .collect();
    if unavailable.len() == REFERENCE_NAMES.len()
        || unavailable
            .iter()
            .any(|name| !REFERENCE_NAMES.contains(name))
    {
        return Err(error("missing required grown-dose direction"));
    }
    let available = || {
        directions
            .iter()
            .filter(|&(name, _)| !unavailable.contains(name))
    };
    let common_ray = available()
        .filter_map(|(_, direction)| limit(&base, direction))
        .fold(f64::INFINITY, f64::min)
        * MARGIN;
    let common_bounded = available()
        .map(|(_, direction)| saturation_limit(&base, direction))
        .chain(
            REFERENCE_NAMES
                .iter()
                .filter(|name| !unavailable.contains(*name))
                .map(|name| norm(&directions[name])),
        )
        .fold(f64::INFINITY, f64::min)
        * MARGIN;
    if !(common_bounded.is_finite()
        && common_ray.is_finite()
        && common_ray > 0.
        && common_bounded > 0.)
    {
        return Err(error("invalid common grown-dose bounds"));
    }
    let baseline = carrier.observe(&base, None)?;
    let reference_full = full_reference
        .into_iter()
        .map(|(name, weights)| {
            let observed = if weights == base {
                baseline.clone()
            } else {
                carrier.observe(&weights, Some(&baseline))?
            };
            Ok((name, observed))
        })
        .collect::<Result<BTreeMap<_, _>>>()?;
    let mut tiers = BTreeMap::new();
    for (tier, dose) in [("ray", common_ray), ("bounded", common_bounded)] {
        let mut arms = BTreeMap::new();
        for (&name, direction) in available() {
            let (weights, evidence) = plan_at(tier, &base, direction, dose)?;
            let mut data = carrier.observe(&weights, Some(&baseline))?;
            data["plan"] = evidence;
            arms.insert(name, data);
        }
        tiers.insert(tier, json!({"requested_norm":dose, "arms":arms}));
    }
    Ok(json!({
        "protocol": PROTOCOL, "seed": seed, "archived_record_checksum": checksum,
        "checkpoint_time_us": development["checkpoint_time_us"],
        "core_ids": assay.core, "target_groups": targets,
        "support": support.iter().map(|edge| edge.id).collect::<Vec<_>>(),
        "carrier_topology": grown, "reference_choice": choice,
        "directions": directions, "unavailable_directions": unavailable,
        "baseline":baseline, "reference_full": reference_full,
        "tiers": tiers,
    }))
}

pub fn replay_grown_dose(capacity: &Value, consolidation: &Value) -> Result<Value> {
    let (source, checksum) = checked_capacity(capacity)?;
    let mut result = run(source, consolidation, &checksum)?;
    if result != run(source, consolidation, &checksum)? {
        return Err(error("grown dose replay differs"));
    }
    result["replay_verified"] = json!(true);
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_dose_preserves_sign_and_reports_rotation() {
        let base = vec![
            (SynapseId(1), 0.1),
            (SynapseId(2), 0.1),
            (SynapseId(3), 0.7),
        ];
        let direction = [-10., 1., 0.];
        let (weights, plan) = bounded_plan(&base, &direction, 1.).unwrap();
        assert_eq!(weights[0].1, 0.);
        assert!(weights[1].1 > 1.);
        assert_eq!(weights[2].1, base[2].1);
        assert_eq!(plan["clipped_coordinates"], 1);
        assert!(plan["direction_cosine"].as_f64().unwrap() < 0.5);
        assert!(plan["relative_norm_error"].as_f64().unwrap() < 1e-6);
    }
}
