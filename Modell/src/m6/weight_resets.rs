//! Optional frozen counterfactuals; no changes to the parent or historical CSVs.
use super::*;
use nerva::core::Network;

#[allow(clippy::too_many_arguments)]
pub(super) fn probe(
    session: &Session<'_>,
    parent: &Actor,
    arm: &str,
    gain: f32,
    raw: bool,
    initial: &Network,
    calibration: &Calibration,
    rest_us: u64,
) -> Result<Value> {
    let checkpoint = parent.model.network().clone();
    let checkpoint_us = parent.model.next_observation_us();
    let mut stages = serde_json::Map::new();
    for (stage, rest) in [("checkpoint_probe", 0), ("rest_checkpoint_probe", rest_us)] {
        let mut interventions = serde_json::Map::new();
        for intervention in ["sham", "restore_core", "restore_all_plastic"] {
            let mut trials = Vec::new();
            for key in [Key::A, Key::D] {
                let mut branch = session.replay(parent, arm, gain, raw)?;
                let mut remaining = rest;
                while remaining > 0 {
                    let duration = remaining.min(BASE);
                    branch.observe(&session.frames[3], None, duration)?;
                    remaining -= duration;
                }
                let before = branch.model.network().clone();
                let intervention_us = branch.model.next_observation_us();
                let mut edits = Vec::new();
                for edge in initial.synapses() {
                    let core = branch.model.processing_ids().contains(&edge.pre())
                        && branch.model.processing_ids().contains(&edge.post());
                    let selected = (intervention == "restore_core" && core)
                        || (intervention == "restore_all_plastic" && edge.is_plastic());
                    let old = before
                        .synapse(edge.id())
                        .ok_or("reset edge missing")?
                        .weight()
                        .get();
                    if selected && old != edge.weight().get() {
                        edits.push((edge.id(), old, edge.weight().get()));
                    }
                }
                for &(id, _, weight) in &edits {
                    branch.model.diagnostic_synapse_weight(id, weight)?;
                }
                ensure(
                    before.neurons().eq(branch.model.network().neurons()),
                    "weight reset changed neuron state",
                )?;
                ensure(
                    branch.model.next_observation_us() == intervention_us,
                    "weight reset advanced time",
                )?;
                let after = branch.model.network().clone();
                // Neutral images only. No world or effect label enters the actor.
                branch.observe(&session.frames[3], None, BASE)?;
                let action_us = branch.model.next_observation_us();
                let mut cue: Option<Step> = None;
                let mut copies = 0;
                for duration in [10_000, 10_000, 30_000] {
                    let (step, audit) = branch.observe(&session.frames[3], Some(key), duration)?;
                    copies += audit.action_copy_arrivals;
                    if let Some(total) = &mut cue {
                        total.end_us = step.end_us;
                        for (count, additional) in
                            total.core_counts.iter_mut().zip(step.core_counts)
                        {
                            *count += additional;
                        }
                        total.core_events.extend(step.core_events);
                    } else {
                        cue = Some(step);
                    }
                }
                ensure(copies == 9, "reset probe action copies differ")?;
                let cue = cue.unwrap();
                let prediction = calibration.readouts[0].predict(&core_features(
                    &cue,
                    branch.model.processing_ids(),
                    false,
                ));
                ensure(
                    learned_parameters_equal(&after, branch.model.network()),
                    "reset probe resumed plasticity",
                )?;
                ensure(
                    parent.model.network() == &checkpoint
                        && parent.model.next_observation_us() == checkpoint_us,
                    "reset probe changed parent",
                )?;
                let potentials: Vec<_> = branch
                    .model
                    .processing_ids()
                    .iter()
                    .map(|id| {
                        branch
                            .model
                            .network()
                            .neuron(*id)
                            .unwrap()
                            .membrane_potential()
                    })
                    .collect();
                trials.push(json!({
                    "key":key.qcode(),"checkpoint_us":checkpoint_us,"rest_us":rest,"intervention_us":intervention_us,
                    "action_us":action_us,"forecast_us":cue.end_us,"prediction_count":prediction,
                    "core_counts":cue.core_counts,"core_events":cue.core_events,"core_potentials":potentials,
                    "copy_arrivals":copies,"effect_not_presented":true,"changed_weights":edits.iter().map(|(id,old,new)|(id.0,old,new)).collect::<Vec<_>>(),
                    "exact_replay_state_before_rest":true,"neurons_preserved_at_intervention":true,
                    "frozen_parameters_preserved":true,"parent_unchanged":true
                }));
            }
            interventions.insert(intervention.to_owned(), json!(trials));
        }
        stages.insert(stage.to_owned(), Value::Object(interventions));
    }
    Ok(json!({"count_readout":calibration.readouts[0].snapshot(),"stages":stages}))
}
