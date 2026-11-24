from copy import deepcopy
from pathlib import Path
import sys
from types import SimpleNamespace
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from contingency_b_source_analysis import (ARMS, CUE, PROBES, TARGET, expected_patch,
    gate_contrasts, read_branch, source_geometry, source_plan)


def fixture():
    start = 100_001
    core = {87, 93, 100, 104, 105, 106}
    coordinates = [(87, .4, True), (93, 0., True), (100, .5, True), (104, .7, True),
                   (105, .51, True), (106, .5, False)]
    geometry = source_geometry([[cell, [x, 0., 0.], exc] for cell, x, exc in coordinates], core, 1.)
    original = [15, 87, TARGET, .25, 1_000, True]
    spikes = [[cell, start + 1_000] for cell in (87, 100, 104, 106)]
    spikes.append([TARGET, start + 2_000])
    stage = dict(end_topology=[original], frames=[
        dict(exploratory=[[15, 1, 0, None, 2, .1]], formed=[[15, 87, TARGET]])],
        probes={cue: dict(start_us=start, core_spikes=deepcopy(spikes) if cue == CUE else [],
                          arrived_edges=[15] if cue == CUE else [], root_spikes=[],
                          rest_core_spikes=0) for cue in PROBES})
    row = dict(original_edge=original, original_exploratory=True, swap_source=100, sham_source=104,
               sham_cue_active=True, arms={})
    for name in ARMS:
        replacement = {'swap': 100, 'sham': 104}.get(name)
        arm = expected_patch(stage, original, replacement, name)
        arm['cancelled_arrivals'] = 0
        arm['probes'] = deepcopy(stage['probes'])
        for cue, probe in arm['probes'].items():
            edge_id = 16 if replacement is not None else 15
            probe.update(arrivals=[], boundaries=[])
            if cue != CUE:
                continue
            if name == 'block':
                probe['arrived_edges'] = []
                probe['core_spikes'].pop()
                continue
            probe['arrived_edges'] = [edge_id]
            probe['arrivals'] = [[start + 2_000, TARGET, edge_id, .25]]
            probe['boundaries'] = [[start + 2_000, TARGET, .75, .25, 1., 0., 0, True]]
        row['arms'][name] = arm
    return row, stage, SimpleNamespace(core=core), start, geometry


class BSourceTests(unittest.TestCase):
    def test_archived_source_and_active_sham_priority(self):
        _, stage, _, _, geometry = fixture()
        original, exploratory, swap, sham, active = source_plan(stage, geometry, 1., 'forward')
        self.assertEqual((original[0], exploratory, swap, sham, active), (15, True, 100, 104, True))
        self.assertEqual(source_plan(stage, geometry, .6, 'forward')[3:], (105, False))
        stage['end_topology'].append([16, 100, TARGET, .4, 1_000, False])
        with self.assertRaisesRegex(ValueError, 'reciprocal source is unavailable'):
            source_plan(stage, geometry, 1., 'forward')

    def test_control_branch_accepts_native_birth_and_requires_correct_receipt(self):
        row, stage, assay, start, geometry = fixture()
        with self.assertRaisesRegex(ValueError, 'birth class'):
            read_branch(row, stage, assay, start, geometry, 1., 'balanced', 'test')
        stage['frames'][0]['exploratory'] = []
        with self.assertRaisesRegex(ValueError, 'source plan differs'):
            read_branch(row, stage, assay, start, geometry, 1., 'balanced', 'test')
        row['original_exploratory'] = False
        result = read_branch(row, stage, assay, start, geometry, 1., 'balanced', 'test')
        self.assertFalse(result['original_exploratory'])

    def test_verify_exact_patch_and_same_clock_replayed_trace(self):
        row, stage, assay, start, geometry = fixture()
        result = read_branch(row, stage, assay, start, geometry, 1., 'forward', 'test')
        self.assertEqual(result['arms']['intact']['probes'][CUE]['target_spikes_us'], [2_000])
        self.assertEqual(result['arms']['block']['probes'][CUE]['target_spikes_us'], [])
        swapped = result['arms']['swap']['probes'][CUE]
        self.assertEqual(swapped['tested_edge_arrivals'], [[2_000, .25]])
        self.assertEqual(swapped['nonrefractory_peak_margin'], 0)
        for change, message in (
            (lambda r: r.__setitem__('sham_source', 105), 'source plan differs'),
            (lambda r: r['arms']['swap']['inserted_edges'][0].__setitem__(3, .5), 'inserted_edges differs'),
            (lambda r: r['arms']['swap']['probes'][CUE].__setitem__('arrived_edges', [15]), 'unavailable edges'),
            (lambda r: r['arms']['sham'].__setitem__('cancelled_arrivals', 1), 'cancellation differs'),
            (lambda r: r['arms']['intact']['probes'][CUE]['core_spikes'].pop(), 'intact probe differs'),
            (lambda r: r['arms']['swap']['probes'][CUE]['core_spikes'][1].__setitem__(1, start + 1001),
             'lacks delayed Core source'),
        ):
            altered = deepcopy(row)
            change(altered)
            with self.assertRaisesRegex(ValueError, message):
                read_branch(altered, stage, assay, start, geometry, 1., 'forward', 'test')

    def test_fixed_control_and_common_arm_gates_distinguish_rescued_recall(self):
        def probes(responsive=False):
            return {cue: dict(core_spikes=[[TARGET, 10]] if cue == CUE and responsive else [],
                              rest_core_spikes=0) for cue in PROBES}

        mappings = ('balanced', 'forward', 'reverse')
        archived = {'a': {'forward': {'probes': probes()}},
                    'b': {'forward': {m: {'probes': probes()} for m in mappings}}}
        trial = {m: {'arms': {arm: {'probes': probes(arm == 'swap' and m != 'reverse')}
                             for arm in ARMS}} for m in mappings}
        assay = SimpleNamespace(reset=probes(), targets={'b': [{TARGET}, {94}]})
        result = gate_contrasts(assay, trial, archived, 'forward')
        fixed = result['fixed_controls']['forward']['swap'][CUE]
        common = result['common_arm']['swap']['forward'][CUE]
        self.assertTrue(fixed['passed'])
        self.assertFalse(common['passed'])
        self.assertEqual(common['balanced'], 1)


if __name__ == '__main__':
    unittest.main()
