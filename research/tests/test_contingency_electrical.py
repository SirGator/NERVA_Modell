import json
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from contingency_electrical_analysis import (
    ARMS, PROTOCOL, analyze_directory, analyze_record, canonical_sum, f32, read_cue,
)


def synthetic_record():
    arms = {}
    for arm in ARMS:
        cues = {}
        for cue in range(2):
            deltas = {
                'reset': [0., 0.],
                'forward_core': [.125, -.125][::1 if cue == 0 else -1],
                'reverse_core': [-.125, .125][::1 if cue == 0 else -1],
                'balanced_core': [.0625, 0.],
            }[arm]
            arrivals, boundaries = [], []
            for index, delta in enumerate(deltas):
                target = 10 + index
                # Reset candidates tie at -0.5. A later arm peak must not change the selected row.
                for time, extra in ((10, delta), (11, .25 if arm != 'reset' else 0.)):
                    summed = .25 + extra
                    arrivals.append([time, target, index, summed])
                    boundaries.append([time, target, .25, summed, 1., .25 + summed, 0, False])
                # A larger candidate in a refractory boundary is ineligible.
                boundaries.append([12, target, .875, 0., 1., .875, 13, False])
            cues[f'cue{cue}'] = {
                'start_us': 0, 'root_spikes': [[cue, 1]], 'core_spikes': [],
                'rest_core_spikes': 0, 'arrived_edges': [0, 1],
                'arrivals': arrivals, 'boundaries': boundaries,
            }
        arms[arm] = {'weights': [[0, .25], [1, .25]], 'cues': cues}
    return {
        'protocol': PROTOCOL, 'replay_verified': True, 'seed': 7, 'gap_frames': 2,
        'archived_record_checksum': 'test',
        'templates': [[10], [11, 12]], 'core_ids': [10, 11, 12],
        'edges': [[0, 12, 10, .25], [1, 12, 11, .25]],
        'arms': arms,
    }


class ElectricalContingencyTests(unittest.TestCase):
    def test_reset_peaks_reversal_controls_and_coverage(self):
        report = analyze_record(synthetic_record())
        self.assertTrue(report['passes'])
        for cue in report['cues'].values():
            self.assertEqual(cue['reset_peaks'], [[10, 10, -.5], [11, 10, -.5]])
            self.assertEqual(cue['mapping_passes'], {'forward_core': True, 'reverse_core': True})
        probe = report['cues']['cue0']['arms']['forward_core']
        self.assertEqual([row['mean_peak_margin_delta'] for row in probe['target_groups']],
                         [.125, -.125])
        self.assertEqual(probe['target_groups'][1]['template_cells'], 2)
        self.assertEqual(probe['target_groups'][1]['paired_peak_cells'], 1)
        self.assertEqual(probe['target_groups'][1]['negative_peak_margin_deltas'], 1)
        self.assertEqual(probe['max_abs_arrival_delta'], .25)
        self.assertEqual(probe['max_abs_summed_input_delta'], .25)
        self.assertEqual(probe['max_abs_margin_delta'], .25)
        self.assertEqual(probe['largest_positive_margin_delta_fraction_of_reset_deficit'], .5)

    def test_balanced_tie_fails_and_boundary_mismatch_is_unassessable(self):
        record = synthetic_record()
        cue = record['arms']['balanced_core']['cues']['cue0']
        cue['arrivals'][0][3] = .375
        cue['boundaries'][0][3] = .375
        cue['boundaries'][0][5] = .625
        report = analyze_record(record)
        self.assertIs(report['passes'], False)
        self.assertEqual(report['cues']['cue0']['mapping_passes'],
                         {'forward_core': False, 'reverse_core': True})
        cue['boundaries'].pop()
        report = analyze_record(record)
        self.assertIsNone(report['passes'])
        self.assertEqual(report['cues']['cue0']['arms']['balanced_core']['mismatches'],
                         ['boundary_keys'])

    def test_f32_rounding_decides_threshold_and_sums_signed_arrivals(self):
        half_ulp = 2. ** -24
        self.assertEqual(f32(1. + half_ulp), 1.)
        self.assertEqual(canonical_sum([2. ** 60, -2. ** 60, half_ulp]), half_ulp)
        cue = {'start_us': 0, 'root_spikes': [], 'core_spikes': [[10, 1]],
               'rest_core_spikes': 0, 'arrived_edges': [0], 'arrivals': [[1, 10, 0, half_ulp]],
               'boundaries': [[1, 10, 1. - half_ulp, half_ulp, 1., 0., 0, True]]}
        borders, _ = read_cue(cue, {10}, {10, 12}, {0: (12, 10)})
        self.assertEqual(borders[1, 10][3], 0.)
        cue['boundaries'][0][3] = 0.
        with self.assertRaisesRegex(ValueError, 'summed input differs'):
            read_cue(cue, {10}, {10, 12}, {0: (12, 10)})

    def test_output_directory_counts_gap_and_rejects_duplicate(self):
        with tempfile.TemporaryDirectory() as temp:
            raw = Path(temp) / 'raw'
            raw.mkdir()
            contents = json.dumps(synthetic_record())
            (raw / 'a.json').write_text(contents)
            self.assertEqual(analyze_directory(temp)['by_gap_frames'][2],
                             {'passes': 1, 'fails': 0, 'not_assessable': 0})
            (raw / 'b.json').write_text(contents)
            with self.assertRaisesRegex(ValueError, 'duplicate seed'):
                analyze_directory(temp)


if __name__ == '__main__':
    unittest.main()
