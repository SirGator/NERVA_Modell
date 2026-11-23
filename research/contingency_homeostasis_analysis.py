#!/usr/bin/env python3
"""Verify and summarize Core homeostasis by inherited eligibility training."""

import argparse
from bisect import bisect_left, bisect_right
from collections import Counter, defaultdict
import hashlib
import json
from pathlib import Path

from contingency_capacity_analysis import Assay, MAPPINGS, natural
from contingency_eligibility_analysis import analyze_record as analyze_eligibility
from contingency_eligibility_analysis import number, source_record
from verify_contingency import FRAME_US, ORIGINAL, require

PROTOCOL = 'NERVA-Contingency-Core-Homeostasis/1'
ARMS = {
    'off_keep': (False, False),
    'off_clear': (False, True),
    'core_keep': (True, False),
    'core_clear': (True, True),
}
METRICS = ('source_spikes', 'nonblank_source_spikes', 'target_spikes', 'task_arrivals',
           'new_tag_events')


def core_state(rows, core):
    state = {}
    for cell, current, drive in rows:
        require(natural(cell) and cell in core and cell not in state,
                'invalid Core homeostasis state identity')
        state[cell] = (number(current), number(drive))
    require(state.keys() == core, 'Core homeostasis state is incomplete')
    return state


def homeostasis_chain(trial, initial, active, start, stop):
    state = dict(initial)
    events = trial['homeostasis_events']
    require(bool(events) == active, 'homeostasis update presence differs from arm')
    previous = start
    for time, cell, old_current, new_current, old_drive, new_drive in events:
        require(natural(time) and previous <= time <= stop and natural(cell) and cell in state,
                'homeostasis updated an unapproved cell or time')
        before, after = ((number(old_current), number(old_drive)),
                         (number(new_current), number(new_drive)))
        require(state[cell] == before and before != after, 'broken homeostasis update chain')
        state[cell] = after
        previous = time
    require(state == core_state(trial['final_core'], initial.keys()),
            'homeostasis final state differs from update chain')
    return len(events)


def training_spikes(frames):
    spikes = defaultdict(list)
    for frame in frames:
        for cell, time in frame['core_spikes']:
            spikes[cell].append(time)
    return {cell: sorted(times) for cell, times in spikes.items()}


def focus_activity(trial, focus, start, stop, window):
    spikes = training_spikes(trial['frames'])
    spike_times = {cell: set(times) for cell, times in spikes.items()}
    arrivals = defaultdict(list)
    emitted = defaultdict(set)
    previous = start
    for time, edge in trial['pre_arrivals']:
        require(natural(time) and previous <= time <= stop and natural(edge) and edge in focus,
                'invalid focus arrival time or identity')
        require(not arrivals[edge] or arrivals[edge][-1] < time, 'duplicate focus arrival')
        arrivals[edge].append(time)
        pre, _, delay = focus[edge]
        if time - delay >= start:
            require(time - delay in spike_times.get(pre, ()),
                    'task arrival has no source spike at its emission time')
            emitted[edge].add(time)
        previous = time

    new_pairs = defaultdict(list)
    seen = set()
    previous = start
    event_arrivals = {edge: set(times) for edge, times in arrivals.items()}
    for time, edge, phase, before, after in trial['tag_events']:
        require(natural(time) and previous <= time <= stop and natural(edge) and edge in focus
                and phase in ('pre', 'post') and (time, edge, phase) not in seen,
                'invalid focus tag event time, identity, or phase')
        seen.add((time, edge, phase))
        previous = time
        _, post, _ = focus[edge]
        delta = number(after) - number(before)
        if phase == 'pre':
            require(delta < 0 and time in event_arrivals.get(edge, ()),
                    'pre tag event has wrong sign or no arrival at event time')
            partners = spikes.get(post, ())
            index = bisect_left(partners, time) - 1
            task_generated = time in emitted[edge]
        else:
            require(delta > 0 and time in spike_times.get(post, ()),
                    'post tag event has wrong sign or no target spike at event time')
            partners = arrivals[edge]
            index = bisect_right(partners, time) - 1
            task_generated = index >= 0 and partners[index] in emitted[edge]
        if index >= 0:
            require(time - partners[index] <= window, 'tag partner is outside the STDP window')
            if task_generated:
                new_pairs[edge].append((time, phase, partners[index]))
        else:
            require(time - start < window, 'tag event has no partner in the STDP window')

    stimuli = [frame['stimulus'] for frame in trial['frames']]
    sources = {
        pre: Counter(stimuli[(time - start) // FRAME_US] for time in spikes.get(pre, ()))
        for pre, _, _ in focus.values()
    }
    return {
        edge: {
            'source_spikes': sources[pre].total(),
            'nonblank_source_spikes': sources[pre].total() - sources[pre]['blank'],
            'source_spikes_by_stimulus': dict(sorted(sources[pre].items())),
            'target_spikes': len(spikes.get(post, ())),
            'native_arrivals': len(arrivals[edge]),
            'task_arrivals': len(emitted[edge]),
            'new_tag_events': len(new_pairs[edge]),
            'new_tag_pairs': new_pairs[edge],
        }
        for edge, (pre, post, _) in focus.items()
    }


def contrasts(counts):
    keep = counts['core_keep'] - counts['off_keep']
    clear = counts['core_clear'] - counts['off_clear']
    return {'counts': counts, 'homeostasis_with_tags': keep,
            'homeostasis_after_clear': clear,
            'retention_without_homeostasis': counts['off_keep'] - counts['off_clear'],
            'retention_with_homeostasis': counts['core_keep'] - counts['core_clear'],
            'interaction': keep - clear}


def focus_effects(edges):
    result = {'focus_edges': len(edges)}
    for metric in METRICS:
        result[metric] = contrasts({
            arm: sum(edge['arms'][arm][metric] for edge in edges) for arm in ARMS
        })
    result['edges_with_new_tags'] = contrasts({
        arm: sum(edge['arms'][arm]['new_tag_events'] > 0 for edge in edges) for arm in ARMS
    })
    return result


def analyze_record(diagnostic, source, eligibility, checksum):
    analyze_eligibility(eligibility, source, checksum)
    job, details = source['job'], source['details']
    config, followup = job['effective_config'], details['followup']
    assay = Assay(details, config)
    require(diagnostic['protocol'] == PROTOCOL and diagnostic['replay_verified'] is True
            and diagnostic['seed'] == job['seed']
            and diagnostic['archived_record_checksum'] == checksum,
            'homeostasis replay does not identify the verified capacity followup')
    reset = {cue: assay.reset[cue] for cue in (*ORIGINAL, 'blank')}
    require(diagnostic['checkpoint_time_us'] == assay.checkpoint
            and diagnostic['topology'] == assay.topology
            and diagnostic['target_groups'] == details['target_groups']
            and diagnostic['core_ids'] == details['core_ids'] and diagnostic['reset'] == reset,
            'homeostasis checkpoint differs from archived capacity checkpoint')
    window = diagnostic['stdp_window_us']
    require(type(window) is int and window == 100_000, 'unexpected native STDP window')
    initial = core_state(diagnostic['initial_core'], assay.core)
    require(diagnostic['focus'].keys() == set(MAPPINGS), 'unexpected focus mappings')
    focus, baseline_spikes = {}, {}
    for mapping in MAPPINGS:
        archive = eligibility['mappings'][mapping]['focus']
        expected = [[row[key] for key in ('id', 'pre', 'post', 'delay_us')] for row in archive]
        require(diagnostic['focus'][mapping] == expected,
                'homeostasis focus differs from archived eligibility focus')
        focus[mapping] = {edge: (pre, post, delay) for edge, pre, post, delay in expected}
        baseline_spikes[mapping] = training_spikes(followup['training'][mapping]['frames'])

    require(diagnostic['arms'].keys() == ARMS.keys(), 'unexpected homeostasis arms')
    activity, recall, updates = {}, {}, {}
    for name, (active, cleared) in ARMS.items():
        arm = diagnostic['arms'][name]
        require(arm['homeostasis'] is active and arm['clear_eligibility'] is cleared,
                'homeostasis arm flags differ from the factorial design')
        installed = assay.training(arm['training'], job['seed'], config['training_repetitions'])
        activity[name], updates[name] = {}, {}
        for mapping in MAPPINGS:
            trial = arm['training'][mapping]
            native = followup['training'][mapping]
            live = eligibility['mappings'][mapping]['live']
            archive = live if cleared else native
            require(natural(trial['removed_tags'])
                    and trial['removed_tags'] == (live['removed_tags'] if cleared else 0),
                    'homeostasis arm removed a different number of checkpoint tags')
            if not active:
                require(trial['frames'] == archive['frames'] and trial['weights'] == archive['weights'],
                        'disabled homeostasis training differs from archived control')
            if name == 'off_keep':
                archived = eligibility['mappings'][mapping]['fixed_history']['pre_arrivals']
                require(sorted(trial['pre_arrivals']) == sorted(archived),
                        'disabled homeostasis arrivals differ from archived native history')
            require(all(frame['root_spikes'] == reference['root_spikes']
                        for frame, reference in zip(trial['frames'], native['frames'])),
                    'paired training differs in presented inputs')
            stop = assay.checkpoint + len(trial['frames']) * FRAME_US
            updates[name][mapping] = homeostasis_chain(trial, initial, active, assay.checkpoint, stop)
            activity[name][mapping] = focus_activity(
                trial, focus[mapping], assay.checkpoint, stop, window)
        if not active:
            observed = eligibility['probes']['live_clear'] if cleared else followup['native']
            require(arm['recall'] == observed, 'disabled homeostasis recall differs from archive')
        passed, _, checks = assay.comparison(arm['recall'], ORIGINAL, reset, installed)
        quiescence = {}
        for mapping in MAPPINGS:
            observed = arm['recall']['arms'][mapping]
            require(dict(observed['weights']) == installed[mapping],
                    'recall did not install the exact Core-only weights')
            responses = observed['responses']
            quiescence[mapping] = {
                'blank_core_spikes': len(responses['blank']['core_spikes']),
                'reset_blank_core_spikes': len(reset['blank']['core_spikes']),
                'cue_rest_core_spikes': {cue: responses[cue]['rest_core_spikes'] for cue in ORIGINAL},
                'reset_cue_rest_core_spikes': {cue: reset[cue]['rest_core_spikes'] for cue in ORIGINAL},
            }
        recall[name] = {'passed_cases': passed, 'checks': checks, 'quiescence': quiescence}

    edges = [
        {'mapping': mapping, 'id': edge, 'pre': pre, 'post': post, 'delay_us': delay,
         'previously_silent': pre not in baseline_spikes[mapping],
         'arms': {arm: activity[arm][mapping][edge] for arm in ARMS}}
        for mapping in MAPPINGS for edge, (pre, post, delay) in focus[mapping].items()
    ]
    silent = [edge for edge in edges if edge['previously_silent']]
    if job['seed'] == 34123:
        require(len(silent) == 8, 'primary seed lacks its eight archived silent-source focus edges')
    return {
        'job_id': job['id'], 'seed': job['seed'], 'archived_record_checksum': checksum,
        'focus_edges': edges, 'homeostasis_updates': updates, 'recall': recall,
        'effects': {
            'all_focus': focus_effects(edges),
            'previously_silent': focus_effects(silent),
            'recall_passed_cases': contrasts({arm: row['passed_cases'] for arm, row in recall.items()}),
        },
    }


def analyze_directory(directory):
    directory = Path(directory)
    raw, capacity, eligibility = [
        {path.name: path for path in (directory / part).glob('*.json')
         if path.name != 'manifest.json'}
        for part in ('raw', 'input/capacity', 'input/eligibility')
    ]
    require(raw and raw.keys() == capacity.keys() == eligibility.keys(),
            'homeostasis replay and archived control files differ')
    records = []
    for filename in sorted(raw):
        replay_bytes = raw[filename].read_bytes()
        capacity_bytes = capacity[filename].read_bytes()
        eligibility_bytes = eligibility[filename].read_bytes()
        source, checksum = source_record(capacity_bytes)
        require(filename == f"{source['job']['id']}.json", 'file name differs from capacity job identity')
        row = analyze_record(json.loads(replay_bytes), source, json.loads(eligibility_bytes), checksum)
        records.append({**row, 'capacity_sha256': hashlib.sha256(capacity_bytes).hexdigest(),
                        'eligibility_sha256': hashlib.sha256(eligibility_bytes).hexdigest(),
                        'diagnostic_sha256': hashlib.sha256(replay_bytes).hexdigest()})
    return {'protocol': PROTOCOL, 'cohort_size': len(records),
            'records': sorted(records, key=lambda row: row['seed'])}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path)
    args = parser.parse_args()
    try:
        print(json.dumps(analyze_directory(args.directory), indent=2, allow_nan=False))
    except (OSError, ValueError, KeyError, TypeError, IndexError, OverflowError) as exc:
        parser.exit(1, f'contingency-homeostasis: {exc}\n')


if __name__ == '__main__':
    main()
