from copy import deepcopy
import json
from pathlib import Path
import sys
import tempfile
from types import SimpleNamespace
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from contingency_lead_growth_analysis import (INTERVENTION, LEAD_RULE, POLICIES, PROTOCOL,
    SCENARIO, aggregate, analyze_directory, analyze_record, choice_ledger, layout, policy_checks)
from contingency_live_rate_analysis import B_NAMES, FRAME_US, PROBES
from test_contingency_live_growth import probe
from verify_contingency import TRAINING, training_sequence


def fixture(seed=34409):
    checkpoint, rest, repeats = 10_001, 100_000, 4
    name, patterns = layout(seed)
    top = [[0, 64, 65, .5, 1000, True]]
    reset = {key: probe(start=checkpoint + rest, pixels=pixels) for key, pixels in patterns.items()}
    config = dict(protocol=PROTOCOL, seed=seed, intervention=INTERVENTION,
        processing_neurons=16, training_repetitions=repeats, rest_us=rest, stimuli=patterns,
        formation_policies=list(POLICIES), layout=name, lead_rule=deepcopy(LEAD_RULE))
    def stage(mapping, index, policy):
        start = checkpoint + index * repeats * 10 * FRAME_US
        schedule = list(training_sequence(mapping, seed, repeats, 0))
        if index:
            schedule = [B_NAMES.get(key, key) for key in schedule]
        frames = []
        for i, key in enumerate(schedule):
            at = start + i * FRAME_US
            frames.append(dict(stimulus=key, start_us=at, end_us=at + FRAME_US - 1,
                root_spikes=[[pixel, at] for pixel in patterns[key]], core_spikes=[],
                weight_changes=0, homeostasis_changes=0, formed=[], pruned=[], exploratory=[], lead_choices=[]))
        return dict(formation_policy=policy, frames=frames, end_topology=deepcopy(top),
            probes={key: probe(start=start + repeats * 10 * FRAME_US + rest, pixels=patterns[key])
                    for key in PROBES})
    trials = {policy: {'a': {am: stage(am, 0, policy) for am in TRAINING},
        'b': {am: {bm: stage(bm, 1, policy) for bm in TRAINING} for am in TRAINING}}
        for policy in POLICIES}
    details = dict(protocol=PROTOCOL, seed=seed, core_ids=list(range(64, 80)), layout=name,
        checkpoint_time_us=checkpoint, stimuli=patterns, initial_topology=top, reset=reset,
        targets={'a': [[], []], 'b': [[], []]}, trials=trials)
    assay = SimpleNamespace(reset=reset, targets={'a': [set(), set()], 'b': [set(), set()]})
    details['checks'], metrics = policy_checks(assay, trials)
    return dict(job=dict(id=f'v00000-s{seed}-i000', seed=seed, variant='v00000',
        intervention=INTERVENTION, effective_config=config), details=details, metrics=metrics,
        full_replay_checked=True)


class LeadGrowthTest(unittest.TestCase):
    def test_layouts_are_bijections_and_predeclared(self):
        for seed, name in ((34409, 'rotate'), (34439, 'interleave')):
            actual, patterns = layout(seed)
            self.assertEqual(actual, name)
            self.assertEqual(set().union(*map(set, patterns.values())), set(range(64)))
            self.assertTrue(all(pixels == sorted(pixels) for pixels in patterns.values()))
            self.assertEqual(analyze_record(fixture(seed))['layout'], name)

    def test_valid_record_and_independent_mutations(self):
        source = fixture()
        self.assertEqual(analyze_record(source)['metrics']['lead_joint_paths'], 0)
        def a(record):
            return record['details']['trials']['lead']['a']['forward']
        mutations = [
            lambda r: r['metrics'].__setitem__('lead_joint_paths', 1),
            lambda r: r.__setitem__('full_replay_checked', False),
            lambda r: r['job']['effective_config']['lead_rule'].__setitem__('lag_us', 100000),
            lambda r: r['details'].__setitem__('layout', 'interleave'),
            lambda r: a(r).__setitem__('formation_policy', 'recent'),
            lambda r: a(r)['frames'][0]['root_spikes'][0].__setitem__(0, 0),
            lambda r: a(r)['frames'][0].__setitem__('lead_choices', [[1, 64, 1, 0]]),
            lambda r: a(r)['end_topology'][0].__setitem__(2, 66),
            lambda r: a(r)['probes']['cue0'].__setitem__('arrived_edges', [99]),
            lambda r: a(r)['probes']['cue0'].__setitem__('start_us', 0),
            lambda r: r['details']['trials']['lead']['b']['forward'].pop('balanced'),
        ]
        for mutate in mutations:
            with self.subTest(mutate=mutate):
                record = deepcopy(source)
                mutate(record)
                with self.assertRaises(ValueError):
                    analyze_record(record)

    def test_choice_alignment_recency_and_lag(self):
        frame = dict(formed=[[1, 64, 66]], exploratory=[[1, 300000, 250000, None, 2, .1]],
                     lead_choices=[[1, 65, 3., 2.]])
        self.assertEqual(choice_ledger([frame], 'lead', {}, {64,65,66}, 10001)['above_recency'], 1)
        for choice in ([1,65,2,2], [1,64,3,2], [1,65,float('nan'),2], [1,65,-1,0], [2,65,3,2]):
            changed = deepcopy(frame)
            changed['lead_choices'] = [choice]
            with self.assertRaises(ValueError):
                choice_ledger([changed], 'lead', {}, {64,65,66}, 10001)
        with self.assertRaises(ValueError):
            choice_ledger([frame], 'recent', {}, {64,65,66}, 10001)
        with self.assertRaisesRegex(ValueError, 'matched delay'):
            choice_ledger([frame], 'lagged', {}, {64,65,66}, 200000)

    def test_policy_specific_a_and_conditional_retention(self):
        record = fixture()
        trials = record['details']['trials']
        assay = SimpleNamespace(reset=record['details']['reset'],
                                targets={'a': [{64}, {65}], 'b': [{66}, {67}]})
        trials['lead']['a']['forward']['probes']['cue0'] = probe(64)
        trials['lead']['b']['forward']['forward']['probes']['cue0'] = probe(64)
        trials['lead']['b']['forward']['forward']['probes']['cue_b0'] = probe(66)
        checks, metrics = policy_checks(assay, trials)
        self.assertTrue(checks['lead']['a_acquisition']['forward']['cue0']['passed'])
        self.assertTrue(checks['lead']['a_retention']['forward']['forward']['cue0']['passed'])
        self.assertFalse(checks['recent']['a_retention']['forward']['forward']['cue0']['eligible'])
        trials['lead']['a']['forward']['probes']['cue_b0'] = probe(66)
        checks, metrics = policy_checks(assay, trials)
        self.assertFalse(checks['lead']['b_acquisition']['forward']['forward']['cue_b0']['passed'])
        result = analyze_record(fixture())
        summary = aggregate([result])
        self.assertEqual(summary['paired_lead']['recent']['b_acquisition']['neither'], 0)
        self.assertEqual(summary['paired_lead']['recent']['per_seed'][0]['joint_paths'], 0)

    def test_fnv_seal_and_smoke_cohort(self):
        record = fixture()
        body = json.dumps(record, separators=(',', ':')).encode()
        checksum = 0xcbf29ce484222325
        for byte in body:
            checksum = ((checksum ^ byte) * 0x100000001b3) & 0xffffffffffffffff
        envelope = b'{"checksum_fnv1a64":"' + f'{checksum:016x}'.encode() + b'","record":' + body + b'}'
        manifest = dict(protocol='NERVA-Research-v1', spec={'experiment': dict(scenario=SCENARIO,
            seeds=[34409], verify_replay=True, interventions=[INTERVENTION])},
            counts=dict(variants=1,seeds=1,interventions=1,trials=1,full_executions=2),jobs=[record['job']])
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary)
            (path / 'records').mkdir()
            (path / 'manifest.json').write_text(json.dumps(manifest))
            raw = path / 'records' / f"{record['job']['id']}.json"
            raw.write_bytes(envelope)
            with self.assertRaisesRegex(ValueError, 'six-seed cohort'):
                analyze_directory(path)
            self.assertEqual(analyze_directory(path, smoke=True)['cohort_size'], 1)
            raw.write_bytes(envelope.replace(b'"full_replay_checked":true', b'"full_replay_checked":false'))
            with self.assertRaisesRegex(ValueError, 'checksum differs'):
                analyze_directory(path, smoke=True)

    def test_joint_path_requires_both_cues_and_prior_a_acquisition(self):
        record = fixture()
        trials = record['details']['trials']
        assay = SimpleNamespace(reset=record['details']['reset'],
                                targets={'a': [{64}, {65}], 'b': [{66}, {67}]})
        lead = trials['lead']
        for cue, cell in (('cue0', 64), ('cue1', 65)):
            lead['a']['forward']['probes'][cue] = probe(cell)
            lead['b']['forward']['forward']['probes'][cue] = probe(cell)
        for cue, cell in (('cue_b0', 66), ('cue_b1', 67)):
            lead['b']['forward']['forward']['probes'][cue] = probe(cell)
        _, metrics = policy_checks(assay, trials)
        self.assertEqual(metrics['lead_joint_paths'], 1)
        lead['b']['forward']['forward']['probes']['cue_b1'] = probe()
        _, metrics = policy_checks(assay, trials)
        self.assertEqual(metrics['lead_joint_paths'], 0)
        lead['b']['forward']['forward']['probes']['cue_b1'] = probe(67)
        lead['a']['forward']['probes']['cue1'] = probe()
        checks, metrics = policy_checks(assay, trials)
        self.assertFalse(checks['lead']['a_retention']['forward']['forward']['cue1']['eligible'])
        self.assertEqual(metrics['lead_joint_paths'], 0)


if __name__ == '__main__':
    unittest.main()
