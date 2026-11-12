#!/usr/bin/env python3
"""Post-hoc descriptions of verified MLD-03 records; primary decisions stay unchanged."""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
from mld03_trace import select
import struct


def sha(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def f32(value):
    return struct.unpack('f', struct.pack('f', value))[0]


def candidate(row):
    return f32(f32(row[2]) + f32(row[5]))


def electrical(probe, reset, core):
    closest = None
    approach = None
    pairs = 0
    for frame, base in zip(probe['frames'][3:], reset['frames'][3:]):
        left = frame['integration_boundaries']
        right = base['integration_boundaries']
        lc = Counter((r[0], r[1]) for r in left)
        rc = Counter((r[0], r[1]) for r in right)
        index = {(r[0], r[1]): r for r in right if rc[r[0], r[1]] == 1}
        for row in left:
            if row[1] not in core or row[6] or row[7]:
                continue
            gap = row[4] - candidate(row)
            if closest is None or gap < closest['gap']:
                closest = {'image':frame['image'],'time_us':row[0],'neuron_id':row[1],'gap':gap}
            key = row[0], row[1]
            old = index.get(key)
            if lc[key] != 1 or old is None or old[6] or old[7] or row[4] != old[4]:
                continue
            pairs += 1
            original_gap = old[4] - candidate(old)
            increment = candidate(row) - candidate(old)
            if original_gap > 0 and increment > 0:
                ratio = increment / original_gap
                if approach is None or ratio > approach['fraction_of_reset_gap']:
                    approach = {'image':frame['image'],'time_us':row[0],'neuron_id':row[1],
                                'candidate_increase':increment,'reset_gap':original_gap,'fraction_of_reset_gap':ratio}
    return {'closest_actual_nonspiking_core_gap':closest,'largest_positive_approach_to_reset_threshold':approach,
            'matched_nonspiking_nonrefractory_pairs_same_threshold':pairs,
            'scope':'Post-hoc cue-only description. Unique same-time/neuron pairs; no changed-time or unequal-threshold comparison. Not a minimum global threshold prediction.'}


def describe(root):
    receipt = json.loads((root/'verification-receipt.json').read_text())
    if not receipt['native_report_verified'] or not receipt['independent_analyses_verified']:
        raise ValueError('verified study required')
    output = {'scope':'Post-hoc supplementary descriptions; no change to preregistered gates, seeds or grid.',
              'verification_receipt_sha256':sha(root/'verification-receipt.json'),'per_seed':{}}
    for relative, digest in sorted(receipt['records_sha256'].items()):
        path = root/relative
        if sha(path) != digest:
            raise ValueError('record changed: '+relative)
        record = json.loads(path.read_text())['record']
        d = record['details'];seed=str(record['job']['seed'])
        entry = output['per_seed'].setdefault(seed,{})
        if d['section']=='excitability':
            entry['electrical']={name:{arm:electrical(row['probe'],value['reset_probe'],set(d['populations']['core']))
                                      for arm,row in value['arms'].items()} for name,value in d['conditions'].items()}
            if seed=='23003':
                example=select(record,arm='B',condition='native',neuron=88,image=2,limit=20)
                (root/'trace-example-23003-B-88.json').write_text(json.dumps({'derived_export':True,'source_record_sha256':digest,**example},indent=2)+'\n')
        else:
            def sequence(arm,population):
                ids=set(d['populations'][population])
                return [[s for s in f['spike_times'] if s[0] in ids] for f in d['arms'][arm]['own_weight_effect']['probe']['frames']]
            entry['population_sequences']={
                name:{population:sequence(a,population)==sequence(b,population) for population in ['sensor','core','motor']}
                for name,a,b in [('sensor_vs_all_bridge','sensor','all_bridge'),('sensor_vs_both','sensor','both'),('both_vs_all_bridge','both','all_bridge')]}
            entry['population_cue_sequences']={
                name:{population:{'original':sequence(a,population)[3:5]==sequence(b,population)[3:5],
                                  'novel':sequence(a,population)[5:]==sequence(b,population)[5:]}
                      for population in ['sensor','core','motor']}
                for name,a,b in [('sensor_vs_all_bridge','sensor','all_bridge'),('sensor_vs_both','sensor','both'),('both_vs_all_bridge','both','all_bridge')]}
            entry['intrinsic_current_deltas']={arm:{pop:sum(n['intrinsic_delta'] for n in value['training']['homeostasis_by_neuron'] if n['neuron_id'] in d['populations'][pop])
                                                  for pop in ['sensor','core','motor','other']} for arm,value in d['arms'].items()}
    output['describer_sha256']=sha(Path(__file__).resolve())
    (root/'supplementary-description.json').write_text(json.dumps(output,indent=2,sort_keys=True,allow_nan=False)+'\n')
    return output


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('root',type=Path)
    args=parser.parse_args()
    print('DESCRIBED',len(describe(args.root.resolve())['per_seed']),'seeds')
