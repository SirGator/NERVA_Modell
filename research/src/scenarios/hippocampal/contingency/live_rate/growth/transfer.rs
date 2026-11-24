//! Assay A-grown connectivity on a common frozen developed state.
use super::{
    Assay, Config, ContingencyScenario, INTERVENTION, LiveGrowthScenario, MAPPINGS, Model,
    TrainingAudit, checkpoint, exact_fork, fail, frames, patterns, topology,
};
use crate::scenarios::hippocampal::contingency::{
    CUES, electrical::Trace, learned_parameters_equal, observe_probe, wait_probe_rest,
};
use crate::{
    Result, error, fingerprint,
    scenario::{Intervention, Scenario},
};
use nerva::core::{NeuronId, Polarity, SynapseId};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

mod b_source;
mod sources;
pub use b_source::replay_b_source;
pub use sources::{replay_a_source, replay_a_timing};

const PROTOCOL: &str = "NERVA-Contingency-A-Transfer/1";
const PROBES: [&str; 3] = ["blank", CUES[0], CUES[1]];
const DISTANCE_CHANGE: f64 = 0.025;
type WeightUpdate = (u64, f32, f32);
type Position = ([f64; 3], bool);

#[derive(Clone, Copy)]
struct Birth {
    id: u64,
    pre: NeuronId,
    post: NeuronId,
    weight: f32,
    delay: u64,
}

impl Birth {
    fn endpoints(self) -> (NeuronId, NeuronId) {
        (self.pre, self.post)
    }

    fn row(self) -> Value {
        json!([
            self.id,
            self.pre.0,
            self.post.0,
            self.weight,
            self.delay,
            true
        ])
    }
}

struct Components {
    born: Vec<Birth>,
    learned_old: Vec<WeightUpdate>,
    restore_old: Vec<WeightUpdate>,
    reset_born: Vec<WeightUpdate>,
    born_ids: Vec<u64>,
    terminal_ids: Vec<u64>,
    other_ids: Vec<u64>,
}

impl Components {
    fn read(
        developed: &Model,
        donor: &Model,
        target_pools: &BTreeSet<u64>,
        birth_weight: f32,
        birth_delay: u64,
    ) -> Result<Self> {
        let mut components = Self {
            born: vec![],
            learned_old: vec![],
            restore_old: vec![],
            reset_born: vec![],
            born_ids: vec![],
            terminal_ids: vec![],
            other_ids: vec![],
        };
        for edge in donor.network().synapses() {
            let id = edge.id().0;
            let learned = edge.weight().get();
            if let Some(old) = developed.network().synapse(edge.id()) {
                let prior = old.weight().get();
                if edge.is_enabled() && old.is_enabled() && prior != learned {
                    components.learned_old.push((id, prior, learned));
                    components.restore_old.push((id, learned, prior));
                }
                continue;
            }
            if !edge.is_enabled() || edge.delay_us() != birth_delay {
                return Err(error("A donor contains an unexpected formation profile"));
            }
            components.born.push(Birth {
                id,
                pre: edge.pre(),
                post: edge.post(),
                weight: learned,
                delay: birth_delay,
            });
            components.born_ids.push(id);
            if target_pools.contains(&edge.post().0) {
                components.terminal_ids.push(id);
            } else {
                components.other_ids.push(id);
            }
            if learned != birth_weight {
                components.reset_born.push((id, learned, birth_weight));
            }
        }
        Ok(components)
    }
}

fn distance(positions: &BTreeMap<NeuronId, Position>, pre: NeuronId, post: NeuronId) -> f64 {
    let [x, y, z] = std::array::from_fn(|i| positions[&pre].0[i] - positions[&post].0[i]);
    (x * x + y * y + z * z).sqrt()
}

fn rewire(
    initial: &Model,
    born: &[Birth],
    positions: &BTreeMap<NeuronId, Position>,
    radius: f64,
) -> (Vec<Birth>, Vec<(u64, u64)>) {
    let mut occupied: BTreeSet<_> = initial
        .network()
        .synapses()
        .map(|s| (s.pre(), s.post()))
        .chain(born.iter().map(|e| e.endpoints()))
        .collect();
    let mut sham = born.to_vec();
    let mut used = vec![false; born.len()];
    let mut swaps = Vec::new();
    for i in 0..born.len() {
        if used[i] {
            continue;
        }
        let left = born[i];
        let left_length = distance(positions, left.pre, left.post);
        for j in (i + 1)..born.len() {
            let right = born[j];
            if used[j]
                || left.pre == right.pre
                || left.post == right.post
                || positions[&left.pre].1 != positions[&right.pre].1
                || right.pre == left.post
                || left.pre == right.post
                || occupied.contains(&(right.pre, left.post))
                || occupied.contains(&(left.pre, right.post))
            {
                continue;
            }
            let new_left = distance(positions, right.pre, left.post);
            let new_right = distance(positions, left.pre, right.post);
            if new_left > radius
                || new_right > radius
                || (new_left - left_length).abs() > DISTANCE_CHANGE
                || (new_right - distance(positions, right.pre, right.post)).abs() > DISTANCE_CHANGE
            {
                continue;
            }
            occupied.remove(&left.endpoints());
            occupied.remove(&right.endpoints());
            sham[i].pre = right.pre;
            sham[j].pre = left.pre;
            occupied.insert(sham[i].endpoints());
            occupied.insert(sham[j].endpoints());
            swaps.push((left.id, right.id));
            used[i] = true;
            used[j] = true;
            break;
        }
    }
    (sham, swaps)
}

#[derive(Default)]
struct Patch<'a> {
    weights: &'a [WeightUpdate],
    blocked: &'a [u64],
    inserted: &'a [Birth],
}

impl Patch<'_> {
    fn publish(&self, model: &mut Model) -> Result<(Value, usize)> {
        let mut expected = topology(model);
        let mut rows = expected
            .as_array_mut()
            .expect("native topology rows")
            .iter_mut()
            .map(|r| (r[0].as_u64().expect("native synapse ID"), r))
            .collect::<BTreeMap<_, _>>();
        for &(id, before, weight) in self.weights {
            let row = rows
                .get_mut(&id)
                .ok_or_else(|| error("A transfer expects a missing weight"))?;
            if row[3] != json!(before) {
                return Err(error(
                    "A transfer weight does not start from the planned carrier value",
                ));
            }
            model
                .diagnostic_synapse_weight(SynapseId(id), weight)
                .map_err(fail)?;
            row[3] = json!(weight);
        }
        for id in self.blocked {
            rows.get_mut(id)
                .ok_or_else(|| error("A transfer cannot block a missing synapse"))?[5] =
                json!(false);
        }
        let cancelled = if self.blocked.is_empty() {
            0
        } else {
            let ids: Vec<_> = self.blocked.iter().copied().map(SynapseId).collect();
            model.diagnostic_block_synapses(&ids).map_err(fail)?
        };
        if !self.inserted.is_empty() {
            let inputs: Vec<_> = self
                .inserted
                .iter()
                .map(|b| (b.pre, b.post, b.weight))
                .collect();
            let ids = model
                .diagnostic_insert_formed_synapses(&inputs)
                .map_err(fail)?;
            if ids
                .iter()
                .map(|id| id.0)
                .ne(self.inserted.iter().map(|b| b.id))
            {
                return Err(error("A transfer allocated different synapse identities"));
            }
            expected
                .as_array_mut()
                .expect("native topology rows")
                .extend(self.inserted.iter().map(|e| e.row()));
        }
        let installed = topology(model);
        if installed != expected {
            return Err(error("A transfer topology differs from requested patch"));
        }
        Ok((installed, cancelled))
    }
}

struct Carrier {
    rested: Model,
    rest_spikes: usize,
}

impl Carrier {
    fn rest(assay: &Assay<'_>, model: &Model) -> Result<Self> {
        let mut rested = exact_fork(model)?;
        rested.freeze().map_err(fail)?;
        let before = rested.network().clone();
        let rest_spikes =
            wait_probe_rest(&mut rested, assay.config.rest_us, &assay.images["blank"])?;
        if !learned_parameters_equal(&before, rested.network()) {
            return Err(error("A transfer rest changed learned parameters"));
        }
        Ok(Self {
            rested,
            rest_spikes,
        })
    }

    fn read(&self, assay: &Assay<'_>, model: &Model, cue: &str, archived: &Value) -> Result<Value> {
        self.read_with_trace(assay, model, cue, archived, None)
    }

    fn read_with_trace(
        &self,
        assay: &Assay<'_>,
        model: &Model,
        cue: &str,
        archived: &Value,
        trace: Option<&mut Trace<'_>>,
    ) -> Result<Value> {
        let mut fork = exact_fork(model)?;
        let response = observe_probe(
            &mut fork,
            &assay.images[cue],
            &assay.images["blank"],
            self.rest_spikes,
            trace,
        )?
        .json();
        if !learned_parameters_equal(model.network(), fork.network())
            || ["start_us", "rest_core_spikes", "root_spikes"]
                .into_iter()
                .any(|key| response[key] != archived[key])
        {
            return Err(error(format!(
                "A transfer probe differs in frozen state or exposure for {cue}"
            )));
        }
        Ok(response)
    }

    fn arm(&self, assay: &Assay<'_>, patch: Patch<'_>, archived: &Value) -> Result<Value> {
        let mut fork = exact_fork(&self.rested)?;
        let (installed, cancelled) = patch.publish(&mut fork)?;
        let mut probes = BTreeMap::new();
        for cue in PROBES {
            let response = self.read(assay, &fork, cue, &archived[cue])?;
            if response["arrived_edges"]
                .as_array()
                .expect("observed arrival IDs")
                .iter()
                .filter_map(Value::as_u64)
                .any(|id| patch.blocked.binary_search(&id).is_ok())
            {
                return Err(error("A transfer observed a blocked arrival"));
            }
            probes.insert(cue, response);
        }
        let inserted: Vec<_> = patch.inserted.iter().map(|e| e.row()).collect();
        Ok(
            json!({"weight_updates":patch.weights,"blocked_edges":patch.blocked,
            "inserted_edges":inserted,"published_topology":installed,
            "probes":probes,"cancelled_arrivals":cancelled}),
        )
    }
}

fn recorded_config(record: &Value) -> Result<(Config, u64)> {
    let job = &record["job"];
    let cfg = &job["effective_config"];
    let seed = job["seed"]
        .as_u64()
        .ok_or_else(|| error("A transfer archive seed missing"))?;
    let mut config = ContingencyScenario.default_config();
    for key in super::super::PARAMETERS {
        ContingencyScenario.set_parameter(&mut config, key, &cfg[key])?;
    }
    ContingencyScenario.validate(&config)?;
    let intervention = Intervention {
        name: INTERVENTION.into(),
    };
    if job["intervention"] != INTERVENTION
        || cfg != &LiveGrowthScenario.snapshot(&config, seed, &intervention)
    {
        return Err(error(
            "A transfer archive configuration differs from live growth assay",
        ));
    }
    Ok((config, seed))
}

fn run(record: &Value, checksum: &str) -> Result<Value> {
    let (config, seed) = recorded_config(record)?;
    let archive = &record["details"];
    let reference = checkpoint(&config, seed)?.0;
    let core: Vec<_> = reference.processing_ids().iter().map(|id| id.0).collect();
    if archive["seed"] != seed
        || archive["core_ids"] != json!(core)
        || archive["initial_topology"] != topology(&reference)
        || archive["checkpoint_time_us"] != reference.next_observation_us()
        || archive["stimuli"] != json!(patterns())
    {
        return Err(error(
            "A transfer developed checkpoint differs from archive",
        ));
    }
    let geometry = reference.diagnostic_formation_geometry();
    let (radius, birth_weight, birth_delay) = geometry;
    let positions: BTreeMap<_, _> = reference
        .processing_ids()
        .iter()
        .map(|&id| {
            let neuron = reference.network().neuron(id).expect("Core neuron");
            let p = neuron.position();
            (
                id,
                (
                    [p.x, p.y, p.z].map(f64::from),
                    neuron.polarity() == Polarity::Excitatory,
                ),
            )
        })
        .collect();
    let targets: [Vec<u64>; 2] =
        serde_json::from_value(archive["targets"]["a"].clone()).map_err(fail)?;
    let all_targets = targets.into_iter().flatten().collect();
    let assay = Assay {
        config: &config,
        seed,
        reference,
        images: frames(&patterns())?,
    };
    let common = Carrier::rest(&assay, &assay.reference)?;
    let mut reset_probes = BTreeMap::new();
    for cue in PROBES {
        let archived = &archive["reset"][cue];
        let r = common.read(&assay, &common.rested, cue, archived)?;
        if &r != archived {
            return Err(error("A transfer reset probe differs from archive"));
        }
        reset_probes.insert(cue, r);
    }
    let mut mappings = BTreeMap::new();
    let start = assay.start_training("native")?;
    for mapping in MAPPINGS {
        let mut donor = exact_fork(&start)?;
        let a = &archive["stages_a"][mapping];
        if assay.train(&mut donor, mapping, 0, TrainingAudit::Formation)? != a["frames"]
            || topology(&donor) != a["end_topology"]
        {
            return Err(error(format!(
                "A transfer training replay differs for {mapping}"
            )));
        }
        let components = Components::read(
            &assay.reference,
            &donor,
            &all_targets,
            birth_weight,
            birth_delay,
        )?;
        let native = Carrier::rest(&assay, &donor)?;
        let mut native_arms = BTreeMap::new();
        for (name, patch) in [
            ("intact", Patch::default()),
            (
                "block_born",
                Patch {
                    blocked: &components.born_ids,
                    ..Patch::default()
                },
            ),
            (
                "block_terminal",
                Patch {
                    blocked: &components.terminal_ids,
                    ..Patch::default()
                },
            ),
            (
                "block_other",
                Patch {
                    blocked: &components.other_ids,
                    ..Patch::default()
                },
            ),
            (
                "restore_old",
                Patch {
                    weights: &components.restore_old,
                    ..Patch::default()
                },
            ),
            (
                "reset_born",
                Patch {
                    weights: &components.reset_born,
                    ..Patch::default()
                },
            ),
        ] {
            let arm = native.arm(&assay, patch, &a["probes"])?;
            if name == "intact"
                && PROBES
                    .into_iter()
                    .any(|cue| arm["probes"][cue] != a["probes"][cue])
            {
                return Err(error(format!(
                    "A transfer intact donor differs for {mapping}"
                )));
            }
            native_arms.insert(name, arm);
        }
        let (sham, swaps) = rewire(
            &assay.reference,
            &components.born,
            &positions,
            f64::from(radius),
        );
        let at_birth = |edges: &[Birth]| {
            edges
                .iter()
                .map(|e| Birth {
                    weight: birth_weight,
                    ..*e
                })
                .collect::<Vec<_>>()
        };
        let initial = at_birth(&components.born);
        let sham_initial = at_birth(&sham);
        let mut transfer = BTreeMap::new();
        for (name, inserted, weights) in [
            ("native_initial", initial.as_slice(), &[][..]),
            ("native_trained", components.born.as_slice(), &[][..]),
            ("sham_initial", sham_initial.as_slice(), &[][..]),
            ("sham_trained", sham.as_slice(), &[][..]),
            ("old_only", &[][..], components.learned_old.as_slice()),
            (
                "joint",
                components.born.as_slice(),
                components.learned_old.as_slice(),
            ),
            (
                "joint_sham",
                sham.as_slice(),
                components.learned_old.as_slice(),
            ),
        ] {
            transfer.insert(
                name,
                common.arm(
                    &assay,
                    Patch {
                        inserted,
                        weights,
                        ..Patch::default()
                    },
                    &archive["reset"],
                )?,
            );
        }
        mappings.insert(
            mapping,
            json!({"swaps":swaps,"native":native_arms,"transfer":transfer}),
        );
    }
    let positions: Vec<_> = positions
        .iter()
        .map(|(id, (p, excitatory))| (id.0, p, excitatory))
        .collect();
    Ok(
        json!({"protocol":PROTOCOL,"seed":seed,"archived_record_checksum":checksum,
        "formation_geometry":geometry,"sham_max_distance_change":DISTANCE_CHANGE,"positions":positions,
        "reset_probes":reset_probes,"mappings":mappings}),
    )
}

fn checked_growth_record(envelope: &Value) -> Result<(&Value, String)> {
    let record = &envelope["record"];
    let checksum = fingerprint(&serde_json::to_vec(record).map_err(fail)?);
    if record["details"]["protocol"] != super::PROTOCOL
        || record["full_replay_checked"] != true
        || envelope["checksum_fnv1a64"] != checksum
    {
        return Err(error(
            "A transfer requires an intact replay-verified live growth record",
        ));
    }
    Ok((record, checksum))
}

/// Repeat a complete retrospective readout before publishing its observations.
pub fn replay_a_transfer(envelope: &Value) -> Result<Value> {
    let (record, checksum) = checked_growth_record(envelope)?;
    let mut result = run(record, &checksum)?;
    if result != run(record, &checksum)? {
        return Err(error("A transfer full counterfactual replay differs"));
    }
    result["full_replay_checked"] = json!(true);
    Ok(result)
}
