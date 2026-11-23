from copy import deepcopy
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from contingency_live_rate_analysis import Assay, CUES, FRAME_US, PROBES
from contingency_live_rate_analysis import gate_rows, layout, target_groups


def probe(*cells, rest=0):
    return {'core_spikes': [[cell, index] for index, cell in enumerate(cells)],
            'rest_core_spikes': rest}


class ContingencyLiveRateTest(unittest.TestCase):
    def test_reset_targets_exclude_all_cues_and_same_task_outcome(self):
        reset = {name: probe() for name in layout()}
        reset.update(blank=probe(10), cue0=probe(11), cue_b1=probe(12),
                     outcome0=probe(1, 7, 11, 12), outcome1=probe(2, 7, 10),
                     outcome_b0=probe(1, 3, 8, 11), outcome_b1=probe(4, 8, 12))
        self.assertEqual(target_groups(reset), {'a': [{1}, {2}], 'b': [{1, 3}, {4}]})

    def test_gates_require_prior_acquisition_and_strict_b_increment(self):
        reset = {name: probe() for name in PROBES}
        arms = {arm: {task: {'probes': deepcopy(reset)} for task in CUES}
                for arm in ('forward', 'reverse', 'balanced')}
        forward = arms['forward']
        forward['a']['probes'].update(cue0=probe(1), cue_b0=probe(3))
        forward['b']['probes'].update(cue0=probe(1), cue1=probe(2),
                                     cue_b0=probe(3), cue_b1=probe(4, rest=1))
        acquired = gate_rows(reset, [{1}, {2}], arms, 'a', 'a')
        self.assertTrue(acquired['forward']['cue0']['passed'])
        self.assertFalse(acquired['forward']['cue1']['passed'])
        retained = gate_rows(reset, [{1}, {2}], arms, 'a', 'b', acquired)['forward']
        self.assertTrue(retained['cue0']['passed'])
        self.assertEqual(retained['cue1']['correct'], 1)
        self.assertFalse(retained['cue1']['eligible'])
        self.assertFalse(retained['cue1']['passed'])
        b = gate_rows(reset, [{3}, {4}], arms, 'b', 'b')['forward']
        self.assertEqual(b['cue_b0']['correct'], b['cue_b0']['before_b'])
        self.assertFalse(b['cue_b0']['passed'])
        self.assertFalse(b['cue_b1']['quiet'])
        self.assertFalse(b['cue_b1']['passed'])
        forward['b']['probes']['cue_b0'] = probe(3, 3)
        self.assertTrue(gate_rows(reset, [{3}, {4}], arms, 'b', 'b')['forward']['cue_b0']['passed'])
        self.assertFalse(gate_rows(reset, [{3}, set()], arms, 'b', 'b')['forward']['cue_b0']['eligible'])

    def test_training_verifies_contiguous_b_clock_and_b_stimuli(self):
        seed = 4
        checkpoint = 10_001
        rest_us = 100_000
        repetitions = 4
        start = checkpoint + repetitions * 10 * FRAME_US
        patterns = layout()
        reset = {}
        for name, pixels in patterns.items():
            at = checkpoint + rest_us
            reset[name] = dict(probe(), start_us=at, arrived_edges=[],
                               root_spikes=[[pixel, at] for pixel in pixels])
        details = {'core_ids': list(range(64, 80)), 'checkpoint_time_us': checkpoint,
                   'stimuli': patterns, 'initial_topology': [[0, 64, 65, .5, 1000, True]],
                   'reset': reset, 'targets': {'a': [[], []], 'b': [[], []]}}
        config = {'rest_us': rest_us, 'training_repetitions': repetitions, 'processing_neurons': 16}
        assay = Assay(details, config, seed)
        trial0 = ('cue_b0', 'outcome_b1', 'blank', 'blank', 'blank')
        trial1 = ('cue_b1', 'outcome_b0', 'blank', 'blank', 'blank')
        names = (trial0 + trial1 + trial1 + trial0) * 2
        frames = []
        for index, name in enumerate(names):
            at = start + index * FRAME_US
            frames.append({'stimulus': name, 'start_us': at, 'end_us': at + FRAME_US - 1,
                           'root_spikes': [[pixel, at] for pixel in patterns[name]],
                           'core_spikes': [], 'weight_changes': 0, 'homeostasis_changes': 0})
        stage = {'frames': frames}
        self.assertEqual(assay.training(stage, 'reverse', 'native', 1)[2].total(), 128)
        stage['frames'][0]['start_us'] += rest_us
        with self.assertRaisesRegex(ValueError, 'training schedule or window'):
            assay.training(stage, 'reverse', 'native', 1)
        stage['frames'][0]['start_us'] = start
        stage['frames'][0]['root_spikes'][0][0] = patterns['cue0'][0]
        with self.assertRaisesRegex(ValueError, 'unexpected spike identity'):
            assay.training(stage, 'reverse', 'native', 1)
