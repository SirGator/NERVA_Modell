import hashlib
import json
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from verify_contingency import verify_directory, verify_record


def completed_record():
    stimuli = {'blank': [], 'cue0': [0, 1], 'cue1': [2, 3],
               'cue0a': [0], 'cue0b': [1], 'cue1a': [2], 'cue1b': [3],
               'outcome0': [4], 'outcome1': [5]}
    job = {'id': 'v00000-s1-i000', 'seed': 1,
           'effective_config': {'training_repetitions': 4, 'gap_frames': 2,
                                'rest_us': 100_000, 'stimuli': stimuli}}
    initial = [[0, 1.], [1, 1.]]
    training, probes, counts = {}, {}, {}
    for arm in ('forward', 'reverse', 'balanced'):
        frames = []
        for repeat, cues in enumerate([(1, 0), (0, 1), (1, 0), (0, 1)]):
            for cue in cues:
                outcome = 1 - cue if arm == 'reverse' or (arm == 'balanced' and repeat >= 2) else cue
                for name in (f'cue{cue}', 'blank', 'blank', f'outcome{outcome}', 'blank'):
                    start = len(frames) * 50_000
                    frames.append({'stimulus': name, 'start_us': start,
                                   'root_spikes': [[i, start + 10] for i in stimuli[name]],
                                   'core_spikes': [[12, start + 11]], 'weight_changes': 1})
        training[arm] = {'weights': [[0, 0.5], [1, 0.5]], 'frames': frames}
    for arm in ('reset', 'forward_core', 'reverse_core', 'balanced_core',
                'forward_all', 'reverse_all', 'balanced_all'):
        weights = initial if arm == 'reset' else [[0, .5], [1, .5 if arm.endswith('_all') else 1.]]
        responses = {}
        counts[arm] = {}
        for name, pixels in stimuli.items():
            ids = [12]
            if name.startswith('outcome'):
                ids.append(10 + int(name[-1]))
            if name.startswith('cue') and arm.startswith(('forward', 'reverse')):
                outcome = int(name[3]) if arm.startswith('forward') else 1 - int(name[3])
                ids += [10 + outcome] * 2
            responses[name] = {'start_us': 100_000, 'rest_core_spikes': 0,
                               'root_spikes': [[i, 100_010] for i in pixels],
                               'core_spikes': [[i, 100_020 + j] for j, i in enumerate(ids)],
                               'arrived_edges': [0, 1]}
            counts[arm][name] = [ids.count(10), ids.count(11)]
        probes[arm] = {'weights': weights, 'responses': responses}
    gates = {suffix: {'original': True, 'fragments': True} for suffix in ('core', 'all')}
    return {'job': job, 'full_replay_checked': True,
            'metrics': {'original_core': 1, 'fragments_core': 1, 'original_all': 1,
                        'fragments_all': 1, 'templates_available': 1, 'core_spike_effects': 12},
            'details': {'protocol': 'NERVA-Visual-Contingency/1', 'gap_frames': 2,
                        'core_ids': [10, 11, 12], 'stimuli': stimuli,
                        'development': {'checkpoint_time_us': 0},
                        'edges': [[0, 10, 11, 1.], [1, 0, 10, 1.]], 'training': training,
                        'probes': probes, 'templates': [[10], [11]], 'counts': counts, 'gates': gates}}


class ContingencyTests(unittest.TestCase):
    def test_native_directory_recounts_raw_training_and_probe_rows(self):
        record = completed_record()
        with tempfile.TemporaryDirectory() as temp:
            directory = Path(temp)
            (directory / 'manifest.json').write_text(json.dumps({
                'protocol': 'NERVA-Research-v1', 'jobs': [record['job']], 'counts': {'trials': 1}}))
            (directory / 'records').mkdir()
            path = directory / 'records' / f"{record['job']['id']}.json"
            raw = json.dumps({'record': record}).encode()
            path.write_bytes(raw)
            row, = verify_directory(directory)['verified_records']
            self.assertEqual(row['record_sha256'], hashlib.sha256(raw).hexdigest())
            self.assertEqual(row['training_core_spikes'], dict.fromkeys(('forward', 'reverse', 'balanced'), 40))
            self.assertEqual(row['training_root_spikes_by_channel'], dict.fromkeys(range(6), 4))
            self.assertEqual(row['counts']['reverse_core']['cue0'], [0, 2])
            self.assertEqual((row['core_spike_effects'], row['all_spike_effects']), (12, 12))
            self.assertEqual(row['training_core_weight_changes']['reverse'], {
                'changed_edges': 1, 'l1': .5, 'linf': .5,
                'changed_edges_with_original_cue_arrivals': 1})
            path.unlink()
            with self.assertRaisesRegex(ValueError, 'result files differ'):
                verify_directory(directory)

    def test_rejects_raw_exposure_timestamp_weight_and_count_corruption(self):
        cases = (
            ('training root spike counts differ', lambda d: d['training']['reverse']['frames'][0]['root_spikes'].pop()),
            ('spike timestamp outside', lambda d: d['probes']['reset']['responses']['cue0']['core_spikes'].append([12, 300_000])),
            ('installed probe weights differ', lambda d: d['probes']['forward_core']['weights'][1].__setitem__(1, .5)),
            ('reported template spike counts differ', lambda d: d['counts']['reverse_core']['cue0'].__setitem__(1, 3)),
            ('probe root spike times differ', lambda d: d['probes']['balanced_all']['responses']['cue0']['root_spikes'][0].__setitem__(1, 100_011)),
        )
        for error, mutate in cases:
            with self.subTest(error=error):
                record = completed_record()
                mutate(record['details'])
                with self.assertRaisesRegex(ValueError, error):
                    verify_record(record)

    def test_balanced_tie_fails_only_matching_weight_scope_and_cue_type(self):
        record = completed_record()
        details = record['details']
        details['probes']['balanced_core']['responses']['cue0a']['core_spikes'] += [[10, 100_027], [10, 100_028]]
        details['counts']['balanced_core']['cue0a'] = [2, 0]
        with self.assertRaisesRegex(ValueError, 'reported contingency gates differ'):
            verify_record(record)
        details['gates']['core']['fragments'] = False
        record['metrics']['fragments_core'] = 0
        record['metrics']['core_spike_effects'] = 13
        self.assertEqual(verify_record(record)['gates']['core'], {'original': True, 'fragments': False})
        self.assertTrue(verify_record(record)['gates']['all']['fragments'])


if __name__ == '__main__':
    unittest.main()
