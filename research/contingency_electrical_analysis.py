#!/usr/bin/env python3
"""Independently analyze electrical observations in native visual contingency records."""

import argparse
from collections import Counter, defaultdict
import json
import math
from pathlib import Path
import struct

from verify_contingency import FRAME_US, ORIGINAL, event_counts, require, weight_map


def f32(value):
    return struct.unpack('<f', struct.pack('<f', value))[0]


def canonical_sum(amplitudes):
    """Match the native CPU summation order and final f32 rounding."""
    total = 0.0
    for amplitude in sorted(map(f32, amplitudes), key=lambda value: (-abs(value), value)):
        total += amplitude
    return f32(total)


PROTOCOL = 'NERVA-Contingency-Electrical/1'
MAPPINGS = ('forward_core', 'reverse_core')
ARMS = ('reset', *MAPPINGS, 'balanced_core')


def number(value):
    require(type(value) in (int, float) and math.isfinite(value), 'invalid electrical value')
    return f32(value)


def read_cue(row, targets, core_ids, edges):
    start = row['start_us']
    stop = start + 4 * FRAME_US
    require(type(start) is int and start >= 0, 'invalid cue start')
    event_counts(row['root_spikes'], start, start + FRAME_US + 1, label='cue root')
    event_counts(row['core_spikes'], start, stop, ids=core_ids, label='cue Core')
    require(type(row['rest_core_spikes']) is int and row['rest_core_spikes'] >= 0,
            'invalid rest spike count')
    reported_edges = set(row['arrived_edges'])
    require(reported_edges <= edges.keys(), 'unknown arrived edge')
    arrivals, inputs = defaultdict(list), defaultdict(list)
    for time, target, edge, amplitude in row['arrivals']:
        require(type(time) is int and start <= time < stop and target in targets
                and edge in edges and edges[edge][1] == target, 'invalid target arrival')
        amplitude = number(amplitude)
        arrivals[time, target, edge].append(amplitude)
        inputs[time, target].append(amplitude)
    require({edge for (_, _, edge), values in arrivals.items() if any(values)}
            == {edge for edge in reported_edges if edges[edge][1] in targets},
            'target arrival edge IDs disagree with cue response')
    boundaries = {}
    spike_keys = {(time, cell) for cell, time in row['core_spikes'] if cell in targets}
    for time, target, voltage, summed, threshold, after, refractory_until, spiked in row['boundaries']:
        key = (time, target)
        require(type(time) is int and start <= time < stop and target in targets
                and key not in boundaries, 'invalid or repeated target integration boundary')
        require(type(refractory_until) is int and refractory_until >= 0
                and type(spiked) is bool, 'invalid target integration state')
        voltage, summed, threshold, after = map(number, (voltage, summed, threshold, after))
        refractory = time < refractory_until
        candidate = f32(voltage + summed)
        require(summed == canonical_sum(inputs.get(key, ())), 'target summed input differs from arrivals')
        require(spiked == (not refractory and candidate >= threshold)
                and spiked == (key in spike_keys), 'target candidate disagrees with recorded spike')
        if not spiked:
            require(after == (voltage if refractory else candidate),
                    'target candidate disagrees with recorded potential')
        boundaries[key] = (summed, threshold, refractory_until, candidate - threshold)
    require(inputs.keys() <= boundaries.keys() and spike_keys <= boundaries.keys(),
            'arrival or target spike missing integration boundary')
    return boundaries, {key: sorted(values) for key, values in arrivals.items()}


def select_peaks(boundaries):
    selected = {}
    for key, (_, _, refractory_until, candidate_margin) in sorted(boundaries.items()):
        time, target = key
        if time >= refractory_until and (target not in selected
                                        or candidate_margin > boundaries[selected[target]][3]):
            selected[target] = key
    return selected


def compare_cue(reset, arm, templates, selected, mismatches):
    base, base_arrivals = reset
    probe, arrivals = arm
    arrival_deltas = [
        (key[1], amplitude - original)
        for key in base_arrivals.keys() & arrivals.keys()
        if len(base_arrivals[key]) == len(arrivals[key])
        for original, amplitude in zip(base_arrivals[key], arrivals[key])
    ]
    aligned = sorted(base.keys() & probe.keys())
    comparable = [key for key in aligned if base[key][2] <= key[0] and probe[key][2] <= key[0]]
    margin_deltas = {key: probe[key][3] - base[key][3] for key in comparable}
    positive_peak = max((key for key in comparable if margin_deltas[key] > 0),
                        key=margin_deltas.__getitem__, default=None)
    deficit_fraction = None
    if positive_peak is not None and base[positive_peak][3] < 0:
        deficit_fraction = margin_deltas[positive_peak] / -base[positive_peak][3]
    if base.keys() != probe.keys():
        mismatches.append('boundary_keys')
    if any(base[key][2] != probe[key][2] for key in aligned):
        mismatches.append('refractory_until')
    if any(base[key][1] != probe[key][1] for key in aligned):
        mismatches.append('thresholds')

    groups = []
    for template in templates:
        keys = [selected[cell] for cell in sorted(template) if cell in selected]
        deltas = [margin_deltas[key] for key in keys if key in margin_deltas]
        groups.append({
            'template_cells': len(template),
            'reset_peak_cells': len(keys),
            'paired_peak_cells': len(deltas),
            'mean_peak_margin_delta': math.fsum(deltas) / len(deltas)
            if deltas and not mismatches else None,
            'positive_peak_margin_deltas': sum(delta > 0 for delta in deltas),
            'negative_peak_margin_deltas': sum(delta < 0 for delta in deltas),
            'changed_arrival_amplitudes': sum(delta != 0 for target, delta in arrival_deltas
                                              if target in template),
            'nonzero_arrival_target_cells': len({
                target for (_, target, _), amplitudes in arrivals.items()
                if target in template and any(amplitudes)
            }),
        })
    return {
        'mismatches': mismatches,
        'target_groups': groups,
        'paired_arrivals': len(arrival_deltas),
        'changed_arrival_amplitudes': sum(delta != 0 for _, delta in arrival_deltas),
        'max_abs_arrival_delta': max((abs(delta) for _, delta in arrival_deltas), default=None),
        'paired_boundaries': len(aligned),
        'max_abs_summed_input_delta': max((abs(probe[key][0] - base[key][0]) for key in aligned),
                                          default=None),
        'paired_nonrefractory_boundaries': len(comparable),
        'max_abs_margin_delta': max(map(abs, margin_deltas.values()), default=None),
        'largest_positive_margin_delta': margin_deltas.get(positive_peak),
        'largest_positive_margin_delta_fraction_of_reset_deficit': deficit_fraction,
        'positive_margin_deltas': sum(delta > 0 for delta in margin_deltas.values()),
        'negative_margin_deltas': sum(delta < 0 for delta in margin_deltas.values()),
    }


def analyze_record(record):
    require(record['protocol'] == PROTOCOL and record['replay_verified'] is True,
            'replay-verified electrical record required')
    seed, gap = record['seed'], record['gap_frames']
    require(type(seed) is int and type(gap) is int and gap in (0, 2), 'invalid seed or gap')
    core_ids = set(record['core_ids'])
    templates = [set(group) for group in record['templates']]
    require(len(templates) == 2 and not templates[0] & templates[1]
            and all(group <= core_ids for group in templates), 'invalid target groups')
    targets = templates[0] | templates[1]
    edges = {edge: (source, target) for edge, source, target, _ in record['edges']}
    initial = weight_map([(edge, weight) for edge, _, _, weight in record['edges']], edges.keys())
    arms = record['arms']
    require(arms.keys() == set(ARMS), 'invalid probe arms')
    for name, arm in arms.items():
        weights = weight_map(arm['weights'], edges.keys())
        require(all(weights[edge] == weight for edge, weight in initial.items()
                    if name == 'reset' or not all(cell in core_ids for cell in edges[edge])),
                'weights changed outside the probe scope')
        require(arm['cues'].keys() == set(ORIGINAL), 'invalid cue names')

    cues = {}
    for cue_index, cue in enumerate(ORIGINAL):
        raw = {name: arms[name]['cues'][cue] for name in ARMS}
        observed = {name: read_cue(row, targets, core_ids, edges) for name, row in raw.items()}
        reset = observed['reset']
        selected = select_peaks(reset[0])
        comparisons = {}
        for name in ARMS[1:]:
            mismatches = [key for key in ('start_us', 'root_spikes', 'core_spikes', 'rest_core_spikes')
                          if raw[name][key] != raw['reset'][key]]
            comparisons[name] = compare_cue(reset, observed[name], templates, selected, mismatches)
        scores = {name: [group['mean_peak_margin_delta'] for group in row['target_groups']]
                  for name, row in comparisons.items()}
        assessable = all(delta is not None for group in scores.values() for delta in group)
        mapping_passes = {name: None for name in MAPPINGS}
        if assessable:
            for index, name in enumerate(MAPPINGS):
                target = cue_index ^ index
                own, wrong = scores[name][target], scores[name][1 - target]
                mapping_passes[name] = own > max(
                    0., wrong, scores[MAPPINGS[1 - index]][target], scores['balanced_core'][target])
        cues[cue] = {
            'assessable': assessable,
            'reset_peaks': [[target, selected[target][0], reset[0][selected[target]][3]]
                            for target in sorted(selected)],
            'mapping_passes': mapping_passes,
            'arms': comparisons,
        }

    assessable = all(cue['assessable'] for cue in cues.values())
    return {
        'seed': seed, 'gap_frames': gap, 'archived_record_checksum': record['archived_record_checksum'],
        'templates': [sorted(group) for group in templates],
        'assessable': assessable,
        'passes': all(all(cue['mapping_passes'].values()) for cue in cues.values())
        if assessable else None,
        'cues': cues,
    }


def analyze_directory(directory):
    paths = sorted((Path(directory) / 'raw').glob('*.json'))
    require(paths, 'no electrical records in raw/')
    results = [analyze_record(json.loads(path.read_text(encoding='utf-8'))) for path in paths]
    require(len({(row['seed'], row['gap_frames']) for row in results}) == len(results),
            'duplicate seed and gap')
    by_gap = {}
    for gap in sorted({row['gap_frames'] for row in results}):
        counts = Counter(row['passes'] for row in results if row['gap_frames'] == gap)
        by_gap[gap] = {'passes': counts[True], 'fails': counts[False], 'not_assessable': counts[None]}
    return {'protocol': PROTOCOL, 'by_gap_frames': by_gap, 'records': results}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path)
    args = parser.parse_args()
    try:
        print(json.dumps(analyze_directory(args.directory), indent=2, allow_nan=False))
    except (OSError, ValueError, KeyError, TypeError, OverflowError) as exc:
        parser.exit(1, f'contingency-electrical: {exc}\n')


if __name__ == '__main__':
    main()
