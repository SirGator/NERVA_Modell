"""Strict M9-v2 adapter for frozen, pre-effect action probes only."""
import csv
import hashlib
import json
from collections import Counter
from evolution.spec import number, require

ARMS = ('before', 'aged_frozen', 'paired', 'scrambled')
STAGES = ('checkpoint_probe', 'rest_checkpoint_probe')
MAPPINGS = ('normal_mapping', 'reversed_mapping')


def source_files(spec):
    root = spec.archive
    expected = {}
    with (root/'raw-sha256.csv').open(newline='') as stream:
        for row in csv.DictReader(stream):
            require(row['path'] not in expected, 'duplicate archive hash entry')
            expected[row['path']] = row
    files = []
    for seed in spec.training+spec.test:
        relative = f'results/{spec.case}/seed-{seed}.json'
        path = root/relative
        require(path.resolve().is_relative_to(root), 'archive input escapes data root')
        require(path.is_file() and relative in expected, 'missing sealed M9 source: '+relative)
        raw = path.read_bytes()
        require(len(raw) == int(expected[relative]['bytes'])
                and hashlib.sha256(raw).hexdigest() == expected[relative]['sha256'], 'M9 raw integrity mismatch: '+relative)
        files.append(dict(path=relative, seed=seed, sha256=hashlib.sha256(raw).hexdigest(), bytes=len(raw),
                          partition='train' if seed in spec.training else 'test'))
    return files


def extract(data, spec, partition, seed):
    require(data.get('protocol') == 'NERVA-M9-v2' and data.get('seed') == seed
            and data.get('case_id') == spec.case, 'M9 source identity differs')
    checks = ['same_initial_network', 'same_actions_across_mappings', 'no_leakage', 'matched_experience',
              'frozen_probe', 'balanced_labels', 'parent_not_calibrated_or_reset', 'zero_calibration_gap_on_parent']
    require(all(data.get('checks', {}).get(k) is True for k in checks), 'M9 source failed required experiment checks')
    # M9 geometry: sensory z=-1.5, motor z=1.5, action-copy z=1.2,
    # processing positions lie inside (-1,1). This is an adapter contract.
    core_ids = sorted(n['id'] for n in data['initial_neurons'] if -1 < n['position'][2] < 1)
    require(core_ids and len(set(core_ids)) == len(core_ids), 'invalid M9 processing neuron identities')
    require(len(core_ids)*spec.bins <= spec.max_features, 'diagnostic feature budget exceeded')
    core_index = {nid:i for i, nid in enumerate(core_ids)}
    core_edges = {e['id'] for e in data['initial_synapses'] if e['pre'] in core_index and e['post'] in core_index}
    initial_weights = {e['id']:e['weight'] for e in data['initial_synapses'] if e['id'] in core_edges}
    rows = []
    for arm in ARMS:
        for mapping in MAPPINGS:
            block = data['arms'][arm][mapping]
            weights = dict(block['trained_weights'])
            changed = sum(weights[eid] != value for eid, value in initial_weights.items())
            for stage in STAGES:
                trials = block['weight_reset_probes']['stages'][stage]['sham']
                require(len(trials) == 2 and {t['key'] for t in trials} == {'a', 'd'}, 'M9 probe lacks balanced actions')
                for trial in trials:
                    flags = ['effect_not_presented', 'exact_replay_state_before_rest', 'frozen_parameters_preserved',
                             'neurons_preserved_at_intervention', 'parent_unchanged']
                    require(all(trial.get(k) is True for k in flags) and trial.get('changed_weights') == [],
                            'unsafe or modified M9 probe')
                    start, end = trial['action_us'], trial['forecast_us']
                    require(type(start) is int and type(end) is int and end-start+1 == 50000, 'unexpected M9 forecast window')
                    counts = trial['core_counts']
                    require(len(counts) == len(core_ids) and all(type(v) is int and v >= 0 for v in counts), 'invalid core counts')
                    temporal = [0.]*(len(core_ids)*spec.bins)
                    observed = Counter()
                    for nid, at in trial['core_events']:
                        require(nid in core_index and type(at) is int and start <= at <= end,
                                'neural feature includes a spike outside the pre-effect window')
                        bin_index = (at-start)*spec.bins//(end-start+1)
                        temporal[core_index[nid]*spec.bins+bin_index] += 1
                        observed[nid] += 1
                    require(counts == [observed[nid] for nid in core_ids], 'spike events differ from count features')
                    action = -1 if trial['key'] == 'a' else 1
                    effect = action if mapping == 'normal_mapping' else -action
                    require(trial['prediction_count'] in (-1, 0, 1), 'invalid fixed decoder prediction')
                    rows.append(dict(seed=seed, partition=partition, arm=arm, mapping=mapping, stage=stage,
                        action=action, effect=effect, fixed_prediction=trial['prediction_count'], core_weights_changed=changed,
                        features=dict(counts=list(counts), temporal=temporal, action_only=[action])))
    return rows


def load(spec, files):
    results = []
    for source in files:
        raw = (spec.archive/source['path']).read_bytes()
        require(hashlib.sha256(raw).hexdigest() == source['sha256'], 'source changed after planning')
        results.extend(extract(json.loads(raw), spec, source['partition'], source['seed']))
    require(len(results) <= spec.max_records, 'diagnostic record budget exceeded')
    return results
