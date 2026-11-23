#!/usr/bin/env python3
"""Verify fixed-history chemistry contrasts and analyze electrical and spike responses."""

import argparse
from collections import Counter
import json
import math
from pathlib import Path

from contingency_electrical_analysis import ARMS, MAPPINGS, analyze_record as analyze_electrical, f32
from verify_contingency import FRAME_US, ORIGINAL, TRAINING, event_counts, require, weight_map

PROTOCOL = 'NERVA-Contingency-Chemical/1'
SCOPES = ('native', 'constant', 'matched')


def norm(vector):
    return math.sqrt(sum(value * value for value in vector))


def finite(value):
    return type(value) in (int, float) and math.isfinite(value)


def validate_tape(tape, edges, initial):
    require(tape['verified_original_local_state'] is True
            and tape['verified_checkpoint_rule_and_local_clocks'] is True,
            'tape requires verified native local state and checkpoint clocks')
    clocks = tape['clock_times_us']
    require(clocks and all(type(time) is int and time >= 0 for time in clocks)
            and sorted(clocks) == clocks, 'invalid tape clocks')
    clock_set = set(clocks)
    for name in ('pre_arrivals', 'post_spikes', 'chemical_samples'):
        last = -1
        for time, identity, *values in tape[name]:
            require(type(time) is int and time in clock_set and time >= last
                    and type(identity) is int and identity >= 0, f'invalid tape {name}')
            require(len(values) == (7 if name == 'chemical_samples' else 0)
                    and all(map(finite, values)), f'invalid tape {name} fields')
            if name == 'pre_arrivals':
                require(identity in edges, 'unknown tape arrival edge')
            last = time

    end = {}
    for variant in ('original', 'constant_positive'):
        synapses = tape[variant]['synapses']
        end[variant] = weight_map([(row['synapse_id'], row['weight']) for row in synapses],
                                  edges.keys())
        for row in synapses:
            edge = row['synapse_id']
            require(type(edge) is int and (row['source_id'], row['target_id']) == edges[edge]
                    and f32(row['initial_weight']) == initial[edge], 'tape edge identities differ')
            positive, negative = row['positive_actual'], row['negative_actual']
            require(finite(positive) and finite(negative) and positive >= 0 >= negative
                    and math.isclose(positive + negative, f32(row['weight']) - initial[edge],
                                     rel_tol=1e-9, abs_tol=1e-12),
                    'tape realized weight changes do not reproduce final weight')
    return [{edge: f32(weight) for edge, weight in end[key].items()}
            for key in ('original', 'constant_positive')]


def normalization(initial, direction, native):
    """Recalculate the f32 publication along the constant-positive update direction."""
    length, requested = norm(direction), norm(native)
    maximum = min(((2 - weight if delta > 0 else -weight) / (delta / length)
                   for weight, delta in zip(initial, direction) if delta), default=None)
    details = {'requested_norm': requested, 'ray_limit': maximum, 'feasible': False,
               'faithful': False}
    if maximum is None or requested > maximum + 1e-12 * max(maximum, 1.):
        details['reason'] = 'undefined_direction_or_norm' if maximum is None else 'weight_bounds'
        return None, details

    def raw_at(scale):
        return [weight + (delta if scale == length else scale * (delta / length))
                for weight, delta in zip(initial, direction)]

    def publish(scale):
        raw = raw_at(scale)
        require(all(-1e-12 <= weight <= 2. + 1e-12 for weight in raw),
                'normalization ray leaves weight bounds')
        return [f32(min(2., max(0., weight))) for weight in raw]

    def deltas(weights):
        return [weight - base for weight, base in zip(weights, initial)]

    scale, iterations = requested, 0
    weights = publish(scale)
    distance = abs(norm(deltas(weights)) - requested)
    if requested > 0 and distance / requested > 1e-4:
        low, high = 0., maximum
        for _ in range(48):
            middle = (low + high) / 2.
            candidate = publish(middle)
            actual = norm(deltas(candidate))
            error = abs(actual - requested)
            if error < distance:
                scale, weights, distance = middle, candidate, error
            if actual < requested:
                low = middle
            else:
                high = middle
            iterations += 1

    delta = deltas(weights)
    actual = norm(delta)
    cosine = (min(1., max(-1., sum(a * b for a, b in zip(delta, direction)) / (actual * length)))
              if actual > 0 else None)
    relative = abs(actual - requested) / requested if requested > 0 else 0.
    details.update(feasible=True, reason=None, ray_norm=scale, normalization_iterations=iterations,
                   actual_norm=actual, relative_norm_error=relative, direction_cosine=cosine,
                   rounding_l2=norm([weight - raw for weight, raw in zip(weights, raw_at(scale))]),
                   faithful=relative <= .01 and cosine is not None and cosine >= .999)
    return weights, details


def verify_plan(reported, expected):
    require(reported.keys() == expected.keys(), 'normalization evidence fields differ')
    for key, value in expected.items():
        observed = reported[key]
        if type(value) is float:
            equal = finite(observed) and math.isclose(observed, value, rel_tol=1e-9, abs_tol=1e-13)
        else:
            equal = type(observed) is type(value) and observed == value
        require(equal, f'normalization evidence disagrees for {key}')


def published_weights(probe, arm, initial):
    return {edge: f32(weight) for edge, weight in
            weight_map(probe['arms'][arm]['weights'], initial.keys()).items()}


def verify_weights(record):
    probes = record['probes']
    native = probes['native']
    edges = {edge: (source, target) for edge, source, target, _ in native['edges']}
    initial = weight_map([(edge, weight) for edge, _, _, weight in native['edges']], edges.keys())
    initial = {edge: f32(weight) for edge, weight in initial.items()}
    core = set(native['core_ids'])
    core_edges = [edge for edge, (pre, post) in edges.items() if pre in core and post in core]
    require(record['training'].keys() == record['norm'].keys() == set(TRAINING),
            'unexpected training or normalization arms')
    matched, plans = {}, {}
    for mapping in TRAINING:
        arm = f'{mapping}_core'
        original, constant = validate_tape(record['training'][mapping], edges, initial)
        for scope, tape_weights in (('native', original), ('constant', constant)):
            expected = {**initial, **{edge: tape_weights[edge] for edge in core_edges}}
            require(published_weights(probes[scope], arm, initial) == expected,
                    f'{scope}/{arm}: installed Core-only weights differ from tape')
        base = [initial[edge] for edge in core_edges]
        direction = [constant[edge] - initial[edge] for edge in core_edges]
        native_delta = [original[edge] - initial[edge] for edge in core_edges]
        weights, plan = normalization(base, direction, native_delta)
        verify_plan(record['norm'][mapping], plan)
        plans[mapping] = {**plan, 'constant_norm': norm(direction)}
        if plan['faithful']:
            matched[arm] = {**initial, **dict(zip(core_edges, weights))}
    require((probes['matched'] is not None) == (len(matched) == len(TRAINING)),
            'matched probe availability disagrees with normalization')
    if probes['matched'] is not None:
        for arm, weights in matched.items():
            require(published_weights(probes['matched'], arm, initial) == weights,
                    f'matched/{arm}: installed weights differ from the normalized ray')
    return plans


def analyze_spikes(probe, blank):
    templates = [set(group) for group in probe['templates']]
    require(blank.keys() == set(ARMS) and all(type(n) is int and n >= 0 for n in blank.values()),
            'invalid blank Core spike counts')
    quiet_blank = all(blank[arm] <= blank['reset'] for arm in MAPPINGS)
    cues = {}
    for index, cue in enumerate(ORIGINAL):
        counts, rest = {}, {}
        for arm in ARMS:
            row = probe['arms'][arm]['cues'][cue]
            start = row['start_us']
            spikes = event_counts(row['core_spikes'], start, start + 4 * FRAME_US,
                                   ids=set(probe['core_ids']), label='chemical cue Core')
            counts[arm] = [sum(spikes[cell] for cell in group) for group in templates]
            rest[arm] = row['rest_core_spikes']
        mapping_passes = {}
        for reversal, arm in enumerate(MAPPINGS):
            target = index ^ reversal
            opposite = MAPPINGS[1 - reversal]
            mapping_passes[arm] = (counts[arm][target] > max(
                counts[arm][1 - target], *(counts[control][target]
                                           for control in ('reset', opposite, 'balanced_core')))
                and rest[arm] <= rest['reset'])
        cues[cue] = {'counts': counts, 'rest_core_spikes': rest, 'mapping_passes': mapping_passes}
    return {'passes': all(templates) and quiet_blank
            and all(all(cue['mapping_passes'].values()) for cue in cues.values()),
            'blank_core_spikes': blank, 'quiet_blank': quiet_blank, 'cues': cues}


def compare_scope(native, probe, native_result, result):
    comparisons = {}
    for cue in ORIGINAL:
        comparisons[cue] = {}
        for arm in ARMS[1:]:
            baseline = native['arms'][arm]['cues'][cue]
            response = probe['arms'][arm]['cues'][cue]
            old = native_result['electrical']['cues'][cue]['arms'][arm]['target_groups']
            new = result['electrical']['cues'][cue]['arms'][arm]['target_groups']
            native_counts = native_result['spikes']['cues'][cue]['counts'][arm]
            counts = result['spikes']['cues'][cue]['counts'][arm]
            comparisons[cue][arm] = {
                'core_spike_history_changed': response['core_spikes'] != baseline['core_spikes'],
                'target_spike_count_delta': [b - a for a, b in zip(native_counts, counts)],
                'mean_peak_margin_delta_vs_native': [
                    b - a if a is not None and b is not None else None
                    for a, b in zip((group['mean_peak_margin_delta'] for group in old),
                                    (group['mean_peak_margin_delta'] for group in new))
                ],
            }
    return comparisons


def analyze_record(record):
    require(record['protocol'] == PROTOCOL and record['replay_verified'] is True,
            'replay-verified chemical record required')
    probes, blank = record['probes'], record['blank_core_spikes']
    require(probes.keys() == blank.keys() == set(SCOPES), 'invalid chemistry probe scopes')
    native = probes['native']
    report = {}
    for scope in SCOPES:
        probe = probes[scope]
        require((probe is None) == (blank[scope] is None), 'blank and cue observations differ in scope')
        if probe is None:
            require(scope == 'matched', f'{scope} probe is required')
            report[scope] = None
            continue
        for field in ('seed', 'gap_frames', 'archived_record_checksum'):
            require(probe[field] == record[field], f'{scope}: probe {field} differs from record')
        for field in ('edges', 'core_ids', 'templates'):
            require(probe[field] == native[field], f'{scope}: probe {field} differs from native')
        require(probe['arms']['reset'] == native['arms']['reset']
                and blank[scope]['reset'] == blank['native']['reset'],
                f'{scope}: reset observation differs from native')
        report[scope] = {'electrical': analyze_electrical(probe),
                         'spikes': analyze_spikes(probe, blank[scope])}
        if scope != 'native':
            report[scope]['vs_native'] = compare_scope(native, probe, report['native'], report[scope])
    return {'seed': record['seed'], 'gap_frames': record['gap_frames'],
            'archived_record_checksum': record['archived_record_checksum'],
            'norm': verify_weights(record), 'scopes': report}


def analyze_directory(directory):
    paths = sorted((Path(directory) / 'raw').glob('*.json'))
    require(paths, 'no chemical records in raw/')
    results = [analyze_record(json.loads(path.read_text(encoding='utf-8'))) for path in paths]
    require(len({(row['seed'], row['gap_frames']) for row in results}) == len(results),
            'duplicate seed and gap')
    by_gap = {}
    for gap in sorted({row['gap_frames'] for row in results}):
        by_gap[gap] = {}
        for scope in SCOPES:
            scopes = [row['scopes'][scope] for row in results if row['gap_frames'] == gap]
            totals = {'unavailable': scopes.count(None)}
            for gate in ('electrical', 'spikes'):
                counts = Counter(row[gate]['passes'] for row in scopes if row is not None)
                totals[gate] = {'passes': counts[True], 'fails': counts[False],
                                'not_assessable': counts[None]}
            by_gap[gap][scope] = totals
    return {'protocol': PROTOCOL, 'by_gap_frames': by_gap, 'records': results}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path)
    args = parser.parse_args()
    try:
        print(json.dumps(analyze_directory(args.directory), indent=2, allow_nan=False))
    except (OSError, ValueError, KeyError, TypeError, IndexError, OverflowError) as exc:
        parser.exit(1, f'contingency-chemical: {exc}\n')


if __name__ == '__main__':
    main()
