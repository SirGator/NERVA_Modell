//! Dose-preserving external tapes for one target in a frozen native network.
use super::{
    Assay, CUE, Carrier, PROBES, Patch, Plan, Prepared, Trace, birth_layout, checked_growth_record,
    cycle_replacements, exact_fork, fail, prepare, recorded_config, topology,
};
use crate::{Result, error};
use nerva::core::NeuronId;
use nerva_vm::model::{Model, WINDOW_US};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

const PROTOCOL: &str = "NERVA-Contingency-A-Timing/1";
const PLAN_PROTOCOL: &str = "NERVA-Contingency-A-Timing-Plan/1";
const DURATION_US: u64 = 4 * WINDOW_US;
type Edit = (usize, u64);
type ArchivedArrival = (u64, u64, u64, f32);
type TapeArrival = (u64, u64, f32);

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TimingPlan {
    protocol: String,
    seed: u64,
    archived_record_checksum: String,
    target_cell: u64,
    source_plan: Plan,
    carriers: BTreeMap<String, CarrierPlan>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CarrierPlan {
    source_family: String,
    source_condition: String,
    probes: BTreeMap<String, Value>,
    arms: BTreeMap<String, Vec<Edit>>,
}

fn tape(reference: &Value, target: u64, start: u64, edits: &[Edit]) -> Result<Vec<TapeArrival>> {
    let arrivals: Vec<ArchivedArrival> =
        serde_json::from_value(reference["arrivals"].clone()).map_err(fail)?;
    let replacement: BTreeMap<_, _> = edits.iter().copied().collect();
    if replacement.len() != edits.len() || replacement.keys().any(|&index| index >= arrivals.len())
    {
        return Err(error("A timing plan refers to invalid arrival rows"));
    }
    let stop = start
        .checked_add(DURATION_US)
        .ok_or_else(|| error("A timing probe clock overflow"))?;
    let mut result = Vec::with_capacity(arrivals.len());
    for (index, (at, cell, edge, amplitude)) in arrivals.into_iter().enumerate() {
        if cell != target || !(start..stop).contains(&at) || !amplitude.is_finite() {
            return Err(error("A timing archive includes an invalid target arrival"));
        }
        let time = replacement.get(&index).copied().unwrap_or(at - start);
        if time >= DURATION_US {
            return Err(error("A timing edit leaves the fixed probe horizon"));
        }
        result.push((time, edge, amplitude));
    }
    result.sort_by_key(|arrival| arrival.0);
    Ok(result)
}

struct Replay<'a> {
    assay: &'a Assay<'a>,
    carrier: &'a Carrier,
    attached: Model,
    target: u64,
    incoming: BTreeSet<u64>,
}

impl Replay<'_> {
    fn probe(&self, cue: &str, edits: &[Edit], reference: &Value) -> Result<Value> {
        let start = self.attached.next_observation_us();
        let rows = tape(reference, self.target, start, edits)?;
        if rows
            .iter()
            .any(|&(_, edge, _)| !self.incoming.contains(&edge))
        {
            return Err(error(
                "A timing tape references an unavailable original synapse",
            ));
        }
        let to_schedule: Vec<_> = rows
            .iter()
            .map(|&(offset, _, amp)| (start + offset, amp))
            .collect();
        let mut fork = exact_fork(&self.attached)?;
        let (disabled, cancelled) = fork
            .diagnostic_replace_core_inputs(NeuronId(self.target), &to_schedule)
            .map_err(fail)?;
        if disabled
            .iter()
            .map(|id| id.0)
            .ne(self.incoming.iter().copied())
        {
            return Err(error(
                "A timing tape did not replace exactly the target inputs",
            ));
        }
        let targets = BTreeSet::from([self.target]);
        let mut trace = Trace::new(&targets);
        let mut response =
            self.carrier
                .read_with_trace(self.assay, &fork, cue, reference, Some(&mut trace))?;
        trace.attach_tape(&mut response);
        let delivered: Vec<_> = to_schedule
            .iter()
            .map(|&(at, amp)| (at, self.target, amp))
            .collect();
        if response["external_inputs"] != json!(delivered) || response["arrivals"] != json!([]) {
            return Err(error(
                "A timing replay did not deliver only the substituted tape",
            ));
        }
        if edits.is_empty() {
            let archived: Vec<u64> =
                serde_json::from_value(reference["arrived_edges"].clone()).map_err(fail)?;
            let expected: Vec<_> = archived
                .into_iter()
                .filter(|id| !self.incoming.contains(id))
                .collect();
            if response["arrived_edges"] != json!(expected)
                || ["core_spikes", "boundaries"]
                    .into_iter()
                    .any(|key| response[key] != reference[key])
            {
                return Err(error(format!(
                    "A timing unedited tape did not reconstruct archived {cue} response"
                )));
            }
        }
        let disabled: Vec<_> = disabled.iter().map(|id| id.0).collect();
        Ok(
            json!({"disabled_edges":disabled,"cancelled_arrivals":cancelled,
            "tape":rows,"response":response}),
        )
    }

    fn measure(&self, plan: &CarrierPlan) -> Result<Value> {
        if plan
            .arms
            .get("reference")
            .is_none_or(|edits| !edits.is_empty())
            || !plan.probes.keys().map(String::as_str).eq(PROBES)
        {
            return Err(error(
                "A timing carrier does not contain an unedited reference and all controls",
            ));
        }
        let mut arms = BTreeMap::new();
        for (name, ed) in &plan.arms {
            let mut probes = BTreeMap::new();
            for cue in PROBES {
                let edits = if cue == CUE { ed.as_slice() } else { &[] };
                probes.insert(cue, self.probe(cue, edits, &plan.probes[cue])?);
            }
            arms.insert(name, json!({"probes":probes}));
        }
        Ok(json!({"pre_clamp_topology":topology(&self.attached),"arms":arms}))
    }
}

fn run(record: &Value, checksum: &str, plan: &TimingPlan) -> Result<Value> {
    let (config, seed) = recorded_config(record)?;
    let expected: &[&str] = if seed == 34313 {
        &["native"]
    } else {
        &["grown", "native", "rotated"]
    };
    if plan.protocol != PLAN_PROTOCOL
        || plan.seed != seed
        || plan.archived_record_checksum != checksum
        || plan.target_cell != plan.source_plan.target_cell
        || plan
            .carriers
            .keys()
            .map(String::as_str)
            .ne(expected.iter().copied())
    {
        return Err(error(
            "A timing plan identity differs from the selected cases",
        ));
    }
    let Prepared {
        assay,
        donor,
        components,
        geometry,
        birth_weight,
    } = prepare(record, checksum, &plan.source_plan, &config, seed)?;
    let native = Carrier::rest(&assay, &donor)?;
    let common = Carrier::rest(&assay, &assay.reference)?;
    let target = NeuronId(plan.target_cell);
    let mut results = BTreeMap::new();
    for (name, carrier_plan) in &plan.carriers {
        let family = carrier_plan.source_family.as_str();
        let condition = carrier_plan.source_condition.as_str();
        let (carrier, inserted) = match (name.as_str(), family, condition) {
            ("native", "native", "intact") => (&native, Vec::new()),
            ("grown", "native_initial", "intact") | ("rotated", "native_initial", "cycle_full") => {
                let graft = &plan.source_plan.grafts[family];
                let layout = birth_layout(graft, &components.born, birth_weight)?;
                let layout = if name == "rotated" {
                    let edits = cycle_replacements(&graft.cycle, &layout, &plan.source_plan)?;
                    geometry.apply(&layout, &edits)?
                } else {
                    layout
                };
                (&common, layout)
            }
            _ => {
                return Err(error(
                    "A timing carrier differs from its preselected source arm",
                ));
            }
        };
        let mut attached = exact_fork(&carrier.rested)?;
        Patch {
            inserted: &inserted,
            ..Patch::default()
        }
        .publish(&mut attached)?;
        let incoming = attached
            .network()
            .incoming_synapse_ids(target)
            .iter()
            .filter(|&&id| {
                attached
                    .network()
                    .synapse(id)
                    .expect("incoming edge")
                    .is_enabled()
            })
            .map(|id| id.0)
            .collect();
        let readout = Replay {
            assay: &assay,
            carrier,
            target: target.0,
            attached,
            incoming,
        };
        results.insert(name, readout.measure(carrier_plan)?);
    }
    Ok(
        json!({"protocol":PROTOCOL,"seed":seed,"archived_record_checksum":checksum,
        "target_cell":plan.target_cell,"carriers":results}),
    )
}

/// Replay an archived target tape and its fixed-amplitude time edits twice.
pub fn replay_a_timing(envelope: &Value, plan: &Value) -> Result<Value> {
    let (record, checksum) = checked_growth_record(envelope)?;
    let plan: TimingPlan = serde_json::from_value(plan.clone()).map_err(fail)?;
    let mut result = run(record, &checksum, &plan)?;
    if result != run(record, &checksum, &plan)? {
        return Err(error("A timing full counterfactual replay differs"));
    }
    result["full_replay_checked"] = json!(true);
    Ok(result)
}
