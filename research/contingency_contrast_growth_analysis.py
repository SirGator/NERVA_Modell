#!/usr/bin/env python3
"""Verify the prospective local observed-minus-lagged growth comparison."""

import argparse
import json
from pathlib import Path

from contingency_eligibility_analysis import number
from contingency_lead_growth_analysis import (LEAD_RULE, analyze_cohort, analyze_trial, cases,
    choice_ledger, choice_rows, paired_summary, policy_summary)
from contingency_live_rate_analysis import GATES, PROBES, layout as original_layout
from verify_contingency import TRAINING, require

PROTOCOL = 'NERVA-Contingency-Contrast-Growth/1'
SCENARIO = 'hippocampal_contingency_contrast_growth'
INTERVENTION = 'crossed_contrast_growth'
CONTROLS = ('recent', 'lead', 'lagged')
POLICIES = (*CONTROLS, 'contrast')
SEEDS = (34501, 34513, 34525, 34531, 34543, 34559)
CONTRAST_RULE = 'observed_minus_lagged'
CONTRAST_FIELDS = ('observed_evidence', 'lagged_evidence', 'negative_evidence',
                   'above_recency_with_negative_score')
GATE_FIELDS = ('eligible', 'quiet', 'target_response', 'beats_balanced',
               'ties_balanced_with_response', 'beats_reverse', 'ties_reverse_with_response',
               'beats_pre_b')
TRAJECTORY_FIELDS = ('compared_stages', 'changed_training_core', 'compared_probes',
                     'changed_probe_core')


def layout(seed):
    name = 'shear' if seed % 4 == 1 else 'stagger'

    def transform(pixel):
        row, col = divmod(pixel, 8)
        if name == 'shear':
            return 8 * row + (col + row + 1) % 8
        return 8 * ((col + 2 * row + 1) % 8) + 7 - row

    return name, {key: sorted(map(transform, pixels)) for key, pixels in original_layout().items()}


def contrast_choices(frames, policy, initial, core, checkpoint):
    if policy != 'contrast':
        require(all('contrast_components' not in frame for frame in frames),
                'control frame reported contrast components')
        return choice_ledger(frames, policy, initial, core, checkpoint)
    counts = dict.fromkeys(('choices', 'above_recency', 'positive_evidence', *CONTRAST_FIELDS), 0)
    for frame in frames:
        components = frame['contrast_components']
        require([row[0] for row in components] == [row[0] for row in frame['exploratory']],
                'contrast components differ from exploratory birth order')
        for choice, component in zip(choice_rows(frame, core), components, strict=True):
            birth, chosen, recent, chosen_score, recent_score = choice
            _, observed, lagged, recent_observed, recent_lagged = component
            channels = [number(value) for value in component[1:]]
            require(all(value >= 0 for value in channels), 'negative lead component')
            require(chosen_score == observed - lagged and recent_score == recent_observed - recent_lagged,
                    'contrast score differs from observed minus lagged evidence')
            require(chosen != recent or (observed, lagged) == (recent_observed, recent_lagged),
                    'the same source has different contrast components')
            require(birth[1] >= checkpoint + LEAD_RULE['lag_us'] or not any(channels),
                    'contrast used evidence before its matched delay')
            counts['choices'] += 1
            counts['above_recency'] += chosen != recent
            counts['positive_evidence'] += chosen_score > 0
            counts['negative_evidence'] += chosen_score < 0
            counts['observed_evidence'] += observed > 0
            counts['lagged_evidence'] += lagged > 0
            counts['above_recency_with_negative_score'] += chosen != recent and chosen_score < 0
    return counts


def paired_trajectories(trial, control):
    result = {phase: dict.fromkeys(TRAJECTORY_FIELDS, 0) for phase in ('a', 'b')}

    def compare(phase, left, right):
        counts = result[phase]
        counts['compared_stages'] += 1
        counts['changed_training_core'] += any(
            a['core_spikes'] != b['core_spikes']
            for a, b in zip(left['frames'], right['frames'], strict=True))
        counts['compared_probes'] += len(PROBES)
        counts['changed_probe_core'] += sum(
            left['probes'][cue]['core_spikes'] != right['probes'][cue]['core_spikes']
            for cue in PROBES)

    for a in TRAINING:
        compare('a', trial['a'][a], control['a'][a])
        for b in TRAINING:
            compare('b', trial['b'][a][b], control['b'][a][b])
    return result


def gate_diagnostics(rows):
    eligible = [case for case in rows if case['eligible']]
    return dict(eligible=len(eligible), quiet=sum(case['quiet'] for case in eligible),
        target_response=sum(case['correct'] > 0 for case in eligible),
        beats_balanced=sum(case['correct'] > case['balanced'] for case in eligible),
        ties_balanced_with_response=sum(case['correct'] == case['balanced'] and case['correct'] > 0
                                        for case in eligible),
        beats_reverse=sum(case['correct'] > case['opposite'] for case in eligible),
        ties_reverse_with_response=sum(case['correct'] == case['opposite'] and case['correct'] > 0
                                       for case in eligible),
        beats_pre_b=sum(case['before_b'] is not None and case['correct'] > case['before_b']
                        for case in eligible))


def analyze_record(record):
    result = analyze_trial(record, protocol=PROTOCOL, intervention=INTERVENTION,
                           policies=POLICIES, trial_layout=layout, read_choices=contrast_choices,
                           snapshot_fields={'contrast_rule': CONTRAST_RULE})
    arms = record['details']['trials']
    result['paired_trajectories'] = {
        control: paired_trajectories(arms['contrast'], arms[control]) for control in CONTROLS}
    result['gate_diagnostics'] = {policy: {gate: gate_diagnostics(cases(result, policy, gate).values())
        for gate in GATES} for policy in POLICIES}
    return result


def aggregate(results):
    summaries = {policy: policy_summary(results, policy, CONTRAST_FIELDS if policy == 'contrast' else ())
                 for policy in POLICIES}
    paired = {control: paired_summary(results, 'contrast', control) for control in CONTROLS}
    gates = {policy: {gate: {key: sum(row['gate_diagnostics'][policy][gate][key] for row in results)
        for key in GATE_FIELDS} for gate in GATES} for policy in POLICIES}
    for control in CONTROLS:
        paired[control]['trajectory'] = {phase: {
            key: sum(row['paired_trajectories'][control][phase][key] for row in results)
            for key in TRAJECTORY_FIELDS} for phase in ('a', 'b')}
    return dict(policies=summaries, paired_contrast=paired, gate_diagnostics=gates)


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
        parser.exit(1, f'contingency-contrast-growth: {exc}\n')


if __name__ == '__main__':
    main()
