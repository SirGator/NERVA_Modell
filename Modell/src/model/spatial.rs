//! Immutable geometric index for the model's stationary processing cells.
//! Queries return the same ID-ordered neighborhood as a full network scan.
use nerva::{core::NeuronId, math::Position3D};
use std::collections::BTreeMap;

const CELL: f32 = 0.25;

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct SpatialIndex {
    buckets: BTreeMap<(i64, i64, i64), Vec<(NeuronId, Position3D)>>,
}

fn cell(p: Position3D) -> (i64, i64, i64) {
    (
        (p.x / CELL).floor() as i64,
        (p.y / CELL).floor() as i64,
        (p.z / CELL).floor() as i64,
    )
}

impl SpatialIndex {
    pub(crate) fn new(cells: impl IntoIterator<Item = (NeuronId, Position3D)>) -> Self {
        let mut index = Self::default();
        for (id, position) in cells {
            index
                .buckets
                .entry(cell(position))
                .or_default()
                .push((id, position));
        }
        index
    }

    pub(crate) fn within(&self, position: Position3D, radius: f32) -> Vec<NeuronId> {
        let (cx, cy, cz) = cell(position);
        // One guard bucket retains points whose f32 distance rounds down onto
        // the radius even just across the mathematical bucket boundary.
        let reach = (radius / CELL).ceil() as i64 + 1;
        let mut result = Vec::new();
        for x in cx - reach..=cx + reach {
            for y in cy - reach..=cy + reach {
                for z in cz - reach..=cz + reach {
                    if let Some(cells) = self.buckets.get(&(x, y, z)) {
                        for &(id, p) in cells {
                            if p.distance_to(position) <= radius {
                                result.push(id);
                            }
                        }
                    }
                }
            }
        }
        result.sort_unstable();
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_matches_full_scan_at_negative_coordinates_and_cell_boundaries() {
        let cells: Vec<_> = (0..173)
            .map(|i| {
                (
                    NeuronId(i),
                    Position3D::new(
                        (i % 11) as f32 * 0.125 - 0.75,
                        ((i / 11) % 7) as f32 * 0.125 - 0.5,
                        (i / 77) as f32 * 0.25 - 0.25,
                    ),
                )
            })
            .collect();
        let index = SpatialIndex::new(cells.clone());
        for (_, p) in &cells {
            for radius in [0., 0.125, 0.25, 0.5] {
                let expected: Vec<_> = cells
                    .iter()
                    .filter(|(_, q)| q.distance_to(*p) <= radius)
                    .map(|(id, _)| *id)
                    .collect();
                assert_eq!(index.within(*p, radius), expected);
            }
        }
    }

    #[test]
    fn rounded_distance_at_a_bucket_boundary_is_not_missed() {
        let query = Position3D::new(f32::from_bits(0.25f32.to_bits() - 1), 0., 0.);
        let point = Position3D::new(0.5, 0., 0.);
        assert!(point.distance_to(query) <= 0.25);
        assert_eq!(
            SpatialIndex::new([(NeuronId(7), point)]).within(query, 0.25),
            vec![NeuronId(7)]
        );
    }
}
