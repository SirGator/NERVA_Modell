from copy import deepcopy
from math import exp
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from contingency_electrical_analysis import f32
from contingency_structural_decisions_analysis import (
    NATIVE_RULES, compare_sample, formation_choice, pruning_choice, validate_formation_row,
)


class StructuralDecisionTest(unittest.TestCase):
    def setUp(self):
        self.rules = {key: f32(value) if isinstance(value, float) else value
                      for key, value in NATIVE_RULES.items()}

    def test_candidate_filters_before_affinity_then_ties_break_by_identity(self):
        now = 100_000
        row = {
            'drive': .75, 'target_trace': 1., 'target_last_spike_us': None,
            'neighbors': [
                [1, .125, .02, now, 10.],
                [2, 0., .1, now, 1.],
                [3, 0., .2, now, .5],
                [4, 0., .25, now, .5],
            ],
        }
        self.assertEqual(formation_choice(row, now, self.rules)[:3], ('formed', 3, 4))
        row['neighbors'].pop()
        self.assertEqual(formation_choice(row, now, self.rules)[:3], ('formed', 2, 2))
        row['neighbors'] = row['neighbors'][:1]
        self.assertEqual(formation_choice(row, now, self.rules)[:3], ('score_blocked', 0, None))
        row['neighbors'] = []
        self.assertEqual(formation_choice(row, now, self.rules)[:3], ('no_new_neighbor', 0, None))
        row['drive'] = .5
        self.assertEqual(formation_choice(row, now, self.rules)[:3], ('demand_blocked', None, None))

    def test_temporal_term_requires_two_recent_spikes(self):
        now = 1_000_000
        row = {
            'drive': .75, 'target_trace': 0., 'target_last_spike_us': None,
            'neighbors': [[2, .125, 1., now, 1.]],
        }
        outcome, _, _, info = formation_choice(row, now, self.rules, checkpoint=10)
        self.assertEqual(outcome, 'score_blocked')
        self.assertEqual(info['unilateral_source_trace_neighbors'], 1)
        self.assertEqual(info['unilateral_source_task_spike_neighbors'], 1)
        row['target_last_spike_us'] = now
        self.assertEqual(formation_choice(row, now, self.rules)[:3], ('formed', 1, 2))
        row['target_last_spike_us'] = 10
        self.assertEqual(formation_choice(row, now, self.rules)[:3], ('score_blocked', 0, None))

    def test_pruning_reconstructs_crossing_then_continuous_duration(self):
        row = {
            'supported': True, 'stored_utility': .02, 'updated_us': 100_000,
            'decayed_utility': .007357589, 'below_since_us': 446_574, 'selected': False,
        }
        previous, selected = pruning_choice(row, 600_000, self.rules, None)
        self.assertFalse(selected)
        self.assertEqual(previous[1], 446_574)
        corrupted = {**row, 'below_since_us': 100_000, 'selected': True}
        with self.assertRaisesRegex(ValueError, 'differs from native timer'):
            pruning_choice(corrupted, 600_000, self.rules, None)
        row.update(decayed_utility=.005449954, selected=True)
        with self.assertRaisesRegex(ValueError, 'decayed utility differs'):
            pruning_choice(row, 746_574, self.rules, previous)
        # The stored utility is twice the threshold, and at the expiry time has
        # decayed for 300 ms beyond their crossing.
        def decayed(now):
            return f32(f32(.02) * f32(exp(f32(-(now - 100_000) / 500_000))))

        now = 746_574
        row['decayed_utility'] = decayed(now)
        self.assertTrue(pruning_choice(row, now, self.rules, previous)[1])
        # One microsecond earlier the cached timer still has not reached expiry.
        row['decayed_utility'] = decayed(now - 1)
        with self.assertRaisesRegex(ValueError, 'differs from native duration'):
            pruning_choice(row, now - 1, self.rules, previous)

    def test_missing_utility_inherits_only_a_plausible_checkpoint_timer(self):
        row = {
            'supported': True, 'stored_utility': 0., 'updated_us': None,
            'decayed_utility': 0., 'below_since_us': 899_475, 'selected': False,
        }
        previous, selected = pruning_choice(row, 1_099_475, self.rules, None, checkpoint=1_000_001)
        self.assertFalse(selected)
        row.update(selected=True)
        self.assertTrue(pruning_choice(row, 1_199_475, self.rules, previous)[1])
        corrupted = {**row, 'below_since_us': 899_474}
        with self.assertRaisesRegex(ValueError, 'differs from native timer'):
            pruning_choice(corrupted, 1_199_475, self.rules, previous)

    def test_geometry_omissions_and_spike_tampering_are_detected(self):
        row = {
            'target': 1, 'target_trace': 0., 'target_last_spike_us': None,
            'neighbors': [[2, .125, .5, 15, 1.]],
        }

        def verify(record):
            validate_formation_row(record, {2: (.125, 1.)}, {}, {2: [15]}, {}, 20, 10)

        verify(row)
        with self.assertRaisesRegex(ValueError, 'neighbor IDs differ'):
            verify({**row, 'neighbors': []})
        corrupted = deepcopy(row)
        corrupted['neighbors'][0][3] = 14
        with self.assertRaisesRegex(ValueError, 'last spike differs'):
            verify(corrupted)
        self.assertEqual(compare_sample(range(24)), list(range(0, 24, 2)))


if __name__ == '__main__':
    unittest.main()
