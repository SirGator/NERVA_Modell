from copy import deepcopy
from pathlib import Path
import sys
from types import SimpleNamespace
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from contingency_contrast_growth_analysis import (CONTRAST_RULE, CONTROLS, INTERVENTION,
    POLICIES, PROTOCOL, aggregate, analyze_record, contrast_choices, layout)
from contingency_lead_growth_analysis import layout as lead_layout, policy_checks
from test_contingency_lead_growth import fixture as lead_fixture


def fixture(seed=34501):
    record = lead_fixture(seed)
    job, details = record['job'], record['details']
    name, stimuli = layout(seed)
    job['intervention'] = INTERVENTION
    job['effective_config'].update(protocol=PROTOCOL, layout=name, stimuli=stimuli,
                                  intervention=INTERVENTION, formation_policies=list(POLICIES),
                                  contrast_rule=CONTRAST_RULE)
    details.update(protocol=PROTOCOL, layout=name, stimuli=stimuli)
    trials = details['trials']
    trials['contrast'] = deepcopy(trials['lead'])
    for name, probe in details['reset'].items():
        probe['root_spikes'] = [[pixel, probe['start_us']] for pixel in stimuli[name]]
    for policy, arms in trials.items():
        stages = list(arms['a'].values())
        stages += [stage for branches in arms['b'].values() for stage in branches.values()]
        for stage in stages:
            stage['formation_policy'] = policy
            for frame in stage['frames']:
                frame['root_spikes'] = [[pixel, frame['start_us']] for pixel in stimuli[frame['stimulus']]]
                if policy == 'contrast':
                    frame['contrast_components'] = []
            for name, probe in stage['probes'].items():
                probe['root_spikes'] = [[pixel, probe['start_us']] for pixel in stimuli[name]]
    assay = SimpleNamespace(reset=details['reset'], targets={'a': [set(), set()], 'b': [set(), set()]})
    details['checks'], record['metrics'] = policy_checks(assay, trials, POLICIES)
    return record


class ContrastGrowthTest(unittest.TestCase):
    def test_new_layouts_and_full_crossed_comparison(self):
        for seed, name in ((34501, 'shear'), (34531, 'stagger')):
            actual, stimuli = layout(seed)
            self.assertEqual(actual, name)
            self.assertEqual(sorted(pixel for image in stimuli.values() for pixel in image), list(range(64)))
            self.assertNotEqual(stimuli, lead_layout(seed)[1])
            result = analyze_record(fixture(seed))
            self.assertEqual(result['layout'], name)
            summary = aggregate([result])
            self.assertEqual(set(summary['paired_contrast']), set(CONTROLS))
            for paired in summary['paired_contrast'].values():
                self.assertEqual(paired['joint_paths']['neither'], 4)
                self.assertEqual(paired['formed_graphs'][0]['compared_stages'], 12)
                self.assertEqual(paired['trajectory']['b']['changed_training_core'], 0)

    def test_exact_signed_contrast_can_choose_above_recency_at_negative_score(self):
        frame = dict(formed=[[10, 64, 66]], exploratory=[[10, 300000, 250000, None, 2, .1]],
                     lead_choices=[[10, 65, -1.0, -2.0]],
                     contrast_components=[[10, 0.5, 1.5, 2.0, 4.0]])
        counts = contrast_choices([frame], 'contrast', {}, {64, 65, 66}, 10001)
        self.assertEqual(counts['above_recency'], 1)
        self.assertEqual(counts['negative_evidence'], 1)
        self.assertEqual(counts['positive_evidence'], 0)
        self.assertEqual(counts['above_recency_with_negative_score'], 1)
        mutations = (
            ('contrast_components', [[9, .5, 1.5, 2., 4.]], 'birth order'),
            ('contrast_components', [[10, -.5, .5, 2., 4.]], 'negative lead component'),
            ('contrast_components', [[10, .5, float('nan'), 2., 4.]], 'numeric ledger'),
            ('contrast_components', [[10, .5, 1.5000000000000002, 2., 4.]], 'differs from observed'),
            ('lead_choices', [[10, 65, -2., -2.]], 'above recency'),
            ('lead_choices', [[10, 66, -1., -2.]], 'invalid recency candidate'),
        )
        for key, rows, message in mutations:
            with self.subTest(rows=rows), self.assertRaisesRegex(ValueError, message):
                contrast_choices([{**frame, key: rows}], 'contrast', {}, {64, 65, 66}, 10001)

    def test_cancellation_does_not_hide_warmup_or_same_source_disagreement(self):
        frame = dict(formed=[[10, 64, 66]], exploratory=[[10, 300000, 250000, None, 1, .1]],
                     lead_choices=[[10, 64, 0., 0.]], contrast_components=[[10, 2., 2., 2., 2.]])
        self.assertEqual(contrast_choices([frame], 'contrast', {}, {64, 66}, 10001)['observed_evidence'], 1)
        with self.assertRaisesRegex(ValueError, 'matched delay'):
            contrast_choices([frame], 'contrast', {}, {64, 66}, 200000)
        frame['contrast_components'][0][-2:] = [3., 3.]
        with self.assertRaisesRegex(ValueError, 'different contrast components'):
            contrast_choices([frame], 'contrast', {}, {64, 66}, 10001)
        with self.assertRaisesRegex(ValueError, 'control frame'):
            contrast_choices([frame], 'lead', {}, {64, 66}, 10001)

    def test_training_and_probe_change_compared_on_matched_paths(self):
        record = fixture()
        stage = record['details']['trials']['contrast']['a']['forward']
        frame = stage['frames'][0]
        frame['core_spikes'] = [[64, frame['start_us']]]
        probe = stage['probes']['cue0']
        probe['core_spikes'] = [[64, probe['start_us']]]
        paired = analyze_record(record)['paired_trajectories']['recent']
        self.assertEqual(paired['a']['changed_training_core'], 1)
        self.assertEqual(paired['a']['changed_probe_core'], 1)
        self.assertEqual(paired['b']['changed_probe_core'], 0)
        record['job']['effective_config']['contrast_rule'] = 'lagged_minus_observed'
        with self.assertRaisesRegex(ValueError, 'effective configuration'):
            analyze_record(record)


if __name__ == '__main__':
    unittest.main()
