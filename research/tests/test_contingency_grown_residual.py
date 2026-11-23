from copy import deepcopy
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from contingency_grown_residual_analysis import (
    common_ray_dose, dot, projected_directions, score_residual, tier_layout,
)


class GrownResidualTests(unittest.TestCase):
    def test_projection_removes_the_prespecified_basis_and_permutes_the_residual(self):
        learned = {'balanced': [1., 2., 2.], 'forward': [3., 2., -1.], 'reverse': [2., -1., 1.]}
        order = [1, 0, 2]
        coefficients, directions = projected_directions(learned, order)
        self.assertAlmostEqual(coefficients['forward']['balanced_coefficient'], 5. / 9.)
        self.assertAlmostEqual(coefficients['forward']['sham_coefficient'], 6. / 9.)
        for mapping in ('forward', 'reverse'):
            residual = directions[f'residual_{mapping}']
            self.assertAlmostEqual(dot(residual, learned['balanced']), 0.)
            self.assertAlmostEqual(dot(directions[f'sham_{mapping}'], [2., 1., 2.]), 0.)
            self.assertEqual(directions[f'shuffle_residual_{mapping}'],
                             [residual[index] for index in order])

    def test_ray_dose_reuses_archive_only_when_every_new_arm_can_reach_it(self):
        base = [1.75, .5]
        vectors = {'residual_forward': [1., 0.], 'sham_forward': [0., -1.]}
        adjusted, limits = common_ray_dose(.3, base, vectors)
        self.assertEqual(limits, {'residual_forward': .25, 'sham_forward': .5})
        self.assertEqual(adjusted, .225)
        self.assertEqual(common_ray_dose(.2, base, vectors)[0], .2)

    def test_strict_improvement_requires_original_advantage_per_cue(self):
        baseline = {
            cue: {'core_spikes': [], 'rest_core_spikes': 0} for cue in ('cue0', 'cue1', 'blank')
        }
        arms = {name: deepcopy(baseline) for name in (
            'balanced', 'forward', 'reverse', 'residual_forward', 'residual_reverse',
            'sham_forward', 'sham_reverse', 'shuffle_residual_forward', 'shuffle_residual_reverse'
        )}
        for mapping, cells in (('forward', (10, 11)), ('reverse', (11, 10))):
            for index, cell in enumerate(cells):
                arms[f'residual_{mapping}'][f'cue{index}']['core_spikes'] = [[cell, 10]]
        arms['forward']['cue0']['core_spikes'] = [[10, 10]]
        specificity, improvement = score_residual(arms, baseline, [{10}, {11}])
        self.assertTrue(specificity['complete'])
        self.assertEqual(improvement['passed_cases'], 3)
        self.assertFalse(improvement['checks']['forward']['cue0']['passed'])

    def test_control_aliases_cannot_substitute_probes_or_reference_unavailable_arms(self):
        probes = {'cue0': {'core_spikes': [[10, 10]]}}
        archived = {'balanced': {'probes': probes}, 'shuffle_forward': {'probes': {}}}
        directions = {'residual_forward': [1.]}
        tier = {'requested_norm': .2, 'reused_controls': ['balanced'], 'arms': {'residual_forward': {}}}
        self.assertEqual(tier_layout(tier, directions, archived, True), {'balanced': probes})
        tier['arms']['balanced'] = {'probes': {}}
        with self.assertRaisesRegex(ValueError, 'unexpected residual tier arms'):
            tier_layout(tier, directions, archived, True)
        with self.assertRaisesRegex(ValueError, 'unexpected archived control aliases'):
            tier_layout(tier, directions, archived, False)
        tier['reused_controls'] = []
        self.assertEqual(tier_layout(tier, directions, archived, False), {})


if __name__ == '__main__':
    unittest.main()
