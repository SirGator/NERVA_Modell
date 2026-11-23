#!/usr/bin/env python3
"""Verify retained-terminal consolidation on fixed recorded NERVA histories."""

import argparse
from collections import Counter, defaultdict
import hashlib
import json
import math
from pathlib import Path

from contingency_capacity_analysis import Assay, MAPPINGS, analyze_record as analyze_capacity, natural
from contingency_electrical_analysis import f32
from contingency_eligibility_analysis import CHEMICAL_FIELDS, close, number, source_record
from contingency_exploration_analysis import PROTOCOL as EXPLORATION
from contingency_exploration_analysis import WINDOW_US, local_distances, verify_births, verify_delivery
from contingency_homeostasis_analysis import training_spikes
from contingency_structural_analysis import lifecycles, read_training, topology, training_activity
from contingency_structural_decisions_analysis import read_geometry
from verify_contingency import FRAME_US, require

PROTOCOL = 'NERVA-Contingency-Consolidation/1'
CONTROLS = (*MAPPINGS, 'balanced')
MODES = {
    'original': (False, True),
    'direct_f32': (False, False),
    'constant_positive': (True, True),
    'constant_direct_f32': (True, False),
}
OBSERVATIONS = {'arrivals', 'tag_events', 'quiet_integrations'}
RULES = {'eligibility_tau_us': 500_000., 'learning_rate': 1.,
         'max_abs_eligibility': 10., 'weight_update_mode': 'accumulated'}
COMPARISON_FIELDS = ('net_tag_integral', 'net_requested', 'net_actual', 'residual')
RECENCY_RELAY = 1394


def bounded(value, low, high):
    return min(high, max(low, value))


def publish(weight, residual, request, accumulated):
    """Reproduce the local f32 publication and return signed clipped mass."""
    low, high = -weight, 2. - weight
    if accumulated:
        pending = residual + request
        allowed = bounded(pending, low, high)
        next_weight = bounded(f32(weight + allowed), 0., 2.)
        return next_weight, allowed - (next_weight - weight), pending - allowed
    next_weight = f32(bounded(weight + f32(request), 0., 2.))
    return next_weight, residual, request - bounded(request, low, high)


def replay_edge(initial, clocks, factors, tags, rules):
    _, weight, tag, anchor, residual, factor = initial
    weight, tag, factor = map(f32, (weight, tag, factor))
    tau, rate = rules['eligibility_tau_us'], rules['learning_rate']
    states = {name: (weight, residual) for name in MODES}
    ledgers = {name: dict.fromkeys((
        'positive_tag_integral', 'negative_tag_integral', 'positive_requested', 'negative_requested',
        'positive_actual', 'negative_actual', 'weight_changes', 'positive_blocked', 'negative_blocked',
        'positive_requested_without_change', 'negative_requested_without_change',
        'nonzero_requested_without_change', 'max_abs_requested', 'positive_clipped', 'negative_clipped',
    ), 0) for name in MODES}
    factors_at_nonzero_tag = Counter()
    for time in clocks:
        fraction = -(time - anchor) / tau if anchor is not None else 0.
        integral = tag * tau * 1e-6 * -math.expm1(fraction)
        if integral:
            factors_at_nonzero_tag['positive' if factor > 0 else 'negative' if factor < 0
                                   else 'zero'] += 1
        for name, (constant, accumulated) in MODES.items():
            request = rate * integral * (1. if constant else factor)
            old_weight, carry = states[name]
            visible, carry, clipped = publish(old_weight, carry, request, accumulated)
            actual = visible - old_weight
            ledger = ledgers[name]
            if integral:
                for field, value in (('tag_integral', integral), ('requested', request),
                                     ('clipped', clipped)):
                    ledger[f'positive_{field}'] += max(0., value)
                    ledger[f'negative_{field}'] += min(0., value)
                ledger['max_abs_requested'] = max(ledger['max_abs_requested'], abs(request))
                if request and not actual:
                    ledger['nonzero_requested_without_change'] += 1
                    ledger['positive_requested_without_change'] += max(0., request)
                    ledger['negative_requested_without_change'] += min(0., request)
            ledger['positive_actual'] += max(0., actual)
            ledger['negative_actual'] += min(0., actual)
            ledger['weight_changes'] += bool(actual)
            states[name] = visible, carry
        tag = f32(tag * math.exp(fraction))
        anchor = time
        factor = factors.get(time, factor)
        for phase in ('pre', 'post'):
            if (time, phase) in tags:
                before, after = tags[time, phase]
                require(tag == f32(before) and abs(f32(after)) <= rules['max_abs_eligibility'],
                        f'pair {phase} tag does not follow delayed decay at {time}')
                tag = f32(after)
    for name, ledger in ledgers.items():
        ledger['weight'], ledger['residual'] = states[name]
        ledger['eligibility'] = tag
    return ledgers, {sign: factors_at_nonzero_tag[sign] for sign in ('positive', 'negative', 'zero')}


def archive_training(trial, archived, focus, quiet):
    """Check that expanded instrumentation projects back to the archived training."""
    require(trial.keys() == archived.keys(), 'retained training fields differ from exploration')
    for key in trial.keys() - OBSERVATIONS:
        require(trial[key] == archived[key], f'retained training changed archived {key}')
    projections = {
        'arrivals': [row for row in trial['arrivals'] if row[1] in focus or row[3] in quiet],
        'tag_events': [row for row in trial['tag_events'] if row[1] in focus],
        'quiet_integrations': [row for row in trial['quiet_integrations'] if row[1] in quiet],
    }
    for field, rows in projections.items():
        require(rows == archived[field], f'expanded instrumentation differs from archived {field}')


def verify_history(history, trial, focus, start, stop, initial_weights, rules):
    require(history['verified_original_local_state'] is True
            and history['verified_retained_local_clocks'] is True,
            'native replay lacks retained local-state and clock checks')
    clocks = history['clock_times_us']
    require(clocks and clocks == sorted(set(clocks))
            and all(natural(time) and start <= time <= stop for time in clocks),
            'invalid delayed integration clock sequence')
    clock_set = set(clocks)
    require(history['chemical_fields'] == CHEMICAL_FIELDS, 'unexpected chemical fields')
    posts = {post for _, post, _ in focus.values()}
    factors = defaultdict(dict)
    prior = start
    for time, cell, *values in history['chemical_samples']:
        require(time in clock_set and time >= prior and cell in posts
                and time not in factors[cell] and len(values) == len(CHEMICAL_FIELDS) - 2,
                'invalid focused chemical sample')
        for value in values:
            number(value)
        prior = time
        factors[cell][time] = f32(values[3])
    expected_spikes = Counter()
    for frame in trial['frames']:
        for cell, time in frame['core_spikes']:
            require(time in clock_set, 'archived training spike has no delayed clock')
        expected_spikes.update((time, cell) for cell, time in frame['core_spikes'] if cell in posts)
    expected_arrivals = Counter((row[0], row[1]) for row in trial['arrivals'] if row[1] in focus)
    for name, expected in (('post_spikes', expected_spikes), ('pre_arrivals', expected_arrivals)):
        events = history[name]
        require(Counter(map(tuple, events)) == expected
                and all(time in clock_set for time, _ in events)
                and [row[0] for row in events] == sorted(row[0] for row in events),
                f'fixed-history {name} differ from training')
    initial = history['initial_local']
    require([row[0] for row in initial] == sorted(focus), 'initial local IDs differ from terminal union')
    target_initial_factors = {}
    tags = defaultdict(dict)
    for time, edge, phase, before, after in trial['tag_events']:
        require(time in clock_set, 'pair tag event has no delayed clock')
        tags[edge][time, phase] = number(before), number(after)
    endpoint = {row[0]: f32(row[3]) for row in trial['end_topology']}
    rows = {}
    for name in MODES:
        ledger = history[name]['synapses']
        require([row['synapse_id'] for row in ledger] == sorted(focus),
                f'{name} synapses differ from terminal union')
        rows[name] = {row['synapse_id']: row for row in ledger}

    results = {}
    for state in initial:
        edge, weight, tag, anchor, residual, factor = state
        pre, post, delay = focus[edge]
        require(f32(number(weight)) == initial_weights[edge] and number(tag) == 0
                and (anchor is None or natural(anchor) and anchor <= start)
                and abs(number(residual)) <= 2.**-24,
                f'edge {edge} initial local state differs from reset checkpoint')
        normalized = f32(number(factor))
        require(target_initial_factors.setdefault(post, normalized) == normalized,
                f'edge {edge} initial held signal differs for shared target')
        ledgers, signal_counts = replay_edge(state, clocks, factors[post], tags[edge], rules)
        result = {
            'source_id': pre, 'target_id': post, 'delay_us': delay,
            'initial_weight': f32(weight), 'initial_residual': residual,
            'initial_held_signal': normalized, 'initial_tag_anchor_us': anchor,
            'tag_events': len(tags[edge]),
            'held_signal_intervals_with_nonzero_tag': signal_counts,
        }
        for name, expected in ledgers.items():
            row = rows[name][edge]
            require((row['source_id'], row['target_id']) == (pre, post)
                    and f32(number(row['initial_weight'])) == f32(weight)
                    and all(row[field] is True for field in ('plastic', 'enabled', 'pre_excitatory')),
                    f'{name} edge {edge} metadata differ from learned terminal')
            for field, value in expected.items():
                observed = number(row[field])
                if field in ('weight', 'eligibility'):
                    matched = f32(observed) == value
                elif field in ('weight_changes', 'nonzero_requested_without_change'):
                    matched = natural(observed) and observed == value
                else:
                    matched = close(observed, value)
                require(matched, f'{name} edge {edge} {field} differs from reconstructed replay')
            if name == 'original':
                require(expected['weight'] == endpoint[edge],
                        f'original edge {edge} weight differs from retained endpoint')
            for field in ('tag_integral', 'requested', 'actual', 'clipped'):
                positive, negative = (expected[f'{sign}_{field}'] for sign in ('positive', 'negative'))
                expected[f'net_{field}'] = positive + negative
                expected[f'absolute_{field}'] = positive - negative
            gross = expected['absolute_requested']
            expected['signed_requested_cancellation'] = (
                1. - abs(expected['net_requested']) / gross if gross else None)
            expected['residual_change'] = expected['residual'] - residual
            expected['unpublished_unclipped_requested'] = (
                expected['net_requested'] - expected['net_actual'] - expected['net_clipped'])
            if MODES[name][1]:
                require(math.isclose(expected['unpublished_unclipped_requested'],
                                     expected['residual_change'], rel_tol=1e-9,
                                     abs_tol=max(1e-18, gross * 1e-12)),
                        f'{name} edge {edge} accumulated publication does not conserve signed mass')
            else:
                require(expected['residual_change'] == 0,
                        f'{name} edge {edge} direct publication changed inherited carry')
            result[name] = expected
        result['local_contrasts'] = {
            label: {field: result[first][field] - result[second][field]
                    for field in ('net_requested', 'net_actual', 'residual',
                                  'nonzero_requested_without_change')}
            for label, first, second in (
                ('native_minus_constant_signal', 'original', 'constant_positive'),
                ('native_accumulated_minus_direct', 'original', 'direct_f32'),
                ('constant_accumulated_minus_direct', 'constant_positive', 'constant_direct_f32'),
            )
        }
        results[edge] = result
    return results


def analyze_record(raw, source, exploration, checksum):
    analyze_capacity(source)
    job, details = source['job'], source['details']
    seed, config = job['seed'], job['effective_config']
    assay = Assay(details, config)
    start, repetitions = assay.checkpoint, config['training_repetitions']
    stop = start + repetitions * 10 * FRAME_US
    for record, protocol in ((raw, PROTOCOL), (exploration, EXPLORATION)):
        require(record['protocol'] == protocol and record['replay_verified'] is True
                and record['seed'] == seed and record['archived_record_checksum'] == checksum
                and record['checkpoint_time_us'] == start and record['core_ids'] == details['core_ids']
                and record['stdp_window_us'] == WINDOW_US,
                'consolidation and exploration do not identify the verified capacity checkpoint')
    require(raw['rules'] == RULES and raw['mappings'].keys() == set(CONTROLS),
            'unexpected retained protocol rules or mappings')
    require(raw['focus'] == exploration['focus'] and raw['quiet_sources'] == exploration['quiet_sources']
            and raw['focus'].keys() == raw['quiet_sources'].keys() == set(MAPPINGS),
            'consolidation focus differs from archived exploration')
    initial_rows = exploration['initial_topology']
    initial = topology(initial_rows)
    require([row[:4] for row in initial_rows] == [row[:4] for row in assay.topology]
            and all(edge[3] for edge in initial.values()),
            'exploration topology differs from capacity checkpoint')
    initial_weights = {row[0]: f32(row[3]) for row in initial_rows}
    positions, polarity = read_geometry(exploration['core_geometry'], raw['core_ids'])
    require({cell for cell, value in polarity.items() if value} == assay.excitatory,
            'exploration geometry differs from capacity polarity')
    distances = local_distances(positions, assay.excitatory)
    last_rows = exploration['initial_last_spikes']
    last = dict(last_rows)
    require([cell for cell, _ in last_rows] == raw['core_ids'] and all(
        time is None or natural(time) and time < start for time in last.values()),
        'exploration checkpoint last spikes differ from expected identities or clocks')

    original_focus, original_quiet = {}, {}
    followup = details['followup']
    for mapping in MAPPINGS:
        expected = sorted({edge for row in followup['terminal_ablation']
                           if row['mapping'] == mapping for edge in row['restored_edges']})
        original_focus[mapping] = {edge: initial[edge][:3] for edge in expected}
        quiet = {pre for pre, _, _ in original_focus[mapping].values()} - training_spikes(
            followup['training'][mapping]['frames']).keys()
        require(raw['focus'][mapping] == [[edge, *initial[edge][:3]] for edge in expected]
                and raw['quiet_sources'][mapping] == sorted(quiet),
                'terminal focus or quiet sources differ from capacity ablation and training')
        original_quiet[mapping] = quiet
    focus = dict(sorted((original_focus['forward'] | original_focus['reverse']).items()))
    quiet = set.union(*original_quiet.values())
    require(focus and set(focus) <= {edge for edge, *_, eligible in assay.topology if eligible}
            and all(pre in assay.excitatory and post in assay.core for pre, post, _ in focus.values()),
            'terminal union includes an ineligible edge')

    mappings, roots, relay = {}, {}, {}
    checkpoints = []
    relay_terminals = [edge for edge, (pre, _, _) in focus.items() if pre == RECENCY_RELAY]
    for mapping in CONTROLS:
        raw_mapping = raw['mappings'][mapping]
        trial = raw_mapping['training']
        archive = (exploration['balanced'].get('recent_retain') if mapping == 'balanced'
                   else exploration['arms']['recent_retain'][mapping])
        if archive is not None:
            archive_training(trial, archive['training'], original_focus.get(mapping, {}),
                             original_quiet.get(mapping, set()))
        roots[mapping] = read_training(trial, assay, mapping, seed, repetitions)
        final = topology(trial['end_topology'])
        edges, births, deaths = lifecycles(initial, final, trial['frames'], assay.core, True)
        require(not deaths, 'recent-retain training pruned a synapse')
        trace, _ = training_activity(trial, focus, quiet, edges, births, deaths,
                                     start, stop, WINDOW_US, assay.core)
        expansion = verify_births(trial, initial, births, last, distances, 'recent')
        deliveries = verify_delivery(trial, expansion, edges, births, deaths,
                                     start, stop, focus, quiet)
        if relay_terminals:
            relay[mapping] = {
                'terminal_edges': relay_terminals,
                'core_spike_times_us': training_spikes(trial['frames']).get(RECENCY_RELAY, []),
                'recorded_exploratory_inputs': [
                    {'synapse_id': edge, **info, 'deliveries': deliveries.get(edge, [])}
                    for edge, info in expansion.items() if info['target'] == RECENCY_RELAY
                ],
            }
        raw_history = raw_mapping['history']
        checkpoints.append(raw_history['initial_local'])
        history = verify_history(raw_history, trial, focus, start, stop, initial_weights, RULES)
        for edge, activity in trace['terminal_edges'].items():
            history[edge]['activity'] = activity
        if archive is None:
            require(trial['removed_tags'] ==
                    exploration['arms']['recent_retain']['forward']['training']['removed_tags'],
                    'balanced training eligibility reset differs from the retained archive')
        mappings[mapping] = {'edges': history, 'quiet_sources': trace['quiet_sources']}
    require(checkpoints[0] == checkpoints[1] == checkpoints[2],
            'mappings did not retain the same checkpoint learning state')
    require(roots['forward'] == roots['reverse'] == roots['balanced'],
            'balanced and assigned training differ in marginal root events')

    comparisons = {}
    for mapping, opposite in zip(MAPPINGS, reversed(MAPPINGS)):
        comparisons[mapping] = {}
        for edge in original_focus[mapping]:
            comparisons[mapping][edge] = {
                control: {
                    variant: {
                        field: mappings[mapping]['edges'][edge][variant][field] -
                        mappings[control]['edges'][edge][variant][field]
                        for field in COMPARISON_FIELDS
                    } for variant in MODES
                } for control in (opposite, 'balanced')
            }
    return {
        'job_id': job['id'], 'seed': seed, 'archived_record_checksum': checksum,
        'assigned_focus': raw['focus'], 'checkpoint_quiet_sources': raw['quiet_sources'],
        'mappings': mappings, 'assigned_minus_controls': comparisons,
        'recency_relay_1394': relay,
    }


def analyze_directory(directory):
    raw, capacity, exploration = [
        {path.name: path for path in (directory / part).glob('*.json') if path.name != 'manifest.json'}
        for part in ('raw', 'input/capacity', 'input/exploration')
    ]
    require(raw and raw.keys() == capacity.keys() == exploration.keys(),
            'consolidation replay and archived inputs differ')
    records = []
    for filename in sorted(raw):
        diagnostic, capacity_bytes, exploration_bytes = (
            paths[filename].read_bytes() for paths in (raw, capacity, exploration))
        source, checksum = source_record(capacity_bytes)
        require(filename == f"{source['job']['id']}.json", 'file name differs from capacity job identity')
        result = analyze_record(json.loads(diagnostic), source, json.loads(exploration_bytes), checksum)
        records.append({
            **result, 'capacity_sha256': hashlib.sha256(capacity_bytes).hexdigest(),
            'exploration_sha256': hashlib.sha256(exploration_bytes).hexdigest(),
            'diagnostic_sha256': hashlib.sha256(diagnostic).hexdigest(),
        })
    return {'protocol': PROTOCOL, 'cohort_size': len(records),
            'records': sorted(records, key=lambda row: (row['seed'], row['job_id']))}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path)
    args = parser.parse_args()
    try:
        print(json.dumps(analyze_directory(args.directory), indent=2, allow_nan=False))
    except (OSError, ValueError, KeyError, TypeError, IndexError, OverflowError) as exc:
        parser.exit(1, f'contingency-consolidation: {exc}\n')


if __name__ == '__main__':
    main()
