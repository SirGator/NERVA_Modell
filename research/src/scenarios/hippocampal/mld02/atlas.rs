use super::super::{
    Config,
    association::{self, Observers, Training},
    causal::verify_only_weights,
    diagnosis::{
        self,
        probe::{self, Probe},
    },
    factorial::{search, weights},
    multilevel::tape::Tape,
};
use super::{CASES, METRICS, images, rule};
use crate::{Result, error, scenario::TrialMetrics};
use nerva::core::{NeuronRole, SynapseId};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn train(c: &Config, seed: u64, case: &str, record: bool) -> Result<(Training, Tape)> {
    let group = if case == "F" {
        "frozen_on"
    } else if case == "P" {
        "pair_on"
    } else {
        "three_on"
    };
    let mut tape = Tape::default();
    let t = association::train_recorded(
        c,
        seed,
        group,
        record,
        Some(rule(case)),
        Observers {
            history: None,
            tape: record.then_some(&mut tape),
            frame: if case == "D" {
                Some(images::distractor)
            } else {
                None
            },
        },
    )?;
    Ok((t, tape))
}
pub(super) fn get_weights(model: &nerva_vm::model::Model) -> Vec<(SynapseId, f32)> {
    model
        .network()
        .synapses()
        .map(|s| (s.id(), s.weight().get()))
        .collect()
}
fn delta(initial: &[(SynapseId, f32)], w: &[(SynapseId, f32)]) -> Vec<f64> {
    initial
        .iter()
        .zip(w)
        .map(|((i, a), (j, b))| {
            assert_eq!(i, j);
            f64::from(*b) - f64::from(*a)
        })
        .collect()
}
pub(super) fn probe(model: &mut nerva_vm::model::Model, c: &Config) -> Result<Probe> {
    probe::run_images(model, c, &images::PROBES, images::probe)
}
pub(super) fn margins(p: &Probe, targets: &[Vec<usize>]) -> Vec<f64> {
    (3..p.counts.len())
        .map(|i| {
            let cue = (i - 3) % 2;
            super::super::cosine(&p.counts[i], &targets[cue])
                - super::super::cosine(&p.counts[i], &targets[1 - cue])
        })
        .collect()
}
pub(super) fn response(p: Probe, base: &Probe, targets: &[Vec<usize>]) -> Value {
    let m = margins(&p, targets);
    let b = margins(base, targets);
    let g = m.iter().zip(&b).map(|(a, b)| a - b).collect::<Vec<_>>();
    let distinct = targets.iter().all(|v| v.iter().sum::<usize>() > 0)
        && super::super::cosine(&targets[0], &targets[1]) < 1. - 1e-9;
    let quiet = p.counts[0].iter().sum::<usize>() <= base.counts[0].iter().sum::<usize>();
    let gate = |start: usize, end: usize| {
        distinct
            && quiet
            && (start..end)
                .all(|i| m[i] > 1e-9 && g[i] > 1e-9 && p.counts[i + 3].iter().sum::<usize>() > 0)
    };
    let potential_effect = p.details["frames"]
        .as_array()
        .unwrap()
        .iter()
        .zip(base.details["frames"].as_array().unwrap())
        .any(|(a, b)| {
            let old = b["neurons"]
                .as_array()
                .unwrap()
                .iter()
                .map(|n| (n["id"].as_u64().unwrap(), &n["stored_after"]))
                .collect::<BTreeMap<_, _>>();
            a["neurons"].as_array().unwrap().iter().any(|n| {
                old.get(&n["id"].as_u64().unwrap())
                    .is_some_and(|v| **v != n["stored_after"])
            })
        });
    json!({"margins":m,"gains":g,"eligible":gate(0,2),"novel_eligible":gate(2,6),"all_cues_eligible":gate(0,6),
        "core_effect":diagnosis::effect(&p,base,"core_event_digest"),"motor_effect":diagnosis::effect(&p,base,"motor_event_digest"),
        "potential_effect":potential_effect,"probe":p.details})
}
struct Matched<'a> {
    reference: &'a Training,
    c: &'a Config,
    seed: u64,
    base: Probe,
    targets: Vec<Vec<usize>>,
    vectors: Vec<Vec<(SynapseId, f32)>>,
    rows: Vec<Value>,
    passes: usize,
}
impl Matched<'_> {
    fn observe(&mut self, name: &str, w: Vec<(SynapseId, f32)>) -> Result<usize> {
        if let Some(i) = self.vectors.iter().position(|old| *old == w) {
            return Ok(i);
        }
        let (mut branch, _) = train(self.c, self.seed, "B", false)?;
        if !branch.model.same_replay_state(&self.reference.model) {
            return Err(error("MLD02 weight transplant checkpoint differs"));
        }
        verify_only_weights(&self.reference.model, &mut branch.model, &w)?;
        let p = probe(&mut branch.model, self.c)?;
        let mut row = response(p, &self.base, &self.targets);
        row["label"] = json!(name);
        row["weights"] = json!(w.iter().map(|(i, w)| (i.0, *w)).collect::<Vec<_>>());
        row["only_weights_changed"] = json!(true);
        self.vectors.push(w);
        self.rows.push(row);
        self.passes += 1;
        Ok(self.rows.len() - 1)
    }
    fn normalized(&mut self, name: &str, d: &[f64], strength: f64) -> Result<Value> {
        let mut plan = weights::plan(&self.reference.weights, d, strength);
        if plan.details["feasible"] == true && plan.details["faithful"] != true {
            plan.details["reason"] = json!("f32_publication_fidelity");
        }
        let i = if plan.details["faithful"] == true {
            Some(self.observe(name, plan.weights.take().unwrap())?)
        } else {
            None
        };
        plan.details["observation"] = json!(i);
        Ok(plan.details)
    }
}
fn offline_vector(value: &Value, field: &str) -> Vec<f64> {
    value["synapses"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s[field].as_f64().unwrap())
        .collect()
}
fn gate_reference(rows: &[Value], a: usize, b: usize) -> bool {
    rows[a]["eligible"] == true
        && (0..2).all(|i| {
            rows[a]["gains"][i].as_f64().unwrap() > rows[b]["gains"][i].as_f64().unwrap() + 1e-9
        })
}
pub(super) fn run(c: &Config, seed: u64) -> Result<TrialMetrics> {
    let (reference, tape) = train(c, seed, "B", true)?;
    let (mut baseline, _) = train(c, seed, "B", false)?;
    if !reference.model.same_replay_state(&baseline.model) {
        return Err(error("MLD02 observer changed native state"));
    }
    let offline = tape.replay_mld02(&reference.model)?;
    verify_only_weights(&reference.model, &mut baseline.model, &reference.weights)?;
    let base = probe(&mut baseline.model, c)?;
    let targets = vec![base.counts[1].clone(), base.counts[2].clone()];
    let edges = diagnosis::build_edges(&reference, &base)?;
    let make_reference = |method: &str, sign: f64| {
        let score = search::scores(method, &edges, &reference, &base);
        edges
            .iter()
            .zip(score)
            .map(|(e, s)| (e.id, (f64::from(e.initial) + sign * s).clamp(0., 2.) as f32))
            .collect::<Vec<_>>()
    };
    let artificial = make_reference("sensory_and_recurrent", 1.);
    let swapped = make_reference("sensory_and_recurrent", -1.);
    let recurrent = make_reference("recurrent_two_hop", 1.);
    let recurrent_swapped = make_reference("recurrent_two_hop", -1.);
    let natural = delta(&reference.weights, &get_weights(&reference.model));
    let ref_delta = delta(&reference.weights, &artificial);
    let mut selected = natural
        .iter()
        .enumerate()
        .map(|(i, d)| (d.abs(), edges[i].id.0))
        .collect::<Vec<_>>();
    selected.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    let selected = selected
        .iter()
        .take(8)
        .map(|v| v.1)
        .chain(
            ref_delta
                .iter()
                .enumerate()
                .filter(|(_, d)| **d != 0.)
                .take(8)
                .map(|(i, _)| edges[i].id.0),
        )
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let init = association::build(c, seed, "three_on", Some(rule("B")))?;
    let history = tape.history_for_initial(&init, &reference.model, &selected, 500000)?;
    let mut matched = Matched {
        reference: &reference,
        c,
        seed,
        base,
        targets,
        vectors: Vec::new(),
        rows: Vec::new(),
        passes: 2,
    };
    let initial_row = json!({"label":"initial","margins":margins(&matched.base,&matched.targets),"gains":[0.,0.,0.,0.,0.,0.],
        "eligible":false,"novel_eligible":false,"all_cues_eligible":false,"core_effect":false,"motor_effect":false,"potential_effect":false,
        "probe":matched.base.details,"weights":reference.weights.iter().map(|(i,w)|(i.0,*w)).collect::<Vec<_>>(),"only_weights_changed":true});
    matched.vectors.push(reference.weights.clone());
    matched.rows.push(initial_row);
    let b_i = matched.observe("B", get_weights(&reference.model))?;
    let e_i = matched.observe("E", artificial.clone())?;
    let swap_i = matched.observe("E_swapped", swapped)?;
    let r_i = matched.observe("E_recurrent", recurrent.clone())?;
    let rs_i = matched.observe("E_recurrent_swapped", recurrent_swapped)?;
    let verified = gate_reference(&matched.rows, e_i, swap_i);
    let recurrent_verified = gate_reference(&matched.rows, r_i, rs_i);
    let mut cases = BTreeMap::new();
    let mut directions = BTreeMap::from([
        ("B".to_string(), natural.clone()),
        ("inverted".into(), natural.iter().map(|d| -d).collect()),
        (
            "positive_contributions".into(),
            offline_vector(&offline.json["original"], "positive_actual"),
        ),
        (
            "negative_contributions".into(),
            offline_vector(&offline.json["original"], "negative_actual"),
        ),
        ("E".into(), ref_delta.clone()),
        ("E_recurrent".into(), delta(&reference.weights, &recurrent)),
    ]);
    for (label, key) in [("offline_G", "gain64"), ("offline_C", "constant_positive")] {
        let rows = offline.json[key]["synapses"].as_array().unwrap();
        directions.insert(
            label.into(),
            rows.iter()
                .map(|s| s["weight"].as_f64().unwrap() - s["initial_weight"].as_f64().unwrap())
                .collect(),
        );
    }
    let mut case_passes = 0;
    for case in CASES.into_iter().filter(|s| *s != "B") {
        let (mut trained, t) = train(c, seed, case, true)?;
        let (mut reset, _) = train(c, seed, case, false)?;
        case_passes += 2;
        if !trained.model.same_replay_state(&reset.model)
            || trained.initial_digest != reference.initial_digest
        {
            return Err(error("MLD02 case checkpoint/initial network differs"));
        }
        if case != "D" && trained.details["input_digest"] != reference.details["input_digest"] {
            return Err(error("MLD02 generated inputs differ"));
        }
        let group = if case == "F" {
            "frozen_on"
        } else if case == "P" {
            "pair_on"
        } else {
            "three_on"
        };
        let init = association::build(c, seed, group, Some(rule(case)))?;
        let h = t.history_for_initial(&init, &trained.model, &selected, 500000)?;
        let w = get_weights(&trained.model);
        directions.insert(case.into(), delta(&reference.weights, &w));
        verify_only_weights(&trained.model, &mut reset.model, &trained.weights)?;
        let without = probe(&mut reset.model, c)?;
        let with = probe(&mut trained.model, c)?;
        let own = response(with, &without, &matched.targets);
        let transplantation = matched.observe(case, w)?;
        cases.insert(case,json!({"training":trained.details,"history":h,"own":own,"reset_probe":without.details,"observation":transplantation,"matched_training_state":true}));
    }
    let path_mask = |d: &[f64], sensor: bool| {
        d.iter()
            .enumerate()
            .map(|(i, v)| {
                let pre = reference
                    .model
                    .network()
                    .neuron(nerva::core::NeuronId(edges[i].pre))
                    .unwrap();
                let wanted = if sensor {
                    edges[i].path == "sensory_to_core"
                } else {
                    pre.role() == Some(NeuronRole::Processing)
                };
                if wanted { *v } else { 0. }
            })
            .collect::<Vec<_>>()
    };
    for (name, d) in [("B", &natural), ("E", &ref_delta)] {
        directions.insert(format!("{name}_sensor"), path_mask(d, true));
        directions.insert(format!("{name}_core"), path_mask(d, false));
    }
    let reference_norm = weights::norm(&ref_delta);
    let common = if verified {
        directions
            .values()
            .filter_map(|d| weights::limit(&reference.weights, d))
            .fold(reference_norm, f64::min)
    } else {
        0.
    };
    let mut normalized = BTreeMap::new();
    if verified {
        for (name, d) in &directions {
            normalized.insert(name.clone(),json!({"reference_strength":matched.normalized(name,d,reference_norm)?,"common_strength":matched.normalized(name,d,common)?}));
        }
    }
    let weak = if verified {
        matched.normalized("E_weak", &ref_delta, weights::norm(&natural))?
    } else {
        Value::Null
    };
    let mut lesions = Vec::new();
    let support = ref_delta
        .iter()
        .enumerate()
        .filter(|(_, d)| **d != 0.)
        .map(|(i, _)| i)
        .collect::<Vec<_>>();
    if verified {
        for &index in support.iter().take(8) {
            let mut ablated = artificial.clone();
            ablated[index].1 = reference.weights[index].1;
            let observation = matched.observe("E_single_edge_ablation", ablated)?;
            lesions.push(json!({"synapse_id":edges[index].id.0,"observation":observation,"necessary_for_this_reference_gate":matched.rows[observation]["eligible"]!=true}));
        }
    }
    let core_delta = &directions["B_core"];
    let native_core_weights = reference
        .weights
        .iter()
        .zip(core_delta)
        .map(|(&(id, w), d)| (id, (f64::from(w) + d) as f32))
        .collect();
    let native_core_i = matched.observe("B_core_native", native_core_weights)?;
    let distractor_at_core_strength = matched.normalized(
        "D_at_native_core_strength",
        &directions["D"],
        weights::norm(core_delta),
    )?;
    let candidate = distractor_at_core_strength["observation"]
        .as_u64()
        .is_some_and(|i| {
            let d = &matched.rows[i as usize];
            let core = &matched.rows[native_core_i];
            core["all_cues_eligible"] == true
                && matched.rows[b_i]["all_cues_eligible"] == true
                && cases["D"]["own"]["all_cues_eligible"] != true
                && (0..6).all(|j| {
                    core["gains"][j].as_f64().unwrap() > d["gains"][j].as_f64().unwrap() + 1e-9
                })
        });
    let passes = matched.passes + case_passes;
    if passes > 80 {
        return Err(error("MLD02 atlas training budget exceeded"));
    }
    let values = [
        f64::from(verified),
        f64::from(recurrent_verified),
        f64::from(candidate),
        passes as f64,
        matched.rows.len() as f64,
        0.,
        0.,
        1.,
    ];
    Ok(TrialMetrics {
        values: METRICS.into_iter().map(str::to_owned).zip(values).collect(),
        details: json!({"protocol":"NERVA-MLD-02/1","section":"atlas","initial_network_digest":reference.initial_digest,
        "B_training":reference.details,"B_history":history,"offline":offline.json,"cases":cases,"B_observation":b_i,
        "reference":{"verified":verified,"observation":e_i,"swap":swap_i,"recurrent_verified":recurrent_verified,"recurrent_observation":r_i,"recurrent_swap":rs_i,"norm":reference_norm},
        "directions":directions,"normalized":normalized,"common_norm":common,"weak_reference":weak,"native_core_observation":native_core_i,"distractor_at_native_core_strength":distractor_at_core_strength,"lesions":lesions,"lesions_truncated":support.len()>8,
        "observations":matched.rows,"edges":edges.iter().map(|e|json!([e.id.0,e.pre,e.post,e.path,e.initial,e.natural])).collect::<Vec<_>>(),
        "limits":"Reference support is not automatically necessary; at most eight single-edge ablations. Novel cues never construct the reference. D has disjoint training inputs. Pair uses no chemical learning factor. Only transplants isolate weights at one full B checkpoint; live cases may diverge."}),
    })
}
