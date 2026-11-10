//! Fixed finite doses; native hippocampal deltas and a guaranteed bench path.
use super::{GAINS, Outcome, flag, memory};
use crate::scenarios::hippocampal::{
    Config,
    association::{recall, train},
    causal::verify_only_weights,
};
use crate::{Result, error};
use serde_json::{Value, json};

fn different(a: &Value, b: &Value, field: &str) -> bool {
    a["observations"]
        .as_array()
        .unwrap()
        .iter()
        .zip(b["observations"].as_array().unwrap())
        .any(|(x, y)| x[field] != y[field])
}
pub(super) fn run(c: &Config, seed: u64) -> Result<Outcome> {
    let reference = train(c, seed, "three_on", true)?;
    let learned: Vec<_> = reference
        .model
        .network()
        .synapses()
        .map(|s| (s.id(), s.weight().get()))
        .collect();
    let mut doses = Vec::new();
    let mut baseline = Value::Null;
    for gain in GAINS {
        let mut branch = train(c, seed, "three_on", false)?;
        if !reference.model.same_replay_state(&branch.model) {
            return Err(error("gain checkpoint differs before intervention"));
        }
        let mut clipped = 0;
        let mut actual_l1 = 0.;
        let mut maximum = 0f64;
        let mut desired = Vec::new();
        for (&(id, w), &(old_id, w0)) in learned.iter().zip(&reference.weights) {
            if id != old_id {
                return Err(error("gain edge identity differs"));
            }
            let unbounded = f64::from(w0) + gain * (f64::from(w) - f64::from(w0));
            clipped += usize::from(!(0. ..=2.).contains(&unbounded));
            let value = unbounded.clamp(0., 2.) as f32;
            let delta = (f64::from(value) - f64::from(w0)).abs();
            actual_l1 += delta;
            maximum = maximum.max(delta);
            desired.push((id, value));
        }
        verify_only_weights(&reference.model, &mut branch.model, &desired)?;
        let response = recall(&mut branch.model, c)?;
        if gain == 0. {
            baseline = response.clone();
        }
        doses.push(json!({"gain":gain,"installed_l1_from_initial":actual_l1,"maximum_delta":maximum,
            "clipped_edges":clipped,"matched_checkpoint":true,"only_weights_changed":true,
            "core_effect":different(&response,&baseline,"core_event_digest"),
            "motor_effect":different(&response,&baseline,"motor_event_digest"),
            "potential_effect":different(&response,&baseline,"stored_potentials"),"recall":response}));
    }
    let mut bench = Vec::new();
    for weight in [0., 0.5, 1., 1.5, 2.] {
        bench.push(memory::probe(weight, true, Some(1), c.rest_us)?);
    }
    let bench_passed = bench[0]["target_fired"] == false && bench[3]["target_fired"] == true;
    let first = doses
        .iter()
        .find(|r| r["gain"].as_f64().unwrap() > 0. && r["core_effect"] == true)
        .map(|r| r["gain"].clone())
        .unwrap_or(Value::Null);
    let native = &doses[1];
    let before = &reference.details["weight_changes"]["all"];
    let representation_distinct =
        doses[0]["recall"]["core_counts"][0] != doses[0]["recall"]["core_counts"][1];
    let natural = flag(native["core_effect"] == true);
    let artificial = flag(doses.iter().skip(2).any(|r| r["core_effect"] == true));
    Ok(Outcome {
        stages: [
            flag(reference.details["input_patterns_disjoint"] == true),
            flag(representation_distinct),
            flag(reference.details["audit"]["tag_events"].as_u64().unwrap() > 0),
            flag(before["l1"].as_f64().unwrap() > 0.),
            natural,
            artificial,
        ],
        passed: bench_passed,
        rows: reference.details["audit"]["observed_rows"]
            .as_u64()
            .unwrap() as usize,
        training: 6,
        details: json!({"scope":"Hippocampal matched-state weight intervention plus a separate calibrated microcircuit",
            "training":reference.details,"initial_network_digest":reference.initial_digest,
            "doses":doses,"first_sampled_core_effect_gain":first,"bench_weight_curve":bench,
            "gains":GAINS,"native_gain":1,"artificial_gains":[10,100,1000],
            "limits":"No adaptive search. Absence within the finite grid is not an infinite threshold. Clipping and nonmonotonic effects are retained. The bench path cannot certify hippocampal recall."}),
    })
}
