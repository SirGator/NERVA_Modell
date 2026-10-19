use crate::model::random::{next_random, shuffle};
use nerva_qemu::keyboard::Key;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Trial {
    pub key: Key,
    pub shown_motion: i16,
}
/// Every four trials have two A/two D. Shuffled training has all four
/// action/outcome combinations exactly once, preserving both marginals.
pub fn schedule(seed: u64, count: usize, reversed: bool, scrambled: bool) -> Vec<Trial> {
    assert!(count.is_multiple_of(4));
    let mut random = seed;
    let mut effect_random = seed ^ 0xd631a9f8;
    let mut result = Vec::new();
    for _ in 0..count / 4 {
        let mut keys = [Key::A, Key::A, Key::D, Key::D];
        shuffle(&mut keys, &mut random);
        let mut labels = keys.map(|key| if (key == Key::D) ^ reversed { 1 } else { -1 });
        if scrambled {
            for key in [Key::A, Key::D] {
                let indices: Vec<_> = keys
                    .iter()
                    .enumerate()
                    .filter(|&(_, candidate)| *candidate == key)
                    .map(|(i, _)| i)
                    .collect();
                let first = if next_random(&mut effect_random).is_multiple_of(2) {
                    -1
                } else {
                    1
                };
                labels[indices[0]] = first;
                labels[indices[1]] = -first;
            }
        }
        result.extend(
            keys.into_iter()
                .zip(labels)
                .map(|(key, shown_motion)| Trial { key, shown_motion }),
        );
    }
    result
}
