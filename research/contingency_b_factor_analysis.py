#!/usr/bin/env python3
"""Independently verify archived native B factor interventions and case gates."""

import argparse
from collections import Counter
from copy import deepcopy
import hashlib
import json
from pathlib import Path

from contingency_capacity_analysis import natural
from contingency_live_rate_analysis import Assay, CUES, MAPPINGS, analyze_record as analyze_live
from contingency_live_rate_analysis import gate_rows, source_record
from contingency_structural_analysis import read_probe, topology
from verify_contingency import require

PROTOCOL = 'NERVA-Contingency-B-Factor/1'
CONDITIONS = ('intact', 'restore_old', 'block_new', 'restore_and_block',
              'block_terminal', 'block_nonterminal')
PROBES = ('blank', *CUES['b'])
ARMS = (*MAPPINGS, 'balanced')
COHORT = {34211, 34223, 34237, 34253}


def plans(stages, targets):
    """Derive each intervention entirely from the archived A and B endpoints."""
    a_rows = {row[0]: row for row in stages['a']['end_topology']}
    b_rows = {row[0]: row for row in stages['b']['end_topology']}
    a, b = topology(list(a_rows.values())), topology(list(b_rows.values()))
    born = set(b) - set(a)
    require(all(b[edge][3] for edge in born), 'disabled B-born endpoint')
    terminal = set().union(*targets)
    terminal_edges = {edge for edge in born if b[edge][1] in terminal}
    updates = [[edge, b_rows[edge][3], a_rows[edge][3]] for edge in sorted(a.keys() & b.keys())
               if a[edge][3] and b[edge][3] and a_rows[edge][3] != b_rows[edge][3]]
    result = {}
    for condition in CONDITIONS:
        blocked = (born if condition in ('block_new', 'restore_and_block') else
                   terminal_edges if condition == 'block_terminal' else
                   born - terminal_edges if condition == 'block_nonterminal' else set())
        changes = updates if condition in ('restore_old', 'restore_and_block') else []
        rows = deepcopy(list(b_rows.values()))
        replacements = {edge: old for edge, _, old in changes}
        for row in rows:
            row[3] = replacements.get(row[0], row[3])
            if row[0] in blocked:
                row[5] = False
        result[condition] = dict(weight_updates=changes, blocked_edges=sorted(blocked),
                                 published_topology=rows)
    return result


def contrasts(values):
    """Raw count contrasts; a positive decrement means intact activity was lost."""
    intact, restored = values['intact'], values['restore_old']
    blocked, both = values['block_new'], values['restore_and_block']
    return dict(restore_with_new=intact - restored, restore_without_new=blocked - both,
                block_with_b_weights=intact - blocked, block_with_a_weights=restored - both,
                interaction=intact - restored - blocked + both,
                block_terminal=intact - values['block_terminal'],
                block_nonterminal=intact - values['block_nonterminal'])


def analyze_record(raw, source, checksum):
    archived = analyze_live(source)
    details, config = source['details'], source['job']['effective_config']
    assay = Assay(details, config, source['job']['seed'])
    start = assay.checkpoint + 2 * assay.duration + assay.rest
    require(raw.keys() == {'protocol', 'seed', 'archived_record_checksum',
                           'full_replay_checked', 'probe_start_us', 'mappings'}
            and raw['protocol'] == PROTOCOL and raw['full_replay_checked'] is True,
            'unexpected B factor record')
    require(type(raw['seed']) is int and raw['seed'] == assay.seed
            and raw['archived_record_checksum'] == checksum
            and type(raw['probe_start_us']) is int and raw['probe_start_us'] == start,
            'B factor archive identity or probe clock differs')
    require(raw['mappings'].keys() == set(ARMS), 'unexpected B factor mappings')
    native = details['branches']['native']
    probes = {condition: {} for condition in CONDITIONS}
    receipts = {}
    for mapping in ARMS:
        row = raw['mappings'][mapping]
        require(row.keys() == {'conditions'} and row['conditions'].keys() == set(CONDITIONS),
                'unexpected B factor condition layout')
        expected = plans(native[mapping], assay.targets['b'])
        receipts[mapping] = {}
        for condition in CONDITIONS:
            arm = row['conditions'][condition]
            require(arm.keys() == {'weight_updates', 'blocked_edges', 'published_topology',
                                    'probes', 'cancelled_arrivals'}, 'unexpected factor arm fields')
            for field, value in expected[condition].items():
                require(arm[field] == value, f'{mapping}/{condition}: {field} differs')
            require(arm['probes'].keys() == set(PROBES)
                    and arm['cancelled_arrivals'].keys() == set(PROBES), 'unexpected factor probes')
            live = {edge for edge, (*_, enabled) in topology(arm['published_topology']).items()
                    if enabled}
            for cue, probe in arm['probes'].items():
                cancelled = arm['cancelled_arrivals'][cue]
                require(natural(cancelled) and (arm['blocked_edges'] or cancelled == 0),
                        'invalid cancelled arrival receipt')
                baseline = native[mapping]['b']['probes'][cue]
                read_probe(probe, cue, assay, start, live, baseline)
                require(probe['rest_core_spikes'] == baseline['rest_core_spikes'],
                        'activity before the intervention differs from archived B rest')
                if condition == 'intact':
                    require(probe == baseline, 'intact probe differs from archived native B')
            probes[condition][mapping] = arm['probes']
            receipts[mapping][condition] = dict(
                weight_updates=len(arm['weight_updates']), blocked_edges=len(arm['blocked_edges']),
                cancelled_arrivals=arm['cancelled_arrivals'])
    gates = {}
    for condition in CONDITIONS:
        arms = {mapping: {'a': native[mapping]['a'], 'b': {'probes': probes[condition][mapping]}}
                for mapping in ARMS}
        gates[condition] = gate_rows(assay.reset, assay.targets['b'], arms, 'b', 'b')
    require(gates['intact'] == archived['checks']['native']['b_acquisition'],
            'intact B criteria differ from archive')
    counts, effects = {}, {}
    for mapping in ARMS:
        counts[mapping], effects[mapping] = {}, {}
        for cue in PROBES:
            counts[mapping][cue] = {
                condition: [sum(cell in group for cell, _ in probes[condition][mapping][cue]['core_spikes'])
                            for group in assay.targets['b']]
                for condition in CONDITIONS}
            effects[mapping][cue] = [contrasts({condition: values[index]
                                               for condition, values in counts[mapping][cue].items()})
                                    for index in range(2)]
    correct_effects = {mapping: {cue: contrasts({condition: gates[condition][mapping][cue]['correct']
                                                for condition in CONDITIONS})
                                for cue in CUES['b']} for mapping in MAPPINGS}
    return dict(job_id=source['job']['id'], seed=assay.seed, archived_record_checksum=checksum,
                target_sizes=list(map(len, assay.targets['b'])), probe_start_us=start,
                receipts=receipts, gates=gates, target_spike_counts=counts,
                target_contrasts=effects, correct_contrasts=correct_effects)


def analyze_directory(directory, smoke=False):
    directory = Path(directory)
    files = [{p.name: p for p in (directory / part).glob('*.json') if p.name != 'manifest.json'}
             for part in ('input', 'raw')]
    require(files[0] and files[0].keys() == files[1].keys(), 'B factor archive and raw files differ')
    records = []
    for name in sorted(files[0]):
        evidence, output = (paths[name].read_bytes() for paths in files)
        source, checksum = source_record(evidence)
        require(name == f"{source['job']['id']}.json", 'B factor filename differs from archived job')
        records.append({**analyze_record(json.loads(output), source, checksum),
                        'input_sha256': hashlib.sha256(evidence).hexdigest(),
                        'raw_sha256': hashlib.sha256(output).hexdigest()})
    require(len({row['seed'] for row in records}) == len(records), 'duplicate B factor seed')
    require(smoke or {row['seed'] for row in records} == COHORT,
            'B factor cohort differs from published four seeds (use --smoke for technical cohorts)')
    return dict(protocol=PROTOCOL, cohort_size=len(records),
                passed_case_histograms={condition: dict(Counter(
                    sum(case['passed'] for mapping in row['gates'][condition].values()
                        for case in mapping.values()) for row in records)) for condition in CONDITIONS},
                records=sorted(records, key=lambda row: row['seed']))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path)
    parser.add_argument('--smoke', action='store_true', help='allow a technical seed cohort')
    args = parser.parse_args()
    try:
        print(json.dumps(analyze_directory(args.directory, smoke=args.smoke), indent=2, allow_nan=False))
    except (OSError, ValueError, KeyError, TypeError, IndexError, OverflowError) as exc:
        parser.exit(1, f'contingency-b-factor: {exc}\n')


if __name__ == '__main__':
    main()
