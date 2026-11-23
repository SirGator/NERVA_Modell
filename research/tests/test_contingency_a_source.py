from copy import deepcopy
from pathlib import Path
import json
import tempfile
from unittest.mock import patch
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from contingency_a_source_analysis import PROTOCOL, analyze_directory, analyze_record, contrast, intervention_plans, validate_trace
from test_contingency_live_growth import fixture


def source_fixture():
    source = fixture()
    stage = source['details']['stages_a']['reverse']
    stage['end_topology'] += [[1, 64, 78, .75, 1000, True], [2, 65, 79, .25, 1000, True]]
    probe = stage['probes']['cue1']
    probe['core_spikes'] = [[78, probe['start_us'] + 1000]]
    reference = {cue: deepcopy(source['details']['reset'][cue]) for cue in ('blank', 'cue0', 'cue1')}
    plan = dict(mapping='reverse', cue='cue1', target_cell=78, active_edges=[1], silent_edges=[2],
        grafts={name: dict(inserted_edges=[[1, 64, 78, .25, 1000, True], [2, 65, 79, .25, 1000, True]],
            reference_probes=deepcopy(reference), cycle=[1, 2], single_replacements=[[1, 66]],
            group_replacements=[[1, 66], [2, 67]]) for name in ('native_initial', 'sham_initial')})
    native, grafts = intervention_plans(source, plan)
    raw = dict(protocol=PROTOCOL, seed=source['job']['seed'], archived_record_checksum='test',
               full_replay_checked=True, target_cell=78, native=native, grafts=grafts)
    for arms, baseline in [(native, stage['probes'])] + [(grafts[name], row['reference_probes']) for name, row in plan['grafts'].items()]:
        for arm in arms.values():
            arm['cancelled_arrivals'] = 0
            arm['probes'] = {}
            for cue in ('blank', 'cue0', 'cue1'):
                p = deepcopy(baseline[cue])
                p.update(arrivals=[], boundaries=[[at, 78, 1., 0., 1., 0., 0, True]
                                                 for cell, at in p['core_spikes'] if cell == 78])
                arm['probes'][cue] = p
    return source, plan, raw


class ASourceTests(unittest.TestCase):
    def test_reconstruct_native_resets_and_cycle_partition(self):
        source, plan, _ = source_fixture()
        native, grafts = intervention_plans(source, plan)
        self.assertEqual(native['reset_all_weights']['weight_updates'], [[1, .75, .25]])
        arms = grafts['native_initial']
        rows = lambda name: {row[0]: row for row in arms[name]['published_topology']}
        self.assertEqual(rows('cycle_terminal')[1][1], 65)
        self.assertEqual(rows('cycle_terminal')[2][1], 65)
        self.assertEqual(rows('cycle_collateral')[1][1], 64)
        self.assertEqual(rows('cycle_collateral')[2][1], 64)
        self.assertEqual(rows('cycle_full')[1][1], 65)
        self.assertEqual(rows('cycle_full')[2][1], 64)
        self.assertEqual(rows('replace_group')[1][1], 66)
        self.assertFalse(rows('block_group')[1][5])
        self.assertEqual(rows('replace_1')[1][1], 66)

    def test_record_fallback_latency_and_full_event_change(self):
        source, plan, raw = source_fixture()
        report = analyze_record(raw, source, 'test', plan)
        self.assertEqual(report['interventions']['native']['intact']['probes']['cue1']['target_relative_spikes_us'], [1000])
        row = report['interventions']['native_initial']['intact']['probes']['cue1']
        self.assertEqual(row['dose_latency_us'], 1000)
        self.assertIn('carrier intact has none', row['latency_basis'])
        for mutate, message in [
            (lambda r: r['native']['block_active']['blocked_edges'].clear(), 'blocked_edges differs'),
            (lambda r: r['native']['intact']['probes']['cue1']['core_spikes'].clear(), 'intact differs'),
            (lambda r: r['grafts']['native_initial']['cycle_full']['probes']['cue0'].__setitem__('start_us', 0), 'clock'),
            (lambda r: r['native']['intact']['probes']['cue1']['boundaries'][0].__setitem__(3, .5), 'summed input'),
            (lambda r: r['native']['intact'].__setitem__('cancelled_arrivals', 1), 'cancelled arrival'),
        ]:
            altered = deepcopy(raw)
            mutate(altered)
            with self.assertRaisesRegex(ValueError, message):
                analyze_record(altered, source, 'test', plan)

    def test_fixed_latency_inclusive_dose_and_relative_event_lists(self):
        reference = dict(start_us=100, core_spikes=[[78, 110], [78, 120]])
        probe = dict(start_us=200, core_spikes=[[78, 210], [78, 225]],
            arrivals=[[209, 78, 1, .25], [210, 78, 1, .5], [211, 78, 1, 2.], [209, 78, 2, -.5]])
        row = contrast(probe, reference, 78, 10, 'fixed')
        self.assertFalse(row['same_target_events'])
        self.assertEqual(row['target_spike_count_delta'], 0)
        self.assertEqual(row['first_spike_latency_delta_us'], 0)
        self.assertEqual(row['excitatory_dose_through_reference_latency'], .75)
        self.assertEqual(row['excitatory_dose_by_edge'], [[1, 2, .75]])

    def test_directory_exact_file_identity_plan_rebuild_and_hashes(self):
        source, plan, raw = source_fixture()
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            filename = source['job']['id'] + '.json'
            for part, value in [('growth_input', {}), ('transfer_input', {}), ('plans', plan), ('raw', raw)]:
                (root / part).mkdir()
                (root / part / filename).write_text(json.dumps(value))
            with patch('contingency_a_source_analysis.source_record', return_value=(source, 'test')), patch(
                    'contingency_a_source_plan.build_plan', return_value=plan):
                report = analyze_directory(root, smoke=True)
                self.assertEqual(report['cohort_size'], 1)
                self.assertEqual(len(report['records'][0]['growth_input_sha256']), 64)
                self.assertIn(source['job']['seed'], report['summary'])
                with self.assertRaisesRegex(ValueError, 'two archived A hits'):
                    analyze_directory(root)
                changed = deepcopy(plan)
                changed['active_edges'] = []
                (root / 'plans' / filename).write_text(json.dumps(changed))
                with self.assertRaisesRegex(ValueError, 'plan differs'):
                    analyze_directory(root, smoke=True)
                (root / 'plans' / filename).unlink()
                with self.assertRaisesRegex(ValueError, 'file identities differ'):
                    analyze_directory(root, smoke=True)

    def test_arrival_sources_and_native_electrical_properties(self):
        rows = [[1, 64, 78, .25, 1000, True]]
        probe = dict(start_us=0, root_spikes=[], core_spikes=[[64, 0]], rest_core_spikes=0,
            arrived_edges=[1], arrivals=[[1000, 78, 1, .25]],
            boundaries=[[1000, 78, 0., .25, 1., .25, 0, False]])
        validate_trace(probe, 78, {64, 78}, rows)
        probe['core_spikes'] = []
        with self.assertRaisesRegex(ValueError, 'delayed Core source'):
            validate_trace(probe, 78, {64, 78}, rows)
        probe['core_spikes'] = [[64, 0]]
        probe['boundaries'][0][5] = .5
        with self.assertRaisesRegex(ValueError, 'recorded potential'):
            validate_trace(probe, 78, {64, 78}, rows)
        probe['boundaries'][0][5] = .25
        probe['arrivals'][0][3] = float('nan')
        with self.assertRaisesRegex(ValueError, 'electrical value'):
            validate_trace(probe, 78, {64, 78}, rows)


if __name__ == '__main__':
    unittest.main()
