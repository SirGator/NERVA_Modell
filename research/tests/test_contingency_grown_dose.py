from copy import deepcopy
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from contingency_grown_dose_analysis import (
    bounded_plan, headrooms, permutation, ray_limit, score, score_summary,
    spike_comparisons, target_margins,
)


class GrownDoseTests(unittest.TestCase):
    def test_fixed_permutation_is_shared_and_preserves_target_support(self):
        targets = [4, 2, 2, 4, 3, 2, 4, 2, 4, 2, 4]
        indices = permutation(34123, targets)
        self.assertEqual(indices, [3, 5, 2, 10, 4, 9, 0, 1, 8, 7, 6])
        self.assertEqual(sorted(indices), list(range(len(targets))))
        self.assertEqual([targets[index] for index in indices], targets)

    def test_bounded_dose_can_exceed_ray_limit_and_preserves_zero_coordinates(self):
        initial, direction = [1.875, .5, 1.], [1., -1., 0.]
        room = headrooms(initial, direction)
        self.assertEqual(room, [.125, .5, 0.])
        self.assertLess(ray_limit(room, direction), .4)
        weights, plan = bounded_plan(initial, direction, .4)
        self.assertEqual(weights[0], 2.)
        self.assertLess(weights[1], initial[1])
        self.assertEqual(weights[2], initial[2])
        self.assertEqual(plan['clipped_coordinates'], 1)
        self.assertLess(plan['direction_cosine'], .99)
        self.assertLess(plan['relative_norm_error'], 1e-6)

    def test_learned_gates_require_both_controls_across_both_orientations(self):
        baseline = {
            cue: {'core_spikes': [], 'rest_core_spikes': 0} for cue in ('cue0', 'cue1', 'blank')
        }
        targets = [{10}, {11}]
        arms = {name: deepcopy(baseline) for name in (
            'forward', 'reverse', 'balanced', 'shuffle_forward', 'shuffle_reverse'
        )}
        for mapping, ids in (('forward', (10, 11)), ('reverse', (11, 10))):
            for index, cell in enumerate(ids):
                arms[mapping][f'cue{index}']['core_spikes'] = [[cell, 10]]
        self.assertTrue(score_summary(score(
            arms, baseline, targets, controls=('balanced', 'shuffle_{mapping}')))['complete'])
        arms['shuffle_reverse']['cue0']['core_spikes'] = [[11, 10]]
        arms['balanced']['cue1']['core_spikes'] = [[11, 10]]
        checks = score_summary(score(
            arms, baseline, targets, controls=('balanced', 'shuffle_{mapping}')))
        self.assertEqual(checks['passed_cases'], 2)
        self.assertFalse(checks['checks']['reverse']['cue0']['passed'])
        self.assertFalse(checks['checks']['forward']['cue1']['passed'])
        arms['reverse']['blank']['core_spikes'] = [[10, 10]]
        arms['forward']['cue0']['rest_core_spikes'] = 1
        self.assertEqual(score_summary(score(arms, baseline, targets))['passed_cases'], 1)
        self.assertEqual(spike_comparisons(arms['reverse'], arms['forward'])['cue0'],
                         dict(matching=0, added=1, removed=1, rest_change=-1))

    def test_missing_reference_uses_baseline_opposite_and_cannot_complete(self):
        baseline = {
            cue: {'core_spikes': [], 'rest_core_spikes': 0} for cue in ('cue0', 'cue1', 'blank')
        }
        targets = [{10}, {11}]
        available = deepcopy(baseline)
        available['cue0']['core_spikes'] = [[11, 10]]
        available['cue1']['core_spikes'] = [[10, 10]]
        checks = score_summary(score({'reference_reverse': available}, baseline, targets,
                                     prefix='reference_'))
        self.assertFalse(checks['complete'])
        self.assertEqual(checks['passed_cases'], 2)
        self.assertEqual(checks['available_cases'], 2)
        self.assertFalse(checks['checks']['forward']['cue0']['available'])
        self.assertEqual(checks['checks']['reverse']['cue0']['opposite'], 0)

    def test_target_margin_comparison_tracks_unaligned_and_refractory_boundaries(self):
        first = {(10, 1): (0., .9, 0, -.2), (11, 2): (0., .9, 12, -.1)}
        second = {(10, 1): (0., .9, 0, -.4), (11, 2): (0., .9, 0, -.1)}
        left = {cue: (first, {}) for cue in ('cue0', 'cue1')}
        right = {cue: (second, {}) for cue in ('cue0', 'cue1')}
        aligned, refractory = target_margins(left, right, [{1}, {2}])['cue0']
        self.assertEqual(aligned['increased'], 1)
        self.assertAlmostEqual(aligned['mean_delta'], .2)
        self.assertEqual(refractory['control_only'], 1)
        self.assertIsNone(refractory['mean_delta'])


if __name__ == '__main__':
    unittest.main()
