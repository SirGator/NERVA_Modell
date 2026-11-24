#!/usr/bin/env python3
"""Verify a reciprocal source intervention on the archived B responder."""

import argparse
import hashlib
import json
import math
from pathlib import Path

from contingency_a_source_analysis import target_events, validate_trace
from contingency_capacity_analysis import natural
from contingency_contrast_growth_analysis import POLICIES, analyze_record as verify_contrast, layout
from contingency_electrical_analysis import f32
from contingency_eligibility_analysis import number, source_record
from contingency_live_rate_analysis import Assay, CUES, MAPPINGS, gate_rows
from contingency_structural_analysis import read_probe, topology
from verify_contingency import TRAINING, require

PROTOCOL = 'NERVA-Contingency-B-Source/1'
SEED = 34531
TARGET = 93
SOURCE_PAIR = (87, 100)
CUE = 'cue_b0'
REFERENCE_LATENCY_US = 36_335
HISTORIES = ('balanced', 'forward')
PROBES = ('blank', *CUES['b'])
ARMS = ('intact', 'block', 'swap', 'sham')
SOURCE_CHECKSUM = '363a851d2d379cd6'
SOURCE_SHA256 = 'cee432505744ef4df9a783cdc21b899affd62055765e7138053dc468ae73f8f9'


def source_geometry(rows, core, radius):
    require(number(radius) > 0, 'invalid formation radius')
    require([row[0] for row in rows] == sorted(core) and all(natural(row[0]) for row in rows),
            'source geometry does not cover Core in identity order')
    result = {}
    for cell, coordinates, excitatory in rows:
        require(len(coordinates) == 3 and type(excitatory) is bool, 'invalid source geometry entry')
        result[cell] = tuple(map(number, coordinates)), excitatory
    return result


def distance(a, b):
    dx, dy, dz = (x - y for x, y in zip(a, b, strict=True))
    return math.sqrt(dx * dx + dy * dy + dz * dz)


def source_plan(stage, geometry, radius, mapping):
    rows = stage['end_topology']
    ends = {row[0]: row for row in rows}
    born = []
    for frame in stage['frames']:
        exploratory = {edge for edge, *_ in frame['exploratory']}
        born.extend((edge, pre, edge in exploratory) for edge, pre, post in frame['formed']
                    if post == TARGET)
    require(len(born) == 1, 'archive lacks exactly one B-born input to target')
    edge, pre, exploratory = born[0]
    require(exploratory == (mapping == 'forward'), 'B source birth class differs from archived mapping')
    original = ends[edge]
    require(original[:3] == [edge, pre, TARGET] and original[5]
            and pre in SOURCE_PAIR, 'unexpected archived B source or endpoint')
    swap = SOURCE_PAIR[1] if pre == SOURCE_PAIR[0] else SOURCE_PAIR[0]
    connected = {src for _, src, post, *_ in rows if post == TARGET}
    active = {cell for cell, _ in stage['probes'][CUE]['core_spikes']}
    target_position = geometry[TARGET][0]
    swap_distance = distance(geometry[swap][0], target_position)
    require(geometry[swap][1] and swap not in connected and swap_distance <= radius,
            'reciprocal source is unavailable for native formation')
    candidates = []
    for cell, (position, excitatory) in geometry.items():
        if not excitatory or cell in connected or cell in (swap, TARGET):
            continue
        target_distance = distance(position, target_position)
        if target_distance <= radius:
            candidates.append((cell not in active, abs(target_distance - swap_distance), cell))
    require(candidates, 'no local sham source is available')
    _, _, sham = min(candidates)
    return original, exploratory, swap, sham, sham in active


def expected_patch(stage, original, replacement, arm_name):
    blocked = [] if arm_name == 'intact' else [original[0]]
    rows = [[*row[:5], False] if row[0] in blocked else row for row in stage['end_topology']]
    new = [] if replacement is None else [
        [rows[-1][0] + 1, replacement, *original[2:5], True]]
    return dict(blocked_edges=blocked, inserted_edges=new, published_topology=rows + new)


def probe_metrics(probe, edge):
    start = probe['start_us']
    arrivals = [[at - start, amplitude] for at, _, edge_id, amplitude in probe['arrivals']
                if edge_id == edge]
    margins = [(at - start, f32(f32(voltage) + f32(summed)) - f32(threshold))
               for at, _, voltage, summed, threshold, _, refractory, _ in probe['boundaries']
               if at >= refractory]
    peak = max(margins, key=lambda item: item[1], default=(None, None))
    fixed_margin = next((margin for time, margin in margins if time == REFERENCE_LATENCY_US), None)
    return dict(target_spikes_us=target_events(probe, TARGET), tested_edge=edge,
                tested_edge_arrivals=arrivals, nonrefractory_peak_margin=peak[1],
                nonrefractory_peak_time_us=peak[0], reference_latency_margin=fixed_margin)


def read_branch(row, stage, assay, start, geometry, radius, mapping, label):
    original, exploratory, swap, sham, active = source_plan(stage, geometry, radius, mapping)
    require(row.keys() == {'original_edge', 'original_exploratory', 'swap_source', 'sham_source',
                           'sham_cue_active', 'arms'}
            and row['original_edge'] == original and row['original_exploratory'] is exploratory
            and natural(row['swap_source'])
            and row['swap_source'] == swap and natural(row['sham_source'])
            and row['sham_source'] == sham and row['sham_cue_active'] is active
            and row['arms'].keys() == set(ARMS), f'{label}: source plan differs')
    metrics = {}
    for name in ARMS:
        arm = row['arms'][name]
        replacement = {'swap': swap, 'sham': sham}.get(name)
        expected = expected_patch(stage, original, replacement, name)
        require(arm.keys() == {*expected, 'cancelled_arrivals', 'probes'},
                f'{label}/{name}: unexpected arm fields')
        for field, value in expected.items():
            require(arm[field] == value, f'{label}/{name}: {field} differs')
        cancelled = arm['cancelled_arrivals']
        require(natural(cancelled) and (name != 'intact' or cancelled == 0),
                f'{label}/{name}: invalid cancellation receipt')
        if name in ('swap', 'sham'):
            require(cancelled == row['arms']['block']['cancelled_arrivals'],
                    f'{label}/{name}: cancellation differs from block')
        enabled = {edge for edge, (*_, live) in topology(arm['published_topology']).items() if live}
        require(arm['probes'].keys() == set(PROBES), f'{label}/{name}: unexpected probe names')
        probes = {}
        for cue, probe in arm['probes'].items():
            baseline = stage['probes'][cue]
            require(probe.keys() == {*baseline, 'arrivals', 'boundaries'},
                    f'{label}/{name}/{cue}: unexpected probe fields')
            require(type(probe['start_us']) is int and probe['start_us'] == start,
                    f'{label}/{name}/{cue}: clock differs')
            read_probe(probe, cue, assay, start, enabled, baseline)
            require(probe['rest_core_spikes'] == baseline['rest_core_spikes'],
                    f'{label}/{name}/{cue}: activity before the intervention differs')
            if name == 'intact':
                require({key: probe[key] for key in baseline} == baseline,
                        f'{label}/{cue}: intact probe differs from archive')
            validate_trace(probe, TARGET, assay.core, expected['published_topology'])
            tested = expected['inserted_edges'][0][0] if replacement is not None else original[0]
            probes[cue] = probe_metrics(probe, tested)
        metrics[name] = dict(cancelled_arrivals=cancelled, probes=probes)
    return dict(original_edge=original, original_exploratory=exploratory, swap_source=swap,
                sham_source=sham, sham_cue_active=active, arms=metrics)


def gate_contrasts(assay, trial, archived, history):
    """Reassess B recall with fixed archived controls and with all conditions edited."""
    def evaluate(edit, mapping):
        stages = {}
        for control in TRAINING:
            probe = trial[control]['arms'][edit]['probes'] if mapping in (None, control) else (
                archived['b'][history][control]['probes'])
            stages[control] = {'a': archived['a'][history], 'b': {'probes': probe}}
        return gate_rows(assay.reset, assay.targets['b'], stages, 'b', 'b')

    fixed_controls = {mapping: {arm: evaluate(arm, mapping)[mapping] for arm in ARMS}
                      for mapping in MAPPINGS}
    common_arm = {arm: evaluate(arm, None) for arm in ARMS}
    return dict(fixed_controls=fixed_controls, common_arm=common_arm)


def summarize(branches, gates):
    summary = {}
    for a in HISTORIES:
        summary[a] = {}
        for b in TRAINING:
            summary[a][b] = {}
            for policy in POLICIES:
                row = branches[policy][a][b]
                summary[a][b][policy] = dict(
                    source=row['original_edge'][1], swap_source=row['swap_source'],
                    original_exploratory=row['original_exploratory'], sham_source=row['sham_source'],
                    sham_cue_active=row['sham_cue_active'],
                    **{arm: {**row['arms'][arm]['probes'][CUE],
                             'b_gate_passed_fixed_controls':
                                 gates[policy][a]['fixed_controls'][b][arm][CUE]['passed'] if b in MAPPINGS else None,
                             'b_gate_passed_common_arm':
                                 gates[policy][a]['common_arm'][arm][b][CUE]['passed'] if b in MAPPINGS else None}
                       for arm in ARMS})
    return summary


def paired_effects(summary):
    """Count branch-level changes within the fixed archived responder seed."""
    result = {}
    for b in TRAINING:
        rows = [summary[a][b][policy] for a in HISTORIES for policy in POLICIES]
        positive = [row for row in rows if row['intact']['target_spikes_us']]
        silent = [row for row in rows if not row['intact']['target_spikes_us']]
        result[b] = dict(
            intact_responsive=len(positive), intact_silent=len(silent),
            complete_losses={name: sum(not row[name]['target_spikes_us'] for row in positive)
                             for name in ARMS[1:]},
            gained_spikes={name: sum(bool(row[name]['target_spikes_us']) for row in silent)
                           for name in ARMS[1:]},
            swap_success_with_active_sham_silent=sum(
                row['sham_cue_active'] and bool(row['swap']['target_spikes_us'])
                and not row['sham']['target_spikes_us'] for row in silent))
    return result


def analyze_record(raw, source, checksum):
    require(source['job']['seed'] == SEED and checksum == SOURCE_CHECKSUM,
            'B source requires the fixed published contrast-growth record')
    details, config = source['details'], source['job']['effective_config']
    require([config[key] for key in ('processing_neurons', 'training_repetitions', 'rest_us')]
            == [256, 8, 11_000_000], 'B source archive differs from fixed workload')
    original = verify_contrast(source)
    assay = Assay(details, config, SEED, expected_stimuli=layout(SEED)[1])
    start = assay.checkpoint + 2 * assay.duration + assay.rest
    require(raw.keys() == {'protocol', 'seed', 'archived_record_checksum', 'target_cell', 'cue',
                          'formation_radius', 'positions', 'branches'}
            and raw['protocol'] == PROTOCOL and type(raw['seed']) is int and raw['seed'] == SEED
            and type(raw['target_cell']) is int and raw['target_cell'] == TARGET
            and raw['archived_record_checksum'] == checksum and raw['cue'] == CUE
            and TARGET in assay.targets['b'][0], 'unexpected B source record or target definition')
    radius = raw['formation_radius']
    geometry = source_geometry(raw['positions'], assay.core, radius)
    require(all(geometry[cell][1] for cell in SOURCE_PAIR), 'source pair differs in polarity')
    require(raw['branches'].keys() == set(POLICIES), 'unexpected B source policy layout')
    result, gates = {}, {}
    for policy, observed in raw['branches'].items():
        result[policy], gates[policy] = {}, {}
        archived = details['trials'][policy]
        require(observed.keys() == set(HISTORIES), 'unexpected B source A-history layout')
        for history, rows in observed.items():
            require(rows.keys() == set(TRAINING), 'unexpected B source mapping layout')
            result[policy][history] = {
                mapping: read_branch(row, archived['b'][history][mapping], assay, start, geometry,
                                     radius, mapping, f'{policy}/{history}/{mapping}')
                for mapping, row in rows.items()}
            scores = gate_contrasts(assay, rows, archived, history)
            require({mapping: arms['intact'] for mapping, arms in scores['fixed_controls'].items()}
                    == scores['common_arm']['intact']
                    == original['checks'][policy]['b_acquisition'][history],
                    f'{policy}/{history}: intact B checks differ from archive')
            gates[policy][history] = scores
    summary = summarize(result, gates)
    return dict(protocol=PROTOCOL, seed=SEED, archived_record_checksum=checksum,
                target_cell=TARGET, cue=CUE, reference_latency_us=REFERENCE_LATENCY_US,
                summary=summary, paired_effects=paired_effects(summary),
                branches=result, b_gates=gates)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('archived', type=Path)
    parser.add_argument('observed', type=Path)
    parser.add_argument('--replay', type=Path, help='check a second native execution byte for byte')
    args = parser.parse_args()
    try:
        archived, observed = (path.read_bytes() for path in (args.archived, args.observed))
        archived_sha256 = hashlib.sha256(archived).hexdigest()
        require(archived_sha256 == SOURCE_SHA256, 'archived B source SHA-256 differs')
        if args.replay is not None:
            require(args.replay.read_bytes() == observed, 'native B source replay differs')
        source, checksum = source_record(archived)
        result = analyze_record(json.loads(observed), source, checksum)
        observed_sha256 = hashlib.sha256(observed).hexdigest()
        print(json.dumps({**result, 'archived_sha256': archived_sha256,
                          'observed_sha256': observed_sha256,
                          'byte_replay_checked': args.replay is not None}, indent=2, allow_nan=False))
    except (OSError, ValueError, KeyError, TypeError, IndexError, OverflowError) as exc:
        parser.exit(1, f'contingency-b-source: {exc}\n')


if __name__ == '__main__':
    main()
