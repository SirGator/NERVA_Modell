#!/usr/bin/env python3
"""Verify and compare equal-dose interventions on the balanced grown Core graph."""

import argparse
from collections import Counter, defaultdict
import hashlib
import json
import math
from pathlib import Path

from contingency_capacity_analysis import Assay, MAPPINGS, analyze_record as analyze_capacity
from contingency_chemical_analysis import finite, norm, normalization, verify_plan
from contingency_consolidation_analysis import PROTOCOL as CONSOLIDATION
from contingency_electrical_analysis import f32, read_cue
from contingency_eligibility_analysis import source_record
from contingency_structural_analysis import read_probe, topology
from verify_contingency import FRAME_US, ORIGINAL, require, weight_map

PROTOCOL = 'NERVA-Contingency-Grown-Dose/1'
LEARNED = (*MAPPINGS, 'balanced')
DIRECTIONS = (*LEARNED, 'shuffle_forward', 'shuffle_reverse', 'reference_forward',
              'reference_reverse')
TIERS = ('ray', 'bounded')
PROBES = (*ORIGINAL, 'blank')
MASK64 = (1 << 64) - 1


def permutation(seed, postsynaptic):
    """Replay the fixed within-target Fisher–Yates permutation."""
    groups = defaultdict(list)
    for index, target in enumerate(postsynaptic):
        groups[target].append(index)
    state = (seed ^ 0xd6e8feb86659fd93) & MASK64
    result = list(range(len(postsynaptic)))
    for target in sorted(groups):
        group = groups[target]
        shuffled = group.copy()
        for index in range(len(group) - 1, 0, -1):
            state = (state + 0x9e3779b97f4a7c15) & MASK64
            value = ((state ^ (state >> 30)) * 0xbf58476d1ce4e5b9) & MASK64
            value = ((value ^ (value >> 27)) * 0x94d049bb133111eb) & MASK64
            swap = (value ^ (value >> 31)) % (index + 1)
            shuffled[index], shuffled[swap] = shuffled[swap], shuffled[index]
        for index, shuffled_index in zip(group, shuffled):
            result[index] = shuffled_index
    return result


def headrooms(initial, direction):
    return [2. - weight if delta > 0 else weight if delta < 0 else 0.
            for weight, delta in zip(initial, direction)]


def ray_limit(room, direction):
    length = norm(direction)
    return min((headroom / abs(delta / length)
                for headroom, delta in zip(room, direction) if delta), default=None)


def bounded_plan(initial, direction, requested):
    room = headrooms(initial, direction)
    low, high = 0., max((headroom / abs(delta)
                         for headroom, delta in zip(room, direction) if delta), default=0.)

    def publish(scale):
        raw = [weight + scale * delta for weight, delta in zip(initial, direction)]
        weights = [f32(min(2., max(0., value))) for value in raw]
        changes = [weight - base for weight, base in zip(weights, initial)]
        return weights, changes, norm(changes), sum(value < 0. or value > 2. for value in raw)

    scale, best = 0., publish(0.)
    for _ in range(64):
        middle = (low + high) / 2.
        candidate = publish(middle)
        if abs(candidate[2] - requested) < abs(best[2] - requested):
            scale, best = middle, candidate
        if candidate[2] < requested:
            low = middle
        else:
            high = middle
    weights, changes, actual, clipped = best
    cosine = (min(1., max(-1., sum(a * b for a, b in zip(changes, direction))
                          / (actual * norm(direction)))) if actual else None)
    return weights, dict(scale=scale, actual_norm=actual,
                         relative_norm_error=abs(actual - requested) / requested,
                         direction_cosine=cosine, clipped_coordinates=clipped)


def published(rows, support):
    require([edge for edge, _ in rows] == support, 'applied weight order differs from support')
    return [f32(weight) for weight in weight_map(rows, set(support)).values()]


def archived_weights(rows):
    return {edge: f32(weight) for edge, _, _, weight, *_ in rows}


def reconstruct(raw, source, consolidation, checksum):
    job, details = source['job'], source['details']
    config = job['effective_config']
    assay = Assay(details, config)
    seed = job['seed']
    for record, protocol in ((raw, PROTOCOL), (consolidation, CONSOLIDATION)):
        require(record['protocol'] == protocol and record['replay_verified'] is True
                and record['seed'] == seed and record['archived_record_checksum'] == checksum
                and record['checkpoint_time_us'] == assay.checkpoint
                and record['core_ids'] == details['core_ids'], 'archived checkpoint identity differs')
    require(raw['target_groups'] == details['target_groups'], 'target groups differ from capacity')
    support = sorted(edge for edge, _, _, _, eligible in assay.topology if eligible)
    require(support and raw['support'] == support, 'support differs from eligible developed edges')
    require(consolidation['mappings'].keys() == set(LEARNED), 'unexpected consolidation mappings')
    training = {key: row['training'] for key, row in consolidation['mappings'].items()}
    carrier = training['balanced']['end_topology']
    require(raw['carrier_topology'] == carrier, 'carrier differs from archived balanced topology')
    final = {key: topology(trial['end_topology']) for key, trial in training.items()}
    for key, edges in final.items():
        require(all(
            identity in edges and edges[identity][:2] == (pre, post) and edges[identity][3]
            for identity, pre, post, _, eligible in assay.topology if eligible),
            f'{key}: final topology changed an original support edge')
    initial = archived_weights(carrier)
    baseline = [initial[edge] for edge in support]
    values = {key: archived_weights(trial['end_topology']) for key, trial in training.items()}
    directions = {key: [values[key][edge] - assay.initial[edge] for edge in support]
                  for key in LEARNED}
    shuffle = permutation(seed, [final['balanced'][edge][1] for edge in support])
    followup = details['followup']
    require(followup is not None and raw['reference_choice'] == followup['choice'],
            'reference choice differs from capacity follow-up')
    references = details['references'][followup['choice']]['arms']
    for key in MAPPINGS:
        directions[f'shuffle_{key}'] = [directions[key][index] for index in shuffle]
        reference = {edge: f32(weight) for edge, weight in references[key]['weights']}
        directions[f'reference_{key}'] = [
            reference[edge] - base if reference[edge] != assay.initial[edge] else 0.
            for edge, base in zip(support, baseline)
        ]
    require(raw['directions'].keys() == set(DIRECTIONS), 'direction set differs from protocol')
    for key, vector in directions.items():
        reported = raw['directions'][key]
        require(all(map(finite, reported)) and reported == vector,
                f'{key}: direction differs from archived weights or prespecified shuffle')
    carrier_end = assay.checkpoint + config['training_repetitions'] * 10 * FRAME_US
    require(all(trial['frames'][-1]['end_us'] == carrier_end - 1 for trial in training.values()),
            'consolidation ends at the wrong clock')
    return assay, support, baseline, directions, final['balanced'], carrier_end + config['rest_us']


def verified_probes(row, assay, edges, start, baseline=None):
    require(row.keys() == set(PROBES), 'unexpected probe names')
    for cue in PROBES:
        read_probe(row[cue], cue, assay, start, edges.keys(), assay.reset[cue])
        if baseline is not None:
            require(row[cue]['root_spikes'] == baseline[cue]['root_spikes'],
                    'probe root input differs from carrier baseline')
    require(not any(row['blank'].get(field) for field in ('arrivals', 'boundaries')),
            'unexpected blank electrical trace')
    targets = set().union(*assay.targets)
    return {cue: read_cue(row[cue], targets, assay.core, edges) for cue in ORIGINAL}


def spike_comparisons(first, second):
    result = {}
    for cue in PROBES:
        left = Counter(map(tuple, first[cue]['core_spikes']))
        right = Counter(map(tuple, second[cue]['core_spikes']))
        result[cue] = dict(matching=(left & right).total(), added=(left - right).total(),
                           removed=(right - left).total(), rest_change=(
                               first[cue]['rest_core_spikes'] - second[cue]['rest_core_spikes']))
    return result


def target_margins(first, second, targets):
    """Summarize candidate-margin deltas where both targets can spike at the same boundary."""
    result = {}
    for cue in ORIGINAL:
        own, _ = first[cue]
        control, _ = second[cue]
        result[cue] = []
        for group in targets:
            left = {key for key, row in own.items() if key[1] in group and key[0] >= row[2]}
            right = {key for key, row in control.items() if key[1] in group and key[0] >= row[2]}
            changes = [own[key][3] - control[key][3] for key in sorted(left & right)]
            result[cue].append(dict(
                aligned=len(changes), arm_only=len(left - right), control_only=len(right - left),
                increased=sum(value > 0 for value in changes), unchanged=changes.count(0.),
                decreased=sum(value < 0 for value in changes),
                mean_delta=sum(changes) / len(changes) if changes else None,
                min_delta=min(changes, default=None), max_delta=max(changes, default=None),
            ))
    return result


def score(arms, baseline, targets, prefix='', controls=()):
    def spike_count(arm, cue, group):
        return sum(cell in targets[group] for cell, _ in arm[cue]['core_spikes'])

    result = {}
    for flip, key in enumerate(MAPPINGS):
        available = prefix + key in arms
        arm = arms.get(prefix + key, baseline)
        opposite = arms.get(prefix + MAPPINGS[1 ^ flip], baseline)
        blank = len(arm['blank']['core_spikes'])
        baseline_blank = len(baseline['blank']['core_spikes'])
        result[key] = {}
        for index, cue in enumerate(ORIGINAL):
            group = index ^ flip
            correct = spike_count(arm, cue, group)
            comparisons = dict(wrong=spike_count(arm, cue, 1 ^ group),
                               baseline=spike_count(baseline, cue, group),
                               opposite=spike_count(opposite, cue, group))
            comparisons.update({name: spike_count(arms[name.format(mapping=key)], cue, group)
                                for name in controls})
            rest, baseline_rest = arm[cue]['rest_core_spikes'], baseline[cue]['rest_core_spikes']
            result[key][cue] = dict(
                correct=correct, correct_group=group, **comparisons,
                rest=rest, baseline_rest=baseline_rest, blank=blank, baseline_blank=baseline_blank,
                available=available,
                passed=available and correct > max(comparisons.values()) and rest <= baseline_rest
                and blank <= baseline_blank,
            )
    return result


def score_summary(checks):
    passes = sum(row['passed'] for mapping in checks.values() for row in mapping.values())
    available = sum(row['available'] for mapping in checks.values() for row in mapping.values())
    return dict(passed_cases=passes, available_cases=available, complete=passes == 4, checks=checks)


def analyze_record(raw, source, consolidation, checksum):
    analyze_capacity(source)
    assay, support, base, directions, edges, start = reconstruct(
        raw, source, consolidation, checksum)
    baseline = raw['baseline']['probes']
    require(published(raw['baseline']['weights'], support) == base,
            'baseline weights differ from archived balanced carrier')
    baseline_electrical = verified_probes(baseline, assay, edges, start)
    missing = {key for key, vector in directions.items() if not any(vector)}
    reference_names = {f'reference_{key}' for key in MAPPINGS}
    require(raw['unavailable_directions'] == sorted(missing) and missing < reference_names,
            'unexpected unavailable directions or no available reference')
    active = {key: vector for key, vector in directions.items() if key not in missing}
    full_probes, full_electrical, full_norms = {}, {}, {}
    require(raw['reference_full'].keys() == set(MAPPINGS), 'unexpected full references')
    for mapping in MAPPINGS:
        arm = raw['reference_full'][mapping]
        direction = directions[f'reference_{mapping}']
        expected = [f32(weight + delta) for weight, delta in zip(base, direction)]
        require(published(arm['weights'], support) == expected,
                f'{mapping}: full reference weights differ from archived transplant')
        full_norms[mapping] = norm(direction)
        require(any(direction) or arm['probes'] == baseline,
                f'{mapping}: zero full reference differs from baseline')
        full_probes[mapping] = arm['probes']
        full_electrical[mapping] = verified_probes(arm['probes'], assay, edges, start, baseline)
    full = score_summary(score(full_probes, baseline, assay.targets))
    full['arms'] = {
        mapping: {
            'actual_norm': full_norms[mapping],
            'changed_coordinates': sum(delta != 0. for delta in directions[f'reference_{mapping}']),
            'spikes_vs_baseline': spike_comparisons(full_probes[mapping], baseline),
            'target_margins_vs_baseline': target_margins(
                full_electrical[mapping], baseline_electrical, assay.targets),
        } for mapping in MAPPINGS
    }

    rooms = {key: headrooms(base, vector) for key, vector in directions.items()}
    ray_limits = {key: ray_limit(rooms[key], vector) for key, vector in directions.items()}
    saturation_limits = {key: norm(room) for key, room in rooms.items()}
    doses = dict(ray=.9 * min(ray_limits[key] for key in active),
                 bounded=.9 * min(*[value for value in full_norms.values() if value],
                                  *[saturation_limits[key] for key in active]))
    require(raw['tiers'].keys() == set(TIERS), 'unexpected intervention tiers')
    tiers = {}
    for key, requested in doses.items():
        require(requested > 0., f'{key}: no positive common intervention dose')
        tier = raw['tiers'][key]
        verify_plan({'requested_norm': tier['requested_norm']}, {'requested_norm': requested})
        require(tier['arms'].keys() == active.keys(), f'{key}: unexpected tier arms')
        probes, electrical, arms = {}, {}, {}
        for name, vector in active.items():
            arm = tier['arms'][name]
            if key == 'ray':
                weights, plan = normalization(base, vector, [requested])
                require(plan['faithful'], f'{name}: ray direction or dose failed tolerance')
            else:
                weights, plan = bounded_plan(base, vector, requested)
            require(plan['relative_norm_error'] < .01, f'{key}/{name}: dose failed tolerance')
            verify_plan(arm['plan'], plan)
            require(published(arm['weights'], support) == weights,
                    f'{key}/{name}: installed weights differ from reconstructed plan')
            probes[name] = arm['probes']
            electrical[name] = verified_probes(arm['probes'], assay, edges, start, baseline)
            arms[name] = {'plan': plan,
                          'changed_coordinates': sum(a != b for a, b in zip(base, weights))}
        learned = score_summary(score(
            probes, baseline, assay.targets, controls=('balanced', 'shuffle_{mapping}')))
        reference = score_summary(score(probes, baseline, assay.targets, prefix='reference_'))
        for name, arm in arms.items():
            comparisons = {'baseline': (baseline, baseline_electrical)}
            if name in MAPPINGS:
                opposite = MAPPINGS[1 ^ MAPPINGS.index(name)]
                for control in (opposite, 'balanced', f'shuffle_{name}', f'reference_{name}'):
                    if control in probes:
                        comparisons[control] = (probes[control], electrical[control])
            if name.startswith('reference_'):
                mapping = name.removeprefix('reference_')
                comparisons['full_reference'] = (full_probes[mapping], full_electrical[mapping])
            arm['comparisons'] = {
                control: {'spikes': spike_comparisons(probes[name], control_probes),
                          'target_margins': target_margins(electrical[name], control_traces,
                                                           assay.targets)}
                for control, (control_probes, control_traces) in comparisons.items()
            }
        outcome = {
            (True, True): 'learned_and_reference_pass',
            (False, True): 'reference_passes_learned_incomplete',
            (True, False): 'learned_passes_reference_incomplete',
            (False, False): 'both_incomplete',
        }[learned['complete'], reference['complete']]
        tiers[key] = {'requested_norm': requested, 'learned': learned, 'reference': reference,
                      'outcome': outcome, 'arms': arms}
    return {
        'job_id': source['job']['id'], 'seed': raw['seed'], 'archived_record_checksum': checksum,
        'reference_choice': raw['reference_choice'], 'support_size': len(support),
        'unavailable_directions': sorted(missing),
        'target_sizes': list(map(len, assay.targets)), 'carrier_edge_count': len(edges),
        'carrier_added_edges': len(edges.keys() - assay.edge_ids), 'probe_start_us': start,
        'direction_norms': {key: norm(vector) for key, vector in directions.items()},
        'direction_nonzero_counts': {key: sum(value != 0. for value in vector)
                                     for key, vector in directions.items()},
        'ray_limits': ray_limits, 'saturation_limits': saturation_limits,
        'reference_full': full, 'tiers': tiers,
    }


def analyze_directory(directory):
    directory = Path(directory)
    raw, capacity, consolidation = [
        {path.name: path for path in (directory / part).glob('*.json') if path.name != 'manifest.json'}
        for part in ('raw', 'input/capacity', 'input/consolidation')
    ]
    require(raw and raw.keys() == capacity.keys() == consolidation.keys(),
            'grown dose replay and archived inputs differ')
    records = []
    for filename in sorted(raw):
        evidence = {name: files[filename].read_bytes() for name, files in (
            ('raw', raw), ('capacity', capacity), ('consolidation', consolidation))}
        source, checksum = source_record(evidence['capacity'])
        require(filename == f"{source['job']['id']}.json", 'file name differs from capacity job identity')
        result = analyze_record(json.loads(evidence['raw']), source,
                                json.loads(evidence['consolidation']), checksum)
        records.append({**result, **{f'{name}_sha256': hashlib.sha256(data).hexdigest()
                                     for name, data in evidence.items()}})
    return {
        'protocol': PROTOCOL, 'cohort_size': len(records),
        'full_reference_complete': sum(row['reference_full']['complete'] for row in records),
        'outcomes': {key: dict(Counter(row['tiers'][key]['outcome'] for row in records))
                     for key in TIERS},
        'records': sorted(records, key=lambda row: (row['seed'], row['job_id'])),
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path)
    args = parser.parse_args()
    try:
        print(json.dumps(analyze_directory(args.directory), indent=2, allow_nan=False))
    except (OSError, ValueError, KeyError, TypeError, IndexError, OverflowError) as exc:
        parser.exit(1, f'contingency-grown-dose: {exc}\n')


if __name__ == '__main__':
    main()
