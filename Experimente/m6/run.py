#!/usr/bin/env python3
"""Preregistered M6 factorial experiment; all neural work runs in Rust."""
import argparse
from collections import Counter,defaultdict
from concurrent.futures import ThreadPoolExecutor
import csv
import json
from pathlib import Path
import shlex
import statistics
import subprocess
import sys
sys.path.insert(0,str(Path(__file__).resolve().parents[1]/'m4'))
from runner import DEFAULT_ARMS,bootstrap_paired,json_out
from diagnose import require

MAPS=['normal_mapping','reversed_mapping']
STAGES=['checkpoint_probe','probe','retained_probe','interference_probe']
CHECKS=['same_initial_network','matched_experience','no_leakage','frozen_probe','balanced_labels',
        'same_actions_across_mappings','calibration_replay_exact','parent_not_calibrated_or_reset',
        'zero_calibration_gap_on_parent','all_copies_before_image']

def cases(seed,trials=(256,128,128)):
    rows=[]
    for family,sensor,resolution,core in [('diag','effect_2ch',0,32),('pixels','pixels',16,128)]:
        for variant in 'ABCD':
            base=dict(protocol='NERVA-M4-v1',case_id=family+'_'+variant,seed=seed,arms=DEFAULT_ARMS,
                train_trials=trials[0],test_trials=trials[1],calibration_trials=trials[2],
                params=dict(sensor=sensor,resolution=resolution,encoding='deterministic',processing=core,
                    action_effect_delay_ms=50,motor_feedback=True,stdp=True,homeostasis=True,development=False,
                    initial_drive=0,pruning_min_us=5000000,normalization='equal_input_current'))
            rows.append(dict(protocol='NERVA-M6-v1',variant=variant,base=base))
    return rows

def plan(stage,trials=(256,128,128)):
    return dict(protocol='NERVA-M6-v1',stage=stage,seeds=list(range(300,310)) if stage=='confirm' else [0,1,2],
        conditions=cases(0,trials),trial_counts=list(trials),
        preregistration=dict(core_homeostasis='on, held fixed; visual on for A/B, off for C/D; motor/copy off',
            variants={'A':[1,True],'B':[3,True],'C':[1,False],'D':[3,False]},
            action_copy_weight=.9,core_membrane_tau_us=20000,copy_times_relative_us=[0,10000,20000],
            action_effect_delay_us=50000,trial_us=160000,
            primary_sensor='effect_2ch / 32 Core',replication_sensor='16x16 pixels / 128 Core',
            primary_readout='visual-only count centroids, calibrated on replay fork at training-end checkpoint',
            checkpoint_probe='two actions on independent exact checkpoint replays; two examples per mapping/seed',
            immediate_probe='128 online trials starting at checkpoint, zero calibration gap; first forecast after 100ms',
            early_probe='first 16 balanced online trials, secondary transient diagnostic',
            mapping='same executed action stream for both mappings; only experienced action/effect relation flips',
            success='ten seeds; paired > chance and all three controls in both mappings, visual >=80%; flip and dual-correct advantages',
            retention='reported separately for immediate, checkpoint, rest and interference; no refitting or parent reset',
            inference='seed-paired bootstrap; descriptive intervals, no multiple-comparison correction; no post hoc tuning'))

def validate(r,config,path=None,*,protocol='NERVA-M6-v1',requested_config=None,stages=None,checkpoint_rest_us=0,extra_branches=None):
    stages=STAGES if stages is None else stages
    expected_config=config if requested_config is None else requested_config
    require(r.get('protocol')==protocol and r.get('requested_config')==expected_config,'wrong protocol/config')
    base=config['base'];require(r.get('seed')==base['seed'] and r.get('case_id')==base['case_id'],'wrong case/seed')
    require(all(r.get('checks',{}).get(k) is True for k in CHECKS),'failed/missing M6 audit')
    ncopy=3 if config['variant'] in 'BD' else 1
    sensor_homeo=config['variant'] in 'AB'
    for key,value in dict(copies_per_action=ncopy,core_tau_us=20000,action_copy_weight=.9,
            core_homeostasis=base['params']['homeostasis'],visual_homeostasis=sensor_homeo,
            motor_and_copy_homeostasis=False,parent_calibration_gap_us=0,action_effect_delay_us=50000).items():
        require(r['protocol_details'][key]==value,'effective protocol differs: '+key)
    for arm in DEFAULT_ARMS:
        for mapping in MAPS:
            m=r['arms'][arm][mapping]
            require(m['checkpoint_us']==m['immediate_probe_start_us'],'parent calibration gap')
            require(m['parameters_preserved'] is True,'frozen parameters changed')
            for stage in stages:
                expected=2 if stage.endswith('checkpoint_probe') else base['test_trials'];probe=m[stage]
                require(probe['test_count']==expected,'wrong sample count')
                require(Counter(probe['labels'])=={-1:expected//2,1:expected//2},'probe labels unbalanced')
                for feature,key in [('count','predictions_count'),('time_bins','predictions_time_bins')]:
                    guesses=probe[key];require(len(guesses)==expected and all(x in [-1,1] for x in guesses),'invalid predictions')
                    measured=sum(a==b for a,b in zip(probe['labels'],guesses))/expected
                    require(probe[feature]['balanced_accuracy']==measured,'score differs from predictions')
            for stage,stats in m['stage_diagnostics'].items():
                neural=stats['neural'];h=stats['sensor']['homeostasis_updates_visual_core_motor_copy']
                require(not (neural['formed'] or neural['pruned']),'M6 development was active')
                require(stage=='training' or not (neural['weight_updates'] or sum(h)),'plasticity in evaluation')
                require(h[2:]==[0,0] and (sensor_homeo or h[0]==0),'homeostasis outside population')
                require(base['params']['homeostasis'] or sum(h)==0,'Core-homeostasis-off model updated homeostasis')
        for stage in stages:
            normal=r['arms'][arm]['normal_mapping'][stage];reverse=r['arms'][arm]['reversed_mapping'][stage]
            require(all(a==-b for a,b in zip(normal['labels'],reverse['labels'])),'mapping labels not reversed')
            for feature,key in [('count','predictions_count'),('time_bins','predictions_time_bins')]:
                a,b=normal[key],reverse[key];comparison=r['arms'][arm]['mapping_comparison'][stage][feature]
                flip=sum(x==-y for x,y in zip(a,b))/len(a)
                dual=sum(x==label and y==inverse for x,y,label,inverse in zip(a,b,normal['labels'],reverse['labels']))/len(a)
                require(comparison==dict(flip_rate=flip,dual_correct_rate=dual,test_count=len(a)),'mapping comparison differs from predictions')
    if not path:return None
    groups=defaultdict(list);trials=0
    with Path(str(path)+'.details.csv').open(newline='') as f:
        for row in csv.DictReader(f):
            trials+=1;arm,mapping,stage=row['arm'],row['mapping'],row['stage']
            require(arm in DEFAULT_ARMS and mapping in ['normal','reversed'] and stage in stages+['training'],'unknown trial group')
            action,forecast,image,motion=(int(row[k]) for k in ['action_us','forecast_us','effect_image_us','motion_us'])
            require(forecast<image==motion and image-action==50000,'causal boundary violated')
            require(int(row['copy_arrivals_before_image'])==3*ncopy,'copy arrivals missing')
            require(action<=int(row['first_copy_arrival_us'])<=forecast,'copy timestamp outside cue')
            if int(row['cue_core_spikes']):require(action<=int(row['first_cue_spike_us'])<=int(row['last_cue_spike_us'])<=forecast,'future Core spike')
            if stage!='training' or arm!='scrambled':require(row['shown_effect']==row['actual_effect'],'physical mapping mismatch')
            groups[arm,mapping,stage].append(row)
    for arm in DEFAULT_ARMS:
        for mapping in ['normal','reversed']:
            for stage in stages+(['training'] if arm!='before' else []):
                rows=groups[arm,mapping,stage];n=2 if stage.endswith('checkpoint_probe') else (base['train_trials'] if stage=='training' else base['test_trials'])
                require(sorted(int(row['trial']) for row in rows)==list(range(n)),'missing/duplicate trial IDs')
                require(Counter(row['key'] for row in rows)=={'a':n//2,'d':n//2},'action balance failed')
                require(Counter(row['shown_effect'] for row in rows)=={'-1':n//2,'1':n//2},'image balance failed')
                if stage!='training':
                    probe=r['arms'][arm][mapping+'_mapping'][stage]
                    require([int(x['actual_effect']) for x in rows]==probe['labels'],'trial CSV labels differ')
                    require([int(x['count_prediction']) for x in rows]==probe['predictions_count'],'trial CSV count predictions differ')
                    require([int(x['time_prediction']) for x in rows]==probe['predictions_time_bins'],'trial CSV timing predictions differ')
            first=groups[arm,mapping,'probe'][0];m=r['arms'][arm][mapping+'_mapping']
            require(int(first['action_us'])-50000==m['checkpoint_us'],'immediate parent probe did not start at checkpoint')
        for stage in stages:
            a=groups[arm,'normal',stage];b=groups[arm,'reversed',stage]
            require(all(x['key']==y['key'] and int(x['actual_effect'])==-int(y['actual_effect']) for x,y in zip(a,b)),'mapping actions differ')
    for mapping in ['normal','reversed']:
        a=groups['aged_frozen',mapping,'training'];b=groups['paired',mapping,'training'];c=groups['scrambled',mapping,'training']
        fields=['trial','key','action_us','effect_image_us','shown_effect','start_frame_hash','effect_frame_hash']
        require([[x[k] for k in fields] for x in a]==[[x[k] for k in fields] for x in b],'paired/frozen replay differs')
        require([(x['key'],x['action_us']) for x in b]==[(x['key'],x['action_us']) for x in c],'scrambled action stream differs')
        require(Counter((x['key'],x['shown_effect']) for x in c)=={(key,effect):base['train_trials']//4 for key in ['a','d'] for effect in ['-1','1']},'scrambled contingency not independent')
    copy_rows=0
    with Path(str(path)+'.copies.csv').open(newline='') as f:
        for row in csv.DictReader(f):
            copy_rows+=1;action=int(row['action_us']);times=json.loads(row['arrival_times_us'])
            require(times==[action+1500+10000*p for p in range(ncopy) for _ in range(3)],'actual copy timing differs')
            require(int(row['copy_emissions'])==ncopy and max(times)<=int(row['forecast_us'])<int(row['effect_image_us']),'copy/image deadline failed')
    require(copy_rows==trials,'copy CSV coverage differs')
    phase_rows=0
    with Path(str(path)+'.audit.csv').open(newline='') as f:
        for row in csv.DictReader(f):
            phase_rows+=1;updates=[int(row[k]) for k in ['weight_updates','homeostasis_updates','formed','pruned']]
            require(not any(updates[2:]),'structure mutated')
            require(row['stage']=='training' and row['arm'] not in ['before','aged_frozen'] or not any(updates),'frozen phase/control changed learning parameters')
    sensor_rows=0;counts=defaultdict(lambda:[0]*6)
    with Path(str(path)+'.sensor.csv').open(newline='') as f:
        for row in csv.DictReader(f):
            sensor_rows+=1;h=[int(row['homeostasis_'+x]) for x in ['visual','core','motor','copy']]
            require(h[2:]==[0,0] and (sensor_homeo or h[0]==0),'sensor CSV population leak')
            if not sensor_homeo:require(all(float(row['sensor_intrinsic_'+x])==0 for x in ['min','mean','max']),'excluded sensor intrinsic current changed')
            group=counts[row['arm'],row['mapping']+'_mapping',row['stage']]
            for i,x in enumerate([int(row['generated_sensor_spikes']),int(row['visual_neuron_spikes'])]+h):group[i]+=x
    require(sensor_rows==phase_rows,'sensor/phase coverage differs')
    for (arm,mapping,stage),actual in counts.items():
        stats=r['arms'][arm][mapping]['stage_diagnostics'][stage]['sensor']
        require(actual==[stats['generated_sensor_spikes'],stats['visual_neuron_spikes']]+stats['homeostasis_updates_visual_core_motor_copy'],'sensor CSV/JSON mismatch')
    branch_rows=0
    with Path(str(path)+'.branches.csv').open(newline='') as f:
        for row in csv.DictReader(f):
            branch_rows+=1;checkpoint=int(row['checkpoint_us']);require(int(row['parent_after_us'])==checkpoint,'branch advanced parent')
            require(row['exact_replay_state']=='true' and row['parent_unchanged']=='true','replay state mismatch')
            duration=base['calibration_trials']*160000 if row['branch']=='calibration' else 160000
            if row['branch'].startswith('rest_checkpoint_'):duration+=checkpoint_rest_us
            if extra_branches and row['branch'] in extra_branches:duration=extra_branches[row['branch']]
            require(int(row['branch_end_us'])-checkpoint==duration,'branch clock drift')
    require(branch_rows==((40 if checkpoint_rest_us else 24)+8*len(extra_branches or {})),'missing calibration/checkpoint branches')
    return dict(trial_rows=trials,phase_rows=phase_rows,sensor_rows=sensor_rows,branch_rows=branch_rows)

def run(args):
    manifest=plan(args.stage,(args.train_trials,args.test_trials,args.calibration_trials));root=args.output;root.mkdir(parents=True,exist_ok=True)
    path=root/'manifest.json'
    if path.exists():require(json.loads(path.read_text())==manifest,'different manifest; use a new directory')
    json_out(path,manifest)
    if not args.command:print('Plan written:',path);return
    require('{config}' in args.command and '{output}' in args.command,'command needs {config} and {output}')
    require(1<=args.jobs<=32,'jobs must be 1..32')
    def execute(item):
        template,seed=item;config=dict(template,base=dict(template['base'],seed=seed));name=config['base']['case_id']
        out=root/'results'/name/f'seed-{seed:03d}.json'
        if out.exists():validate(json.loads(out.read_text()),config);return f'VALID {name} seed {seed}'
        cfg=root/'configs'/name/f'seed-{seed:03d}.json';json_out(cfg,config);out.parent.mkdir(parents=True,exist_ok=True)
        cmd=args.command.replace('{config}',shlex.quote(str(cfg))).replace('{output}',shlex.quote(str(out)))
        proc=subprocess.run(cmd,shell=True,text=True,capture_output=True)
        out.with_suffix('.stdout.log').write_text(proc.stdout);out.with_suffix('.stderr.log').write_text(proc.stderr)
        require(proc.returncode==0,f'Failed {name} seed {seed}: {proc.stderr[-2000:]}')
        validate(json.loads(out.read_text()),config);return f'OK {name} seed {seed}'
    with ThreadPoolExecutor(max_workers=args.jobs) as pool:
        for message in pool.map(execute,[(c,s) for c in manifest['conditions'] for s in manifest['seeds']]):print(message,flush=True)
    report(root)

def learning_contrasts(name,values,stage):
    n=len(values);contrasts=[]
    passed=n>=10
    for mapping in MAPS:
        passed=passed and statistics.mean(r['arms']['paired'][mapping]['visual_decodability'] for r in values)>=.8
        for feature in ['count','time_bins']:
            p=[r['arms']['paired'][mapping][stage][feature]['balanced_accuracy'] for r in values]
            for reference in ['before','aged_frozen','scrambled','chance']:
                q=[.5]*n if reference=='chance' else [r['arms'][reference][mapping][stage][feature]['balanced_accuracy'] for r in values]
                d=bootstrap_paired(p,q)
                contrasts.append(dict(case=name,stage=stage,mapping=mapping,feature=feature,reference=reference,
                    paired=statistics.mean(p),control=statistics.mean(q),delta=d['mean_difference'],ci_low=d['ci95'][0],ci_high=d['ci95'][1]))
                if feature=='count':passed=passed and d['ci95'][0]>0
    for metric in ['flip_rate','dual_correct_rate']:
        for reference in ['before','aged_frozen','scrambled']:
            p=[r['arms']['paired']['mapping_comparison'][stage]['count'][metric] for r in values]
            q=[r['arms'][reference]['mapping_comparison'][stage]['count'][metric] for r in values]
            d=bootstrap_paired(p,q);passed=passed and d['ci95'][0]>0
            contrasts.append(dict(case=name,stage=stage,mapping='both',feature=metric,reference=reference,
                paired=statistics.mean(p),control=statistics.mean(q),delta=d['mean_difference'],ci_low=d['ci95'][0],ci_high=d['ci95'][1]))
    return passed,contrasts

def report(root):
    manifest=json.loads((root/'manifest.json').read_text());records={};audits=[]
    for template in manifest['conditions']:
        name=template['base']['case_id'];records[name]=[]
        for seed in manifest['seeds']:
            config=dict(template,base=dict(template['base'],seed=seed));path=root/'results'/name/f'seed-{seed:03d}.json';r=json.loads(path.read_text())
            audit=validate(r,config,path);audits.append(dict(case=name,seed=seed,**audit));records[name].append(r)
    for family in ['diag','pixels']:
        for index,_ in enumerate(manifest['seeds']):
            rows=[records[family+'_'+v][index] for v in 'ABCD']
            require(len({r['initial_network_fingerprint'] for r in rows})==1,'factorial variants have different initial networks')
            require(all(r['normalization']==rows[0]['normalization'] for r in rows),'factorial normalization differs')
            for arm in DEFAULT_ARMS:
                for mapping in MAPS:
                    generated=[{stage:stats['sensor']['generated_sensor_spikes'] for stage,stats in r['arms'][arm][mapping]['stage_diagnostics'].items()} for r in rows]
                    require(all(x==generated[0] for x in generated),'generated visual input differs across variants')
            for arm in ['before','aged_frozen']:
                require(rows[0]['arms'][arm]==rows[2]['arms'][arm] and rows[1]['arms'][arm]==rows[3]['arms'][arm],'frozen sensor-toggle controls differ')
    summaries=[];contrasts=[]
    for name,values in records.items():
        n=len(values);row=dict(case=name,n_seeds=n)
        row['visual']=statistics.mean(r['arms']['paired']['visual_decodability'] for r in values)
        row['sensor_transfer']=statistics.mean(r['arms']['paired'][m]['stage_diagnostics']['training']['sensor']['visual_emissions_per_generated_spike'] for r in values for m in MAPS)
        row['training_sensor_current']=statistics.mean(r['arms']['paired'][m]['stage_diagnostics']['training']['neural']['sensor_to_core_current_mean_per_neuron_second'] for r in values for m in MAPS)
        for stage in STAGES:
            row[stage]=statistics.mean(r['arms']['paired'][m][stage]['count']['balanced_accuracy'] for r in values for m in MAPS)
            passed,extra=learning_contrasts(name,values,stage)
            contrasts.extend(extra)
            row[stage+'_evidence']=passed
        row['early16']=statistics.mean(r['arms']['paired'][m]['probe']['early_count']['balanced_accuracy'] for r in values for m in MAPS)
        row['flip_rate']=statistics.mean(r['arms']['paired']['mapping_comparison']['probe']['count']['flip_rate'] for r in values)
        row['dual_correct_rate']=statistics.mean(r['arms']['paired']['mapping_comparison']['probe']['count']['dual_correct_rate'] for r in values)
        summaries.append(row)
    factorial=[]
    for family in ['diag','pixels']:
        def measure(variant,metric,stage):
            values=records[family+'_'+variant]
            if metric=='visual':return [r['arms']['paired']['visual_decodability'] for r in values]
            return [statistics.mean(r['arms']['paired'][m][stage]['count']['balanced_accuracy'] for m in MAPS) for r in values]
        for stage in STAGES:
            for left,right in [('B','A'),('C','A'),('D','A'),('D','B'),('D','C')]:
                factorial.append(dict(family=family,stage=stage,contrast=left+' minus '+right,**bootstrap_paired(measure(left,'prediction',stage),measure(right,'prediction',stage))))
            a,b,c,d=[measure(v,'prediction',stage) for v in 'ABCD']
            interaction=[dd-bb-cc+aa for aa,bb,cc,dd in zip(a,b,c,d)]
            factorial.append(dict(family=family,stage=stage,contrast='D-B-C+A interaction',**bootstrap_paired(interaction,[0]*len(a))))
    for filename,rows in [('summary.csv',summaries),('prediction-contrasts.csv',contrasts)]:
        with (root/filename).open('w',newline='') as f:
            w=csv.DictWriter(f,fieldnames=list(rows[0]));w.writeheader();w.writerows(rows)
    json_out(root/'diagnostics.json',dict(validated_records=len(audits),validated_trial_rows=sum(x['trial_rows'] for x in audits),
        validated_phase_rows=sum(x['phase_rows'] for x in audits),validated_branch_rows=sum(x['branch_rows'] for x in audits),
        generated_input_matched=True,frozen_sensor_toggle_controls_equal=True,audits=audits,factorial_contrasts=factorial))
    signals=[]
    for name in ['diag_B','diag_D','pixels_B','pixels_D']:
        groups=defaultdict(list)
        for seed in manifest['seeds']:
            with (root/'results'/name/f'seed-{seed:03d}.json.details.csv').open(newline='') as f:
                for row in csv.DictReader(f):groups[row['arm'],row['stage']].append(int(row['cue_core_spikes']))
        for (arm,stage),counts in sorted(groups.items()):
            signals.append(dict(case=name,arm=arm,stage=stage,windows=len(counts),silent_windows=counts.count(0),
                core_spikes_sum=sum(counts),core_spikes_mean=statistics.mean(counts)))
    json_out(root/'signal-diagnostics.json',signals)
    lines=['# NERVA M6 – gemessene Ergebnisse','',f"Stufe: {manifest['stage']}; Seeds: {manifest['seeds']}.",'',
        'A: Einzelkopie/Sensor-Homöostase an; B: drei Kopien/an; C: Einzelkopie/aus; D: drei Kopien/aus. Core-Homöostase bleibt an. Alle Varianten: Kopiegewicht 0,9, Core-Tau 20 ms, Wirkungsabstand 50 ms.',
        '', 'Kalibrierung läuft ausschließlich auf exakten Replayzweigen. Die laufende Elterninstanz startet ohne Kalibrierzeit am Trainingsendzustand. Checkpoint prüft A und D aus demselben Zustand separat (zwei Beispiele pro Belegung/Seed).',
        '', '| Fall | Seeds | Checkpoint | Online sofort | Erste 16 | Nach Ruhe | Nach Interferenz | Visuelle Auslesbarkeit | Sensorweiterleitung | Belegungswechsel / beide richtig |',
        '|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|']
    for r in summaries:lines.append(f"| {r['case']} | {r['n_seeds']} | {100*r['checkpoint_probe']:.1f}% | {100*r['probe']:.1f}% | {100*r['early16']:.1f}% | {100*r['retained_probe']:.1f}% | {100*r['interference_probe']:.1f}% | {100*r['visual']:.1f}% | {100*r['sensor_transfer']:.1f}% | {100*r['flip_rate']:.1f}% / {100*r['dual_correct_rate']:.1f}% |")
    for stage in STAGES:lines+=['',stage+' – bestätigte Kriterien: '+(', '.join(r['case'] for r in summaries if r[stage+'_evidence']) or 'keine')+'.']
    lines+=['','[Alle gepaarten Vorhersagekontraste](prediction-contrasts.csv) · [Faktorielle Kontraste und unabhängiger Datenaudit](diagnostics.json).',
        '', 'Intervalle sind deskriptive Bootstrap-Intervalle über Seeds, ohne Korrektur für multiple Vergleiche. Spikezahlen, Aktionsunterscheidung und visuelle Auslesbarkeit allein sind kein Lernnachweis. Die erste Prognose liegt 100 ms nach dem Checkpoint; noch kürzere Zustände sind nicht ausgeschlossen.']
    (root/'summary.md').write_text('\n'.join(lines)+'\n');print('Report:',root/'summary.md')

def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('mode',choices=['plan','run','report']);p.add_argument('--output',required=True,type=Path)
    p.add_argument('--stage',choices=['pilot','confirm'],default='pilot');p.add_argument('--jobs',type=int,default=4);p.add_argument('--command')
    p.add_argument('--train-trials',type=int,default=256);p.add_argument('--test-trials',type=int,default=128);p.add_argument('--calibration-trials',type=int,default=128);args=p.parse_args()
    if args.mode=='report':report(args.output)
    else:
        if args.mode=='plan':args.command=None
        run(args)
if __name__=='__main__':main()
