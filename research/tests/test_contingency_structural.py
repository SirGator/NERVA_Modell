from copy import deepcopy
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from contingency_structural_analysis import lifecycles, training_activity


class ContingencyStructuralTest(unittest.TestCase):
    def test_topology_reconstruction_rejects_early_or_mismatched_mutations(self):
        initial = {1: (2, 3, 1000, True)}
        final = {2: (3, 4, 1000, True)}
        frames = [
            {'end_us': 100_000, 'formed': [[2, 3, 4]], 'pruned': []},
            {'end_us': 200_000, 'formed': [], 'pruned': [1]},
        ]
        core = {2, 3, 4}
        edges, births, deaths = lifecycles(initial, final, frames, core, True)
        self.assertEqual(edges, initial | final)
        self.assertEqual(births, {2: 100_000})
        self.assertEqual(deaths, {1: 200_000})
        with self.assertRaisesRegex(ValueError, 'maintenance boundary'):
            lifecycles(initial, final, frames, core, False)
        with self.assertRaisesRegex(ValueError, 'does not reconstruct'):
            lifecycles(initial, {2: (4, 3, 1000, True)}, frames, core, True)

    def test_first_spike_checks_formed_edge_and_native_integration(self):
        trial = {
            'frames': [{'stimulus': 'cue0', 'core_spikes': [[2, 1100], [3, 2100], [4, 2200]]}],
            'arrivals': [[2100, 2, 2, 3, .5], [2200, 1, 3, 4, .25]],
            'quiet_integrations': [[2100, 3, .75, .5, 1., 0, True]],
            'tag_events': [[2200, 1, 'post', 0., .25]],
        }

        def analyze(data, birth=1099):
            return training_activity(data, {1: (3, 4, 100)}, {3},
                                     {1: (3, 4, 100, True), 2: (2, 3, 1000, True)},
                                     {2: birth}, {}, 1000, 51_000, 100_000, {2, 3, 4})[0]

        observed = analyze(trial)
        source = observed['quiet_sources'][3]
        self.assertEqual(source['new_input_arrivals'], 1)
        self.assertEqual(source['first_spike']['new_positive_arrivals'], [(2, .5)])
        self.assertEqual(source['first_spike']['candidate_without_new_positive_input_in_observed_batch'],
                         .75)
        self.assertEqual(observed['terminal_edges'][1]['new_tag_pairs'], [(2200, 'post', 2200)])
        with self.assertRaisesRegex(ValueError, 'no source spike after edge formation'):
            analyze(trial, 1100)
        corrupted = deepcopy(trial)
        corrupted['quiet_integrations'][0][3] = .25
        with self.assertRaisesRegex(ValueError, 'differs from arrivals'):
            analyze(corrupted)
