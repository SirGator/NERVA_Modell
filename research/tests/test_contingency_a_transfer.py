from copy import deepcopy
from pathlib import Path
import sys
from types import SimpleNamespace
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from contingency_a_transfer_analysis import (ARMS, GEOMETRY, PROBES, PROTOCOL, TOLERANCE,
    aggregate, analyze_record, plans, sham_rows)
from test_contingency_live_growth import fixture
from contingency_live_growth_analysis import recompute_checks


def transfer_fixture(source=None):
    source = fixture() if source is None else source
    details = source['details']
    positions = {cell: ([float(cell - 64) / 100, 0., 0.], True)
                 for cell in details['core_ids']}
    raw = dict(protocol=PROTOCOL, seed=source['job']['seed'], archived_record_checksum='test',
               full_replay_checked=True, formation_geometry=GEOMETRY,
               sham_max_distance_change=TOLERANCE,
               positions=[[cell, xyz, polarity] for cell, (xyz, polarity) in positions.items()],
               reset_probes={cue: deepcopy(details['reset'][cue]) for cue in PROBES}, mappings={})
    for mapping in ARMS:
        stage = details['stages_a'][mapping]
        native, transfer, swaps = plans(details['initial_topology'], stage['end_topology'],
                                       [set(), set()], positions)
        row = dict(swaps=swaps, native=native, transfer=transfer)
        for kind in ('native', 'transfer'):
            baseline = stage['probes'] if kind == 'native' else details['reset']
            for arm in row[kind].values():
                arm.update(probes={cue: deepcopy(baseline[cue]) for cue in PROBES},
                           cancelled_arrivals=0)
        raw['mappings'][mapping] = row
    return source, raw


class ATransferTest(unittest.TestCase):
    def test_sham_greedy_pair_preserves_incoming_slots(self):
        positions = {64: ([0., 0., 0.], True), 65: ([.01, 0., 0.], True),
                     66: ([.1, 0., 0.], False), 67: ([.11, 0., 0.], False)}
        born = [[4, 64, 66, .7, 1000, True], [5, 65, 67, .9, 1000, True]]
        swaps, rows = sham_rows([], born, positions)
        self.assertEqual(swaps, [[4, 5]])
        self.assertEqual(rows, [[4, 65, 66, .7, 1000, True], [5, 64, 67, .9, 1000, True]])
        self.assertEqual(born[0][1], 64)
        self.assertEqual(sham_rows([[0, 65, 66, .5, 1000, True]], born, positions)[0], [])
        changed = deepcopy(positions)
        changed[65] = changed[65][0], False
        self.assertEqual(sham_rows([], born, changed)[0], [])
        changed[65] = ([.25, 0., 0.], True)
        self.assertEqual(sham_rows([], born, changed)[0], [])

    def test_native_partitions_and_transfer_old_and_born_weights(self):
        initial = [[0, 64, 65, .5, 1000, True], [1, 65, 64, .4, 1000, False]]
        trained = deepcopy(initial)
        trained[0][3], trained[1][3] = .8, .6
        trained += [[2, 64, 66, .7, 1000, True], [3, 65, 67, .25, 1000, True]]
        positions = {cell: ([float(cell - 64) / 100, 0., 0.], True) for cell in range(64, 68)}
        native, transfer, swaps = plans(initial, trained, [{66}, set()], positions)
        self.assertEqual(native['restore_old']['weight_updates'], [[0, .8, .5]])
        self.assertEqual(native['reset_born']['weight_updates'], [[2, .7, .25]])
        self.assertEqual(native['block_terminal']['blocked_edges'], [2])
        self.assertEqual(native['block_other']['blocked_edges'], [3])
        self.assertEqual(transfer['old_only']['weight_updates'], [[0, .5, .8]])
        self.assertEqual(transfer['old_only']['inserted_edges'], [])
        self.assertEqual(transfer['native_initial']['inserted_edges'][0][3], .25)
        self.assertEqual(transfer['joint']['published_topology'][0][3], .8)
        self.assertEqual(transfer['joint']['inserted_edges'][0][3], .7)
        self.assertEqual(swaps, [[2, 3]])

    def test_archived_hit_counts_and_partial_transfer_are_recomputed(self):
        source = fixture()
        details = source['details']
        for name, cell in (('outcome0', 78), ('outcome1', 79)):
            probe = details['reset'][name]
            probe['core_spikes'] = [[cell, probe['start_us']]]
        details['targets']['a'] = [[78], [79]]
        probe = details['stages_a']['forward']['probes']['cue0']
        probe['core_spikes'] = [[78, probe['start_us']]]
        assay = SimpleNamespace(reset=details['reset'], targets={'a': [{78}, {79}], 'b': [set(), set()]})
        details['checks'], source['metrics'] = recompute_checks(assay, details['stages_a'], details['branches_b'])
        source, raw = transfer_fixture(source)
        probe = raw['mappings']['forward']['transfer']['joint']['probes']['cue0']
        probe['core_spikes'] = [[78, probe['start_us']]]
        probe = raw['mappings']['forward']['native']['block_born']['probes']['cue0']
        probe['core_spikes'] = []
        result = analyze_record(raw, source, 'test')
        summary = aggregate([result])
        self.assertEqual(summary['native']['intact']['passed_cases'], 1)
        self.assertEqual(summary['transfer']['joint']['partial_seed_passes'], 1)
        self.assertEqual(summary['archived_hit_cases'][0]['decrements']['block_born'], 1)
        effect = result['geometry_contrasts']['forward']['cue0']['joint']
        self.assertEqual(effect['sham_corrected'], 1)
        self.assertFalse(effect['eligible'])  # no changed geometry in this donor
        self.assertFalse(effect['geometry_specific'])

    def test_complete_record_and_reject_changed_receipts_or_probes(self):
        source, raw = transfer_fixture()
        result = analyze_record(raw, source, 'test')
        self.assertEqual(aggregate([result])['transfer']['joint']['passed_cases'], 0)
        mutations = [
            lambda r: r.__setitem__('full_replay_checked', False),
            lambda r: r['positions'][0][1].__setitem__(0, float('nan')),
            lambda r: r['positions'][0].__setitem__(0, 999),
            lambda r: r.__setitem__('archived_record_checksum', 'other'),
            lambda r: r['mappings']['forward']['native']['intact']['published_topology'][0].__setitem__(4, 2000),
            lambda r: r['mappings']['forward']['native']['intact']['probes']['cue0'].__setitem__('arrived_edges', [99]),
            lambda r: r['mappings']['forward']['native']['intact']['probes']['cue0'].__setitem__('rest_core_spikes', 1),
            lambda r: r['mappings']['forward']['transfer']['joint']['probes']['cue0']['root_spikes'][0].__setitem__(1, 0),
            lambda r: r['mappings']['forward']['transfer']['joint'].__setitem__('cancelled_arrivals', 1),
            lambda r: r['mappings']['forward'].__setitem__('swaps', [[1, 2]]),
        ]
        for mutate in mutations:
            with self.subTest(mutation=mutate):
                altered = deepcopy(raw)
                mutate(altered)
                with self.assertRaises(ValueError):
                    analyze_record(altered, source, 'test')


if __name__ == '__main__':
    unittest.main()
