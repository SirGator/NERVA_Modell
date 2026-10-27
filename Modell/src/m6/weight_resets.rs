//! Optional frozen counterfactuals; no changes to the parent or historical CSVs.
use super::*;
use nerva::core::Network;
use nerva::runtime::ObservationEvent;

fn potentials_at(actor: &Actor, time: u64) -> Result<Vec<f32>> {
    actor
        .model
        .processing_ids()
        .iter()
        .map(|id| {
            let mut copy = actor.model.network().neuron(*id).unwrap().clone();
            copy.advance_to(nerva::core::SimTime(time))?;
            Ok(copy.membrane_potential())
        })
        .collect()
}

fn observed_phase(
    actor: &mut Actor,
    frame: &Frame,
    key: Option<Key>,
    duration: u64,
    trace: &mut Vec<Value>,
    enabled: bool,
) -> Result<(Step, InputAudit)> {
    let before_us = actor.model.next_observation_us() - 1;
    let before = if enabled {
        potentials_at(actor, before_us)?
    } else {
        Vec::new()
    };
    let refractory_before: Vec<_> = if enabled {
        actor
            .model
            .processing_ids()
            .iter()
            .map(|id| {
                actor
                    .model
                    .network()
                    .neuron(*id)
                    .unwrap()
                    .refractory_until()
                    .0
            })
            .collect()
    } else {
        Vec::new()
    };
    let (step, audit) = actor.observe(frame, key, duration)?;
    if enabled {
        let events: Result<Vec<_>> = audit.raw_events.iter().map(|event| Ok(match event {
            ObservationEvent::ExternalInput {time,target,amplitude} => json!({"kind":"external","time_us":time.0,"target":target.0,"amplitude":amplitude}),
            ObservationEvent::SynapticArrival {time,synapse_id,source,target,amplitude} => json!({"kind":"arrival","time_us":time.0,"synapse":synapse_id.0,"source":source.0,"target":target.0,"amplitude":amplitude}),
            ObservationEvent::SpikeEmitted(spike) => json!({"kind":"spike","time_us":spike.time.0,"source":spike.neuron_id.0}),
            _ => return Err("plasticity event in frozen M10 trace".into()),
        })).collect();
        trace.push(json!({"start_us":step.start_us,"end_us":step.end_us,"before_us":before_us,
            "potentials_before":before,"potentials_after":potentials_at(actor,step.end_us)?,
            "refractory_until_before":refractory_before,
            "refractory_until_after":actor.model.processing_ids().iter().map(|id|actor.model.network().neuron(*id).unwrap().refractory_until().0).collect::<Vec<_>>(),
            "events":events?,"core_counts":step.core_counts,
            "arrivals_per_core":audit.core_arrivals.iter().map(|a|a.arrivals).collect::<Vec<_>>(),
            "signed_input_per_core":audit.core_arrivals.iter().map(|a|a.amplitude_sum).collect::<Vec<_>>(),
            "absolute_input_per_core":audit.core_arrivals.iter().map(|a|a.absolute_amplitude_sum).collect::<Vec<_>>(),
            "copy_arrivals":audit.action_copy_arrivals}));
    }
    Ok((step, audit))
}

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
    trace_enabled: bool,
) -> Result<Value> {
    probe_impl(
        session,
        parent,
        arm,
        gain,
        raw,
        initial,
        calibration,
        rest_us,
        trace_enabled,
        false,
        false,
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn recurrence_probe(
    session: &Session<'_>,
    parent: &Actor,
    arm: &str,
    gain: f32,
    raw: bool,
    initial: &Network,
    calibration: &Calibration,
    rest_us: u64,
) -> Result<Value> {
    probe_impl(
        session,
        parent,
        arm,
        gain,
        raw,
        initial,
        calibration,
        rest_us,
        true,
        true,
        false,
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn crossed_probe(
    session: &Session<'_>,
    parent: &Actor,
    arm: &str,
    gain: f32,
    raw: bool,
    initial: &Network,
    calibration: &Calibration,
    rest_us: u64,
) -> Result<Value> {
    probe_impl(
        session,
        parent,
        arm,
        gain,
        raw,
        initial,
        calibration,
        rest_us,
        true,
        true,
        true,
    )
}

#[allow(clippy::too_many_arguments)]
fn probe_impl(
    session: &Session<'_>,
    parent: &Actor,
    arm: &str,
    gain: f32,
    raw: bool,
    initial: &Network,
    calibration: &Calibration,
    rest_us: u64,
    trace_enabled: bool,
    recurrence: bool,
    crossed: bool,
) -> Result<Value> {
    let checkpoint = parent.model.network().clone();
    let checkpoint_us = parent.model.next_observation_us();
    let mut stages = serde_json::Map::new();
    for (stage, rest) in [("checkpoint_probe", 0), ("rest_checkpoint_probe", rest_us)] {
        let mut interventions = serde_json::Map::new();
        let variants: &[&str] = if crossed {
            &[
                "trained__sham",
                "trained__block_exc",
                "trained__block_inh",
                "trained__block_both",
                "reset_core__sham",
                "reset_core__block_exc",
                "reset_core__block_inh",
                "reset_core__block_both",
            ]
        } else if recurrence {
            &["sham", "block_exc", "block_inh", "block_both"]
        } else {
            &["sham", "restore_core", "restore_all_plastic"]
        };
        for &intervention in variants {
            let (weight_mode, gate_mode) = if crossed {
                intervention.split_once("__").unwrap()
            } else {
                ("trained", intervention)
            };
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
                if trace_enabled {
                    branch.model.enable_audit(AuditOptions {
                        raw_events: true,
                        input_spikes: true,
                    });
                }
                let mut trace = Vec::new();
                // Neutral images only. No world or effect label enters the actor.
                observed_phase(
                    &mut branch,
                    &session.frames[3],
                    None,
                    BASE,
                    &mut trace,
                    trace_enabled,
                )?;
                let action_us = branch.model.next_observation_us();
                if weight_mode == "reset_core" {
                    let before_action = branch.model.network().clone();
                    let mut expected = before_action.clone();
                    for edge in initial.synapses() {
                        if branch.model.processing_ids().contains(&edge.pre())
                            && branch.model.processing_ids().contains(&edge.post())
                        {
                            let old = before_action.synapse(edge.id()).unwrap().weight().get();
                            if old != edge.weight().get() {
                                branch
                                    .model
                                    .diagnostic_synapse_weight(edge.id(), edge.weight().get())?;
                                expected
                                    .synapse_mut(edge.id())
                                    .unwrap()
                                    .set_weight(edge.weight())?;
                                edits.push((edge.id(), old, edge.weight().get()));
                            }
                        }
                    }
                    ensure(
                        branch.model.network() == &expected,
                        "action weight reset changed non-weight state",
                    )?;
                    ensure(
                        branch.model.next_observation_us() == action_us,
                        "action weight reset advanced time",
                    )?;
                }
                let mut blocked = Vec::new();
                let mut cancelled = 0;
                if recurrence {
                    for edge in initial.synapses() {
                        if branch.model.processing_ids().contains(&edge.pre())
                            && branch.model.processing_ids().contains(&edge.post())
                        {
                            let exc = initial.neuron(edge.pre()).unwrap().polarity()
                                == nerva::core::Polarity::Excitatory;
                            if gate_mode == "block_both"
                                || (gate_mode == "block_exc" && exc)
                                || (gate_mode == "block_inh" && !exc)
                            {
                                blocked.push(edge.id());
                            }
                        }
                    }
                    let state = branch.model.network().clone();
                    cancelled = branch.model.diagnostic_block_synapses(&blocked)?;
                    let mut expected = state.clone();
                    for id in &blocked {
                        expected.synapse_mut(*id).unwrap().set_enabled(false);
                    }
                    ensure(
                        &expected == branch.model.network(),
                        "recurrence block changed more than enabled flags",
                    )?;
                    ensure(
                        branch.model.next_observation_us() == action_us,
                        "recurrence block advanced time",
                    )?;
                }
                let frozen = branch.model.network().clone();
                let mut cue: Option<Step> = None;
                let mut copies = 0;
                for duration in [10_000, 10_000, 30_000] {
                    let (step, audit) = observed_phase(
                        &mut branch,
                        &session.frames[3],
                        Some(key),
                        duration,
                        &mut trace,
                        trace_enabled,
                    )?;
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
                    learned_parameters_equal(
                        if recurrence { &frozen } else { &after },
                        branch.model.network(),
                    ),
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
                if trace_enabled {
                    trials.last_mut().unwrap()["trace"] = json!(trace);
                }
                if crossed {
                    trials.last_mut().unwrap()["weight_intervention"] = json!({
                        "mode":weight_mode,"activation_us":action_us,"baseline_unmodified":true,
                        "only_core_weights_changed":true,"neurons_and_time_preserved":true,
                        "pending_unblocked_arrivals_keep_emission_amplitudes":true});
                }
                if recurrence {
                    trials.last_mut().unwrap()["recurrence_gate"] = json!({
                        "activation_us":action_us,"blocked_synapses":blocked.iter().map(|id|id.0).collect::<Vec<_>>(),
                        "cancelled_arrivals":cancelled,"only_enabled_flags_changed":true,
                        "weights_and_cells_preserved":true,"baseline_unmodified":true});
                }
            }
            interventions.insert(intervention.to_owned(), json!(trials));
        }
        stages.insert(stage.to_owned(), Value::Object(interventions));
    }
    Ok(json!({"count_readout":calibration.readouts[0].snapshot(),"stages":stages}))
}
