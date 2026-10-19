//! Paired development assay on one recorded pixel/action stream.
//! Action labels describe the host's own commands, never guest state or reward.

use crate::{
    Result,
    model::{KeyboardLayout, Model, ModelConfig, ModelKind, Step},
};
use nerva::core::Network;
use nerva_qemu::{capture::Frame, keyboard::Key};
use serde_json::{Value, json};
use std::{
    fs,
    io::Write,
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

#[derive(Clone, Debug, PartialEq)]
pub struct RecordedFrame {
    pub frame: Frame,
    pub executed_key: Key,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Recording {
    pub frames: Vec<RecordedFrame>,
}

impl Recording {
    /// General A/D sequence; unlike the original paired study, no fixed order.
    pub fn validate_sequence(&self) -> Result<()> {
        if !(8..=10_000).contains(&self.frames.len()) {
            return Err("recording needs 8..=10000 frames".into());
        }
        if self
            .frames
            .iter()
            .any(|frame| !matches!(frame.executed_key, Key::A | Key::D))
        {
            return Err("recording supports executed A/D keys only".into());
        }
        Ok(())
    }

    pub fn write_sequence_manifest(&self, directory: &Path) -> Result<()> {
        self.validate_sequence()?;
        let actions: Vec<_> = self
            .frames
            .iter()
            .map(|frame| frame.executed_key.channel())
            .collect();
        fs::write(
            directory.join("recording.json"),
            serde_json::to_vec_pretty(&json!({
                "version": 2, "actions": actions,
                "input_boundary": "pixels and own executed A/D commands only"
            }))?,
        )?;
        Ok(())
    }

    pub fn validate(&self) -> Result<()> {
        if !(8..=200).contains(&self.frames.len()) || !self.frames.len().is_multiple_of(4) {
            return Err("study recording needs 8..=200 frames in four-action blocks".into());
        }
        if self
            .frames
            .iter()
            .enumerate()
            .any(|(index, frame)| frame.executed_key != [Key::A, Key::A, Key::D, Key::D][index % 4])
        {
            return Err("study recording action order must be A,A,D,D".into());
        }
        Ok(())
    }

    pub fn write_manifest(&self, directory: &Path) -> Result<()> {
        self.validate()?;
        let actions: Vec<_> = self
            .frames
            .iter()
            .map(|frame| frame.executed_key.channel())
            .collect();
        fs::write(
            directory.join("recording.json"),
            serde_json::to_vec_pretty(&json!({
            "version": 1, "actions": actions, "images": "frames/000000.ppm etc.",
            "input_boundary": "pixels and own executed A/D commands only" }))?,
        )?;
        Ok(())
    }

    pub fn load(directory: &Path) -> Result<Self> {
        let manifest: Value = serde_json::from_slice(&fs::read(directory.join("recording.json"))?)?;
        if manifest["version"] != 1 && manifest["version"] != 2 {
            return Err("unsupported recording version".into());
        }
        let actions = manifest["actions"]
            .as_array()
            .ok_or("recording has no action array")?;
        if !(8..=10_000).contains(&actions.len()) {
            return Err("recording needs 8..=10000 frames".into());
        }
        let mut frames = Vec::with_capacity(actions.len());
        for (index, action) in actions.iter().enumerate() {
            let key = action
                .as_u64()
                .and_then(Key::from_channel)
                .ok_or("invalid recorded action")?;
            frames.push(RecordedFrame {
                frame: Frame::read_ppm(&directory.join(format!("frames/{index:06}.ppm")))?,
                executed_key: key,
            });
        }
        let recording = Self { frames };
        recording.validate_sequence()?;
        if manifest["version"] == 1 {
            recording.validate()?;
        }
        Ok(recording)
    }
}

#[derive(Debug, PartialEq)]
pub struct AssayRun {
    pub steps: Vec<Step>,
    pub final_network: Network,
}

pub fn replay(mut model: Model, recording: &Recording, cancelled: &AtomicBool) -> Result<AssayRun> {
    let mut steps = Vec::with_capacity(recording.frames.len());
    for frame in &recording.frames {
        if cancelled.load(Ordering::Relaxed) {
            return Err("study interrupted; recording remains reusable".into());
        }
        model.record_executed_keys(&[frame.executed_key])?;
        steps.push(model.step(&frame.frame)?);
        // Proposed neural keys are measured but never actuated in this assay.
    }
    Ok(AssayRun {
        steps,
        final_network: model.network().clone(),
    })
}

pub fn run_study(
    recording: &Recording,
    base: &ModelConfig,
    seeds: &[u64],
    output: &Path,
    cancelled: &AtomicBool,
) -> Result<Value> {
    recording.validate()?;
    base.validate()?;
    if base.kind != ModelKind::Recurrent
        || base.keyboard != KeyboardLayout::Wasd
        || !base.learning
        || !base.homeostasis
        || !base.feedback
    {
        return Err(
            "study requires recurrent WASD, STDP, homeostasis and feedback; use demo for ablations"
                .into(),
        );
    }
    if seeds.is_empty() || seeds.len() > 12 {
        return Err("study needs 1..=12 explicit seeds".into());
    }
    let mut unique = seeds.to_vec();
    unique.sort();
    unique.dedup();
    if unique.len() != seeds.len() {
        return Err("study seeds must be distinct".into());
    }
    let mut activity = fs::File::create_new(output.join("study-core.csv"))?;
    writeln!(
        activity,
        "seed,development,frame,executed_key,neuron,spikes"
    )?;
    let mut topology = fs::File::create_new(output.join("study-topology.csv"))?;
    writeln!(topology, "seed,development,synapse,source,target,weight")?;
    let mut spikes = fs::File::create_new(output.join("study-spikes.csv"))?;
    writeln!(spikes, "seed,development,frame,neuron,time_us")?;
    let mut structure = fs::File::create_new(output.join("study-structure.csv"))?;
    writeln!(
        structure,
        "seed,development,frame,event,synapse,source,target"
    )?;
    let mut results = Vec::new();
    for &seed in seeds {
        if cancelled.load(Ordering::Relaxed) {
            return Err("study interrupted; recording remains reusable".into());
        }
        let mut config = base.clone();
        config.seed = seed;
        config.development = true;
        let mut control_config = config.clone();
        control_config.development = false;
        let developing = Model::from_config(config.clone())?;
        let fixed = Model::from_config(control_config.clone())?;
        let initial_equal = developing.network() == fixed.network();
        if !initial_equal {
            return Err("paired study initial networks differ".into());
        }
        let processing = developing.processing_ids().to_vec();
        let initial_core = developing.core_synapse_count();
        let developed = replay(developing, recording, cancelled)?;
        let control = replay(fixed, recording, cancelled)?;
        let developed_replay = replay(Model::from_config(config)?, recording, cancelled)?;
        let control_replay = replay(Model::from_config(control_config)?, recording, cancelled)?;
        let exact_replay = developed == developed_replay && control == control_replay;
        if !exact_replay {
            return Err(format!(
                "non-reproducible core activity or final neural state for seed {seed}"
            )
            .into());
        }
        let mut changed_bins = 0usize;
        let mut l1 = 0usize;
        let mut events_differ = false;
        for (a, b) in developed.steps.iter().zip(&control.steps) {
            events_differ |= a.core_events != b.core_events;
            for (&x, &y) in a.core_counts.iter().zip(&b.core_counts) {
                changed_bins += usize::from(x != y);
                l1 += x.abs_diff(y);
            }
        }
        for (development, run) in [(true, &developed), (false, &control)] {
            for (frame, step) in run.steps.iter().enumerate() {
                for &(neuron, time) in &step.core_events {
                    writeln!(spikes, "{seed},{development},{frame},{neuron},{time}")?;
                }
                for &(id, source, target) in &step.formed {
                    writeln!(
                        structure,
                        "{seed},{development},{frame},formed,{id},{source},{target}"
                    )?;
                }
                for &id in &step.pruned {
                    writeln!(structure, "{seed},{development},{frame},pruned,{id},,")?;
                }
                for (index, neuron) in processing.iter().enumerate() {
                    writeln!(
                        activity,
                        "{seed},{development},{frame},{},{},{}",
                        recording.frames[frame].executed_key.qcode(),
                        neuron.0,
                        step.core_counts[index]
                    )?;
                }
            }
            for synapse in run.final_network.synapses() {
                writeln!(
                    topology,
                    "{seed},{development},{},{},{},{}",
                    synapse.id().0,
                    synapse.pre().0,
                    synapse.post().0,
                    synapse.weight().get()
                )?;
            }
        }
        let developed_summary = summary(&developed, recording);
        let control_summary = summary(&control, recording);
        results.push(json!({"seed": seed,"same_initial_network": initial_equal,
            "exact_replay": exact_replay,"initial_core_synapses": initial_core,
            "changed_neuron_frame_bins": changed_bins,"core_spike_l1_difference": l1,
            "core_spike_trains_differ": events_differ,
            "development": developed_summary,"fixed_topology": control_summary}));
    }
    let any_difference = results
        .iter()
        .any(|result| result["core_spike_trains_differ"] == true);
    let report = json!({"experiment": "paired-recurrent-core-development", "frames": recording.frames.len(),
        "seeds": seeds,"processing_neurons": base.processing_neurons,"motor_channels": 5,
        "same_recorded_pixel_and_action_stream": true,"all_exact_replays": true,
        "any_core_activity_difference": any_difference,"results": results,
        "interpretation": "Measured development-induced changes, not proof of learned action effects. A/D-conditioned activity can differ from sensory input alone.",
        "replay_contract": "all per-frame measurements, exact core spike times and final Network state", 
        "simulation_frame_us": super::model::WINDOW_US,
        "initial_structural_drive": base.initial_structural_drive,
        "pruning_min_below_us": base.pruning_min_below_us});
    let mut summary = String::from(
        "# Gepaarter Versuch mit rekurrentem Core\n\nEine gemeinsame QEMU-Bild-/A/D-Aufzeichnung; je Variante eine exakte Wiederholung.\nDie Zahlen messen den Einfluss von Strukturentwicklung, keine gelernte Handlungswirkung.\n\n| Seed | Core-Spikes mit Entwicklung | Core-Spikes fest | Neue Synapsen | Entfernte Synapsen | Abweichende Neuron/Frame-Zählungen | L1-Zähldifferenz | Exakter Replay |\n|---|---|---|---|---|---|---|---|\n",
    );
    for result in report["results"].as_array().expect("result records") {
        summary.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} |\n",
            result["seed"],
            result["development"]["core_spikes"],
            result["fixed_topology"]["core_spikes"],
            result["development"]["formed"],
            result["development"]["pruned"],
            result["changed_neuron_frame_bins"],
            result["core_spike_l1_difference"],
            result["exact_replay"]
        ));
    }
    summary.push_str("\nRohdaten: [study.json](study.json), [Core-Aktivität](study-core.csv), [Endtopologien](study-topology.csv).\n");
    fs::write(output.join("study-summary.md"), summary)?;
    fs::write(
        output.join("study.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    Ok(report)
}

fn summary(run: &AssayRun, recording: &Recording) -> Value {
    let size = run.steps.first().map_or(0, |step| step.core_counts.len());
    let mut a = vec![0usize; size];
    let mut d = vec![0usize; size];
    let mut a_frames = 0usize;
    let mut d_frames = 0usize;
    for (step, frame) in run.steps.iter().zip(&recording.frames) {
        let target = if frame.executed_key == Key::A {
            a_frames += 1;
            &mut a
        } else {
            d_frames += 1;
            &mut d
        };
        for (total, &count) in target.iter_mut().zip(&step.core_counts) {
            *total += count;
        }
    }
    json!({"core_spikes": run.steps.iter().map(|step| step.core_events.len()).sum::<usize>(),
        "motor_spikes": run.steps.iter().map(|step| step.motor_spikes).sum::<usize>(),
        "formed": run.steps.iter().map(|step| step.formed.len()).sum::<usize>(),
        "pruned": run.steps.iter().map(|step| step.pruned.len()).sum::<usize>(),
        "weight_changes": run.steps.iter().map(|step| step.weight_changes).sum::<usize>(),
        "homeostasis_changes": run.steps.iter().map(|step| step.homeostasis_changes).sum::<usize>(),
        "raw_motor_feedback_arrivals": run.steps.iter().map(|step| step.feedback_arrivals).sum::<usize>(),
        "executed_action_copy_arrivals": run.steps.iter().map(|step| step.action_copy_arrivals).sum::<usize>(),
        "final_core_synapses": run.steps.last().map_or(0, |step| step.core_synapses),
        "mean_core_spikes_after_A": a.iter().map(|count| *count as f64/a_frames.max(1) as f64).collect::<Vec<_>>(),
        "mean_core_spikes_after_D": d.iter().map(|count| *count as f64/d_frames.max(1) as f64).collect::<Vec<_>>()})
}
