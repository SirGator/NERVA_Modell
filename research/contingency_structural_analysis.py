#!/usr/bin/env python3
"""Verify structural training, recruitment and acute recall interventions."""

import argparse
from collections import Counter, defaultdict
import hashlib
import json
from pathlib import Path

from contingency_capacity_analysis import Assay, MAPPINGS, analyze_record as analyze_capacity
from contingency_capacity_analysis import identity_set, natural
from contingency_electrical_analysis import canonical_sum, f32
from contingency_eligibility_analysis import number, source_record
from contingency_homeostasis_analysis import PROTOCOL as HOMEOSTASIS, focus_activity, training_spikes
from verify_contingency import FRAME_US, ORIGINAL, event_counts, require, training_sequence

PROTOCOL = 'NERVA-Contingency-Structural/1'
PROBES = ('blank', *ORIGINAL)
ARCHIVE_FIELDS = ('stimulus', 'start_us', 'root_spikes', 'core_spikes', 'weight_changes')
DEVELOPMENT_US = 100_000
FORMATION_DELAY_US = 1_000


def topology(rows):
    ids = [row[0] for row in rows]
    require(identity_set(ids, 'structural topology') and ids == sorted(ids),
            'invalid topology ID order')
    edges = {}
    for edge, pre, post, weight, delay, enabled in rows:
        require(natural(pre) and natural(post) and natural(delay) and delay > 0
                and 0 <= number(weight) <= 2 and type(enabled) is bool,
                'invalid structural topology entry')
        edges[edge] = (pre, post, delay, enabled)
    return edges


def lifecycles(initial, final, frames, core, growth):
    live = dict(initial)
    edges = dict(initial)
    births, deaths = {}, {}
    highest = max(edges)
    for frame in frames:
        end = frame['end_us']
        formed, pruned = frame['formed'], frame['pruned']
        require(not (formed or pruned) or (growth and end % DEVELOPMENT_US == 0),
                'topology changed outside a growing maintenance boundary')
        pairs = {(pre, post) for pre, post, _, _ in live.values()}
        for edge, pre, post in formed:
            require(natural(edge) and edge > highest
                    and natural(pre) and natural(post) and pre in core and post in core and pre != post
                    and (pre, post) not in pairs,
                    'invalid formation edge or endpoints')
            highest = edge
            pairs.add((pre, post))
            live[edge] = edges[edge] = (pre, post, FORMATION_DELAY_US, True)
            births[edge] = end
        for edge in pruned:
            require(natural(edge) and edge in live and births.get(edge, -1) < end
                    and set(live[edge][:2]) <= core, 'invalid pruning edge or endpoints')
            deaths[edge] = end
            del live[edge]
    require(live == final, 'topology ledger does not reconstruct the training endpoint')
    return edges, births, deaths


def read_training(trial, assay, mapping, seed, repetitions):
    frames = trial['frames']
    schedule = tuple(training_sequence(mapping, seed, repetitions, 0))
    require(len(frames) == len(schedule), 'training frame count differs from schedule')
    roots = Counter()
    for i, (frame, stimulus) in enumerate(zip(frames, schedule)):
        start = assay.checkpoint + i * FRAME_US
        end = start + FRAME_US - 1
        require(frame['stimulus'] == stimulus and frame['start_us'] == start
                and frame['end_us'] == end and natural(frame['weight_changes']),
                'training stimulus or time differs from schedule')
        current = assay.stimuli[stimulus]
        previous = assay.stimuli[schedule[i - 1]] if i else set()
        root_counts = event_counts(frame['root_spikes'], start, end + 1,
                                   ids=current | previous, label=f'{mapping}/{stimulus} root')
        require(current <= root_counts.keys()
                and all(channel in current or time == start for channel, time in frame['root_spikes']),
                'training root identities differ from schedule')
        roots.update(root_counts)
        event_counts(frame['core_spikes'], start, end + 1, ids=assay.core, label='training Core')
    require(natural(trial['removed_tags']) and natural(trial['homeostasis_changes'])
            and trial['homeostasis_changes'] > 0, 'invalid training maintenance record')
    return roots


def training_activity(trial, focus, quiet, edges, births, deaths, start, stop, window, core):
    spikes = training_spikes(trial['frames'])
    spike_times = {cell: set(times) for cell, times in spikes.items()}
    inputs, task_arrivals, new_arrivals = defaultdict(list), [], defaultdict(list)
    last_time = start
    seen = set()
    for time, edge, pre, post, amplitude in trial['arrivals']:
        require(natural(time) and last_time <= time < stop and natural(edge) and edge in edges
                and (edge in focus or post in quiet)
                and edges[edge][:2] == (pre, post) and edges[edge][3]
                and births.get(edge, -1) < time <= deaths.get(edge, stop)
                and (time, edge) not in seen,
                'arrival does not match an observed live edge')
        last_time = time
        seen.add((time, edge))
        amplitude = f32(number(amplitude))
        emit = time - edges[edge][2]
        if pre in core and emit >= start:
            require(emit in spike_times.get(pre, ()) and emit > births.get(edge, -1),
                    'arrival has no source spike after edge formation')
        if edge in focus:
            task_arrivals.append((time, edge))
        if post in quiet:
            inputs[time, post].append((edge, amplitude))
            if edge in births:
                new_arrivals[post].append((time, edge, amplitude))

    integrals = {}
    integral_counts = Counter()
    last_time = start
    for time, target, before, summed, threshold, refractory, spiked in trial['quiet_integrations']:
        key = time, target
        require(natural(time) and last_time <= time < stop and target in quiet and key not in integrals
                and natural(refractory) and type(spiked) is bool, 'invalid quiet-source integration')
        last_time = time
        before, summed, threshold = (f32(number(value)) for value in (before, summed, threshold))
        require(canonical_sum(amplitude for _, amplitude in inputs.get(key, ())) == summed, 'integration input differs from arrivals')
        candidate = f32(before + summed)
        require(spiked == (time >= refractory and candidate >= threshold)
                and spiked == (time in spike_times.get(target, ())),
                'integration differs from native candidate or spike ledger')
        integrals[key] = (before, summed, threshold, candidate)
        integral_counts[target] += 1
    require(inputs.keys() <= integrals.keys() and all(
        (time, cell) in integrals for cell in quiet for time in spikes.get(cell, ())),
        'quiet-source arrivals or spikes lack native integrations')
    require(all(natural(row[0]) and start <= row[0] < stop
                and row[0] <= deaths.get(row[1], stop) for row in trial['tag_events']),
            'tag event after edge pruning or outside training')
    terminal = focus_activity({**trial, 'pre_arrivals': task_arrivals}, focus, start, stop, window)

    sources = {}
    for cell in sorted(quiet):
        times = spikes.get(cell, ())
        incoming = new_arrivals[cell]
        first_spike = None
        if times:
            time = times[0]
            before, summed, threshold, candidate = integrals[time, cell]
            positive = [(edge, amplitude) for at, edge, amplitude in incoming
                        if at == time and amplitude > 0]
            positive_ids = {edge for edge, _ in positive}
            remaining = canonical_sum(amplitude for edge, amplitude in inputs.get((time, cell), ())
                                      if edge not in positive_ids)
            first_spike = {
                'time_us': time, 'potential_before': before, 'summed_input': summed,
                'threshold': threshold, 'candidate': candidate,
                'new_positive_arrivals': positive,
                'candidate_without_new_positive_input_in_observed_batch': f32(before + remaining),
            }
        sources[cell] = {
            'spikes': len(times),
            'new_input_arrivals': len(incoming),
            'new_positive_arrivals': sum(amplitude > 0 for _, _, amplitude in incoming),
            'new_input_edges': sorted({edge for _, edge, _ in incoming}),
            'new_positive_arrivals_before_first_spike': (None if not times else
                sum(at < times[0] and amplitude > 0 for at, _, amplitude in incoming)),
            'integrations': integral_counts[cell],
            'first_spike': first_spike,
        }
    return {'quiet_sources': sources, 'terminal_edges': terminal}, task_arrivals


def read_probe(probe, name, assay, start, live, baseline):
    require(probe['start_us'] == start and natural(probe['rest_core_spikes']),
            'invalid recall start or rest count')
    require(probe['root_spikes'] == [[channel, start + time - baseline['start_us']]
                                    for channel, time in baseline['root_spikes']],
            'recall root input differs from archived stimulus')
    require(identity_set(probe['arrived_edges'], 'recall arrival') <= live,
            'recall contains arrivals from unavailable edges')
    return event_counts(probe['core_spikes'], start, start + 4 * FRAME_US, ids=assay.core,
                        label='recall Core')


def matched(first, second):
    return all(first[key] == second[key] for key in ('start_us', 'root_spikes'))


def gate_rows(frozen, growing, targets, balanced=None):
    def spikes(probe, population):
        return sum(cell in population for cell, _ in probe['core_spikes'])

    result = {}
    for flip, mapping in enumerate(MAPPINGS):
        own, fixed = growing[mapping], frozen[mapping]
        opponent = growing[MAPPINGS[1 ^ flip]]
        quiet = len(own['blank']['core_spikes']) <= len(fixed['blank']['core_spikes'])
        result[mapping] = {}
        for i, cue in enumerate(ORIGINAL):
            target = targets[i ^ flip]
            correct = spikes(own[cue], target)
            wrong = spikes(own[cue], targets[1 ^ i ^ flip])
            baseline = spikes(fixed[cue], target)
            opposite = spikes(opponent[cue], target)
            control = None if balanced is None else spikes(balanced[cue], target)
            inputs = matched(own[cue], fixed[cue]) and matched(own[cue], opponent[cue]) and (
                balanced is None or matched(own[cue], balanced[cue]))
            passed = inputs and quiet and correct > max(wrong, baseline, opposite, control or 0) and (
                own[cue]['rest_core_spikes'] <= fixed[cue]['rest_core_spikes'])
            result[mapping][cue] = dict(correct=correct, incorrect=wrong, frozen=baseline,
                                        opposite=opposite, balanced=control, same_root_input=inputs,
                                        passed=passed)
    return result


def analyze_record(raw, source, homeostasis, checksum):
    analyze_capacity(source)
    job, details = source['job'], source['details']
    seed, config = job['seed'], job['effective_config']
    assay = Assay(details, config)
    followup = details['followup']
    for record, protocol in ((raw, PROTOCOL), (homeostasis, HOMEOSTASIS)):
        require(record['protocol'] == protocol and record['replay_verified'] is True
                and record['seed'] == seed and record['archived_record_checksum'] == checksum,
                'study replay does not identify the verified capacity source')
        require(record['checkpoint_time_us'] == assay.checkpoint
                and record['core_ids'] == details['core_ids']
                and record['target_groups'] == details['target_groups'],
                'study checkpoint identities differ from archived source')
    window = raw['stdp_window_us']
    require(type(window) is int and window == homeostasis['stdp_window_us'] == 100_000,
            'unexpected native STDP window')
    initial = topology(raw['initial_topology'])
    require(homeostasis['topology'] == assay.topology and all(edge[3] for edge in initial.values()) and
            [row[:4] for row in raw['initial_topology']] == [row[:4] for row in assay.topology],
            'initial topology differs from archived checkpoint')
    require(raw['focus'].keys() == raw['quiet_sources'].keys() == set(MAPPINGS)
            and raw['focus'] == homeostasis['focus']
            and raw['arms'].keys() == {'frozen', 'growing'}
            and all(arms.keys() == set(MAPPINGS) for arms in raw['arms'].values()),
            'structural training mappings or archived focus differ')

    repetitions = config['training_repetitions']
    start = assay.checkpoint
    stop = start + repetitions * 10 * FRAME_US
    recall_start = stop + config['rest_us']
    upstream = homeostasis['arms']['core_clear']
    require(upstream['homeostasis'] is True and upstream['clear_eligibility'] is True,
            'archived frozen training has unexpected rule settings')

    def read_arm(arm, mapping, growth, focus, quiet):
        trial = arm['training']
        roots = read_training(trial, assay, mapping, seed, repetitions)
        final = topology(trial['end_topology'])
        edges, births, deaths = lifecycles(initial, final, trial['frames'], assay.core, growth)
        activity, arrivals = training_activity(trial, focus, quiet, edges, births, deaths,
                                               start, stop, window, assay.core)
        live = {edge for edge, (*_, enabled) in final.items() if enabled}
        require(arm['probes'].keys() == set(PROBES), 'unexpected structural probes')
        for name, probe in arm['probes'].items():
            read_probe(probe, name, assay, recall_start, live, assay.reset[name])
        return {**activity, 'births': len(births), 'prunings': len(deaths),
                'surviving_new_edges': len(live & births.keys())}, roots, live, arrivals

    results = {'frozen': {}, 'growing': {}}
    surviving, training_roots = {}, {}
    for mapping in MAPPINGS:
        archive = upstream['training'][mapping]
        native = followup['training'][mapping]
        focus_ids = sorted({edge for row in followup['terminal_ablation']
                            if row['mapping'] == mapping for edge in row['restored_edges']})
        focus = {edge: initial[edge][:3] for edge in focus_ids}
        require(raw['focus'][mapping] == [[edge, *binding] for edge, binding in focus.items()],
                'focus edges differ from archived terminal ablation')
        quiet = {pre for pre, _, _ in focus.values()} - training_spikes(native['frames']).keys()
        require(raw['quiet_sources'][mapping] == sorted(quiet),
                'formerly silent source IDs differ from archived native training')

        for name in ('frozen', 'growing'):
            arm = raw['arms'][name][mapping]
            growth = name == 'growing'
            summary, training_roots[mapping], live, arrivals = read_arm(
                arm, mapping, growth, focus, quiet)
            results[name][mapping] = summary
            trial = arm['training']
            require(trial['removed_tags'] == archive['removed_tags']
                    and [frame['root_spikes'] for frame in trial['frames']] ==
                    [frame['root_spikes'] for frame in native['frames']],
                    'matched training differs in eligibility reset or presented root input')
            if growth:
                surviving[mapping] = (live, sorted(live - initial.keys()))
            else:
                require([{key: frame[key] for key in ARCHIVE_FIELDS} for frame in trial['frames']]
                        == archive['frames'] and
                        [[row[0], row[3]] for row in trial['end_topology']] == archive['weights']
                        and trial['homeostasis_changes'] == len(archive['homeostasis_events'])
                        and [list(row) for row in arrivals] == archive['pre_arrivals']
                        and trial['tag_events'] == archive['tag_events'],
                        'frozen training differs from archived Core-only homeostasis')

    frozen, growing = ({m: raw['arms'][name][m]['probes'] for m in MAPPINGS}
                       for name in ('frozen', 'growing'))
    preliminary = gate_rows(frozen, growing, assay.targets)
    require(raw['preliminary_gates'] == preliminary, 'preliminary recall gates differ')
    triggered = any(row['passed'] for cues in preliminary.values() for row in cues.values())
    balanced = raw['balanced']
    require((balanced is not None) == triggered, 'balanced training does not match preliminary trigger')
    balanced_summary = None
    if triggered:
        balanced_summary, roots, _, _ = read_arm(balanced, 'balanced', True, {}, set())
        require(all(roots == counts for counts in training_roots.values())
                and balanced['training']['removed_tags'] == upstream['training']['forward']['removed_tags'],
                'balanced training differs in total root input or eligibility reset')
    gates = gate_rows(frozen, growing, assay.targets, balanced['probes'] if triggered else None)
    require(raw['gates'] == gates, 'final recall gates differ')

    expected = [(mapping, cue) for mapping in MAPPINGS for cue in ORIGINAL
                if gates[mapping][cue]['passed'] and surviving[mapping][1]]
    require([(row['mapping'], row['cue']) for row in raw['ablations']] == expected,
            'acute ablations differ from eligible recall cases')
    ablations = []
    for row in raw['ablations']:
        mapping, cue = row['mapping'], row['cue']
        live, blocked = surviving[mapping]
        require(row['blocked_edges'] == blocked and natural(row['queued_arrivals_cancelled']),
                'acute recall did not block exactly the surviving new edges')
        probe = row['probe']
        counts = read_probe(probe, cue, assay, recall_start, live - set(blocked), assay.reset[cue])
        require(matched(probe, growing[mapping][cue]), 'acute ablation changed presented root input')
        target_index = ORIGINAL.index(cue) ^ MAPPINGS.index(mapping)
        correct, wrong = [sum(counts[cell] for cell in assay.targets[i])
                          for i in (target_index, 1 ^ target_index)]
        gate = gates[mapping][cue]
        ablations.append({'mapping': mapping, 'cue': cue, 'blocked_edges': len(blocked),
                          'queued_arrivals_cancelled': row['queued_arrivals_cancelled'],
                          'unblocked_correct': gate['correct'], 'blocked_correct': correct,
                          'blocked_incorrect': wrong,
                          'blocked_rest_core_spikes': probe['rest_core_spikes'],
                          'correct_decrement': gate['correct'] - correct,
                          'target_advantage_abolished': correct <= max(
                              wrong, gate['frozen'], gate['opposite'], gate['balanced'] or 0),
                          'correct_at_or_below_frozen': correct <= gate['frozen']})

    return {'job_id': job['id'], 'seed': seed, 'archived_record_checksum': checksum,
            'arms': results, 'balanced': balanced_summary, 'preliminary_gates': preliminary,
            'gates': gates, 'ablations': ablations}


def analyze_directory(directory):
    directory = Path(directory)
    raw, capacity, homeostasis = [
        {path.name: path for path in (directory / part).glob('*.json')
         if path.name != 'manifest.json'}
        for part in ('raw', 'input/capacity', 'input/homeostasis')
    ]
    require(raw and raw.keys() == capacity.keys() == homeostasis.keys(),
            'structural replay and archived input files differ')
    records = []
    for filename in sorted(raw):
        diagnostic = raw[filename].read_bytes()
        capacity_bytes = capacity[filename].read_bytes()
        homeostasis_bytes = homeostasis[filename].read_bytes()
        source, checksum = source_record(capacity_bytes)
        require(filename == f"{source['job']['id']}.json", 'file name differs from capacity job identity')
        row = analyze_record(json.loads(diagnostic), source, json.loads(homeostasis_bytes), checksum)
        records.append({**row, 'capacity_sha256': hashlib.sha256(capacity_bytes).hexdigest(),
                        'homeostasis_sha256': hashlib.sha256(homeostasis_bytes).hexdigest(),
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
        parser.exit(1, f'contingency-structural: {exc}\n')


if __name__ == '__main__':
    main()
