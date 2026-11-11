//! Norm-controlled rays. Bound violations never silently rotate a direction.
use nerva::core::SynapseId;
use serde_json::{Value, json};
pub(in crate::scenarios::hippocampal) struct Plan {
    pub weights: Option<Vec<(SynapseId, f32)>>,
    pub details: Value,
}
pub(in crate::scenarios::hippocampal) fn norm(vector: &[f64]) -> f64 {
    vector.iter().map(|v| v * v).sum::<f64>().sqrt()
}
pub(in crate::scenarios::hippocampal) fn limit(
    initial: &[(SynapseId, f32)],
    direction: &[f64],
) -> Option<f64> {
    assert_eq!(initial.len(), direction.len());
    let length = norm(direction);
    if length == 0. {
        return None;
    }
    Some(
        initial
            .iter()
            .zip(direction)
            .filter(|(_, d)| **d != 0.)
            .map(|((_, w), &d)| {
                let unit = d / length;
                if unit > 0. {
                    (2. - f64::from(*w)) / unit
                } else {
                    -f64::from(*w) / unit
                }
            })
            .fold(f64::INFINITY, f64::min),
    )
}
pub(in crate::scenarios::hippocampal) fn plan(
    initial: &[(SynapseId, f32)],
    direction: &[f64],
    requested: f64,
) -> Plan {
    let length = norm(direction);
    let maximum = limit(initial, direction);
    if length == 0. || !requested.is_finite() || requested < 0. {
        return Plan {
            weights: None,
            details: json!({"requested_norm":requested,"ray_limit":maximum,
            "feasible":false,"reason":"undefined_direction_or_norm","faithful":false}),
        };
    }
    let maximum = maximum.unwrap();
    if requested > maximum + 1e-12 * maximum.max(1.) {
        return Plan {
            weights: None,
            details: json!({"requested_norm":requested,"ray_limit":maximum,
            "feasible":false,"reason":"weight_bounds","faithful":false}),
        };
    }
    let publish = |scale: f64| {
        initial
            .iter()
            .zip(direction)
            .map(|(&(id, w), &d)| {
                let raw = f64::from(w)
                    + if scale == length {
                        d
                    } else {
                        scale * (d / length)
                    };
                assert!((-1e-12..=2. + 1e-12).contains(&raw));
                (id, raw.clamp(0., 2.) as f32)
            })
            .collect::<Vec<_>>()
    };
    let magnitude = |weights: &[(SynapseId, f32)]| {
        norm(
            &weights
                .iter()
                .zip(initial)
                .map(|((_, w), (_, w0))| f64::from(*w) - f64::from(*w0))
                .collect::<Vec<_>>(),
        )
    };
    let mut ray_norm = requested;
    let mut weights = publish(ray_norm);
    let mut distance = (magnitude(&weights) - requested).abs();
    let mut iterations = 0;
    // Calibrate only publication, never spike responses. f32 magnitude along a
    // fixed ray is monotone, so a bounded bisection can reduce rounding mismatch.
    if requested > 0. && distance / requested > 1e-4 {
        let mut low = 0.;
        let mut high = maximum;
        for _ in 0..48 {
            let middle = (low + high) / 2.;
            let candidate = publish(middle);
            let actual = magnitude(&candidate);
            let error = (actual - requested).abs();
            if error < distance {
                distance = error;
                ray_norm = middle;
                weights = candidate;
            }
            if actual < requested {
                low = middle;
            } else {
                high = middle;
            }
            iterations += 1;
        }
    }
    let delta: Vec<_> = weights
        .iter()
        .zip(initial)
        .map(|((_, w), (_, w0))| f64::from(*w) - f64::from(*w0))
        .collect();
    let rounding: Vec<_> = weights
        .iter()
        .zip(initial)
        .zip(direction)
        .map(|(((_, w), (_, w0)), &d)| {
            f64::from(*w)
                - (f64::from(*w0)
                    + if ray_norm == length {
                        d
                    } else {
                        ray_norm * (d / length)
                    })
        })
        .collect();
    let actual = norm(&delta);
    let cosine = if actual > 0. {
        Some(
            (delta.iter().zip(direction).map(|(a, b)| a * b).sum::<f64>() / (actual * length))
                .clamp(-1., 1.),
        )
    } else {
        None
    };
    let relative = if requested > 0. {
        (actual - requested).abs() / requested
    } else {
        0.
    };
    let faithful = relative <= 0.01 && cosine.is_some_and(|c| c >= 0.999);
    Plan {
        weights: Some(weights),
        details: json!({"requested_norm":requested,"ray_limit":maximum,
        "feasible":true,"reason":null,"ray_norm":ray_norm,"normalization_iterations":iterations,"actual_norm":actual,"relative_norm_error":relative,
        "direction_cosine":cosine,"rounding_l2":norm(&rounding),"faithful":faithful}),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn normalization_preserves_sign_and_strength_and_rejects_clipping() {
        let initial = [(SynapseId(1), 1.), (SynapseId(2), 1.)];
        let direction = [0.3, -0.4];
        let result = plan(&initial, &direction, 1.2);
        assert_eq!(result.details["faithful"], true);
        assert!((result.details["actual_norm"].as_f64().unwrap() - 1.2).abs() < 1e-7);
        assert_eq!(
            result.weights.unwrap(),
            vec![(SynapseId(1), 1.72), (SynapseId(2), 0.04)]
        );
        let blocked = plan(&initial, &direction, 2.);
        assert!(blocked.weights.is_none());
        assert_eq!(blocked.details["reason"], "weight_bounds");
        assert!(plan(&initial, &[0., 0.], 1.).weights.is_none());
    }
    #[test]
    fn low_publication_precision_is_visible_instead_of_becoming_a_matched_arm() {
        let result = plan(&[(SynapseId(1), 1.)], &[1.], 1e-12);
        assert!(result.weights.is_some());
        assert_eq!(result.details["actual_norm"], 0.);
        assert_eq!(result.details["faithful"], false);
        assert!(plan(&[(SynapseId(1), 0.)], &[-1.], 0.1).weights.is_none());
    }
}
