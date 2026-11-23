//! Frozen phase-B readout with independent weight and transmission interventions.
use super::{
    Assay, CUES_B, Config, ContingencyScenario, LiveRateScenario, MAPPINGS, Model, PARAMETERS,
    checkpoint, fail, frames, patterns, topology,
};
use crate::scenarios::hippocampal::contingency::{
    learned_parameters_equal, observe_probe, wait_probe_rest,
};
use crate::{Result, error, fingerprint, scenario::Intervention, scenario::Scenario};
use nerva::core::SynapseId;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

const PROTOCOL: &str = "NERVA-Contingency-B-Factor/1";
const PROBES: [&str; 3] = ["blank", CUES_B[0], CUES_B[1]];
type WeightUpdates = Vec<(u64, f32, f32)>;

struct Plans {
    weights: WeightUpdates,
    new: Vec<u64>,
    terminal: Vec<u64>,
    nonterminal: Vec<u64>,
}

impl Plans {
    fn new(after_a: &BTreeMap<u64, (f32, bool)>, after_b: &Model, targets: &BTreeSet<u64>) -> Self {
        let mut plans = Self {
            weights: Vec::new(),
            new: Vec::new(),
            terminal: Vec::new(),
            nonterminal: Vec::new(),
        };
        for edge in after_b
            .network()
            .synapses()
            .filter(|edge| edge.is_enabled())
        {
            let id = edge.id().0;
            if let Some(&(old, enabled)) = after_a.get(&id) {
                let current = edge.weight().get();
                if enabled && old != current {
                    plans.weights.push((id, current, old));
                }
            } else {
                plans.new.push(id);
                if targets.contains(&edge.post().0) {
                    plans.terminal.push(id);
                } else {
                    plans.nonterminal.push(id);
                }
            }
        }
        plans
    }

    fn conditions(&self) -> [(&'static str, Patch<'_>); 6] {
        [
            ("intact", Patch::new(&[], &[])),
            ("restore_old", Patch::new(&self.weights, &[])),
            ("block_new", Patch::new(&[], &self.new)),
            ("restore_and_block", Patch::new(&self.weights, &self.new)),
            ("block_terminal", Patch::new(&[], &self.terminal)),
            ("block_nonterminal", Patch::new(&[], &self.nonterminal)),
        ]
    }
}

struct Patch<'a> {
    weights: &'a [(u64, f32, f32)],
    blocked: &'a [u64],
}

impl<'a> Patch<'a> {
    fn new(weights: &'a [(u64, f32, f32)], blocked: &'a [u64]) -> Self {
        Self { weights, blocked }
    }

    fn expected_topology(&self, base: &Value) -> Value {
        let weights: BTreeMap<_, _> = self.weights.iter().map(|&(id, _, old)| (id, old)).collect();
        let mut published = base.clone();
        for row in published.as_array_mut().expect("replayed topology rows") {
            let id = row[0].as_u64().expect("replayed synapse ID");
            if let Some(&weight) = weights.get(&id) {
                row[3] = json!(weight);
            }
            if self.blocked.binary_search(&id).is_ok() {
                row[5] = json!(false);
            }
        }
        published
    }

    fn apply(&self, model: &mut Model) -> Result<usize> {
        for &(id, _, old) in self.weights {
            model
                .diagnostic_synapse_weight(SynapseId(id), old)
                .map_err(fail)?;
        }
        if self.blocked.is_empty() {
            Ok(0)
        } else {
            let ids: Vec<_> = self.blocked.iter().copied().map(SynapseId).collect();
            model.diagnostic_block_synapses(&ids).map_err(fail)
        }
    }
}

fn recorded_config(record: &Value) -> Result<(Config, u64)> {
    let job = &record["job"];
    let cfg = &job["effective_config"];
    let seed = job["seed"]
        .as_u64()
        .ok_or_else(|| error("B factor archive seed missing"))?;
    let mut config = ContingencyScenario.default_config();
    for key in PARAMETERS {
        ContingencyScenario.set_parameter(&mut config, key, &cfg[key])?;
    }
    ContingencyScenario.validate(&config)?;
    let intervention = Intervention {
        name: "paired_rate_rules".into(),
    };
    if job["intervention"] != intervention.name
        || cfg != &LiveRateScenario.snapshot(&config, seed, &intervention)
    {
        return Err(error(
            "B factor archive configuration differs from native A/B assay",
        ));
    }
    Ok((config, seed))
}

fn verified_training(
    assay: &Assay<'_>,
    model: &mut Model,
    mapping: &str,
    task: usize,
    archive: &Value,
) -> Result<()> {
    if assay.train(model, mapping, task, true)? != archive["frames"]
        || topology(model) != archive["end_topology"]
    {
        return Err(error(format!(
            "B factor training replay differs for {mapping}, stage {task}"
        )));
    }
    Ok(())
}

struct Readout<'a> {
    assay: &'a Assay<'a>,
    parent: &'a Model,
    mapping: &'a str,
    archive: &'a Value,
}

impl Readout<'_> {
    fn measure(&self, name: &str, patch: Patch<'_>) -> Result<Value> {
        let mut probes = BTreeMap::new();
        let mut cancelled = BTreeMap::new();
        let published = patch.expected_topology(&self.archive["end_topology"]);
        let blank = &self.assay.images["blank"];
        for cue in PROBES {
            let mut fork = self.assay.start_training("native")?;
            self.assay.train(&mut fork, self.mapping, 0, false)?;
            self.assay.train(&mut fork, self.mapping, 1, false)?;
            if !self.parent.same_replay_state(&fork) {
                return Err(error("B factor probe fork differs before freeze"));
            }
            fork.freeze().map_err(fail)?;
            let rest = wait_probe_rest(&mut fork, self.assay.config.rest_us, blank)?;
            if topology(&fork) != self.archive["end_topology"] {
                return Err(error(
                    "B factor pre-intervention rest changed topology or weights",
                ));
            }
            let cancel_count = patch.apply(&mut fork)?;
            if topology(&fork) != published {
                return Err(error(
                    "B factor published topology differs from planned interventions",
                ));
            }
            let before = fork.network().clone();
            let response = observe_probe(&mut fork, &self.assay.images[cue], blank, rest, None)?;
            if !learned_parameters_equal(&before, fork.network())
                || patch
                    .blocked
                    .iter()
                    .any(|id| response.arrived_edges.contains(id))
            {
                return Err(error(
                    "B factor readout changed frozen parameters or leaked blocked arrivals",
                ));
            }
            let observed = response.json();
            let baseline = &self.archive["probes"][cue];
            if ["start_us", "rest_core_spikes", "root_spikes"]
                .into_iter()
                .any(|key| observed[key] != baseline[key])
                || (name == "intact" && &observed != baseline)
            {
                return Err(error(format!(
                    "B factor probe replay differs for {}/{name}/{cue}",
                    self.mapping
                )));
            }
            probes.insert(cue, observed);
            cancelled.insert(cue, cancel_count);
        }
        Ok(
            json!({"weight_updates":patch.weights,"blocked_edges":patch.blocked,
            "published_topology":published,"probes":probes,"cancelled_arrivals":cancelled}),
        )
    }
}

fn run(record: &Value, checksum: &str) -> Result<Value> {
    let (config, seed) = recorded_config(record)?;
    let archive = &record["details"];
    let reference = checkpoint(&config, seed)?.0;
    if archive["seed"] != seed
        || archive["stimuli"] != json!(patterns())
        || archive["initial_topology"] != topology(&reference)
        || archive["checkpoint_time_us"] != reference.next_observation_us()
        || archive["core_ids"]
            != json!(
                reference
                    .processing_ids()
                    .iter()
                    .map(|id| id.0)
                    .collect::<Vec<_>>()
            )
    {
        return Err(error("B factor developed checkpoint differs from archive"));
    }
    let targets: [Vec<u64>; 2] =
        serde_json::from_value(archive["targets"]["b"].clone()).map_err(fail)?;
    let targets = targets.into_iter().flatten().collect();
    let assay = Assay {
        config: &config,
        seed,
        reference,
        images: frames(&patterns())?,
    };
    let mut mappings = BTreeMap::new();
    let mut probe_start_us = None;
    for mapping in MAPPINGS {
        let archived = &archive["branches"]["native"][mapping];
        let mut parent = assay.start_training("native")?;
        verified_training(&assay, &mut parent, mapping, 0, &archived["a"])?;
        let before_b = parent
            .network()
            .synapses()
            .map(|edge| (edge.id().0, (edge.weight().get(), edge.is_enabled())))
            .collect();
        verified_training(&assay, &mut parent, mapping, 1, &archived["b"])?;
        let start = parent.next_observation_us() + config.rest_us;
        if probe_start_us
            .replace(start)
            .is_some_and(|previous| previous != start)
        {
            return Err(error("B factor probe clock differs across mappings"));
        }
        let plans = Plans::new(&before_b, &parent, &targets);
        let readout = Readout {
            assay: &assay,
            parent: &parent,
            mapping,
            archive: &archived["b"],
        };
        let conditions: BTreeMap<_, _> = plans
            .conditions()
            .into_iter()
            .map(|(name, patch)| Ok((name, readout.measure(name, patch)?)))
            .collect::<Result<_>>()?;
        mappings.insert(mapping, json!({"conditions":conditions}));
    }
    Ok(
        json!({"protocol":PROTOCOL,"seed":seed,"archived_record_checksum":checksum,
        "probe_start_us":probe_start_us,"mappings":mappings}),
    )
}

/// Reconstruct native post-B arms and repeat the complete frozen causal readout.
pub fn replay_b_factor(envelope: &Value) -> Result<Value> {
    let record = &envelope["record"];
    let checksum = fingerprint(&serde_json::to_vec(record).map_err(fail)?);
    if record["details"]["protocol"] != super::PROTOCOL
        || record["full_replay_checked"] != true
        || envelope["checksum_fnv1a64"] != checksum
    {
        return Err(error(
            "B factor requires an intact replay-verified live-rate record",
        ));
    }
    let mut result = run(record, &checksum)?;
    if result != run(record, &checksum)? {
        return Err(error("B factor full counterfactual replay differs"));
    }
    result["full_replay_checked"] = json!(true);
    Ok(result)
}
