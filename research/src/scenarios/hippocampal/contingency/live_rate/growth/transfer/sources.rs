//! Targeted source interventions for the two archived A responses.
use super::{
    Assay, Birth, Carrier, Components, DISTANCE_CHANGE, PROBES, Patch, Position, Trace,
    TrainingAudit, WeightUpdate, checked_growth_record, checkpoint, distance, exact_fork, fail,
    frames, patterns, recorded_config, topology,
};
use crate::{Result, error};
use nerva::core::{NeuronId, Polarity};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

const PLAN_PROTOCOL: &str = "NERVA-Contingency-A-Source-Plan/1";
const PROTOCOL: &str = "NERVA-Contingency-A-Source/1";
const MAPPING: &str = "reverse";
const CUE: &str = "cue1";
const GRAFT_NAMES: [&str; 2] = ["native_initial", "sham_initial"];
type Row = (u64, u64, u64, f32, u64, bool);
type Replacement = (u64, u64);

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Plan {
    protocol: String,
    seed: u64,
    archived_record_checksum: String,
    mapping: String,
    cue: String,
    target_cell: u64,
    active_edges: Vec<u64>,
    silent_edges: Vec<u64>,
    grafts: BTreeMap<String, GraftPlan>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GraftPlan {
    inserted_edges: Vec<Row>,
    reference_probes: Value,
    cycle: Vec<u64>,
    single_replacements: Vec<Replacement>,
    group_replacements: Vec<Replacement>,
}

struct Readout<'a> {
    assay: &'a Assay<'a>,
    carrier: &'a Carrier,
    target: &'a BTreeSet<u64>,
    reference_probes: &'a Value,
}

impl Readout<'_> {
    fn measure(
        &self,
        name: &str,
        inserted: &[Birth],
        blocked: &[u64],
        weights: &[WeightUpdate],
    ) -> Result<Value> {
        let mut fork = exact_fork(&self.carrier.rested)?;
        if !inserted.is_empty() {
            Patch {
                inserted,
                ..Patch::default()
            }
            .publish(&mut fork)?;
        }
        let (published, cancelled) = Patch {
            weights,
            blocked,
            ..Patch::default()
        }
        .publish(&mut fork)?;
        let mut probes = BTreeMap::new();
        for cue in PROBES {
            let baseline = &self.reference_probes[cue];
            let mut trace = Trace::new(self.target);
            let mut response =
                self.carrier
                    .read_with_trace(self.assay, &fork, cue, baseline, Some(&mut trace))?;
            if name == "intact" && &response != baseline {
                return Err(error(format!("A source intact probe differs for {cue}")));
            }
            if response["arrived_edges"]
                .as_array()
                .expect("observed arrival IDs")
                .iter()
                .filter_map(Value::as_u64)
                .any(|id| blocked.binary_search(&id).is_ok())
            {
                return Err(error("A source probe leaked a blocked arrival"));
            }
            trace.attach(&mut response);
            probes.insert(cue, response);
        }
        Ok(
            json!({"published_topology":published,"blocked_edges":blocked,"weight_updates":weights,
            "cancelled_arrivals":cancelled,"probes":probes}),
        )
    }
}

struct Geometry {
    radius: f64,
    positions: BTreeMap<NeuronId, Position>,
}

impl Geometry {
    fn apply(&self, native: &[Birth], replacements: &[Replacement]) -> Result<Vec<Birth>> {
        let desired: BTreeMap<_, _> = replacements.iter().copied().collect();
        if desired.len() != replacements.len() {
            return Err(error("A source plan replaces a slot more than once"));
        }
        let mut edited = 0;
        let mut revised = native.to_vec();
        for slot in &mut revised {
            let Some(&pre) = desired.get(&slot.id) else {
                continue;
            };
            let pre = NeuronId(pre);
            let Some((_, polarity)) = self.positions.get(&pre) else {
                return Err(error("A source plan places a source outside Core"));
            };
            let previous = distance(&self.positions, slot.pre, slot.post);
            let replacement = distance(&self.positions, pre, slot.post);
            if pre == slot.pre
                || pre == slot.post
                || *polarity != self.positions[&slot.pre].1
                || replacement > self.radius
                || (replacement - previous).abs() > DISTANCE_CHANGE
            {
                return Err(error(
                    "A source plan violates the slot geometry or polarity control",
                ));
            }
            slot.pre = pre;
            edited += 1;
        }
        if edited != replacements.len() {
            return Err(error("A source plan contains unavailable slots"));
        }
        Ok(revised)
    }
}

fn birth_layout(graft: &GraftPlan, born: &[Birth], weight: f32) -> Result<Vec<Birth>> {
    if graft.inserted_edges.len() != born.len() {
        return Err(error("A source graft does not cover the archived births"));
    }
    graft
        .inserted_edges
        .iter()
        .zip(born)
        .map(|(&(id, pre, post, installed, delay, enabled), source)| {
            if !enabled
                || id != source.id
                || post != source.post.0
                || delay != source.delay
                || installed != weight
            {
                return Err(error(
                    "A source graft changes archived slot identity, weight or delay",
                ));
            }
            Ok(Birth {
                id,
                pre: NeuronId(pre),
                post: NeuronId(post),
                weight,
                delay,
            })
        })
        .collect()
}

fn cycle_replacements(cycle: &[u64], births: &[Birth], plan: &Plan) -> Result<Vec<Replacement>> {
    let Some(first) = cycle.first() else {
        return Err(error("A source rotation requires a cycle"));
    };
    let slots: BTreeMap<_, _> = births.iter().map(|edge| (edge.id, *edge)).collect();
    if cycle.len() < 3
        || !plan.active_edges.contains(first)
        || cycle.iter().copied().collect::<BTreeSet<_>>().len() != cycle.len()
        || cycle.iter().any(|id| !slots.contains_key(id))
        || cycle[1..]
            .iter()
            .any(|id| slots[id].post.0 == plan.target_cell)
    {
        return Err(error(
            "A source rotation must change exactly one selected target slot",
        ));
    }
    Ok(cycle
        .iter()
        .zip(cycle.iter().cycle().skip(1))
        .map(|(&id, next)| (id, slots[next].pre.0))
        .collect())
}

fn follow_graft(
    readout: &Readout<'_>,
    geometry: &Geometry,
    graft: &GraftPlan,
    layout: &[Birth],
    plan: &Plan,
) -> Result<Value> {
    let rotated = cycle_replacements(&graft.cycle, layout, plan)?;
    let group_ids: Vec<_> = graft.group_replacements.iter().map(|edge| edge.0).collect();
    if graft
        .single_replacements
        .iter()
        .map(|edge| edge.0)
        .ne(plan.active_edges.iter().copied())
        || group_ids.is_empty()
        || group_ids.windows(2).any(|w| w[0] >= w[1])
        || group_ids.iter().any(|id| !plan.active_edges.contains(id))
    {
        return Err(error(
            "A source replacement plan must cover valid targeted slots",
        ));
    }
    let mut conditions = BTreeMap::new();
    for (name, blocked) in [
        ("intact", &[][..]),
        ("block_group", &group_ids),
        ("block_active", &plan.active_edges),
    ] {
        conditions.insert(
            name.to_owned(),
            readout.measure(name, layout, blocked, &[])?,
        );
    }
    for (name, edits) in [
        ("cycle_terminal", &rotated[..1]),
        ("cycle_collateral", &rotated[1..]),
        ("cycle_full", &rotated),
        ("replace_group", &graft.group_replacements),
    ] {
        let revised = geometry.apply(layout, edits)?;
        conditions.insert(name.to_owned(), readout.measure(name, &revised, &[], &[])?);
    }
    for edit in &graft.single_replacements {
        let name = format!("replace_{}", edit.0);
        let revised = geometry.apply(layout, std::slice::from_ref(edit))?;
        conditions.insert(name.clone(), readout.measure(&name, &revised, &[], &[])?);
    }
    Ok(json!(conditions))
}

fn check_target(plan: &Plan, archive: &Value, born: &[Birth]) -> Result<()> {
    let stage = &archive["stages_a"][MAPPING];
    let group: BTreeSet<u64> =
        serde_json::from_value(archive["targets"]["a"][0].clone()).map_err(fail)?;
    let probe = &stage["probes"][CUE];
    let spikes: Vec<(u64, u64)> =
        serde_json::from_value(probe["core_spikes"].clone()).map_err(fail)?;
    let arrivals: BTreeSet<u64> =
        serde_json::from_value(probe["arrived_edges"].clone()).map_err(fail)?;
    let activated: BTreeSet<_> = spikes
        .into_iter()
        .map(|(id, _)| id)
        .filter(|id| group.contains(id))
        .collect();
    if archive["checks"]["a_acquisition"][MAPPING][CUE]["passed"] != true
        || activated != BTreeSet::from([plan.target_cell])
    {
        return Err(error(
            "A source plan does not select exactly the archived target response",
        ));
    }
    let (active, silent): (Vec<_>, Vec<_>) = born
        .iter()
        .filter(|edge| edge.post.0 == plan.target_cell)
        .map(|edge| edge.id)
        .partition(|id| arrivals.contains(id));
    if active != plan.active_edges || silent != plan.silent_edges || active.is_empty() {
        return Err(error(
            "A source plan differs from the archived native support",
        ));
    }
    Ok(())
}

fn run(record: &Value, checksum: &str, plan: &Plan) -> Result<Value> {
    let (config, seed) = recorded_config(record)?;
    if plan.protocol != PLAN_PROTOCOL
        || plan.archived_record_checksum != checksum
        || plan.seed != seed
        || !matches!(seed, 34313 | 34327)
        || plan.mapping != MAPPING
        || plan.cue != CUE
        || plan.grafts.keys().map(String::as_str).ne(GRAFT_NAMES)
    {
        return Err(error(
            "A source plan identity differs from the fixed A cohort",
        ));
    }
    let archive = &record["details"];
    let reference = checkpoint(&config, seed)?.0;
    if topology(&reference) != archive["initial_topology"]
        || reference.next_observation_us() != archive["checkpoint_time_us"]
    {
        return Err(error("A source developed checkpoint differs from archive"));
    }
    let (radius, birth_weight, delay) = reference.diagnostic_formation_geometry();
    let positions = reference
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
    let geometry = Geometry {
        radius: f64::from(radius),
        positions,
    };
    let assay = Assay {
        config: &config,
        seed,
        reference,
        images: frames(&patterns())?,
    };
    let after_a = &archive["stages_a"][MAPPING];
    let mut donor = assay.start_training("native")?;
    if assay.train(&mut donor, MAPPING, 0, TrainingAudit::Formation)? != after_a["frames"]
        || topology(&donor) != after_a["end_topology"]
    {
        return Err(error("A source archived native donor training differs"));
    }
    let target = BTreeSet::from([plan.target_cell]);
    let components = Components::read(&assay.reference, &donor, &target, birth_weight, delay)?;
    check_target(plan, archive, &components.born)?;
    let native = Carrier::rest(&assay, &donor)?;
    let common = Carrier::rest(&assay, &assay.reference)?;
    let native_readout = Readout {
        assay: &assay,
        carrier: &native,
        target: &target,
        reference_probes: &after_a["probes"],
    };
    let mut adjustments = components.restore_old;
    adjustments.extend(components.reset_born);
    adjustments.sort_by_key(|weight| weight.0);
    let mut native_arms = BTreeMap::new();
    for (name, blocked, weights) in [
        ("intact", &[][..], &[][..]),
        ("block_active", plan.active_edges.as_slice(), &[][..]),
        ("block_silent", plan.silent_edges.as_slice(), &[][..]),
        ("reset_all_weights", &[][..], adjustments.as_slice()),
    ] {
        native_arms.insert(
            name.to_owned(),
            native_readout.measure(name, &[], blocked, weights)?,
        );
    }
    for edge in &plan.active_edges {
        let name = format!("block_{edge}");
        native_arms.insert(
            name.clone(),
            native_readout.measure(&name, &[], &[*edge], &[])?,
        );
    }
    let mut grafts = BTreeMap::new();
    for name in GRAFT_NAMES {
        let graft = &plan.grafts[name];
        let layout = birth_layout(graft, &components.born, birth_weight)?;
        let readout = Readout {
            assay: &assay,
            carrier: &common,
            target: &target,
            reference_probes: &graft.reference_probes,
        };
        grafts.insert(
            name,
            follow_graft(&readout, &geometry, graft, &layout, plan)?,
        );
    }
    Ok(
        json!({"protocol":PROTOCOL,"seed":seed,"archived_record_checksum":checksum,
        "target_cell":plan.target_cell,"native":native_arms,"grafts":grafts}),
    )
}

/// Run each targeted readout twice before publishing the electrical trace.
pub fn replay_a_source(envelope: &Value, plan: &Value) -> Result<Value> {
    let (record, checksum) = checked_growth_record(envelope)?;
    let plan: Plan = serde_json::from_value(plan.clone()).map_err(fail)?;
    let mut result = run(record, &checksum, &plan)?;
    if result != run(record, &checksum, &plan)? {
        return Err(error("A source full counterfactual replay differs"));
    }
    result["full_replay_checked"] = json!(true);
    Ok(result)
}
