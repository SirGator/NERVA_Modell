#!/usr/bin/env python3
"""Verify the balanced-component removal replay and compare dose-matched probes."""

import argparse
from collections import Counter
import hashlib
import json
import math
from pathlib import Path

from contingency_chemical_analysis import finite, normalization, verify_plan
from contingency_eligibility_analysis import source_record
from contingency_grown_dose_analysis import (
    MAPPINGS, TIERS, analyze_record as analyze_grown_dose, bounded_plan, headrooms,
    permutation, published, reconstruct, score, score_summary, verified_probes,
)
from verify_contingency import require

PROTOCOL = 'NERVA-Contingency-Grown-Residual/1'
CONTROLS = ('balanced', *MAPPINGS, *(f'reference_{mapping}' for mapping in MAPPINGS))
SPECIFICITY_CONTROLS = ('balanced', 'shuffle_residual_{mapping}', 'sham_{mapping}')
RECORD_FIELDS = {
    'protocol', 'seed', 'archived_record_checksum', 'checkpoint_time_us',
    'replay_verified', 'projection', 'directions', 'tiers',
}


def dot(left, right):
    """Accumulate in support order with f64 addition, as in the Rust replay."""
    total = 0.
    for first, second in zip(left, right):
        total += first * second
    return total


def length(vector):
    return math.sqrt(dot(vector, vector))


def projected_directions(directions, shuffle):
    balanced = directions['balanced']
    shuffled_balanced = [balanced[index] for index in shuffle]
    bases = {'balanced': balanced, 'sham': shuffled_balanced}
    denominators = {name: dot(vector, vector) for name, vector in bases.items()}
    require(all(value > 0. for value in denominators.values()), 'zero projection basis')
    coefficients, projected = {}, {}
    for mapping in MAPPINGS:
        direction = directions[mapping]
        coefficients[mapping] = {
            f'{name}_coefficient': dot(direction, vector) / denominators[name]
            for name, vector in bases.items()
        }
        for name, arm in (('balanced', 'residual'), ('sham', 'sham')):
            coefficient = coefficients[mapping][f'{name}_coefficient']
            residual = [
                value - coefficient * component for value, component in zip(direction, bases[name])
            ]
            require(length(residual) > length(direction) * 1e-12,
                    f'{arm}_{mapping}: numerically zero projection direction')
            projected[f'{arm}_{mapping}'] = residual
        projected[f'shuffle_residual_{mapping}'] = [
            projected[f'residual_{mapping}'][index] for index in shuffle
        ]
    return coefficients, projected


def common_ray_dose(archived_dose, base, directions):
    limits = {}
    for name, vector in directions.items():
        magnitude = length(vector)
        require(magnitude > 0., f'{name}: zero projection direction')
        limits[name] = min(
            room / abs(delta / magnitude)
            for room, delta in zip(headrooms(base, vector), vector) if delta
        )
    return min(archived_dose, .9 * min(limits.values())), limits


def tier_layout(tier, new_directions, archived_arms, reuse):
    """Require aliases to reference only verified archive arms and never carry replacement data."""
    require(tier.keys() == {'requested_norm', 'reused_controls', 'arms'},
            'unexpected residual tier fields')
    controls = set(CONTROLS) & archived_arms.keys()
    reused = controls if reuse else set()
    active = new_directions.keys() | (controls - reused)
    require(tier['reused_controls'] == sorted(reused), 'unexpected archived control aliases')
    require(tier['arms'].keys() == active, 'unexpected residual tier arms')
    return {name: archived_arms[name]['probes'] for name in sorted(reused)}


def score_residual(probes, baseline, targets):
    specificity = score(probes, baseline, targets, prefix='residual_', controls=SPECIFICITY_CONTROLS)
    improvement = score(probes, baseline, targets, prefix='residual_',
                        controls=(*SPECIFICITY_CONTROLS, '{mapping}'))
    for checks in (specificity, improvement):
        for mapping in checks.values():
            for cue in mapping.values():
                cue['shuffled_residual'] = cue.pop('shuffle_residual_{mapping}')
                cue['sham'] = cue.pop('sham_{mapping}')
                if '{mapping}' in cue:
                    cue['original'] = cue.pop('{mapping}')
    return score_summary(specificity), score_summary(improvement)


def analyze_record(raw, source, consolidation, grown_dose, checksum):
    archived = analyze_grown_dose(grown_dose, source, consolidation, checksum)
    assay, support, base, previous, edges, start = reconstruct(
        grown_dose, source, consolidation, checksum)
    require(raw.keys() == RECORD_FIELDS and raw['protocol'] == PROTOCOL
            and raw['replay_verified'] is True, 'unexpected residual replay record')
    require(type(raw['seed']) is int and raw['seed'] == source['job']['seed']
            and raw['archived_record_checksum'] == checksum
            and type(raw['checkpoint_time_us']) is int
            and raw['checkpoint_time_us'] == assay.checkpoint, 'archived checkpoint identity differs')
    shuffle = permutation(raw['seed'], [edges[edge][1] for edge in support])
    coefficients, directions = projected_directions(previous, shuffle)
    require(raw['projection'].keys() == coefficients.keys(), 'unexpected projection mappings')
    for name, values in coefficients.items():
        reported = raw['projection'][name]
        require(reported.keys() == values.keys()
                and all(finite(reported[key]) and reported[key] == value
                        for key, value in values.items()), f'{name}: projection coefficients differ')
    require(raw['directions'].keys() == directions.keys(), 'unexpected residual directions')
    for name, vector in directions.items():
        reported = raw['directions'][name]
        require(all(map(finite, reported)) and reported == vector,
                f'{name}: direction differs from reconstructed projection')

    baseline = grown_dose['baseline']['probes']
    archived_ray = grown_dose['tiers']['ray']['requested_norm']
    ray, ray_limits = common_ray_dose(archived_ray, base, directions)
    doses = dict(ray=ray, bounded=grown_dose['tiers']['bounded']['requested_norm'])
    require(raw['tiers'].keys() == set(TIERS), 'unexpected residual tiers')
    all_directions = previous | directions
    tiers = {}
    for kind, requested in doses.items():
        tier = raw['tiers'][kind]
        require(requested > 0., f'{kind}: no positive common intervention dose')
        require(finite(tier['requested_norm']) and tier['requested_norm'] == requested,
                f'{kind}: dose differs from reconstructed dose')
        reuse = kind == 'bounded' or requested == archived_ray
        archived_arms = grown_dose['tiers'][kind]['arms']
        probes = tier_layout(tier, directions, archived_arms, reuse)
        arms = {}
        for name, arm in tier['arms'].items():
            require(arm.keys() == {'plan', 'weights', 'probes'}, f'{kind}/{name}: unexpected fields')
            vector = all_directions[name]
            if kind == 'ray':
                weights, plan = normalization(base, vector, [requested])
                require(plan['faithful'], f'{name}: ray direction or dose failed tolerance')
            else:
                weights, plan = bounded_plan(base, vector, requested)
            require(plan['relative_norm_error'] < .01, f'{kind}/{name}: dose failed tolerance')
            verify_plan(arm['plan'], plan)
            require(published(arm['weights'], support) == weights,
                    f'{kind}/{name}: installed weights differ from reconstructed plan')
            verified_probes(arm['probes'], assay, edges, start, baseline)
            probes[name] = arm['probes']
            arms[name] = {
                'plan': plan,
                'changed_coordinates': sum(left != right for left, right in zip(base, weights)),
            }
        specificity, improvement = score_residual(probes, baseline, assay.targets)
        original = score_summary(score(probes, baseline, assay.targets, controls=('balanced',)))
        tiers[kind] = {
            'requested_norm': requested, 'archived_requested_norm': archived['tiers'][kind]['requested_norm'],
            'reused_controls': tier['reused_controls'], 'specificity': specificity,
            'improvement': improvement, 'original_vs_balanced': original,
            'original': archived['tiers'][kind]['learned'] if reuse else None,
            'reference': score_summary(score(probes, baseline, assay.targets, prefix='reference_')),
            'arms': arms,
        }
    return {
        'job_id': source['job']['id'], 'seed': raw['seed'], 'archived_record_checksum': checksum,
        'support_size': len(support), 'probe_start_us': start,
        'reference_choice': archived['reference_choice'],
        'unavailable_directions': archived['unavailable_directions'],
        'projection': coefficients, 'ray_limits': ray_limits,
        'direction_norms': {name: length(vector) for name, vector in all_directions.items()},
        'direction_nonzero_counts': {
            name: sum(value != 0. for value in vector) for name, vector in directions.items()
        },
        'tiers': tiers,
    }


def analyze_directory(directory):
    directory = Path(directory)
    parts = {
        'raw': 'raw', 'capacity': 'input/capacity', 'consolidation': 'input/consolidation',
        'grown_dose': 'input/grown_dose',
    }
    matched = {
        name: {path.name: path for path in (directory / part).glob('*.json')
               if path.name != 'manifest.json'}
        for name, part in parts.items()
    }
    raw = matched['raw']
    require(raw and all(raw.keys() == files.keys() for files in matched.values()),
            'residual replay and archived inputs differ')
    records = []
    for filename in sorted(raw):
        evidence = {name: files[filename].read_bytes() for name, files in matched.items()}
        source, checksum = source_record(evidence['capacity'])
        require(filename == f"{source['job']['id']}.json", 'file name differs from capacity job identity')
        result = analyze_record(json.loads(evidence['raw']), source,
                                json.loads(evidence['consolidation']),
                                json.loads(evidence['grown_dose']), checksum)
        records.append({**result, **{f'{name}_sha256': hashlib.sha256(data).hexdigest()
                                    for name, data in evidence.items()}})
    return {
        'protocol': PROTOCOL, 'cohort_size': len(records),
        'tiers': {
            kind: {
                'reused_control_records': sum(bool(row['tiers'][kind]['reused_controls']) for row in records),
                'passed_case_histograms': {
                    metric: dict(Counter(row['tiers'][kind][metric]['passed_cases'] for row in records
                                         if row['tiers'][kind][metric] is not None))
                    for metric in ('specificity', 'improvement', 'original_vs_balanced', 'original', 'reference')
                },
            } for kind in TIERS
        },
        'records': sorted(records, key=lambda row: (row['seed'], row['job_id'])),
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path)
    args = parser.parse_args()
    try:
        print(json.dumps(analyze_directory(args.directory), indent=2, allow_nan=False))
    except (OSError, ValueError, KeyError, TypeError, IndexError, OverflowError) as exc:
        parser.exit(1, f'contingency-grown-residual: {exc}\n')


if __name__ == '__main__':
    main()
