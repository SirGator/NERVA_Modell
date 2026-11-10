//! Read-only training-weight measurements; no neural transitions live here.
use crate::{Result, error};
use nerva::core::{Network, NeuronRole, Polarity};
use nerva_vm::model::Model;
use serde_json::{Value, json};
use std::collections::BTreeMap;

// Exact widened f32 values keep external recalculation independent of decimal
// rounding. These bounded read-only snapshots never advance neural state.
#[derive(Clone, Copy)]
pub(super) struct WeightRow {
    synapse_id: u64,
    pre: u64,
    post: u64,
    weight: f64,
    stored_learning_tag: f64,
    tag_updated_at_us: Option<u64>,
}

impl WeightRow {
    fn identity(self) -> (u64, u64, u64) {
        (self.synapse_id, self.pre, self.post)
    }

    fn json(self) -> Value {
        json!([
            self.synapse_id,
            self.pre,
            self.post,
            self.weight,
            self.stored_learning_tag,
            self.tag_updated_at_us
        ])
    }
}

pub(super) fn snapshot_json(rows: &[WeightRow]) -> Value {
    Value::Array(rows.iter().map(|row| row.json()).collect())
}
pub(super) fn weight_snapshot(model: &Model) -> Vec<WeightRow> {
    model
        .network()
        .synapses()
        .map(|edge| WeightRow {
            synapse_id: edge.id().0,
            pre: edge.pre().0,
            post: edge.post().0,
            weight: f64::from(edge.weight().get()),
            stored_learning_tag: f64::from(edge.learning_eligibility()),
            tag_updated_at_us: edge.learning_eligibility_updated_at().map(|at| at.0),
        })
        .collect()
}
#[derive(Default)]
struct WeightDelta {
    edges: usize,
    changed: usize,
    l1: f64,
    maximum: f64,
    signed: f64,
    stored_tag_l1: f64,
}
impl WeightDelta {
    fn add(&mut self, before: f64, after: f64, tag: f64) {
        let delta = after - before;
        self.edges += 1;
        self.changed += usize::from(delta != 0.);
        self.l1 += delta.abs();
        self.maximum = self.maximum.max(delta.abs());
        self.signed += delta;
        self.stored_tag_l1 += tag.abs();
    }
    fn json(&self) -> Value {
        json!({"edges":self.edges,"changed":self.changed,"l1":self.l1,
            "maximum":self.maximum,"signed":self.signed,"stored_tag_l1":self.stored_tag_l1})
    }
}
pub(super) fn weight_changes(
    before: &[WeightRow],
    after: &[WeightRow],
    network: &Network,
) -> Result<Value> {
    if before.len() != after.len() {
        return Err(error("training changed topology"));
    }
    let mut total = WeightDelta::default();
    let mut paths = BTreeMap::<&str, WeightDelta>::new();
    for (a, b) in before.iter().zip(after) {
        if a.identity() != b.identity() {
            return Err(error("training changed synapse identities"));
        }
        let pre = network
            .neuron(nerva::core::NeuronId(a.pre))
            .ok_or_else(|| error("weight snapshot source missing"))?;
        let post = network
            .neuron(nerva::core::NeuronId(a.post))
            .ok_or_else(|| error("weight snapshot target missing"))?;
        let path = match (pre.role(), post.role(), pre.polarity()) {
            (Some(NeuronRole::Sensory), Some(NeuronRole::Processing), _) => "sensory_to_core",
            (Some(NeuronRole::Processing), Some(NeuronRole::Processing), Polarity::Excitatory) => {
                "core_exc_to_core"
            }
            (Some(NeuronRole::Processing), Some(NeuronRole::Processing), Polarity::Inhibitory) => {
                "core_inh_to_core"
            }
            (Some(NeuronRole::Processing), Some(NeuronRole::Motor), _) => "core_to_motor",
            _ => "other",
        };
        total.add(a.weight, b.weight, b.stored_learning_tag);
        paths
            .entry(path)
            .or_default()
            .add(a.weight, b.weight, b.stored_learning_tag);
    }
    let paths: BTreeMap<_, _> = paths
        .into_iter()
        .map(|(name, value)| (name, value.json()))
        .collect();
    Ok(json!({"all":total.json(),"by_path":paths}))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn weight_summary_distinguishes_update_counts_from_net_magnitude() {
        let mut summary = WeightDelta::default();
        summary.add(0.5, 0.75, -0.125);
        summary.add(1., 0.5, 0.25);
        summary.add(0.25, 0.25, 0.);
        let result = summary.json();
        assert_eq!(result["edges"], 3);
        assert_eq!(result["changed"], 2);
        assert_eq!(result["l1"], 0.75);
        assert_eq!(result["maximum"], 0.5);
        assert_eq!(result["signed"], -0.25);
        assert_eq!(result["stored_tag_l1"], 0.375);
        let row = WeightRow {
            synapse_id: 1,
            pre: 2,
            post: 3,
            weight: 0.5,
            stored_learning_tag: 0.,
            tag_updated_at_us: None,
        };
        assert!(weight_changes(&[row], &[], &Network::new()).is_err());
        assert!(
            weight_changes(
                &[row],
                &[WeightRow {
                    synapse_id: 2,
                    ..row
                }],
                &Network::new()
            )
            .is_err()
        );
    }
}
