#!/usr/bin/env python3
"""Verify the crossed A/B comparison of source rule and formation deferral."""

import argparse
import json
from pathlib import Path

from contingency_contrast_growth_analysis import (GATE_FIELDS, TRAJECTORY_FIELDS,
    gate_diagnostics, paired_trajectories)
from contingency_lead_growth_analysis import (analyze_cohort, analyze_trial, cases,
    choice_ledger, joint_paths, paired_summary, policy_summary)
from contingency_live_rate_analysis import GATES, layout as original_layout
from verify_contingency import FRAME_US, TRAINING, require

PROTOCOL = 'NERVA-Contingency-Deferred-Growth/1'
SCENARIO = 'hippocampal_contingency_deferred_growth'
INTERVENTION = 'crossed_deferred_growth'
POLICIES = ('recent', 'lead', 'recent_deferred', 'lead_deferred')
SEEDS = (34601, 34609, 34617, 34627, 34639, 34651)
DEFER_US = 500_000
COMPARISONS = (('lead', 'recent'), ('lead_deferred', 'recent_deferred'),
               ('recent_deferred', 'recent'), ('lead_deferred', 'lead'))
ENDPOINTS = (*GATES, 'joint_paths')
FORMATION_FIELDS = ('formed', 'exploratory', 'before_open', 'after_open',
                    'positive_evidence', 'above_recency')


def layout(seed):
    name = 'column_stride' if seed % 4 == 1 else 'row_stride'

    def transform(pixel):
        row, col = divmod(pixel, 8)
        if name == 'column_stride':
            return 8 * row + (3 * col + 2 * row + 1) % 8
        return 8 * ((3 * row + col + 1) % 8) + 7 - col

    return name, {key: sorted(map(transform, pixels)) for key, pixels in original_layout().items()}


def deferred_choices(frames, policy, initial, core, checkpoint):
    require(all('contrast_components' not in frame for frame in frames),
            'deferred growth frame reported contrast components')
    counts = choice_ledger(frames, policy.removesuffix('_deferred'), initial, core, checkpoint)
    opening = checkpoint + DEFER_US
    before = sum(birth[1] < opening for frame in frames for birth in frame['exploratory'])
    require(not policy.endswith('_deferred') or before == 0,
            'deferred policy formed an exploratory edge before opening')
    return dict(**counts, before_open=before, after_open=counts['choices'] - before)


def verify_closed_prefix(trials, checkpoint):
    opening = checkpoint + DEFER_US
    compared = 0
    for mapping in TRAINING:
        prefixes = []
        for policy in ('recent_deferred', 'lead_deferred'):
            frames = trials[policy]['a'][mapping]['frames']
            prefixes.append([{key: value for key, value in frame.items() if key != 'lead_choices'}
                             for frame in frames if frame['end_us'] < opening])
        left, right = prefixes
        require(len(left) == len(right) == DEFER_US // FRAME_US and left == right,
                'deferred A trajectories differ before opening')
        compared += len(left)
    return compared


def endpoint_cases(row, policy, endpoint):
    if endpoint == 'joint_paths':
        targets_present = all(size > 0 for sizes in row['target_sizes'].values() for size in sizes)
        return {key: (passed, targets_present) for key, passed in joint_paths(row, policy).items()}
    return {key: (case['passed'], case['eligible']) for key, case in cases(row, policy, endpoint).items()}


def factorial_seed(row):
    effects = {}
    for endpoint in ENDPOINTS:
        cells = [endpoint_cases(row, policy, endpoint) for policy in POLICIES]
        all_counts = dict.fromkeys(POLICIES, 0)
        eligible_counts = dict.fromkeys(POLICIES, 0)
        jointly_eligible = 0
        for key in cells[0]:
            eligible = all(cell[key][1] for cell in cells)
            jointly_eligible += eligible
            for policy, cell in zip(POLICIES, cells, strict=True):
                passed = cell[key][0]
                all_counts[policy] += passed
                eligible_counts[policy] += eligible and passed

        def effects_for(counts):
            recent, lead, recent_deferred, lead_deferred = (counts[policy] for policy in POLICIES)
            return dict(passed=counts, lead_effect_immediate=lead - recent,
                        lead_effect_deferred=lead_deferred - recent_deferred,
                        interaction=(lead_deferred - recent_deferred) - (lead - recent))

        effects[endpoint] = dict(
            all_cases=dict(compared=len(cells[0]), **effects_for(all_counts)),
            jointly_eligible=dict(compared=jointly_eligible, **effects_for(eligible_counts)))
    return dict(seed=row['seed'], layout=row['layout'], endpoints=effects)


def factorial_summary(seeds):
    def counts(endpoint, population):
        estimates = [row['endpoints'][endpoint][population] for row in seeds]
        effect = [item['interaction'] for item in estimates if item['compared']]
        return dict(
            cases=sum(item['compared'] for item in estimates),
            seeds_with_cases=len(effect), seeds_without_cases=len(seeds) - len(effect),
            interaction_sum=sum(effect), positive_seeds=sum(value > 0 for value in effect),
            tied_seeds=sum(value == 0 for value in effect),
            negative_seeds=sum(value < 0 for value in effect))

    return {endpoint: {population: counts(endpoint, population)
                       for population in ('all_cases', 'jointly_eligible')} for endpoint in ENDPOINTS}


def analyze_record(record):
    result = analyze_trial(record, protocol=PROTOCOL, intervention=INTERVENTION,
                           policies=POLICIES, trial_layout=layout, read_choices=deferred_choices,
                           snapshot_fields={'exploratory_delay_us': DEFER_US})
    details = record['details']
    trials = details['trials']
    result['compared_closed_frames'] = verify_closed_prefix(trials, details['checkpoint_time_us'])
    result['paired_trajectories'] = {f'{treatment}_vs_{control}':
        paired_trajectories(trials[treatment], trials[control]) for treatment, control in COMPARISONS}
    result['gate_diagnostics'] = {policy: {gate: gate_diagnostics(cases(result, policy, gate).values())
        for gate in GATES} for policy in POLICIES}
    return result


def aggregate(results):
    summaries = {policy: policy_summary(results, policy, ('before_open', 'after_open'))
                 for policy in POLICIES}
    paired = {}
    for treatment, control in COMPARISONS:
        name = f'{treatment}_vs_{control}'
        contrast = paired_summary(results, treatment, control)
        contrast['trajectory'] = {phase: {
            key: sum(row['paired_trajectories'][name][phase][key] for row in results)
            for key in TRAJECTORY_FIELDS} for phase in ('a', 'b')}
        paired[name] = contrast
    phases = {}
    for policy in POLICIES:
        a = [stage for row in results for stage in row['training'][policy]['a'].values()]
        b = [stage for row in results for branches in row['training'][policy]['b'].values()
             for stage in branches.values()]
        phases[policy] = {phase: {field: sum(stage[field] for stage in stages)
                                  for field in FORMATION_FIELDS}
                          for phase, stages in (('a', a), ('b', b))}
    gates = {policy: {gate: {
        key: sum(row['gate_diagnostics'][policy][gate][key] for row in results)
        for key in GATE_FIELDS} for gate in GATES} for policy in POLICIES}
    seeds = [factorial_seed(row) for row in results]
    layouts = sorted({row['layout'] for row in seeds})
    factorial = dict(per_seed=seeds, by_endpoint=factorial_summary(seeds),
                     by_layout={layout_name: factorial_summary(
                         [row for row in seeds if row['layout'] == layout_name]) for layout_name in layouts})
    return dict(policies=summaries, formation_by_phase=phases, paired=paired, factorial=factorial,
                gate_diagnostics=gates, compared_closed_frames=sum(row['compared_closed_frames'] for row in results))


def analyze_directory(directory, smoke=False):
    return analyze_cohort(directory, smoke, protocol=PROTOCOL, scenario=SCENARIO,
                          intervention=INTERVENTION, planned_seeds=SEEDS,
                          analyze=analyze_record, summarize=aggregate)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path)
    parser.add_argument('--smoke', action='store_true', help='allow a non-prospective cohort and workload')
    args = parser.parse_args()
    try:
        print(json.dumps(analyze_directory(args.directory, args.smoke), indent=2, allow_nan=False))
    except (OSError, ValueError, KeyError, TypeError, IndexError, OverflowError) as exc:
        parser.exit(1, f'contingency-deferred-growth: {exc}\n')


if __name__ == '__main__':
    main()
