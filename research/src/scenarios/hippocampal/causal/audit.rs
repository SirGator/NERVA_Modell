//! Lossless bounded training observations: pair/weight events, chemical samples,
//! and exact per-synapse sums of every delayed integration within each frame.
use crate::{Result, error};
use nerva::runtime::{DelayedLearningSummary, LearningAuditEvent, LearningAuditLog, LearningPhase};
use serde_json::{Value, json};

#[derive(Default)]
pub(super) struct TrainingAudit {
    traces: Vec<Value>,
    chemicals: Vec<Value>,
    windows: Vec<Value>,
    rows: usize,
    weight_events: usize,
}
fn summary(s: &DelayedLearningSummary) -> Value {
    json!({"integrations":s.integrations,"first_us":s.first_us,"last_us":s.last_us,
        "peak_abs_tag":f64::from(s.peak_abs_tag),
        "positive_tag_integral":s.positive_tag_integral,"negative_tag_integral":s.negative_tag_integral,
        "positive_requested":s.positive_requested,"negative_requested":s.negative_requested,
        "positive_requested_no_change":s.positive_requested_no_change,"negative_requested_no_change":s.negative_requested_no_change,
        "max_abs_requested":s.max_abs_requested,"min_weight":s.min_weight.map(f64::from),"max_weight":s.max_weight.map(f64::from),
        "positive_actual":s.positive_actual,"negative_actual":s.negative_actual,"actual_changes":s.actual_changes,
        "nonzero_requested":s.nonzero_requested,"nonzero_requested_without_weight_change":s.nonzero_requested_without_weight_change,
        "zero_signal_integrations":s.zero_signal_integrations,"min_signal":s.min_signal.map(f64::from),"max_signal":s.max_signal.map(f64::from),
        "first_nonzero_signal_us":s.first_nonzero_signal_us,"last_nonzero_signal_us":s.last_nonzero_signal_us})
}
impl TrainingAudit {
    pub fn add(
        &mut self,
        start: u64,
        end: u64,
        log: LearningAuditLog,
        weight_events: usize,
    ) -> Result<()> {
        if !log.is_complete() {
            return Err(error("incomplete library learning audit"));
        }
        self.rows += log.events.len() + log.delayed.len();
        if self.rows > 1_000_000 {
            return Err(error(
                "causal training audit exceeds fixed one-million-row budget",
            ));
        }
        let mut observed = 0;
        for e in log.events {
            match e {
                LearningAuditEvent::Trace {
                    time,
                    synapse,
                    phase,
                    tag_before,
                    tag_after,
                    tag_anchor_before,
                    held_signal,
                    old_weight,
                    new_weight,
                    delayed,
                } => {
                    observed += usize::from(old_weight != new_weight);
                    let phase = match phase {
                        LearningPhase::Delayed => 0,
                        LearningPhase::PreArrival => 1,
                        LearningPhase::PostSpike => 2,
                    };
                    self.traces.push(json!([
                        time.0,
                        synapse.0,
                        phase,
                        f64::from(tag_before),
                        f64::from(tag_after),
                        tag_anchor_before.map(|v| v.0),
                        f64::from(held_signal),
                        f64::from(old_weight.get()),
                        f64::from(new_weight.get()),
                        delayed.map(|u| json!([
                            u.apply_weight,
                            u.tag_integral_seconds,
                            u.unrounded_delta
                        ]))
                    ]));
                }
                LearningAuditEvent::ChemicalSample {
                    time,
                    neuron,
                    concentrations,
                    response,
                } => {
                    self.chemicals.push(json!([
                        time.0,
                        neuron.0,
                        f64::from(concentrations.dopamine),
                        f64::from(concentrations.acetylcholine),
                        f64::from(concentrations.noradrenaline),
                        f64::from(response.learning_signal),
                        f64::from(response.threshold_shift),
                        f64::from(response.intrinsic_drive),
                        f64::from(response.transmission_gain)
                    ]));
                }
            }
        }
        if observed != weight_events {
            return Err(error(
                "learning audit omitted or duplicated a native weight event",
            ));
        }
        self.weight_events += observed;
        let rows: Vec<_> = log
            .delayed
            .into_iter()
            .map(|(id, s)| json!([id.0, summary(&s)]))
            .collect();
        self.windows
            .push(json!({"start_us":start,"end_us":end,"delayed":rows}));
        Ok(())
    }
    pub fn json(self) -> Value {
        json!({"complete":true,"rows":self.rows,"native_weight_events":self.weight_events,
            "trace_fields":["time_us","synapse_id","phase_0_delayed_1_pre_2_post","tag_before","tag_after","anchor_before_us","held_signal","weight_before","weight_after","optional_delayed_apply_integral_request"],
            "chemical_fields":["time_us","neuron_id","dopamine","acetylcholine","noradrenaline","held_learning_signal","threshold_shift","intrinsic_drive","transmission_gain"],
            "traces":self.traces,"chemicals":self.chemicals,"windows":self.windows,
            "limits":"Actual weight changes and pair-tag hooks retained; nonchanging delayed integrations are aggregated exactly per synapse/frame, not stored as individual raw rows."})
    }
}
