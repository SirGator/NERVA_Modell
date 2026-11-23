//! Factor Core homeostasis and inherited tags during native post-development learning.
use super::super::{Tape, fail, train_with_homeostasis};
use super::{
    Assay, CUES, CapacityScenario, Config, MAPPINGS, Scenario, Weights, build, checkpoint,
    eligibility::{checked_capacity, recorded_focus, u64_field},
    get_weights, json_responses, schedule, serialized,
};
use crate::{Result, error};
use nerva::{
    config::LearningConfig,
    core::SynapseId,
    runtime::{LearningAuditEvent, LearningPhase, ObservationEvent},
};
use nerva_vm::model::HomeostasisScope;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

const PROTOCOL: &str = "NERVA-Contingency-Core-Homeostasis/1";
const ARMS: [(&str, bool, bool); 4] = [
    ("off_keep", false, false),
    ("off_clear", false, true),
    ("core_keep", true, false),
    ("core_clear", true, true),
];

fn mapped_focus(assay: &Assay<'_>, focus: &BTreeSet<SynapseId>) -> Vec<(u64, u64, u64, u64)> {
    focus
        .iter()
        .map(|&id| {
            let s = assay.reference.network().synapse(id).unwrap();
            (id.0, s.pre().0, s.post().0, s.delay_us())
        })
        .collect()
}

fn training_audit(
    tape: &Tape,
    focus: &BTreeSet<SynapseId>,
    core: &BTreeSet<u64>,
    active: bool,
) -> Result<Value> {
    let mut homeostasis = Vec::new();
    let mut arrivals = Vec::new();
    let mut pairs = Vec::new();
    let mut delayed = Vec::new();
    for (index, (input, log)) in tape.frames.iter().enumerate() {
        for event in &input.raw_events {
            match *event {
                ObservationEvent::HomeostasisChanged {
                    time,
                    neuron_id,
                    old_intrinsic_current,
                    new_intrinsic_current,
                    old_structural_drive,
                    new_structural_drive,
                    ..
                } => {
                    if !active || !core.contains(&neuron_id.0) {
                        return Err(error("homeostasis changed an unapproved neuron"));
                    }
                    homeostasis.push((
                        time.0,
                        neuron_id.0,
                        old_intrinsic_current,
                        new_intrinsic_current,
                        old_structural_drive,
                        new_structural_drive,
                    ));
                }
                ObservationEvent::SynapticArrival {
                    time, synapse_id, ..
                } if focus.contains(&synapse_id) => {
                    arrivals.push((time.0, synapse_id.0));
                }
                _ => {}
            }
        }
        for event in &log.events {
            if let LearningAuditEvent::Trace {
                time,
                synapse,
                phase,
                tag_before,
                tag_after,
                ..
            } = event
                && focus.contains(synapse)
                && *phase != LearningPhase::Delayed
                && tag_before != tag_after
            {
                let phase = if *phase == LearningPhase::PreArrival {
                    "pre"
                } else {
                    "post"
                };
                pairs.push((time.0, synapse.0, phase, *tag_before, *tag_after));
            }
        }
        for (edge, row) in log.delayed.iter().filter(|(edge, _)| focus.contains(edge)) {
            if row.positive_requested != 0. || row.negative_requested != 0. {
                delayed.push((
                    index,
                    edge.0,
                    row.positive_requested,
                    row.negative_requested,
                ));
            }
        }
    }
    if active && homeostasis.is_empty() {
        return Err(error(
            "the Core maintenance arm produced no observable updates",
        ));
    }
    Ok(
        json!({"homeostasis_events":homeostasis,"pre_arrivals":arrivals,
              "tag_events":pairs,"delay_requests":delayed}),
    )
}

fn observe_training(
    assay: &Assay<'_>,
    mapping: &str,
    focus: &BTreeSet<SynapseId>,
    active: bool,
    cleared: bool,
    archive: &Value,
) -> Result<(Value, Weights)> {
    let (mut model, _) = checkpoint(assay.c, assay.seed)?;
    if !assay.reference.same_replay_state(&model) {
        return Err(error(
            "homeostasis arm starts from a different developed state",
        ));
    }
    let removed_tags = if cleared {
        model
            .diagnostic_clear_learning_eligibility()
            .map_err(fail)?
    } else {
        0
    };
    if active {
        model
            .set_checkpoint_homeostasis_scope(HomeostasisScope::Core)
            .map_err(fail)?;
    }
    let mut tape = Tape::default();
    let order = schedule(assay.c, assay.seed, mapping, 0);
    let frames =
        train_with_homeostasis(&mut model, &order, &assay.frames, Some(&mut tape), active)?;
    let weights = get_weights(&model);
    let core = assay.core_weights(&weights)?;
    let published = json!(serialized(&weights));
    if !active && (frames != archive["frames"] || published != archive["weights"]) {
        return Err(error(format!(
            "archived {mapping} training differs in disabled homeostasis arm"
        )));
    }
    let mut audit = training_audit(&tape, focus, &assay.core, active)?;
    audit["frames"] = frames;
    audit["weights"] = published;
    audit["removed_tags"] = json!(removed_tags);
    audit["final_core"] = json!(
        model
            .processing_ids()
            .iter()
            .map(|id| {
                let cell = model.network().neuron(*id).unwrap();
                (id.0, cell.intrinsic_current(), cell.structural_drive())
            })
            .collect::<Vec<_>>()
    );
    Ok((audit, core))
}

fn run(source: &Value, eligibility: &Value, checksum: &str) -> Result<Value> {
    let details = &source["details"];
    let followup = &details["followup"];
    let settings = &source["job"]["effective_config"];
    let cfg = Config {
        processing_neurons: usize::try_from(u64_field(settings, "processing_neurons")?)
            .map_err(fail)?,
        training_repetitions: usize::try_from(u64_field(settings, "training_repetitions")?)
            .map_err(fail)?,
        rest_us: u64_field(settings, "rest_us")?,
    };
    CapacityScenario.validate(&cfg)?;
    let seed = u64_field(&source["job"], "seed")?;
    let (assay, development, _) = build(&cfg, seed)?;
    if eligibility["protocol"] != "NERVA-Contingency-Eligibility/1"
        || eligibility["replay_verified"] != true
        || eligibility["archived_record_checksum"] != checksum
        || eligibility["seed"] != seed
        || details["seed"] != seed
        || details["development"] != development
        || details["topology"] != json!(assay.topology())
        || eligibility["topology"] != details["topology"]
        || eligibility["target_groups"] != details["target_groups"]
    {
        return Err(error(
            "archived controls differ from the homeostasis checkpoint",
        ));
    }
    let targets: [BTreeSet<u64>; 2] =
        serde_json::from_value(details["target_groups"].clone()).map_err(fail)?;
    let focus = [
        recorded_focus(&assay, followup, "forward")?,
        recorded_focus(&assay, followup, "reverse")?,
    ];
    let reset = assay.observe(&assay.initial, &["cue0", "cue1", "blank"])?;
    let reset_json = json_responses(&reset);
    if reset_json
        .iter()
        .any(|(key, row)| *row != details["reset"][*key])
    {
        return Err(error(
            "rebuilt homeostasis reset differs from archived reset",
        ));
    }
    let mut arms = BTreeMap::new();
    for (name, active, cleared) in ARMS {
        let mut training = BTreeMap::new();
        let mut weights = Vec::new();
        for (idx, mapping) in MAPPINGS.into_iter().enumerate() {
            let archive = if cleared {
                &eligibility["mappings"][mapping]["live"]
            } else {
                &followup["training"][mapping]
            };
            let (observed, core) =
                observe_training(&assay, mapping, &focus[idx], active, cleared, archive)?;
            if cleared
                && observed["removed_tags"]
                    != eligibility["mappings"][mapping]["live"]["removed_tags"]
            {
                return Err(error("homeostasis trial removed different checkpoint tags"));
            }
            training.insert(mapping, observed);
            weights.push(core);
        }
        let [a, b] = weights
            .try_into()
            .map_err(|_| error("homeostasis requires two cue mappings"))?;
        let recall = if !active {
            if cleared {
                eligibility["probes"]["live_clear"].clone()
            } else {
                followup["native"].clone()
            }
        } else {
            assay.compare(&[a, b], &reset, &targets, &CUES)?.0
        };
        arms.insert(name, json!({"homeostasis":active,"clear_eligibility":cleared,"training":training,"recall":recall}));
    }
    let initial_core = assay
        .reference
        .processing_ids()
        .iter()
        .map(|id| {
            let n = assay.reference.network().neuron(*id).unwrap();
            (id.0, n.intrinsic_current(), n.structural_drive())
        })
        .collect::<Vec<_>>();
    Ok(
        json!({"protocol":PROTOCOL,"seed":seed,"archived_record_checksum":checksum,
        "checkpoint_time_us":development["checkpoint_time_us"],"stdp_window_us":LearningConfig::default().stdp_window_us,
        "topology":assay.topology(),"core_ids":assay.core,"target_groups":targets,"initial_core":initial_core,
        "focus":{"forward":mapped_focus(&assay,&focus[0]),"reverse":mapped_focus(&assay,&focus[1])},
        "reset":reset_json,"arms":arms}),
    )
}

pub fn replay_homeostasis(capacity: &Value, eligibility: &Value) -> Result<Value> {
    let (source, checksum) = checked_capacity(capacity)?;
    let mut output = run(source, eligibility, &checksum)?;
    if output != run(source, eligibility, &checksum)? {
        return Err(error("Core homeostasis replay differs"));
    }
    output["replay_verified"] = json!(true);
    Ok(output)
}
