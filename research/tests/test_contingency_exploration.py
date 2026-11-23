from copy import deepcopy
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from contingency_exploration_analysis import expected_ablations, local_distances, verify_births
from contingency_exploration_analysis import verify_delivery


class ContingencyExplorationTest(unittest.TestCase):
    def test_formation_uses_pre_pruning_neighborhood_and_policy(self):
        initial = {1: (4, 3, 1000, True)}
        last_spikes = {1: 30_000, 2: None, 3: None, 4: 49_000}
        distances = local_distances(
            {1: (.20, 0, 0), 2: (.05, 0, 0), 3: (0, 0, 0), 4: (.01, 0, 0)},
            {1, 2, 4},
        )
        base = {
            'frames': [{'start_us': 150_001, 'end_us': 200_000, 'root_spikes': [],
                        'core_spikes': [[1, 190_000], [4, 195_000]],
                        'formed': [[2, 1, 3]], 'pruned': [1]}],
            'exploratory': [[2, 199_000, 190_000, None, 1, .6]],
        }

        def analyze(trial, policy):
            return verify_births(trial, initial, {2: 200_000}, last_spikes, distances, policy)

        self.assertEqual(analyze(base, 'recent')[2]['source'], 1)
        with self.assertRaisesRegex(ValueError, 'differs from policy choice'):
            analyze(base, 'nearest')
        nearest = deepcopy(base)
        nearest['frames'][0]['formed'][0][1] = 2
        nearest['exploratory'][0][2] = None
        self.assertEqual(analyze(nearest, 'nearest')[2]['source'], 2)
        nearest['exploratory'][0][4] = 2
        with self.assertRaisesRegex(ValueError, 'recent free excitatory neighborhood'):
            analyze(nearest, 'nearest')

    def test_exploratory_delivery_is_complete_and_matches_quiet_capture(self):
        trial = {
            'frames': [{'core_spikes': [[1, 101_000], [1, 102_000], [1, 102_500]]}],
            'arrivals': [[102_000, 2, 1, 3, .5], [103_000, 2, 1, 3, .25]],
            'exploratory_arrivals': [[102_000, 2, 1, 3, .5], [103_000, 2, 1, 3, .25]],
        }

        def analyze(data):
            return verify_delivery(data, {2: {}}, {2: (1, 3, 1000, True)},
                                   {2: 100_000}, {2: 103_000}, 50_000, 150_000, {}, {3})

        self.assertEqual(analyze(trial)[2], [(102_000, .5), (103_000, .25)])
        missing = deepcopy(trial)
        missing['exploratory_arrivals'].pop()
        with self.assertRaisesRegex(ValueError, 'omit emissions'):
            analyze(missing)
        mismatch = deepcopy(trial)
        mismatch['exploratory_arrivals'][0][-1] = .75
        with self.assertRaisesRegex(ValueError, 'quiet capture'):
            analyze(mismatch)

    def test_acute_ablation_demands_win_over_both_matched_controls(self):
        controls = ('native_prune', 'native_retain', 'nearest_prune', 'nearest_retain')
        gates = {arm: {'forward': {'cue0': {'correct': 2}}} for arm in controls}
        surviving = {}
        for arm in ('recent_prune', 'recent_retain'):
            gates[arm] = {
                'forward': {'cue0': {'correct': 3, 'passed': True},
                            'cue1': {'correct': 0, 'passed': False}},
                'reverse': {'cue0': {'correct': 0, 'passed': False},
                            'cue1': {'correct': 0, 'passed': False}},
            }
            surviving[arm] = {'forward': [1], 'reverse': []}
        gates['nearest_retain']['forward']['cue0']['correct'] = 3
        self.assertEqual(expected_ablations(gates, surviving), [('recent_prune', 'forward', 'cue0')])
