#!/usr/bin/env python3
"""Derive fixed arrival timing interventions from independently checked archives."""
import argparse
from copy import deepcopy
import json
from pathlib import Path

from contingency_a_source_analysis import analyze_record, target_events
from contingency_a_source_plan import build_plan as source_build_plan, SEEDS
from contingency_eligibility_analysis import source_record
from verify_contingency import require

PROTOCOL = 'NERVA-Contingency-A-Timing-Plan/1'
HORIZON_US = 200000


def arrival(probe, edge, positive, at=None, latest_before=None):
    matches = [(index, row[0] - probe['start_us'])
               for index, row in enumerate(probe['arrivals'])
               if row[2] == edge and (row[3] > 0 if positive else row[3] < 0)
               and (at is None or row[0] - probe['start_us'] == at)
               and (latest_before is None or row[0] - probe['start_us'] < latest_before)]
    if latest_before is not None:
        require(matches, 'missing preceding positive arrival')
        latest = max(time for _, time in matches)
        matches = [row for row in matches if row[1] == latest]
    elif at is None:
        require(matches, 'missing signed arrival')
        first = min(time for _, time in matches)
        matches = [row for row in matches if row[1] == first]
    require(len(matches) == 1, 'timing arrival selection is not unique')
    return matches[0]


def build_plan(growth: bytes, transfer: bytes, source_plan: bytes, source_raw: bytes) -> dict:
    source, checksum = source_record(growth)
    prior_plan = source_build_plan(growth, transfer)
    require(json.loads(source_plan) == prior_plan, 'saved source plan differs from independent reconstruction')
    raw = json.loads(source_raw)
    checked = analyze_record(raw, source, checksum, prior_plan)
    seed, target = checked['seed'], checked['target_cell']
    carriers = {}

    def carrier(name, family, condition):
        probes = raw['native'][condition]['probes'] if family == 'native' else raw['grafts'][family][condition]['probes']
        row = dict(source_family=family, source_condition=condition,
                   probes=deepcopy(probes), arms={'reference': []})
        carriers[name] = row
        return row, probes['cue1']

    def add(row, name, edits):
        probe = row['probes']['cue1']
        edits = sorted([[index, time] for index, time in edits
                        if time != probe['arrivals'][index][0] - probe['start_us']])
        require(len({index for index, _ in edits}) == len(edits), 'repeated timing row')
        require(all(type(time) is int and 0 <= time < HORIZON_US for _, time in edits),
                'timing edit outside probe horizon')
        row['arms'][name] = edits

    native, probe = carrier('native', 'native', 'intact')
    if seed == 34313:
        spike = target_events(probe, target)[0]
        active = sorted(prior_plan['active_edges'])
        require(len(active) == 3, 'expected three active support edges')
        selected = [arrival(probe, edge, True, at=spike) for edge in active]
        add(native, 'exc_spread', [(index, time + shift) for (index, time), shift in zip(selected, (-4000, 0, 4000))])
        add(native, 'exc_translate', [(index, time + 4000) for index, time in selected])
    else:
        require(seed == 34327, 'unsupported timing seed')
        index, time = arrival(probe, 1440, False)
        disinhibited = target_events(raw['native']['block_1440']['probes']['cue1'], target)
        require(disinhibited, 'missing disinhibited target spike')
        delayed = disinhibited[0] + 1000
        displacement = delayed - time
        require(displacement > 0, 'inhibitory displacement must be positive')
        add(native, 'inh_delay', [(index, delayed)])
        add(native, 'inh_advance', [(index, time - displacement)])
        grown, grown_probe = carrier('grown', 'native_initial', 'intact')
        rotated, rotated_probe = carrier('rotated', 'native_initial', 'cycle_full')
        events = target_events(rotated_probe, target)
        require(events, 'missing rotated target spike')
        anchor = events[0]
        preceding_index, preceding = arrival(grown_probe, 1632, True, latest_before=anchor)
        for name, shift in [('exc_align', 0), ('exc_left', -1000), ('exc_right', 1000)]:
            add(grown, name, [(preceding_index, anchor + shift)])
        pair = [arrival(rotated_probe, edge, True, at=anchor) for edge in (1542, 1632)]
        add(rotated, 'exc_separate', [(pair[1][0], preceding)])
        add(rotated, 'exc_translate_pair', [(index, preceding) for index, _ in pair])
    return dict(protocol=PROTOCOL, seed=seed, archived_record_checksum=checksum,
                target_cell=target, source_plan=deepcopy(prior_plan), carriers=carriers)


def plan_directory(directory, output):
    directory, output = Path(directory), Path(output)
    parts = ('growth_input', 'transfer_input', 'plans', 'raw')
    files = {part: {p.name: p for p in (directory / part).glob('*.json') if p.name != 'manifest.json'} for part in parts}
    require(files['raw'] and all(rows.keys() == files['raw'].keys() for rows in files.values()),
            'source input, plan and raw file identities differ')
    pending = []
    for name in sorted(files['raw']):
        data = [files[part][name].read_bytes() for part in parts]
        source, _ = source_record(data[0])
        require(name == f"{source['job']['id']}.json", 'source filename differs from archive job')
        pending.append((name, build_plan(*data)))
    require(len(pending) == len(SEEDS) and {row['seed'] for _, row in pending} == set(SEEDS),
            'timing cohort differs from two archived A hits')
    output.mkdir(parents=True, exist_ok=True)
    for name, row in pending:
        (output / name).write_text(json.dumps(row, indent=2, allow_nan=False) + '\n')
    return [name for name, _ in pending]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    try:
        plan_directory(args.directory, args.output)
    except (OSError, ValueError, KeyError, TypeError, IndexError, OverflowError) as exc:
        parser.exit(1, f'contingency-a-timing-plan: {exc}\n')


if __name__ == '__main__':
    main()
