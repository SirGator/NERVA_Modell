#!/usr/bin/env python3
"""Independently verify inherited-eligibility contrasts on capacity terminal edges."""

import argparse
from collections import Counter, defaultdict
import hashlib
import json
import math
from pathlib import Path
import re

from contingency_capacity_analysis import Assay, MAPPINGS, analyze_record as analyze_capacity, natural
from contingency_electrical_analysis import f32
from verify_contingency import FRAME_US, ORIGINAL, require, weight_map

PROTOCOL = 'NERVA-Contingency-Eligibility/1'
ARMS = ('original', 'clear_eligibility', 'clear_pairing', 'clear_both')
CHEMICAL_FIELDS = ['time_us', 'neuron_id', 'dopamine', 'acetylcholine', 'noradrenaline',
                   'native_signal', 'threshold_shift', 'intrinsic_drive', 'transmission_gain']
CONTRASTS = {
    'inherited_eligibility': ('clear_pairing', 'clear_both'),
    'inherited_pairing': ('clear_eligibility', 'clear_both'),
    'eligibility_with_pairing': ('original', 'clear_eligibility'),
    'pairing_with_eligibility': ('original', 'clear_pairing'),
}


def source_record(raw):
    # Keep native number spellings while removing whitespace outside strings.
    compact = re.sub(rb'("(?:[^"\\]|\\.)*")|\s+', rb'\1', raw)
    match = re.fullmatch(rb'\{"checksum_fnv1a64":"([a-f0-9]{16})","record":(\{.*\})\}', compact)
    require(match is not None, 'expected a native capacity envelope')
    checksum, body = match.groups()
    value = 0xcbf29ce484222325
    for byte in body:
        value = ((value ^ byte) * 0x100000001b3) & 0xffffffffffffffff
    require(f'{value:016x}'.encode() == checksum, 'capacity envelope checksum differs')
    return json.loads(body), checksum.decode()


def number(value):
    require(type(value) in (int, float) and math.isfinite(value), 'invalid numeric ledger field')
    return value


def close(actual, expected):
    return math.isclose(actual, expected, rel_tol=1e-9, abs_tol=1e-18)


def integrate_unpaired_tag(edge, clocks, factors, tau):
    """Integrate the decaying f32 checkpoint tag under held local chemistry."""
    time = edge['initial_tag_anchor']
    tag, factor = f32(edge['initial_tag']), f32(edge['initial_factor'])
    totals = dict.fromkeys(('positive_requested', 'negative_requested',
                            'positive_tag_integral', 'negative_tag_integral'), 0.)
    for boundary in clocks:
        fraction = -(boundary - time) / tau if time is not None else 0.
        integral = tag * tau * 1e-6 * -math.expm1(fraction)
        requested = integral * factor
        totals['positive_requested'] += max(0., requested)
        totals['negative_requested'] += min(0., requested)
        totals['positive_tag_integral'] += max(0., integral)
        totals['negative_tag_integral'] += min(0., integral)
        tag = f32(tag * math.exp(fraction))
        time = boundary
        factor = factors.get(boundary, factor)
    return {**totals, 'eligibility': tag}


def signed_ledger(row):
    totals = {}
    for field in ('requested', 'actual', 'tag_integral', 'blocked'):
        positive = number(row[f'positive_{field}'])
        negative = number(row[f'negative_{field}'])
        require(positive >= 0 >= negative, f'invalid signs for {field} in edge ledger')
        totals[f'positive_{field}'] = positive
        totals[f'negative_{field}'] = negative
        totals[f'net_{field}'] = positive + negative
        totals[f'absolute_{field}'] = positive - negative
    return totals


def compare_focus(mapping, native_frames, focus_ids, assay):
    focus = mapping['focus']
    require([edge['id'] for edge in focus] == sorted(focus_ids), 'terminal focus differs from ablations')
    topology = {edge: (pre, post, eligible) for edge, pre, post, _, eligible in assay.topology}
    for edge in focus:
        pre, post, eligible = topology[edge['id']]
        anchor = edge['initial_tag_anchor']
        require(eligible and (edge['pre'], edge['post']) == (pre, post)
                and natural(edge['delay_us']) and edge['delay_us'] > 0
                and (anchor is None or natural(anchor) and anchor <= assay.checkpoint),
                'invalid focus topology, delay or tag anchor')
        for field in ('initial_factor', 'initial_tag', 'initial_residual'):
            number(edge[field])
    history = mapping['fixed_history']
    require(history['verified_original_local_state'] is True
            and history['verified_checkpoint_rule_and_local_clocks'] is True,
            'native fixed history lacks full local replay checks')
    clocks = history['clock_times_us']
    stop = assay.checkpoint + len(native_frames) * FRAME_US
    require(all(natural(time) and assay.checkpoint <= time <= stop for time in clocks)
            and clocks == sorted(clocks), 'invalid fixed-history clock sequence')
    clock_set = set(clocks)
    require(history['chemical_fields'] == CHEMICAL_FIELDS, 'unexpected chemical fields')
    posts = {edge['post'] for edge in focus}
    factors = defaultdict(dict)
    for time, cell, *values in history['chemical_samples']:
        require(time in clock_set and cell in posts and len(values) == len(CHEMICAL_FIELDS) - 2,
                'invalid focused chemical sample')
        for value in values:
            number(value)
        factors[cell][time] = f32(values[3])
    spikes = Counter((time, cell) for frame in native_frames for cell, time in frame['core_spikes'])
    require(all(time in clock_set for time, _ in spikes), 'native Core spike lacks a history clock')
    spike_counts = Counter(cell for frame in native_frames for cell, _ in frame['core_spikes'])
    require(Counter(map(tuple, history['post_spikes'])) ==
            Counter({key: count for key, count in spikes.items() if key[1] in posts})
            and all(time in clock_set for time, _ in history['post_spikes']),
            'fixed-history post spikes differ from native frames')
    by_id = {edge['id']: edge for edge in focus}
    arrivals = defaultdict(Counter)
    task_arrivals = Counter()
    for time, identity in history['pre_arrivals']:
        require(natural(time) and time in clock_set and identity in by_id,
                'invalid focused pre-arrival')
        edge = by_id[identity]
        emitted = time - edge['delay_us']
        require(emitted >= 0, 'focused arrival precedes its axonal delay')
        if emitted < assay.checkpoint:
            arrivals[identity]['development'] += 1
        else:
            key = (identity, emitted)
            task_arrivals[key] += 1
            require(task_arrivals[key] <= spikes[emitted, edge['pre']],
                    'task arrival lacks a matching native source spike')
            arrivals[identity]['task'] += 1

    ledgers = {}
    for arm in ARMS:
        rows = history[arm]['synapses']
        require([row['synapse_id'] for row in rows] == sorted(focus_ids),
                f'{arm}: ledger edges differ from terminal focus')
        ledgers[arm] = {row['synapse_id']: row for row in rows}
    tau = f32(number(mapping['eligibility_tau_us']))
    require(tau > 0, 'invalid eligibility time constant')
    full_weights = {arm: validated_weights(mapping['fixed_weights'][arm], assay) for arm in ARMS}
    edges = []
    for identity, edge in by_id.items():
        arms = {}
        for arm, rows in ledgers.items():
            row = rows[identity]
            require((row['source_id'], row['target_id']) == (edge['pre'], edge['post'])
                    and all(row[key] is True for key in ('enabled', 'plastic', 'pre_excitatory'))
                    and row['initial_weight'] == assay.initial[identity]
                    and row['weight'] == full_weights[arm][identity]
                    and natural(row['weight_changes']), f'{arm}: inconsistent focus ledger')
            totals = signed_ledger(row)
            change = row['weight'] - row['initial_weight']
            require(math.isclose(change, totals['net_actual'], rel_tol=0., abs_tol=1e-14),
                    f'{arm}: realized ledger deltas disagree with terminal weight')
            arms[arm] = {**totals, 'weight_change': change, 'weight_changes': row['weight_changes'],
                         'final_tag': number(row['eligibility']), 'final_residual': number(row['residual'])}
        unpaired = None
        if not arrivals[identity].total():
            unpaired = integrate_unpaired_tag(edge, clocks, factors[edge['post']], tau)
            require(all(close(ledgers['clear_pairing'][identity][key], expected)
                        for key, expected in unpaired.items()),
                    'unpaired inherited tag integral disagrees with clear_pairing ledger')
            require(all(ledgers['clear_both'][identity][key] == 0. for key in unpaired),
                    'clear_both accrued a tag without presynaptic arrivals')
        contrasts = {
            label: {field: arms[left][field] - arms[right][field]
                    for field in ('net_requested', 'absolute_requested', 'weight_change')}
            for label, (left, right) in CONTRASTS.items()
        }
        contrasts['interaction'] = {
            field: contrasts['eligibility_with_pairing'][field] - contrasts['inherited_eligibility'][field]
            for field in contrasts['inherited_eligibility']
        }
        edges.append({**edge, 'source_spikes': spike_counts[edge['pre']],
                      'target_spikes': spike_counts[edge['post']],
                      'arrivals': dict.fromkeys(('development', 'task'), 0) | dict(arrivals[identity]),
                      'unpaired_integral': unpaired, 'arms': arms, 'contrasts': contrasts})
    return edges, full_weights


def validated_weights(rows, assay):
    require([identity for identity, _ in rows] == list(assay.initial),
            'weight vector order differs from checkpoint topology')
    return weight_map(rows, assay.edge_ids)


def transfer(weights, assay):
    return {edge: weights[edge] if edge in assay.recurrent else initial
            for edge, initial in assay.initial.items()}


def analyze_record(diagnostic, source, checksum):
    checked = analyze_capacity(source)
    details, job = source['details'], source['job']
    followup = details['followup']
    require(followup is not None and diagnostic['protocol'] == PROTOCOL
            and diagnostic['archived_record_checksum'] == checksum
            and diagnostic['seed'] == job['seed'] and diagnostic['replay_verified'] is True,
            'eligibility replay does not identify the verified capacity followup')
    assay = Assay(details, job['effective_config'])
    require(diagnostic['topology'] == assay.topology and diagnostic['core_ids'] == details['core_ids']
            and diagnostic['target_groups'] == details['target_groups']
            and diagnostic['checkpoint_time_us'] == assay.checkpoint
            and validated_weights(diagnostic['initial_weights'], assay) == assay.initial,
            'eligibility checkpoint differs from archived capacity checkpoint')
    mappings, fixed, live = {}, {}, {}
    require(diagnostic['mappings'].keys() == set(MAPPINGS), 'unexpected mapping interventions')
    live_training = {name: row['live'] for name, row in diagnostic['mappings'].items()}
    assay.training(live_training, job['seed'], job['effective_config']['training_repetitions'])
    for name in MAPPINGS:
        mapping = diagnostic['mappings'][name]
        focus = {edge for row in followup['terminal_ablation'] if row['mapping'] == name
                 for edge in row['restored_edges']}
        native = followup['training'][name]
        require(mapping['fixed_weights'].keys() == set(ARMS), 'unexpected fixed replay interventions')
        edges, weights = compare_focus(mapping, native['frames'], focus, assay)
        require(mapping['fixed_weights']['original'] == native['weights'],
                'fixed-history original weights differ from archived native training')
        live_arm = mapping['live']
        live_weights = validated_weights(live_arm['weights'], assay)
        same_history = live_arm['same_learning_history']
        same_weights = live_arm['same_fixed_weights']
        removed = live_arm['removed_tags']
        require(type(same_history) is bool and type(same_weights) is bool
                and same_weights == (live_weights == weights['clear_eligibility'])
                and (not same_history or same_weights)
                and natural(removed) and sum(edge['initial_tag'] != 0 for edge in edges) <= removed
                <= len(assay.edge_ids), 'inconsistent live eligibility reset or match flags')
        if same_history:
            require(all(live_frame['core_spikes'] == native_frame['core_spikes']
                        and live_frame['root_spikes'] == native_frame['root_spikes']
                        for live_frame, native_frame in zip(live_arm['frames'], native['frames'])),
                    'identical live history has different recorded spikes')
        fixed[name] = transfer(weights['clear_eligibility'], assay)
        live[name] = transfer(live_weights, assay)
        mappings[name] = {'removed_tags': removed, 'same_learning_history': same_history,
                          'same_fixed_weights': same_weights, 'focus_edges': edges,
                          'live_focus_weight_changes': {
                              edge['id']: live_weights[edge['id']] - assay.initial[edge['id']]
                              for edge in edges},
                          'live_offline_core_weight_differences': sum(
                              live[name][edge] != fixed[name][edge] for edge in assay.recurrent)}
    require(len({mapping['removed_tags'] for mapping in mappings.values()}) == 1,
            'live mappings removed different numbers of checkpoint tags')
    probes = diagnostic['probes']
    reset = {cue: assay.reset[cue] for cue in (*ORIGINAL, 'blank')}
    require(probes['reset'] == reset and probes['native'] == followup['native'],
            'reset or native recall differs from archived capacity observations')
    shared = live == fixed
    require(probes['identical_live_and_offline_core_weights'] is shared
            and (not shared or probes['live_clear'] == probes['offline_clear']),
            'identical Core transfers disagree with recall equality')
    recall = {'native': {'passed_cases': checked['followup']['native_passed_cases'],
                         'checks': checked['followup']['native_checks']}}
    for label, weights in (('live_clear', live), ('offline_clear', fixed)):
        passed, _, checks = assay.comparison(probes[label], ORIGINAL, reset, weights)
        for name in MAPPINGS:
            require(dict(probes[label]['arms'][name]['weights']) == weights[name],
                    'recall did not install the exact Core-only weights')
        recall[label] = {'passed_cases': passed, 'checks': checks}
    return {'job_id': job['id'], 'seed': job['seed'], 'archived_record_checksum': checksum,
            'identical_live_and_offline_core_weights': shared, 'recall': recall, 'mappings': mappings}


def analyze_directory(directory):
    directory = Path(directory)
    raw = {path.name: path for path in (directory / 'raw').glob('*.json')}
    sources = {path.name: path for path in (directory / 'input').glob('*.json')}
    require(raw and raw.keys() == sources.keys(), 'native replay and capacity input files differ')
    results = []
    for filename in sorted(raw):
        source_bytes, replay_bytes = sources[filename].read_bytes(), raw[filename].read_bytes()
        source, checksum = source_record(source_bytes)
        require(filename == f"{source['job']['id']}.json", 'file name differs from capacity job identity')
        result = analyze_record(json.loads(replay_bytes), source, checksum)
        results.append({**result, 'source_sha256': hashlib.sha256(source_bytes).hexdigest(),
                        'diagnostic_sha256': hashlib.sha256(replay_bytes).hexdigest()})
    edges = [edge for row in results for mapping in row['mappings'].values()
             for edge in mapping['focus_edges']]
    return {'protocol': PROTOCOL, 'cohort_size': len(results), 'focus_edges': len(edges),
            'silent_source_focus_edges': sum(edge['source_spikes'] == 0 for edge in edges),
            'zero_arrival_focus_edges': sum(not sum(edge['arrivals'].values()) for edge in edges),
            'records': sorted(results, key=lambda row: row['seed'])}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path)
    args = parser.parse_args()
    try:
        print(json.dumps(analyze_directory(args.directory), indent=2, allow_nan=False))
    except (OSError, ValueError, KeyError, TypeError, IndexError, OverflowError) as exc:
        parser.exit(1, f'contingency-eligibility: {exc}\n')


if __name__ == '__main__':
    main()
