#!/usr/bin/env python3
"""Independently verify native A necessity and common-carrier transfer receipts."""
import argparse
from collections import Counter
from copy import deepcopy
import hashlib
import json
import math
from pathlib import Path

from contingency_capacity_analysis import natural
from contingency_eligibility_analysis import number, source_record
from contingency_live_growth_analysis import SEEDS, analyze_record as analyze_growth
from contingency_live_rate_analysis import Assay, CUES, MAPPINGS, gate_rows
from contingency_structural_analysis import read_probe, topology
from verify_contingency import require

PROTOCOL = 'NERVA-Contingency-A-Transfer/1'
ARMS = (*MAPPINGS, 'balanced')
PROBES = ('blank', *CUES['a'])
NATIVE = ('intact', 'block_born', 'block_terminal', 'block_other', 'restore_old', 'reset_born')
TRANSFER = ('native_initial', 'native_trained', 'sham_initial', 'sham_trained',
            'old_only', 'joint', 'joint_sham')
GEOMETRY = [0.25, 0.25, 1000]
TOLERANCE = 0.025


def distance(positions, pre, post):
    x, y, z = [a - b for a, b in zip(positions[pre][0], positions[post][0])]
    return math.sqrt(x * x + y * y + z * z)


def sham_rows(initial, born, positions, radius=GEOMETRY[0], tolerance=TOLERANCE):
    """Greedily swap compatible source pairs, keeping each incoming slot intact."""
    rows = deepcopy(born)
    occupied = {(row[1], row[2]) for row in (*initial, *rows)}
    used, swaps = set(), []
    for i, left in enumerate(rows):
        if i in used:
            continue
        a, target_a = left[1:3]
        for j in range(i + 1, len(rows)):
            if j in used:
                continue
            right = rows[j]
            b, target_b = right[1:3]
            if a == b or target_a == target_b or positions[a][1] != positions[b][1]:
                continue
            if b == target_a or a == target_b or (b, target_a) in occupied or (a, target_b) in occupied:
                continue
            old_a, old_b = distance(positions, a, target_a), distance(positions, b, target_b)
            new_a, new_b = distance(positions, b, target_a), distance(positions, a, target_b)
            if (new_a > radius or new_b > radius or abs(new_a - old_a) > tolerance
                    or abs(new_b - old_b) > tolerance):
                continue
            occupied.remove((a, target_a))
            occupied.remove((b, target_b))
            occupied.update(((b, target_a), (a, target_b)))
            left[1], right[1] = b, a
            used.update((i, j))
            swaps.append([left[0], right[0]])
            break
    return swaps, rows


def plans(initial, trained, targets, positions):
    """Reconstruct all edits from archive endpoints; receipts are never trusted."""
    before, after = topology(initial), topology(trained)
    old = {row[0]: row for row in initial}
    born = [deepcopy(row) for row in trained if row[0] not in before]
    require(all(row[5] and row[4] == GEOMETRY[2] and row[1] != row[2]
                and row[1] in positions and row[2] in positions
                and distance(positions, row[1], row[2]) <= GEOMETRY[0] for row in born),
            'invalid archived A birth geometry')
    changed = [[row[0], row[3], old[row[0]][3]] for row in trained
               if row[0] in before and before[row[0]][3] and after[row[0]][3]
               and row[3] != old[row[0]][3]]
    learned = [[edge, reset, current] for edge, current, reset in changed]
    resets = [[row[0], row[3], GEOMETRY[1]] for row in born if row[3] != GEOMETRY[1]]
    terminal = set().union(*targets)
    native = {}
    for name in NATIVE:
        blocked = [row[0] for row in born if name == 'block_born'
                   or (name == 'block_terminal' and row[2] in terminal)
                   or (name == 'block_other' and row[2] not in terminal)]
        updates = changed if name == 'restore_old' else resets if name == 'reset_born' else []
        native[name] = patch(trained, updates, blocked, [])
    swaps, sham = sham_rows(initial, born, positions)
    transfer = {}
    for name in TRANSFER:
        insert = deepcopy(sham if 'sham' in name else born) if name != 'old_only' else []
        if name.endswith('initial'):
            for row in insert:
                row[3] = GEOMETRY[1]
        transfer[name] = patch(initial, learned if name in ('old_only', 'joint', 'joint_sham') else [],
                               [], insert)
    return native, transfer, swaps


def patch(base, updates, blocked, inserted):
    rows = deepcopy(base)
    weights = {edge: new for edge, _, new in updates}
    for row in rows:
        row[3] = weights.get(row[0], row[3])
        if row[0] in blocked:
            row[5] = False
    rows.extend(deepcopy(inserted))
    rows.sort(key=lambda row: row[0])
    topology(rows)
    return dict(weight_updates=deepcopy(updates), blocked_edges=blocked,
                inserted_edges=inserted, published_topology=rows)


def count(probe, group):
    return sum(cell in group for cell, _ in probe['core_spikes'])


def analyze_record(raw, source, checksum):
    archived = analyze_growth(source)
    details, config = source['details'], source['job']['effective_config']
    assay = Assay(details, config, source['job']['seed'])
    require(raw.keys() == {'protocol', 'seed', 'archived_record_checksum', 'full_replay_checked',
                           'formation_geometry', 'sham_max_distance_change', 'positions',
                           'reset_probes', 'mappings'} and raw['protocol'] == PROTOCOL
            and raw['full_replay_checked'] is True, 'unexpected A transfer record')
    require(type(raw['seed']) is int and raw['seed'] == assay.seed
            and raw['archived_record_checksum'] == checksum, 'A transfer archive identity differs')
    require(raw['formation_geometry'] == GEOMETRY and raw['sham_max_distance_change'] == TOLERANCE,
            'A transfer formation geometry differs')
    positions = {}
    require([row[0] for row in raw['positions']] == sorted(assay.core),
            'position IDs differ from archived Core')
    for cell, xyz, polarity in raw['positions']:
        require(len(xyz) == 3 and all(math.isfinite(number(value)) for value in xyz)
                and type(polarity) is bool, 'invalid Core position or polarity')
        positions[cell] = xyz, polarity
    require(raw['reset_probes'] == {cue: assay.reset[cue] for cue in PROBES},
            'common carrier reset differs from archive')
    require(raw['mappings'].keys() == set(ARMS), 'unexpected A transfer mappings')
    probes = {kind: {name: {} for name in conditions}
              for kind, conditions in (('native', NATIVE), ('transfer', TRANSFER))}
    receipts = {}
    for mapping in ARMS:
        row = raw['mappings'][mapping]
        require(row.keys() == {'swaps', 'native', 'transfer'}, 'unexpected A mapping fields')
        stage = details['stages_a'][mapping]
        native, transfer, swaps = plans(details['initial_topology'], stage['end_topology'],
                                        assay.targets['a'], positions)
        require(row['swaps'] == swaps, 'sham source swaps differ from deterministic reconstruction')
        receipts[mapping] = {'swaps': swaps}
        for kind, expected in (('native', native), ('transfer', transfer)):
            require(row[kind].keys() == expected.keys(), 'unexpected A intervention conditions')
            receipts[mapping][kind] = {}
            for name, plan in expected.items():
                arm = row[kind][name]
                require(arm.keys() == {*plan, 'probes', 'cancelled_arrivals'}, 'unexpected A arm fields')
                for field, value in plan.items():
                    require(arm[field] == value, f'{mapping}/{kind}/{name}: {field} differs')
                require(natural(arm['cancelled_arrivals'])
                        and (plan['blocked_edges'] or arm['cancelled_arrivals'] == 0),
                        'invalid cancelled arrival receipt')
                require(arm['probes'].keys() == set(PROBES), 'unexpected A probe names')
                live = {edge for edge, (*_, enabled) in topology(arm['published_topology']).items() if enabled}
                for cue, probe in arm['probes'].items():
                    baseline = stage['probes'][cue] if kind == 'native' else assay.reset[cue]
                    start = assay.checkpoint + assay.rest + (assay.duration if kind == 'native' else 0)
                    require(type(probe['start_us']) is int, 'invalid probe clock type')
                    read_probe(probe, cue, assay, start, live, baseline)
                    require(probe['rest_core_spikes'] == baseline['rest_core_spikes'],
                            'pre-intervention rest differs from archive')
                    if kind == 'native' and name == 'intact':
                        require(probe == baseline, 'intact A probe differs from archive')
                probes[kind][name][mapping] = arm['probes']
                receipts[mapping][kind][name] = dict(
                    weight_updates=len(plan['weight_updates']), blocked_edges=len(plan['blocked_edges']),
                    inserted_edges=len(plan['inserted_edges']), cancelled_arrivals=arm['cancelled_arrivals'])
    gates = {kind: {name: gate_rows(assay.reset, assay.targets['a'],
              {mapping: {'a': {'probes': arms[mapping]}} for mapping in ARMS}, 'a', 'a')
              for name, arms in conditions.items()} for kind, conditions in probes.items()}
    require(gates['native']['intact'] == archived['checks']['a_acquisition'],
            'intact A gates differ from archive')
    native_effects, transfer_effects = {}, {}
    for flip, mapping in enumerate(MAPPINGS):
        native_effects[mapping], transfer_effects[mapping] = {}, {}
        for index, cue in enumerate(CUES['a']):
            target = assay.targets['a'][index ^ flip]
            native_counts = {name: count(arms[mapping][cue], target) for name, arms in probes['native'].items()}
            native_effects[mapping][cue] = dict(archived_hit=gates['native']['intact'][mapping][cue]['passed'],
                correct_counts=native_counts,
                decrements={name: native_counts['intact'] - value for name, value in native_counts.items() if name != 'intact'})
            transfer_effects[mapping][cue] = {}
            for name, sham in (('native_initial', 'sham_initial'), ('native_trained', 'sham_trained'), ('joint', 'joint_sham')):
                correct = gates['transfer'][name][mapping][cue]['correct']
                control = count(probes['transfer'][sham][mapping][cue], target)
                eligible = bool(receipts[mapping]['swaps']) and gates['transfer'][name][mapping][cue]['eligible']
                transfer_effects[mapping][cue][name] = dict(correct=correct, sham=control,
                    sham_corrected=correct-control, eligible=eligible,
                    geometry_specific=eligible and gates['transfer'][name][mapping][cue]['passed'] and correct > control)
    return dict(job_id=source['job']['id'], seed=assay.seed, archived_record_checksum=checksum,
                target_sizes=list(map(len, assay.targets['a'])), receipts=receipts, gates=gates,
                native_contrasts=native_effects, geometry_contrasts=transfer_effects)


def aggregate(records):
    result = {}
    for kind, conditions in (('native', NATIVE), ('transfer', TRANSFER)):
        result[kind] = {}
        for name in conditions:
            counts = [sum(case['passed'] for cues in row['gates'][kind][name].values()
                          for case in cues.values()) for row in records]
            result[kind][name] = dict(eligible_cases=sum(case['eligible'] for row in records
                for cues in row['gates'][kind][name].values() for case in cues.values()),
                passed_cases=sum(counts), full_seed_passes=counts.count(4),
                partial_seed_passes=sum(0 < value < 4 for value in counts),
                passed_case_histogram=dict(Counter(counts)))
    result['geometry_specific'] = {}
    for name in ('native_initial', 'native_trained', 'joint'):
        counts = [sum(case[name]['geometry_specific'] for cues in row['geometry_contrasts'].values()
                      for case in cues.values()) for row in records]
        result['geometry_specific'][name] = dict(
            eligible_cases=sum(case[name]['eligible'] for row in records
                for cues in row['geometry_contrasts'].values() for case in cues.values()),
            passed_cases=sum(counts), full_seed_passes=counts.count(4),
            partial_seed_passes=sum(0 < value < 4 for value in counts))
    result['archived_hit_cases'] = [dict(seed=row['seed'], mapping=mapping, cue=cue, **case)
        for row in records for mapping, cues in row['native_contrasts'].items()
        for cue, case in cues.items() if case['archived_hit']]
    return result


def analyze_directory(directory, smoke=False):
    directory = Path(directory)
    files = [{p.name: p for p in (directory / part).glob('*.json') if p.name != 'manifest.json'}
             for part in ('input', 'raw')]
    require(files[0] and files[0].keys() == files[1].keys(), 'A transfer archive and raw files differ')
    records = []
    for name in sorted(files[0]):
        evidence, output = (paths[name].read_bytes() for paths in files)
        source, checksum = source_record(evidence)
        require(name == f"{source['job']['id']}.json", 'A transfer filename differs from archived job')
        fixed = {'processing_neurons': 256, 'training_repetitions': 8, 'rest_us': 11_000_000}
        require(smoke or all(source['job']['effective_config'][key] == value for key, value in fixed.items()),
                'A transfer archive differs from prospective workload')
        records.append({**analyze_record(json.loads(output), source, checksum),
                        'input_sha256': hashlib.sha256(evidence).hexdigest(),
                        'raw_sha256': hashlib.sha256(output).hexdigest()})
    require(len({row['seed'] for row in records}) == len(records), 'duplicate A transfer seed')
    require(smoke or {row['seed'] for row in records} == set(SEEDS),
            'A transfer cohort differs from published six seeds (use --smoke for technical cohorts)')
    return dict(protocol=PROTOCOL, cohort_size=len(records), summary=aggregate(records),
                records=sorted(records, key=lambda row: row['seed']))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path)
    parser.add_argument('--smoke', action='store_true')
    args = parser.parse_args()
    try:
        print(json.dumps(analyze_directory(args.directory, args.smoke), indent=2, allow_nan=False))
    except (OSError, ValueError, KeyError, TypeError, IndexError, OverflowError) as exc:
        parser.exit(1, f'contingency-a-transfer: {exc}\n')


if __name__ == '__main__':
    main()
