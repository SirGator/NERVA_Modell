//! External diagnostic readouts; never used as a neural learning rule or policy.
use serde_json::{Value, json};
use std::collections::BTreeMap;

pub struct Readout {
    mean: Vec<f64>,
    scale: Vec<f64>,
    centers: BTreeMap<i16, Vec<f64>>,
}
impl Readout {
    /// Read-only external decoder parameters for independent probe audits.
    pub(crate) fn snapshot(&self) -> Value {
        json!({"mean":self.mean,"scale":self.scale,"centers":self.centers})
    }

    pub fn fit(features: &[Vec<f64>], labels: &[i16]) -> Self {
        assert!(!features.is_empty() && features.len() == labels.len());
        let n = features.len() as f64;
        let size = features[0].len();
        let mut mean = vec![0.0; size];
        for row in features {
            for (sum, &x) in mean.iter_mut().zip(row) {
                *sum += x / n;
            }
        }
        let mut scale = vec![0.0; size];
        for row in features {
            for ((sum, &x), &m) in scale.iter_mut().zip(row).zip(&mean) {
                *sum += (x - m).powi(2) / n;
            }
        }
        for s in &mut scale {
            *s = s.sqrt();
            if *s < 1e-9 {
                *s = 1.0;
            }
        }
        let mut centers: BTreeMap<i16, (Vec<f64>, usize)> = BTreeMap::new();
        for (row, &label) in features.iter().zip(labels) {
            let (sum, count) = centers.entry(label).or_insert_with(|| (vec![0.0; size], 0));
            for (((sum, &x), &m), &s) in sum.iter_mut().zip(row).zip(&mean).zip(&scale) {
                *sum += (x - m) / s;
            }
            *count += 1;
        }
        Self {
            mean,
            scale,
            centers: centers
                .into_iter()
                .map(|(label, (mut sum, count))| {
                    for x in &mut sum {
                        *x /= count as f64;
                    }
                    (label, sum)
                })
                .collect(),
        }
    }
    pub fn predict(&self, row: &[f64]) -> i16 {
        *self
            .centers
            .iter()
            .min_by(|(_, a), (_, b)| {
                let distance = |center: &[f64]| {
                    row.iter()
                        .zip(&self.mean)
                        .zip(&self.scale)
                        .zip(center)
                        .map(|(((&x, &m), &s), &c)| ((x - m) / s - c).powi(2))
                        .sum::<f64>()
                };
                distance(a).total_cmp(&distance(b))
            })
            .unwrap()
            .0
    }
}

pub fn score(truth: &[i16], predicted: &[i16]) -> Value {
    assert_eq!(truth.len(), predicted.len());
    let mut classes: BTreeMap<i16, (usize, usize)> = BTreeMap::new();
    let mut correct = 0;
    for (&truth, &prediction) in truth.iter().zip(predicted) {
        let (c, n) = classes.entry(truth).or_default();
        *n += 1;
        *c += usize::from(truth == prediction);
        correct += usize::from(truth == prediction);
    }
    json!({"tested":truth.len(),"correct":correct,"accuracy":correct as f64/truth.len().max(1) as f64,
        "balanced_accuracy":classes.values().map(|&(c,n)|c as f64/n as f64).sum::<f64>()/classes.len().max(1) as f64,
        "classes":classes.len(),"per_class":classes.iter().map(|(&label,&(correct,n))|json!({"label":label,"correct":correct,"tested":n})).collect::<Vec<_>>()})
}

pub fn chronological(features: &[Vec<f64>], labels: &[i16]) -> Value {
    let split = labels.len() / 2;
    let readout = Readout::fit(&features[..split], &labels[..split]);
    score(
        &labels[split..],
        &features[split..]
            .iter()
            .map(|row| readout.predict(row))
            .collect::<Vec<_>>(),
    )
}

pub fn core_features(
    step: &crate::model::Step,
    ids: &[nerva::core::NeuronId],
    temporal: bool,
) -> Vec<f64> {
    if !temporal {
        return step.core_counts.iter().map(|&x| x as f64).collect();
    }
    let mut features = vec![0.0; ids.len() * 8];
    let duration = step.end_us - step.start_us + 1;
    for &(id, time) in &step.core_events {
        if let Ok(index) = ids.binary_search(&nerva::core::NeuronId(id)) {
            let bin = ((time - step.start_us) * 8 / duration).min(7) as usize;
            features[index * 8 + bin] += 1.0;
        }
    }
    features
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fit_and_test_are_separate_and_scores_do_not_reward_majority_imbalance() {
        let rows = vec![
            vec![-1.0, 100.0],
            vec![-1.0, 100.0],
            vec![1.0, 100.0],
            vec![1.0, 100.0],
        ];
        let readout = Readout::fit(&rows, &[-1, -1, 1, 1]);
        assert_eq!(readout.predict(&[-2.0, 999.0]), -1);
        assert_eq!(readout.predict(&[2.0, -99.0]), 1);
        let s = score(&[-1, -1, -1, 1], &[-1, -1, -1, -1]);
        assert_eq!(s["accuracy"], 0.75);
        assert_eq!(s["balanced_accuracy"], 0.5);
    }
}
