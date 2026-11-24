#!/usr/bin/env python3
"""Independently verify signed arrival tapes and their native electrical responses."""
import argparse
from collections import defaultdict
from copy import deepcopy
import hashlib
import json
import math
from pathlib import Path

from contingency_a_source_analysis import intervention_plans, target_events
from contingency_capacity_analysis import natural
from contingency_electrical_analysis import number, read_cue
from contingency_eligibility_analysis import source_record
from contingency_live_rate_analysis import Assay
from contingency_structural_analysis import PROBES, read_probe, topology
from verify_contingency import FRAME_US, require

PROTOCOL = 'NERVA-Contingency-A-Timing/1'
SEEDS = {34313, 34327}
PARTS = ('growth_input', 'transfer_input', 'source_plans', 'source_input', 'plans', 'raw')


def issued_tape(probe, edits):
    """Rebuild exact signed tape, preserving archived row order at tied times."""
    changes = {}
    for index, time in edits:
        require(type(index) is int and 0 <= index < len(probe['arrivals'])
                and index not in changes and type(time) is int and 0 <= time < 4 * FRAME_US,
                'invalid timing row edit')
        changes[index] = time
    tape = [[changes.get(index, at - probe['start_us']), edge, amp]
            for index, (at, _, edge, amp) in enumerate(probe['arrivals'])]
    return sorted(tape, key=lambda row: row[0])


def tape_invariants(tape):
    """Window dose retains sign and multiplicity, including zero-valued slots."""
    values = defaultdict(list)
    for _, edge, amp in tape:
        values[edge].append(amp)
    return [[edge, len(amps), sum(amp > 0 for amp in amps), sum(amp < 0 for amp in amps),
             math.fsum(amp for amp in amps if amp > 0),
             math.fsum(amp for amp in amps if amp < 0), sorted(amps)]
            for edge, amps in sorted(values.items())]


def validate_probe(receipt, baseline, edits, rows, target, assay, cue, unchanged):
    require(receipt.keys() == {'disabled_edges', 'cancelled_arrivals', 'tape', 'response'},
            'unexpected timing probe receipt fields')
    edges = topology(rows)
    disabled = sorted(edge for edge, (_, post, _, enabled) in edges.items() if post == target and enabled)
    require(receipt['disabled_edges'] == disabled, 'timing disabled edges differ from carrier')
    require(natural(receipt['cancelled_arrivals']), 'invalid timing cancelled arrivals')
    tape = issued_tape(baseline, edits)
    require(receipt['tape'] == tape, 'issued timing tape differs from independent reconstruction')
    for time, edge, amp in receipt['tape']:
        require(type(time) is int and 0 <= time < 4 * FRAME_US and type(edge) is int
                and edge in disabled, 'invalid timing tape slot')
        number(amp)
    response = receipt['response']
    require(response.keys() == {*baseline, 'external_inputs'}, 'unexpected timing response fields')
    require(type(response['start_us']) is int and response['start_us'] == baseline['start_us'],
            'timing probe clock differs from archive')
    live = {edge for edge, (*_, enabled) in edges.items() if enabled} - set(disabled)
    read_probe(response, cue, assay, baseline['start_us'], live, baseline)
    require(response['rest_core_spikes'] == baseline['rest_core_spikes'],
            'timing clean no-op rest differs from carrier archive')
    require(response['arrivals'] == [], 'target native arrivals remain after substitution')
    external = [[response['start_us'] + time, target, amp] for time, _, amp in tape]
    require(response['external_inputs'] == external, 'logged external inputs differ from issued timing tape')
    # External inputs have no native arrival IDs. Only the electrical helper gets
    # their original slot labels, preserving receipt semantics in the raw record.
    virtual = deepcopy(response)
    virtual['arrivals'] = [[response['start_us'] + time, target, edge, amp] for time, edge, amp in tape]
    virtual['arrived_edges'] = sorted(set(response['arrived_edges']) | {edge for _, edge, amp in tape if amp != 0})
    read_cue(virtual, {target}, assay.core, {edge: (pre, post) for edge, pre, post, *_ in rows})
    require([row[0] for row in response['boundaries']] == sorted(row[0] for row in response['boundaries']),
            'unordered timing target boundaries')
    if unchanged:
        for key in baseline:
            expected = sorted(set(baseline[key]) - set(disabled)) if key == 'arrived_edges' else ([] if key == 'arrivals' else baseline[key])
            require(response[key] == expected, f'timing clean no-op {key} differs from carrier archive')
    events = target_events(response, target)
    before = target_events(baseline, target)
    return dict(target_relative_spikes_us=events, reference_relative_spikes_us=before,
                same_target_events=events == before, target_spike_count_delta=len(events) - len(before),
                first_spike_latency_delta_us=(events[0] - before[0] if events and before else None),
                cancelled_arrivals=receipt['cancelled_arrivals'], tape_rows=len(tape),
                window_amplitude_invariants=tape_invariants(tape),
                clean_noop_checked=unchanged)


def analyze_record(raw, source, checksum, plan):
    assay = Assay(source['details'], source['job']['effective_config'], source['job']['seed'])
    require(raw.keys() == {'protocol', 'seed', 'archived_record_checksum', 'target_cell',
            'full_replay_checked', 'carriers'} and raw['protocol'] == PROTOCOL
            and raw['full_replay_checked'] is True, 'unexpected timing record')
    require(type(raw['seed']) is int and raw['seed'] == assay.seed == plan['seed']
            and raw['archived_record_checksum'] == checksum == plan['archived_record_checksum']
            and type(raw['target_cell']) is int and raw['target_cell'] == plan['target_cell'],
            'timing identity differs from archive or plan')
    native, grafts = intervention_plans(source, plan['source_plan'])
    require(raw['carriers'].keys() == plan['carriers'].keys(), 'unexpected timing carriers')
    carriers = {}
    for name, carrier in plan['carriers'].items():
        actual = raw['carriers'][name]
        require(actual.keys() == {'pre_clamp_topology', 'arms'}, 'unexpected timing carrier fields')
        family, condition = carrier['source_family'], carrier['source_condition']
        rows = (native if family == 'native' else grafts[family])[condition]['published_topology']
        require(actual['pre_clamp_topology'] == rows, 'timing pre-clamp topology differs from source carrier')
        require(actual['arms'].keys() == carrier['arms'].keys(), 'unexpected timing arms')
        carriers[name] = {}
        for arm, edits in carrier['arms'].items():
            observed = actual['arms'][arm]
            require(observed.keys() == {'probes'} and observed['probes'].keys() == set(PROBES),
                    'unexpected timing arm probes')
            carriers[name][arm] = {'probes': {cue: validate_probe(observed['probes'][cue],
                carrier['probes'][cue], edits if cue == 'cue1' else [], rows, plan['target_cell'],
                assay, cue, not edits or cue != 'cue1') for cue in PROBES}}
            for cue in PROBES:
                require(carriers[name][arm]['probes'][cue]['window_amplitude_invariants'] ==
                        tape_invariants(issued_tape(carrier['probes'][cue], [])),
                        'timing arm changed signed per-edge window amplitude invariants')
    return dict(job_id=source['job']['id'], seed=assay.seed, target_cell=plan['target_cell'],
                archived_record_checksum=checksum, carriers=carriers)


def analyze_directory(directory):
    from contingency_a_timing_plan import build_plan
    directory = Path(directory)
    files = {part: {p.name: p for p in (directory / part).glob('*.json') if p.name != 'manifest.json'} for part in PARTS}
    require(files['raw'] and all(rows.keys() == files['raw'].keys() for rows in files.values()),
            'timing input, plan and raw file identities differ')
    records = []
    for name in sorted(files['raw']):
        data = {part: rows[name].read_bytes() for part, rows in files.items()}
        source, checksum = source_record(data['growth_input'])
        require(name == f"{source['job']['id']}.json", 'timing filename differs from archive job')
        plan = build_plan(*(data[part] for part in PARTS[:4]))
        require(json.loads(data['plans']) == plan, 'saved timing plan differs from independent reconstruction')
        records.append({**analyze_record(json.loads(data['raw']), source, checksum, plan),
                        **{f'{part}_sha256': hashlib.sha256(value).hexdigest() for part, value in data.items()}})
    require(len(records) == 2 and {row['seed'] for row in records} == SEEDS,
            'timing cohort differs from two archived A hits')
    return dict(protocol=PROTOCOL, cohort_size=len(records),
        clean_noop_interpretation='Every carrier reference and unchanged blank/cue0 control must reproduce its own archived full root/Core/rest response and all target boundaries; native target arrivals are replaced by the exact signed external tape and disabled IDs are removed from arrived_edges.',
        amplitude_invariant_columns=['edge', 'count', 'positive_count', 'negative_count', 'positive_sum', 'negative_sum', 'signed_amplitude_multiset'],
        summary={record['seed']: {name: {arm: values['probes']['cue1'] for arm, values in arms.items()}
                                 for name, arms in record['carriers'].items()} for record in records},
        records=sorted(records, key=lambda row: row['seed']))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path)
    args = parser.parse_args()
    try:
        print(json.dumps(analyze_directory(args.directory), indent=2, allow_nan=False))
    except (OSError, ValueError, KeyError, TypeError, IndexError, OverflowError) as exc:
        parser.exit(1, f'contingency-a-timing: {exc}\n')


if __name__ == '__main__':
    main()
