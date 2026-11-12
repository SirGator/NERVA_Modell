#!/usr/bin/env python3
"""Export verified MLD-02 histories and supplementary descriptions; no simulation."""
import argparse
import hashlib
import json
from pathlib import Path


def sha(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def write(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, sort_keys=True, allow_nan=False) + '\n')


def export(root):
    receipt = json.loads((root / 'verification-receipt.json').read_text())
    summary = json.loads((root / 'study-summary.json').read_text())
    if sha(root / 'study-summary.json') != receipt['summary_sha256']:
        raise ValueError('verified summary changed')
    outputs = {}
    supplemental = {}
    for relative, expected in sorted(receipt['records_sha256'].items()):
        path = root / relative
        if sha(path) != expected:
            raise ValueError('verified record changed: ' + relative)
        record = json.loads(path.read_text())['record']
        details = record['details']
        seed = str(record['job']['seed'])
        if details['section'] == 'atlas':
            histories = {'B': details['B_history']}
            histories.update({name: value['history'] for name, value in details['cases'].items()})
            for name, history in histories.items():
                output = root / 'histories' / seed / (name + '.json')
                write(output, {'derived_export': True, 'source_record_sha256': expected,
                               'source_record': relative, 'history_arm': name,
                               'record': {'job': record['job'], 'details': {'history': history}}})
                outputs[str(output.relative_to(root))] = sha(output)
        elif details['section'] == 'interaction':
            arms = {}
            for name, value in details['arms'].items():
                training = value['training']
                frames = value['own_weight_effect']['probe']['frames']
                arms[name] = {
                    'image_order': [f['image'] for f in frames],
                    'sensory_spikes': [sum(n for i, n in f['source_spike_counts'] if i < 64) for f in frames],
                    'core_spikes': [sum(f['core_counts']) for f in frames],
                    'margins': value['margins'],
                    'native_weight_changes': training['native_weight_changes'],
                    'normalizations': training['normalizations'],
                    'homeostasis': training['homeostasis'],
                    'homeostasis_changes': sum(n['changes'] for n in training['homeostasis_by_neuron']),
                    'mass_balance_error': training['mass_balance_error'],
                }
            weights = {name: dict(value['training']['weights_after']) for name, value in details['arms'].items()}
            contrast = [weights['H1_K1'][i] - weights['H1_K0'][i] - weights['H0_K1'][i] + weights['H0_K0'][i]
                        for i in weights['H0_K0']]
            supplemental[seed] = {
                'source_record_sha256': expected, 'development': details['development'], 'arms': arms,
                'exploratory_weight_contrast': {
                    'scope': 'Post-hoc descriptive weight vector; does not replace preregistered functional interaction.',
                    'l2': sum(x*x for x in contrast)**0.5, 'nonzero_edges': sum(x != 0 for x in contrast)},
            }
    write(root / 'supplementary-description.json', supplemental)
    outputs['supplementary-description.json'] = sha(root / 'supplementary-description.json')
    write(root / 'history-export-receipt.json', {
        'preregistration_sha256': summary['preregistration_sha256'],
        'verification_receipt_sha256': sha(root / 'verification-receipt.json'),
        'exporter_sha256': sha(Path(__file__).resolve()), 'exports_sha256': outputs,
        'scope': 'Derived exports only. Full synapse aggregates; detailed events only for declared selected IDs.',
    })
    return len(outputs) - 1


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('root', type=Path)
    args = parser.parse_args()
    print('EXPORTED', export(args.root.resolve()), 'histories')
