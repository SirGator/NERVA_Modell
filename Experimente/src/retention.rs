//! Train a running instance, freeze in place, then probe without plasticity.
//! Readouts are external measurements: their labels never enter NERVA.

use crate::{
    Result,
    experiment::Recording,
    model::{KeyboardLayout, Model, ModelConfig, ModelKind, Step},
};
use nerva::core::Network;
use nerva_qemu::{keyboard::Key, vision_encoder::grayscale};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionControl {
    Aligned,
    Swapped,
    Delayed,
    Absent,
    ConstantA,
    ConstantD,
}

impl ActionControl {
    pub const ALL: [Self; 6] = [
        Self::Aligned,
        Self::Swapped,
        Self::Delayed,
        Self::Absent,
        Self::ConstantA,
        Self::ConstantD,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Aligned => "aligned",
            Self::Swapped => "swapped",
            Self::Delayed => "delayed_one_frame",
            Self::Absent => "absent",
            Self::ConstantA => "constant_image_a",
            Self::ConstantD => "constant_image_d",
        }
    }

    pub fn key(self, recording: &Recording, index: usize) -> Option<Key> {
        let key = recording.frames[index].executed_key;
        match self {
            Self::Aligned => Some(key),
            Self::Swapped => Some(if key == Key::A { Key::D } else { Key::A }),
            Self::Delayed => index
                .checked_sub(1)
                .map(|i| recording.frames[i].executed_key),
            Self::Absent => None,
            Self::ConstantA => Some(Key::A),
            Self::ConstantD => Some(Key::D),
        }
    }

    fn constant_image(self) -> bool {
        matches!(self, Self::ConstantA | Self::ConstantD)
    }
}

pub use nerva_vm::model::audit::learned_parameters_equal;

fn check_cancelled(cancelled: &AtomicBool) -> Result<()> {
    if cancelled.load(Ordering::Relaxed) {
        return Err("retention assay interrupted; recordings remain reusable".into());
    }
    Ok(())
}

pub fn probe(
    model: &mut Model,
    recording: &Recording,
    control: ActionControl,
    cancelled: &AtomicBool,
) -> Result<Vec<Step>> {
    if model.config().learning || model.config().development || model.config().homeostasis {
        return Err("retention probes require an already frozen model".into());
    }
    recording.validate_sequence()?;
    let checkpoint = model.network().clone();
    let mut steps = Vec::with_capacity(recording.frames.len());
    for (index, frame) in recording.frames.iter().enumerate() {
        check_cancelled(cancelled)?;
        let keys: Vec<_> = control.key(recording, index).into_iter().collect();
        model.record_executed_keys(&keys)?;
        let frame = if control.constant_image() {
            &recording.frames[0].frame
        } else {
            &frame.frame
        };
        let step = model.step(frame)?;
        if step.weight_changes != 0
            || step.homeostasis_changes != 0
            || !step.formed.is_empty()
            || !step.pruned.is_empty()
            || !learned_parameters_equal(&checkpoint, model.network())
        {
            return Err("learned parameters changed during a frozen probe".into());
        }
        steps.push(step);
    }
    Ok(steps)
}

/// The split is chronological. Targets are next-image grayscale differences at
/// 16x16, quantized to 1/255; no time index or future action is a feature.
/// This is a diagnostic readout, not a learned motor policy or task score.
pub fn prediction_readout(recording: &Recording, steps: &[Step]) -> Result<Value> {
    if recording.frames.len() != steps.len() || steps.len() < 16 {
        return Err("prediction readout needs matching sequences of at least 16 frames".into());
    }
    let images: Vec<_> = recording
        .frames
        .iter()
        .map(|frame| grayscale(&frame.frame, 16, 16))
        .collect::<Result<_>>()?;
    let core: Vec<Vec<f64>> = steps
        .iter()
        .map(|step| step.core_counts.iter().map(|&n| n as f64).collect())
        .collect();
    let visual: Vec<Vec<f64>> = images
        .iter()
        .map(|image| image.iter().map(|&x| f64::from(x)).collect())
        .collect();
    let targets: Vec<Vec<i16>> = images
        .windows(2)
        .map(|pair| {
            pair[1]
                .iter()
                .zip(&pair[0])
                .map(|(&next, &now)| ((next - now) * 255.0).round() as i16)
                .collect()
        })
        .collect();
    let split = targets.len() / 2;
    let core_score = centroid_score(&core[..targets.len()], &targets, split);
    let visual_score = centroid_score(&visual[..targets.len()], &targets, split);
    let actions: Vec<Vec<i16>> = recording
        .frames
        .iter()
        .map(|frame| vec![frame.executed_key.channel() as i16])
        .collect();
    Ok(json!({
        "target": "next grayscale image difference, 16x16, quantized to 1/255",
        "features": "current frame Core spike counts; no next action or frame index",
        "calibration_frames": split, "test_frames": targets.len() - split,
        "next_change_from_core": core_score,
        "next_change_from_current_pixels": visual_score,
        "current_recorded_action_from_core": centroid_score(&core, &actions, actions.len()/2),
    }))
}

pub(crate) fn centroid_score(features: &[Vec<f64>], labels: &[Vec<i16>], split: usize) -> Value {
    let mut classes: BTreeMap<&Vec<i16>, (Vec<f64>, usize)> = BTreeMap::new();
    for (features, label) in features[..split].iter().zip(&labels[..split]) {
        let (sum, count) = classes
            .entry(label)
            .or_insert_with(|| (vec![0.0; features.len()], 0));
        for (sum, &value) in sum.iter_mut().zip(features) {
            *sum += value;
        }
        *count += 1;
    }
    let majority = classes
        .iter()
        .max_by_key(|(_, (_, n))| *n)
        .map(|(&label, _)| label);
    let mut correct = 0usize;
    let mut majority_correct = 0usize;
    let mut unseen = 0usize;
    for (features, label) in features[split..].iter().zip(&labels[split..]) {
        let predicted = classes
            .iter()
            .min_by(|(_, (a, na)), (_, (b, nb))| {
                let distance = |sum: &[f64], n: usize| {
                    features
                        .iter()
                        .zip(sum)
                        .map(|(value, sum)| (value - sum / n as f64).powi(2))
                        .sum::<f64>()
                };
                distance(a, *na).total_cmp(&distance(b, *nb))
            })
            .map(|(&label, _)| label);
        correct += usize::from(predicted == Some(label));
        majority_correct += usize::from(majority == Some(label));
        unseen += usize::from(!classes.contains_key(label));
    }
    let tested = labels.len() - split;
    json!({"correct": correct, "tested": tested, "accuracy": correct as f64/tested as f64,
        "calibration_classes": classes.len(), "unseen_test_labels": unseen,
        "majority_correct": majority_correct, "majority_accuracy": majority_correct as f64/tested as f64})
}

fn activity_difference(a: &[Step], b: &[Step]) -> Value {
    let mut bins = 0usize;
    let mut l1 = 0usize;
    for (a, b) in a.iter().zip(b) {
        for (&a, &b) in a.core_counts.iter().zip(&b.core_counts) {
            bins += usize::from(a != b);
            l1 += a.abs_diff(b);
        }
    }
    json!({"changed_neuron_frame_bins": bins, "spike_count_l1": l1})
}

/// Branches repeat the identical prefix rather than reconstructing the network
/// from weights. Thus each freeze retains its own scheduler and random state.
pub fn run_retention(
    training: &Recording,
    testing: &Recording,
    base: &ModelConfig,
    train_frames: usize,
    seeds: &[u64],
    output: &Path,
    cancelled: &AtomicBool,
) -> Result<Value> {
    training.validate_sequence()?;
    testing.validate_sequence()?;
    base.validate()?;
    if base.kind != ModelKind::Recurrent
        || base.keyboard != KeyboardLayout::Wasd
        || !base.learning
        || !base.homeostasis
        || !base.feedback
        || !base.development
    {
        return Err("retention uses recurrent WASD with all learning rules; controls are generated automatically".into());
    }
    if !(1..=10_000).contains(&train_frames) || testing.frames.len() < 16 {
        return Err(
            "retention needs 1..=10000 training frames and at least 16 probe frames".into(),
        );
    }
    let mut unique = seeds.to_vec();
    unique.sort();
    unique.dedup();
    if seeds.is_empty() || seeds.len() > 12 || seeds.len() != unique.len() {
        return Err("retention needs 1..=12 distinct seeds".into());
    }
    // Include wraparound training windows because the recording is cycled.
    let training_windows: Vec<Vec<_>> = (0..training.frames.len())
        .map(|i| {
            (0..4)
                .map(|j| training.frames[(i + j) % training.frames.len()].executed_key)
                .collect()
        })
        .collect();
    let novel_windows = testing
        .frames
        .windows(4)
        .filter(|frames| {
            let window: Vec<_> = frames.iter().map(|frame| frame.executed_key).collect();
            !training_windows.contains(&window)
        })
        .count();
    if novel_windows == 0 {
        return Err(
            "probe recording needs a new four-action sequence absent from cyclic training".into(),
        );
    }
    let mut activity = fs::File::create_new(output.join("retention-core.csv"))?;
    writeln!(
        activity,
        "seed,condition,control,frame,recorded_key,injected_key,neuron,spikes"
    )?;
    let mut spikes = fs::File::create_new(output.join("retention-spikes.csv"))?;
    writeln!(spikes, "seed,condition,control,frame,neuron,time_us")?;
    let mut trace = fs::File::create_new(output.join("retention-training.csv"))?;
    writeln!(
        trace,
        "seed,condition,frame,core_spikes,weight_changes,homeostasis_changes,formed,pruned,core_synapses"
    )?;
    let mut topology = fs::File::create_new(output.join("retention-topology.csv"))?;
    writeln!(
        topology,
        "seed,condition,synapse,source,target,weight,delay_us,plastic,enabled,utility"
    )?;
    let mut neurons = fs::File::create_new(output.join("retention-neurons.csv"))?;
    writeln!(
        neurons,
        "seed,condition,neuron,base_threshold,intrinsic_current,structural_drive"
    )?;
    let mut results = Vec::new();
    for &seed in seeds {
        let mut config = base.clone();
        config.seed = seed;
        let initial = Model::from_config(config.clone())?;
        let initial_network = initial.network().clone();
        let initial_core = initial.core_synapse_count();
        drop(initial);
        for condition in [
            "before_training",
            "aged_frozen",
            "trained_fixed",
            "trained_development",
        ] {
            let mut config = config.clone();
            if condition == "trained_fixed" {
                config.development = false;
            }
            let frames = if condition == "before_training" {
                0
            } else {
                train_frames
            };
            let mut checkpoint: Option<Network> = None;
            let mut runs: Vec<Vec<Step>> = Vec::new();
            let mut controls = Vec::new();
            let mut training_summary = json!(null);
            for control in ActionControl::ALL {
                check_cancelled(cancelled)?;
                let mut model = Model::from_config(config.clone())?;
                if model.network() != &initial_network {
                    return Err("retention initial networks differ".into());
                }
                if matches!(condition, "before_training" | "aged_frozen") {
                    model.freeze()?;
                }
                let mut weight_changes = 0usize;
                let mut formed = 0usize;
                let mut pruned = 0usize;
                for index in 0..frames {
                    check_cancelled(cancelled)?;
                    let frame = &training.frames[index % training.frames.len()];
                    model.record_executed_keys(&[frame.executed_key])?;
                    let step = model.step(&frame.frame)?;
                    weight_changes += step.weight_changes;
                    formed += step.formed.len();
                    pruned += step.pruned.len();
                    if control == ActionControl::Aligned {
                        writeln!(
                            trace,
                            "{seed},{condition},{index},{},{},{},{},{},{}",
                            step.core_events.len(),
                            step.weight_changes,
                            step.homeostasis_changes,
                            step.formed.len(),
                            step.pruned.len(),
                            step.core_synapses
                        )?;
                    }
                }
                if let Some(checkpoint) = &checkpoint {
                    if model.network() != checkpoint {
                        return Err("repeated training prefixes differ before probing".into());
                    }
                } else {
                    training_summary = json!({"frames": frames, "weight_changes": weight_changes,
                        "formed": formed, "pruned": pruned, "initial_core_synapses": initial_core,
                        "frozen_core_synapses": model.core_synapse_count(),
                        "parameters_changed_since_initial": !learned_parameters_equal(&initial_network, model.network())});
                    for synapse in model.network().synapses() {
                        writeln!(
                            topology,
                            "{seed},{condition},{},{},{},{},{},{},{},{}",
                            synapse.id().0,
                            synapse.pre().0,
                            synapse.post().0,
                            synapse.weight().get(),
                            synapse.delay_us(),
                            synapse.is_plastic(),
                            synapse.is_enabled(),
                            synapse.utility()
                        )?;
                    }
                    for neuron in model.network().neurons() {
                        writeln!(
                            neurons,
                            "{seed},{condition},{},{},{},{}",
                            neuron.id().0,
                            neuron.base_threshold(),
                            neuron.intrinsic_current(),
                            neuron.structural_drive()
                        )?;
                    }
                    checkpoint = Some(model.network().clone());
                }
                model.freeze()?;
                if Some(model.network()) != checkpoint.as_ref() {
                    return Err("freeze altered the trained network".into());
                }
                let steps = probe(&mut model, testing, control, cancelled)?;
                for (index, step) in steps.iter().enumerate() {
                    for &(neuron, time) in &step.core_events {
                        writeln!(
                            spikes,
                            "{seed},{condition},{},{index},{neuron},{time}",
                            control.name()
                        )?;
                    }
                    let injected = control
                        .key(testing, index)
                        .map_or("none", |key| key.qcode());
                    for (i, id) in model.processing_ids().iter().enumerate() {
                        writeln!(
                            activity,
                            "{seed},{condition},{},{index},{},{injected},{},{}",
                            control.name(),
                            testing.frames[index].executed_key.qcode(),
                            id.0,
                            step.core_counts[i]
                        )?;
                    }
                }
                controls.push(json!({"control": control.name(), "parameters_retained": true,
                    "core_spikes": steps.iter().map(|step| step.core_events.len()).sum::<usize>(),
                    "action_copy_arrivals": steps.iter().map(|step| step.action_copy_arrivals).sum::<usize>(),
                    "difference_from_aligned": if runs.is_empty() { json!(null) } else { activity_difference(&runs[0], &steps) },
                    "readout": if control.constant_image() { json!(null) } else { prediction_readout(testing, &steps)? }
                }));
                runs.push(steps);
            }
            results.push(json!({"seed": seed, "condition": condition,
                "same_initial_network": true, "identical_prefix_checkpoints": true,
                "freeze_preserved_full_network": true, "training": training_summary,
                "constant_image_a_vs_d": activity_difference(&runs[4], &runs[5]),
                "controls": controls}));
        }
    }
    let report = json!({"version": 1, "training_frames": train_frames,
        "unique_training_frames": training.frames.len(), "probe_frames": testing.frames.len(),
        "training_stream": "recording cycled, never counted as new captured experience",
        "novel_probe_four_action_windows": novel_windows,
        "initial_structural_drive": base.initial_structural_drive,
        "pruning_min_below_us": base.pruning_min_below_us,
        "processing_neurons": base.processing_neurons, "encoding": format!("{:?}",base.encoding),
        "training_recording": recording_identity(training),
        "probe_recording": recording_identity(testing),
        "all_parameters_retained": true,
        "limits": ["Offline teacher-forced replay, no autonomous task-performance measurement",
            "Action controls alter copies only; raw neural motor feedback remains active",
            "Before/after differences include neural state and input-history effects; aged_frozen is an age-matched control",
            "Readout calibration is chronological within the frozen probe, not training of NERVA",
            "Constant-image A/D differences can already come from fixed feedback wiring",
            "Prediction is next pixel change, not proof of understanding or causal action prediction"],
        "results": results});
    fs::write(
        output.join("retention.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    fs::write(output.join("retention-summary.md"), summary(&report))?;
    Ok(report)
}

fn recording_identity(recording: &Recording) -> Value {
    // Stable diagnostic fingerprints, not a cryptographic integrity guarantee.
    let fingerprints: Vec<_> = recording
        .frames
        .iter()
        .map(|frame| {
            let mut hash = 0xcbf29ce484222325u64;
            for byte in (frame.frame.width() as u64)
                .to_le_bytes()
                .into_iter()
                .chain((frame.frame.height() as u64).to_le_bytes())
                .chain(frame.frame.rgb().iter().copied())
            {
                hash = (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3);
            }
            format!("{hash:016x}")
        })
        .collect();
    json!({"actions": recording.frames.iter().map(|frame| frame.executed_key.qcode()).collect::<Vec<_>>(),
        "frame_fingerprints_fnv1a64": fingerprints})
}

fn summary(report: &Value) -> String {
    let mut text = format!(
        "# Trainieren – Einfrieren – Prüfen\n\n{} Trainingsframes durch zyklische Wiederholung von {} aufgenommenen Bildern; {} separate Prüfungsbilder.\n\nAlle Prüfzweige behalten Gewichte, Topologie, STDP-/Utility-Zustände und homöostatische Parameter.\n\n| Seed | Bedingung | Core-Synapsen beim Einfrieren | A/D bei gleichem Bild: L1 | Nächstes Pixel-Delta: Core | Nur aktuelles Bild | Mehrheitskontrolle |\n|---|---|---:|---:|---:|---:|---:|\n",
        report["training_frames"], report["unique_training_frames"], report["probe_frames"]
    );
    for result in report["results"].as_array().expect("results") {
        let readout = &result["controls"][0]["readout"];
        text.push_str(&format!(
            "| {} | {} | {} | {} | {:.3} | {:.3} | {:.3} |\n",
            result["seed"],
            result["condition"].as_str().unwrap(),
            result["training"]["frozen_core_synapses"],
            result["constant_image_a_vs_d"]["spike_count_l1"],
            readout["next_change_from_core"]["accuracy"]
                .as_f64()
                .unwrap(),
            readout["next_change_from_current_pixels"]["accuracy"]
                .as_f64()
                .unwrap(),
            readout["next_change_from_core"]["majority_accuracy"]
                .as_f64()
                .unwrap()
        ));
    }
    text.push_str("\nDie Werte messen eine externe Vorhersage auf den eingefrorenen Aktivitätsmustern. Sie sind keine autonome Aufgabenleistung. Neue Klassen im Test zählen als Fehler. Aktionskopien werden vertauscht, um einen Frame verzögert oder entfernt; die Bilder bleiben dabei gleich. Die A/D-Festbildkontrolle hält das erste Prüfungsbild konstant.\n");
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readout_uses_calibration_only_and_counts_new_test_classes_as_errors() {
        let features = vec![
            vec![0.0],
            vec![1.0],
            vec![0.0],
            vec![1.0],
            vec![0.0],
            vec![1.0],
            vec![0.0],
            vec![1.0],
        ];
        let mut labels = vec![
            vec![0],
            vec![1],
            vec![0],
            vec![1],
            vec![0],
            vec![1],
            vec![0],
            vec![1],
        ];
        let score = centroid_score(&features, &labels, 4);
        assert_eq!(score["accuracy"], 1.0);
        assert_eq!(score["majority_accuracy"], 0.5);
        labels[4..].fill(vec![9]);
        let unknown = centroid_score(&features, &labels, 4);
        assert_eq!(unknown["accuracy"], 0.0);
        assert_eq!(unknown["unseen_test_labels"], 4);
        assert_eq!(unknown["calibration_classes"], 2);
    }
}
