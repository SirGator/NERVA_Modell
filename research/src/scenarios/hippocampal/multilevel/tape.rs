//! Local plasticity replay; never invokes a network simulation or propagates a spike.
use crate::{Result, error};
use nerva::{
    config::LearningConfig,
    core::{Network, NeuronId, Polarity, SimTime, SynapseId},
    learning::{PairStdp, PlasticityRule, ThreeFactorConfig, WeightUpdateMode},
    runtime::{LearningAuditEvent, LearningAuditLog, ObservationEvent},
};
use nerva_vm::model::{Model, audit::InputAudit};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

const LIMIT: usize = 2_000_000;
#[derive(Default)]
pub(in crate::scenarios::hippocampal) struct Tape {
    initial: Option<Network>,
    pub frames: Vec<(InputAudit, LearningAuditLog)>,
    rows: usize,
}
impl Tape {
    pub fn initialize(&mut self, model: &Model) {
        self.initial = Some(model.network().clone());
    }
    pub fn add(&mut self, input: &InputAudit, log: &LearningAuditLog) -> Result<()> {
        self.rows +=
            input.raw_events.len() + log.events.len() + log.delayed.len() + log.boundaries.len();
        if !log.is_complete() || self.rows > LIMIT {
            return Err(error("incomplete or over-budget plasticity tape"));
        }
        self.frames.push((input.clone(), log.clone()));
        Ok(())
    }
    pub fn history(
        &self,
        c: &super::super::Config,
        seed: u64,
        group: &str,
        rule: nerva_vm::model::LearningRule,
        model: &Model,
    ) -> Result<Value> {
        let mut candidates: Vec<_> = model
            .network()
            .synapses()
            .map(|s| {
                let original = self
                    .initial
                    .as_ref()
                    .unwrap()
                    .synapse(s.id())
                    .unwrap()
                    .weight()
                    .get();
                (
                    s.id().0,
                    (f64::from(s.weight().get()) - f64::from(original)).abs(),
                )
            })
            .collect();
        candidates.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
        let initial = super::super::association::build(c, seed, group, Some(rule))?;
        self.history_for_initial(
            &initial,
            model,
            &candidates.iter().take(8).map(|e| e.0).collect::<Vec<_>>(),
            250000,
        )
    }
    pub fn history_for_initial(
        &self,
        initial: &Model,
        model: &Model,
        selected: &[u64],
        max_rows: usize,
    ) -> Result<Value> {
        use crate::scenario::Scenario;
        let mut config = super::super::history::HistoryScenario.default_config();
        super::super::history::HistoryScenario.set_parameter(
            &mut config,
            "history.synapses",
            &json!(selected),
        )?;
        super::super::history::HistoryScenario.set_parameter(
            &mut config,
            "history.max_rows",
            &json!(max_rows),
        )?;
        let mut history = super::super::history::History::new(&config);
        history.initialize(initial)?;
        for (input, log) in &self.frames {
            history.add("training", input, Some(log), model)?;
        }
        history.checkpoint(model);
        Ok(history.finish())
    }
    pub fn replay(&self, reference: &Model) -> Result<ReplayResult> {
        self.replay_variants(reference, false)
    }
    pub fn replay_mld02(&self, reference: &Model) -> Result<ReplayResult> {
        self.replay_variants(reference, true)
    }
    fn replay_variants(&self, reference: &Model, extended: bool) -> Result<ReplayResult> {
        let initial = self
            .initial
            .as_ref()
            .ok_or_else(|| error("missing tape initial state"))?;
        let mut kinds = vec!["original", "constant", "pair"];
        if extended {
            kinds.extend(["gain64", "positive", "negative"]);
        }
        let mut locals = kinds
            .iter()
            .map(|&name| (name, Local::new(initial.clone(), name)))
            .collect::<BTreeMap<_, _>>();
        let mut clocks = Vec::new();
        let mut chemicals = Vec::new();
        let mut arrivals = Vec::new();
        let mut spikes = Vec::new();
        for (input, log) in &self.frames {
            let mut times = log.boundaries.iter().map(|t| t.0).collect::<BTreeSet<_>>();
            for event in &input.raw_events {
                match event {
                    ObservationEvent::SynapticArrival { time, .. } => {
                        times.insert(time.0);
                    }
                    ObservationEvent::SpikeEmitted(s) => {
                        times.insert(s.time.0);
                    }
                    _ => {}
                }
            }
            let boundaries = log.boundaries.iter().map(|t| t.0).collect::<BTreeSet<_>>();
            require(
                times == boundaries,
                "tape activity lacks an integration boundary",
            )?;
            let mut chemical_by_time = BTreeMap::<u64, Vec<_>>::new();
            for event in &log.events {
                if let LearningAuditEvent::ChemicalSample {
                    time,
                    neuron,
                    concentrations,
                    response,
                } = event
                {
                    chemical_by_time
                        .entry(time.0)
                        .or_default()
                        .push((*neuron, *response));
                    chemicals.push(json!([
                        time.0,
                        neuron.0,
                        concentrations.dopamine,
                        concentrations.acetylcholine,
                        concentrations.noradrenaline,
                        response.learning_signal,
                        response.threshold_shift,
                        response.intrinsic_drive,
                        response.transmission_gain
                    ]));
                }
            }
            let mut activity = BTreeMap::<u64, Vec<&ObservationEvent>>::new();
            for event in &input.raw_events {
                let time = match event {
                    ObservationEvent::SynapticArrival { time, .. } => time.0,
                    ObservationEvent::SpikeEmitted(s) => s.time.0,
                    _ => continue,
                };
                activity.entry(time).or_default().push(event);
            }
            for time in times {
                clocks.push(time);
                for local in locals.values_mut() {
                    local.clock(SimTime(time));
                    for &(id, response) in chemical_by_time.get(&time).into_iter().flatten() {
                        local
                            .network
                            .neuron_mut(id)
                            .unwrap()
                            .set_modulation(SimTime(time), response)
                            .map_err(super::super::fail)?;
                    }
                    for event in activity.get(&time).into_iter().flatten() {
                        match event {
                            ObservationEvent::SynapticArrival { synapse_id, .. } => {
                                local.pre(*synapse_id, SimTime(time))
                            }
                            ObservationEvent::SpikeEmitted(s) => local.post(s.neuron_id, s.time),
                            _ => {}
                        }
                    }
                }
                for event in activity.get(&time).into_iter().flatten() {
                    match event {
                        ObservationEvent::SynapticArrival { synapse_id, .. } => {
                            arrivals.push(json!([time, synapse_id.0]))
                        }
                        ObservationEvent::SpikeEmitted(s) => {
                            spikes.push(json!([time, s.neuron_id.0]))
                        }
                        _ => {}
                    }
                }
            }
        }
        let original = &locals["original"];
        let constant = &locals["constant"];
        let pair = &locals["pair"];
        for s in original.network.synapses() {
            let native = reference.network().synapse(s.id()).unwrap();
            require(
                s.weight() == native.weight()
                    && s.learning_weight_residual() == native.learning_weight_residual()
                    && s.learning_eligibility() == native.learning_eligibility()
                    && s.pre_trace() == native.pre_trace(),
                "fixed-history original rule does not reproduce native local state",
            )?;
        }
        let mut json = json!({"verified_original_local_state":true,"scope":"Frozen recorded history; local hooks only, no network stepping. Other rules may generate different spike histories when trained live.",
            "clock_times_us":clocks,"chemical_fields":["time_us","neuron_id","dopamine","acetylcholine","noradrenaline","native_signal","threshold_shift","intrinsic_drive","transmission_gain"],
            "chemical_samples":chemicals,"pre_arrivals":arrivals,"post_spikes":spikes,
            "phase_order":"At each exact batch: delayed integration under old held chemistry, chemical refresh, arrivals, post-spike hooks. Order within each activity phase is native.",
            "original":original.json(initial),"constant_positive":constant.json(initial),"immediate_pair":pair.json(initial)});
        if extended {
            for key in ["gain64", "positive", "negative"] {
                json[key] = locals[key].json(initial);
            }
        }
        Ok(ReplayResult {
            json,
            constant: constant.weights(),
            pair: pair.weights(),
        })
    }
}
fn require(ok: bool, message: &str) -> Result<()> {
    if ok { Ok(()) } else { Err(error(message)) }
}
pub(in crate::scenarios::hippocampal) struct ReplayResult {
    pub json: Value,
    pub constant: Vec<(SynapseId, f32)>,
    pub pair: Vec<(SynapseId, f32)>,
}
#[derive(Default)]
struct Ledger {
    positive: f64,
    negative: f64,
    requested_positive: f64,
    requested_negative: f64,
    positive_integral: f64,
    negative_integral: f64,
    changes: usize,
    blocked_positive: f64,
    blocked_negative: f64,
}
struct Local {
    network: Network,
    rule: PairStdp,
    ledger: BTreeMap<SynapseId, Ledger>,
}
impl Local {
    fn new(network: Network, kind: &str) -> Self {
        let rule = PairStdp::new(&LearningConfig {
            a_plus: 0.0001,
            a_minus: 0.00012,
            min_weight: 0.,
            max_weight: 2.,
            ..Default::default()
        });
        let rule = if kind == "pair" {
            rule
        } else {
            rule.with_three_factor(ThreeFactorConfig::default())
                .unwrap()
                .with_weight_update_mode(WeightUpdateMode::Accumulated)
        };
        let rule = match kind {
            "gain64" => rule.with_learning_signal_gain(64.).unwrap(),
            "positive" => {
                rule.with_contribution_filter(nerva::learning::WeightContribution::PotentiatingOnly)
            }
            "negative" => {
                rule.with_contribution_filter(nerva::learning::WeightContribution::DepressingOnly)
            }
            _ => rule,
        };
        let rule = if kind == "constant" {
            rule.with_learning_signal_override(1.).unwrap()
        } else {
            rule
        };
        Self {
            network,
            rule,
            ledger: BTreeMap::new(),
        }
    }
    fn delta(&mut self, id: SynapseId, old: f32) {
        let delta = f64::from(self.network.synapse(id).unwrap().weight().get()) - f64::from(old);
        if delta != 0. {
            let row = self.ledger.entry(id).or_default();
            row.changes += 1;
            row.positive += delta.max(0.);
            row.negative += delta.min(0.);
        }
    }
    fn clock(&mut self, time: SimTime) {
        if !self.rule.needs_time_updates() {
            return;
        }
        let ids = self.network.synapse_ids().collect::<Vec<_>>();
        for id in ids {
            let s = self.network.synapse(id).unwrap();
            let post = self.network.neuron(s.post()).unwrap().clone();
            let enabled = self.network.neuron(s.pre()).unwrap().polarity() == Polarity::Excitatory;
            let update = self
                .rule
                .delayed_update_diagnostics(s, &post, time, enabled)
                .unwrap();
            if update.tag_integral_seconds != 0. {
                let row = self.ledger.entry(id).or_default();
                row.blocked_positive += (update.unfiltered_delta - update.unrounded_delta).max(0.);
                row.blocked_negative += (update.unfiltered_delta - update.unrounded_delta).min(0.);
                row.requested_positive += update.unrounded_delta.max(0.);
                row.requested_negative += update.unrounded_delta.min(0.);
                row.positive_integral += update.tag_integral_seconds.max(0.);
                row.negative_integral += update.tag_integral_seconds.min(0.);
            }
            let old = s.weight().get();
            self.rule
                .advance_time(self.network.synapse_mut(id).unwrap(), &post, time, enabled);
            self.delta(id, old);
        }
    }
    fn pre(&mut self, id: SynapseId, time: SimTime) {
        let s = self.network.synapse(id).unwrap();
        if !self
            .rule
            .accepts(s, self.network.neuron(s.pre()).unwrap().polarity())
        {
            return;
        }
        let post = self.network.neuron(s.post()).unwrap().clone();
        let old = s.weight().get();
        self.rule
            .on_pre_arrival(self.network.synapse_mut(id).unwrap(), &post, time);
        self.delta(id, old);
    }
    fn post(&mut self, id: NeuronId, time: SimTime) {
        let neuron = self.network.neuron(id).unwrap().clone();
        let mut incoming = self
            .network
            .incoming_synapse_ids(id)
            .iter()
            .filter_map(|sid| {
                let s = self.network.synapse(*sid).unwrap();
                self.rule
                    .accepts(s, self.network.neuron(s.pre()).unwrap().polarity())
                    .then(|| s.clone())
            })
            .collect::<Vec<_>>();
        self.rule.on_post_spike(&neuron, &mut incoming, time);
        for s in incoming {
            let old = self.network.synapse(s.id()).unwrap().weight().get();
            let sid = s.id();
            *self.network.synapse_mut(sid).unwrap() = s;
            self.delta(sid, old);
        }
    }
    fn weights(&self) -> Vec<(SynapseId, f32)> {
        self.network
            .synapses()
            .map(|s| (s.id(), s.weight().get()))
            .collect()
    }
    fn json(&self, initial: &Network) -> Value {
        let edges=self.network.synapses().map(|s| {
            let empty=Ledger::default();let l=self.ledger.get(&s.id()).unwrap_or(&empty);
            json!({"synapse_id":s.id().0,"source_id":s.pre().0,"target_id":s.post().0,"initial_weight":initial.synapse(s.id()).unwrap().weight().get(),
                "plastic":s.is_plastic(),"enabled":s.is_enabled(),"pre_excitatory":self.network.neuron(s.pre()).unwrap().polarity()==Polarity::Excitatory,"weight":s.weight().get(),"residual":s.learning_weight_residual(),"eligibility":s.learning_eligibility(),
                "positive_blocked":l.blocked_positive,"negative_blocked":l.blocked_negative,"positive_actual":l.positive,"negative_actual":l.negative,"weight_changes":l.changes,
                "positive_requested":if self.rule.needs_time_updates(){Some(l.requested_positive)}else{None},"negative_requested":if self.rule.needs_time_updates(){Some(l.requested_negative)}else{None},
                "positive_tag_integral":l.positive_integral,"negative_tag_integral":l.negative_integral})
        }).collect::<Vec<_>>();
        json!({"synapses":edges})
    }
}
