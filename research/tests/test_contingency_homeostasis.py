from copy import deepcopy
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from contingency_homeostasis_analysis import focus_activity, homeostasis_chain
from verify_contingency import FRAME_US


class ContingencyHomeostasisTest(unittest.TestCase):
    def test_pair_events_use_latest_partner_and_task_emission(self):
        focus = {1: (8, 9, 10)}
        trial = {
            'frames': [{
                'stimulus': 'cue0',
                'core_spikes': [[9, 100], [9, 105], [8, 106], [9, 116]],
            }],
            'pre_arrivals': [[104, 1], [116, 1]],
            'tag_events': [
                [104, 1, 'pre', 1., .5],
                [105, 1, 'post', .5, .75],
                [116, 1, 'pre', .75, .5],
                [116, 1, 'post', .5, .75],
            ],
        }
        result = focus_activity(trial, focus, 100, 100 + FRAME_US, 11)[1]
        self.assertEqual(result['native_arrivals'], 2)
        self.assertEqual(result['task_arrivals'], 1)
        self.assertEqual(result['new_tag_events'], 2)
        self.assertEqual(result['new_tag_pairs'], [(116, 'pre', 105), (116, 'post', 116)])
        self.assertEqual(result['nonblank_source_spikes'], 1)
        with self.assertRaisesRegex(ValueError, 'outside the STDP window'):
            focus_activity(trial, focus, 100, 100 + FRAME_US, 10)
        for event in ([117, 1, 'post', .5, .75], [116, 1, 'post', .75, .5]):
            corrupted = deepcopy(trial)
            corrupted['tag_events'][-1] = event
            with self.assertRaisesRegex(ValueError, 'wrong sign or no target spike'):
                focus_activity(corrupted, focus, 100, 100 + FRAME_US, 11)

    def test_homeostasis_chain_checks_scope_and_state(self):
        initial = {8: (1., 2.), 9: (1., 1.)}
        trial = {'homeostasis_events': [[100, 8, 1., 1.5, 2., 3.]],
                 'final_core': [[8, 1.5, 3.], [9, 1., 1.]]}
        self.assertEqual(homeostasis_chain(trial, initial, True, 100, 200), 1)
        corrupted = deepcopy(trial)
        corrupted['homeostasis_events'][0][1] = 10
        with self.assertRaisesRegex(ValueError, 'unapproved cell'):
            homeostasis_chain(corrupted, initial, True, 100, 200)
        corrupted = deepcopy(trial)
        corrupted['homeostasis_events'][0][4] = 3.
        with self.assertRaisesRegex(ValueError, 'broken homeostasis update chain'):
            homeostasis_chain(corrupted, initial, True, 100, 200)
        with self.assertRaisesRegex(ValueError, 'update presence differs'):
            homeostasis_chain(trial, initial, False, 100, 200)
