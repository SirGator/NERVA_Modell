#!/usr/bin/env python3
"""Verify the paired native and rate-corrected sequential A/B contingency assay."""

import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path

from contingency_capacity_analysis import MAPPINGS, identity_set, natural
from contingency_eligibility_analysis import source_record
from contingency_structural_analysis import lifecycles, read_probe, topology
from verify_contingency import FRAME_US, ORIGINAL, OUTCOMES, TRAINING
from verify_contingency import event_counts, require, training_sequence

PROTOCOL = 'NERVA-Contingency-Live-Rate/1'
SCENARIO = 'hippocampal_contingency_live_rate'
INTERVENTION = 'paired_rate_rules'
RATE_TAU_US = 2_000_000
POLICIES = ('native', 'corrected')
CUES = {'a': ORIGINAL, 'b': ('cue_b0', 'cue_b1')}
TASK_OUTCOMES = {'a': OUTCOMES, 'b': ('outcome_b0', 'outcome_b1')}
PROBES = ('blank', *CUES['a'], *CUES['b'])
B_NAMES = dict(zip((*ORIGINAL, *OUTCOMES), (*CUES['b'], *TASK_OUTCOMES['b'])))
GATES = ('a_acquisition', 'b_acquisition', 'a_retention')


def layout():
    def pixels(x, *rows):
        return [8 * y + x + dx for y in rows for dx in range(4)]

    return {
        'blank': [],
        'cue0': pixels(0, 0, 1), 'cue1': pixels(4, 0, 1),
        'outcome0': pixels(0, 4, 7), 'outcome1': pixels(4, 4, 5),
        'cue_b0': pixels(0, 2, 3), 'cue_b1': pixels(4, 2, 3),
        'outcome_b0': pixels(0, 5, 6), 'outcome_b1': pixels(4, 6, 7),
    }


class Assay:
    def __init__(self, details, config, seed):
        self.seed = seed
        self.core = identity_set(details['core_ids'], 'Core')
        self.checkpoint = details['checkpoint_time_us']
        self.rest = config['rest_us']
        self.repetitions = config['training_repetitions']
        require(natural(self.checkpoint) and natural(self.rest) and 100_000 <= self.rest <= 20_000_000,
                'invalid live rate checkpoint or rest clock')
        require(natural(self.repetitions) and 4 <= self.repetitions <= 16
                and self.repetitions % 4 == 0,
                'invalid training repetition count')
        require(16 <= len(self.core) <= 256 and len(self.core) == config['processing_neurons'],
                'Core identities differ from planned population')
        require(details['stimuli'] == layout(), 'stimuli differ from the A/B design')
        self.stimuli = {name: set(pixels) for name, pixels in details['stimuli'].items()}
        self.initial = topology(details['initial_topology'])
        self.reset = details['reset']
        self.duration = self.repetitions * 10 * FRAME_US
        live = {edge for edge, (*_, enabled) in self.initial.items() if enabled}
        require(self.reset.keys() == self.stimuli.keys(), 'unexpected reset probe names')
        for name, probe in self.reset.items():
            start = self.checkpoint + self.rest
            roots = event_counts(probe['root_spikes'], start, start + FRAME_US + 1,
                                 ids=self.stimuli[name], label=f'{name} reset root')
            require(roots.keys() == self.stimuli[name], 'incomplete reset root channel coverage')
            read_probe(probe, name, self, start, live, probe)
        self.targets = target_groups(self.reset)
        require(details['targets'] == {
            task: [sorted(group) for group in groups] for task, groups in self.targets.items()
        }, 'published targets differ from the exclusive reset populations')

    def training(self, stage, mapping, branch, stage_index):
        start = self.checkpoint + stage_index * self.duration
        schedule = list(training_sequence(mapping, self.seed, self.repetitions, 0))
        if stage_index:
            schedule = [B_NAMES.get(name, name) for name in schedule]
        frames = stage['frames']
        require(len(frames) == len(schedule), 'training frame count differs from schedule')
        totals = Counter()
        previous = set()
        for index, (frame, name) in enumerate(zip(frames, schedule)):
            first = start + index * FRAME_US
            stop = first + FRAME_US
            label = f'{branch}/{mapping}/{name}'
            require(frame['stimulus'] == name and frame['start_us'] == first
                    and frame['end_us'] == stop - 1, f'{label}: invalid training schedule or window')
            require(all(natural(frame[field]) for field in ('weight_changes', 'homeostasis_changes')),
                    f'{label}: invalid weight or homeostasis change count')
            current = self.stimuli[name]
            roots = event_counts(frame['root_spikes'], first, stop, ids=current | previous,
                                 label=f'{label} root')
            require(current <= roots.keys() and all(
                channel in current or time == first for channel, time in frame['root_spikes']
            ), f'{label}: training roots disagree with presented stimulus')
            event_counts(frame['core_spikes'], first, stop, ids=self.core, label=f'{label} Core')
            totals.update(roots)
            previous = current
        return schedule, [frame['root_spikes'] for frame in frames], totals

    def branch(self, branch, mapping, stages):
        require(stages.keys() == CUES.keys(), 'unexpected A/B training stages')
        ledgers = {}
        frames, summary = [], {}
        for index, task in enumerate(CUES):
            stage = stages[task]
            ledgers[task] = self.training(stage, mapping, branch, index)
            frames.extend(stage['frames'])
            final = topology(stage['end_topology'])
            require(all(not frame['pruned'] for frame in stage['frames']),
                    'a live rate branch pruned edges under the retention policy')
            lifecycles(self.initial, final, frames, self.core, True)
            live = {edge for edge, (*_, enabled) in final.items() if enabled}
            require(stage['probes'].keys() == set(PROBES), 'unexpected trained probe names')
            for name, probe in stage['probes'].items():
                read_probe(probe, name, self, self.checkpoint + (index + 1) * self.duration + self.rest,
                           live, self.reset[name])
            summary[task] = {
                'core_spikes': sum(len(frame['core_spikes']) for frame in stage['frames']),
                'formed': sum(len(frame['formed']) for frame in stage['frames']),
                'pruned': sum(len(frame['pruned']) for frame in stage['frames']),
                'weight_changes': sum(frame['weight_changes'] for frame in stage['frames']),
                'homeostasis_changes': sum(frame['homeostasis_changes'] for frame in stage['frames']),
            }
        return ledgers, summary


def population(probe):
    return {cell for cell, _ in probe['core_spikes']}


def target_groups(reset):
    excluded = set().union(*(population(reset[name]) for name in PROBES))
    return {task: [population(reset[name]) - population(reset[other]) - excluded
                   for name, other in (outcomes, outcomes[::-1])]
            for task, outcomes in TASK_OUTCOMES.items()}


def gate_rows(reset, targets, arms, task, stage, acquired=None):
    """Score either acquisition or retention using the specified stage snapshot."""
    result = {}
    for flip, mapping in enumerate(MAPPINGS):
        own = arms[mapping][stage]['probes']
        opposite = arms[MAPPINGS[1 ^ flip]][stage]['probes']
        balanced = arms['balanced'][stage]['probes']
        result[mapping] = {}
        for cue_index, cue in enumerate(CUES[task]):
            correct_group = targets[cue_index ^ flip]
            wrong_group = targets[1 ^ cue_index ^ flip]

            def count(probe, group=correct_group):
                return sum(cell in group for cell, _ in probe['core_spikes'])

            correct = count(own[cue])
            wrong = count(own[cue], wrong_group)
            reset_count = count(reset[cue])
            opposite_count = count(opposite[cue])
            balanced_count = count(balanced[cue])
            before_b = count(arms[mapping]['a']['probes'][cue]) if task == 'b' else None
            quiet = (len(own['blank']['core_spikes']) <= len(reset['blank']['core_spikes'])
                     and own[cue]['rest_core_spikes'] <= reset[cue]['rest_core_spikes'])
            eligible = all(targets) and (acquired is None or acquired[mapping][cue]['passed'])
            passed = eligible and quiet and correct > max(
                wrong, reset_count, opposite_count, balanced_count, before_b or 0)
            result[mapping][cue] = dict(
                correct=correct, wrong=wrong, reset=reset_count, opposite=opposite_count,
                balanced=balanced_count, before_b=before_b, quiet=quiet, eligible=eligible,
                passed=passed)
    return result


def pass_counts(checks):
    return {gate: sum(row['passed'] for cues in checks[gate].values() for row in cues.values())
            for gate in GATES}


def analyze_record(record):
    job, details = record['job'], record['details']
    require(details['protocol'] == PROTOCOL and record['full_replay_checked'] is True
            and natural(job['seed']) and details['seed'] == job['seed']
            and details['rate_tau_us'] == RATE_TAU_US, 'live rate record or rule identity differs')
    config = job['effective_config']
    require(config['protocol'] == PROTOCOL and config['seed'] == job['seed']
            and config['intervention'] == INTERVENTION
            and config['rate_tau_us'] == RATE_TAU_US and config['stimuli'] == details['stimuli'],
            'live rate record differs from the effective configuration')
    assay = Assay(details, config, job['seed'])
    branches = details['branches']
    require(branches.keys() == set(POLICIES), 'unexpected live rate policies')
    checks, metrics, activity = {}, {}, {}
    schedules, root_counts = {}, {}
    for policy, arms in branches.items():
        require(arms.keys() == set(TRAINING), 'unexpected live rate mappings')
        activity[policy] = {}
        for mapping, stages in arms.items():
            ledgers, activity[policy][mapping] = assay.branch(policy, mapping, stages)
            for task, (schedule, roots, counts) in ledgers.items():
                key = (task, tuple(schedule))
                require(roots == schedules.setdefault(key, roots),
                        'identically scheduled arms have different root events')
                require(counts == root_counts.setdefault(task, counts),
                        'training root spike counts differ by channel')
        a = gate_rows(assay.reset, assay.targets['a'], arms, 'a', 'a')
        b = gate_rows(assay.reset, assay.targets['b'], arms, 'b', 'b')
        retain = gate_rows(assay.reset, assay.targets['a'], arms, 'a', 'b', a)
        checks[policy] = dict(zip(GATES, (a, b, retain)))
        counts = pass_counts(checks[policy])
        metrics.update({
            f'{policy}_a': counts['a_acquisition'], f'{policy}_b': counts['b_acquisition'],
            f'{policy}_retained_a': counts['a_retention'],
            f'{policy}_joint': int(all(count == 4 for count in counts.values())),
        })
    require(details['checks'] == checks, 'published A/B checks differ from recomputed gates')
    require(record['metrics'] == metrics, 'published metrics differ from recomputed gates')
    return {'job_id': job['id'], 'seed': job['seed'], 'metrics': metrics, 'checks': checks,
            'target_sizes': {task: list(map(len, groups)) for task, groups in assay.targets.items()},
            'training': activity}


def analyze_directory(directory):
    directory = Path(directory)
    manifest = json.loads((directory / 'manifest.json').read_text(encoding='utf-8'))
    experiment = manifest['spec']['experiment']
    seeds = identity_set(experiment['seeds'], 'manifest cohort')
    require(manifest['protocol'] == 'NERVA-Research-v1' and experiment['scenario'] == SCENARIO
            and experiment['verify_replay'] is True and seeds
            and experiment['interventions'] == [INTERVENTION],
            'live rate manifest with full replay required')
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
                and job['intervention'] == INTERVENTION,
                'record job differs from manifest')
        results.append({**analyze_record(record), 'record_checksum': checksum,
                        'record_sha256': hashlib.sha256(raw).hexdigest()})
    summary = {
        policy: {gate: {
            'eligible_cases': sum(
                case['eligible'] for row in results
                for mapping in row['checks'][policy][gate].values() for case in mapping.values()
            ),
            'passed_cases': sum(row['metrics'][f'{policy}_{metric}'] for row in results),
            'full_passes': sum(row['metrics'][f'{policy}_{metric}'] == 4 for row in results),
        } for gate, metric in zip(GATES, ('a', 'b', 'retained_a'))}
        | {'joint_passes': sum(row['metrics'][f'{policy}_joint'] for row in results)}
        for policy in POLICIES
    }
    return {'protocol': PROTOCOL, 'cohort_size': len(results), 'summary': summary,
            'records': sorted(results, key=lambda row: row['seed'])}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path)
    args = parser.parse_args()
    try:
        print(json.dumps(analyze_directory(args.directory), indent=2, allow_nan=False))
    except (OSError, ValueError, KeyError, TypeError, IndexError, OverflowError) as exc:
        parser.exit(1, f'contingency-live-rate: {exc}\n')


if __name__ == '__main__':
    main()
