from copy import deepcopy
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from contingency_deferred_growth_analysis import (DEFER_US, deferred_choices, layout,
                                                   verify_closed_prefix)
from verify_contingency import FRAME_US, TRAINING


class DeferredGrowthTest(unittest.TestCase):
    def test_two_layouts_preserve_all_pixels(self):
        for seed, label in ((34601, 'column_stride'), (34627, 'row_stride')):
            name, stimuli = layout(seed)
            self.assertEqual(name, label)
            self.assertEqual(sorted(pixel for image in stimuli.values() for pixel in image), list(range(64)))

    def test_birth_at_opening_is_allowed_but_early_birth_is_rejected(self):
        start = 100
        frame = dict(formed=[[91, 64, 65]], exploratory=[[91, start + DEFER_US, 42, None, 1, .1]],
                     lead_choices=[[91, 64, 0, 0]])
        ledger = deferred_choices([frame], 'recent_deferred', {}, {64, 65}, start)
        self.assertEqual((ledger['before_open'], ledger['after_open']), (0, 1))
        frame['exploratory'][0][1] -= 1
        with self.assertRaisesRegex(ValueError, 'before opening'):
            deferred_choices([frame], 'recent_deferred', {}, {64, 65}, start)
        ledger = deferred_choices([frame], 'recent', {}, {64, 65}, start)
        self.assertEqual((ledger['before_open'], ledger['after_open']), (1, 0))

    def test_closed_trajectory_cannot_differ_between_source_rules(self):
        start = 100
        frames = [dict(end_us=start + end - 1, core_spikes=[], lead_choices=[])
                  for end in range(FRAME_US, DEFER_US + FRAME_US, FRAME_US)]
        trials = {policy: {'a': {mapping: {'frames': deepcopy(frames)} for mapping in TRAINING}}
                  for policy in ('recent_deferred', 'lead_deferred')}
        self.assertEqual(verify_closed_prefix(trials, start), 30)
        trials['lead_deferred']['a']['forward']['frames'][0]['core_spikes'] = [[64, start]]
        with self.assertRaisesRegex(ValueError, 'trajectories differ'):
            verify_closed_prefix(trials, start)


if __name__ == '__main__':
    unittest.main()
