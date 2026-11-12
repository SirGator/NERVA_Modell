#!/usr/bin/env python3
"""Preregister, archive, execute and verify the bounded MLD-02 seed cohorts."""
import argparse
from concurrent.futures import ThreadPoolExecutor, as_completed
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import shutil
import signal
import subprocess
import sys
import tempfile
import time
import tomllib
import zipfile

HERE=Path(__file__).resolve().parent
PROJECT=HERE.parents[1]

def require(ok,message):
    if not ok:raise ValueError(message)

def sha(p):
    with Path(p).open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()

def atomic(path,data):
    path=Path(path);path.parent.mkdir(parents=True,exist_ok=True)
    with tempfile.NamedTemporaryFile(dir=path.parent,delete=False) as f:
        tmp=Path(f.name);f.write(data);f.flush();os.fsync(f.fileno())
    try:os.replace(tmp,path)
    finally:tmp.unlink(missing_ok=True)

def write_json(path,value):atomic(path,(json.dumps(value,indent=2,sort_keys=True,allow_nan=False)+'\n').encode())

def parse(path):
    s=tomllib.loads(Path(path).read_text())
    require(set(s)=={'study','model'},'expected [study] and [model] only')
    q=s['study'];m=s['model']
    require(set(q)=={'name','known_seeds','confirmation_seeds','workers','max_native_training_passes','max_wall_seconds'},'unknown/missing study fields')
    require(isinstance(q['name'],str) and re.fullmatch(r'[A-Za-z0-9_-]{1,80}',q['name']),'invalid study name')
    require(isinstance(q['known_seeds'],list) and isinstance(q['confirmation_seeds'],list),'seed cohorts must be arrays')
    require(len(q['known_seeds'])==2 and len(q['confirmation_seeds'])>=3,'two known and at least three confirmation seeds required')
    seeds=q['known_seeds']+q['confirmation_seeds']
    require(all(type(i)is int and 0<=i<2**64 for i in seeds) and len(set(seeds))==len(seeds),'seeds must be unique u64 integers')
    require(type(q['workers'])is int and 1<=q['workers']<=2,'workers must be 1..2')
    require(type(q['max_native_training_passes'])is int and q['max_native_training_passes']>=196*len(seeds),'training-pass budget below replay upper bound')
    require(type(q['max_wall_seconds'])is int and 1<=q['max_wall_seconds']<=43200,'wall budget must be 1..43200 seconds')
    require(set(m)=={'processing_neurons','training_repetitions','rest_us'},'unexpected model fields')
    require(all(type(v)is int for v in m.values()),'model settings must be integers')
    require(16<=m['processing_neurons']<=256 and 1<=m['training_repetitions']<=16 and 100000<=m['rest_us']<=20000000,'model outside MLD02 budget')
    return s

def leaf_toml(name,seed,m):
    return f'''[experiment]
name = "{name}"
scenario = "hippocampal_mld02"
seeds = [{seed}]
interventions = ["atlas", "interaction"]
verify_replay = true
max_runs = 2
[base]
processing_neurons = {m['processing_neurons']}
training_repetitions = {m['training_repetitions']}
rest_us = {m['rest_us']}
[measurements]
metrics = ["reference_verified", "recurrent_reference_verified", "novel_core_memory_candidate", "native_training_passes", "matched_observations", "interaction_a", "interaction_b", "offline_verified"]
'''

def native(root,seed,mode,timeout=None):
    leaf=root/'native'/str(seed)
    command=[sys.executable,str(root/'producer.py'),mode,str(root/'configs'/f'{seed}.toml'),'--output',str(leaf),'--binary',str(root/'runner')]
    leaf.parent.mkdir(exist_ok=True)
    log=root/f'{mode}-{seed}.log'
    with log.open('w') as stream:
        p=subprocess.Popen(command,stdout=stream,stderr=subprocess.STDOUT,start_new_session=True)
        try:code=p.wait(timeout=timeout)
        except subprocess.TimeoutExpired:
            os.killpg(p.pid,signal.SIGTERM)
            try:p.wait(timeout=5)
            except subprocess.TimeoutExpired:os.killpg(p.pid,signal.SIGKILL);p.wait()
            raise ValueError('native wall budget exceeded; see '+str(log))
    require(code==0,'native '+mode+' failed for seed '+str(seed)+'; see '+str(log))

def load(root):
    raw=(root/'preregistration.json').read_bytes()
    require(hashlib.sha256(raw).hexdigest()==(root/'preregistration.sha256').read_text().strip(),'preregistration hash differs')
    p=json.loads(raw)
    for name,digest in p['artifacts_sha256'].items():require(sha(root/name)==digest,'archived artifact changed: '+name)
    for seed,digest in p['leaf_manifest_sha256'].items():require(sha(root/'native'/seed/'manifest.json')==digest,'leaf manifest changed')
    return p

def plan(spec_path,root):
    spec=parse(spec_path)
    if root.exists():
        old=load(root);require(old['specification']==spec,'preregistered specification differs');return old
    seeds=spec['study']['known_seeds']+spec['study']['confirmation_seeds']
    previous=[]
    for pattern in ['**/manifest.json','**/control-plan.json','**/preregistration.json']:
        for p in (HERE/'results').glob(pattern):
            text=p.read_text()
            for seed in spec['study']['confirmation_seeds']:
                require(re.search(r'(?<!\d)'+str(seed)+r'(?!\d)',text)is None,'confirmation seed appeared in previous plan: '+str(p))
            previous.append(str(p.relative_to(HERE)))
    subprocess.run(['cargo','build','--offline','--release','--manifest-path',str(HERE/'Cargo.toml')],check=True)
    root.mkdir(parents=True);(root/'configs').mkdir();(root/'native').mkdir()
    shutil.copy2(HERE/'target/release/nerva-research',root/'runner')
    for source,dest in [('run.py','producer.py'),('mld02_analysis.py','analysis.py'),('mld02_study.py','study.py'),('learning_history.py','learning_history.py'),('MLD02.md','PROTOCOL.md')]:shutil.copy2(HERE/source,root/dest)
    shutil.copy2(spec_path,root/'source.toml')
    paths=[]
    for package in ['NERVA','NERVA_Modell/Modell','NERVA_Modell/nerva-qemu','NERVA_Modell/research']:
        base=PROJECT/package
        for folder in ['src','tests']:paths.extend((base/folder).rglob('*.rs'))
        paths.extend(base/n for n in ['Cargo.toml','Cargo.lock','build.rs'] if (base/n).is_file())
    paths.extend(HERE/n for n in ['run.py','mld02_study.py','mld02_analysis.py','MLD02.md'])
    paths.append(Path(spec_path).resolve())
    with zipfile.ZipFile(root/'source-code.zip','w',compression=zipfile.ZIP_DEFLATED) as z:
        for p in sorted(set(paths)):z.write(p,p.relative_to(PROJECT))
    for seed in seeds:
        (root/'configs'/f'{seed}.toml').write_text(leaf_toml('mld02_'+str(seed),seed,spec['model']))
        native(root,seed,'plan')
    artifacts=['runner','producer.py','analysis.py','study.py','learning_history.py','PROTOCOL.md','source.toml','source-code.zip']+[f'configs/{seed}.toml' for seed in seeds]
    p={'protocol':'NERVA-MLD-02-Preregistration/1','specification':spec,'confirmation_seed_audit':{'prior_documents':len(previous),'confirmation_seeds_absent':True},
       'counts':{'seeds':len(seeds),'native_records':2*len(seeds),'full_executions':4*len(seeds),'max_native_training_passes_with_replay':196*len(seeds)},
       'fixed_design':{'gain':64,'reference_dose':1,'pair_window_us':100000,'decision_epsilon':1e-9,'new_cue_pixels':[[5,5],[5,1],[6,5],[6,1]],'common_development_frames':20,
                       'original_cue_indices':[0,1],'novel_cue_indices':[2,3,4,5],'primary_norm_level':'reference_strength','secondary_norm_level':'common_strength'},
       'artifacts_sha256':{name:sha(root/name) for name in artifacts},'leaf_manifest_sha256':{str(seed):sha(root/'native'/str(seed)/'manifest.json') for seed in seeds},
       'git_heads':{name:subprocess.check_output(['git','-C',str(PROJECT/name),'rev-parse','HEAD'],text=True).strip() for name in ['NERVA','NERVA_Modell']}}
    write_json(root/'preregistration.json',p);atomic(root/'preregistration.sha256',(sha(root/'preregistration.json')+'\n').encode())
    return p

def analyze_seed(root,seed):
    module_spec=importlib.util.spec_from_file_location('mld02_archived_analysis',root/'analysis.py')
    module=importlib.util.module_from_spec(module_spec);module_spec.loader.exec_module(module)
    out={}
    for path in sorted((root/'native'/str(seed)/'records').glob('*.json')):
        r=module.analyze_file(path);out[r['section']]=r
    require(set(out)=={'atlas','interaction'},'missing complete MLD02 sections')
    for section,value in out.items():write_json(root/'analysis'/str(seed)/(section+'.json'),value)
    return out

def aggregate(root,p):
    seeds=p['specification']['study']['known_seeds']+p['specification']['study']['confirmation_seeds']
    data={str(s):{kind:json.loads((root/'analysis'/str(s)/(kind+'.json')).read_text()) for kind in ['atlas','interaction']} for s in seeds}
    fresh=[data[str(s)] for s in p['specification']['study']['confirmation_seeds']]
    keys=['positive_contribution_direction_advantage','chemical_distribution_changes_function_at_equal_norm','strength_alone_insufficient_at_reference_norm','novel_core_memory_candidate','potential_without_spike_effect']
    replication={}
    for k in keys:
        values=[r['atlas']['decisions'][k] for r in fresh]
        replication[k]={'per_confirmation_seed':values,'criterion_met_on_all':all(v is True for v in values),'all_evaluable':all(v is not None for v in values)}
    interactions=[[r['interaction']['interaction'][i] for r in fresh] for i in range(6)]
    total=2*sum(r[section]['metrics']['native_training_passes'] if section=='atlas' else 18 for r in data.values() for section in ['atlas','interaction'])
    require(total<=p['specification']['study']['max_native_training_passes'],'measured native reconstruction budget exceeded')
    result={'protocol':'NERVA-MLD-02-Results/1','preregistration_sha256':sha(root/'preregistration.json'),'known_seeds':p['specification']['study']['known_seeds'],
            'confirmation_seeds':p['specification']['study']['confirmation_seeds'],'per_seed':data,'confirmation_criteria':replication,
            'interaction_confirmation': [{'contrasts':v,'same_positive_direction':all(x>1e-9 for x in v),'same_negative_direction':all(x< -1e-9 for x in v)} for v in interactions],
            'native_training_passes_with_replay':total,'limits':['Three confirmation seeds provide descriptive replication; no conventional 5% significance claim.','A missing feasible functional reference leaves relevant directional decisions unevaluable.','Artificial input-path improvements are not automatically learned memory.']}
    write_json(root/'study-summary.json',result)
    return result

def run(root):
    p=load(root);q=p['specification']['study'];deadline=time.monotonic()+q['max_wall_seconds']
    def job(seed):
        native(root,seed,'run',max(1,deadline-time.monotonic()));r=analyze_seed(root,seed)
        print('VERIFIED',seed,'reference=',r['atlas']['reference']['verified'],'memory_candidate=',r['atlas']['decisions']['novel_core_memory_candidate'],flush=True)
    # A cohort barrier protects preselected fresh seeds from technical changes after controls.
    for label,seeds in [('known',q['known_seeds']),('confirmation',q['confirmation_seeds'])]:
        load(root);print('COHORT',label,seeds,flush=True)
        with ThreadPoolExecutor(max_workers=q['workers']) as pool:
            futures=[pool.submit(job,s) for s in seeds]
            for f in as_completed(futures):f.result()
    return aggregate(root,p)

def report(root):
    p=load(root)
    for seed in p['specification']['study']['known_seeds']+p['specification']['study']['confirmation_seeds']:
        native(root,seed,'report');analyze_seed(root,seed)
    result=aggregate(root,p)
    hashes={str(path.relative_to(root)):sha(path) for path in sorted((root/'native').rglob('records/*.json'))}
    write_json(root/'verification-receipt.json',{'preregistration_sha256':sha(root/'preregistration.json'),'summary_sha256':sha(root/'study-summary.json'),
               'native_report_verified':True,'independent_analyses_verified':True,'records_sha256':hashes})
    return result

def main():
    a=argparse.ArgumentParser(description=__doc__);a.add_argument('mode',choices=['plan','run','report']);a.add_argument('config',type=Path);a.add_argument('--output',type=Path)
    args=a.parse_args()
    try:
        spec=parse(args.config);root=(args.output or HERE/'results'/spec['study']['name']).resolve()
        if args.mode=='plan':p=plan(args.config,root);print('PREREGISTERED',root,p['counts'])
        else:
            p=load(root);require(p['specification']==spec,'configuration differs from preregistration')
            r=run(root) if args.mode=='run' else report(root);print('COMPLETE',root,'native_training_passes=',r['native_training_passes_with_replay'])
    except (OSError,ValueError,subprocess.CalledProcessError) as e:a.exit(1,'mld02-study: '+str(e)+'\n')
if __name__=='__main__':main()
