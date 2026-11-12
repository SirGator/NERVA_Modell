#!/usr/bin/env python3
"""Read selected probe spikes and integration boundaries from MLD-03; never simulate."""
import argparse
import hashlib
import json
from pathlib import Path


def select(record, *, arm, condition='native', image=2, neuron=None, reset=False, limit=20):
    if limit < 1:
        raise ValueError('limit must be positive')
    details = record['details']
    if details['protocol'] != 'NERVA-MLD-03/1':
        raise ValueError('MLD-03 record required')
    if details['section'] == 'excitability':
        value = details['conditions'][condition]
        probe = value['reset_probe'] if reset else value['arms'][arm]['probe']
    else:
        value = details['arms'][arm]
        probe = value['reset_probe'] if reset else value['own_weight_effect']['probe']
    frame = next(f for f in probe['frames'] if f['image'] == image)
    spikes = [s for s in frame['spike_times'] if neuron is None or s[0] == neuron]
    boundaries = [b for b in frame['integration_boundaries'] if neuron is None or b[1] == neuron]
    return {'job': record['job'], 'section': details['section'], 'arm': arm,
            'condition': condition if details['section'] == 'excitability' else None,
            'weight_reset': reset, 'image': image, 'neuron': neuron,
            'interval_us': [frame['start_us'], frame['end_us']],
            'spike_columns': ['neuron_id', 'time_us'], 'spikes': spikes[:limit],
            'matching_spikes': len(spikes), 'spikes_truncated': len(spikes) > limit,
            'boundary_columns': frame['boundary_columns'], 'boundaries': boundaries[:limit],
            'matching_boundaries': len(boundaries), 'boundaries_truncated': len(boundaries) > limit,
            'scope': 'Probe frame only; no complete rest or training timeline in this export.'}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('record', type=Path)
    parser.add_argument('--arm', required=True)
    parser.add_argument('--condition', default='native')
    parser.add_argument('--image', type=int, default=2)
    parser.add_argument('--neuron', type=int)
    parser.add_argument('--reset', action='store_true')
    parser.add_argument('--limit', type=int, default=20)
    args = parser.parse_args()
    try:
        raw = args.record.read_bytes()
        result = select(json.loads(raw)['record'], arm=args.arm, condition=args.condition,
                        image=args.image, neuron=args.neuron, reset=args.reset, limit=args.limit)
        print(json.dumps({'derived_export': True, 'source_record_sha256': hashlib.sha256(raw).hexdigest(), **result}, indent=2, allow_nan=False))
    except (OSError, ValueError, KeyError, StopIteration) as exc:
        parser.exit(1, 'mld03-trace: ' + str(exc) + '\n')
