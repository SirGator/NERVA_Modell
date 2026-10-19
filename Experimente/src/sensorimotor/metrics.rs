use crate::model::{Step, WINDOW_US};
use nerva_qemu::{capture::Frame, keyboard::Key, vision_encoder::grayscale};
use serde_json::{Value, json};
use std::collections::BTreeMap;

pub fn motion(before: &Frame, after: &Frame) -> i16 {
    let moment = |frame: &Frame| {
        frame
            .rgb()
            .as_chunks::<3>()
            .0
            .iter()
            .enumerate()
            .map(|(i, rgb)| {
                (i % frame.width()) as i64 * rgb.iter().map(|&x| i64::from(x)).sum::<i64>()
            })
            .sum::<i64>()
    };
    (moment(after) - moment(before)).signum() as i16
}

pub fn features(step: &Step, core_ids: &[nerva::core::NeuronId]) -> (Vec<f64>, Vec<f64>) {
    let counts = step.core_counts.iter().map(|&x| x as f64).collect();
    let mut temporal = vec![0.0; step.core_counts.len() * 10];
    for &(id, time) in &step.core_events {
        if let Ok(neuron) = core_ids.binary_search(&nerva::core::NeuronId(id)) {
            let bin = ((time - step.start_us) / (WINDOW_US / 10)).min(9) as usize;
            temporal[neuron * 10 + bin] += 1.0;
        }
    }
    (counts, temporal)
}

#[derive(Default)]
pub struct MotorMeasures {
    pub windows: usize,
    pub ties: usize,
    pub silence: usize,
    pub chosen: usize,
    pub temporal_distinct: usize,
    pub first_distinct: usize,
    pub four_four: usize,
    pub a: usize,
    pub d: usize,
    pub moved: usize,
    positive: usize,
    negative: usize,
    refractory: usize,
    positive_amplitude: f64,
    negative_amplitude: f64,
    isi: Vec<u64>,
    last: [Option<u64>; 2],
}

impl MotorMeasures {
    pub fn observe(&mut self, step: &Step, key: Option<Key>, pixel_motion: Option<i16>) {
        self.windows += 1;
        self.chosen += usize::from(key.is_some());
        self.a += usize::from(key == Some(Key::A));
        self.d += usize::from(key == Some(Key::D));
        self.moved += usize::from(pixel_motion.is_some_and(|motion| motion != 0));
        let a = step.motor_counts[Key::A as usize];
        let d = step.motor_counts[Key::D as usize];
        self.silence += usize::from(a == 0 && d == 0);
        if a == d && a > 0 {
            self.ties += 1;
            self.four_four += usize::from(a == 4);
            let times = |key: Key| {
                step.motor_events
                    .iter()
                    .filter(|p| p.channel == key.channel())
                    .map(|p| p.at_us)
                    .collect::<Vec<_>>()
            };
            let aa = times(Key::A);
            let dd = times(Key::D);
            self.temporal_distinct += usize::from(aa != dd);
            self.first_distinct += usize::from(aa.first() != dd.first());
        }
        for (index, key) in [Key::A, Key::D].iter().enumerate() {
            let input = &step.motor_inputs[*key as usize];
            self.positive += input.positive_arrivals;
            self.negative += input.negative_arrivals;
            self.refractory += input.arrivals_during_refractory;
            self.positive_amplitude += input.positive_amplitude;
            self.negative_amplitude += input.negative_amplitude;
            for event in step
                .motor_events
                .iter()
                .filter(|p| p.channel == key.channel())
            {
                if let Some(last) = self.last[index] {
                    self.isi.push(event.at_us - last);
                }
                self.last[index] = Some(event.at_us);
            }
        }
    }

    pub fn report(&self) -> Value {
        let mut sorted = self.isi.clone();
        sorted.sort();
        let near = sorted
            .iter()
            .filter(|&&x| (10_000..=11_000).contains(&x))
            .count();
        json!({"windows":self.windows,"selected_actions":self.chosen,"selection_rate":self.chosen as f64/self.windows.max(1) as f64,
            "a":self.a,"d":self.d,"visible_moves":self.moved,"silent_windows":self.silence,"count_ties":self.ties,
            "four_four_ties":self.four_four,"ties_with_different_spike_trains":self.temporal_distinct,
            "ties_with_different_first_spikes":self.first_distinct,"positive_motor_arrivals":self.positive,
            "negative_motor_arrivals":self.negative,"arrivals_during_refractory":self.refractory,
            "arrival_refractory_fraction":self.refractory as f64/(self.positive+self.negative).max(1) as f64,
            "positive_arrival_amplitude_sum":self.positive_amplitude,"negative_arrival_amplitude_sum":self.negative_amplitude,
            "motor_refractory_us":10_000,"isi_samples":sorted.len(),"minimum_isi_us":sorted.first(),
            "median_isi_us":sorted.get(sorted.len()/2),"isi_in_10000_to_11000_us":near,
            "near_refractory_isi_fraction":near as f64/sorted.len().max(1) as f64})
    }
}

/// Calibration-only nearest centroids, with test confusion and macro recall.
/// The motion classes are -1/0/+1. Test-only classes remain errors.
pub fn score(features: &[Vec<f64>], labels: &[i16]) -> Value {
    if labels.len() < 8 || features.len() != labels.len() {
        return json!(null);
    }
    let split = labels.len() / 2;
    let mut classes: BTreeMap<i16, (Vec<f64>, usize)> = BTreeMap::new();
    for (feature, &label) in features[..split].iter().zip(&labels[..split]) {
        let (sum, n) = classes
            .entry(label)
            .or_insert_with(|| (vec![0.0; feature.len()], 0));
        for (sum, &x) in sum.iter_mut().zip(feature) {
            *sum += x;
        }
        *n += 1;
    }
    let majority = *classes.iter().max_by_key(|(_, (_, n))| *n).unwrap().0;
    let mut confusion = [[0usize; 3]; 3];
    let mut correct = 0;
    let mut majority_correct = 0;
    let mut unseen = 0;
    for (feature, &label) in features[split..].iter().zip(&labels[split..]) {
        let predicted = *classes
            .iter()
            .min_by(|(_, (a, na)), (_, (b, nb))| {
                let distance = |sum: &[f64], n: usize| {
                    feature
                        .iter()
                        .zip(sum)
                        .map(|(x, sum)| (x - sum / n as f64).powi(2))
                        .sum::<f64>()
                };
                distance(a, *na).total_cmp(&distance(b, *nb))
            })
            .unwrap()
            .0;
        correct += usize::from(label == predicted);
        majority_correct += usize::from(label == majority);
        unseen += usize::from(!classes.contains_key(&label));
        confusion[(label + 1) as usize][(predicted + 1) as usize] += 1;
    }
    let present: Vec<_> = (0..3)
        .filter(|&i| confusion[i].iter().sum::<usize>() > 0)
        .collect();
    let balanced = present
        .iter()
        .map(|&i| confusion[i][i] as f64 / confusion[i].iter().sum::<usize>() as f64)
        .sum::<f64>()
        / present.len() as f64;
    json!({"calibration_samples":split,"tested":labels.len()-split,"correct":correct,"accuracy":correct as f64/(labels.len()-split) as f64,
        "balanced_accuracy":balanced,"test_classes":present.len(),"calibration_classes":classes.len(),
        "majority_accuracy":majority_correct as f64/(labels.len()-split) as f64,
        "majority_balanced_accuracy":f64::from(present.contains(&((majority+1) as usize)))/present.len() as f64,
        "unseen_test_labels":unseen,"confusion_true_rows_predicted_columns":confusion})
}

pub fn pixel_features(frame: &Frame) -> crate::Result<Vec<f64>> {
    Ok(grayscale(frame, 16, 16)?
        .iter()
        .map(|&x| f64::from(x))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{KeyboardLayout, Model, ModelConfig};
    use nerva_qemu::vision_encoder::Encoding;

    #[test]
    fn balanced_accuracy_exposes_majority_bias_and_unseen_classes_are_errors() {
        let features = vec![vec![0.0]; 12];
        let labels = vec![0, 0, 0, -1, -1, 1, 0, 0, 0, 0, -1, 1];
        let result = score(&features, &labels);
        assert_eq!(result["test_classes"], 3);
        assert_eq!(result["majority_accuracy"], 4.0 / 6.0);
        assert_eq!(result["majority_balanced_accuracy"], 1.0 / 3.0);
        let labels = vec![0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1];
        let result = score(&features, &labels);
        assert_eq!(result["unseen_test_labels"], 6);
        assert_eq!(result["accuracy"], 0.0);
        assert_eq!(result["balanced_accuracy"], 0.0);
    }

    #[test]
    fn temporal_bins_preserve_neuron_identity_and_counts_for_both_visual_encodings() {
        let black = Frame::new(64, 64, vec![0; 64 * 64 * 3]).unwrap();
        let white = Frame::new(64, 64, vec![255; 64 * 64 * 3]).unwrap();
        for encoding in [Encoding::Brightness, Encoding::Change] {
            let mut model = Model::from_config(ModelConfig {
                encoding,
                keyboard: KeyboardLayout::Ad,
                processing_neurons: 16,
                ..ModelConfig::default().frozen()
            })
            .unwrap();
            model.step(&black).unwrap();
            let step = model.step(&white).unwrap();
            assert!(!step.core_events.is_empty());
            let (counts, timing) = features(&step, model.processing_ids());
            for (index, &count) in counts.iter().enumerate() {
                assert_eq!(
                    timing[index * 10..(index + 1) * 10].iter().sum::<f64>(),
                    count
                );
            }
        }
    }
}
