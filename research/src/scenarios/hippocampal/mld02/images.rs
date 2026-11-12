use nerva_qemu::capture::Frame;
pub const PROBES: [usize; 9] = [4, 0, 1, 2, 3, 5, 6, 7, 8];
fn make(points: &[(usize, usize)]) -> nerva_qemu::Result<Frame> {
    let mut pixels = vec![0u8; 64];
    for &(x, y) in points {
        pixels[y * 8 + x] = 255;
    }
    Frame::new(8, 8, pixels.into_iter().flat_map(|v| [v; 3]).collect())
}
pub fn probe(kind: usize) -> nerva_qemu::Result<Frame> {
    match kind {
        5 => make(&[(5, 5)]),
        6 => make(&[(5, 1)]),
        7 => make(&[(6, 5)]),
        8 => make(&[(6, 1)]),
        _ => super::super::frame(kind),
    }
}
pub fn distractor(kind: usize) -> nerva_qemu::Result<Frame> {
    let common = [(1, 5), (2, 5), (1, 6), (2, 6), (3, 5), (3, 6)];
    let a = [(5, 3), (6, 3)];
    let b = [(5, 7), (6, 7)];
    let points: Vec<_> = match kind {
        0 => common.into_iter().chain(a).collect(),
        1 => common.into_iter().chain(b).collect(),
        2 => a.to_vec(),
        3 => b.to_vec(),
        _ => Vec::new(),
    };
    make(&points)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn active(frame: Frame) -> std::collections::BTreeSet<usize> {
        frame
            .rgb()
            .as_chunks::<3>()
            .0
            .iter()
            .enumerate()
            .filter(|(_, p)| p[0] > 0)
            .map(|(i, _)| i)
            .collect()
    }
    #[test]
    fn distractor_channels_are_disjoint_and_novel_fragments_were_not_construction_images() {
        let original = (0..4)
            .flat_map(|i| active(probe(i).unwrap()))
            .collect::<std::collections::BTreeSet<_>>();
        let other = (0..4)
            .flat_map(|i| active(distractor(i).unwrap()))
            .collect::<std::collections::BTreeSet<_>>();
        assert!(original.is_disjoint(&other));
        for i in 0..4 {
            assert_eq!(
                active(probe(i).unwrap()).len(),
                active(distractor(i).unwrap()).len()
            );
        }
        for i in 5..9 {
            let cue = active(probe(i).unwrap());
            assert_eq!(cue.len(), 1);
            assert!(cue.is_subset(&active(probe(2 + (i - 5) % 2).unwrap())));
        }
    }
}
