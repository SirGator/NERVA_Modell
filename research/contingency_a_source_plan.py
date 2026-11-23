#!/usr/bin/env python3
"""Derive fixed source interventions from independently checked A-transfer archives."""
import argparse
from collections import deque
from copy import deepcopy
import json
from pathlib import Path

from contingency_a_transfer_analysis import (GEOMETRY, TOLERANCE, analyze_record,
                                            distance, source_record)
from verify_contingency import require

PROTOCOL = 'NERVA-Contingency-A-Source-Plan/1'
SEEDS = (34313, 34327)
GRAFTS = ('native_initial', 'sham_initial')
MAX_CYCLE = 8


def source_options(initial, graft, positions, selected):
    """Return feasible replacements with their absolute length deviations."""
    occupied = {(row[1], row[2]) for row in (*initial, *graft)}
    options = {}
    for edge, pre, post, *_ in graft:
        if edge not in selected:
            continue
        old = distance(positions, pre, post)
        candidates = []
        for new in sorted(positions):
            if new == post or positions[new][1] != positions[pre][1] or (new, post) in occupied:
                continue
            length = distance(positions, new, post)
            delta = abs(length - old)
            if length <= GEOMETRY[0] and delta <= TOLERANCE:
                candidates.append((delta, new))
        options[edge] = sorted(candidates)
    require(set(options) == set(selected), 'selected support edge absent from graft')
    return options


def replacements(initial, graft, positions, selected):
    selected = sorted(selected)
    options = source_options(initial, graft, positions, selected)
    singles = [[edge, options[edge][0][1]] for edge in sorted(options) if options[edge]]
    best, best_key = [], None

    def visit(index, chosen, used, deviation):
        nonlocal best, best_key
        if index == len(selected):
            key = (-len(chosen), deviation, chosen)
            if best_key is None or key < best_key:
                best, best_key = deepcopy(chosen), key
            return
        edge = selected[index]
        visit(index + 1, chosen, used, deviation)
        for delta, new in options[edge]:
            if new not in used:
                visit(index + 1, chosen + [[edge, new]], used | {new}, deviation + delta)

    visit(0, [], set(), 0.)
    return singles, best


def shortest_cycle(initial, graft, positions, selected, target, max_length=MAX_CYCLE):
    """Rotate next-slot sources into current slots; every new pair must be initially free.

    Breadth-first path search retains path-local slot and endpoint-pair history so
    duplicate sources cannot create duplicate new edges in an otherwise simple cycle.
    All donor slots are A births; only the first slot may enter the selected target.
    """
    rows = {row[0]: row for row in graft}
    options = source_options(initial, graft, positions, rows)
    allowed = {edge: {source for _, source in choices} for edge, choices in options.items()}
    adjacency = {edge: [other for other in sorted(rows)
                        if other != edge and rows[other][1] in allowed[edge]] for edge in rows}
    cycles = []
    for start in sorted(selected):
        queue = deque([([start], set())])
        while queue:
            path, pairs = queue.popleft()
            current = path[-1]
            close = (rows[start][1], rows[current][2])
            if len(path) > 1 and rows[start][1] in allowed[current] and close not in pairs:
                cycles.append(path)
                break
            if len(path) >= max_length:
                continue
            for other in adjacency[current]:
                pair = (rows[other][1], rows[current][2])
                if other not in path and rows[other][2] != target and pair not in pairs:
                    queue.append((path + [other], pairs | {pair}))
    return min(cycles, key=lambda path: (len(path), path)) if cycles else []


def build_plan(growth_input: bytes, transfer_input: bytes) -> dict:
    source, checksum = source_record(growth_input)
    raw = json.loads(transfer_input)
    checked = analyze_record(raw, source, checksum)
    seed = checked['seed']
    require(seed in SEEDS, 'source plan only supports the fixed archived seeds')
    require(checked['native_contrasts']['reverse']['cue1']['archived_hit'],
            'selected archived reverse/cue1 case is not a hit')
    details = source['details']
    native = raw['mappings']['reverse']['native']['intact']['probes']['cue1']
    firing = {cell for cell, _ in native['core_spikes']} & set(details['targets']['a'][0])
    require(len(firing) == 1, 'selected native hit must have one firing target cell')
    target = next(iter(firing))
    initial = details['initial_topology']
    initial_ids = {row[0] for row in initial}
    born = [row for row in details['stages_a']['reverse']['end_topology'] if row[0] not in initial_ids]
    incoming = {row[0] for row in born if row[2] == target}
    active = sorted(incoming & set(native['arrived_edges']))
    silent = sorted(incoming - set(active))
    require(len(active) == 3 and len(silent) == 2, 'selected target support partition differs')
    positions = {cell: (xyz, polarity) for cell, xyz, polarity in raw['positions']}
    grafts = {}
    for name in GRAFTS:
        prior = raw['mappings']['reverse']['transfer'][name]
        graft = prior['inserted_edges']
        singles, group = replacements(initial, graft, positions, active)
        cycle = shortest_cycle(initial, graft, positions, active, target)
        require(cycle and len(singles) == len(active), 'selected source interventions are infeasible')
        grafts[name] = dict(inserted_edges=deepcopy(graft), reference_probes=deepcopy(prior['probes']),
                            cycle=cycle, single_replacements=singles, group_replacements=group)
    return dict(protocol=PROTOCOL, seed=seed, archived_record_checksum=checksum,
                mapping='reverse', cue='cue1', target_cell=target,
                active_edges=active, silent_edges=silent, grafts=grafts)


def plan_directory(directory, output):
    directory, output = Path(directory), Path(output)
    pending = []
    for seed in SEEDS:
        matches = sorted((directory / 'input').glob(f'*-s{seed}-*.json'))
        require(len(matches) == 1, f'expected exactly one archived input for seed {seed}')
        path = matches[0]
        evidence = path.read_bytes()
        source, _ = source_record(evidence)
        require(path.name == f"{source['job']['id']}.json", 'source-plan archive filename differs')
        pending.append((path.name, build_plan(evidence, (directory / 'raw' / path.name).read_bytes())))
    output.mkdir(parents=True, exist_ok=True)
    for name, plan in pending:
        (output / name).write_text(json.dumps(plan, indent=2, allow_nan=False) + '\n')
    return [name for name, _ in pending]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    try:
        plan_directory(args.directory, args.output)
    except (OSError, ValueError, KeyError, TypeError, IndexError, OverflowError) as exc:
        parser.exit(1, f'contingency-a-source-plan: {exc}\n')


if __name__ == '__main__':
    main()
