use crate::{
    Result,
    model::{Step, audit::InputAudit},
};
use serde_json::{Value, json};
use std::{
    fs::{File, OpenOptions},
    io::{BufWriter, Write},
    path::Path,
};

pub struct Logs {
    pub details: BufWriter<File>,
    audit: BufWriter<File>,
    raw: Option<BufWriter<File>>,
    sensor: Option<BufWriter<File>>,
    pub arm: &'static str,
    pub mapping: &'static str,
}
fn create(path: &Path) -> Result<BufWriter<File>> {
    Ok(BufWriter::new(
        OpenOptions::new().write(true).create_new(true).open(path)?,
    ))
}
pub fn sidecar(output: &Path, suffix: &str) -> std::path::PathBuf {
    let mut path = output.as_os_str().to_owned();
    path.push(suffix);
    path.into()
}
impl Logs {
    pub fn new(output: &Path, raw: bool, extended: bool) -> Result<Self> {
        let mut details = create(&sidecar(output, ".details.csv"))?;
        writeln!(
            details,
            "arm,mapping,stage,trial,key,action_us,motion_us,forecast_us,effect_image_us,shown_effect,actual_effect,count_prediction,time_prediction,copy_arrivals_before_image,first_copy_arrival_us,cue_core_spikes,first_cue_spike_us,last_cue_spike_us,start_frame_hash,effect_frame_hash"
        )?;
        let mut audit = create(&sidecar(output, ".audit.csv"))?;
        writeln!(
            audit,
            "arm,mapping,stage,trial,phase,start_us,end_us,sensory_spikes,core_spikes,motor_spikes,expected_sensor_spikes,sensor_to_core_arrivals,sensor_to_core_amplitude,first_sensor_to_core_us,copy_arrivals,first_copy_us,motor_feedback,weight_updates,homeostasis_updates,formed,pruned,synapses,active_pre_traces,pre_trace_sum,pre_trace_max,zero_weights,saturated_weights,utility_sum,eligibility_sum,delta_pairs,min_delta_us,max_delta_us,core_membrane_min,core_membrane_mean,core_membrane_max,inhibitory_core_spikes"
        )?;
        let mut raw_file = if raw {
            Some(create(&sidecar(output, ".events.csv"))?)
        } else {
            None
        };
        if let Some(file) = &mut raw_file {
            writeln!(file, "arm,mapping,stage,trial,phase,event")?;
        }
        let sensor = if extended {
            let mut file = create(&sidecar(output, ".sensor.csv"))?;
            writeln!(
                file,
                "arm,mapping,stage,trial,phase,start_us,end_us,generated_sensor_spikes,visual_neuron_spikes,action_copy_spikes,homeostasis_visual,homeostasis_core,homeostasis_motor,homeostasis_copy,sensor_intrinsic_min,sensor_intrinsic_mean,sensor_intrinsic_max"
            )?;
            Some(file)
        } else {
            None
        };
        Ok(Self {
            details,
            audit,
            raw: raw_file,
            sensor,
            arm: "before",
            mapping: "normal",
        })
    }
    pub fn extended(&self) -> bool {
        self.sensor.is_some()
    }
    pub fn phase(
        &mut self,
        meta: (&str, usize, &str),
        step: &Step,
        a: &InputAudit,
        neural: &Value,
    ) -> Result<()> {
        let (stage, trial, phase) = meta;
        let opt = |v: Option<u64>| v.map(|x| x.to_string()).unwrap_or_default();
        if let Some(file) = &mut self.sensor {
            writeln!(
                file,
                "{},{},{stage},{trial},{phase},{},{},{},{},{},{},{},{},{},{},{},{}",
                self.arm,
                self.mapping,
                step.start_us,
                step.end_us,
                a.emitted_sensor_spikes,
                a.visual_neuron_spikes,
                a.action_copy_spikes,
                a.homeostasis_by_population[0],
                a.homeostasis_by_population[1],
                a.homeostasis_by_population[2],
                a.homeostasis_by_population[3],
                neural["sensor_intrinsic_min"],
                neural["sensor_intrinsic_mean"],
                neural["sensor_intrinsic_max"]
            )?;
        }
        writeln!(
            self.audit,
            "{},{},{stage},{trial},{phase},{},{},{},{},{},{:.9},{},{:.9},{},{},{},{},{},{},{},{},{},{},{:.9},{:.9},{},{},{:.9},{:.9},{},{},{},{:.9},{:.9},{:.9},{}",
            self.arm,
            self.mapping,
            step.start_us,
            step.end_us,
            step.sensory_spikes,
            step.core_counts.iter().sum::<usize>(),
            step.motor_spikes,
            a.expected_sensor_spikes,
            a.sensor_to_core_arrivals,
            a.sensor_to_core_amplitude,
            opt(a.first_sensor_to_core_arrival_us),
            a.action_copy_arrivals,
            opt(a.first_action_copy_arrival_us),
            a.motor_feedback_arrivals,
            a.weight_updates,
            a.homeostasis_updates,
            a.formed,
            a.pruned,
            a.after.synapses,
            a.after.active_pre_traces,
            a.after.pre_trace_sum,
            a.after.pre_trace_max,
            a.after.weights_at_zero,
            a.after.weights_at_upper_bound,
            a.after.utility_sum,
            a.after.eligibility_sum,
            a.pre_post_deltas.count,
            a.pre_post_deltas
                .min_us
                .map(|x| x.to_string())
                .unwrap_or_default(),
            a.pre_post_deltas
                .max_us
                .map(|x| x.to_string())
                .unwrap_or_default(),
            neural["core_potential_min"].as_f64().unwrap(),
            neural["core_potential_mean"].as_f64().unwrap(),
            neural["core_potential_max"].as_f64().unwrap(),
            neural["inhibitory_core_spikes"].as_u64().unwrap()
        )?;
        if let Some(file) = &mut self.raw {
            for event in &a.raw_events {
                writeln!(
                    file,
                    "{},{},{stage},{trial},{phase},\"{}\"",
                    self.arm,
                    self.mapping,
                    format!("{event:?}").replace('"', "\"\"")
                )?;
            }
        }
        Ok(())
    }
    pub fn flush(&mut self) -> Result<()> {
        self.audit.flush()?;
        self.details.flush()?;
        if let Some(raw) = &mut self.raw {
            raw.flush()?;
        }
        if let Some(sensor) = &mut self.sensor {
            sensor.flush()?;
        }
        Ok(())
    }
}
#[derive(Default)]
pub struct Stats {
    pub inhibitory: usize,
    pub sensory: usize,
    pub core: usize,
    pub motor: usize,
    pub weight: usize,
    pub formed: usize,
    pub pruned: usize,
    pub copy: usize,
    pub feedback: usize,
    pub arrivals: usize,
    pub refractory: usize,
    pub expected: f64,
    pub sensor_current: f64,
    pub elapsed: u64,
    pub windows: usize,
    pub selected: usize,
    pub visual_emissions: usize,
    pub copy_emissions: usize,
    pub homeostasis_populations: [usize; 4],
    pub per_core_current: Vec<f64>,
    pub per_core_arrivals: Vec<usize>,
    pub delta_bins: [usize; 11],
}
impl Stats {
    pub fn add(&mut self, s: &Step, a: &InputAudit) {
        self.sensory += s.sensory_spikes;
        self.visual_emissions += a.visual_neuron_spikes;
        self.copy_emissions += a.action_copy_spikes;
        for (sum, count) in self
            .homeostasis_populations
            .iter_mut()
            .zip(a.homeostasis_by_population)
        {
            *sum += count;
        }
        self.core += s.core_counts.iter().sum::<usize>();
        self.motor += s.motor_spikes;
        self.weight += s.weight_changes;
        self.formed += s.formed.len();
        self.pruned += s.pruned.len();
        self.copy += s.action_copy_arrivals;
        self.feedback += s.feedback_arrivals;
        self.expected += a.expected_sensor_spikes;
        self.sensor_current += a.sensor_to_core_amplitude;
        self.elapsed += s.end_us - s.start_us + 1;
        self.windows += 1;
        self.selected += usize::from(s.action.is_some());
        if self.per_core_current.is_empty() {
            self.per_core_current.resize(a.core_arrivals.len(), 0.0);
            self.per_core_arrivals.resize(a.core_arrivals.len(), 0);
        }
        for (i, arrival) in a.core_arrivals.iter().enumerate() {
            self.arrivals += arrival.nonzero_arrivals;
            self.refractory += arrival.refractory_arrivals;
            self.per_core_current[i] += arrival.amplitude_sum;
            self.per_core_arrivals[i] += arrival.nonzero_arrivals;
        }
        for (sum, n) in self.delta_bins.iter_mut().zip(a.pre_post_deltas.histogram) {
            *sum += n;
        }
    }
    pub fn value(&self) -> Value {
        json!({"spike_count":self.sensory+self.core+self.motor,"sensory_spikes":self.sensory,"core_spikes":self.core,"inhibitory_core_spikes":self.inhibitory,"motor_spikes":self.motor,
        "weight_updates":self.weight,"formed":self.formed,"pruned":self.pruned,"motor_feedback_arrivals":self.feedback,"action_copy_arrivals":self.copy,
        "refractory_arrival_fraction":self.refractory as f64/self.arrivals.max(1) as f64,"nonzero_core_arrivals":self.arrivals,"refractory_core_arrivals":self.refractory,
        "expected_sensor_spikes":self.expected,"actual_sensor_spikes_minus_expectation":self.sensory as f64-self.expected,
        "sensor_to_core_current_mean_per_neuron_second":self.sensor_current/(self.elapsed.max(1) as f64/1e6)/self.per_core_current.len().max(1) as f64,
        "all_input_signed_current_per_core_second":self.per_core_current.iter().map(|x|x/(self.elapsed.max(1) as f64/1e6)).collect::<Vec<_>>(),"arrivals_per_core":self.per_core_arrivals,
        "pre_post_delta_histogram":self.delta_bins,"delta_bin_upper_us":[-100000,-20000,-8000,-2000,-1,0,2000,8000,20000,100000],"observation_us":self.elapsed})
    }
    pub fn sensor_value(&self) -> Value {
        json!({"generated_sensor_spikes":self.sensory,"visual_neuron_spikes":self.visual_emissions,
            "visual_emissions_per_generated_spike":self.visual_emissions as f64 / self.sensory.max(1) as f64,
            "action_copy_spikes":self.copy_emissions,"homeostasis_updates_visual_core_motor_copy":self.homeostasis_populations})
    }
}
