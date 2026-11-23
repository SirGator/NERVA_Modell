#!/usr/bin/env python3
"""Verify and summarize the native visual contingency capacity assay."""

import argparse
from collections import Counter, defaultdict
import json
from pathlib import Path

from contingency_chemical_analysis import norm, normalization, verify_plan
from contingency_electrical_analysis import f32
from verify_contingency import FRAME_US, FRAGMENTS, ORIGINAL, OUTCOMES, STIMULI
from verify_contingency import event_counts, records, require, training_sequence, weight_map

PROTOCOL = 'NERVA-Contingency-Capacity/1'
SCENARIO = 'hippocampal_contingency_capacity'
MAPPINGS = ('forward', 'reverse')
CASES = (
    ('direct_half', False, .5),
    ('direct_full', False, 1.),
    ('relay_half', True, .5),
    ('relay_full', True, 1.),
)
COMPARATORS = ('native_at_reference', 'reference_at_native')
CUE_INDEX = {name: i for i, cue in enumerate(ORIGINAL)
             for name in (cue, *[fragment for fragment in FRAGMENTS if fragment.startswith(cue)])}
FIXED_RUN = 'visual_contingency_capacity_20261123'
FIXED_SEEDS = {34123, 34127, 34141, 34147, 34157}


def natural(value):
    return type(value) is int and value >= 0


def identity_set(values, label):
    require(all(map(natural, values)) and len(values) == len(set(values)),
            f'{label}: invalid or duplicate IDs')
    return set(values)


def routes(topology, cue_arrivals, blank_arrivals, quiet, targets):
    eligible = {identity: (pre, post) for identity, pre, post, _, allowed in topology if allowed}
    result = []
    for flip in range(2):
        direct, via_relay, support = [], [], []
        for cue in range(2):
            expected = targets[cue ^ flip]
            specific = cue_arrivals[cue] - cue_arrivals[1 - cue] - blank_arrivals
            direct_edges = {edge for edge in eligible.keys() & specific if eligible[edge][1] in expected}
            outgoing = defaultdict(set)
            for edge, (pre, post) in eligible.items():
                if pre in quiet and post in expected:
                    outgoing[pre].add(edge)
            incoming = defaultdict(set)
            for edge in eligible.keys() & specific:
                _, post = eligible[edge]
                if post in outgoing:
                    incoming[post].add(edge)
            relay_inputs = set().union(*incoming.values())
            relay_outputs = set().union(*(outgoing[relay] for relay in incoming))
            direct.append(direct_edges)
            via_relay.append(direct_edges | relay_inputs | relay_outputs)
            support.append({
                'cue_specific_eligible_arrivals': len(specific & eligible.keys()),
                'direct_edges': len(direct_edges),
                'complete_relays': len(incoming),
                'relay_input_edges': len(relay_inputs),
                'relay_terminal_edges': len(relay_outputs),
                'two_edge_paths': sum(len(inputs) * len(outgoing[relay])
                                      for relay, inputs in incoming.items()),
            })
        result.append(({'direct': direct, 'via_relay': via_relay}, support))
    return result


class Assay:
    def __init__(self, details, config):
        self.core = identity_set(details['core_ids'], 'Core')
        self.excitatory = identity_set(details['excitatory_core'], 'excitatory Core')
        require(len(self.core) == config['processing_neurons'] and self.excitatory <= self.core,
                'Core or excitatory Core identities differ from planned population')
        self.topology = details['topology']
        self.edge_ids = identity_set([edge for edge, *_ in self.topology], 'topology')
        initial = weight_map([(edge, weight) for edge, _, _, weight, _ in self.topology],
                             self.edge_ids)
        self.initial = {edge: f32(weight) for edge, weight in initial.items()}
        for edge, pre, post, _, eligible in self.topology:
            require(natural(pre) and natural(post) and type(eligible) is bool,
                    'invalid topology entry')
            require(not eligible or (pre in self.excitatory and post in self.core
                                     and self.initial[edge] < 2.),
                    'eligible route edge violates the Core or weight constraint')
        self.recurrent = {edge for edge, pre, post, _, _ in self.topology
                          if pre in self.core and post in self.core}
        self.stimuli = {name: identity_set(pixels, name) for name, pixels in details['stimuli'].items()}
        require(details['stimuli'] == config['stimuli'] and self.stimuli.keys() == set(STIMULI)
                and not self.stimuli['blank'] and all(self.stimuli[name] for name in STIMULI[1:]),
                'invalid capacity stimuli')
        self.checkpoint = details['development']['checkpoint_time_us']
        require(natural(self.checkpoint) and natural(config['rest_us']), 'invalid probe clock')
        self.start = self.checkpoint + config['rest_us']
        self.reset = details['reset']
        active = self.responses(self.reset, (*ORIGINAL, *OUTCOMES, 'blank'))
        nearby = set().union(*(active[name] for name in (*ORIGINAL, 'blank')))
        self.targets = [active[outcome].keys() - active[OUTCOMES[1 - i]].keys() - nearby
                        for i, outcome in enumerate(OUTCOMES)]
        require(details['target_groups'] == [sorted(group) for group in self.targets],
                'reported target groups disagree with checkpoint spikes')
        self.quiet = self.excitatory - nearby - set().union(*self.targets)
        require(details['quiet_relays'] == sorted(self.quiet),
                'reported quiet relays disagree with checkpoint spikes')

    def response(self, row, name, baseline=None):
        start = row['start_us']
        require(natural(start) and start == self.start, f'{name}: invalid probe start')
        roots = event_counts(row['root_spikes'], start, start + FRAME_US + 1,
                             ids=self.stimuli[name], label=f'{name} root')
        require(roots.keys() == self.stimuli[name], f'{name}: incomplete root channel coverage')
        require(baseline is None or row['root_spikes'] == baseline['root_spikes'],
                f'{name}: probe root spikes differ from baseline')
        require(natural(row['rest_core_spikes']), f'{name}: invalid rest spike count')
        require(identity_set(row['arrived_edges'], name) <= self.edge_ids,
                f'{name}: unknown arrived edge')
        return event_counts(row['core_spikes'], start, start + 4 * FRAME_US,
                            ids=self.core, label=f'{name} Core')

    def responses(self, rows, names, baseline=None):
        require(rows.keys() == set(names), 'unexpected capacity response names')
        return {name: self.response(rows[name], name, baseline[name] if baseline else None)
                for name in names}

    def comparison(self, observed, cues, reset, expected_weights=None):
        arms = observed['arms']
        require(arms.keys() == set(MAPPINGS), 'unexpected capacity comparison mappings')
        populations, installed, checks = {}, {}, {}
        names = (*cues, 'blank')
        reset_pop = self.responses(reset, names) if reset is not self.reset else {
            name: self.response(reset[name], name) for name in names
        }
        for mapping, arm in arms.items():
            populations[mapping] = self.responses(arm['responses'], names, reset)
            require([edge for edge, _ in arm['weights']] == list(self.initial),
                    f'{mapping}: weight vector order differs from topology')
            installed[mapping] = {
                edge: f32(weight) for edge, weight in weight_map(arm['weights'], self.edge_ids).items()
            }
            require(all(installed[mapping][edge] == self.initial[edge]
                        for edge in self.edge_ids - self.recurrent),
                    f'{mapping}: changed a weight outside recurrent Core')
            if expected_weights is not None:
                require(installed[mapping] == expected_weights[mapping],
                        f'{mapping}: installed weights differ from computed weights')
        for flip, (mapping, opposite) in enumerate(zip(MAPPINGS, reversed(MAPPINGS))):
            arm, other = populations[mapping], populations[opposite]
            quiet = arm['blank'].total() <= reset_pop['blank'].total()
            checks[mapping] = {}
            for cue in cues:
                target = CUE_INDEX[cue] ^ flip
                right, wrong = self.targets[target], self.targets[1 - target]
                correct = sum(arm[cue][identity] for identity in right)
                incorrect = sum(arm[cue][identity] for identity in wrong)
                base = sum(reset_pop[cue][identity] for identity in right)
                alt = sum(other[cue][identity] for identity in right)
                passed = (correct > max(incorrect, base, alt) and quiet
                          and arms[mapping]['responses'][cue]['rest_core_spikes']
                          <= reset[cue]['rest_core_spikes'])
                checks[mapping][cue] = dict(correct=correct, incorrect=incorrect,
                                             reset=base, opposite=alt, passed=passed)
            require(arms[mapping]['checks'] == checks[mapping],
                    f'{mapping}: reported spike counts or gates differ')
        passes = sum(check['passed'] for mapping in checks.values() for check in mapping.values())
        require(observed['passed_cases'] == passes, 'reported count of passed mapping cases differs')
        return passes, installed, checks

    def reference_weights(self, paths, strength):
        return {
            mapping: {edge: (f32(weight + f32(strength * f32(2. - weight)))
                             if edge in selected else weight)
                      for edge, weight in self.initial.items()}
            for mapping, selected in zip(MAPPINGS, (left | right for left, right in paths))
        }

    def training(self, arms, seed, repetitions):
        require(arms.keys() == set(MAPPINGS) and natural(repetitions) and repetitions > 0,
                'invalid capacity training specification')
        inputs, installed = {}, {}
        for mapping, arm in arms.items():
            require([edge for edge, _ in arm['weights']] == list(self.initial),
                    f'{mapping}: trained weight vector order differs from topology')
            trained = weight_map(arm['weights'], self.edge_ids)
            installed[mapping] = {edge: (f32(trained[edge]) if edge in self.recurrent else weight)
                                  for edge, weight in self.initial.items()}
            expected = list(training_sequence(mapping, seed, repetitions, 0))
            require([frame['stimulus'] for frame in arm['frames']] == expected,
                    f'{mapping}: training stimulus order differs')
            inputs[mapping] = Counter()
            for i, frame in enumerate(arm['frames']):
                name = expected[i]
                start = self.checkpoint + FRAME_US * i
                require(frame['start_us'] == start and natural(frame['weight_changes']),
                        f'{mapping}/{name}: invalid training frame')
                previous = self.stimuli[expected[i - 1]] if i else set()
                roots = event_counts(frame['root_spikes'], start, start + FRAME_US,
                                     ids=self.stimuli[name] | previous,
                                     label=f'{mapping}/{name} root')
                require(self.stimuli[name] <= roots.keys()
                        and all(channel in self.stimuli[name] or timestamp == start
                                for channel, timestamp in frame['root_spikes']),
                        f'{mapping}/{name}: training root channels differ from scheduled inputs')
                inputs[mapping].update(roots)
                event_counts(frame['core_spikes'], start, start + FRAME_US,
                             ids=self.core, label=f'{mapping}/{name} Core')
        require(inputs['forward'] == inputs['reverse'], 'training root spike counts differ by channel')
        return installed

    def comparator(self, row, direction, target_norm):
        installed, plans = {}, {}
        for mapping in MAPPINGS:
            base = list(self.initial.values())
            direct = [direction[mapping][edge] - weight for edge, weight in self.initial.items()]
            target = [target_norm[mapping][edge] - weight for edge, weight in self.initial.items()]
            weights, plan = normalization(base, direct, target)
            verify_plan(row['plans'][mapping], plan)
            plans[mapping] = plan
            if plan['faithful']:
                installed[mapping] = dict(zip(self.initial, weights))
        available = len(installed) == len(MAPPINGS)
        require(available == (row['comparison'] is not None),
                'comparator availability disagrees with the normalization plan')
        passes = (self.comparison(row['comparison'], ORIGINAL, self.reset, installed)[0]
                  if available else None)
        return {'passed_cases': passes, 'plans': plans}

    def ablations(self, rows, paths, checks):
        expected = [(flip, mapping, cue) for flip, mapping in enumerate(MAPPINGS)
                    for cue in ORIGINAL if checks[mapping][cue]['passed']]
        require(len(rows) == len(expected), 'terminal ablation coverage differs from passing cases')
        results = []
        for row, (flip, mapping, cue) in zip(rows, expected):
            require((row['mapping'], row['cue']) == (mapping, cue),
                    'terminal ablation mapping or cue differs')
            index = CUE_INDEX[cue]
            target = self.targets[index ^ flip]
            restored = [edge for edge, _, post, _, _ in self.topology
                        if edge in paths[flip][index] and post in target]
            require(row['restored_edges'] == restored, 'terminal ablation restored different edges')
            counts = self.response(row['response'], cue, self.reset[cue])
            ablated = sum(counts[identity] for identity in target)
            intact = checks[mapping][cue]['correct']
            require((row['intact_correct_count'], row['ablated_correct_count']) == (intact, ablated),
                    'terminal ablation reported spike counts differ')
            results.append({'mapping': mapping, 'cue': cue, 'restored_edges': len(restored),
                            'intact_correct_count': intact, 'ablated_correct_count': ablated,
                            'correct_spike_drop': intact - ablated})
        return results


def analyze_record(record):
    job, details = record['job'], record['details']
    config = job['effective_config']
    require(details['protocol'] == config['protocol'] == PROTOCOL
            and job['seed'] == config['seed'] == details['seed']
            and job['intervention'] == config['intervention'] == 'fixed_routes'
            and record['full_replay_checked'] is True, 'verified capacity record required')
    require(config['cases'] == [list(case) for case in CASES], 'fixed reference arm order differs')
    assay = Assay(details, config)
    require(details['references'].keys() == {case[0] for case in CASES},
            'unexpected fixed reference arms')
    route_rows = routes(assay.topology, [set(assay.reset[cue]['arrived_edges']) for cue in ORIGINAL],
                        set(assay.reset['blank']['arrived_edges']), assay.quiet, assay.targets)
    require(details['routes'] == [{key: [sorted(path) for path in paths] for key, paths in row.items()}
                                  for row, _ in route_rows],
            'reported reference routes differ from reconstructed routes')
    references, fixed = {}, {}
    for name, relay, strength in CASES:
        paths = [row['via_relay' if relay else 'direct'] for row, _ in route_rows]
        expected = assay.reference_weights(paths, strength)
        passes, _, checks = assay.comparison(details['references'][name], ORIGINAL, assay.reset, expected)
        references[name] = passes
        fixed[name] = (paths, expected, checks)
    best = max(references, key=references.__getitem__)
    best_score = references[best]
    followup = details['followup']
    require((followup is not None) == (best_score > 0),
            'followup availability differs from fixed reference scores')
    followup_result = None
    native_match = None
    if followup is not None:
        require(followup['choice'] == best, 'selected reference differs from fixed tie order')
        paths, chosen, checks = fixed[best]
        learned = assay.training(followup['training'], job['seed'], config['training_repetitions'])
        native_passes, native, native_checks = assay.comparison(
            followup['native'], ORIGINAL, assay.reset, learned)
        strengthened = assay.comparator(followup['native_at_reference'], native, chosen)
        attenuated = assay.comparator(followup['reference_at_native'], chosen, native)
        native_match = strengthened['passed_cases']
        require(followup['fragment_order'] == list(FRAGMENTS), 'unexpected held-out fragment order')
        require(followup['fragment_reset']['blank'] == assay.reset['blank'],
                'held-out blank baseline differs from the checkpoint blank')
        fragment_passes, _, fragment_checks = assay.comparison(
            followup['fragments'], FRAGMENTS, followup['fragment_reset'], chosen)
        ablations = assay.ablations(followup['terminal_ablation'], paths, checks)
        followup_result = {
            'choice': best,
            'reference_checks': checks,
            'native_passed_cases': native_passes,
            'native_checks': native_checks,
            'native_weight_changes': {mapping: {
                'changed_recurrent_edges': sum(values[edge] != initial
                                               for edge, initial in assay.initial.items()),
                'l2': norm([values[edge] - initial for edge, initial in assay.initial.items()]),
            } for mapping, values in native.items()},
            'native_at_reference': strengthened,
            'reference_at_native': attenuated,
            'fragment_passed_cases': fragment_passes,
            'fragment_checks': fragment_checks,
            'terminal_ablation': ablations,
        }
    require(record['metrics'] == {'best_reference_cases': best_score,
                                  'full_reference': int(best_score == 4),
                                  'full_native_at_reference': int(native_match == 4)},
            'reported capacity metrics differ')
    return {
        'job_id': job['id'],
        'seed': job['seed'],
        'target_sizes': list(map(len, assay.targets)),
        'quiet_relays': len(assay.quiet),
        'route_support': {mapping: dict(zip(ORIGINAL, support))
                          for mapping, (_, support) in zip(MAPPINGS, route_rows)},
        'reference_passed_cases': references,
        'best_reference_cases': best_score,
        'followup': followup_result,
    }


def analyze_directory(directory):
    directory = Path(directory)
    manifest = json.loads((directory / 'manifest.json').read_text(encoding='utf-8'))
    experiment = manifest['spec']['experiment']
    seeds = identity_set(experiment['seeds'], 'manifest cohort')
    require(experiment['scenario'] == SCENARIO and experiment['verify_replay'] is True,
            'capacity manifest with full replay required')
    require(experiment['name'] != FIXED_RUN or seeds == FIXED_SEEDS,
            'manifest differs from fixed capacity study cohort')
    counts = manifest['counts']
    require(counts == {'variants': 1, 'seeds': len(seeds), 'interventions': 1,
                      'trials': len(seeds), 'full_executions': 2 * len(seeds)},
            'manifest differs from one replay-verified run per seed')
    results = []
    for record, digest in records(directory):
        job = record['job']
        require(job['id'] == f"v00000-s{job['seed']}-i000" and job['variant'] == 'v00000',
                'unexpected capacity job identity')
        results.append({**analyze_record(record), 'record_sha256': digest})
    require(len(results) == len(seeds) and {row['seed'] for row in results} == seeds,
            'record seeds differ from manifest cohort')
    followups = [row['followup'] for row in results if row['followup'] is not None]
    comparators = {}
    for comparator in COMPARATORS:
        values = [row[comparator]['passed_cases'] for row in followups]
        available = sum(value is not None for value in values)
        comparators[comparator] = {
            'available': available,
            'attempted': len(values),
            'available_fraction': available / len(values) if values else None,
            'full_passes': values.count(4),
        }
    lesions = [lesion for row in followups for lesion in row['terminal_ablation']]
    return {
        'protocol': PROTOCOL,
        'cohort_size': len(results),
        'references_with_all_cases': sum(row['best_reference_cases'] == 4 for row in results),
        'followups': len(followups),
        'comparators': comparators,
        'terminal_ablation': {
            'assessed_cases': len(lesions),
            'correct_spike_count_decreased': sum(row['correct_spike_drop'] > 0 for row in lesions),
        },
        'records': sorted(results, key=lambda row: row['seed']),
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path)
    args = parser.parse_args()
    try:
        print(json.dumps(analyze_directory(args.directory), indent=2, allow_nan=False))
    except (OSError, ValueError, KeyError, TypeError, IndexError, OverflowError) as exc:
        parser.exit(1, f'contingency-capacity: {exc}\n')


if __name__ == '__main__':
    main()
