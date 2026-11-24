from copy import deepcopy
from pathlib import Path
from types import SimpleNamespace
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from contingency_a_timing_analysis import (PROTOCOL, analyze_record, issued_tape,
                                          tape_invariants, validate_probe)
from test_contingency_a_source import source_fixture


class ATimingTests(unittest.TestCase):
    def test_signed_tape_stable_ties_and_window_invariants(self):
        baseline = dict(start_us=100, arrivals=[[110, 78, 1, .5], [120, 78, 2, -.25],
                                               [120, 78, 3, 0.]])
        edited = issued_tape(baseline, [[0, 20]])
        self.assertEqual(edited, [[20, 1, .5], [20, 2, -.25], [20, 3, 0.]])
        self.assertEqual(tape_invariants(edited), tape_invariants(issued_tape(baseline, [])))
        for edits in ([[0, 0], [0, 10]], [[True, 5]], [[0, -1]], [[0, 200000]]):
            with self.assertRaisesRegex(ValueError, 'timing row edit'):
                issued_tape(baseline, edits)

    def probe_fixture(self):
        baseline = dict(start_us=100, root_spikes=[], core_spikes=[], rest_core_spikes=0,
            arrived_edges=[1, 2], arrivals=[[110, 78, 1, .5], [110, 78, 2, -.25], [120, 78, 1, 0.]],
            boundaries=[[110, 78, 0., .25, 1., .25, 0, False],
                        [120, 78, .25, 0., 1., .25, 0, False]])
        receipt = dict(disabled_edges=[1, 2], cancelled_arrivals=0,
            tape=issued_tape(baseline, []), response={**deepcopy(baseline), 'arrived_edges': [],
                'arrivals': [], 'external_inputs': [[110, 78, .5], [110, 78, -.25], [120, 78, 0.]]})
        rows = [[1, 64, 78, .5, 1000, True], [2, 65, 78, .25, 1000, True]]
        return baseline, receipt, rows

    def test_external_log_native_disable_and_every_boundary(self):
        baseline, receipt, rows = self.probe_fixture()
        validate_probe(receipt, baseline, [], rows, 78, SimpleNamespace(core={64, 65, 78}), 'cue1', True)
        changes = [
            (lambda r: r['disabled_edges'].pop(), 'disabled edges'),
            (lambda r: r['tape'][1].__setitem__(2, .25), 'issued timing tape'),
            (lambda r: r['response']['external_inputs'][1].__setitem__(2, .25), 'logged external'),
            (lambda r: r['response']['arrived_edges'].append(1), 'unavailable edges'),
            (lambda r: r['response']['arrivals'].append([110, 78, 1, .5]), 'native arrivals'),
            (lambda r: r['response']['boundaries'][0].__setitem__(3, .5), 'summed input'),
            (lambda r: r['response']['boundaries'][1].__setitem__(5, .5), 'recorded potential'),
            (lambda r: r['response']['boundaries'].pop(), 'missing integration boundary'),
            (lambda r: r['response'].__setitem__('rest_core_spikes', 1), 'clean no-op rest'),
        ]
        for mutate, message in changes:
            changed = deepcopy(receipt)
            mutate(changed)
            with self.subTest(message=message), self.assertRaisesRegex(ValueError, message):
                validate_probe(changed, baseline, [], rows, 78, SimpleNamespace(core={64, 65, 78}), 'cue1', True)

    def test_all_carrier_reference_and_control_probes(self):
        source, source_plan, source_raw = source_fixture()
        carriers = {name: dict(source_family=family, source_condition=condition,
                              probes=source_raw['native'][condition]['probes'] if family == 'native'
                              else source_raw['grafts'][family][condition]['probes'],
                              arms={'reference': [], 'edited': []})
                    for name, family, condition in [('native', 'native', 'intact'),
                        ('grown', 'native_initial', 'intact'), ('rotated', 'native_initial', 'cycle_full')]}
        plan = dict(seed=source['job']['seed'], archived_record_checksum='test', target_cell=78,
                    source_plan=source_plan, carriers=carriers)
        raw = dict(protocol=PROTOCOL, seed=source['job']['seed'], archived_record_checksum='test',
                   target_cell=78, full_replay_checked=True, carriers={})
        for name, carrier in carriers.items():
            original = source_raw['native'] if carrier['source_family'] == 'native' else source_raw['grafts'][carrier['source_family']]
            rows = original[carrier['source_condition']]['published_topology']
            raw['carriers'][name] = dict(pre_clamp_topology=rows, arms={arm: dict(probes={cue:
                dict(disabled_edges=sorted(row[0] for row in rows if row[2] == 78 and row[5]),
                     cancelled_arrivals=0, tape=[], response={**deepcopy(probe), 'external_inputs': []})
                for cue, probe in carrier['probes'].items()}) for arm in carrier['arms']})
        report = analyze_record(raw, source, 'test', plan)
        self.assertEqual(set(report['carriers']), set(carriers))
        for arms in report['carriers'].values():
            for arm in arms.values():
                self.assertTrue(all(row['clean_noop_checked'] for row in arm['probes'].values()))
        raw['carriers']['rotated']['arms']['reference']['probes']['blank']['response']['rest_core_spikes'] += 1
        with self.assertRaisesRegex(ValueError, 'clean no-op rest'):
            analyze_record(raw, source, 'test', plan)


if __name__ == '__main__':
    unittest.main()
