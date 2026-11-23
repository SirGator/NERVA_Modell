#!/usr/bin/env python3
"""Verify frozen A-hit source interventions and fixed-latency electrical contrasts."""
import argparse
from copy import deepcopy
import hashlib
import json
import math
from pathlib import Path

from contingency_a_transfer_analysis import PROBES, patch
from contingency_capacity_analysis import natural
from contingency_electrical_analysis import read_cue
from contingency_eligibility_analysis import source_record
from contingency_live_rate_analysis import Assay
from contingency_structural_analysis import read_probe, topology
from verify_contingency import require

PROTOCOL = 'NERVA-Contingency-A-Source/1'
SEEDS = {34313, 34327}


def intervention_plans(source, plan):
    """Reconstruct every edit from archived endpoint and independently rebuilt plan."""
    initial = source['details']['initial_topology']
    trained = source['details']['stages_a'][plan['mapping']]['end_topology']
    old = {row[0]: row for row in initial}
    active, silent = plan['active_edges'], plan['silent_edges']
    reset = [[edge, weight, old[edge][3] if edge in old else .25]
             for edge, _, _, weight, _, enabled in trained if enabled
             and (edge not in old or old[edge][5])
             and weight != (old[edge][3] if edge in old else .25)]
    native = {}
    for name, blocked, updates in [('intact', [], []), ('block_active', active, []),
            ('block_silent', silent, []), ('reset_all_weights', [], reset),
            *((f'block_{edge}', [edge], []) for edge in active)]:
        row = patch(trained, updates, list(blocked), [])
        row.pop('inserted_edges')
        native[name] = row
    grafts = {}
    for name, graft in plan['grafts'].items():
        base = sorted(deepcopy(initial) + deepcopy(graft['inserted_edges']), key=lambda row: row[0])
        slots = {row[0]: row[1] for row in base}
        cycle = graft['cycle']
        rotated = [[edge, slots[cycle[(index + 1) % len(cycle)]]]
                   for index, edge in enumerate(cycle)]
        replacements = {'intact': [], 'cycle_terminal': rotated[:1],
            'cycle_collateral': rotated[1:], 'cycle_full': rotated,
            'replace_group': graft['group_replacements'], 'block_group': [], 'block_active': [],
            **{f'replace_{edge}': [[edge, cell]] for edge, cell in graft['single_replacements']}}
        grafts[name] = {}
        for arm, replace in replacements.items():
            blocked = ([edge for edge, _ in graft['group_replacements']] if arm == 'block_group'
                       else active if arm == 'block_active' else [])
            row = patch(base, [], list(blocked), [])
            row.pop('inserted_edges')
            new_sources = dict(replace)
            for edge in row['published_topology']:
                edge[1] = new_sources.get(edge[0], edge[1])
            topology(row['published_topology'])
            grafts[name][arm] = row
    return native, grafts


def validate_trace(probe, target, core, rows):
    """Check native f32 integration properties and available source emission times."""
    edges = {edge: (pre, post) for edge, pre, post, _, _, enabled in rows if enabled}
    read_cue(probe, {target}, core, edges)
    delays = {edge: delay for edge, _, _, _, delay, enabled in rows if enabled}
    spikes = {(cell, at) for cell, at in probe['core_spikes']}
    previous, seen = -1, set()
    for at, cell, edge, _ in probe['arrivals']:
        require(at >= previous and (at, edge) not in seen, 'unordered or repeated target arrival')
        previous = at
        seen.add((at, edge))
        emitted = at - delays[edge]
        pre = edges[edge][0]
        if pre in core and emitted >= probe['start_us']:
            require((pre, emitted) in spikes, 'target arrival lacks delayed Core source spike')
    require([row[0] for row in probe['boundaries']] == sorted(row[0] for row in probe['boundaries']),
            'unordered target boundaries')


def target_events(probe, target):
    return [at - probe['start_us'] for cell, at in probe['core_spikes'] if cell == target]


def contrast(probe, reference, target, latency, basis):
    events, before = target_events(probe, target), target_events(reference, target)
    doses = [(edge, amp) for at, cell, edge, amp in probe['arrivals']
             if cell == target and latency is not None and at - probe['start_us'] <= latency and amp > 0]
    by_edge = {}
    for edge, amp in doses:
        by_edge.setdefault(edge, []).append(amp)
    return dict(target_relative_spikes_us=events, reference_relative_spikes_us=before,
        same_target_events=events == before, target_spike_count_delta=len(events) - len(before),
        first_spike_latency_delta_us=(events[0] - before[0] if events and before else None),
        dose_latency_us=latency, latency_basis=basis,
        excitatory_arrivals_through_reference_latency=len(doses),
        excitatory_dose_through_reference_latency=math.fsum(amp for _, amp in doses),
        excitatory_dose_by_edge=[[edge, len(values), math.fsum(values)] for edge, values in sorted(by_edge.items())])


def analyze_record(raw, source, checksum, plan):
    assay = Assay(source['details'], source['job']['effective_config'], source['job']['seed'])
    require(raw.keys() == {'protocol', 'seed', 'archived_record_checksum', 'full_replay_checked',
            'target_cell', 'native', 'grafts'} and raw['protocol'] == PROTOCOL
            and raw['full_replay_checked'] is True, 'unexpected A source record')
    require(type(raw['seed']) is int and raw['seed'] == assay.seed
            and raw['archived_record_checksum'] == checksum
            and type(raw['target_cell']) is int and raw['target_cell'] == plan['target_cell'],
            'A source identity differs from archive or plan')
    native, grafts = intervention_plans(source, plan)
    require(raw['grafts'].keys() == grafts.keys(), 'unexpected source graft carriers')
    target = plan['target_cell']
    archived = source['details']['stages_a'][plan['mapping']]['probes']
    native_events = target_events(archived[plan['cue']], target)
    require(native_events, 'archived target hit has no spike')
    result = {}
    families = [('native', raw['native'], native, archived)] + [
        (name, raw['grafts'][name], arms, plan['grafts'][name]['reference_probes'])
        for name, arms in grafts.items()]
    for family, arms, expected, reference in families:
        require(arms.keys() == expected.keys(), 'unexpected source intervention arms')
        events = target_events(reference[plan['cue']], target)
        latency = events[0] if events else native_events[0]
        basis = 'carrier intact first target spike' if events else 'archived native first target spike (carrier intact has none)'
        result[family] = {}
        for name, edit in expected.items():
            arm = arms[name]
            require(arm.keys() == {*edit, 'cancelled_arrivals', 'probes'}, 'unexpected source arm fields')
            for key, value in edit.items():
                require(arm[key] == value, f'{family}/{name}: {key} differs')
            require(natural(arm['cancelled_arrivals']) and (edit['blocked_edges'] or arm['cancelled_arrivals'] == 0),
                    'invalid source cancelled arrival receipt')
            require(arm['probes'].keys() == set(PROBES), 'unexpected source probe names')
            live = {edge for edge, (*_, enabled) in topology(edit['published_topology']).items() if enabled}
            result[family][name] = dict(cancelled_arrivals=arm['cancelled_arrivals'], probes={})
            for cue, probe in arm['probes'].items():
                baseline = reference[cue]
                require(probe.keys() == {*baseline, 'arrivals', 'boundaries'}, 'unexpected source probe fields')
                require(type(probe['start_us']) is int and probe['start_us'] == baseline['start_us'],
                        'invalid source probe clock')
                read_probe(probe, cue, assay, baseline['start_us'], live, baseline)
                require(probe['rest_core_spikes'] == baseline['rest_core_spikes'], 'source rest differs from archive')
                if name == 'intact':
                    require({key: probe[key] for key in baseline} == baseline, 'source intact differs from archive')
                validate_trace(probe, target, assay.core, edit['published_topology'])
                result[family][name]['probes'][cue] = contrast(probe, arms['intact']['probes'][cue], target, latency, basis)
    for arms in result.values():
        for arm in arms.values():
            for cue, row in arm['probes'].items():
                row['excitatory_dose_delta'] = row['excitatory_dose_through_reference_latency'] - arms['intact']['probes'][cue]['excitatory_dose_through_reference_latency']
    return dict(job_id=source['job']['id'], seed=assay.seed, mapping=plan['mapping'], cue=plan['cue'],
                target_cell=target, active_edges=plan['active_edges'], silent_edges=plan['silent_edges'],
                archived_record_checksum=checksum, interventions=result)


def summarize(records):
    summary = {}
    for record in records:
        cue = record['cue']
        summary[record['seed']] = {}
        for family, arms in record['interventions'].items():
            summary[record['seed']][family] = {
                name: {key: arm['probes'][cue][key] for key in (
                    'target_relative_spikes_us', 'same_target_events', 'target_spike_count_delta',
                    'first_spike_latency_delta_us', 'excitatory_dose_delta')}
                for name, arm in arms.items()}
    return summary


def analyze_directory(directory, smoke=False):
    from contingency_a_source_plan import build_plan
    directory = Path(directory)
    names = ('growth_input', 'transfer_input', 'plans', 'raw')
    files = {part: {p.name: p for p in (directory / part).glob('*.json') if p.name != 'manifest.json'} for part in names}
    require(files['raw'] and all(rows.keys() == files['raw'].keys() for rows in files.values()),
            'source input, plan and raw file identities differ')
    records = []
    for name in sorted(files['raw']):
        data = {part: rows[name].read_bytes() for part, rows in files.items()}
        source, checksum = source_record(data['growth_input'])
        require(name == f"{source['job']['id']}.json", 'source filename differs from archive job')
        plan = build_plan(data['growth_input'], data['transfer_input'])
        require(json.loads(data['plans']) == plan, 'saved source plan differs from independent reconstruction')
        records.append({**analyze_record(json.loads(data['raw']), source, checksum, plan),
            **{f'{part}_sha256': hashlib.sha256(value).hexdigest() for part, value in data.items()}})
    require(len({row['seed'] for row in records}) == len(records), 'duplicate source seed')
    require(smoke or {row['seed'] for row in records} == SEEDS, 'source cohort differs from two archived A hits')
    return dict(protocol=PROTOCOL, cohort_size=len(records),
        dose_window='Relative time 0 through fixed focal-cue reference first spike, inclusive; positive arrivals only. All probes and interventions share their carrier reference latency.',
        summary=summarize(records), records=sorted(records, key=lambda row: row['seed']))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path)
    parser.add_argument('--smoke', action='store_true')
    args = parser.parse_args()
    try:
        print(json.dumps(analyze_directory(args.directory, args.smoke), indent=2, allow_nan=False))
    except (OSError, ValueError, KeyError, TypeError, IndexError, OverflowError) as exc:
        parser.exit(1, f'contingency-a-source: {exc}\n')


if __name__ == '__main__':
    main()
