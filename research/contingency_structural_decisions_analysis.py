#!/usr/bin/env python3
"""Verify native formation and pruning decisions against archived structural training."""

import argparse
from bisect import bisect_right
from collections import Counter
import hashlib
import json
import math
from pathlib import Path
import re

from contingency_capacity_analysis import MAPPINGS, identity_set, natural
from contingency_electrical_analysis import f32
from contingency_eligibility_analysis import number
from contingency_homeostasis_analysis import training_spikes
from contingency_structural_analysis import DEVELOPMENT_US, FORMATION_DELAY_US
from contingency_structural_analysis import PROTOCOL as ARCHIVE_PROTOCOL
from contingency_structural_analysis import lifecycles, topology
from verify_contingency import FRAME_US, event_counts, require, training_sequence

PROTOCOL = 'NERVA-Contingency-Structural-Decisions/1'
NATIVE_RULES = {
    'growth_threshold': .5,
    'radius': .25,
    'activity_weight': 1.,
    'temporal_weight': 1.,
    'distance_weight': .1,
    'temporal_tau_us': 20_000.,
    'recency_tau_us': 100_000.,
    'min_candidate_score': .01,
    'pruning_threshold': .01,
    'min_below_duration_us': 300_000,
    'utility_tau_us': 500_000.,
}
OUTCOMES = ('demand_blocked', 'no_new_neighbor', 'score_blocked', 'formed')
F32_MAX = f32(3.4028234663852886e38)


def finite_f32(value):
    return f32(max(-F32_MAX, min(F32_MAX, value)))


def optional_time(value, now):
    require(value is None or (natural(value) and value <= now), 'invalid native timestamp')
    return value


def nonnegative_f32(value):
    value = f32(number(value))
    require(math.isfinite(value) and value >= 0, 'invalid nonnegative native value')
    return value


def candidate_score(source_trace, target_trace, source_last, target_last, distance, now, rules):
    activity = finite_f32(source_trace * target_trace)
    temporal = 0.
    if source_last is not None and target_last is not None:
        proximity = math.exp(-abs(source_last - target_last) / rules['temporal_tau_us'])
        recency = math.exp(-(now - max(source_last, target_last)) / rules['recency_tau_us'])
        temporal = finite_f32(proximity * recency)
    return finite_f32(
        rules['activity_weight'] * activity
        + rules['temporal_weight'] * temporal
        - rules['distance_weight'] * distance
    )


def formation_choice(row, now, rules, *, checkpoint=0):
    drive = nonnegative_f32(row['drive'])
    target_trace = nonnegative_f32(row['target_trace'])
    target_last = optional_time(row['target_last_spike_us'], now)
    live_scores = []
    unilateral_trace = 0
    unilateral_task_spikes = 0
    ages = []
    for source, distance, trace, last, affinity in row['neighbors']:
        distance, trace, affinity = map(nonnegative_f32, (distance, trace, affinity))
        require(distance <= rules['radius'] and affinity > 0, 'neighbor distance or affinity outside rule')
        last = optional_time(last, now)
        score = candidate_score(trace, target_trace, last, target_last, distance, now, rules)
        if score >= rules['min_candidate_score']:
            live_scores.append((f32(score * affinity), -source))
        unilateral_trace += trace > 0 and target_trace == 0
        if last is not None:
            ages.append(now - last)
            unilateral_task_spikes += last >= checkpoint and (target_last is None
                                                               or target_last < checkpoint)
    comparison = {
        'neighbors': len(row['neighbors']),
        'unilateral_source_trace_neighbors': unilateral_trace,
        'unilateral_source_task_spike_neighbors': unilateral_task_spikes,
        'target_last_spike_age_us': None if target_last is None else now - target_last,
        'youngest_neighbor_last_spike_age_us': min(ages, default=None),
    }
    if drive <= rules['growth_threshold']:
        return 'demand_blocked', None, None, comparison
    if not row['neighbors']:
        return 'no_new_neighbor', 0, None, comparison
    if not live_scores:
        return 'score_blocked', 0, None, comparison
    return 'formed', len(live_scores), -max(live_scores)[1], comparison


def pruning_choice(row, now, rules, previous, *, checkpoint=None):
    stored = nonnegative_f32(row['stored_utility'])
    updated = optional_time(row['updated_us'], now)
    supported = row['supported']
    require(type(supported) is bool, 'invalid pruning rule support')
    threshold = rules['pruning_threshold']
    tau = rules['utility_tau_us']
    reported = nonnegative_f32(row['decayed_utility'])
    if updated is None:
        require(stored == 0, 'utility without an evidence timestamp')
        decayed = 0.
    else:
        exponent = f32(-f32(now - updated) / tau)
        decayed = f32(stored * f32(math.exp(exponent)))
    # Rust uses expf and Python uses exp. Two f32 roundings cover libm differences.
    require(math.isclose(reported, decayed, rel_tol=2 ** -22, abs_tol=2 ** -148),
            'decayed utility differs from native utility memory')
    state = stored, updated
    if previous is not None:
        old_state, _, old_support = previous
        require(supported == old_support
                and (old_state[1] is None or (updated is not None and updated >= old_state[1]))
                and (updated != old_state[1] or stored == old_state[0]),
                'pruning support or utility memory changed without new evidence')
    reported_since = optional_time(row['below_since_us'], now)
    since = None
    if supported and reported < threshold:
        if previous is not None and previous[0] == state and previous[1] is not None:
            since = previous[1]
        elif updated is None:
            since = now
            # A live checkpoint edge can carry a timer from the preceding development phase.
            # The archive does not record those controller observations or this edge's birth.
            if (previous is None and checkpoint is not None and reported_since is not None
                    and checkpoint - FRAME_US - rules['min_below_duration_us'] <= reported_since
                    and reported_since < checkpoint
                    and ((reported_since - 1) // FRAME_US) % 2 == 1):
                since = reported_since
        elif stored <= threshold:
            since = updated
        else:
            crossing_delay = math.floor(tau * math.log(stored / threshold) + .5)
            since = min(now, updated + crossing_delay)
    require(reported_since == since,
            'below-threshold timestamp differs from native timer')
    selected = since is not None and now - since >= rules['min_below_duration_us']
    require(type(row['selected']) is bool and row['selected'] == selected,
            'pruning selection differs from native duration rule')
    return (state, since, supported), selected


def compare_sample(cells):
    cells = sorted(cells)
    count = min(len(cells), 12)
    return [cells[i * len(cells) // count] for i in range(count)]


def archived_frames(archive, mapping, core, checkpoint, seed, expected_boundaries):
    frame_sets = {}
    expected_stimuli = list(training_sequence(mapping, seed, expected_boundaries // 5, 0))
    for arm in ('frozen', 'growing'):
        frames = archive['arms'][arm][mapping]['training']['frames']
        require(len(frames) == len(expected_stimuli), 'archived training frame count differs')
        for i, (frame, stimulus) in enumerate(zip(frames, expected_stimuli)):
            start = checkpoint + i * FRAME_US
            require(frame['stimulus'] == stimulus and frame['start_us'] == start
                    and frame['end_us'] == start + FRAME_US - 1,
                    'archived training frame time or stimulus differs')
            event_counts(frame['core_spikes'], start, start + FRAME_US, ids=core, label='Core')
            event_counts(frame['root_spikes'], start, start + FRAME_US, label='root')
        frame_sets[arm] = frames
    require([frame['root_spikes'] for frame in frame_sets['frozen']]
            == [frame['root_spikes'] for frame in frame_sets['growing']],
            'archived frozen and growing root inputs differ')
    return frame_sets['frozen'], frame_sets['growing']


def check_spike(cell, last, now, checkpoint, spikes, initial_spikes):
    last = optional_time(last, now)
    times = spikes.get(cell, ())
    index = bisect_right(times, now)
    if index:
        require(last == times[index - 1], 'last spike differs from growing training spike ledger')
    else:
        require(last is None or last < checkpoint, 'last spike missing from growing training ledger')
        require(initial_spikes.setdefault(cell, last) == last,
                'inconsistent last spike before checkpoint')


def distance_between(first, second):
    dx, dy, dz = (f32(a - b) for a, b in zip(first, second))
    return f32(math.sqrt(f32(dx * dx + f32(dy * dy + f32(dz * dz)))))


def area_weights(x, extent):
    normalized = f32(min(1., max(0., (x + extent) / (2. * extent))))
    distances = [(normalized - center) ** 2 for center in (0., 1. / 3, 2. / 3, 1.)]
    closest = min(distances)
    raw = [math.exp((closest - distance) / (2. * f32(.18) ** 2)) for distance in distances]
    total = sum(raw)
    return [f32(value / total) for value in raw]


def affinity_between(pre, post, excitatory):
    def weighted(weights):
        total = 0.
        for position, coefficient in weights:
            total = f32(total + f32(post[position] * f32(coefficient)))
        return total

    if not excitatory:
        return f32(1. + f32(f32(.8) * post[1]))
    factors = (
        ((0, .8), (1, .55), (3, .45)),
        ((2, 1.2), (3, .2)),
        ((2, .8), (3, 1.), (1, .15)),
        ((0, .4), (2, .3), (3, .3)),
    )
    result = f32(.35)
    for influence, weights in zip(pre, factors):
        result = f32(result + f32(influence * weighted(weights)))
    return result


def read_geometry(rows, core_ids):
    require([row[0] for row in rows] == core_ids and all(natural(row[0]) for row in rows),
            'Core geometry IDs differ from archived checkpoint')
    positions = {}
    excitatory = {}
    for cell, x, y, z, polarity in rows:
        require(type(polarity) is bool, 'invalid Core polarity')
        positions[cell] = tuple(f32(number(axis)) for axis in (x, y, z))
        excitatory[cell] = polarity
    return positions, excitatory


def local_geometry(positions, excitatory, targets, radius):
    side = math.ceil(math.cbrt(len(positions)))
    # The model uses f64 cbrt with a ceil. An exact integral cube has that same side.
    if (side - 1) ** 3 == len(positions):
        side -= 1
    extent = f32(f32((side - 1) * f32(.06)) + f32(.01))
    weights = {cell: area_weights(x, extent) for cell, (x, _, _) in positions.items()}
    local = {}
    for target in targets:
        local[target] = {}
        for source in positions:
            if source != target:
                distance = distance_between(positions[target], positions[source])
                if distance <= radius:
                    affinity = affinity_between(weights[source], weights[target], excitatory[source])
                    local[target][source] = distance, affinity
    return local


def validate_formation_row(row, neighbors, observed_traces, spikes, initial_spikes, now, checkpoint):
    require([neighbor[0] for neighbor in row['neighbors']] == sorted(neighbors),
            'new neighbor IDs differ from pre-maintenance Core geometry and topology')

    def check_trace(cell, trace, last):
        trace = nonnegative_f32(trace)
        check_spike(cell, last, now, checkpoint, spikes, initial_spikes)
        require(last is not None or trace == 0, 'positive trace without a spike')
        require(observed_traces.setdefault(cell, trace) == trace,
                'inconsistent spike trace for the same cell and boundary')

    check_trace(row['target'], row['target_trace'], row['target_last_spike_us'])
    for source, distance, trace, last, affinity in row['neighbors']:
        expected_distance, expected_affinity = neighbors[source]
        require(natural(source) and nonnegative_f32(distance) == expected_distance,
                'neighbor distance differs from Core geometry')
        require(math.isclose(nonnegative_f32(affinity), expected_affinity, rel_tol=2 ** -22),
                'neighbor affinity differs from Hippocampal area field')
        check_trace(source, trace, last)


def summarize_formation(events):
    score_blocked = [item for outcome, item in events if outcome == 'score_blocked']
    ages = {}
    for key in ('target_last_spike_age_us', 'youngest_neighbor_last_spike_age_us'):
        distribution = Counter('never' if row[key] is None else row[key] for row in score_blocked)
        ages[key] = dict(sorted(distribution.items(), key=lambda item: (item[0] == 'never', item[0])))
    return {
        'outcomes': {key: sum(outcome == key for outcome, _ in events) for key in OUTCOMES},
        'score_blocked_neighbors': sum(row['neighbors'] for row in score_blocked),
        'score_blocked_last_spike_ages': ages,
        'score_blocked_with_recent_source': sum(
            row['recent_source'] for row in score_blocked),
        'score_blocked_with_recent_excitatory_source': sum(
            row['recent_excitatory_source'] for row in score_blocked),
        'score_blocked_unilateral_activity': {
            activity: {
                'decisions': sum(row[f'unilateral_source_{activity}_neighbors'] > 0
                                 for row in score_blocked),
                'neighbors': sum(row[f'unilateral_source_{activity}_neighbors'] for row in score_blocked),
            } for activity in ('trace', 'task_spike')
        },
    }


def verify_mapping(raw, archive, mapping, initial, core, positions, excitatory, rules,
                   initial_spikes, expected_boundaries):
    checkpoint = archive['checkpoint_time_us']
    frozen, growing = archived_frames(archive, mapping, core, checkpoint, archive['seed'],
                                      expected_boundaries)
    active = training_spikes(frozen).keys()
    focus_relays = archive['quiet_sources'][mapping]
    focus_rows = archive['focus'][mapping]
    focus_edges = [row[0] for row in focus_rows]
    stratum_members = {
        'quiet_comparison': compare_sample(core - active),
        'active_comparison': compare_sample(active),
        'focus_relays': focus_relays,
    }
    for name, cells in stratum_members.items():
        require(raw[name] == cells and all(map(natural, raw[name])),
                f'{mapping}: {name} differs from archived sampling rule')
    require(focus_relays == sorted(identity_set(focus_relays, 'focus relays'))
            and focus_edges == sorted(identity_set(focus_edges, 'focus edges'))
            and raw['focus_edges'] == focus_edges and all(map(natural, raw['focus_edges']))
            and all(edge in initial and list(initial[edge][:3]) == binding
                    and set(binding[:2]) <= core and excitatory[binding[0]]
                    for edge, *binding in focus_rows)
            and set(focus_relays) <= {row[1] for row in focus_rows},
            'focus identities differ from archived quiet relays and edges')
    targets = sorted({cell for cells in stratum_members.values() for cell in cells})
    expected_local = local_geometry(positions, excitatory, targets, rules['radius'])
    decisions = raw['decisions']
    boundaries = [frame['end_us'] for frame in growing if frame['end_us'] % DEVELOPMENT_US == 0]
    require(len(boundaries) == len(decisions) == expected_boundaries,
            'decisions differ from the expected native maintenance boundaries')
    final = topology(archive['arms']['growing'][mapping]['training']['end_topology'])
    lifecycles(initial, final, growing, core, True)
    spikes = training_spikes(growing)
    spike_sets = {cell: set(times) for cell, times in spikes.items()}
    live = dict(initial)
    formation_events = {target: [] for target in targets}
    pruning_events = {edge: Counter() for edge in focus_edges}
    pruning_states, first_below, deaths = {}, {}, {}
    decisions = iter(decisions)
    for frame in growing:
        if frame['end_us'] % DEVELOPMENT_US:
            continue
        decision = next(decisions)
        now = decision['time_us']
        last_spike = max((time for _, time in frame['root_spikes'] + frame['core_spikes']),
                         default=frame['start_us'])
        require(natural(now) and last_spike <= now <= frame['end_us'],
                'native decision timestamp lies outside its processed maintenance frame')
        formation, pruning = decision['formation'], decision['pruning']
        require([row['target'] for row in formation] == targets
                and all(natural(row['target']) for row in formation),
                'formation rows differ from audited target IDs')
        require([row['edge'] for row in pruning] == sorted(live.keys() & set(focus_edges))
                and all(natural(row['edge']) for row in pruning),
                'pruning rows differ from live focus edges')
        connected = {target: set() for target in targets}
        for source, target, _, _ in live.values():
            if target in connected:
                connected[target].add(source)
        formed = {target: source for _, source, target in frame['formed']}
        require(len(formed) == len(frame['formed']), 'multiple formations for one target')
        observed_traces = {}
        for row in formation:
            target = row['target']
            neighbors = {source: inputs for source, inputs in expected_local[target].items()
                         if source not in connected[target]}
            validate_formation_row(row, neighbors, observed_traces, spikes, initial_spikes,
                                   now, checkpoint)
            outcome, eligible, source, comparison = formation_choice(row, now, rules, checkpoint=checkpoint)
            require(row['native_eligible'] == eligible
                    and (eligible is None or natural(row['native_eligible'])),
                    f'{mapping} at {now}, Core {target}: native eligibility count differs')
            require(row['selected'] == source == formed.get(target)
                    and (source is None or natural(row['selected'])),
                    f'{mapping} at {now}, Core {target}: formation choice differs')
            recent = [candidate[0] for candidate in row['neighbors']
                      if candidate[3] is not None and now - candidate[3] <= rules['recency_tau_us']]
            comparison['recent_source'] = bool(recent)
            comparison['recent_excitatory_source'] = any(excitatory[cell] for cell in recent)
            formation_events[target].append((outcome, comparison))
        pruned = set(frame['pruned'])
        for row in pruning:
            edge = row['edge']
            source, target, _, _ = live[edge]
            require(row['supported'] is excitatory[source], 'focus edge support differs from Core polarity')
            updated = optional_time(row['updated_us'], now)
            if updated is not None and updated >= checkpoint:
                require(updated in spike_sets.get(target, ()), 'utility update lacks postsynaptic spike')
            state, selected = pruning_choice(row, now, rules, pruning_states.get(edge),
                                             checkpoint=checkpoint)
            require(selected == (edge in pruned), f'{mapping} at {now}: pruning choice differs')
            pruning_states[edge] = state
            counts = pruning_events[edge]
            counts['observations'] += 1
            since = state[1]
            if since is not None:
                first_below.setdefault(edge, {'time_us': now, 'below_since_us': since})
                counts['below_threshold'] += 1
                counts['longest_below_us'] = max(counts['longest_below_us'], now - since)
            if selected:
                deaths[edge] = {
                    'time_us': now, 'below_since_us': since, 'elapsed_below_us': now - since,
                    'updated_us': updated, 'decayed_utility': row['decayed_utility'],
                }
        for edge, source, target in frame['formed']:
            live[edge] = (source, target, FORMATION_DELAY_US, True)
        for edge in pruned:
            del live[edge]
    return {
        'strata': {name: {'cells': len(cells), **summarize_formation(
            [event for cell in cells for event in formation_events[cell]])}
            for name, cells in stratum_members.items()},
        'focus_relays': {cell: summarize_formation(formation_events[cell]) for cell in focus_relays},
        'focus_pruning': {edge: {
            **{key: pruning_events[edge][key] for key in
               ('observations', 'below_threshold', 'longest_below_us')},
            'first_below_threshold_observation': first_below.get(edge),
            'pruned': deaths.get(edge),
        } for edge in focus_edges},
    }


def analyze_record(raw, archive):
    require(raw['protocol'] == PROTOCOL and archive['protocol'] == ARCHIVE_PROTOCOL
            and raw['replay_verified'] is True and archive['replay_verified'] is True,
            'inputs must be verified native structural and decision replays')
    for key in ('seed', 'archived_record_checksum', 'checkpoint_time_us', 'core_ids'):
        require(raw[key] == archive[key], f'decision replay differs from structural archive: {key}')
    seed, checkpoint = raw['seed'], raw['checkpoint_time_us']
    require(natural(seed) and natural(checkpoint) and checkpoint % DEVELOPMENT_US == 1
            and re.fullmatch(r'[a-f0-9]{16}', raw['archived_record_checksum']),
            'invalid seed, checkpoint or archived checksum')
    require(set(raw['rules']) == NATIVE_RULES.keys(), 'native rule parameters differ')
    rules = {}
    for key, value in NATIVE_RULES.items():
        if type(value) is int:
            require(type(raw['rules'][key]) is int and raw['rules'][key] == value,
                    f'native rule parameter differs: {key}')
            rules[key] = value
        else:
            rules[key] = nonnegative_f32(raw['rules'][key])
            require(rules[key] == f32(value), f'native rule parameter differs: {key}')
    core_ids = raw['core_ids']
    core = identity_set(core_ids, 'Core')
    require(core and core_ids == sorted(core) and raw['mappings'].keys() == set(MAPPINGS),
            'Core order or mappings differ from fixed protocol')
    frame_count = len(archive['arms']['growing']['forward']['training']['frames'])
    require(frame_count in (40, 80), 'unexpected native training extent')
    expected_boundaries = frame_count // 2
    positions, excitatory = read_geometry(raw['core_geometry'], core_ids)
    initial = topology(archive['initial_topology'])
    initial_spikes = {}
    return {
        'seed': seed,
        'archived_record_checksum': raw['archived_record_checksum'],
        'arms': {'growing': {
            mapping: verify_mapping(raw['mappings'][mapping], archive, mapping, initial, core,
                                    positions, excitatory, rules, initial_spikes, expected_boundaries)
            for mapping in MAPPINGS
        }},
    }


def analyze_directory(directory):
    directory = Path(directory)
    raw, archived = [
        {path.name: path for path in (directory / part).glob('*.json')
         if path.name != 'manifest.json'}
        for part in ('raw', 'input/structural')
    ]
    require(raw and raw.keys() == archived.keys(), 'decision and archived structural files differ')
    records = []
    for filename in sorted(raw):
        diagnostic = raw[filename].read_bytes()
        structural = archived[filename].read_bytes()
        row = analyze_record(json.loads(diagnostic), json.loads(structural))
        identity = re.fullmatch(r'v\d+-s(\d+)-i\d+\.json', filename)
        require(identity and int(identity[1]) == row['seed'], 'filename differs from seed identity')
        records.append({
            **row,
            'job_id': Path(filename).stem,
            'decision_sha256': hashlib.sha256(diagnostic).hexdigest(),
            'structural_sha256': hashlib.sha256(structural).hexdigest(),
        })
    require(len({row['seed'] for row in records}) == len(records), 'duplicate decision seed')
    return {'protocol': PROTOCOL, 'cohort_size': len(records),
            'records': sorted(records, key=lambda row: row['seed'])}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path)
    args = parser.parse_args()
    try:
        print(json.dumps(analyze_directory(args.directory), indent=2, allow_nan=False))
    except (OSError, ValueError, KeyError, TypeError, IndexError, OverflowError) as exc:
        parser.exit(1, f'contingency-structural-decisions: {exc}\n')


if __name__ == '__main__':
    main()
