//! Reciprocal source edits on the archived B responder and its crossed controls.
use super::super::{
    CUES_B,
    lead::{LeadGrowthScenario, POLICIES, training_audit},
};
use super::{
    Assay, Birth, Carrier, Config, MAPPINGS, Model, Patch, Position, Trace, TrainingAudit,
    checkpoint, distance, exact_fork, fail, fingerprint, frames, topology,
};
use crate::{
    Result, error,
    scenario::{Intervention, Scenario},
};
use nerva::core::{NeuronId, Polarity, SynapseId};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

const PROTOCOL: &str = "NERVA-Contingency-B-Source/1";
const SOURCE_CHECKSUM: &str = "363a851d2d379cd6";
const SEED: u64 = 34531;
const TARGET: NeuronId = NeuronId(93);
const CUE: &str = CUES_B[0];
const SOURCES: [NeuronId; 2] = [NeuronId(87), NeuronId(100)];
const HISTORIES: [&str; 2] = ["balanced", "forward"];
const PROBES: [&str; 3] = ["blank", CUE, CUES_B[1]];

fn verify_stage(
    assay: &Assay<'_>,
    model: &mut Model,
    mapping: &str,
    task: usize,
    audit: TrainingAudit,
    old: &Value,
) -> Result<()> {
    if assay.train(model, mapping, task, audit)? != old["frames"]
        || topology(model) != old["end_topology"]
    {
        return Err(error(format!(
            "B source stage {task}/{mapping} differs from archive"
        )));
    }
    Ok(())
}

fn original_edge(model: &Model, stage: &Value, mapping: &str) -> Result<(Birth, bool)> {
    let mut born = vec![];
    for frame in stage["frames"]
        .as_array()
        .ok_or_else(|| error("missing B source frames"))?
    {
        let formed: Vec<(u64, u64, u64)> =
            serde_json::from_value(frame["formed"].clone()).map_err(fail)?;
        for (id, _, _) in formed
            .into_iter()
            .filter(|&(_, _, target)| target == TARGET.0)
        {
            let exploratory = frame["exploratory"]
                .as_array()
                .ok_or_else(|| error("missing B source exploration ledger"))?
                .iter()
                .any(|row| row[0].as_u64() == Some(id));
            born.push((id, exploratory));
        }
    }
    let [(id, exploratory)] = born.as_slice() else {
        return Err(error(
            "B source expected one B-born edge into the responder",
        ));
    };
    let edge = model
        .network()
        .synapse(SynapseId(*id))
        .ok_or_else(|| error("missing B source edge"))?;
    if !edge.is_enabled()
        || !SOURCES.contains(&edge.pre())
        || *exploratory != (mapping == "forward")
    {
        return Err(error("B source found an unexpected B-born edge"));
    }
    Ok((
        Birth {
            id: *id,
            pre: edge.pre(),
            post: TARGET,
            weight: edge.weight().get(),
            delay: edge.delay_us(),
        },
        *exploratory,
    ))
}

fn sham_source(
    model: &Model,
    geometry: &BTreeMap<NeuronId, Position>,
    radius: f32,
    swap: NeuronId,
    archived: &Value,
) -> Result<(NeuronId, bool)> {
    let network = model.network();
    let occupied: BTreeSet<_> = network
        .incoming_synapse_ids(TARGET)
        .iter()
        .map(|&id| network.synapse(id).expect("incoming edge").pre())
        .collect();
    let cue_spikes: Vec<(u64, u64)> =
        serde_json::from_value(archived[CUE]["core_spikes"].clone()).map_err(fail)?;
    let active: BTreeSet<_> = cue_spikes.into_iter().map(|(id, _)| NeuronId(id)).collect();
    let desired = distance(geometry, swap, TARGET);
    geometry
        .iter()
        .filter(|&(&id, (_, excitatory))| {
            *excitatory
                && id != TARGET
                && id != swap
                && !occupied.contains(&id)
                && distance(geometry, id, TARGET) <= f64::from(radius)
        })
        .map(|(&id, _)| {
            (
                id,
                active.contains(&id),
                (distance(geometry, id, TARGET) - desired).abs(),
            )
        })
        .min_by(|a, b| b.1.cmp(&a.1).then(a.2.total_cmp(&b.2)).then(a.0.cmp(&b.0)))
        .map(|(id, active, _)| (id, active))
        .ok_or_else(|| error("B source has no local sham candidate"))
}

fn measure(
    assay: &Assay<'_>,
    model: &Model,
    archive: &Value,
    mapping: &str,
    geometry: &BTreeMap<NeuronId, Position>,
    radius: f32,
) -> Result<Value> {
    let (edge, exploratory) = original_edge(model, archive, mapping)?;
    let swap = if edge.pre == SOURCES[0] {
        SOURCES[1]
    } else {
        SOURCES[0]
    };
    if !geometry[&edge.pre].1 || !geometry[&swap].1 {
        return Err(error("B source swap must retain an excitatory source"));
    }
    let old = &archive["probes"];
    let (sham, sham_active) = sham_source(model, geometry, radius, swap, old)?;
    let new_id = model
        .network()
        .synapses()
        .map(|e| e.id().0)
        .max()
        .and_then(|id| id.checked_add(1))
        .ok_or_else(|| error("invalid B source synapse IDs"))?;
    let carrier = Carrier::rest(assay, model)?;
    let targets = BTreeSet::from([TARGET.0]);
    let mut arms = BTreeMap::new();
    for (arm, blocked, replacement) in [
        ("intact", false, None),
        ("block", true, None),
        ("swap", true, Some(swap)),
        ("sham", true, Some(sham)),
    ] {
        let blocked: Vec<_> = blocked.then_some(edge.id).into_iter().collect();
        let inserted: Vec<_> = replacement
            .map(|pre| Birth {
                id: new_id,
                pre,
                ..edge
            })
            .into_iter()
            .collect();
        let mut fork = exact_fork(&carrier.rested)?;
        let (installed, cancelled) = Patch {
            blocked: &blocked,
            inserted: &inserted,
            ..Patch::default()
        }
        .publish(&mut fork)?;
        let mut probes = BTreeMap::new();
        for cue in PROBES {
            let mut trace = Trace::new(&targets);
            let mut probe =
                carrier.read_with_trace(assay, &fork, cue, &old[cue], Some(&mut trace))?;
            if (arm == "intact" && probe != old[cue])
                || blocked.iter().any(|&id| {
                    probe["arrived_edges"]
                        .as_array()
                        .expect("arrival IDs")
                        .contains(&json!(id))
                })
            {
                return Err(error(format!(
                    "B source replay or blocked arrival differs for {arm}/{cue}"
                )));
            }
            trace.attach(&mut probe);
            probes.insert(cue, probe);
        }
        arms.insert(
            arm,
            json!({"published_topology":installed,"blocked_edges":blocked,
            "inserted_edges":inserted.iter().map(|birth| birth.row()).collect::<Vec<_>>(),
            "cancelled_arrivals":cancelled,"probes":probes}),
        );
    }
    Ok(
        json!({"original_edge":edge.row(),"original_exploratory":exploratory,"swap_source":swap.0,"sham_source":sham.0,
        "sham_cue_active":sham_active,"arms":arms}),
    )
}

pub fn replay_b_source(envelope: &Value) -> Result<Value> {
    let record = &envelope["record"];
    let checksum = fingerprint(&serde_json::to_vec(record).map_err(fail)?);
    let archive = &record["details"];
    let job = &record["job"];
    let study = LeadGrowthScenario::Contrast;
    let intervention = Intervention {
        name: study.intervention().into(),
    };
    let config = Config {
        processing_neurons: 256,
        training_repetitions: 8,
        rest_us: 11_000_000,
    };
    let (layout, stimuli) = study.layout(SEED);
    if checksum != SOURCE_CHECKSUM
        || envelope["checksum_fnv1a64"] != checksum
        || record["full_replay_checked"] != true
        || job["seed"] != SEED
        || job["intervention"] != intervention.name
        || job["effective_config"] != study.snapshot(&config, SEED, &intervention)
        || archive["protocol"] != study.protocol()
        || archive["seed"] != SEED
        || archive["stimuli"] != json!(stimuli)
        || archive["layout"] != layout
    {
        return Err(error(
            "B source requires the verified contrast-growth seed 34531",
        ));
    }
    let reference = checkpoint(&config, SEED)?.0;
    let (radius, _, _) = reference.diagnostic_formation_geometry();
    let geometry: BTreeMap<_, _> = reference
        .processing_ids()
        .iter()
        .map(|&id| {
            let cell = reference.network().neuron(id).expect("Core neuron");
            let p = cell.position();
            (
                id,
                (
                    [p.x, p.y, p.z].map(f64::from),
                    cell.polarity() == Polarity::Excitatory,
                ),
            )
        })
        .collect();
    let positions: Vec<_> = geometry
        .iter()
        .map(|(id, (p, excitatory))| (id.0, p, excitatory))
        .collect();
    if archive["checkpoint_time_us"] != reference.next_observation_us()
        || archive["initial_topology"] != topology(&reference)
        || archive["core_ids"] != json!(geometry.keys().map(|id| id.0).collect::<Vec<_>>())
        || !archive["targets"]["b"][0]
            .as_array()
            .is_some_and(|ids| ids.contains(&json!(TARGET.0)))
    {
        return Err(error(
            "B source developed checkpoint or target differs from archive",
        ));
    }
    let assay = Assay {
        config: &config,
        seed: SEED,
        reference,
        images: frames(&stimuli)?,
    };
    let start = assay.start_training("native")?;
    let mut branches = BTreeMap::new();
    for (policy, mode) in POLICIES {
        let old = &archive["trials"][policy];
        let mut policy_start = exact_fork(&start)?;
        policy_start
            .diagnostic_switch_exploratory_formation(mode)
            .map_err(fail)?;
        let audit = training_audit(mode);
        let mut histories = BTreeMap::new();
        for a in HISTORIES {
            let mut after_a = exact_fork(&policy_start)?;
            verify_stage(&assay, &mut after_a, a, 0, audit, &old["a"][a])?;
            let mut mappings = BTreeMap::new();
            for b in MAPPINGS {
                let mut after_b = exact_fork(&after_a)?;
                let stage = &old["b"][a][b];
                verify_stage(&assay, &mut after_b, b, 1, audit, stage)?;
                mappings.insert(b, measure(&assay, &after_b, stage, b, &geometry, radius)?);
            }
            histories.insert(a, mappings);
        }
        branches.insert(policy, histories);
    }
    Ok(
        json!({"protocol":PROTOCOL,"archived_record_checksum":checksum,"seed":SEED,"target_cell":TARGET.0,
        "cue":CUE,"formation_radius":f64::from(radius),"positions":positions,"branches":branches}),
    )
}
