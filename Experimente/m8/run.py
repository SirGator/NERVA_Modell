#!/usr/bin/env python3
"""M8: fixed one-factor scans of existing dynamics, paired controls and exact checkpoint probes."""
import argparse,copy,csv,json,math,statistics,subprocess,sys
from collections import Counter,defaultdict
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
sys.path.insert(0,str(Path(__file__).resolve().parents[1]/'m6'))
import run as m6
STAGES=['checkpoint_probe','rest_checkpoint_probe','probe','retained_probe','interference_probe']
PRIMARY=['checkpoint_probe','rest_checkpoint_probe']
LEVELS=[('reference','reference',1),('a_weight2','core_weight_scale',2),('a_weight4','core_weight_scale',4),
 ('b_threshold08','core_threshold',.8),('b_threshold06','core_threshold',.6),('c_tau50','stdp_tau_us',50000),('c_tau100','stdp_tau_us',100000),
 ('d_delay500','core_delay_us',500),('d_delay5000','core_delay_us',5000),('e_fanout8','core_fanout',8),('e_fanout12','core_fanout',12),
 ('f_visual05','sensory_weight_scale',.5),('f_visual15','sensory_weight_scale',1.5)]
def cases(seed,counts=(256,128,128)):
    return [dict(protocol='NERVA-M8-v1',case_id=name,seed=seed,train_trials=counts[0],test_trials=counts[1],calibration_trials=counts[2],condition=dict(parameter=p,value=v)) for name,p,v in LEVELS]
def plan(stage,counts=(256,128,128)):
    return dict(protocol='NERVA-M8-v1',stage=stage,seeds=list(range(510,520)) if stage=='confirm' else [500,501,502],conditions=cases(0,counts),
        preregistration=dict(fixed='effect_2ch/32, three copies, copy0.9, core tau20ms, action/effect50ms, homeostasis/development off',
            design='one factor per condition, all 13 conditions carried to fresh confirmation without tuning or pilot selection',
            rate_gain='baseline reference gain fixed across conditions; visual weights intentionally change input current',
            density='extra local edges after original scaffold using independent RNG; original cells and all original edges retained',
            delay='only Core->Core delays replaced; original delay RNG still consumed',
            stdp='existing nearest-pair rule, both plus/minus tau changed together, amplitudes and 100ms pair window fixed',
            primary=PRIMARY,checkpoint='Two separate exact A/D replays at training end; independent 2s rest branches, no calibration on parent',
            no_copy='Exact replay with neutral frame and no action copies at both primary checkpoints; background control',
            secondary=['128 online trials starting at checkpoint','first16 balanced trials','sequential retention and interference','time-bin readout'],
            learning='M6 count-readout chance/control/mapping criteria plus one-sided exact paired sign tests, alpha0.05/(13*2) for all conjunctive contrasts at each primary endpoint',
            inference='seed-paired 95% bootstrap descriptive; exact sign tests on nonzero paired differences with ties omitted; ten seeds required',
            limits='two A/D examples per checkpoint/mapping/seed limit power; extra spikes alone are not learning; no generic optimal-parameter claim'))
def engine_config(config):
    c=m6.cases(config['seed'],(config['train_trials'],config['test_trials'],config['calibration_trials']))[3]
    c['base']['case_id']=config['case_id'];c['base']['params']['homeostasis']=False
    return c
def validate(r,config,path=None):
    audit=m6.validate(r,engine_config(config),path,protocol='NERVA-M8-v1',requested_config=config,stages=STAGES,checkpoint_rest_us=2000000,
        extra_branches={'no_copy_checkpoint_probe':100000,'no_copy_rest_checkpoint_probe':2100000})
    defaults=dict(core_weight_scale=1,core_threshold=1,core_fanout=4,core_delay_us=None,stdp_tau_us=20000,sensory_weight_scale=1,
        core_membrane_tau_us=20000,action_copy_weight=.9,homeostasis=False,development=False)
    if config['condition']['parameter']!='reference':defaults[config['condition']['parameter']]=config['condition']['value']
    for key,value in defaults.items():
        actual=r['effective_parameters'][key]
        m6.require(actual==value if value is None or type(value)==bool else abs(actual-value)<1e-6,'effective parameter differs: '+key)
    m6.require(r['condition']==config['condition'],'condition differs')
    m6.require(r['normalization']['mode']=='fixed_reference_gain' and r['normalization']['fixed_rate_gain']==r['reference_normalization']['fixed_rate_gain'],'rate gain changed')
    m6.require(abs(r['normalization']['initial_mean_current_per_core_second']-10*defaults['sensory_weight_scale'])<1e-4,'unexpected initial sensor current')
    edges={s['id']:s for s in r['initial_synapses']};core=set(range(2,34));nodes={n['id']:n for n in r['initial_neurons']}
    m6.require(set(nodes)==set(range(38)),'node identities changed')
    m6.require(all(abs(n['threshold']-(defaults['core_threshold'] if ident in core else 1))<1e-6 for ident,n in nodes.items()),'threshold applied outside Core')
    for arm in m6.DEFAULT_ARMS:
        for mapping in m6.MAPS:
            model=r['arms'][arm][mapping];weights=dict(model['trained_weights']);m6.require(set(weights)==set(edges),'trained weights missing')
            m6.require(all(math.isfinite(w) and 0<=w<=2 for w in weights.values()),'invalid trained weight')
            if arm in ['before','aged_frozen']:m6.require(all(w==edges[i]['weight'] for i,w in weights.items()),'frozen weights changed')
            for stage,rest in [('checkpoint_probe',0),('rest_checkpoint_probe',2000000)]:
                q=model['no_copy_probes'][stage]
                m6.require(q['exact_replay_state_before_rest'] is True and q['parent_unchanged'] is True,'no-copy replay mismatch')
                m6.require(q['start_us']==model['checkpoint_us']+rest+50000 and q['end_us']-q['start_us']==49999,'no-copy clock drift')
                m6.require(all(n in core and q['start_us']<=time<=q['end_us'] for n,time in q['core_events']),'invalid no-copy event')
    if path is None:return None
    details={}
    with Path(str(path)+'.details.csv').open(newline='') as f:
        for row in csv.DictReader(f):details[row['arm'],row['mapping'],row['stage'],int(row['trial'])]=row
    seen=set();counts=defaultdict(list)
    with Path(str(path)+'.propagation.csv').open(newline='') as f:
        for row in csv.DictReader(f):
            ident=(row['arm'],row['mapping'],row['stage'],int(row['trial']));m6.require(ident in details and ident not in seen,'missing/duplicate propagation row');seen.add(ident);d=details[ident]
            targets=json.loads(row['direct_targets']);events=json.loads(row['core_events']);action=int(row['action_us']);forecast=int(row['forecast_us'])
            expected=[s['post'] for s in r['initial_synapses'] if s['pre']==(36 if row['key']=='a' else 37)]
            m6.require(targets==expected and len(set(targets))==3,'copy target identities differ')
            m6.require(all(n in core and action<=time<=forecast for n,time in events),'future/unknown Core event')
            extra=sum(n not in targets for n,time in events)
            m6.require(int(row['extra_core_spikes'])==extra and int(d['cue_core_spikes'])==len(events),'propagation/detail counts differ')
            m6.require((row['key'],row['action_us'],row['forecast_us'])==(d['key'],d['action_us'],d['forecast_us']),'propagation trial metadata differs')
            baseline=None
            if row['stage'] in PRIMARY:
                no=r['arms'][row['arm']][row['mapping']+'_mapping']['no_copy_probes'][row['stage']]['core_events']
                baseline=sum(n not in targets for n,time in no)
            counts[row['arm'],row['mapping'],row['stage']].append(dict(extra=extra,total=len(events),no_copy_extra=baseline))
    m6.require(seen==set(details),'propagation coverage differs');audit['propagation_rows']=len(seen)
    return audit,counts
def sign_p(values):
    d=[x for x in values if x!=0];n=len(d);k=sum(x>0 for x in d)
    return sum(math.comb(n,j) for j in range(k,n+1))/2**n if n else 1.0
def contrasts(name,values,stage):
    passed,rows=m6.learning_contrasts(name,values,stage);exact=True
    for row in rows:
        if row['feature']=='time_bins':continue
        if row['mapping']=='both':
            p=[r['arms']['paired']['mapping_comparison'][stage]['count'][row['feature']] for r in values]
            q=[r['arms'][row['reference']]['mapping_comparison'][stage]['count'][row['feature']] for r in values]
        else:
            p=[r['arms']['paired'][row['mapping']][stage]['count']['balanced_accuracy'] for r in values]
            q=[.5]*len(values) if row['reference']=='chance' else [r['arms'][row['reference']][row['mapping']][stage]['count']['balanced_accuracy'] for r in values]
        row['one_sided_sign_p']=sign_p([a-b for a,b in zip(p,q)]);row['family_alpha']=.05/26
        exact=exact and row['one_sided_sign_p']<=.05/26
    for row in rows:
        row.setdefault('one_sided_sign_p','');row.setdefault('family_alpha','')
    return passed,passed and exact and stage in PRIMARY,rows
def report(root):
    manifest=json.loads((root/'manifest.json').read_text());records={};summaries=[];tests=[];audits=[];prop=[];weights=[]
    for template in manifest['conditions']:
        name=template['case_id'];values=[];all_counts=[]
        for seed in manifest['seeds']:
            config=dict(template,seed=seed);path=root/'results'/name/f'seed-{seed}.json';r=json.loads(path.read_text());a,c=validate(r,config,path);audits.append(dict(case=name,seed=seed,**a));values.append(r);all_counts.append(c)
            edges={e['id']:e for e in r['initial_synapses']};node={n['id']:n for n in r['initial_neurons']}
            action={key:{e['post'] for e in edges.values() if e['pre']==ident} for key,ident in [('a',36),('d',37)]}
            visual={label:{e['post'] for e in edges.values() if e['pre']==ident} for label,ident in [(-1,0),(1,1)]}
            for arm in m6.DEFAULT_ARMS:
                alignment=[]
                for mapping in m6.MAPS:
                    trained=dict(r['arms'][arm][mapping]['trained_weights']);deltas={i:trained[i]-e['weight'] for i,e in edges.items()};score=0
                    for key in ['a','d']:
                        target=(-1 if key=='a' else 1)*(1 if mapping=='normal_mapping' else -1)
                        for i,e in edges.items():
                            if e['pre'] in action[key] and e['post'] in range(2,34) and e['plastic'] and node[e['pre']]['polarity']=='Excitatory':score+=deltas[i]*(int(e['post'] in visual[target])-int(e['post'] in visual[-target]))
                    alignment.append(score)
                weights.append(dict(case=name,seed=seed,arm=arm,alignment=statistics.mean(alignment),
                    core_mapping_l1=sum(abs(dict(r['arms'][arm]['normal_mapping']['trained_weights'])[i]-dict(r['arms'][arm]['reversed_mapping']['trained_weights'])[i]) for i,e in edges.items() if e['pre'] in range(2,34) and e['post'] in range(2,34)),
                    visual_initial_sum=sum(e['weight'] for e in edges.values() if e['pre']<2),
                    visual_trained_sum=statistics.mean(sum(w for i,w in r['arms'][arm][mapping]['trained_weights'] if edges[i]['pre']<2) for mapping in m6.MAPS)))
        records[name]=values;row=dict(case=name,seeds=len(values),visual=statistics.mean(r['arms']['paired']['visual_decodability'] for r in values))
        row['initial_sensor_current']=statistics.mean(r['normalization']['initial_mean_current_per_core_second'] for r in values)
        for stage in STAGES:
            row[stage]=statistics.mean(r['arms']['paired'][m][stage]['count']['balanced_accuracy'] for r in values for m in m6.MAPS)
            screen,confirmed,extra=contrasts(name,values,stage);row[stage+'_screen']=screen;row[stage+'_confirmed']=confirmed;tests+=extra
        for stage in PRIMARY:
            for arm in m6.DEFAULT_ARMS:
                measurements=[q for counts in all_counts for mapping in ['normal','reversed'] for q in counts[arm,mapping,stage]]
                prop.append(dict(case=name,stage=stage,arm=arm,windows=len(measurements),mean_core_spikes=statistics.mean(q['total'] for q in measurements),mean_extra_core_spikes=statistics.mean(q['extra'] for q in measurements),
                    mean_no_copy_extra_spikes=statistics.mean(q['no_copy_extra'] for q in measurements),mean_action_extra_difference=statistics.mean(q['extra']-q['no_copy_extra'] for q in measurements),
                    windows_with_extra_spikes=sum(q['extra']>0 for q in measurements),windows_with_action_excess=sum(q['extra']>q['no_copy_extra'] for q in measurements),max_extra_core_spikes=max(q['extra'] for q in measurements)))
        summaries.append(row)
    reference=records['reference']
    for name,values in records.items():
        for r,base in zip(values,reference):
            for arm in m6.DEFAULT_ARMS:
                for mapping in m6.MAPS:
                    a=r['arms'][arm][mapping]['stage_diagnostics'];b=base['arms'][arm][mapping]['stage_diagnostics']
                    m6.require({s:v['sensor']['generated_sensor_spikes'] for s,v in a.items()}=={s:v['sensor']['generated_sensor_spikes'] for s,v in b.items()},'generated visual inputs changed across conditions')
            param=r['condition']['parameter'];scale=r['effective_parameters']['sensory_weight_scale'];cs=r['effective_parameters']['core_weight_scale'];delay=r['effective_parameters']['core_delay_us']
            edges={e['id']:e for e in r['initial_synapses']};base_edges={e['id']:e for e in base['initial_synapses']}
            m6.require(set(base_edges)<=set(edges),'density removed original scaffold')
            for ident,e in base_edges.items():
                a=edges[ident];m6.require(all(a[k]==e[k] for k in ['id','pre','post','plastic']),'scaffold wiring changed')
                core=e['pre'] in range(2,34) and e['post'] in range(2,34);factor=cs if core else (scale if e['pre']<2 else 1)
                m6.require(abs(a['weight']-e['weight']*factor)<1e-6 and a['delay_us']==(delay if core and delay is not None else e['delay_us']),'unrelated initial weight/delay changed')
            m6.require(all(a['position']==b['position'] and a['polarity']==b['polarity'] for a,b in zip(r['initial_neurons'],base['initial_neurons'])),'cell geometry/polarity changed')
            if param!='core_fanout':m6.require(set(edges)==set(base_edges),'unexpected added edges')
    for filename,rows in [('summary.csv',summaries),('prediction-contrasts.csv',tests),('propagation-summary.csv',prop),('weight-summary.csv',weights)]:
        with (root/filename).open('w',newline='') as f:w=csv.DictWriter(f,fieldnames=list(rows[0]));w.writeheader();w.writerows(rows)
    alignment_contrasts=[]
    for name in records:
        if name=='reference':continue
        def vector(case):return [next(w['alignment'] for w in weights if w['case']==case and w['arm']=='paired' and w['seed']==seed) for seed in manifest['seeds']]
        alignment_contrasts.append(dict(case=name,reference='reference',metric='paired_topological_weight_alignment',**m6.bootstrap_paired(vector(name),vector('reference'))))
    m6.json_out(root/'diagnostics.json',dict(audits=audits,total={key:sum(a[key] for a in audits) for key in ['trial_rows','phase_rows','sensor_rows','branch_rows','propagation_rows']},
        matched_visual_generation=True,original_scaffold_preserved=True,exact_sign_family_alpha=.05/26,descriptive_alignment_contrasts=alignment_contrasts))
    lines=['# M8 – gemessene Ergebnisse','',f"Stufe {manifest['stage']}; Seeds {manifest['seeds']}. Alle Bedingungen verwenden bestehende Lernregeln und dieselben vier Kontrollarme.",'',
        '| Fall | Sofort-Checkpoint | Checkpoint nach 2s Ruhe | Online | Visuell | Extra-Spikes sofort / ohne Kopie | Extra-Spikes nach Ruhe / ohne Kopie |',
        '|---|---:|---:|---:|---:|---:|---:|']
    for row in summaries:
        p=[next(x for x in prop if x['case']==row['case'] and x['stage']==stage and x['arm']=='paired') for stage in PRIMARY]
        lines.append(f"| {row['case']} | {row['checkpoint_probe']:.1%} | {row['rest_checkpoint_probe']:.1%} | {row['probe']:.1%} | {row['visual']:.1%} | {p[0]['mean_extra_core_spikes']:.2f} / {p[0]['mean_no_copy_extra_spikes']:.2f} | {p[1]['mean_extra_core_spikes']:.2f} / {p[1]['mean_no_copy_extra_spikes']:.2f} |")
    for stage in PRIMARY:lines+=['',stage+' – Kriterien einschließlich Familienkorrektur: '+(', '.join(r['case'] for r in summaries if r[stage+'_confirmed']) or 'keine')+'.']
    lines+=['','[Kontroll-/Belegungskontraste](prediction-contrasts.csv) · [Weiterleitung und Kontrollarme](propagation-summary.csv) · [Gewichtsdiagnose](weight-summary.csv) · [Unabhängiger Datenaudit](diagnostics.json).',
        '', 'Spikezahlen außerhalb der direkten Kopie-Zielzellen sind getrennt von der Vorhersageleistung zu bewerten. Prüfzeitpunkte sind getrennte Endpunkte; 95%-Bootstrapintervalle sind deskriptiv. Primär wird zusätzlich jeder gerichtete Kontrollkontrast durch einen exakten gepaarten Vorzeichentest mit Bonferroni über 13×2 Endpunkte geprüft.']
    (root/'summary.md').write_text('\n'.join(lines)+'\n');print('Report:',root/'summary.md')
def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('mode',choices=['plan','run','report']);p.add_argument('--stage',choices=['pilot','confirm'],default='pilot');p.add_argument('--output',type=Path,required=True);p.add_argument('--jobs',type=int,default=4)
    p.add_argument('--train-trials',type=int,default=256);p.add_argument('--test-trials',type=int,default=128);p.add_argument('--calibration-trials',type=int,default=128);args=p.parse_args();root=args.output
    if args.mode=='report':report(root);return
    root.mkdir(parents=True,exist_ok=True);manifest=plan(args.stage,(args.train_trials,args.test_trials,args.calibration_trials));path=root/'manifest.json'
    if path.exists():m6.require(json.loads(path.read_text())==manifest,'manifest differs; choose new directory')
    m6.json_out(path,manifest)
    if args.mode=='plan':print('Plan:',path);return
    m6.require(1<=args.jobs<=16,'jobs must be 1..16')
    def execute(item):
        template,seed=item;config=dict(template,seed=seed);out=root/'results'/config['case_id']/f'seed-{seed}.json'
        if out.exists():validate(json.loads(out.read_text()),config,out);return f"VALID {config['case_id']} seed {seed}"
        cfg=root/'configs'/config['case_id']/f'seed-{seed}.json';m6.json_out(cfg,config);out.parent.mkdir(parents=True,exist_ok=True)
        proc=subprocess.run([str(Path(__file__).resolve().parents[2] / 'Experimente/run.sh'),'m8','--config',str(cfg),'--output',str(out)],capture_output=True,text=True);out.with_suffix('.log').write_text(proc.stdout+proc.stderr)
        m6.require(proc.returncode==0,f"failed {config['case_id']} seed{seed}: {proc.stderr[-1200:]}");validate(json.loads(out.read_text()),config,out);return f"OK {config['case_id']} seed {seed}"
    with ThreadPoolExecutor(max_workers=args.jobs) as pool:
        for status in pool.map(execute,[(c,s) for c in manifest['conditions'] for s in manifest['seeds']]):print(status,flush=True)
    report(root)
if __name__=='__main__':main()
