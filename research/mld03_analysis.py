#!/usr/bin/env python3
"""Independent checks of full MLD-03 spike times, electrical boundaries and paired gains."""
from collections import Counter
import hashlib
import json
import math
from pathlib import Path
import struct

EPS = 1e-9
ORDER = [4, 0, 1, 2, 3, 5, 6, 7, 8]
GRID = ['native', 'threshold_09999', 'threshold_099', 'threshold_090', 'tau_half', 'tau_double']


def require(ok, message):
    if not ok:
        raise ValueError(message)


def cosine(a, b):
    scale = math.sqrt(math.fsum(x*x for x in a)*math.fsum(y*y for y in b))
    return math.fsum(x*y for x,y in zip(a,b))/scale if scale else 0.


def margins(frames, targets):
    return [cosine(f['core_counts'],targets[i%2])-cosine(f['core_counts'],targets[1-i%2]) for i,f in enumerate(frames[3:])]


def f32(x):
    return struct.unpack('f', struct.pack('f', x))[0]


def events(frame, ids):
    ids = set(ids)
    return [tuple(e) for e in frame['spike_times'] if e[0] in ids]


def scan(probe, populations):
    require(probe['complete'] and probe['parameters_preserved'], 'incomplete/unfrozen probe')
    require([f['image'] for f in probe['frames']] == ORDER, 'probe order differs')
    for frame in probe['frames']:
        spikes = frame['spike_times']
        require(all(frame['start_us'] <= t < frame['end_us'] for _, t in spikes), 'spike outside probe frame')
        require(Counter(i for i, _ in spikes) == Counter(dict(frame['source_spike_counts'])), 'source spike counts differ')
        for population, column in [('core', 'core_counts'), ('motor', 'motor_counts')]:
            counts = Counter(i for i, _ in events(frame, populations[population]))
            actual = [counts[i] for i in populations[population]]
            reported = frame[column]
            require(actual == reported[:len(actual)] and not any(reported[len(actual):]), 'population counts differ from times')
        fired = []
        for time, identity, before, after, threshold, summed, spiked, refractory, until in frame['integration_boundaries']:
            candidate = f32(f32(before) + f32(summed))
            require(refractory == (time < until), 'refractory flag differs')
            require(spiked == (not refractory and candidate >= f32(threshold)), 'threshold crossing differs')
            if not spiked:
                require(f32(after) == (f32(before) if refractory else candidate), 'integration potential differs')
            if spiked and identity in populations['core']:
                fired.append((identity, time))
        require(Counter(fired) == Counter(events(frame, populations['core'])), 'missing core spike boundary')


def electrical(left, right):
    diffs = []
    gaps = []
    excluded = 0
    for a, b in zip(left['frames'], right['frames']):
        ac = Counter((v[0], v[1]) for v in a['integration_boundaries'])
        bc = Counter((v[0], v[1]) for v in b['integration_boundaries'])
        old = {(v[0], v[1]): v for v in b['integration_boundaries'] if bc[v[0], v[1]] == 1}
        for row in a['integration_boundaries']:
            key = row[0], row[1]
            if not row[7]:
                gaps.append(row[4] - f32(f32(row[2]) + f32(row[5])))
            if ac[key] == 1 and key in old:
                diffs.append(row[2] - old[key][2])
            else:
                excluded += 1
    return {'matched_unique_time_neuron_boundaries': len(diffs), 'unmatched_or_ambiguous_boundaries': excluded,
            'potential_before_delta_linf': max(map(abs, diffs), default=0.),
            'potential_before_delta_rms': math.sqrt(math.fsum(x*x for x in diffs)/len(diffs)) if diffs else None,
            'minimum_eligible_threshold_gap': min(gaps, default=None),
            'scope': 'Same unique integration times/neurons only; diverged or ambiguous samples excluded. Gap includes spike crossings.'}


def response(row, reset, targets, populations):
    p = row['probe']
    m = margins(p['frames'], targets)
    gain = [a-b for a, b in zip(m, margins(reset['frames'], targets))]
    require(all(abs(a-b) < 1e-12 for a, b in zip(m, row['margins'])), 'margins differ')
    require(all(abs(a-b) < 1e-12 for a, b in zip(gain, row['gains'])), 'gains differ')
    distinct = all(sum(t) > 0 for t in targets) and cosine(*targets) < 1-EPS
    quiet = sum(p['frames'][0]['core_counts']) <= sum(reset['frames'][0]['core_counts'])
    def gate(indices):
        return distinct and quiet and all(m[i] > EPS and gain[i] > EPS and sum(p['frames'][i+3]['core_counts']) > 0 for i in indices)
    result = {'margins': m, 'gains': gain}
    for key, indices in [('original_success', range(2)), ('novel_success', range(2, 6)), ('all_cues_success', range(6))]:
        result[key] = gate(indices)
        require(row[key] == result[key], 'success gate differs')
    for population in ['core', 'motor']:
        result[population+'_effect'] = any(events(a, populations[population]) != events(b, populations[population]) for a, b in zip(p['frames'], reset['frames']))
        require(result[population+'_effect'] == row[population+'_effect'], 'spike effect differs')
    result['electrical'] = electrical(p, reset)
    result['core_spikes'] = [sum(f['core_counts']) for f in p['frames']]
    result['sensory_spikes'] = [len(events(f, populations['sensor'])) for f in p['frames']]
    return result


def analyze_file(path):
    path = Path(path)
    raw = path.read_bytes()
    r = json.loads(raw)['record']
    d = r['details']
    require(r['full_replay_checked'] and d['shared_checkpoint_verified'], 'missing complete replay/checkpoint')
    pop = d['populations']
    require(not set(pop['core']) & set(pop['sensor']), 'population overlap')
    targets = d['fixed_targets']
    output = {'seed': r['job']['seed'], 'section': d['section'], 'record_sha256': hashlib.sha256(raw).hexdigest(), 'metrics': r['metrics']}
    if d['section'] == 'excitability':
        require(set(d['conditions']) == set(GRID), 'fixed grid differs')
        scan(d['reference_probe'], pop)
        vectors = {name: dict(v) for name, v in d['weight_vectors'].items()}
        require(vectors['reset'] == dict(d['weights_initial']), 'reset weights differ')
        require(all(set(v) == set(vectors['reset']) and all(0 <= x <= 2 for x in v.values()) for v in vectors.values()), 'invalid weight vectors')
        require(set(vectors)=={'reset','B','B_core','G','D'}, 'weight arms differ')
        for identity, pre, post in d['edges']:
            expected = vectors['B'][identity] if pre in pop['core'] and post in pop['core'] else vectors['reset'][identity]
            require(vectors['B_core'][identity] == expected, 'B-Core support differs')
        output['conditions'] = {}
        effects = candidates = 0
        for name in GRID:
            value = d['conditions'][name]
            require({k:dict(v) for k,v in value['applied_weights'].items()}==vectors, 'applied weights differ')
            expected = {'native':(1.,1.),'threshold_09999':(.9999,1.),'threshold_099':(.99,1.),'threshold_090':(.9,1.),'tau_half':(1.,.5),'tau_double':(1.,2.)}[name]
            require(abs(value['threshold_scale']-expected[0])<1e-7 and value['tau_scale']==expected[1], 'dynamics scales differ')
            native = {n[0]:n for n in d['conditions']['native']['core_parameters']}
            for identity, threshold, effective, tau in value['core_parameters']:
                require(abs(threshold-f32(native[identity][1]*f32(expected[0])))<1e-7 and tau==native[identity][3]*expected[1], 'applied Core parameters differ')
            reset = value['reset_probe']
            scan(reset, pop)
            arms = {}
            for arm, row in value['arms'].items():
                scan(row['probe'], pop)
                arms[arm] = response(row, reset, targets, pop)
                effects += arms[arm]['core_effect']
                candidates += arms[arm]['all_cues_success']
                require(all(events(a, pop['sensor']) == events(b, pop['sensor']) for a, b in zip(row['probe']['frames'], reset['frames'])), 'sensory input changed in a weight contrast')
            candidate = arms['B']['all_cues_success'] and arms['B_core']['all_cues_success'] and not arms['D']['all_cues_success'] and all(a > b+EPS for a, b in zip(arms['B_core']['gains'], arms['D']['gains']))
            output['conditions'][name] = {'threshold_scale': value['threshold_scale'], 'tau_scale': value['tau_scale'], 'arms': arms,
                'reset_core_spikes': [sum(f['core_counts']) for f in reset['frames']], 'diagnostic_memory_candidate': candidate,
                'reset_margins': margins(reset['frames'], targets)}
        require(effects == r['metrics']['spike_effects'] and candidates == r['metrics']['functional_candidates'], 'metric counts differ')
        output['first_B_spike_grid_condition'] = next((name for name in GRID if output['conditions'][name]['arms']['B']['core_effect']), None)
        output['first_B_additional_cue_spikes_grid_condition'] = next((name for name in GRID if any(a>b for a,b in zip(output['conditions'][name]['arms']['B']['core_spikes'][3:],output['conditions'][name]['reset_core_spikes'][3:]))), None)
        output['first_B_original_success_grid_condition'] = next((name for name in GRID if output['conditions'][name]['arms']['B']['original_success']), None)
        output['any_diagnostic_memory_candidate'] = any(v['diagnostic_memory_candidate'] for v in output['conditions'].values())
    else:
        require(d['section'] == 'population' and set(d['arms']) == {'off','sensor','core','both','all_bridge'}, 'population arms differ')
        scan(d['baseline_probe'], pop)
        output['development'] = d['development']
        require(d['development']['formed'] + d['development']['pruned'] > 0, 'structure never developed')
        output['arms'] = {}
        for name, value in d['arms'].items():
            require(value['common_checkpoint_verified'] and value['training']['topology_preserved'], 'population topology/checkpoint differs')
            require(value['training']['weights_before'] == d['arms']['off']['training']['weights_before'], 'initial weights differ')
            scan(value['reset_probe'], pop)
            scan(value['own_weight_effect']['probe'], pop)
            a = response(value['own_weight_effect'], value['reset_probe'], targets, pop)
            require(a['margins'] == value['margins'], 'population margins differ')
            selected = set(pop['sensor'] if name == 'sensor' else pop['core'] if name == 'core' else pop['sensor']+pop['core'] if name == 'both' else sum(pop.values(), []) if name == 'all_bridge' else [])
            require(all(n['neuron_id'] in selected for n in value['training']['homeostasis_by_neuron']), 'homeostasis escaped selected population')
            a['homeostasis_changes'] = sum(n['changes'] for n in value['training']['homeostasis_by_neuron'])
            require((a['homeostasis_changes'] > 0) == (name != 'off'), 'controller not exercised')
            output['arms'][name] = a
        contrast = [output['arms']['both']['margins'][i]-output['arms']['sensor']['margins'][i]-output['arms']['core']['margins'][i]+output['arms']['off']['margins'][i] for i in range(6)]
        require(all(abs(a-b)<1e-12 for a,b in zip(contrast,d['interaction'])), 'population interaction differs')
        output['interaction'] = contrast
    return output
