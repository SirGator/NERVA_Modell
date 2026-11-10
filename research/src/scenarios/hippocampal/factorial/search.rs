//! Fixed oracle directions using measured activity and existing recurrent routes.
use super::super::{
    association::Training,
    diagnosis::{Edge, probe::Probe},
};
use std::collections::BTreeMap;
pub(super) const METHODS: [&str; 3] = [
    "sensory_and_recurrent",
    "recurrent_contrast",
    "recurrent_two_hop",
];
pub(super) fn normalize(mut values: Vec<f64>) -> Vec<f64> {
    let scale = values.iter().map(|v| v.abs()).fold(0f64, f64::max);
    if scale > 0. {
        for v in &mut values {
            *v /= scale;
        }
    }
    values
}
pub(super) fn scores(method: &str, edges: &[Edge], training: &Training, base: &Probe) -> Vec<f64> {
    if method == "sensory_and_recurrent" {
        return normalize(edges.iter().map(|e| e.score).collect());
    }
    let count = |image: usize, id: u64| *base.source_counts[image].get(&id).unwrap_or(&0) as f64;
    let ids = training.model.processing_ids();
    let mut cue: BTreeMap<_, _> = ids
        .iter()
        .map(|id| (id.0, count(3, id.0) - count(4, id.0)))
        .collect();
    let mut target: BTreeMap<_, _> = ids
        .iter()
        .map(|id| (id.0, count(1, id.0) - count(2, id.0)))
        .collect();
    fn normalize_map(values: &mut BTreeMap<u64, f64>) {
        let scale = values.values().map(|v| v.abs()).fold(0f64, f64::max);
        if scale > 0. {
            for v in values.values_mut() {
                *v /= scale;
            }
        }
    }
    normalize_map(&mut cue);
    normalize_map(&mut target);
    let mut forward = BTreeMap::<u64, f64>::new();
    let mut backward = BTreeMap::<u64, f64>::new();
    if method == "recurrent_two_hop" {
        // Excitatory relay paths only. No synthetic edges or forced spikes are introduced.
        for e in edges.iter().filter(|e| e.path == "core_exc_to_core") {
            if training
                .model
                .network()
                .synapse(e.id)
                .is_some_and(|s| s.is_enabled())
            {
                *forward.entry(e.post).or_default() += cue[&e.pre];
                *backward.entry(e.pre).or_default() += target[&e.post];
            }
        }
        normalize_map(&mut forward);
        normalize_map(&mut backward);
    }
    normalize(
        edges
            .iter()
            .map(|e| {
                if !e.path.starts_with("core_")
                    || !training
                        .model
                        .network()
                        .synapse(e.id)
                        .is_some_and(|s| s.is_enabled() && s.is_plastic())
                {
                    return 0.;
                }
                let sign = if e.path == "core_inh_to_core" {
                    -1.
                } else {
                    1.
                };
                let q = cue[&e.pre];
                let t = target[&e.post];
                sign * (q * t
                    + q * backward.get(&e.post).copied().unwrap_or(0.)
                    + forward.get(&e.pre).copied().unwrap_or(0.) * t)
            })
            .collect(),
    )
}
