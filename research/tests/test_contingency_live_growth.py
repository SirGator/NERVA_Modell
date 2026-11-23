from copy import deepcopy
import json
from pathlib import Path
import sys
import tempfile
from types import SimpleNamespace
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from contingency_live_growth_analysis import (INTERVENTION, POLICIES, PROTOCOL, SCENARIO,
    analyze_directory, analyze_record, exploratory_ledger, recompute_checks)
from contingency_live_rate_analysis import B_NAMES, CUES, FRAME_US, PROBES, layout
from verify_contingency import TRAINING, training_sequence


def probe(*cells, start=0, pixels=()):
    return {'start_us': start, 'root_spikes': [[pixel, start] for pixel in pixels],
            'core_spikes': [[cell, start + i] for i, cell in enumerate(cells)],
            'rest_core_spikes': 0, 'arrived_edges': []}


def fixture():
    seed, checkpoint, rest, repeats = 34301, 10_001, 100_000, 4
    patterns = layout()
    top = [[0, 64, 65, .5, 1000, True]]
    reset = {name: probe(start=checkpoint + rest, pixels=pixels)
             for name, pixels in patterns.items()}
    config = dict(protocol=PROTOCOL, seed=seed, intervention=INTERVENTION,
                  processing_neurons=16, training_repetitions=repeats, rest_us=rest,
                  stimuli=patterns, formation_policies=list(POLICIES),
                  a_formation='recent', pruning=False, homeostasis_scope='core')

    def stage(mapping, index, policy):
        start = checkpoint + index * repeats * 10 * FRAME_US
        schedule = list(training_sequence(mapping, seed, repeats, 0))
        if index:
            schedule = [B_NAMES.get(name, name) for name in schedule]
        frames = []
        for i, name in enumerate(schedule):
            at = start + i * FRAME_US
            frames.append(dict(stimulus=name, start_us=at, end_us=at + FRAME_US - 1,
                root_spikes=[[pixel, at] for pixel in patterns[name]], core_spikes=[],
                weight_changes=0, homeostasis_changes=0, formed=[], pruned=[], exploratory=[]))
        return dict(formation_policy=policy, frames=frames, end_topology=deepcopy(top),
                    probes={name: probe(start=start + repeats * 10 * FRAME_US + rest,
                                        pixels=patterns[name]) for name in PROBES})

    a = {mapping: stage(mapping, 0, 'recent') for mapping in TRAINING}
    b = {policy: {am: {bm: stage(bm, 1, policy) for bm in TRAINING} for am in TRAINING}
         for policy in POLICIES}
    details = dict(protocol=PROTOCOL, seed=seed, core_ids=list(range(64, 80)),
                   checkpoint_time_us=checkpoint, stimuli=patterns, initial_topology=top,
                   reset=reset, targets={'a': [[], []], 'b': [[], []]}, stages_a=a, branches_b=b)
    assay = SimpleNamespace(reset=reset, targets={'a': [set(), set()], 'b': [set(), set()]})
    details['checks'], metrics = recompute_checks(assay, a, b)
    return dict(job=dict(id=f'v00000-s{seed}-i000', seed=seed, variant='v00000',
                         intervention=INTERVENTION, effective_config=config),
                details=details, metrics=metrics, full_replay_checked=True)


class ContingencyLiveGrowthTest(unittest.TestCase):
    def test_full_crossed_record_and_mutation_rejection(self):
        record = fixture()
        self.assertEqual(analyze_record(record)['metrics']['off_joint_paths'], 0)
        mutations = [
            lambda r: r['metrics'].__setitem__('recent_joint_paths', 1),
            lambda r: r.__setitem__('full_replay_checked', False),
            lambda r: r['job']['effective_config'].__setitem__('pruning', True),
            lambda r: r['details']['branches_b']['nearest']['forward']['reverse'].__setitem__(
                'formation_policy', 'recent'),
            lambda r: r['details']['branches_b']['off']['balanced']['balanced']['frames'][0]
                .__setitem__('formed', [[1, 64, 66]]),
            lambda r: r['details']['branches_b']['recent']['forward']['reverse']['frames'][0]
                ['root_spikes'][0].__setitem__(0, 0),
            lambda r: r['details']['branches_b']['recent']['forward']['reverse']['probes']['cue0']
                .__setitem__('arrived_edges', [99]),
        ]
        for mutate in mutations:
            with self.subTest(mutate=mutate):
                changed = deepcopy(record)
                mutate(changed)
                with self.assertRaises(ValueError):
                    analyze_record(changed)

    def test_b_comparisons_are_within_common_a_and_require_increment(self):
        record = fixture()
        details = record['details']
        a, b = details['stages_a'], details['branches_b']
        assay = SimpleNamespace(reset=details['reset'], targets={'a': [{64}, {65}], 'b': [{66}, {67}]})
        for policy in POLICIES:
            b[policy]['forward']['forward']['probes']['cue_b0'] = probe(66)
            b[policy]['reverse']['forward']['probes']['cue_b0'] = probe(66, 66, 66)
        checks, _ = recompute_checks(assay, a, b)
        self.assertTrue(checks['b_acquisition']['recent']['forward']['forward']['cue_b0']['passed'])
        a['forward']['probes']['cue_b0'] = probe(66)
        checks, _ = recompute_checks(assay, a, b)
        self.assertFalse(checks['b_acquisition']['recent']['forward']['forward']['cue_b0']['passed'])

    def test_retention_compares_a_histories_under_same_b(self):
        record = fixture()
        details = record['details']
        a, b = details['stages_a'], details['branches_b']
        assay = SimpleNamespace(reset=details['reset'], targets={'a': [{64}, {65}], 'b': [{66}, {67}]})
        a['forward']['probes']['cue0'] = probe(64)
        b['recent']['forward']['forward']['probes']['cue0'] = probe(64)
        b['recent']['forward']['reverse']['probes']['cue0'] = probe(64, 64, 64)
        checks, _ = recompute_checks(assay, a, b)
        retained = checks['a_retention']['recent']['forward']['forward']['cue0']
        self.assertTrue(retained['passed'])
        b['recent']['reverse']['forward']['probes']['cue0'] = probe(64)
        checks, _ = recompute_checks(assay, a, b)
        self.assertFalse(checks['a_retention']['recent']['forward']['forward']['cue0']['passed'])

    def test_exploratory_source_recency_and_one_shot_targets(self):
        frame = dict(end_us=300_525, formed=[[1, 64, 65]],
                     exploratory=[[1, 300_000, 250_000, None, 2, 0.1]])
        self.assertEqual(exploratory_ledger([frame], 'recent'), {65})
        with self.assertRaisesRegex(ValueError, 'target reused'):
            exploratory_ledger([frame], 'recent', {65})
        frame['exploratory'][0][2] = 199_999
        with self.assertRaisesRegex(ValueError, 'stale source'):
            exploratory_ledger([frame], 'recent')
        self.assertEqual(exploratory_ledger([frame], 'nearest'), {65})
        frame['exploratory'][0][3] = 200_000
        with self.assertRaisesRegex(ValueError, 'recently active target'):
            exploratory_ledger([frame], 'nearest')

    def test_directory_seal_cohort_and_smoke_opt_in(self):
        record = fixture()
        body = json.dumps(record, separators=(',', ':')).encode()
        checksum = 0xcbf29ce484222325
        for byte in body:
            checksum = ((checksum ^ byte) * 0x100000001b3) & 0xffffffffffffffff
        envelope = b'{"checksum_fnv1a64":"' + f'{checksum:016x}'.encode() + b'","record":' + body + b'}'
        manifest = dict(protocol='NERVA-Research-v1', spec={'experiment': dict(
            scenario=SCENARIO, seeds=[34301], verify_replay=True, interventions=[INTERVENTION])},
            counts=dict(variants=1, seeds=1, interventions=1, trials=1, full_executions=2),
            jobs=[record['job']])
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary)
            (path / 'records').mkdir()
            (path / 'manifest.json').write_text(json.dumps(manifest))
            source = path / 'records' / f"{record['job']['id']}.json"
            source.write_bytes(envelope)
            with self.assertRaisesRegex(ValueError, 'six-seed cohort'):
                analyze_directory(path)
            self.assertEqual(analyze_directory(path, smoke=True)['cohort_size'], 1)
            source.write_bytes(envelope.replace(b'"full_replay_checked":true',
                                                b'"full_replay_checked":false'))
            with self.assertRaisesRegex(ValueError, 'checksum differs'):
                analyze_directory(path, smoke=True)


if __name__ == '__main__':
    unittest.main()
