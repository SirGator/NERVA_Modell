//! Training-free electrical replay from a verified contingency record.
use super::{
    CUES, Config, PROTOCOL, Weights, checkpoint, fail, frames, get_weights, probe, serialized,
    stimuli, verify_only_weights,
};
use crate::{Result, error, fingerprint};
use nerva::{core::SynapseId, runtime::ObservationEvent};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

const ARMS: [&str; 4] = ["reset", "forward_core", "reverse_core", "balanced_core"];
type Boundary = (u64, u64, f32, f32, f32, f32, u64, bool);
type Arrival = (u64, u64, u64, f32);

pub(super) struct Trace<'a> {
    targets: &'a BTreeSet<u64>,
    boundaries: Vec<Boundary>,
    arrivals: Vec<Arrival>,
}
impl<'a> Trace<'a> {
    fn new(targets: &'a BTreeSet<u64>) -> Self {
        Self {
            targets,
            boundaries: Vec::new(),
            arrivals: Vec::new(),
        }
    }

    pub(super) fn record(&mut self, events: &[ObservationEvent]) {
        for event in events {
            match *event {
                ObservationEvent::NeuronIntegrated {
                    time,
                    target,
                    potential_before,
                    summed_input,
                    threshold,
                    potential_after,
                    refractory_until,
                    spiked,
                    ..
                } if self.targets.contains(&target.0) => self.boundaries.push((
                    time.0,
                    target.0,
                    potential_before,
                    summed_input,
                    threshold,
                    potential_after,
                    refractory_until.0,
                    spiked,
                )),
                ObservationEvent::SynapticArrival {
                    time,
                    target,
                    synapse_id,
                    amplitude,
                    ..
                } if self.targets.contains(&target.0) => {
                    self.arrivals
                        .push((time.0, target.0, synapse_id.0, amplitude));
                }
                _ => {}
            }
        }
    }
}

fn unsigned(value: &Value, name: &str) -> Result<u64> {
    value[name]
        .as_u64()
        .ok_or_else(|| error(format!("invalid archived {name}")))
}

fn trace_recall(record: &Value) -> Result<Value> {
    let details = &record["details"];
    let config = &record["job"]["effective_config"];
    let c = Config {
        processing_neurons: usize::try_from(unsigned(config, "processing_neurons")?)
            .map_err(fail)?,
        training_repetitions: usize::try_from(unsigned(config, "training_repetitions")?)
            .map_err(fail)?,
        rest_us: unsigned(config, "rest_us")?,
    };
    super::Scenario::validate(&super::ContingencyScenario, &c)?;
    let seed = unsigned(&record["job"], "seed")?;
    let gap = unsigned(details, "gap_frames")?;
    if !matches!(gap, 0 | 2) || unsigned(config, "gap_frames")? != gap {
        return Err(error("inconsistent archived gap"));
    }
    let patterns = stimuli();
    if details["stimuli"] != json!(patterns) {
        return Err(error("archived stimuli differ from native stimuli"));
    }
    let frames = frames(&patterns)?;
    let (reference, checkpoint_details) = checkpoint(&c, seed)?;
    let initial = get_weights(&reference);
    let edges = reference
        .network()
        .synapses()
        .map(|s| (s.id().0, s.pre().0, s.post().0, s.weight().get()))
        .collect::<Vec<_>>();
    let core = reference
        .processing_ids()
        .iter()
        .map(|n| n.0)
        .collect::<BTreeSet<_>>();
    if details["development"] != checkpoint_details
        || details["edges"] != json!(edges)
        || details["core_ids"] != json!(core)
    {
        return Err(error("archived contingency development checkpoint differs"));
    }
    let templates: [BTreeSet<u64>; 2] =
        serde_json::from_value(details["templates"].clone()).map_err(fail)?;
    let targets = templates[0]
        .union(&templates[1])
        .copied()
        .collect::<BTreeSet<_>>();
    if !targets.is_subset(&core) || !templates[0].is_disjoint(&templates[1]) {
        return Err(error("invalid archived contingency target groups"));
    }
    let mut arms = BTreeMap::new();
    for arm in ARMS {
        let archived = &details["probes"][arm];
        let values: Vec<(u64, f32)> =
            serde_json::from_value(archived["weights"].clone()).map_err(fail)?;
        let weights: Weights = values.into_iter().map(|(i, w)| (SynapseId(i), w)).collect();
        if weights.len() != initial.len()
            || edges.iter().zip(&weights).any(|(edge, &(id, value))| {
                edge.0 != id.0
                    || ((arm == "reset" || !core.contains(&edge.1) || !core.contains(&edge.2))
                        && edge.3 != value)
            })
        {
            return Err(error(
                "archived weight vector exceeds contingency intervention scope",
            ));
        }
        let mut cues = BTreeMap::new();
        for cue in CUES {
            let (mut branch, _) = checkpoint(&c, seed)?;
            verify_only_weights(&reference, &mut branch, &weights)?;
            if get_weights(&branch) != weights {
                return Err(error(
                    "electrical replay installed a different weight vector",
                ));
            }
            let mut trace = Trace::new(&targets);
            let mut response = probe(
                &mut branch,
                &c,
                &frames[cue],
                &frames["blank"],
                Some(&mut trace),
            )?
            .json();
            if response != archived["responses"][cue] {
                return Err(error(format!(
                    "{arm}/{cue} electrical observation differs from archived recall"
                )));
            }
            response["boundaries"] = json!(trace.boundaries);
            response["arrivals"] = json!(trace.arrivals);
            cues.insert(cue, response);
        }
        arms.insert(arm, json!({"weights":serialized(&weights),"cues":cues}));
    }
    Ok(
        json!({"protocol":"NERVA-Contingency-Electrical/1","seed":seed,"gap_frames":gap,
        "checkpoint":checkpoint_details,"templates":templates,"edges":edges,"core_ids":core,"arms":arms}),
    )
}

/// Revisit only the two original cues on frozen checkpoint copies. No training is rerun.
pub fn replay_electrical(envelope: &Value) -> Result<Value> {
    let record = &envelope["record"];
    let checksum = fingerprint(&serde_json::to_vec(record).map_err(fail)?);
    if record["details"]["protocol"] != PROTOCOL
        || record["full_replay_checked"] != true
        || envelope["checksum_fnv1a64"] != checksum
    {
        return Err(error("intact replay-verified contingency record required"));
    }
    let mut observed = trace_recall(record)?;
    if observed != trace_recall(record)? {
        return Err(error("contingency electrical replay differs"));
    }
    observed["archived_record_checksum"] = json!(checksum);
    observed["replay_verified"] = json!(true);
    Ok(observed)
}
