//! Optional, read-only diagnostics for model inputs and exact NERVA events.
use nerva::{
    core::{Network, NeuronId, SynapseId},
    runtime::ObservationEvent,
    transduction::ChannelSpike,
};
use nerva_qemu::signals::ChannelRate;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Default)]
pub struct AuditOptions {
    pub raw_events: bool,
    pub input_spikes: bool,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct NetworkSummary {
    pub plastic_synapses: usize,
    pub neurons: usize,
    pub synapses: usize,
    pub active_pre_traces: usize,
    pub pre_trace_sum: f64,
    pub pre_trace_max: f32,
    pub weights_at_zero: usize,
    pub weights_at_upper_bound: usize,
    pub weight_sum: f64,
    pub utility_sum: f64,
    pub eligibility_sum: f64,
}
impl NetworkSummary {
    pub fn read(network: &Network, upper: f32) -> Self {
        let mut result = Self {
            neurons: network.neuron_count(),
            synapses: network.synapse_count(),
            ..Self::default()
        };
        for synapse in network.synapses() {
            let w = synapse.weight().get();
            if synapse.is_plastic() {
                result.plastic_synapses += 1;
                result.weights_at_zero += usize::from(w == 0.0);
                result.weights_at_upper_bound += usize::from(w >= upper);
            }
            result.weight_sum += f64::from(w);
            result.active_pre_traces += usize::from(synapse.pre_trace() > 0.0);
            result.pre_trace_sum += f64::from(synapse.pre_trace());
            result.pre_trace_max = result.pre_trace_max.max(synapse.pre_trace());
            result.utility_sum += f64::from(synapse.utility());
            result.eligibility_sum += f64::from(synapse.utility_eligibility());
        }
        result
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ArrivalSummary {
    pub arrivals: usize,
    pub nonzero_arrivals: usize,
    pub refractory_arrivals: usize,
    pub amplitude_sum: f64,
    pub absolute_amplitude_sum: f64,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PairDeltas {
    pub count: usize,
    pub histogram: [usize; 11],
    pub min_us: Option<i64>,
    pub max_us: Option<i64>,
    pub sum_us: i128,
    pub simultaneous: usize,
    pub within_20ms: usize,
    pub within_100ms: usize,
}
impl PairDeltas {
    fn record(&mut self, post: u64, pre: u64) {
        let dt = i128::from(post) - i128::from(pre);
        let dt = i64::try_from(dt).expect("model observation time fits signed delta");
        self.count += 1;
        let bounds = [
            -100_000, -20_000, -8_000, -2_000, -1, 0, 2_000, 8_000, 20_000, 100_000,
        ];
        self.histogram[bounds.partition_point(|&bound| dt > bound)] += 1;
        self.sum_us += i128::from(dt);
        self.min_us = Some(self.min_us.map_or(dt, |old| old.min(dt)));
        self.max_us = Some(self.max_us.map_or(dt, |old| old.max(dt)));
        self.simultaneous += usize::from(dt == 0);
        self.within_20ms += usize::from(dt.unsigned_abs() <= 20_000);
        self.within_100ms += usize::from(dt.unsigned_abs() <= 100_000);
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct InputAudit {
    pub start_us: u64,
    pub end_us: u64,
    pub input_rates: Vec<ChannelRate>,
    /// Integral of held rates, including the 1-us boundary under the old rates.
    pub expected_sensor_spikes: f64,
    pub emitted_sensor_spikes: usize,
    /// Actual emissions of visual neurons, distinct from generated root spikes.
    pub visual_neuron_spikes: usize,
    pub action_copy_spikes: usize,
    /// Visual, processing, motor, action-copy populations respectively.
    pub homeostasis_by_population: [usize; 4],
    pub input_spikes: Vec<ChannelSpike>,
    pub core_arrivals: Vec<ArrivalSummary>,
    pub sensor_to_core_arrivals: usize,
    pub sensor_to_core_amplitude: f64,
    pub first_action_copy_arrival_us: Option<u64>,
    pub first_sensor_to_core_arrival_us: Option<u64>,
    pub action_copy_arrivals: usize,
    pub action_copy_arrival_times_us: Vec<u64>,
    pub motor_feedback_arrivals: usize,
    pub weight_updates: usize,
    pub homeostasis_updates: usize,
    pub formed: usize,
    pub pruned: usize,
    pub pre_post_deltas: PairDeltas,
    pub before: NetworkSummary,
    pub after: NetworkSummary,
    pub raw_events: Vec<ObservationEvent>,
}

pub(crate) struct Collector {
    pub last: Option<InputAudit>,
    options: AuditOptions,
    core: Vec<NeuronId>,
    copies: Vec<NeuronId>,
    motors: Vec<NeuronId>,
    sensors: u64,
    previous_rates: Vec<ChannelRate>,
    last_post: BTreeMap<NeuronId, u64>,
    last_pre: BTreeMap<NeuronId, BTreeMap<SynapseId, u64>>,
    refractory: Vec<(u64, u64)>,
}
impl Collector {
    pub fn new(
        options: AuditOptions,
        core: Vec<NeuronId>,
        copies: Vec<NeuronId>,
        motors: Vec<NeuronId>,
        sensors: u64,
    ) -> Self {
        Self {
            last: None,
            options,
            core,
            copies,
            motors,
            sensors,
            previous_rates: Vec::new(),
            last_post: BTreeMap::new(),
            last_pre: BTreeMap::new(),
            refractory: Vec::new(),
        }
    }
    pub fn begin(
        &mut self,
        start: u64,
        end: u64,
        rates: &[ChannelRate],
        network: &Network,
        upper: f32,
    ) -> InputAudit {
        self.refractory = self
            .core
            .iter()
            .map(|id| {
                let n = network.neuron(*id).expect("core");
                (n.refractory_until().0, n.params().refractory_period_us)
            })
            .collect();
        let expected = (self
            .previous_rates
            .iter()
            .map(|r| f64::from(r.rate_hz))
            .sum::<f64>()
            + rates
                .iter()
                .map(|r| f64::from(r.rate_hz) * (end - start) as f64)
                .sum::<f64>())
            / 1_000_000.0;
        self.previous_rates = rates.to_vec();
        InputAudit {
            start_us: start,
            end_us: end,
            input_rates: rates.to_vec(),
            expected_sensor_spikes: expected,
            core_arrivals: vec![ArrivalSummary::default(); self.core.len()],
            before: NetworkSummary::read(network, upper),
            ..InputAudit::default()
        }
    }
    pub fn input(&self, audit: &mut InputAudit, spikes: &[ChannelSpike]) {
        audit.emitted_sensor_spikes = spikes.len();
        if self.options.input_spikes {
            audit.input_spikes = spikes.to_vec();
        }
    }
    pub fn event(&mut self, audit: &mut InputAudit, event: &ObservationEvent, network: &Network) {
        if self.options.raw_events {
            audit.raw_events.push(event.clone());
        }
        match event {
            ObservationEvent::SpikeEmitted(spike) => {
                audit.visual_neuron_spikes += usize::from(spike.neuron_id.0 < self.sensors);
                audit.action_copy_spikes +=
                    usize::from(self.copies.binary_search(&spike.neuron_id).is_ok());
                if let Ok(index) = self.core.binary_search(&spike.neuron_id) {
                    self.refractory[index].0 =
                        spike.time.0.saturating_add(self.refractory[index].1);
                    if let Some(incoming) = self.last_pre.get(&spike.neuron_id) {
                        for &pre in incoming.values() {
                            audit.pre_post_deltas.record(spike.time.0, pre);
                        }
                    }
                    self.last_post.insert(spike.neuron_id, spike.time.0);
                }
            }
            ObservationEvent::SynapticArrival {
                time,
                synapse_id,
                source,
                target,
                amplitude,
            } => {
                if let Ok(index) = self.core.binary_search(target) {
                    let summary = &mut audit.core_arrivals[index];
                    summary.arrivals += 1;
                    summary.nonzero_arrivals += usize::from(*amplitude != 0.0);
                    summary.refractory_arrivals +=
                        usize::from(*amplitude != 0.0 && time.0 < self.refractory[index].0);
                    summary.amplitude_sum += f64::from(*amplitude);
                    summary.absolute_amplitude_sum += f64::from(amplitude.abs());
                    if source.0 < self.sensors {
                        audit.sensor_to_core_arrivals += 1;
                        audit.first_sensor_to_core_arrival_us.get_or_insert(time.0);
                        audit.sensor_to_core_amplitude += f64::from(*amplitude);
                    }
                    if self.copies.contains(source) {
                        audit.action_copy_arrivals += 1;
                        audit.action_copy_arrival_times_us.push(time.0);
                        audit.first_action_copy_arrival_us.get_or_insert(time.0);
                    }
                    if self.motors.contains(source) {
                        audit.motor_feedback_arrivals += 1;
                    }
                    if network.synapse(*synapse_id).is_some_and(|s| s.is_plastic()) {
                        if let Some(&post) = self.last_post.get(target) {
                            audit.pre_post_deltas.record(post, time.0);
                        }
                        self.last_pre
                            .entry(*target)
                            .or_default()
                            .insert(*synapse_id, time.0);
                    }
                }
            }
            ObservationEvent::WeightChanged { .. } => audit.weight_updates += 1,
            ObservationEvent::HomeostasisChanged { neuron_id, .. } => {
                audit.homeostasis_updates += 1;
                let population = if neuron_id.0 < self.sensors {
                    Some(0)
                } else if self.core.binary_search(neuron_id).is_ok() {
                    Some(1)
                } else if self.motors.binary_search(neuron_id).is_ok() {
                    Some(2)
                } else if self.copies.binary_search(neuron_id).is_ok() {
                    Some(3)
                } else {
                    None
                };
                if let Some(index) = population {
                    audit.homeostasis_by_population[index] += 1;
                }
            }
            _ => {}
        }
    }
    pub fn finish(&mut self, audit: InputAudit) {
        self.last = Some(audit);
    }
    pub fn forget_pruned(&mut self, ids: &[u64]) {
        for &id in ids {
            for incoming in self.last_pre.values_mut() {
                incoming.remove(&SynapseId(id));
            }
        }
    }
}

/// Compare learned parameters; evolving membrane state and transmission counters
/// may change while STDP, stored utility, homeostasis and structure stay frozen.
pub fn learned_parameters_equal(a: &Network, b: &Network) -> bool {
    a.neuron_count() == b.neuron_count()
        && a.synapse_count() == b.synapse_count()
        && a.neurons().zip(b.neurons()).all(|(x, y)| {
            x.id() == y.id()
                && x.position() == y.position()
                && x.role() == y.role()
                && x.polarity() == y.polarity()
                && x.params() == y.params()
                && x.base_threshold() == y.base_threshold()
                && x.intrinsic_current() == y.intrinsic_current()
                && x.structural_drive() == y.structural_drive()
        })
        && a.synapses().zip(b.synapses()).all(|(x, y)| {
            x.id() == y.id()
                && x.pre() == y.pre()
                && x.post() == y.post()
                && x.weight() == y.weight()
                && x.delay_us() == y.delay_us()
                && x.is_plastic() == y.is_plastic()
                && x.is_enabled() == y.is_enabled()
                && x.pre_trace() == y.pre_trace()
                && x.pre_trace_updated_at() == y.pre_trace_updated_at()
                && x.utility() == y.utility()
                && x.utility_updated_at() == y.utility_updated_at()
                && x.utility_eligibility() == y.utility_eligibility()
                && x.utility_eligibility_updated_at() == y.utility_eligibility_updated_at()
        })
}
