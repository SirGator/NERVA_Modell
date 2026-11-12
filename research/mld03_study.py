#!/usr/bin/env python3
"""Freeze and verify a paired MLD-03 study using the existing native harness."""
import argparse
from concurrent.futures import ThreadPoolExecutor, as_completed
import importlib.util
import json
from pathlib import Path
import re
import shutil
import subprocess
import time
import tomllib
import zipfile
import mld02_study as support

HERE = Path(__file__).resolve().parent
PROJECT = HERE.parents[1]
sha, require, write_json = support.sha, support.require, support.write_json


def parse(path):
    spec = tomllib.loads(Path(path).read_text())
    require(set(spec) == {'study','model'}, 'expected study and model')
    q, m = spec['study'], spec['model']
    require(set(q) == {'name','seeds','workers','max_native_training_passes','max_wall_seconds'}, 'invalid study keys')
    require(isinstance(q['name'],str) and re.fullmatch(r'[A-Za-z0-9_-]{1,80}',q['name']), 'invalid name')
    require(isinstance(q['seeds'],list) and len(q['seeds']) == 5 and all(type(s) is int and 0<=s<2**64 for s in q['seeds']) and len(set(q['seeds'])) == 5, 'five unique paired seeds required')
    require(type(q['workers']) is int and 1<=q['workers']<=2, 'workers must be 1..2')
    require(type(q['max_native_training_passes']) is int and q['max_native_training_passes']>=112*len(q['seeds']), 'replay budget too small')
    require(type(q['max_wall_seconds']) is int and 1<=q['max_wall_seconds']<=7200, 'wall budget outside 1..7200')
    require(set(m)=={'processing_neurons','training_repetitions','rest_us'} and all(type(x) is int for x in m.values()), 'invalid model keys')
    require(16<=m['processing_neurons']<=256 and 1<=m['training_repetitions']<=8 and 100000<=m['rest_us']<=11000000, 'model outside bounded design')
    return spec


def leaf(seed, model):
    text = support.leaf_toml('mld03_'+str(seed),seed,model)
    text = text.replace('hippocampal_mld02','hippocampal_mld03').replace('["atlas", "interaction"]','["excitability", "population"]')
    return text[:text.index('[measurements]')] + '[measurements]\nmetrics = ["native_training_passes", "spike_effects", "functional_candidates", "population_interaction_a"]\n'


def load(root):
    p = support.load(root)
    require(p['protocol'] == 'NERVA-MLD-03-Preregistration/1', 'wrong preregistration')
    return p


def plan(config,root):
    spec = parse(config)
    if root.exists():
        p=load(root);require(p['specification']==spec,'registered configuration differs');return p
    subprocess.run(['cargo','build','--offline','--release','--manifest-path',str(HERE/'Cargo.toml')],check=True)
    root.mkdir(parents=True);(root/'configs').mkdir();(root/'native').mkdir()
    shutil.copy2(HERE/'target/release/nerva-research',root/'runner')
    for source,target in [('run.py','producer.py'),('mld03_analysis.py','analysis.py'),('mld03_study.py','study.py'),('mld02_study.py','mld02_study.py'),('mld02_analysis.py','mld02_analysis.py'),('learning_history.py','learning_history.py'),('MLD03.md','PROTOCOL.md')]:
        shutil.copy2(HERE/source,root/target)
    shutil.copy2(config,root/'source.toml')
    paths=[]
    for package in ['NERVA','NERVA_Modell/Modell','NERVA_Modell/nerva-qemu','NERVA_Modell/research']:
        base=PROJECT/package
        for folder in ['src','tests']: paths.extend((base/folder).rglob('*.rs'))
        paths.extend(base/n for n in ['Cargo.toml','Cargo.lock','build.rs'] if (base/n).is_file())
    paths.extend(HERE/n for n in ['run.py','mld03_study.py','mld03_analysis.py','mld02_study.py','mld02_analysis.py','MLD03.md'])
    paths.append(Path(config).resolve())
    with zipfile.ZipFile(root/'source-code.zip','w',compression=zipfile.ZIP_DEFLATED) as z:
        for p in sorted(set(paths)): z.write(p,p.relative_to(PROJECT))
    for seed in spec['study']['seeds']:
        (root/'configs'/f'{seed}.toml').write_text(leaf(seed,spec['model']));support.native(root,seed,'plan')
    artifacts=[p for p in root.iterdir() if p.is_file()]+list((root/'configs').glob('*.toml'))
    p={'protocol':'NERVA-MLD-03-Preregistration/1','specification':spec,
       'seed_scope':'Paired follow-up on five MLD-02 seeds already observed; no new confirmation cohort.',
       'fixed_design':{'conditions':[['native',1,1],['threshold_09999',0.9999,1],['threshold_099',0.99,1],['threshold_090',0.9,1],['tau_half',1,0.5],['tau_double',1,2]],'epsilon':1e-9,'native_records':10,'full_executions':20,'max_native_training_passes_with_replay':560},
       'artifacts_sha256':{str(path.relative_to(root)):sha(path) for path in artifacts},
       'leaf_manifest_sha256':{str(seed):sha(root/'native'/str(seed)/'manifest.json') for seed in spec['study']['seeds']},
       'git_heads':{name:subprocess.check_output(['git','-C',str(PROJECT/name),'rev-parse','HEAD'],text=True).strip() for name in ['NERVA','NERVA_Modell']}}
    write_json(root/'preregistration.json',p);support.atomic(root/'preregistration.sha256',(sha(root/'preregistration.json')+'\n').encode())
    return p


def analyze_seed(root,seed):
    module_spec=importlib.util.spec_from_file_location('mld03_archived_analysis',root/'analysis.py')
    module=importlib.util.module_from_spec(module_spec);module_spec.loader.exec_module(module)
    rows={}
    for path in sorted((root/'native'/str(seed)/'records').glob('*.json')):
        row=module.analyze_file(path);rows[row['section']]=row
    require(set(rows)=={'excitability','population'},'missing complete sections')
    for name,value in rows.items():write_json(root/'analysis'/str(seed)/(name+'.json'),value)
    return rows


def aggregate(root,p):
    data={str(s):{kind:json.loads((root/'analysis'/str(s)/(kind+'.json')).read_text()) for kind in ['excitability','population']} for s in p['specification']['study']['seeds']}
    total=2*sum(v['metrics']['native_training_passes'] for seed in data.values() for v in seed.values())
    require(total<=p['specification']['study']['max_native_training_passes'],'training budget exceeded')
    result={'protocol':'NERVA-MLD-03-Results/1','preregistration_sha256':sha(root/'preregistration.json'),'seed_scope':p['seed_scope'],'per_seed':data,'native_training_passes_with_replay':total}
    write_json(root/'study-summary.json',result)
    return result


def run(root):
    p=load(root);q=p['specification']['study'];deadline=time.monotonic()+q['max_wall_seconds']
    def job(seed):
        support.native(root,seed,'run',max(1,deadline-time.monotonic()))
        rows=analyze_seed(root,seed)
        print('VERIFIED',seed,'B_first_spike_condition=',rows['excitability']['first_B_spike_grid_condition'],'memory_candidate=',rows['excitability']['any_diagnostic_memory_candidate'],flush=True)
    with ThreadPoolExecutor(max_workers=q['workers']) as pool:
        futures=[pool.submit(job,s) for s in q['seeds']]
        for future in as_completed(futures):future.result()
    return aggregate(root,p)


def report(root):
    p=load(root)
    for seed in p['specification']['study']['seeds']:
        support.native(root,seed,'report');analyze_seed(root,seed)
    result=aggregate(root,p)
    write_json(root/'verification-receipt.json',{'preregistration_sha256':sha(root/'preregistration.json'),'summary_sha256':sha(root/'study-summary.json'),
        'native_report_verified':True,'independent_analyses_verified':True,'records_sha256':{str(path.relative_to(root)):sha(path) for path in sorted((root/'native').rglob('records/*.json'))}})
    return result


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('mode',choices=['plan','run','report']);parser.add_argument('config',type=Path);parser.add_argument('--output',type=Path)
    args=parser.parse_args()
    try:
        spec=parse(args.config);root=(args.output or HERE/'results'/spec['study']['name']).resolve()
        if args.mode=='plan':plan(args.config,root);print('PREREGISTERED',root)
        else:
            p=load(root);require(p['specification']==spec,'configuration differs from preregistration')
            value=run(root) if args.mode=='run' else report(root)
            print('COMPLETE',root,'native_training_passes=',value['native_training_passes_with_replay'])
    except (OSError,ValueError,subprocess.CalledProcessError) as exc:parser.exit(1,'mld03-study: '+str(exc)+'\n')
