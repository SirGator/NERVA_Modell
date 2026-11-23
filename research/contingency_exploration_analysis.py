#!/usr/bin/env python3
"""Verify matched structural exploration, delivery, and acute recall in NERVA."""

import argparse
from collections import Counter, defaultdict
import hashlib
import json
from pathlib import Path

from contingency_capacity_analysis import Assay, MAPPINGS, analyze_record as analyze_capacity
from contingency_capacity_analysis import identity_set, natural
from contingency_eligibility_analysis import number, source_record
from contingency_homeostasis_analysis import training_spikes
from contingency_structural_analysis import PROBES, gate_rows, lifecycles, matched, read_probe
from contingency_structural_analysis import PROTOCOL as STRUCTURAL
from contingency_structural_analysis import read_training, topology, training_activity
from contingency_structural_decisions_analysis import distance_between, read_geometry
from verify_contingency import FRAME_US, ORIGINAL, require

PROTOCOL = 'NERVA-Contingency-Structural-Exploration/1'
ARMS = ('native_prune', 'native_retain', 'recent_prune', 'recent_retain',
        'nearest_prune', 'nearest_retain')
RECENT = ('recent_prune', 'recent_retain')
EXPLORATORY_FIELDS = {'exploratory', 'exploratory_arrivals'}
WINDOW_US = 100_000
RADIUS = 0.25


def local_distances(positions, excitatory):
    distances = {}
    for target in positions:
        distances[target] = {}
        for source in excitatory - {target}:
            distance = distance_between(positions[source], positions[target])
            if distance <= RADIUS:
                distances[target][source] = distance
    return distances


def verify_births(trial, initial, births, initial_last, distances, policy):
    rows = trial['exploratory']
    ids = [row[0] for row in rows]
    require(ids == sorted(identity_set(ids, 'exploratory formation'))
            and set(ids) <= births.keys()
            and (policy != 'native' or not rows),
            'exploratory ledger does not identify training births')
    observations = {row[0]: row[1:] for row in rows}
    incoming = defaultdict(Counter)
    live = {edge: pair[:2] for edge, pair in initial.items()}
    for source, target in live.values():
        incoming[target][source] += 1
    last = initial_last.copy()
    used_targets = set()
    result = {}
    for frame in trial['frames']:
        latest = max((time for _, time in frame['root_spikes'] + frame['core_spikes']),
                     default=frame['start_us'])
        for cell, time in frame['core_spikes']:
            last[cell] = time if last[cell] is None else max(last[cell], time)
        formed = frame['formed']
        require(len({target for _, _, target in formed}) == len(formed),
                'multiple formations to one target on a maintenance boundary')
        decision_times = set()
        for edge, source, target in formed:
            if edge not in observations:
                continue
            now, source_last, target_last, count, drive = observations[edge]
            require(natural(now) and latest <= now <= frame['end_us']
                    and target not in used_targets and .5 < number(drive) <= 3.,
                    'exploratory formation has invalid clock, reused target, or low drive')
            decision_times.add(now)
            require((source_last is None or natural(source_last))
                    and (target_last is None or natural(target_last))
                    and target_last == last[target]
                    and (target_last is None or now - target_last > WINDOW_US)
                    and source_last == last[source],
                    'exploratory birth timestamps differ from the last spike ledger')
            neighbors = distances[target]
            free = neighbors.keys() - incoming[target]
            recent = {cell for cell in free if last[cell] is not None
                      and 0 <= now - last[cell] <= WINDOW_US}
            require(natural(count) and count == len(recent) and recent,
                    'exploratory birth lacks the recorded recent free excitatory neighborhood')
            if policy == 'recent':
                selected = min(recent, key=lambda cell: (-last[cell], neighbors[cell], cell))
            else:
                selected = min(free, key=lambda cell: (neighbors[cell], cell))
            require(source == selected, 'exploratory birth source differs from policy choice')
            used_targets.add(target)
            result[edge] = {
                'source': source, 'target': target, 'time_us': now, 'distance': neighbors[source],
                'source_last_spike_age_us': None if source_last is None else now - source_last,
                'target_last_spike_age_us': None if target_last is None else now - target_last,
                'recent_free_excitatory_candidates': count, 'target_drive': drive,
            }
        require(len(decision_times) <= 1, 'exploratory births at one boundary disagree on clock')
        for edge, source, target in formed:
            live[edge] = (source, target)
            incoming[target][source] += 1
        for edge in frame['pruned']:
            source, target = live.pop(edge)
            incoming[target][source] -= 1
            if not incoming[target][source]:
                del incoming[target][source]
    return result


def verify_delivery(trial, exploration, edges, births, deaths, start, stop, focus, quiet):
    spikes = training_spikes(trial['frames'])
    expected = {}
    for edge in exploration:
        source, target, delay, _ = edges[edge]
        for time in spikes.get(source, ()):
            arrival = time + delay
            if births[edge] < time and arrival < stop and arrival <= deaths.get(edge, stop):
                expected[arrival, edge] = (source, target)

    actual = {}
    previous = start
    by_edge = defaultdict(list)
    for time, edge, source, target, amplitude in trial['exploratory_arrivals']:
        key = time, edge
        require(natural(time) and previous <= time < stop and natural(edge)
                and key not in actual and expected.get(key) == (source, target)
                and number(amplitude) >= 0,
                'exploratory arrival does not match a live emitted excitatory edge')
        previous = time
        actual[key] = (source, target, amplitude)
        by_edge[edge].append((time, amplitude))
    require(actual.keys() == expected.keys(),
            'exploratory arrivals omit emissions from the training spike ledger')
    overlap = {(time, edge): (pre, post, amp) for time, edge, pre, post, amp in trial['arrivals']
               if edge in exploration}
    require(overlap == {key: row for key, row in actual.items()
                        if key[1] in focus or row[1] in quiet},
            'exploratory arrivals disagree with the archived focus and quiet capture')
    return by_edge


def population_activity(trial, terminal, quiet, dormant, dormant_both, exploration,
                        delivery, deaths):
    spikes = training_spikes(trial['frames'])
    exploratory_targets = {row['target'] for row in exploration.values()}
    surviving_ids = set(exploration) - deaths.keys()
    survivor_targets = {exploration[edge]['target'] for edge in surviving_ids}
    delivered_targets, positive_targets, survivor_positive_targets = set(), set(), set()
    for edge, row in exploration.items():
        target = row['target']
        arrivals = delivery[edge]
        positive = [time for time, amplitude in arrivals if amplitude > 0]
        if arrivals:
            delivered_targets.add(target)
        if positive:
            positive_targets.add(target)
            if edge in surviving_ids:
                survivor_positive_targets.add(target)
        times = spikes.get(target, ())
        row.update({
            'pruned_at_frame_end_us': deaths.get(edge),
            'survived': edge in surviving_ids,
            'arrivals': len(arrivals),
            'positive_arrivals': len(positive),
            'positive_arrival_amplitude_sum': sum(amplitude for _, amplitude in arrivals
                                                   if amplitude > 0),
            'first_positive_arrival_us': min(positive, default=None),
            'first_target_training_spike_us': min(times, default=None),
            'target_spikes_after_first_positive_arrival': (
                sum(time >= positive[0] for time in times) if positive else 0),
            'target_dormant_in_matched_frozen': target in dormant,
            'target_dormant_in_both_frozen': target in dormant_both,
            'target_is_focus_relay': target in quiet,
        })
    summary = {
        'training_core_spikes': sum(map(len, spikes.values())),
        'training_core_cells_firing': len(spikes),
        'baseline_dormant_cells': len(dormant),
        'baseline_dormant_cells_firing': len(dormant & spikes.keys()),
        'baseline_dormant_in_both_cells': len(dormant_both),
        'baseline_dormant_in_both_cells_firing': len(dormant_both & spikes.keys()),
        'focus_relays_firing': len(quiet & spikes.keys()),
        'focus_relays_with_positive_exploratory_delivery': len(quiet & positive_targets),
        'focus_terminal_task_arrivals': sum(row['task_arrivals'] for row in terminal.values()),
        'focus_terminal_new_tag_events': sum(row['new_tag_events'] for row in terminal.values()),
        'focus_terminal_prunings': len(terminal.keys() & deaths.keys()),
        'exploratory_births': len(exploration),
        'exploratory_survivors': len(surviving_ids),
        'exploratory_targets': len(exploratory_targets),
        'exploratory_targets_with_arrivals': len(delivered_targets),
        'exploratory_targets_with_positive_arrivals': len(positive_targets),
        'dormant_targets_with_positive_arrivals': len(dormant & positive_targets),
        'surviving_exploratory_targets': len(survivor_targets),
        'surviving_exploratory_targets_with_positive_arrivals': len(survivor_positive_targets),
        'surviving_exploratory_dormant_targets': len(survivor_targets & dormant),
        'surviving_exploratory_dormant_targets_firing': len(survivor_targets & dormant & spikes.keys()),
        'exploratory_arrivals': sum(row['arrivals'] for row in exploration.values()),
        'exploratory_positive_arrivals': sum(row['positive_arrivals'] for row in exploration.values()),
        'exploratory_positive_amplitude_sum': sum(
            row['positive_arrival_amplitude_sum'] for row in exploration.values()),
    }
    summary['exploratory'] = exploration
    return summary


def expected_ablations(gates, surviving):
    return [(arm, mapping, cue)
            for arm in RECENT
            for mapping in MAPPINGS
            for cue in ORIGINAL
            if surviving[arm][mapping] and gates[arm][mapping][cue]['passed']
            and all(gates[arm][mapping][cue]['correct'] >
                    gates[control + arm.removeprefix('recent')][mapping][cue]['correct']
                    for control in ('native', 'nearest'))]


def analyze_record(raw, source, structural, checksum):
    analyze_capacity(source)
    job, details = source['job'], source['details']
    seed, config = job['seed'], job['effective_config']
    assay = Assay(details, config)
    repetitions, start = config['training_repetitions'], assay.checkpoint
    stop = start + repetitions * 10 * FRAME_US
    recall_start = stop + config['rest_us']
    for record, protocol in ((raw, PROTOCOL), (structural, STRUCTURAL)):
        require(record['protocol'] == protocol and record['replay_verified'] is True
                and record['seed'] == seed and record['archived_record_checksum'] == checksum
                and record['checkpoint_time_us'] == start
                and record['core_ids'] == details['core_ids']
                and record['target_groups'] == details['target_groups']
                and type(record['stdp_window_us']) is int and record['stdp_window_us'] == WINDOW_US,
                'exploration and structural checkpoint identities differ from verified capacity')
    for key in ('initial_topology', 'focus', 'quiet_sources'):
        require(raw[key] == structural[key], f'exploration checkpoint differs from structural: {key}')
    initial = topology(raw['initial_topology'])
    require(all(edge[3] for edge in initial.values()) and
            [row[:4] for row in raw['initial_topology']] == [row[:4] for row in assay.topology],
            'exploration initial topology differs from capacity checkpoint')
    positions, polarity = read_geometry(raw['core_geometry'], raw['core_ids'])
    require({cell for cell in polarity if polarity[cell]} == assay.excitatory,
            'exploration Core polarity differs from capacity checkpoint')
    distances = local_distances(positions, assay.excitatory)
    initial_last_rows = raw['initial_last_spikes']
    require([row[0] for row in initial_last_rows] == raw['core_ids']
            and all(natural(cell) and (last is None or natural(last) and last < start)
                    for cell, last in initial_last_rows),
            'exploration checkpoint last spikes have invalid IDs or times')
    initial_last = dict(initial_last_rows)
    require(raw['arms'].keys() == set(ARMS)
            and structural['arms'].keys() == {'frozen', 'growing'}
            and all(rows.keys() == set(MAPPINGS)
                    for rows in (*raw['arms'].values(), *structural['arms'].values()))
            and raw['focus'].keys() == raw['quiet_sources'].keys() == set(MAPPINGS),
            'exploration arm and observation mappings differ from design')
    dormant = {
        mapping: assay.core - training_spikes(
            structural['arms']['frozen'][mapping]['training']['frames']).keys()
        for mapping in MAPPINGS
    }
    dormant_both = set.intersection(*dormant.values())

    def read_arm(arm, mapping, name, focus, quiet):
        trial = arm['training']
        roots = read_training(trial, assay, mapping, seed, repetitions)
        final = topology(trial['end_topology'])
        edges, births, deaths = lifecycles(initial, final, trial['frames'], assay.core, name != 'frozen')
        require(not name.endswith('_retain') or not deaths,
                'retain arm pruned topology during training')
        activity, _ = training_activity(trial, focus, quiet, edges, births, deaths, start,
                                        stop, WINDOW_US, assay.core)
        live = {edge for edge, (*_, enabled) in final.items() if enabled}
        require(arm['probes'].keys() == set(PROBES), 'unexpected exploration recall probes')
        for cue, probe in arm['probes'].items():
            read_probe(probe, cue, assay, recall_start, live, assay.reset[cue])
        expansion, delivered = {}, {}
        if name != 'frozen':
            expansion = verify_births(trial, initial, births, initial_last, distances, name.split('_')[0])
            delivered = verify_delivery(trial, expansion, edges, births, deaths, start, stop, focus, quiet)
        activity.update(population_activity(trial, activity['terminal_edges'], quiet,
                                            dormant.get(mapping, dormant_both), dormant_both,
                                            expansion, delivered, deaths))
        return ({**activity, 'births': len(births), 'prunings': len(deaths),
                 'surviving_new_edges': len(live & births.keys())}, roots, live, sorted(live & expansion.keys()))

    results = {name: {} for name in ('frozen', *ARMS)}
    training_roots, frozen = {}, {}
    live, surviving, probes = ({name: {} for name in ARMS} for _ in range(3))
    followup = details['followup']
    for mapping in MAPPINGS:
        baseline = structural['arms']['frozen'][mapping]
        native = followup['training'][mapping]
        focus_ids = sorted({edge for row in followup['terminal_ablation']
                            if row['mapping'] == mapping for edge in row['restored_edges']})
        focus = {edge: initial[edge][:3] for edge in focus_ids}
        quiet = {pre for pre, _, _ in focus.values()} - training_spikes(native['frames']).keys()
        require(raw['focus'][mapping] == [[edge, *binding] for edge, binding in focus.items()]
                and raw['quiet_sources'][mapping] == sorted(quiet),
                'exploration focus disagrees with capacity terminal ablation and training')
        frozen[mapping] = baseline['probes']
        reference = [frame['root_spikes'] for frame in native['frames']]
        require([frame['root_spikes'] for frame in baseline['training']['frames']] == reference,
                'archived frozen training differs from capacity root input')
        results['frozen'][mapping], training_roots[mapping], _, _ = read_arm(
            baseline, mapping, 'frozen', focus, quiet)
        native_prune = raw['arms']['native_prune'][mapping]
        stripped = {**native_prune, 'training': {
            key: value for key, value in native_prune['training'].items()
            if key not in EXPLORATORY_FIELDS}}
        require(stripped == structural['arms']['growing'][mapping],
                'native pruning arm differs from prior structural growing replay')
        for name in ARMS:
            arm = raw['arms'][name][mapping]
            trial = arm['training']
            require(trial['removed_tags'] == baseline['training']['removed_tags']
                    and [frame['root_spikes'] for frame in trial['frames']] == reference,
                    'exploration training differs in eligibility reset or presented root input')
            summary, roots, live[name][mapping], surviving[name][mapping] = read_arm(
                arm, mapping, name, focus, quiet)
            require(roots == training_roots[mapping], 'exploration training changes root spike totals')
            results[name][mapping], probes[name][mapping] = summary, arm['probes']

    preliminary = {name: gate_rows(frozen, probes[name], assay.targets) for name in ARMS}
    require(raw['preliminary_gates'] == preliminary, 'preliminary exploration recall gates differ')
    triggered = {name for name in ARMS if any(row['passed']
                 for cues in preliminary[name].values() for row in cues.values())}
    require(raw['balanced'].keys() == triggered,
            'balanced exploration controls differ from preliminary recall trigger')
    balanced, gates = {}, {}
    for name in ARMS:
        control = raw['balanced'].get(name)
        if control is not None:
            balanced[name], roots, _, _ = read_arm(control, 'balanced', name, {}, set())
            require(all(roots == counts for counts in training_roots.values())
                    and control['training']['removed_tags'] ==
                    structural['arms']['frozen']['forward']['training']['removed_tags'],
                    'balanced exploration differs in marginal root input or eligibility reset')
        gates[name] = gate_rows(frozen, probes[name], assay.targets, control['probes'] if control else None)
    require(raw['gates'] == gates, 'final exploration recall gates differ')

    expected = expected_ablations(gates, surviving)
    require([(row['arm'], row['mapping'], row['cue']) for row in raw['ablations']] == expected,
            'acute exploration ablations differ from eligible recall cases')
    ablations = []
    for row in raw['ablations']:
        arm, mapping, cue = (row[key] for key in ('arm', 'mapping', 'cue'))
        blocked = surviving[arm][mapping]
        require(row['blocked_edges'] == blocked and natural(row['queued_arrivals_cancelled']),
                'acute recall did not block exactly the surviving exploratory edges')
        probe = row['probe']
        counts = read_probe(probe, cue, assay, recall_start,
                            live[arm][mapping] - set(blocked), assay.reset[cue])
        require(matched(probe, probes[arm][mapping][cue]), 'acute ablation changed presented root input')
        target = ORIGINAL.index(cue) ^ MAPPINGS.index(mapping)
        correct, wrong = [sum(counts[cell] for cell in assay.targets[i]) for i in (target, 1 ^ target)]
        gate = gates[arm][mapping][cue]
        ablations.append({
            'arm': arm, 'mapping': mapping, 'cue': cue, 'blocked_edges': blocked,
            'queued_arrivals_cancelled': row['queued_arrivals_cancelled'],
            'unblocked_correct': gate['correct'], 'blocked_correct': correct,
            'blocked_incorrect': wrong, 'blocked_rest_core_spikes': probe['rest_core_spikes'],
            'correct_decrement': gate['correct'] - correct,
            'target_advantage_abolished': correct <= max(
                wrong, gate['frozen'], gate['opposite'], gate['balanced'] or 0),
            'correct_at_or_below_frozen': correct <= gate['frozen'],
        })

    paired_metrics = (
        'baseline_dormant_cells_firing', 'baseline_dormant_in_both_cells_firing',
        'focus_relays_firing', 'focus_relays_with_positive_exploratory_delivery',
        'focus_terminal_task_arrivals', 'focus_terminal_new_tag_events', 'focus_terminal_prunings',
        'exploratory_births', 'exploratory_survivors', 'exploratory_arrivals',
        'exploratory_positive_arrivals', 'exploratory_positive_amplitude_sum',
        'exploratory_targets_with_positive_arrivals', 'dormant_targets_with_positive_arrivals',
        'surviving_exploratory_targets_with_positive_arrivals',
        'surviving_exploratory_dormant_targets', 'surviving_exploratory_dormant_targets_firing',
    )
    contrasts = {}
    for pruning in ('prune', 'retain'):
        contrasts[pruning] = {}
        for mapping in MAPPINGS:
            recent, nearest, native = (results[f'{rule}_{pruning}'][mapping]
                                       for rule in ('recent', 'nearest', 'native'))
            contrasts[pruning][mapping] = {
                'recent_minus_nearest': {key: recent[key] - nearest[key] for key in paired_metrics},
                'recent_minus_native': {key: recent[key] - native[key] for key in paired_metrics},
                'nearest_minus_native': {key: nearest[key] - native[key] for key in paired_metrics},
                'recall_correct_spikes': {
                    cue: {
                        'recent_minus_nearest': gates[f'recent_{pruning}'][mapping][cue]['correct']
                        - gates[f'nearest_{pruning}'][mapping][cue]['correct'],
                        'recent_minus_native': gates[f'recent_{pruning}'][mapping][cue]['correct']
                        - gates[f'native_{pruning}'][mapping][cue]['correct'],
                    } for cue in ORIGINAL
                },
            }
    return {'job_id': job['id'], 'seed': seed, 'archived_record_checksum': checksum,
            'arms': results, 'balanced': balanced, 'preliminary_gates': preliminary,
            'gates': gates, 'ablations': ablations, 'matched_training_contrasts': contrasts}


def analyze_directory(directory):
    raw, capacity, structural = [
        {path.name: path for path in (directory / part).glob('*.json')
         if path.name != 'manifest.json'}
        for part in ('raw', 'input/capacity', 'input/structural')
    ]
    require(raw and raw.keys() == capacity.keys() == structural.keys(),
            'exploration replay and archived input files differ')
    records = []
    for filename in sorted(raw):
        diagnostic, capacity_bytes, structural_bytes = (
            paths[filename].read_bytes() for paths in (raw, capacity, structural))
        source, checksum = source_record(capacity_bytes)
        require(filename == f"{source['job']['id']}.json", 'file name differs from capacity job identity')
        result = analyze_record(json.loads(diagnostic), source, json.loads(structural_bytes), checksum)
        records.append({**result, 'capacity_sha256': hashlib.sha256(capacity_bytes).hexdigest(),
                        'structural_sha256': hashlib.sha256(structural_bytes).hexdigest(),
                        'diagnostic_sha256': hashlib.sha256(diagnostic).hexdigest()})
    return {'protocol': PROTOCOL, 'cohort_size': len(records),
            'records': sorted(records, key=lambda row: (row['seed'], row['job_id']))}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path)
    args = parser.parse_args()
    try:
        print(json.dumps(analyze_directory(args.directory), indent=2, allow_nan=False))
    except (OSError, ValueError, KeyError, TypeError, IndexError, OverflowError) as exc:
        parser.exit(1, f'contingency-exploration: {exc}\n')


if __name__ == '__main__':
    main()
