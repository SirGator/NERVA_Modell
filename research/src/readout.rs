//! External diagnostic readouts; never used as a neural learning rule or policy.
use std::collections::BTreeMap;

pub struct Readout {
    mean: Vec<f64>,
    scale: Vec<f64>,
    centers: BTreeMap<i16, Vec<f64>>,
}

impl Readout {
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

#[cfg(test)]
mod tests {
    use super::Readout;

    #[test]
    fn calibration_remains_separate_from_probes_at_arbitrary_dimensions() {
        for dimensions in [1, 17, 65] {
            let rows = vec![vec![-1.; dimensions], vec![1.; dimensions]];
            let readout = Readout::fit(&rows, &[-1, 1]);
            assert_eq!(readout.predict(&vec![-2.; dimensions]), -1);
            assert_eq!(readout.predict(&vec![2.; dimensions]), 1);
            assert_eq!(readout.predict(&vec![1e9; dimensions]), 1);
            assert_eq!(readout.predict(&vec![-2.; dimensions]), -1);
        }
        let readout = Readout::fit(
            &[vec![-1., 100.], vec![1., 100.], vec![3., 100.]],
            &[-1, 1, 3],
        );
        assert_eq!(readout.predict(&[-2., 999.]), -1);
        assert_eq!(readout.predict(&[2.9, -99.]), 3);
    }
}
