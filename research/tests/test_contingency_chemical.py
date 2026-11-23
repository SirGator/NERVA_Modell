from copy import deepcopy
import json
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from contingency_chemical_analysis import (
    ARMS, PROTOCOL, SCOPES, analyze_directory, analyze_record, normalization,
)
from test_contingency_electrical import synthetic_record


def chemical_record():
    native = synthetic_record()
    constant = deepcopy(native)
    matched = deepcopy(native)
    record = {
        'protocol': PROTOCOL, 'replay_verified': True, 'seed': native['seed'],
        'gap_frames': native['gap_frames'],
        'archived_record_checksum': native['archived_record_checksum'],
        'probes': dict(zip(SCOPES, (native, constant, matched))),
        'blank_core_spikes': {scope: dict.fromkeys(ARMS, 0) for scope in SCOPES},
        'norm': {}, 'training': {},
    }
    for arm in ARMS[1:]:
        mapping = arm.removesuffix('_core')
        native['arms'][arm]['weights'] = [[0, .375], [1, .125]]
        constant['arms'][arm]['weights'] = [[0, .25], [1, .3125]]
        matched['arms'][arm]['weights'] = [[0, .25], [1, .4267767071723938]]
        record['norm'][mapping] = {
            'requested_norm': .1767766952966369, 'ray_limit': 1.75,
            'feasible': True, 'reason': None, 'ray_norm': .1767766952966369,
            'normalization_iterations': 0, 'actual_norm': .1767767071723938,
            'relative_norm_error': 6.717942597593579e-08, 'direction_cosine': 1.,
            'rounding_l2': 1.187575693339715e-08, 'faithful': True,
        }
        tape = {
            'verified_original_local_state': True,
            'verified_checkpoint_rule_and_local_clocks': True,
            'clock_times_us': [1], 'pre_arrivals': [[1, 0]], 'post_spikes': [[1, 12]],
            'chemical_samples': [[1, 10, .1, .1, .1, -.2, 0., 0., 1.]],
        }
        for name, probe in (('original', native), ('constant_positive', constant)):
            tape[name] = {'synapses': [
                {'synapse_id': edge, 'source_id': 12, 'target_id': 10 + edge,
                 'initial_weight': .25, 'weight': weight,
                 'positive_actual': max(0., weight - .25),
                 'negative_actual': min(0., weight - .25)}
                for edge, weight in probe['arms'][arm]['weights']
            ]}
        record['training'][mapping] = tape
    return record


def add_selected_spikes(record):
    for index in range(2):
        for reversal, name in enumerate(('forward_core', 'reverse_core')):
            row = record['probes']['matched']['arms'][name]['cues'][f'cue{index}']
            group = index ^ reversal
            offset = group * 3
            row['arrivals'][group * 2][3] = .75
            row['boundaries'][offset][3:8] = [.75, 1., 0., 0, True]
            row['core_spikes'] = [[10 + group, 10]]


class ChemistryContingencyTests(unittest.TestCase):
    def test_verified_vectors_and_by_gap_summary(self):
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / 'raw'
            path.mkdir()
            (path / 'seed7.json').write_text(json.dumps(chemical_record()))
            result = analyze_directory(temp)
        by_scope = result['by_gap_frames'][2]
        for scope in SCOPES:
            self.assertEqual(by_scope[scope], {
                'unavailable': 0, 'electrical': {'passes': 1, 'fails': 0, 'not_assessable': 0},
                'spikes': {'passes': 0, 'fails': 1, 'not_assessable': 0},
            })
        self.assertTrue(result['records'][0]['norm']['forward']['faithful'])

    def test_spike_gates_remain_available_when_electrical_gate_is_unassessable(self):
        record = chemical_record()
        add_selected_spikes(record)
        matched = analyze_record(record)['scopes']['matched']
        self.assertIsNone(matched['electrical']['passes'])
        self.assertTrue(matched['spikes']['passes'])
        contrast = matched['vs_native']['cue0']['forward_core']
        self.assertTrue(contrast['core_spike_history_changed'])
        self.assertEqual(contrast['target_spike_count_delta'], [1, 0])
        self.assertEqual(contrast['mean_peak_margin_delta_vs_native'], [None, None])
        record['blank_core_spikes']['matched']['forward_core'] = 1
        self.assertFalse(analyze_record(record)['scopes']['matched']['spikes']['passes'])
        record['blank_core_spikes']['matched']['forward_core'] = 0
        record['probes']['matched']['arms']['reverse_core']['cues']['cue1']['rest_core_spikes'] = 1
        self.assertFalse(analyze_record(record)['scopes']['matched']['spikes']['passes'])

    def test_raw_vectors_and_normalization_evidence_are_verified(self):
        record = chemical_record()
        record['probes']['constant']['arms']['reverse_core']['weights'][1][1] = .375
        with self.assertRaisesRegex(ValueError, 'weights differ from tape'):
            analyze_record(record)
        record = chemical_record()
        record['training']['reverse']['constant_positive']['synapses'][1]['positive_actual'] = .125
        with self.assertRaisesRegex(ValueError, 'weight changes do not reproduce'):
            analyze_record(record)
        record = chemical_record()
        record['norm']['reverse']['direction_cosine'] = .999
        with self.assertRaisesRegex(ValueError, 'direction_cosine'):
            analyze_record(record)
        record = chemical_record()
        record['probes']['matched']['arms']['forward_core']['weights'][0][1] = .375
        with self.assertRaisesRegex(ValueError, 'normalized ray'):
            analyze_record(record)
        record = chemical_record()
        record['probes']['matched'] = None
        record['blank_core_spikes']['matched'] = None
        with self.assertRaisesRegex(ValueError, 'probe availability'):
            analyze_record(record)

    def test_bound_and_precision_failure_are_distinguished(self):
        weights, plan = normalization([1., 1.], [.5, 0.], [1., -1.])
        self.assertIsNone(weights)
        self.assertEqual(plan['reason'], 'weight_bounds')
        weights, plan = normalization([1.], [0.], [.125])
        self.assertIsNone(weights)
        self.assertEqual(plan['reason'], 'undefined_direction_or_norm')
        weights, plan = normalization([1.], [.5], [1e-12])
        self.assertEqual(weights, [1.])
        self.assertTrue(plan['feasible'])
        self.assertFalse(plan['faithful'])
        self.assertEqual(plan['normalization_iterations'], 48)

    def test_infeasible_match_does_not_hide_raw_constant_probe(self):
        record = chemical_record()
        arm = record['probes']['constant']['arms']['forward_core']
        arm['weights'] = [[0, .25], [1, .25]]
        row = record['training']['forward']['constant_positive']['synapses'][1]
        row.update(weight=.25, positive_actual=0.)
        record['norm']['forward'] = {
            'requested_norm': .1767766952966369, 'ray_limit': None,
            'feasible': False, 'reason': 'undefined_direction_or_norm', 'faithful': False,
        }
        record['probes']['matched'] = None
        record['blank_core_spikes']['matched'] = None
        result = analyze_record(record)
        self.assertIsNone(result['scopes']['matched'])
        self.assertTrue(result['scopes']['constant']['electrical']['passes'])


if __name__ == '__main__':
    unittest.main()
