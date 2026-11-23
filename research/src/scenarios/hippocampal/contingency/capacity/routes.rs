use std::collections::{BTreeMap, BTreeSet};

pub(super) struct Edge {
    pub id: u64,
    pub pre: u64,
    pub post: u64,
    pub initial: f32,
    pub eligible: bool,
}

pub(super) struct Routes {
    pub direct: [BTreeSet<u64>; 2],
    pub via_relay: [BTreeSet<u64>; 2],
}

pub(super) fn routes(
    edges: &[Edge],
    cue_arrivals: &[BTreeSet<u64>; 2],
    blank_arrivals: &BTreeSet<u64>,
    quiet_relays: &BTreeSet<u64>,
    targets: &[BTreeSet<u64>; 2],
    reverse: bool,
) -> Routes {
    let mut direct = [BTreeSet::new(), BTreeSet::new()];
    let mut via_relay = [BTreeSet::new(), BTreeSet::new()];

    for cue in 0..2 {
        let mapped_targets = &targets[cue ^ usize::from(reverse)];
        let mut relay_outputs = BTreeMap::<u64, Vec<u64>>::new();
        for edge in edges.iter().filter(|edge| edge.eligible) {
            if quiet_relays.contains(&edge.pre) && mapped_targets.contains(&edge.post) {
                relay_outputs.entry(edge.pre).or_default().push(edge.id);
            }
        }

        let mut reached_relays = BTreeSet::new();
        for edge in edges.iter().filter(|edge| edge.eligible) {
            if !cue_arrivals[cue].contains(&edge.id)
                || cue_arrivals[1 - cue].contains(&edge.id)
                || blank_arrivals.contains(&edge.id)
            {
                continue;
            }
            if mapped_targets.contains(&edge.post) {
                direct[cue].insert(edge.id);
            }
            if let Some(outputs) = relay_outputs.get(&edge.post) {
                via_relay[cue].insert(edge.id);
                if reached_relays.insert(edge.post) {
                    via_relay[cue].extend(outputs.iter().copied());
                }
            }
        }
        via_relay[cue].extend(direct[cue].iter().copied());
    }

    Routes { direct, via_relay }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes_use_cue_specific_arrivals_and_require_complete_relay_paths() {
        let edges = [
            (1, 10, 20, true),
            (2, 10, 21, true),
            (3, 10, 20, true),
            (4, 10, 20, true),
            (5, 10, 30, true),
            (6, 30, 20, true),
            (7, 30, 21, true),
            (8, 10, 31, true),
            (9, 31, 20, false),
            (10, 10, 32, false),
            (11, 32, 20, true),
            (12, 11, 21, true),
        ]
        .map(|(id, pre, post, eligible)| Edge {
            id,
            pre,
            post,
            initial: 1.0,
            eligible,
        });
        let cues = [
            BTreeSet::from([1, 2, 3, 4, 5, 8, 10]),
            BTreeSet::from([3, 12]),
        ];
        let blank = BTreeSet::from([4]);
        let relays = BTreeSet::from([30, 31, 32]);
        let targets = [BTreeSet::from([20]), BTreeSet::from([21])];

        let natural = routes(&edges, &cues, &blank, &relays, &targets, false);
        assert_eq!(natural.direct, [BTreeSet::from([1]), BTreeSet::from([12])]);
        assert_eq!(
            natural.via_relay,
            [BTreeSet::from([1, 5, 6]), BTreeSet::from([12])]
        );

        let reversed = routes(&edges, &cues, &blank, &relays, &targets, true);
        assert_eq!(reversed.direct, [BTreeSet::from([2]), BTreeSet::new()]);
        assert_eq!(
            reversed.via_relay,
            [BTreeSet::from([2, 5, 7]), BTreeSet::new()]
        );
    }
}
