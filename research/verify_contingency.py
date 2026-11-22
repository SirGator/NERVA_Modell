#!/usr/bin/env python3
"""Independently verify a native NERVA visual contingency result directory."""

import argparse
from collections import Counter
import hashlib
import json
import math
from pathlib import Path
import re


def require(condition, message):
    if not condition:
        raise ValueError(message)


def records(directory):
    manifest = json.loads((directory / 'manifest.json').read_text(encoding='utf-8'))
    require(manifest['protocol'] == 'NERVA-Research-v1', 'native research manifest required')
    jobs = {job['id']: job for job in manifest['jobs']}
    require(jobs and len(jobs) == len(manifest['jobs']) == manifest['counts']['trials'],
            'invalid manifest job count')
    require(all(re.fullmatch(r'v\d+-s\d+-i\d+', identity) for identity in jobs),
            'invalid manifest job ID')
    expected = {f'{identity}.json' for identity in jobs}
    require({p.name for p in (directory / 'records').glob('*.json')} == expected,
            'result files differ from manifest jobs')
    for identity, job in jobs.items():
        path = directory / 'records' / f'{identity}.json'
        raw = path.read_bytes()
        record = json.loads(raw)['record']
        require(record['job'] == job, f'{identity}: record job differs from manifest')
        yield record, hashlib.sha256(raw).hexdigest()


TRAINING = ('forward', 'reverse', 'balanced')
ORIGINAL = ('cue0', 'cue1')
FRAGMENTS = ('cue0a', 'cue0b', 'cue1a', 'cue1b')
OUTCOMES = ('outcome0', 'outcome1')
STIMULI = ('blank', *ORIGINAL, *FRAGMENTS, *OUTCOMES)
SUFFIXES = ('core', 'all')
FRAME_US = 50_000


def event_counts(rows, first, stop, *, ids=None, label):
    counts = Counter()
    for identity, timestamp in rows:
        require(type(identity) is int and identity >= 0 and (ids is None or identity in ids),
                f'{label}: unexpected spike identity {identity}')
        require(type(timestamp) is int and first <= timestamp < stop,
                f'{label}: spike timestamp outside recorded window')
        counts[identity] += 1
    return counts


def weight_map(rows, edge_ids):
    weights = dict(rows)
    require(len(weights) == len(rows) and weights.keys() == edge_ids,
            'weight vector edge IDs differ')
    require(all(type(weight) in (int, float) and 0 <= weight <= 2 for weight in weights.values()),
            'invalid weight vector value')
    return weights


def training_sequence(arm, seed, repetitions, gap):
    for repetition in range(repetitions):
        first = (seed + repetition) % 2
        invert = arm == 'reverse' or (arm == 'balanced' and repetition % 4 >= 2)
        for cue in (first, 1 - first):
            frames = ['blank'] * 5
            frames[0] = f'cue{cue}'
            frames[1 + gap] = f'outcome{cue ^ invert}'
            yield from frames


def verify_training(arms, core_ids, edge_ids, seed, repetitions, gap, checkpoint):
    require(set(arms) == set(TRAINING), 'training arms differ')
    weights, root_totals, core_totals = {}, {}, {}
    for arm in TRAINING:
        row = arms[arm]
        weights[arm] = weight_map(row['weights'], edge_ids)
        frames = row['frames']
        expected = list(training_sequence(arm, seed, repetitions, gap))
        require([frame['stimulus'] for frame in frames] == expected,
                f'{arm}: training stimulus order differs')
        root_totals[arm], core_totals[arm] = Counter(), Counter()
        for index, frame in enumerate(frames):
            start = checkpoint + FRAME_US * index
            require(frame['start_us'] == start, f'{arm}: invalid training window')
            root_totals[arm].update(event_counts(frame['root_spikes'], start, start + FRAME_US,
                                                 label=f'{arm} root'))
            core_totals[arm].update(event_counts(frame['core_spikes'], start, start + FRAME_US,
                                                 ids=core_ids, label=f'{arm} Core'))
            require(type(frame['weight_changes']) is int and frame['weight_changes'] >= 0,
                    f'{arm}: invalid frame weight change count')
    require(root_totals['forward'] == root_totals['reverse'] == root_totals['balanced'],
            'training root spike counts differ by channel')
    return weights, root_totals['forward'], core_totals


def verify_probes(probes, stimuli, core_ids, edges, initial, trained, probe_start):
    expected_arms = {'reset', *(f'{arm}_{suffix}' for arm in TRAINING for suffix in SUFFIXES)}
    require(probes.keys() == expected_arms, 'probe arms differ')
    populations, rest, roots = {}, {}, {}
    spike_effects = dict.fromkeys(SUFFIXES, 0)
    cue_arrivals = {arm: set() for arm in TRAINING}
    baseline = probes['reset']['responses']
    core_edges = {identity for identity, pre, post, _ in edges if pre in core_ids and post in core_ids}
    for arm, probe in probes.items():
        expected = initial.copy()
        if arm != 'reset':
            learning, suffix = arm.rsplit('_', 1)
            edges_to_use = core_edges if suffix == 'core' else initial.keys()
            expected.update((identity, trained[learning][identity]) for identity in edges_to_use)
        require(weight_map(probe['weights'], initial.keys()) == expected,
                f'{arm}: installed probe weights differ')
        require(probe['responses'].keys() == set(STIMULI), f'{arm}: probe stimuli differ')
        populations[arm], rest[arm], roots[arm] = {}, {}, {}
        for name, row in probe['responses'].items():
            label = f'{arm}/{name}'
            start = row['start_us']
            require(start == probe_start, f'{label}: invalid probe start')
            root_counts = event_counts(row['root_spikes'], start, start + FRAME_US + 1,
                                       ids=stimuli[name], label=f'{label} root')
            require(root_counts.keys() == stimuli[name], f'{label}: incomplete root channel coverage')
            roots[arm][name] = (start, row['root_spikes'])
            populations[arm][name] = event_counts(
                row['core_spikes'], start, start + 4 * FRAME_US, ids=core_ids, label=f'{label} Core')
            if arm != 'reset' and row['core_spikes'] != baseline[name]['core_spikes']:
                spike_effects[suffix] += 1
            rest[arm][name] = row['rest_core_spikes']
            require(type(rest[arm][name]) is int and rest[arm][name] >= 0,
                    f'{label}: invalid rest spike count')
            require(all(type(identity) is int and identity in initial for identity in row['arrived_edges']),
                    f'{label}: unknown arrived edge')
            if arm != 'reset' and suffix == 'core' and name in ORIGINAL:
                cue_arrivals[learning].update(row['arrived_edges'])
    require(all(row == roots['reset'] for row in roots.values()), 'probe root spike times differ by arm')
    changes = {}
    for arm, weights in trained.items():
        deltas = {identity: abs(weights[identity] - initial[identity]) for identity in core_edges
                  if weights[identity] != initial[identity]}
        changes[arm] = {'changed_edges': len(deltas), 'l1': math.fsum(deltas.values()),
                        'linf': max(deltas.values(), default=0.),
                        'changed_edges_with_original_cue_arrivals': len(deltas.keys() & cue_arrivals[arm])}
    return populations, rest, spike_effects, changes


def score(populations, rest):
    baseline = populations['reset']
    excluded = set().union(*(baseline[name] for name in ('blank', *ORIGINAL, *FRAGMENTS)))
    templates = [sorted(set(baseline[name]) - set(baseline[other]) - excluded)
                 for name, other in (OUTCOMES, OUTCOMES[::-1])]
    counts = {arm: {name: [sum(spikes[i] for i in template) for template in templates]
                    for name, spikes in responses.items()}
              for arm, responses in populations.items()}
    gates = {}
    for suffix in SUFFIXES:
        forward, reverse, balanced = (f'{arm}_{suffix}' for arm in TRAINING)

        def passes(name):
            cue = int(name[3])
            for arm, opposite, correct in ((forward, reverse, cue), (reverse, forward, 1 - cue)):
                match, wrong = counts[arm][name][correct], counts[arm][name][1 - correct]
                if match <= max(wrong, counts['reset'][name][correct],
                                counts[balanced][name][correct], counts[opposite][name][correct]):
                    return False
                if rest[arm][name] > rest['reset'][name]:
                    return False
            return True

        quiet_blank = all(populations[arm]['blank'].total() <= baseline['blank'].total()
                          for arm in (forward, reverse))
        gates[suffix] = {key: all(templates) and quiet_blank and all(map(passes, names))
                         for key, names in (('original', ORIGINAL), ('fragments', FRAGMENTS))}
    return templates, counts, gates


def verify_record(record):
    details, job = record['details'], record['job']
    require(details['protocol'] == 'NERVA-Visual-Contingency/1', 'visual contingency record required')
    require(record['full_replay_checked'] is True, 'full replay check required')
    config = job['effective_config']
    gap, repetitions = details['gap_frames'], config['training_repetitions']
    require(type(gap) is int and gap in (0, 2) and gap == config['gap_frames'], 'invalid gap frame count')
    require(type(repetitions) is int and repetitions > 0, 'invalid training repetition count')
    core_ids = set(details['core_ids'])
    require(core_ids and len(core_ids) == len(details['core_ids']) and
            all(type(identity) is int and identity >= 0 for identity in core_ids), 'invalid Core IDs')
    require(details['stimuli'] == config['stimuli'], 'stimuli differ from planned definitions')
    stimuli = {name: set(pixels) for name, pixels in details['stimuli'].items()}
    require(stimuli.keys() == set(STIMULI) and not stimuli['blank'] and
            all(stimuli[name] for name in STIMULI[1:]), 'invalid stimulus definitions')
    edges = details['edges']
    initial = weight_map([(identity, weight) for identity, _, _, weight in edges],
                         {identity for identity, _, _, _ in edges})
    checkpoint = details['development']['checkpoint_time_us']
    rest_us = config['rest_us']
    require(type(checkpoint) is int and checkpoint >= 0 and type(rest_us) is int and rest_us >= 0,
            'invalid checkpoint time or probe rest duration')
    trained, training_roots, training_core = verify_training(
        details['training'], core_ids, initial.keys(), job['seed'], repetitions, gap, checkpoint)
    require(training_roots.keys() == set().union(*(stimuli[name] for name in (*ORIGINAL, *OUTCOMES))),
            'training root channels differ from scheduled stimuli')
    populations, rest, spike_effects, changes = verify_probes(
        details['probes'], stimuli, core_ids, edges, initial, trained, checkpoint + rest_us)
    templates, counts, gates = score(populations, rest)
    require(templates == [sorted(ids) for ids in details['templates']], 'reported templates differ')
    require(counts == details['counts'], 'reported template spike counts differ')
    require(gates == details['gates'], 'reported contingency gates differ')
    expected_metrics = {f'{name}_{suffix}': int(passed) for suffix, row in gates.items()
                        for name, passed in row.items()}
    expected_metrics['templates_available'] = int(all(templates))
    expected_metrics['core_spike_effects'] = spike_effects['core']
    require(all(record['metrics'][key] == value for key, value in expected_metrics.items()),
            'reported contingency metrics differ')
    return {'job_id': job['id'], 'seed': job['seed'], 'gap_frames': gap,
            'training_root_spikes_by_channel': dict(sorted(training_roots.items())),
            'training_core_spikes': {arm: counts.total() for arm, counts in training_core.items()},
            'templates': templates, 'counts': counts, 'gates': gates,
            'core_spike_effects': spike_effects['core'], 'all_spike_effects': spike_effects['all'],
            'training_core_weight_changes': changes}


def verify_directory(directory):
    return {'verified_records': [{**verify_record(record), 'record_sha256': sha256}
                                 for record, sha256 in records(Path(directory))]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path)
    args = parser.parse_args()
    try:
        print(json.dumps(verify_directory(args.directory), indent=2, allow_nan=False))
    except (OSError, ValueError, KeyError, TypeError, IndexError) as exc:
        parser.exit(1, f'verify-contingency: {exc}\n')


if __name__ == '__main__':
    main()
