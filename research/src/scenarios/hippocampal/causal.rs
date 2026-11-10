//! Matched-state weight interventions, with native training-path observations.
//! Replays establish the same full model checkpoint; no neural state is reset.
use super::{
    Config, fail, frame, model_config, probes,
    weights::{snapshot_json, weight_snapshot},
};
use crate::{
    Result, error, fingerprint_debug,
    scenario::{Intervention, Scenario, TrialMetrics},
};
use nerva::{
    core::{NeuronRole, SynapseId},
    runtime::ObservationEvent,
};
use nerva_vm::model::{
    Model, PlasticitySettings, SensorConfig, SensorMode, SpikeCoding, audit::AuditOptions,
};
use serde_json::{Value, json};
use std::collections::BTreeMap;
mod audit;
use audit::TrainingAudit;

const GROUPS: [&str; 4] = ["cortical", "smooth", "no_chemistry", "frozen"];
const GAIN: f64 = 100.;
pub struct CausalScenario;
struct Checkpoint {
    model: Model,
    initial: Vec<(SynapseId, f32)>,
    details: Value,
}
fn checkpoint(c: &Config, seed: u64, group: &str, record: bool) -> Result<Checkpoint> {
    let mut model = Model::with_sensor(
        model_config(c, seed, group),
        SensorConfig {
            mode: SensorMode::Pixels,
            resolution: 8,
            coding: SpikeCoding::Deterministic,
            max_rate_hz: 120.,
        },
    )
    .map_err(fail)?;
    let initial_digest = fingerprint_debug(model.network());
    let before = probes(&mut model, c)?;
    let weights_before = snapshot_json(&weight_snapshot(&model));
    let initial = model
        .network()
        .synapses()
        .map(|s| (s.id(), s.weight().get()))
        .collect();
    let initial_signals: Vec<_> = model
        .network()
        .neurons()
        .map(|n| (n.id().0, f64::from(n.modulation().learning_signal)))
        .collect();
    let start = model.next_observation_us();
    model
        .set_plasticity(PlasticitySettings {
            learning: group != "frozen",
            development: false,
            homeostasis: false,
        })
        .map_err(fail)?;
    if record {
        model.enable_learning_audit(100_000).map_err(fail)?;
    }
    let mut observations = TrainingAudit::default();
    let mut updates = 0;
    let mut core_spikes = 0;
    let mut schedule = Vec::new();
    for trial in 0..c.training_repetitions {
        let order = if (seed ^ trial as u64) & 1 == 0 {
            [0, 1]
        } else {
            [1, 0]
        };
        for kind in order {
            schedule.push(kind);
            for image in [frame(kind).map_err(fail)?, frame(4).map_err(fail)?] {
                let step = model.step(&image).map_err(fail)?;
                updates += step.weight_changes;
                core_spikes += step.core_counts.iter().sum::<usize>();
                if record {
                    observations.add(
                        step.start_us,
                        step.end_us,
                        model.take_learning_audit().map_err(fail)?,
                        step.weight_changes,
                    )?;
                }
            }
        }
    }
    if record {
        let pending = model.disable_learning_audit().map_err(fail)?;
        if !pending.events.is_empty() || !pending.delayed.is_empty() {
            return Err(error("undrained learning observations"));
        }
    }
    let end = model.next_observation_us();
    let weights_after = snapshot_json(&weight_snapshot(&model));
    let nodes:Vec<_>=model.network().neurons().map(|n|json!({"id":n.id().0,"role":format!("{:?}",n.role()),"polarity":format!("{:?}",n.polarity()),"position":[n.position().x,n.position().y,n.position().z],
        "receptors":[n.receptor_profile().dopamine_d1_like,n.receptor_profile().dopamine_d2_like,n.receptor_profile().acetylcholine,n.receptor_profile().noradrenaline]})).collect();
    model.freeze().map_err(fail)?;
    Ok(Checkpoint {
        model,
        initial,
        details: json!({"initial_network_digest":initial_digest,"before":before,"training_schedule":schedule,
        "weights_before":weights_before,"weights_after":weights_after,"initial_learning_signals":initial_signals,
        "start_us":start,"end_us":end,"weight_events":updates,"core_spikes":core_spikes,"neurons":nodes,"audit":if record {observations.json()} else {Value::Null}}),
    })
}
fn weights(model: &Model) -> Vec<(SynapseId, f32)> {
    model
        .network()
        .synapses()
        .map(|s| (s.id(), s.weight().get()))
        .collect()
}
fn apply(model: &mut Model, values: &[(SynapseId, f32)]) -> Result<()> {
    for &(id, w) in values {
        if model
            .network()
            .synapse(id)
            .ok_or_else(|| error("intervention edge missing"))?
            .weight()
            .get()
            != w
        {
            model.diagnostic_synapse_weight(id, w).map_err(fail)?;
        }
    }
    Ok(())
}
fn verify_only_weights(
    reference: &Model,
    branch: &mut Model,
    desired: &[(SynapseId, f32)],
) -> Result<()> {
    apply(branch, &weights(reference))?;
    if !reference.same_replay_state(branch) {
        return Err(error("counterfactual changed non-weight replay state"));
    }
    apply(branch, desired)
}
#[derive(Default)]
struct Response {
    frames: Vec<Value>,
    core_events: Vec<(u64, u64)>,
    motor_events: Vec<(u64, u64)>,
    arrivals: Vec<(u64, u64, f64)>,
    potentials: Vec<Vec<(u64, f64)>>,
}
impl Response {
    fn json(&self) -> Value {
        json!({"frames":self.frames,"core_events":self.core_events,"motor_events":self.motor_events,"arrivals":self.arrivals,"potentials":self.potentials})
    }
}
fn observe(
    model: &mut Model,
    image: usize,
    duration: u64,
    out: &mut Response,
    is_probe: bool,
) -> Result<()> {
    let step = model
        .step_for(&frame(image).map_err(fail)?, duration)
        .map_err(fail)?;
    let audit = model
        .take_audit()
        .ok_or_else(|| error("probe audit absent"))?;
    for e in audit.raw_events {
        match e {
            ObservationEvent::SpikeEmitted(s) => {
                match model.network().neuron(s.neuron_id).unwrap().role() {
                    Some(NeuronRole::Processing) => out.core_events.push((s.time.0, s.neuron_id.0)),
                    Some(NeuronRole::Motor) => out.motor_events.push((s.time.0, s.neuron_id.0)),
                    _ => {}
                }
            }
            ObservationEvent::SynapticArrival {
                time,
                synapse_id,
                target,
                amplitude,
                ..
            } if matches!(
                model.network().neuron(target).unwrap().role(),
                Some(NeuronRole::Processing | NeuronRole::Motor)
            ) =>
            {
                out.arrivals
                    .push((time.0, synapse_id.0, f64::from(amplitude)));
            }
            _ => {}
        }
    }
    if is_probe {
        out.frames.push(json!({"kind":image,"start_us":step.start_us,"end_us":step.end_us,"core_counts":step.core_counts,"motor_counts":step.motor_counts,"keys":step.keys.iter().map(|k|format!("{k:?}")).collect::<Vec<_>>()}));
        out.potentials.push(
            model
                .network()
                .neurons()
                .filter(|n| matches!(n.role(), Some(NeuronRole::Processing | NeuronRole::Motor)))
                .map(|n| (n.id().0, f64::from(n.numerical_state().membrane_potential)))
                .collect(),
        );
    }
    Ok(())
}
fn run_probes(model: &mut Model, c: &Config, rest: bool) -> Result<Response> {
    model.enable_audit(AuditOptions {
        raw_events: true,
        input_spikes: false,
    });
    let mut out = Response::default();
    for image in 0..4 {
        let mut remaining = if rest { c.rest_us } else { 0 };
        while remaining > 0 {
            let d = remaining.min(50_000);
            observe(model, 4, d, &mut out, false)?;
            remaining -= d;
        }
        observe(model, image, 50_000, &mut out, true)?;
    }
    Ok(out)
}
fn comparison(a: &Response, b: &Response) -> Value {
    let core_counts = a
        .frames
        .iter()
        .map(|v| v["core_counts"].clone())
        .collect::<Vec<_>>()
        != b.frames
            .iter()
            .map(|v| v["core_counts"].clone())
            .collect::<Vec<_>>();
    let motor_counts = a
        .frames
        .iter()
        .map(|v| v["motor_counts"].clone())
        .collect::<Vec<_>>()
        != b.frames
            .iter()
            .map(|v| v["motor_counts"].clone())
            .collect::<Vec<_>>();
    json!({"core_counts_differ":core_counts,"motor_counts_differ":motor_counts,"core_spike_times_differ":a.core_events!=b.core_events,"motor_spike_times_differ":a.motor_events!=b.motor_events,
        "arrivals_differ":a.arrivals!=b.arrivals,"sampled_potentials_differ":a.potentials!=b.potentials,"keys_differ":a.frames.iter().map(|v|v["keys"].clone()).collect::<Vec<_>>()!=b.frames.iter().map(|v|v["keys"].clone()).collect::<Vec<_>>()})
}
fn contrast(c: &Config, seed: u64, group: &str, rest: bool) -> Result<(Value, Value)> {
    let mut trained = checkpoint(c, seed, group, rest)?;
    let mut reset = checkpoint(c, seed, group, false)?;
    let mut amplified = checkpoint(c, seed, group, false)?;
    if !trained.model.same_replay_state(&reset.model)
        || !trained.model.same_replay_state(&amplified.model)
    {
        return Err(error("deterministically reconstructed checkpoints differ"));
    }
    let learned = weights(&trained.model);
    verify_only_weights(&trained.model, &mut reset.model, &trained.initial)?;
    let target: Vec<_> = learned
        .iter()
        .zip(&trained.initial)
        .map(|(&(id, w), &(original_id, w0))| {
            assert_eq!(id, original_id);
            (
                id,
                (f64::from(w0) + GAIN * (f64::from(w) - f64::from(w0))).clamp(0., 2.) as f32,
            )
        })
        .collect();
    let clipped = learned
        .iter()
        .zip(&trained.initial)
        .filter(|((_, w), (_, w0))| {
            !(0. ..=2.).contains(&(f64::from(*w0) + GAIN * (f64::from(*w) - f64::from(*w0))))
        })
        .count();
    let a = run_probes(&mut trained.model, c, rest)?;
    let b = run_probes(&mut reset.model, c, rest)?;
    let native = comparison(&a, &b);
    let spike_effect =
        native["core_spike_times_differ"] == true || native["motor_spike_times_differ"] == true;
    let amp = if !spike_effect {
        // This held branch has not advanced since the identical trained checkpoint.
        apply(&mut amplified.model, &target)?;
        let x = run_probes(&mut amplified.model, c, rest)?;
        json!({"executed":true,"gain":GAIN,"clipped_edges":clipped,"target_weights":target.iter().map(|(id,w)|(id.0,f64::from(*w))).collect::<Vec<_>>(),"versus_original":comparison(&x,&b),"response":x.json(),"diagnostic_only":true})
    } else {
        json!({"executed":false,"reason":"native weight contrast already changes spikes","diagnostic_only":true})
    };
    Ok((
        json!({"matched_checkpoint":true,"only_weights_changed":true,"pending_arrival_amplitudes_preserved":true,"probe_rest_us":if rest {c.rest_us} else {0},
        "trained":a.json(),"original_weights":b.json(),"native":native,"amplified":amp}),
        trained.details,
    ))
}
impl Scenario for CausalScenario {
    type Config = Config;
    fn name(&self) -> &'static str {
        "hippocampal_causality"
    }
    fn default_config(&self) -> Config {
        Config {
            processing_neurons: 256,
            training_repetitions: 8,
            rest_us: 11_000_000,
        }
    }
    fn set_parameter(&self, c: &mut Config, name: &str, v: &Value) -> Result<()> {
        super::HippocampalScenario.set_parameter(c, name, v)
    }
    fn validate(&self, c: &Config) -> Result<()> {
        super::HippocampalScenario.validate(c)?;
        if c.processing_neurons > 256 || c.training_repetitions > 8 {
            return Err(error("causal probe is bounded to256cells and8repetitions"));
        }
        Ok(())
    }
    fn snapshot(&self, c: &Config, seed: u64, i: &Intervention) -> Value {
        json!({"protocol":"NERVA-Hippocampal-Causality/1","base":super::HippocampalScenario.snapshot(c,seed,i),"gain":GAIN,"probe_modes":["original_rest","immediate_sequence"],"checkpoint_method":"deterministic reconstruction with full Model::same_replay_state checks","learning_audit_rows_per_frame":100000})
    }
    fn interventions(&self) -> Vec<Intervention> {
        GROUPS
            .iter()
            .map(|&name| Intervention { name: name.into() })
            .collect()
    }
    fn metric_names(&self) -> Vec<&'static str> {
        vec![
            "native_core_spike_effect",
            "native_motor_spike_effect",
            "native_key_effect",
            "amplified_spike_effect",
            "matched_checkpoints",
        ]
    }
    fn run(&self, c: &Config, seed: u64, i: &Intervention) -> Result<TrialMetrics> {
        self.validate(c)?;
        if !GROUPS.contains(&i.name.as_str()) {
            return Err(error("unknown causal control"));
        }
        let (standard, training) = contrast(c, seed, &i.name, true)?;
        let (immediate, _) = contrast(c, seed, &i.name, false)?;
        let modes = [&standard, &immediate];
        let flag = |field: &str| {
            if modes.iter().any(|m| m["native"][field] == true) {
                1.
            } else {
                0.
            }
        };
        let amp = modes.iter().any(|m| {
            m["amplified"]["executed"] == true
                && (m["amplified"]["versus_original"]["core_spike_times_differ"] == true
                    || m["amplified"]["versus_original"]["motor_spike_times_differ"] == true)
        });
        let values = [
            flag("core_spike_times_differ"),
            flag("motor_spike_times_differ"),
            flag("keys_differ"),
            if amp { 1. } else { 0. },
            1.,
        ];
        Ok(TrialMetrics {
            values: self
                .metric_names()
                .into_iter()
                .map(str::to_owned)
                .zip(values)
                .collect::<BTreeMap<_, _>>(),
            details: json!({"training":training,"standard":standard,"immediate":immediate,"limits":"Matched weight intervention, not a memory or directional-learning success. Probe modes share their own checkpoint; later frames follow the same input sequence but may diverge causally. Amplification is fixed100x and diagnostic only."}),
        })
    }
    fn evaluate(&self, records: &[Value], replay: bool) -> Result<Value> {
        Ok(
            json!({"status":"causal_weight_diagnosis","records":records.len(),"returned_replay_checked":replay,"functional_memory_demonstrated":false}),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn matched_counterfactuals_replay_and_no_chemistry_changes_no_weights() {
        let c = Config {
            processing_neurons: 16,
            training_repetitions: 1,
            rest_us: 100_000,
        };
        let row = CausalScenario
            .run_verified(
                &c,
                42,
                &Intervention {
                    name: "no_chemistry".into(),
                },
            )
            .unwrap();
        assert_eq!(row.values["matched_checkpoints"], 1.);
        assert_eq!(row.values["native_core_spike_effect"], 0.);
        assert_eq!(row.values["native_motor_spike_effect"], 0.);
        assert_eq!(row.values["amplified_spike_effect"], 0.);
        assert_eq!(row.details["standard"]["only_weights_changed"], true);
    }
    #[test]
    fn active_chemical_counterfactual_exports_complete_training_chain() {
        let c = Config {
            processing_neurons: 32,
            training_repetitions: 1,
            rest_us: 100_000,
        };
        let row = CausalScenario
            .run(
                &c,
                42,
                &Intervention {
                    name: "smooth".into(),
                },
            )
            .unwrap();
        assert_eq!(row.details["training"]["audit"]["complete"], true);
        assert!(
            !row.details["training"]["audit"]["chemicals"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        assert!(
            !row.details["training"]["audit"]["traces"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        for mode in ["standard", "immediate"] {
            assert_eq!(row.details[mode]["matched_checkpoint"], true);
            assert_eq!(row.details[mode]["only_weights_changed"], true);
        }
    }
}
