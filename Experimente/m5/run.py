#!/usr/bin/env python3
"""Two preregistered M5 causes, using the Rust model and existing M4 controls."""
import argparse
from concurrent.futures import ThreadPoolExecutor
import csv
import json
import math
from collections import defaultdict
from pathlib import Path
import shlex
import statistics
import subprocess
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'm4'))
from runner import DEFAULT_ARMS, bootstrap_paired, json_out, validate_record
from diagnose import audit_details, require

SCOPES = ['none', 'core', 'core_sensor', 'all']
MAPS = ['normal_mapping', 'reversed_mapping']
PHASES = ['probe', 'retained_probe', 'interference_probe']


def cases(seed, trials=(256, 128, 128)):
    base = dict(protocol='NERVA-M4-v1', seed=seed, arms=DEFAULT_ARMS,
                train_trials=trials[0], test_trials=trials[1], calibration_trials=trials[2],
                params=dict(sensor='pixels', resolution=16, encoding='deterministic', processing=128,
                            action_effect_delay_ms=8, motor_feedback=True, stdp=True, homeostasis=True,
                            development=False, initial_drive=0, pruning_min_us=5000000,
                            normalization='equal_input_current'))
    result = []
    for weight in [.45, .9, 1.8, 3.6]:
        for tau in [5000, 20000, 50000]:
            name = f'a_weight{round(weight*100):03d}_tau{tau}'
            b = dict(base, case_id=name, params=dict(base['params'], sensor='effect_2ch', resolution=0,
                     processing=32, homeostasis=False))
            result.append(dict(protocol='NERVA-M5-v1', experiment='motor_threshold', base=b,
                               action_copy_weight=weight, core_membrane_tau_us=tau, homeostasis_scope='none'))
    for scope in SCOPES:
        b = dict(base, case_id='b_'+scope, params=dict(base['params'], homeostasis=scope!='none'))
        result.append(dict(protocol='NERVA-M5-v1', experiment='sensor_homeostasis', base=b,
                           action_copy_weight=.9, core_membrane_tau_us=20000, homeostasis_scope=scope))
    return result


def plan(stage, trials=(256,128,128)):
    return dict(protocol='NERVA-M5-v1', stage=stage, seeds=list(range(200,210)) if stage=='confirm' else [0,1,2],
                conditions=cases(0, trials), trial_counts=list(trials),
                preregistration=dict(motor_weights=[.45,.9,1.8,3.6], core_tau_us=[5000,20000,50000],
                    motor_copy_times_us=[1,10001,20001], motor_horizon_us=50000,
                    sensor_scopes=SCOPES, sensor_protocol='unchanged M4 b01 16x16/128/8ms; all is the legacy reference',
                    signal_endpoint='Both A/D produce distinct count patterns and actual arrivals on plastic Core edges',
                    primary_causal_contrast='core_sensor minus core: emitted visual spikes / generated input spikes',
                    prediction_endpoint='Paired beats before, aged_frozen and scrambled in both mappings and three probes; visual >=80%; ten new seeds',
                    limits='A measures signal prerequisites only. B first probe follows 20.48 s visual calibration; rest means neutral image. No result-dependent gain or parameter selection.'))


def validate(record, config, path=None):
    require(record.get('protocol')=='NERVA-M5-v1' and record.get('requested_config')==config,
            'wrong M5 protocol or input configuration')
    require(record.get('seed')==config['base']['seed'] and record.get('case_id')==config['base']['case_id'], 'wrong seed/case')
    require(all(v is True for v in record['checks'].values()), 'failed M5 check')
    if config['experiment']=='motor_threshold':
        require(len(record['observations'])==6, 'missing motor controls')
        require(all(record['checks'].get(k) is True for k in ['same_initial_network','identical_neutral_images',
                'fixed_copy_pulse_count_and_timing','frozen_control_preserved']), 'missing motor audit')
        require({(r['action'],r['learning']) for r in record['observations']}==
                {(action,learning) for action in ['none','a','d'] for learning in [False,True]}, 'missing/duplicate motor interventions')
        for row in record['observations']:
            require(row['generated_and_emitted_visual_spikes']==0, 'visual input in motor bench')
            require(row['copy_spikes']==(0 if row['action']=='none' else 3), 'wrong copy emissions')
            require(row['copy_arrivals']==(0 if row['action']=='none' else 9), 'wrong copy arrivals')
            require(row['learning'] or row['core_weight_update_count']==0, 'frozen motor weights changed')
        return None
    adapted = dict(record, protocol='NERVA-M4-v1', requested_config=config['base'])
    validate_record(adapted,config['base']['seed'],config['base']['case_id'])
    scope=config['homeostasis_scope']
    allowed={'none':[False]*4,'core':[False,True,False,False],
             'core_sensor':[True,True,False,False],'all':[True]*4}[scope]
    for arm in DEFAULT_ARMS:
        for mapping in MAPS:
            data=record['arms'][arm][mapping]
            for stage, stats in data['m5_sensor']['stages'].items():
                h=stats['homeostasis_updates_visual_core_motor_copy']
                require(all(yes or n==0 for yes,n in zip(allowed,h)), 'maintenance outside population')
                require(stage=='training' or sum(h)==0, 'homeostasis in frozen probe')
    if not path:
        return None
    audit=audit_details(path,adapted)
    counters=defaultdict(lambda:[0]*6)
    sensor_rows=0
    with Path(str(path)+'.sensor.csv').open(newline='') as file:
        for row in csv.DictReader(file):
            sensor_rows+=1
            h=[int(row['homeostasis_'+p]) for p in ['visual','core','motor','copy']]
            require(all(yes or n==0 for yes,n in zip(allowed,h)), 'CSV maintenance outside population')
            require(row['stage']=='training' and row['arm'] not in ['before','aged_frozen'] or sum(h)==0,
                    'CSV homeostasis in frozen phase/control')
            potentials=[float(row['sensor_intrinsic_'+p]) for p in ['min','mean','max']]
            require(all(math.isfinite(v) for v in potentials),'nonfinite sensor intrinsic current')
            require(scope not in ['none','core'] or potentials==[0,0,0], 'excluded visual neuron intrinsic current changed')
            counts=counters[row['arm'],row['mapping']+'_mapping',row['stage']]
            for i,n in enumerate([int(row['generated_sensor_spikes']),int(row['visual_neuron_spikes'])]+h):counts[i]+=n
    require(sensor_rows==audit['phase_rows'],'sensor/phase CSV coverage differs')
    for (arm,mapping,stage),counts in counters.items():
        stats=record['arms'][arm][mapping]['m5_sensor']['stages'][stage]
        require(counts==[stats['generated_sensor_spikes'],stats['visual_neuron_spikes']]+stats['homeostasis_updates_visual_core_motor_copy'],
                'sensor CSV counters differ from result JSON')
    audit['sensor_rows']=sensor_rows
    return audit


def run(args):
    manifest=plan(args.stage,(args.train_trials,args.test_trials,args.calibration_trials))
    root=args.output;root.mkdir(parents=True,exist_ok=True)
    manifest_path=root/'manifest.json'
    if manifest_path.exists(): require(json.loads(manifest_path.read_text())==manifest, 'manifest differs; use a new directory')
    json_out(manifest_path,manifest)
    if not args.command:
        print('Plan written:',manifest_path);return
    require('{config}' in args.command and '{output}' in args.command, 'command needs {config} and {output}')
    require(1<=args.jobs<=32, 'jobs must be 1..32')
    def execute(item):
        template,seed=item;config=dict(template,base=dict(template['base'],seed=seed));name=config['base']['case_id']
        out=root/'results'/name/f'seed-{seed:03d}.json'
        if out.exists():validate(json.loads(out.read_text()),config);return f'VALID {name} seed {seed}'
        cfg=root/'configs'/name/f'seed-{seed:03d}.json';json_out(cfg,config);out.parent.mkdir(parents=True,exist_ok=True)
        command=args.command.replace('{config}',shlex.quote(str(cfg))).replace('{output}',shlex.quote(str(out)))
        proc=subprocess.run(command,shell=True,text=True,capture_output=True)
        out.with_suffix('.stdout.log').write_text(proc.stdout);out.with_suffix('.stderr.log').write_text(proc.stderr)
        require(proc.returncode==0, f'Failed {name} seed {seed}: {proc.stderr[-2500:]}')
        validate(json.loads(out.read_text()),config);return f'OK {name} seed {seed}'
    work=[(c,s) for c in manifest['conditions'] for s in manifest['seeds']]
    with ThreadPoolExecutor(max_workers=args.jobs) as pool:
        for message in pool.map(execute,work):print(message,flush=True)
    report(root)


def report(root):
    manifest=json.loads((root/'manifest.json').read_text());records={};audits=[]
    for template in manifest['conditions']:
        name=template['base']['case_id'];records[name]=[]
        for seed in manifest['seeds']:
            config=dict(template,base=dict(template['base'],seed=seed));path=root/'results'/name/f'seed-{seed:03d}.json'
            record=json.loads(path.read_text());audit=validate(record,config,path)
            if audit:audits.append(dict(case=name,seed=seed,**audit))
            records[name].append(record)
    for index,_ in enumerate(manifest['seeds']):
        b=[records['b_'+scope][index] for scope in SCOPES]
        require(len({r['initial_network_fingerprint'] for r in b})==1,'scopes have different initial networks')
        require(all(r['normalization']==b[0]['normalization'] for r in b),'input normalization differs across scopes')
        for arm in DEFAULT_ARMS:
            for mapping in MAPS:
                generated=[{stage:stats['generated_sensor_spikes'] for stage,stats in
                            r['arms'][arm][mapping]['m5_sensor']['stages'].items()} for r in b]
                require(all(g==generated[0] for g in generated),'generated sensory input differs across scopes')
        for arm in ['before','aged_frozen']:
            require(all(r['arms'][arm]==b[0]['arms'][arm] for r in b),'frozen controls differ across scopes')
    arows=[];brows=[];contrasts=[]
    for name,values in records.items():
        n=len(values)
        if name.startswith('a_'):
            motor=[o for r in values for o in r['observations'] if o['learning'] and o['action']!='none']
            arows.append(dict(case=name,n_seeds=n,weight=values[0]['requested_config']['action_copy_weight'],
                tau_us=values[0]['requested_config']['core_membrane_tau_us'],
                both_signal_criteria_seeds=sum(r['signal_criteria']['both_pass'] for r in values),
                mean_core_spikes=statistics.mean(o['core_spike_count'] for o in motor),
                mean_plastic_core_arrivals=statistics.mean(o['plastic_core_arrival_count'] for o in motor),
                mean_core_weight_updates=statistics.mean(o['core_weight_update_count'] for o in motor)))
            continue
        get=lambda r,arm,field:statistics.mean(r['arms'][arm][m][field] for m in MAPS)
        predict=[get(r,'paired','balanced_accuracy') for r in values]
        visual=[get(r,'paired','visual_decodability') for r in values]
        currents=[statistics.mean(r['arms']['paired'][m]['stage_diagnostics']['training']['sensor_to_core_current_mean_per_neuron_second'] for m in MAPS) for r in values]
        ratios=[statistics.mean(r['arms']['paired'][m]['m5_sensor']['stages']['training']['visual_emissions_per_generated_spike'] for m in MAPS) for r in values]
        row=dict(case=name,n_seeds=n,paired=statistics.mean(predict),visual=statistics.mean(visual),
                 training_sensor_current=statistics.mean(currents),visual_emissions_per_input=statistics.mean(ratios),
                 retained=statistics.mean(r['arms']['paired'][m]['retained_probe']['count']['balanced_accuracy'] for r in values for m in MAPS),
                 interference=statistics.mean(r['arms']['paired'][m]['interference_probe']['count']['balanced_accuracy'] for r in values for m in MAPS))
        criteria=n>=10
        for mapping in MAPS:
            criteria=criteria and statistics.mean(r['arms']['paired'][mapping]['visual_decodability'] for r in values)>=.8
            for stage in PHASES:
                for feature in ['count','time_bins']:
                    for reference in ['before','aged_frozen','scrambled']:
                        p=[r['arms']['paired'][mapping][stage][feature]['balanced_accuracy'] for r in values]
                        q=[r['arms'][reference][mapping][stage][feature]['balanced_accuracy'] for r in values]
                        d=bootstrap_paired(p,q)
                        contrasts.append(dict(case=name,mapping=mapping,stage=stage,feature=feature,reference=reference,
                            paired=statistics.mean(p),control=statistics.mean(q),
                            delta=d['mean_difference'],ci_low=d['ci95'][0],ci_high=d['ci95'][1]))
                        if feature=='count':criteria=criteria and d['ci95'][0]>0
        row['prediction_criteria_met']=criteria;brows.append(row)
    causal=[]
    for left,right in [('b_core_sensor','b_core'),('b_all','b_core_sensor'),('b_core','b_none')]:
        for metric in ['visual_decodability','balanced_accuracy','visual_transfer','sensor_current']:
            def measure(r):
                if metric=='visual_transfer':return statistics.mean(r['arms']['paired'][m]['m5_sensor']['stages']['training']['visual_emissions_per_generated_spike'] for m in MAPS)
                if metric=='sensor_current':return statistics.mean(r['arms']['paired'][m]['stage_diagnostics']['training']['sensor_to_core_current_mean_per_neuron_second'] for m in MAPS)
                return statistics.mean(r['arms']['paired'][m][metric] for m in MAPS)
            causal.append(dict(contrast=left+' minus '+right,metric=metric,**bootstrap_paired([measure(r) for r in records[left]],[measure(r) for r in records[right]])))
    for filename,rows in [('motor-summary.csv',arows),('sensor-summary.csv',brows),('prediction-contrasts.csv',contrasts)]:
        with (root/filename).open('w',newline='') as f:
            writer=csv.DictWriter(f,fieldnames=list(rows[0]));writer.writeheader();writer.writerows(rows)
    json_out(root/'diagnostics.json',dict(validated_records=sum(map(len,records.values())),
        validated_sensor_trial_rows=sum(a['trial_rows'] for a in audits),validated_sensor_phase_rows=sum(a['phase_rows'] for a in audits),
        frozen_controls_equal_across_scopes=True,generated_input_equal_across_scopes=True,
        audits=audits,causal_population_contrasts=causal))
    lines=['# NERVA M5 – gemessene Ergebnisse','',f"Stufe: {manifest['stage']}; Seeds: {manifest['seeds']}.",'',
           'A prüft eine Signalvoraussetzung, kein Wirkungslernen. Jede Bedingung hat drei Aktionskopien bei 1/10.001/20.001 µs und denselben 50-ms-Stichtag.','',
           '| Kopie-Gewicht | Core-Tau | Seeds mit beiden Signalkriterien | Core-Spikes pro Aktion | Ankünfte auf plastischen Core-Kanten | Core-Gewichtsupdates |',
           '|---:|---:|---:|---:|---:|---:|']
    for r in arows:lines.append(f"| {r['weight']} | {r['tau_us']/1000:g} ms | {r['both_signal_criteria_seeds']}/{r['n_seeds']} | {r['mean_core_spikes']:.2f} | {r['mean_plastic_core_arrivals']:.2f} | {r['mean_core_weight_updates']:.2f} |")
    lines+=['','B verwendet denselben Anfangszustand und dieselben Streams in allen Bereichen. Die erste Vorhersageprobe folgt nach 20,48 s visueller Kalibrierung.','',
        '| Homöostase | Vorhersage | Visuelle Auslesbarkeit | Sensor-Neuron-Emissionen / Eingangsspikes | Sensorstrom je Core/s | Nach Ruhe | Nach Interferenz |',
        '|---|---:|---:|---:|---:|---:|---:|---:|']
    for r in brows:lines.append(f"| {r['case'][2:]} | {100*r['paired']:.1f}% | {100*r['visual']:.1f}% | {r['visual_emissions_per_input']:.4f} | {r['training_sensor_current']:.3f} | {100*r['retained']:.1f}% | {100*r['interference']:.1f}% |")
    lines+=['','Gepaarte Intervalle: [diagnostics.json](diagnostics.json). Alle Vorhersagekontraste je Belegung, Zeitpunkt und Auswerter: [prediction-contrasts.csv](prediction-contrasts.csv).',
        '', 'Bestätigte Vorhersagekriterien: '+(', '.join(r['case'] for r in brows if r['prediction_criteria_met']) or 'keine')+'.',
        '', 'Bootstrap-Einheit ist der Seed. Intervalle sind deskriptiv, ohne Korrektur für multiple Vergleiche. Motoraktivität, Plastizität und Vorhersagequalität bleiben getrennte Endpunkte.']
    (root/'summary.md').write_text('\n'.join(lines)+'\n');print('Report:',root/'summary.md')


def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('mode',choices=['run','plan','report'])
    p.add_argument('--stage',choices=['pilot','confirm'],default='pilot');p.add_argument('--output',required=True,type=Path)
    p.add_argument('--jobs',default=4,type=int);p.add_argument('--command');p.add_argument('--train-trials',default=256,type=int)
    p.add_argument('--test-trials',default=128,type=int);p.add_argument('--calibration-trials',default=128,type=int);args=p.parse_args()
    if args.mode=='report':report(args.output)
    else:
        if args.mode=='plan':args.command=None
        run(args)

if __name__=='__main__':main()
