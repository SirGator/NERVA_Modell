use super::super::{Config, association, causal::verify_only_weights};
use super::atlas::{get_weights, margins, probe, response};
use super::{METRICS, images, rule};
use crate::{Result, error, fingerprint_debug, scenario::TrialMetrics};
use nerva::{learning::CompetitionEvent, runtime::ObservationEvent};
use nerva_vm::model::{
    Model, PlasticitySettings, SensorConfig, SensorMode, SpikeCoding, audit::AuditOptions,
};
use serde_json::{Value, json};
use std::collections::BTreeMap;

fn checkpoint(c: &Config, seed: u64) -> Result<(Model, Value)> {
    let mut cfg = super::super::model_config(c, seed, "smooth");
    cfg.development = true;
    cfg.homeostasis = true;
    let mut model = Model::with_sensor_and_learning(
        cfg,
        SensorConfig {
            mode: SensorMode::Pixels,
            resolution: 8,
            coding: SpikeCoding::Deterministic,
            max_rate_hz: 120.,
        },
        rule("B"),
    )
    .map_err(super::super::fail)?;
    let initial = fingerprint_debug(model.network());
    let mut formed = 0;
    let mut pruned = 0;
    for i in 0..20 {
        let kind = [0, 1, 4, 4][i % 4];
        let step = model
            .step(&images::distractor(kind).map_err(super::super::fail)?)
            .map_err(super::super::fail)?;
        formed += step.formed.len();
        pruned += step.pruned.len();
    }
    model.freeze().map_err(super::super::fail)?;
    if formed + pruned == 0 {
        return Err(error(
            "MLD02 development phase produced no topology changes",
        ));
    }
    let details = json!({"initial_network_digest":initial,"developed_network_digest":fingerprint_debug(model.network()),"formed":formed,"pruned":pruned,
        "synapses":model.network().synapse_count(),"checkpoint_time_us":model.next_observation_us(),"development_frozen_after_checkpoint":true});
    Ok((model, details))
}
#[derive(Default)]
struct EdgeLedger {
    total: f64,
    competition: f64,
    local: f64,
    changes: usize,
    normalizations: usize,
}
fn training(
    model: &mut Model,
    c: &Config,
    seed: u64,
    h: bool,
    k: bool,
    record: bool,
) -> Result<Value> {
    if k {
        model
            .install_checkpoint_competition()
            .map_err(super::super::fail)?;
    }
    let pools=model.competition_pools().iter().map(|p|json!({"target":p.target.0,"synapses":p.synapses.iter().map(|i|i.0).collect::<Vec<_>>(),"budget":p.budget})).collect::<Vec<_>>();
    model
        .set_plasticity(PlasticitySettings {
            learning: true,
            development: false,
            homeostasis: h,
        })
        .map_err(super::super::fail)?;
    if record {
        model.enable_audit(AuditOptions {
            raw_events: true,
            input_spikes: false,
        });
    }
    let identities = model
        .network()
        .synapses()
        .map(|s| (s.id(), s.pre(), s.post()))
        .collect::<Vec<_>>();
    let before = get_weights(model);
    let mut edges = BTreeMap::<u64, EdgeLedger>::new();
    let mut homeostasis = BTreeMap::<u64, (usize, f64, f64)>::new();
    let mut windows = Vec::new();
    let mut norm_examples = Vec::new();
    let mut homeo_examples = Vec::new();
    let mut observed = 0usize;
    let mut raw_weight_changes = 0;
    let mut normalizations = 0;
    for (index, image) in association::schedule(c, seed, false)
        .into_iter()
        .enumerate()
    {
        let step = model
            .step(&images::probe(image).map_err(super::super::fail)?)
            .map_err(super::super::fail)?;
        if !step.formed.is_empty() || !step.pruned.is_empty() {
            return Err(error("interaction arm changed common topology"));
        }
        let events = model.take_competition_events();
        if record {
            let audit = model
                .take_audit()
                .ok_or_else(|| error("missing interaction audit"))?;
            observed += audit.raw_events.len() + events.len();
            if observed > 4_000_000 || audit.raw_events.len() + events.len() > 200_000 {
                return Err(error("interaction audit budget exceeded"));
            }
            for event in &audit.raw_events {
                match *event {
                    ObservationEvent::WeightChanged {
                        synapse_id,
                        old_weight,
                        new_weight,
                        ..
                    } => {
                        let e = edges.entry(synapse_id.0).or_default();
                        e.total += f64::from(new_weight.get()) - f64::from(old_weight.get());
                        e.changes += 1;
                        raw_weight_changes += 1;
                    }
                    ObservationEvent::HomeostasisChanged {
                        time,
                        neuron_id,
                        old_intrinsic_current,
                        new_intrinsic_current,
                        old_structural_drive,
                        new_structural_drive,
                        ..
                    } => {
                        let e = homeostasis.entry(neuron_id.0).or_default();
                        e.0 += 1;
                        e.1 += f64::from(new_intrinsic_current) - f64::from(old_intrinsic_current);
                        e.2 += f64::from(new_structural_drive) - f64::from(old_structural_drive);
                        if homeo_examples.len() < 64 {
                            homeo_examples.push(json!({"time_us":time.0,"neuron_id":neuron_id.0,"old_current":old_intrinsic_current,"new_current":new_intrinsic_current,"old_drive":old_structural_drive,"new_drive":new_structural_drive}));
                        }
                    }
                    _ => {}
                }
            }
            for event in events {
                match event {
                    CompetitionEvent::LocalChange {
                        synapse,
                        before,
                        after,
                        ..
                    } => {
                        edges.entry(synapse.0).or_default().local +=
                            f64::from(after) - f64::from(before);
                    }
                    CompetitionEvent::Normalized {
                        time,
                        pool,
                        before,
                        after,
                        residual,
                    } => {
                        normalizations += 1;
                        for ((id, a), b) in pool.synapses.iter().zip(&before).zip(&after) {
                            let e = edges.entry(id.0).or_default();
                            e.competition += f64::from(*b) - f64::from(*a);
                            e.normalizations += 1;
                        }
                        if norm_examples.len() < 64 {
                            norm_examples.push(json!({"time_us":time.0,"target":pool.target.0,"budget":pool.budget,"synapses":pool.synapses.iter().map(|i|i.0).collect::<Vec<_>>(),"before":before,"after":after,"residual":residual}));
                        }
                    }
                }
            }
            windows.push(json!({"index":index,"start_us":step.start_us,"end_us":step.end_us,"native_weight_changes":step.weight_changes,"homeostasis_changes":audit.homeostasis_updates,"core_spikes":step.core_counts.iter().sum::<usize>()}));
        }
    }
    let after = get_weights(model);
    if identities
        != model
            .network()
            .synapses()
            .map(|s| (s.id(), s.pre(), s.post()))
            .collect::<Vec<_>>()
    {
        return Err(error("MLD02 common structure changed"));
    }
    let mut max_error = 0f64;
    if record {
        for ((id, a), (_, b)) in before.iter().zip(&after) {
            let observed = edges.get(&id.0).map_or(0., |e| e.total);
            max_error = max_error.max((observed - (f64::from(*b) - f64::from(*a))).abs());
        }
        if max_error > 1e-12 {
            return Err(error("interaction native weight ledger does not balance"));
        }
    }
    model.freeze().map_err(super::super::fail)?;
    Ok(
        json!({"homeostasis":h,"competition":k,"topology_preserved":true,"pool_membership_and_budgets":pools,"observed_rows":observed,
        "native_weight_changes":raw_weight_changes,"normalizations":normalizations,"mass_balance_error":max_error,"windows":windows,
        "weights_before":before.iter().map(|(i,w)|(i.0,*w)).collect::<Vec<_>>(),"weights_after":after.iter().map(|(i,w)|(i.0,*w)).collect::<Vec<_>>(),
        "synapse_ledger":edges.into_iter().map(|(i,e)|json!({"synapse_id":i,"total_delta":e.total,"competition_delta":e.competition,
            "local_learning_delta":e.total-e.competition,"pooled_local_change_audit":e.local,"events":e.changes,"normalizations":e.normalizations})).collect::<Vec<_>>(),
        "homeostasis_by_neuron":homeostasis.into_iter().map(|(i,(n,current,drive))|json!({"neuron_id":i,"changes":n,"intrinsic_delta":current,"structural_drive_delta":drive})).collect::<Vec<_>>(),
        "normalization_examples":norm_examples,"homeostasis_examples":homeo_examples,"example_limit_per_kind":64,
        "examples_scope":"First 64 only; complete per-synapse and per-neuron aggregates cover all observed changes."}),
    )
}
pub(super) fn run(c: &Config, seed: u64) -> Result<TrialMetrics> {
    let (shared, development) = checkpoint(c, seed)?;
    let (mut before, _) = checkpoint(c, seed)?;
    if !shared.same_replay_state(&before) {
        return Err(error("developed baseline differs"));
    }
    let baseline = probe(&mut before, c)?;
    let targets = vec![baseline.counts[1].clone(), baseline.counts[2].clone()];
    let initial = get_weights(&shared);
    let mut arms = BTreeMap::new();
    for (h, k) in [(false, false), (false, true), (true, false), (true, true)] {
        let (mut learned, _) = checkpoint(c, seed)?;
        let (mut reset, _) = checkpoint(c, seed)?;
        if !shared.same_replay_state(&learned) || !shared.same_replay_state(&reset) {
            return Err(error("2x2 arms do not share the full developed checkpoint"));
        }
        let observed = training(&mut learned, c, seed, h, k, true)?;
        training(&mut reset, c, seed, h, k, false)?;
        if !learned.same_replay_state(&reset) {
            return Err(error("interaction observation changed replay state"));
        }
        // Same mechanism/controller state; only the post-checkpoint weights are reset.
        let reference = association::Training {
            weights: initial.clone(),
            model: learned,
            initial_digest: String::new(),
            details: Value::Null,
        };
        verify_only_weights(&reference.model, &mut reset, &initial)?;
        let without = probe(&mut reset, c)?;
        let mut model = reference.model;
        let with = probe(&mut model, c)?;
        let margin = margins(&with, &targets);
        let own = response(with, &without, &targets);
        arms.insert(format!("H{}_K{}",usize::from(h),usize::from(k)),json!({"training":observed,"margins":margin,"own_weight_effect":own,"reset_probe":without.details,"common_checkpoint_verified":true}));
    }
    let contrast = (0..6)
        .map(|i| {
            arms["H1_K1"]["margins"][i].as_f64().unwrap()
                - arms["H1_K0"]["margins"][i].as_f64().unwrap()
                - arms["H0_K1"]["margins"][i].as_f64().unwrap()
                + arms["H0_K0"]["margins"][i].as_f64().unwrap()
        })
        .collect::<Vec<_>>();
    let values = [0., 0., 0., 18., 8., contrast[0], contrast[1], 0.];
    Ok(TrialMetrics {
        values: METRICS.into_iter().map(str::to_owned).zip(values).collect(),
        details: json!({"protocol":"NERVA-MLD-02/1","section":"interaction","development":development,
        "shared_checkpoint_verified":true,"baseline_probe":baseline.details,"fixed_targets":targets,"arms":arms,"interaction":contrast,
        "definition":"F(H=1,K=1)-F(1,0)-F(0,1)+F(0,0) for fixed-target completion margins; first two original cues, last four novel cues.",
        "limits":"Topology is frozen after common native development; homeostasis may change cellular state. Competition captures budgets at that same checkpoint. This 2x2 measures total mechanism interaction, not isolated weights at one post-training state. Nonzero per-seed contrasts are deterministic, not a population significance test."}),
    })
}
