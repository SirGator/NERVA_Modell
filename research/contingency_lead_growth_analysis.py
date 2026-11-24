#!/usr/bin/env python3
"""Independently verify the prospective local lead/lag growth comparison."""

import argparse
import hashlib
import json
from pathlib import Path

from contingency_capacity_analysis import MAPPINGS, identity_set, natural
from contingency_eligibility_analysis import number, source_record
from contingency_live_growth_analysis import exploratory_ledger, recompute_checks
from contingency_live_rate_analysis import Assay, PROBES, layout as original_layout
from contingency_structural_analysis import lifecycles, read_probe, topology
from verify_contingency import TRAINING, require

PROTOCOL = 'NERVA-Contingency-Lead-Growth/1'
SCENARIO = 'hippocampal_contingency_lead_growth'
INTERVENTION = 'crossed_lead_growth'
POLICIES = ('recent', 'lead', 'lagged')
SEEDS = (34409, 34421, 34429, 34439, 34447, 34459)
LEAD_RULE = dict(sample_interval_us=10_000, min_lead_us=1000, max_lead_us=75_000,
                 lag_us=157_000, decay_us=2_000_000,
                 age_tiers_us=[15_000, 40_000, 75_000], age_scores=[3, 2, 1])


def layout(seed):
    name = 'interleave' if seed % 4 == 3 else 'rotate'
    rows = [0, 2, 4, 6, 1, 3, 5, 7]
    def transform(pixel):
        row, col = divmod(pixel, 8)
        return 8 * rows[row] + 7 - col if name == 'interleave' else 8 * col + 7 - row
    return name, {key: sorted(map(transform, pixels)) for key, pixels in original_layout().items()}


def choice_rows(frame, core):
    exploratory = frame['exploratory']
    choices = frame['lead_choices']
    require([row[0] for row in choices] == [row[0] for row in exploratory],
            'lead choices differ from exploratory birth order')
    formed = {edge: (pre, post) for edge, pre, post in frame['formed']}
    for (edge, recent, chosen_score, recent_score), birth in zip(choices, exploratory):
        chosen, target = formed[edge]
        require(natural(recent) and recent in core and recent != target,
                'invalid recency candidate identity')
        chosen_score, recent_score = number(chosen_score), number(recent_score)
        require((chosen == recent and chosen_score == recent_score)
                or (chosen != recent and chosen_score > recent_score and birth[4] > 1),
                'chosen source lacks evidence above recency')
        yield birth, chosen, recent, chosen_score, recent_score


def choice_ledger(frames, policy, initial, core, checkpoint):
    counts = dict(choices=0, above_recency=0, positive_evidence=0)
    for frame in frames:
        for birth, chosen, recent, chosen_score, recent_score in choice_rows(frame, core):
            require(chosen_score >= 0 and recent_score >= 0, 'negative lead choice evidence')
            require(policy != 'recent' or (chosen == recent and chosen_score == recent_score == 0),
                    'recency baseline used lead evidence')
            require(policy == 'recent' or birth[1] >= checkpoint + LEAD_RULE['lag_us']
                    or chosen_score == recent_score == 0,
                    'lead choice used evidence before its matched delay')
            counts['choices'] += 1
            counts['above_recency'] += chosen != recent
            counts['positive_evidence'] += chosen_score > 0
    return counts


def policy_checks(assay, trials, policies=POLICIES):
    # The established crossed design is applied independently within each policy.
    checks, metrics = {}, {}
    for policy in policies:
        arms = trials[policy]
        old, scores = recompute_checks(assay, arms['a'], {policy: arms['b']}, (policy,))
        checks[policy] = dict(a_acquisition=old['a_acquisition'],
                              b_acquisition=old['b_acquisition'][policy],
                              a_retention=old['a_retention'][policy])
        metrics[f'{policy}_a_acquisition'] = scores['a_acquisition']
        for gate in ('b_acquisition', 'a_retention', 'joint_paths'):
            metrics[f'{policy}_{gate}'] = scores[f'{policy}_{gate}']
    return checks, metrics


def analyze_record(record):
    return analyze_trial(record, protocol=PROTOCOL, intervention=INTERVENTION,
                         policies=POLICIES, trial_layout=layout, read_choices=choice_ledger)


def analyze_trial(record, *, protocol, intervention, policies, trial_layout, read_choices,
                  snapshot_fields=None):
    job, details = record['job'], record['details']
    config = job['effective_config']
    require(natural(job['seed']), 'invalid seed')
    name, stimuli = trial_layout(job['seed'])
    expected = dict(protocol=protocol, seed=job['seed'], intervention=intervention,
                    stimuli=stimuli, formation_policies=list(policies), layout=name,
                    lead_rule=LEAD_RULE, **(snapshot_fields or {}))
    require(record['full_replay_checked'] is True
            and all(config[key] == value for key, value in expected.items())
            and details['protocol'] == protocol and details['seed'] == job['seed']
            and details['layout'] == name and details['stimuli'] == stimuli,
            'formation growth record differs from effective configuration or layout')
    assay = Assay(details, config, job['seed'], expected_stimuli=stimuli)
    trials = details['trials']
    require(trials.keys() == set(policies), 'unexpected formation policies')
    schedules, root_counts, training = {}, {}, {}

    def stage_read(stage, mapping, policy, index, initial, previous=()):
        require(stage['formation_policy'] == policy, 'stage formation policy differs')
        schedule, roots, counts = assay.training(stage, mapping, policy, index)
        require(roots == schedules.setdefault((index, tuple(schedule)), roots),
                'identically scheduled arms have different root events')
        require(counts == root_counts.setdefault(index, counts),
                'training root spike counts differ by channel')
        frames = stage['frames']
        require(all(not frame['pruned'] for frame in frames), 'retention policy pruned an edge')
        final = topology(stage['end_topology'])
        lifecycles(initial, final, frames, assay.core, True)
        targets = exploratory_ledger(frames, 'recent', previous)
        choices = read_choices(frames, policy, initial, assay.core, assay.checkpoint)
        live = {edge for edge, (*_, enabled) in final.items() if enabled}
        require(stage['probes'].keys() == set(PROBES), 'unexpected trained probe names')
        for probe_name, probe in stage['probes'].items():
            read_probe(probe, probe_name, assay,
                       assay.checkpoint + (index + 1) * assay.duration + assay.rest,
                       live, assay.reset[probe_name])
        activity = {key: sum(len(frame[key]) for frame in frames)
                    for key in ('core_spikes', 'formed', 'exploratory', 'pruned')}
        activity.update({key: sum(frame[key] for frame in frames)
                         for key in ('weight_changes', 'homeostasis_changes')})
        return final, targets, {**activity, **choices,
            'formed_pairs': sorted([pre, post] for frame in frames
                                   for _, pre, post in frame['formed'])}

    for policy in policies:
        arm = trials[policy]
        require(arm.keys() == {'a', 'b'} and arm['a'].keys() == set(TRAINING)
                and arm['b'].keys() == set(TRAINING), 'unexpected A histories')
        training[policy] = {'a': {}, 'b': {}}
        for a_mapping in TRAINING:
            final, targets, activity = stage_read(arm['a'][a_mapping], a_mapping, policy, 0,
                                                  assay.initial)
            training[policy]['a'][a_mapping] = activity
            branches = arm['b'][a_mapping]
            require(branches.keys() == set(TRAINING), 'unexpected B mappings')
            training[policy]['b'][a_mapping] = {}
            for b_mapping in TRAINING:
                _, _, activity = stage_read(branches[b_mapping], b_mapping, policy, 1,
                                             final, targets)
                training[policy]['b'][a_mapping][b_mapping] = activity
    checks, metrics = policy_checks(assay, trials, policies)
    require(details['checks'] == checks, 'published crossed A/B checks differ from recomputed gates')
    require(record['metrics'] == metrics, 'published metrics differ from recomputed gates')
    return dict(job_id=job['id'], seed=job['seed'], layout=name, checks=checks, metrics=metrics,
                target_sizes={task: list(map(len, groups)) for task, groups in assay.targets.items()},
                training=training)


def cases(row, policy, gate):
    values = row['checks'][policy][gate]
    if gate == 'a_acquisition':
        return {(mapping, cue): case for mapping, cues in values.items() for cue, case in cues.items()}
    return {(stratum, mapping, cue): case for stratum, mappings in values.items()
            for mapping, cues in mappings.items() for cue, case in cues.items()}


def joint_paths(row, policy):
    checks = row['checks'][policy]
    return {(a, b): all(case['passed'] for case in checks['b_acquisition'][a][b].values())
            and all(case['passed'] for case in checks['a_retention'][b][a].values())
            for a in MAPPINGS for b in MAPPINGS}


def graph_contrast(row, control, treatment='lead'):
    def stages(policy):
        train = row['training'][policy]
        return {('a', mapping): stage for mapping, stage in train['a'].items()} | {
            ('b', a, b): stage for a, arms in train['b'].items() for b, stage in arms.items()}
    trial, other = stages(treatment), stages(control)
    common = treatment_only = control_only = identical = 0
    for key, stage in trial.items():
        left = set(map(tuple, stage['formed_pairs']))
        right = set(map(tuple, other[key]['formed_pairs']))
        common += len(left & right)
        treatment_only += len(left - right)
        control_only += len(right - left)
        identical += left == right
    return dict(common_formed_pairs=common, **{f'{treatment}_only_formed_pairs': treatment_only},
                control_only_formed_pairs=control_only, identical_stage_graphs=identical,
                compared_stages=len(trial))


def policy_summary(results, policy, extra_formation_fields=()):
    summary = {}
    for gate, full in (('a_acquisition', 4), ('b_acquisition', 12), ('a_retention', 12)):
        rows = [case for row in results for case in cases(row, policy, gate).values()]
        summary[gate] = dict(eligible_cases=sum(case['eligible'] for case in rows),
            passed_cases=sum(case['passed'] for case in rows),
            full_seed_passes=sum(row['metrics'][f'{policy}_{gate}'] == full for row in results))
    summary['joint_paths'] = sum(row['metrics'][f'{policy}_joint_paths'] for row in results)
    stages = [stage for row in results for stage in row['training'][policy]['a'].values()]
    stages += [stage for row in results for branches in row['training'][policy]['b'].values()
               for stage in branches.values()]
    summary['formation'] = {key: sum(stage[key] for stage in stages)
        for key in ('formed', 'exploratory', 'choices', 'above_recency', 'positive_evidence',
                    *extra_formation_fields)}
    return summary


def paired_summary(results, treatment, control):
    paired = {}
    outcome = ('neither', 'control_only', f'{treatment}_only', 'both')
    count_fields = (f'{treatment}_only', 'control_only', 'both', 'neither')
    gates = ('a_acquisition', 'b_acquisition', 'a_retention')
    for gate in gates:
        counts = dict.fromkeys(count_fields, 0)
        counts['jointly_eligible'] = 0
        for row in results:
            trial, other = cases(row, treatment, gate), cases(row, control, gate)
            for key, value in trial.items():
                if value['eligible'] and other[key]['eligible']:
                    counts['jointly_eligible'] += 1
                    counts[outcome[2 * value['passed'] + other[key]['passed']]] += 1
        paired[gate] = counts
    paired['per_seed'] = [dict(seed=row['seed'], **{gate: row['metrics'][f'{treatment}_{gate}']
        - row['metrics'][f'{control}_{gate}'] for gate in (*gates, 'joint_paths')}) for row in results]
    joint = dict.fromkeys(count_fields, 0)
    for row in results:
        trial, other = joint_paths(row, treatment), joint_paths(row, control)
        for key, passed in trial.items():
            joint[outcome[2 * passed + other[key]]] += 1
    paired['joint_paths'] = joint
    paired['formed_graphs'] = [dict(seed=row['seed'], **graph_contrast(row, control, treatment))
                                for row in results]
    return paired


def aggregate(results):
    return dict(policies={policy: policy_summary(results, policy) for policy in POLICIES},
                paired_lead={control: paired_summary(results, 'lead', control)
                             for control in ('recent', 'lagged')})


def analyze_directory(directory, smoke=False):
    return analyze_cohort(directory, smoke, protocol=PROTOCOL, scenario=SCENARIO,
                          intervention=INTERVENTION, planned_seeds=SEEDS,
                          analyze=analyze_record, summarize=aggregate)


def analyze_cohort(directory, smoke, *, protocol, scenario, intervention, planned_seeds,
                   analyze, summarize):
    directory = Path(directory)
    manifest = json.loads((directory / 'manifest.json').read_text(encoding='utf-8'))
    experiment = manifest['spec']['experiment']
    seeds = identity_set(experiment['seeds'], 'manifest cohort')
    require(manifest['protocol'] == 'NERVA-Research-v1' and experiment['scenario'] == scenario
            and experiment['verify_replay'] is True and seeds
            and experiment['interventions'] == [intervention], 'formation growth manifest with full replay required')
    require(smoke or seeds == set(planned_seeds), 'manifest differs from the prospective six-seed cohort')
    require(manifest['counts'] == dict(variants=1, seeds=len(seeds), interventions=1,
            trials=len(seeds), full_executions=2 * len(seeds)), 'manifest run counts differ')
    jobs = {f'v00000-s{seed}-i000': seed for seed in seeds}
    require(len(manifest['jobs']) == len(jobs)
            and {job['id']: job['seed'] for job in manifest['jobs']} == jobs
            and {path.name for path in (directory / 'records').glob('*.json')}
            == {f'{identity}.json' for identity in jobs}, 'result files or jobs differ from manifest cohort')
    results = []
    for job in manifest['jobs']:
        raw = (directory / 'records' / f"{job['id']}.json").read_bytes()
        record, checksum = source_record(raw)
        require(record['job'] == job and job['variant'] == 'v00000'
                and job['intervention'] == intervention, 'record job differs from manifest')
        fixed = dict(processing_neurons=256, training_repetitions=8, rest_us=11_000_000)
        require(smoke or all(job['effective_config'][key] == value for key, value in fixed.items()),
                'record differs from prospective workload')
        results.append({**analyze(record), 'record_checksum': checksum,
                        'record_sha256': hashlib.sha256(raw).hexdigest()})
    results.sort(key=lambda row: row['seed'])
    return dict(protocol=protocol, smoke=smoke, cohort_size=len(results),
                summary=summarize(results), records=results)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path)
    parser.add_argument('--smoke', action='store_true', help='allow a non-prospective cohort and workload')
    args = parser.parse_args()
    try:
        print(json.dumps(analyze_directory(args.directory, args.smoke), indent=2, allow_nan=False))
    except (OSError, ValueError, KeyError, TypeError, IndexError, OverflowError) as exc:
        parser.exit(1, f'contingency-lead-growth: {exc}\n')


if __name__ == '__main__':
    main()
