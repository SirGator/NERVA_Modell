#!/usr/bin/env python3
"""Independently verify the prospectively crossed A/B formation-policy assay."""

import argparse
import hashlib
import json
from pathlib import Path

from contingency_capacity_analysis import MAPPINGS, identity_set, natural
from contingency_eligibility_analysis import number, source_record
from contingency_live_rate_analysis import Assay, PROBES, gate_rows
from contingency_structural_analysis import lifecycles, read_probe, topology
from verify_contingency import TRAINING, require

PROTOCOL = 'NERVA-Contingency-Live-Growth/1'
SCENARIO = 'hippocampal_contingency_live_growth'
INTERVENTION = 'crossed_b_formation'
POLICIES = ('recent', 'nearest', 'off')
SEEDS = (34301, 34307, 34313, 34319, 34327, 34333)


def exploratory_ledger(frames, policy, previous_targets=()):
    targets = set(previous_targets)
    last_decision_time = 0
    for frame in frames:
        formed = {edge: (pre, post) for edge, pre, post in frame['formed']}
        rows = frame['exploratory']
        ids = [row[0] for row in rows]
        require(ids == sorted(ids) and len(ids) == len(set(ids)) and set(ids) <= formed.keys(),
                'exploratory births differ from formed-edge ledger')
        require(policy != 'off' or not rows, 'off policy recorded exploratory births')
        require(len({row[1] for row in rows}) <= 1, 'multiple exploratory times in one frame')
        for edge, time, source_last, target_last, candidates, drive in rows:
            # The scheduler reports its most recent executed event, which may precede the frame end.
            require(natural(time) and last_decision_time <= time <= frame['end_us']
                    and natural(candidates) and candidates > 0 and number(drive) > 0,
                    'invalid exploratory birth time, candidates or drive')
            last_decision_time = time
            require(source_last is None or (natural(source_last) and source_last <= time),
                    'invalid exploratory source clock')
            require(policy != 'recent' or (source_last is not None
                    and time - source_last <= 100_000),
                    'recent formation used a stale source')
            require(target_last is None or (natural(target_last)
                    and target_last <= time and time - target_last > 100_000),
                    'exploratory formation used a recently active target')
            target = formed[edge][1]
            require(target not in targets, 'exploratory target reused across training stages')
            targets.add(target)
    return targets


def count_passed(rows):
    return sum(case['passed'] for cues in rows.values() for case in cues.values())


def recompute_checks(assay, stages_a, branches_b):
    """Compare B mappings within A, and A histories under an identical B mapping."""
    a_arms = {mapping: {'a': stage} for mapping, stage in stages_a.items()}
    acquisition = gate_rows(assay.reset, assay.targets['a'], a_arms, 'a', 'a')
    b_checks, retention = {}, {}
    metrics = {'a_acquisition': count_passed(acquisition)}
    for policy in POLICIES:
        branches = branches_b[policy]
        b_checks[policy] = {}
        for a_mapping in TRAINING:
            arms = {b_mapping: {'a': stages_a[a_mapping], 'b': stage}
                    for b_mapping, stage in branches[a_mapping].items()}
            b_checks[policy][a_mapping] = gate_rows(
                assay.reset, assay.targets['b'], arms, 'b', 'b')
        retention[policy] = {}
        for b_mapping in TRAINING:
            arms = {a_mapping: {'a': stages_a[a_mapping],
                                'b': branches[a_mapping][b_mapping]}
                    for a_mapping in TRAINING}
            retention[policy][b_mapping] = gate_rows(
                assay.reset, assay.targets['a'], arms, 'a', 'b', acquisition)
        metrics[f'{policy}_b_acquisition'] = sum(
            count_passed(rows) for rows in b_checks[policy].values())
        metrics[f'{policy}_a_retention'] = sum(
            count_passed(rows) for rows in retention[policy].values())
        metrics[f'{policy}_joint_paths'] = sum(
            all(case['passed'] for case in b_checks[policy][a][b].values())
            and all(case['passed'] for case in retention[policy][b][a].values())
            for a in MAPPINGS for b in MAPPINGS)
    return {'a_acquisition': acquisition, 'b_acquisition': b_checks,
            'a_retention': retention}, metrics


def analyze_record(record):
    job, details = record['job'], record['details']
    config = job['effective_config']
    require(details['protocol'] == PROTOCOL and record['full_replay_checked'] is True
            and natural(job['seed']) and details['seed'] == job['seed']
            and config['protocol'] == PROTOCOL and config['seed'] == job['seed']
            and config['intervention'] == INTERVENTION
            and config['stimuli'] == details['stimuli']
            and config['formation_policies'] == list(POLICIES)
            and config['a_formation'] == 'recent' and config['pruning'] is False
            and config['homeostasis_scope'] == 'core',
            'live growth record differs from the effective configuration')
    assay = Assay(details, config, job['seed'])
    stages_a, branches_b = details['stages_a'], details['branches_b']
    require(stages_a.keys() == set(TRAINING) and branches_b.keys() == set(POLICIES),
            'unexpected A mappings or B formation policies')
    schedules, root_counts = {}, {}

    def read_stage(stage, mapping, policy, index, initial, previous_targets=()):
        require(stage['formation_policy'] == policy, 'stage formation policy differs')
        schedule, roots, counts = assay.training(stage, mapping, policy, index)
        key = (index, tuple(schedule))
        require(roots == schedules.setdefault(key, roots),
                'identically scheduled arms have different root events')
        require(counts == root_counts.setdefault(index, counts),
                'training root spike counts differ by channel')
        require(all(not frame['pruned'] for frame in stage['frames']),
                'retention policy pruned an edge')
        final = topology(stage['end_topology'])
        lifecycles(initial, final, stage['frames'], assay.core, policy != 'off')
        exploratory = exploratory_ledger(stage['frames'], policy, previous_targets)
        live = {edge for edge, (*_, enabled) in final.items() if enabled}
        require(stage['probes'].keys() == set(PROBES), 'unexpected trained probe names')
        for name, probe in stage['probes'].items():
            read_probe(probe, name, assay,
                       assay.checkpoint + (index + 1) * assay.duration + assay.rest,
                       live, assay.reset[name])
        return final, exploratory, {
            'core_spikes': sum(len(frame['core_spikes']) for frame in stage['frames']),
            'formed': sum(len(frame['formed']) for frame in stage['frames']),
            'exploratory': sum(len(frame['exploratory']) for frame in stage['frames']),
            'pruned': sum(len(frame['pruned']) for frame in stage['frames']),
            'weight_changes': sum(frame['weight_changes'] for frame in stage['frames']),
            'homeostasis_changes': sum(frame['homeostasis_changes'] for frame in stage['frames']),
        }

    topology_a, targets_a, activity_a, activity_b = {}, {}, {}, {}
    for mapping in TRAINING:
        topology_a[mapping], targets_a[mapping], activity_a[mapping] = read_stage(
            stages_a[mapping], mapping, 'recent', 0, assay.initial)
    for policy in POLICIES:
        require(branches_b[policy].keys() == set(TRAINING), 'unexpected B A-strata')
        activity_b[policy] = {}
        for a_mapping in TRAINING:
            arms = branches_b[policy][a_mapping]
            require(arms.keys() == set(TRAINING), 'unexpected B mappings')
            activity_b[policy][a_mapping] = {}
            for b_mapping in TRAINING:
                _, _, activity_b[policy][a_mapping][b_mapping] = read_stage(
                    arms[b_mapping], b_mapping, policy, 1, topology_a[a_mapping],
                    targets_a[a_mapping])
    checks, metrics = recompute_checks(assay, stages_a, branches_b)
    require(details['checks'] == checks, 'published crossed A/B checks differ from recomputed gates')
    require(record['metrics'] == metrics, 'published metrics differ from recomputed gates')
    return {'job_id': job['id'], 'seed': job['seed'], 'metrics': metrics, 'checks': checks,
            'target_sizes': {task: list(map(len, groups)) for task, groups in assay.targets.items()},
            'training': {'a': activity_a, 'b': activity_b}}


def aggregate(results):
    summary = {}
    for policy in POLICIES:
        gates = {}
        for gate in ('b_acquisition', 'a_retention'):
            cases = [case for row in results for stratum in row['checks'][gate][policy].values()
                     for cues in stratum.values() for case in cues.values()]
            gates[gate] = {'eligible_cases': sum(case['eligible'] for case in cases),
                           'passed_cases': sum(case['passed'] for case in cases),
                           'full_seed_passes': sum(row['metrics'][f'{policy}_{gate}'] == 12
                                                   for row in results)}
        summary[policy] = {**gates,
            'joint_paths': sum(row['metrics'][f'{policy}_joint_paths'] for row in results),
            'full_b_strata': sum(count_passed(cases) == 4 for row in results
                                 for cases in row['checks']['b_acquisition'][policy].values()),
            'formed_b': sum(stage['formed'] for row in results
                            for arms in row['training']['b'][policy].values()
                            for stage in arms.values()),
            'exploratory_b': sum(stage['exploratory'] for row in results
                                  for arms in row['training']['b'][policy].values()
                                  for stage in arms.values())}
    paired = {}
    for control in ('nearest', 'off'):
        counts = {'recent_only': 0, 'control_only': 0, 'both': 0, 'neither': 0}
        for row in results:
            checks = row['checks']['b_acquisition']
            for history, directed in checks['recent'].items():
                for mapping, cues in directed.items():
                    for cue, recent in cues.items():
                        other = checks[control][history][mapping][cue]
                        if recent['eligible'] and other['eligible']:
                            outcome = ('neither', 'control_only', 'recent_only', 'both')
                            counts[outcome[2 * recent['passed'] + other['passed']]] += 1
        paired[control] = counts
    return {'a_acquisition': {
        'eligible_cases': sum(case['eligible'] for row in results
                              for cues in row['checks']['a_acquisition'].values()
                              for case in cues.values()),
        'passed_cases': sum(row['metrics']['a_acquisition'] for row in results),
        'full_seed_passes': sum(row['metrics']['a_acquisition'] == 4 for row in results)},
        'policies': summary, 'paired_b': paired}


def analyze_directory(directory, smoke=False):
    directory = Path(directory)
    manifest = json.loads((directory / 'manifest.json').read_text(encoding='utf-8'))
    experiment = manifest['spec']['experiment']
    seeds = identity_set(experiment['seeds'], 'manifest cohort')
    require(manifest['protocol'] == 'NERVA-Research-v1' and experiment['scenario'] == SCENARIO
            and experiment['verify_replay'] is True and seeds
            and experiment['interventions'] == [INTERVENTION],
            'live growth manifest with full replay required')
    require(smoke or seeds == set(SEEDS), 'manifest differs from the prospective six-seed cohort')
    require(manifest['counts'] == {'variants': 1, 'seeds': len(seeds), 'interventions': 1,
                                  'trials': len(seeds), 'full_executions': 2 * len(seeds)},
            'manifest differs from one replay-verified run per seed')
    jobs = {f'v00000-s{seed}-i000': seed for seed in seeds}
    require(len(manifest['jobs']) == len(jobs)
            and {job['id']: job['seed'] for job in manifest['jobs']} == jobs
            and {path.name for path in (directory / 'records').glob('*.json')}
            == {f'{identity}.json' for identity in jobs},
            'result files or jobs differ from manifest cohort')
    results = []
    for job in manifest['jobs']:
        raw = (directory / 'records' / f"{job['id']}.json").read_bytes()
        record, checksum = source_record(raw)
        require(record['job'] == job and job['variant'] == 'v00000'
                and job['intervention'] == INTERVENTION, 'record job differs from manifest')
        fixed = {'processing_neurons': 256, 'training_repetitions': 8, 'rest_us': 11_000_000}
        require(smoke or all(job['effective_config'][key] == value for key, value in fixed.items()),
                'record differs from prospective workload')
        results.append({**analyze_record(record), 'record_checksum': checksum,
                        'record_sha256': hashlib.sha256(raw).hexdigest()})
    return {'protocol': PROTOCOL, 'smoke': smoke, 'cohort_size': len(results),
            'summary': aggregate(results), 'records': sorted(results, key=lambda row: row['seed'])}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path)
    parser.add_argument('--smoke', action='store_true', help='allow a non-prospective cohort')
    args = parser.parse_args()
    try:
        print(json.dumps(analyze_directory(args.directory, args.smoke), indent=2, allow_nan=False))
    except (OSError, ValueError, KeyError, TypeError, IndexError, OverflowError) as exc:
        parser.exit(1, f'contingency-live-growth: {exc}\n')


if __name__ == '__main__':
    main()
