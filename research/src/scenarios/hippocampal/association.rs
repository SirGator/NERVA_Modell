//! Paired hippocampal rule/chemistry factorial and compact loss-path observations.
use super::{
    Config, HippocampalScenario,
    causal::{apply, verify_only_weights},
    fail, frame, model_config, summarize,
    weights::{snapshot_json, weight_changes, weight_snapshot},
};
use crate::{
    Result, error, fingerprint_debug,
    scenario::{Intervention, Scenario, TrialMetrics},
};
use nerva::{
    core::{NeuronRole, SynapseId},
    runtime::{LearningAuditEvent, LearningAuditLog},
};
use nerva_vm::model::{
    LearningRule, Model, PlasticitySettings, SensorConfig, SensorMode, SpikeCoding,
    audit::{AuditOptions, learned_parameters_equal},
};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

pub const GROUPS: [&str; 8] = [
    "three_on",
    "three_off",
    "pair_on",
    "pair_off",
    "frozen_on",
    "frozen_off",
    "three_unpaired",
    "pair_unpaired",
];
pub struct AssociationScenario;
fn settings(group: &str) -> Result<(LearningRule, bool, bool, bool)> {
    if !GROUPS.contains(&group) {
        return Err(error("unknown association control"));
    }
    Ok((
        if group.starts_with("pair") {
            LearningRule::PairStdp
        } else {
            LearningRule::ThreeFactor
        },
        !group.ends_with("off"),
        !group.starts_with("frozen"),
        group.ends_with("unpaired"),
    ))
}
fn build(c: &Config, seed: u64, group: &str, override_rule: Option<LearningRule>) -> Result<Model> {
    let (rule, chemistry, _, _) = settings(group)?;
    Model::with_sensor_and_learning(
        model_config(c, seed, if chemistry { "smooth" } else { "no_chemistry" }),
        SensorConfig {
            mode: SensorMode::Pixels,
            resolution: 8,
            coding: SpikeCoding::Deterministic,
            max_rate_hz: 120.,
        },
        override_rule.unwrap_or(rule),
    )
    .map_err(fail)
}
fn schedule(c: &Config, seed: u64, unpaired: bool) -> Vec<usize> {
    let mut images = Vec::new();
    for repetition in 0..c.training_repetitions {
        let order = if (seed ^ repetition as u64) & 1 == 0 {
            [0, 1]
        } else {
            [1, 0]
        };
        for cue in order {
            images.extend([cue + 2, if unpaired { 1 - cue } else { cue }, 4]);
        }
    }
    images
}
#[derive(Default)]
struct Audit {
    rows: usize,
    traces: usize,
    chemicals: usize,
    changed: usize,
    tags: usize,
    requested: usize,
    rounded: usize,
    zero_signal: usize,
    requested_mass: f64,
    actual_mass: f64,
    motor_signal_samples: usize,
    track_mass: bool,
    mass: BTreeMap<u64, MassLedger>,
}

#[derive(Default)]
struct MassLedger {
    requested: f64,
    actual: f64,
    first_residual: Option<f64>,
    last_residual: f64,
    clipped: f64,
    clipped_l1: f64,
    rounding: f64,
    rounding_l1: f64,
}
impl Audit {
    fn add(&mut self, log: LearningAuditLog, native_changes: usize, model: &Model) -> Result<()> {
        if !log.is_complete() {
            return Err(error("association learning audit overflow"));
        }
        self.rows += log.events.len() + log.delayed.len();
        if self.rows > 1_000_000 {
            return Err(error("association audit exceeds one million rows"));
        }
        let mut changes = 0;
        for event in log.events {
            match event {
                LearningAuditEvent::Trace {
                    old_weight,
                    new_weight,
                    tag_before,
                    tag_after,
                    ..
                } => {
                    self.traces += 1;
                    self.tags += usize::from(tag_before != 0. || tag_after != 0.);
                    let delta = (f64::from(new_weight.get()) - f64::from(old_weight.get())).abs();
                    changes += usize::from(delta != 0.);
                    self.actual_mass += delta;
                }
                LearningAuditEvent::ChemicalSample {
                    neuron, response, ..
                } => {
                    self.chemicals += 1;
                    if model
                        .network()
                        .neuron(neuron)
                        .is_some_and(|n| n.role() == Some(NeuronRole::Motor))
                        && response.learning_signal != 0.
                    {
                        self.motor_signal_samples += 1;
                    }
                }
            }
        }
        if changes != native_changes {
            return Err(error("learning audit weight-event mismatch"));
        }
        self.changed += changes;
        for (id, summary) in &log.delayed {
            self.requested += summary.nonzero_requested;
            self.rounded += summary.nonzero_requested_without_weight_change;
            self.zero_signal += summary.zero_signal_integrations;
            self.requested_mass += summary.positive_requested - summary.negative_requested;
            if self.track_mass {
                let entry = self.mass.entry(id.0).or_default();
                entry.requested += summary.positive_requested + summary.negative_requested;
                entry.actual += summary.positive_actual + summary.negative_actual;
                if let Some(value) = summary.first_residual {
                    entry.first_residual.get_or_insert(value);
                }
                if let Some(value) = summary.last_residual {
                    entry.last_residual = value;
                }
                entry.clipped += summary.positive_clipped + summary.negative_clipped;
                entry.clipped_l1 += summary.positive_clipped - summary.negative_clipped;
                entry.rounding += summary.positive_rounding_error + summary.negative_rounding_error;
                entry.rounding_l1 +=
                    summary.positive_rounding_error - summary.negative_rounding_error;
            }
        }
        Ok(())
    }
    fn json(&self) -> Value {
        let mut result = json!({"complete":true,"observed_rows":self.rows,"trace_events":self.traces,
            "chemical_samples":self.chemicals,"tag_events":self.tags,
            "weight_events":self.changed,"requested_updates":self.requested,
            "requests_without_change":self.rounded,"zero_factor_integrations":self.zero_signal,
            "requested_absolute_mass":self.requested_mass,"actual_absolute_mass":self.actual_mass,
            "nonzero_motor_factor_samples":self.motor_signal_samples,
            "storage":"Streaming aggregates; full raw audit traces are not retained."});
        if self.track_mass {
            let rows:Vec<_>=self.mass.iter().map(|(id,m)|json!({"synapse_id":id,
                "requested":m.requested,"actual":m.actual,"residual_before":m.first_residual.unwrap_or(0.),
                "residual_after":m.last_residual,"clipped":m.clipped,"clipped_l1":m.clipped_l1,
                "rounding":m.rounding,"rounding_l1":m.rounding_l1,
                "balance_error":m.requested-m.actual-(m.last_residual-m.first_residual.unwrap_or(0.))-m.clipped-m.rounding})).collect();
            result["mass_ledger"] = json!(rows);
        }
        result
    }
}
pub(super) struct Training {
    pub model: Model,
    pub weights: Vec<(SynapseId, f32)>,
    pub initial_digest: String,
    pub details: Value,
}
pub(super) fn train(c: &Config, seed: u64, group: &str, audited: bool) -> Result<Training> {
    train_with_learning(c, seed, group, audited, None)
}
pub(super) fn train_with_learning(
    c: &Config,
    seed: u64,
    group: &str,
    audited: bool,
    override_rule: Option<LearningRule>,
) -> Result<Training> {
    let mut model = build(c, seed, group, override_rule)?;
    let initial_digest = fingerprint_debug(model.network());
    let weights = model
        .network()
        .synapses()
        .map(|s| (s.id(), s.weight().get()))
        .collect();
    let before = weight_snapshot(&model);
    let (_, _, learning, unpaired) = settings(group)?;
    model
        .set_plasticity(PlasticitySettings {
            learning,
            development: false,
            homeostasis: false,
        })
        .map_err(fail)?;
    if audited {
        model.enable_learning_audit(100_000).map_err(fail)?;
        model.enable_audit(AuditOptions {
            raw_events: false,
            input_spikes: true,
        });
    }
    let mut audit = Audit {
        track_mass: override_rule.is_some(),
        ..Default::default()
    };
    let mut windows = Vec::new();
    let mut input_signatures = Vec::new();
    let mut input_patterns = BTreeMap::<usize, BTreeSet<Vec<(u64, usize)>>>::new();
    let mut sensory = 0;
    let mut arrivals = 0;
    let mut core = 0;
    for (index, image) in schedule(c, seed, unpaired).into_iter().enumerate() {
        let step = model.step(&frame(image).map_err(fail)?).map_err(fail)?;
        sensory += step.sensory_spikes;
        core += step.core_counts.iter().sum::<usize>();
        if audited {
            let input = model
                .take_audit()
                .ok_or_else(|| error("missing association input audit"))?;
            arrivals += input.sensor_to_core_arrivals;
            let signature = fingerprint_debug(&(&input.input_rates, &input.input_spikes));
            // Compare channel patterns, excluding absolute frame times. Nonzero
            // activity alone does not establish distinguishable input content.
            let mut channels = BTreeMap::<u64, usize>::new();
            for spike in &input.input_spikes {
                *channels.entry(spike.channel.0).or_default() += 1;
            }
            let channel_counts: Vec<_> = channels.into_iter().collect();
            input_patterns
                .entry(image)
                .or_default()
                .insert(channel_counts.clone());
            input_signatures.push(signature.clone());
            let log = model.take_learning_audit().map_err(fail)?;
            audit.add(log, step.weight_changes, &model)?;
            windows.push(json!({"index":index,"image":image,"start_us":step.start_us,"end_us":step.end_us,
                "input_digest":signature,"generated_spikes":step.sensory_spikes,
                "input_channel_counts":channel_counts,
                "visual_spikes":input.visual_neuron_spikes,"sensor_to_core_arrivals":input.sensor_to_core_arrivals,
                "sensor_to_core_amplitude":input.sensor_to_core_amplitude,
                "core_spikes":step.core_counts.iter().sum::<usize>(),"motor_spikes":step.motor_spikes,
                "weight_events":step.weight_changes}));
        }
    }
    if audited {
        let log = model.disable_learning_audit().map_err(fail)?;
        if !log.events.is_empty() || !log.delayed.is_empty() {
            return Err(error("undrained association audit"));
        }
    }
    model.freeze().map_err(fail)?;
    let after = weight_snapshot(&model);
    let residuals: Vec<_> = if override_rule.is_some() {
        model
            .network()
            .synapses()
            .map(|s| (s.id().0, s.learning_weight_residual()))
            .collect()
    } else {
        Vec::new()
    };
    let changes = weight_changes(&before, &after, model.network())?;
    let input_patterns_disjoint = input_patterns
        .get(&0)
        .zip(input_patterns.get(&1))
        .is_some_and(|(a, b)| !a.is_empty() && !b.is_empty() && a.is_disjoint(b));
    Ok(Training {
        model,
        weights,
        initial_digest,
        details: json!({"input_digest":fingerprint_debug(&input_signatures),"sensory_spikes":sensory,
            "sensor_to_core_arrivals":arrivals,"core_spikes":core,"windows":windows,"audit":audit.json(),
            "input_patterns_disjoint":input_patterns_disjoint,
            "weight_residuals_after":residuals,
            "weights_before":snapshot_json(&before),"weights_after":snapshot_json(&after),"weight_changes":changes}),
    })
}
pub(super) fn recall(model: &mut Model, c: &Config) -> Result<Value> {
    model.freeze().map_err(fail)?;
    let original = model.network().clone();
    let mut responses = Vec::new();
    let mut observations = Vec::new();
    for image in 0..4 {
        let mut remaining = c.rest_us;
        let mut resting = 0usize;
        while remaining > 0 {
            let duration = remaining.min(nerva_vm::model::WINDOW_US);
            let step = model
                .step_for(&frame(4).map_err(fail)?, duration)
                .map_err(fail)?;
            resting += step.core_counts.iter().sum::<usize>();
            remaining -= duration;
        }
        let step = model.step(&frame(image).map_err(fail)?).map_err(fail)?;
        let potentials: Vec<_> = model
            .processing_ids()
            .iter()
            .map(|&id| {
                f64::from(
                    model
                        .network()
                        .neuron(id)
                        .unwrap()
                        .numerical_state()
                        .membrane_potential,
                )
            })
            .collect();
        observations.push(
            json!({"image":image,"core_event_digest":fingerprint_debug(&step.core_events),
            "motor_event_digest":fingerprint_debug(&step.motor_events),
            "motor_counts":step.motor_counts,"keys":format!("{:?}",step.keys),
            "stored_potentials":potentials,"rest_core_spikes":resting}),
        );
        responses.push(step.core_counts);
    }
    if !learned_parameters_equal(&original, model.network()) {
        return Err(error("association recall changed weights"));
    }
    let mut result = summarize(&responses);
    result["observations"] = json!(observations);
    result["parameters_preserved"] = json!(true);
    Ok(result)
}
fn differs(a: &Value, b: &Value, key: &str) -> bool {
    a["observations"]
        .as_array()
        .unwrap()
        .iter()
        .zip(b["observations"].as_array().unwrap())
        .any(|(x, y)| x[key] != y[key])
}
pub(super) fn run_with_learning(
    c: &Config,
    seed: u64,
    group: &str,
    override_rule: Option<LearningRule>,
) -> Result<TrialMetrics> {
    AssociationScenario.validate(c)?;
    let mut trained = train_with_learning(c, seed, group, true, override_rule)?;
    let mut reset = train_with_learning(c, seed, group, false, override_rule)?;
    if !trained.model.same_replay_state(&reset.model) {
        return Err(error(
            "audited and unaudited association checkpoints differ",
        ));
    }
    let mut baseline = build(c, seed, group, override_rule)?;
    if fingerprint_debug(baseline.network()) != trained.initial_digest {
        return Err(error("association initial network differs"));
    }
    let before = recall(&mut baseline, c)?;
    verify_only_weights(&trained.model, &mut reset.model, &trained.weights)?;
    // Verify both directions: restoring learned weights must recover the full checkpoint.
    let learned: Vec<_> = trained
        .model
        .network()
        .synapses()
        .map(|s| (s.id(), s.weight().get()))
        .collect();
    apply(&mut reset.model, &learned)?;
    if !trained.model.same_replay_state(&reset.model) {
        return Err(error("non-weight state was changed"));
    }
    apply(&mut reset.model, &trained.weights)?;
    let after = recall(&mut trained.model, c)?;
    let without = recall(&mut reset.model, c)?;
    let margin = |phase: &Value, cue: usize| phase["completion_margins"][cue].as_f64().unwrap();
    let audit = &trained.details["audit"];
    let number = |key: &str| audit[key].as_f64().unwrap();
    let flag = |x: bool| if x { 1. } else { 0. };
    let values = vec![
        margin(&after, 0),
        margin(&after, 1),
        margin(&after, 0) - margin(&before, 0),
        margin(&after, 1) - margin(&before, 1),
        margin(&after, 0) - margin(&without, 0),
        margin(&after, 1) - margin(&without, 1),
        after["separation"].as_f64().unwrap(),
        flag(after["all_responses_active"] == true),
        trained.details["weight_changes"]["all"]["l1"]
            .as_f64()
            .unwrap(),
        trained.details["sensory_spikes"].as_f64().unwrap(),
        trained.details["sensor_to_core_arrivals"].as_f64().unwrap(),
        trained.details["core_spikes"].as_f64().unwrap(),
        number("requested_updates"),
        number("requests_without_change"),
        number("actual_absolute_mass"),
        number("zero_factor_integrations"),
        number("nonzero_motor_factor_samples"),
        flag(differs(&after, &without, "core_event_digest")),
        flag(differs(&after, &without, "motor_event_digest")),
        flag(differs(&after, &without, "stored_potentials")),
        1.,
    ];
    Ok(TrialMetrics {
        values: AssociationScenario
            .metric_names()
            .into_iter()
            .map(str::to_owned)
            .zip(values)
            .collect(),
        details: json!({"initial_network_digest":trained.initial_digest,"training":trained.details,
                "before":before,"learned_recall":after,"reset_recall":without,
                "matched_checkpoint":true,"only_weights_changed":true,
                "limits":"External cue-completion assay; stored potentials have native cell timestamps. Input/event fingerprints are noncryptographic. No labels, reward or artificial modulator pulses enter learning. No independent confirmation."}),
    })
}

impl Scenario for AssociationScenario {
    type Config = Config;
    fn name(&self) -> &'static str {
        "hippocampal_association"
    }
    fn default_config(&self) -> Config {
        Config {
            processing_neurons: 256,
            training_repetitions: 8,
            rest_us: 11_000_000,
        }
    }
    fn set_parameter(&self, c: &mut Config, name: &str, value: &Value) -> Result<()> {
        HippocampalScenario.set_parameter(c, name, value)
    }
    fn validate(&self, c: &Config) -> Result<()> {
        HippocampalScenario.validate(c)?;
        if c.processing_neurons > 256 || c.training_repetitions > 16 {
            return Err(error(
                "association budget: at most 256 cells and 16 repetitions",
            ));
        }
        Ok(())
    }
    fn snapshot(&self, c: &Config, seed: u64, i: &Intervention) -> Value {
        let setting = settings(&i.name).unwrap();
        json!({"protocol":"NERVA-Hippocampal-Association/1","processing_neurons":c.processing_neurons,
            "training_repetitions":c.training_repetitions,"rest_us":c.rest_us,
            "model":format!("{:?}",model_config(c, seed, if setting.1 {"smooth"} else {"no_chemistry"})),
            "learning_rule":format!("{:?}",setting.0),"chemistry":setting.1,"learning":setting.2,
            "unpaired":setting.3,"training_schedule":schedule(c,seed,setting.3),
            "training_passes_per_execution":2,"audit_rows_per_frame":100000,
            "audit_rows_total":1000000,"recall":"full_a/full_b/partial_a/partial_b; frozen matched weight reset"})
    }
    fn interventions(&self) -> Vec<Intervention> {
        GROUPS
            .iter()
            .map(|&name| Intervention { name: name.into() })
            .collect()
    }
    fn metric_names(&self) -> Vec<&'static str> {
        vec![
            "completion_a",
            "completion_b",
            "acquisition_a",
            "acquisition_b",
            "weight_gain_a",
            "weight_gain_b",
            "separation",
            "all_responses_active",
            "weight_l1",
            "sensory_spikes",
            "sensor_to_core_arrivals",
            "training_core_spikes",
            "requested_updates",
            "requests_without_change",
            "actual_update_mass",
            "zero_factor_integrations",
            "motor_factor_samples",
            "recall_core_effect",
            "recall_motor_effect",
            "recall_potential_effect",
            "matched_checkpoints",
        ]
    }
    fn run(&self, c: &Config, seed: u64, i: &Intervention) -> Result<TrialMetrics> {
        run_with_learning(c, seed, &i.name, None)
    }
    fn evaluate(&self, records: &[Value], replay: bool) -> Result<Value> {
        let mut paired = BTreeMap::<u64, BTreeMap<String, &Value>>::new();
        for row in records {
            let seed = row["job"]["seed"]
                .as_u64()
                .ok_or_else(|| error("missing association seed"))?;
            let group = row["job"]["intervention"]
                .as_str()
                .ok_or_else(|| error("missing association group"))?;
            if paired
                .entry(seed)
                .or_default()
                .insert(group.into(), row)
                .is_some()
            {
                return Err(error("duplicate association seed/control"));
            }
        }
        let all_controls = !paired.is_empty()
            && paired
                .values()
                .all(|groups| GROUPS.iter().all(|g| groups.contains_key(*g)));
        let mut contrasts = Vec::new();
        for (seed, groups) in paired {
            let digests: BTreeSet<_> = groups
                .values()
                .map(|r| r["details"]["initial_network_digest"].to_string())
                .collect();
            if digests.len() != 1 {
                return Err(error(
                    "association controls do not share initial neurons/weights",
                ));
            }
            let inputs: BTreeSet<_> = groups
                .iter()
                .filter(|(g, _)| !g.ends_with("unpaired"))
                .map(|(_, r)| r["details"]["training"]["input_digest"].to_string())
                .collect();
            if inputs.len() > 1 {
                return Err(error(
                    "paired association controls received different inputs",
                ));
            }
            for (priority, a, b) in [
                (1, "three_on", "three_off"),
                (2, "three_on", "pair_on"),
                (2, "three_off", "pair_off"),
                (3, "three_on", "frozen_on"),
                (3, "three_on", "three_unpaired"),
                (3, "pair_on", "pair_unpaired"),
            ] {
                if let (Some(a_row), Some(b_row)) = (groups.get(a), groups.get(b)) {
                    contrasts.push(json!({"seed":seed,"priority":priority,"a":a,"b":b,
                        "weight_gain_a":a_row["metrics"]["weight_gain_a"].as_f64().unwrap()-b_row["metrics"]["weight_gain_a"].as_f64().unwrap(),
                        "weight_gain_b":a_row["metrics"]["weight_gain_b"].as_f64().unwrap()-b_row["metrics"]["weight_gain_b"].as_f64().unwrap()}));
                }
            }
        }
        Ok(
            json!({"status":"exploratory_paired_association","passed":null,"replay_checked":replay,
            "all_controls": all_controls &&
                records.iter().all(|r|r["details"]["matched_checkpoint"]==true),
            "contrasts":contrasts,"functional_memory_demonstrated":false}),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unpaired_exposure_is_balanced_and_rules_start_with_identical_networks() {
        let c = Config {
            processing_neurons: 16,
            training_repetitions: 1,
            rest_us: 100000,
        };
        let expected = build(&c, 7, "three_on", None).unwrap();
        for group in GROUPS {
            assert_eq!(
                build(&c, 7, group, None).unwrap().network(),
                expected.network()
            );
        }
        let mut paired = schedule(&c, 7, false);
        let mut unpaired = schedule(&c, 7, true);
        assert_ne!(paired, unpaired);
        paired.sort();
        unpaired.sort();
        assert_eq!(paired, unpaired);
    }
    #[test]
    fn frozen_and_no_chemistry_controls_have_no_weight_effect_and_audits_replay() {
        let c = Config {
            processing_neurons: 16,
            training_repetitions: 1,
            rest_us: 100000,
        };
        for group in ["frozen_on", "three_off", "pair_on"] {
            let row = AssociationScenario
                .run_verified(&c, 7, &Intervention { name: group.into() })
                .unwrap();
            assert_eq!(row.details["training"]["audit"]["complete"], true);
            assert_eq!(row.values["matched_checkpoints"], 1.);
            if group != "pair_on" {
                assert_eq!(row.values["weight_l1"], 0.);
                assert_eq!(row.values["recall_core_effect"], 0.);
            }
        }
    }
}
