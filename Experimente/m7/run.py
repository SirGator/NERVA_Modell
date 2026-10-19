#!/usr/bin/env python3
"""Exploratory replay audit of the existing M6 Core-off seeds; no new learning rule."""
import argparse
from collections import Counter,defaultdict
from concurrent.futures import ThreadPoolExecutor
import csv,hashlib,json,math,statistics,struct,subprocess,sys
from pathlib import Path
sys.path.insert(0,str(Path(__file__).resolve().parents[1]/'m4'))
from runner import bootstrap_paired

ARMS=['aged_frozen','paired','scrambled']
MAPS=['normal_mapping','reversed_mapping']
CONDITIONS=['sham','restore_core','restore_all_plastic','block_core','no_copy']
def require(ok,message):
    if not ok:raise ValueError(message)
def f32(x):return struct.unpack('f',struct.pack('f',float(x)))[0]
def save(path,value):path.write_text(json.dumps(value,indent=2,sort_keys=True)+'\n')
def reference_path(value):
    path=Path(value)
    if path.exists():return path.resolve()
    # Historical manifests retain their original paths and reference hashes.
    parts=path.parts
    for i in range(len(parts)-1):
        if parts[i:i+2]==('Experimente','runs'):
            return Path(__file__).resolve().parents[3]/'Messungen/Experimente'/Path(*parts[i+2:])
    return path.resolve()

def normalized_manifest(manifest):
    return dict(manifest,references=[dict(ref,path=str(reference_path(ref['path']))) for ref in manifest['references']])

def plan(reference_root):
    references=[]
    for seed in range(400,410):
        p=reference_root/'results/core_D_off'/f'seed-{seed}.json'
        references.append(dict(seed=seed,path=str(p),sha256=hashlib.sha256(p.read_bytes()).hexdigest()))
    return dict(protocol='NERVA-M7-synapse-audit-v1',stage='exploratory mechanism audit, existing M6 seeds',references=references,
        fixed=dict(arms=ARMS,mappings=MAPS,conditions=CONDITIONS,train_trials=256,processing=32,sensor='effect_2ch',
            copies=3,weight=.9,tau_us=20000,delay_us=50000,homeostasis=False,development=False,rest_us=2000000,interference_us=1400000),
        endpoints=['per-synapse positive/negative weight changes and nearest pre/post partners',
            'same-trial action-to-effect and cross-trial pairs, separately for LTP and LTD',
            'mapping-dependent weight differences and direct action/visual topology proxy versus scrambled/frozen',
            'frozen weight retention; explicitly not consolidation',
            'Core spike/event/potential changes after restoring initial Core/all-plastic weights or blocking Core recurrence',
            'all counterfactuals start at exact executable replay state after 2s neutral rest'],
        limits=['Reuses all ten existing seeds; no new confirmation or parameter selection',
            'Initial connectivity is unchanged; topology proxy is not a learned functional module',
            'One A/D probe per counterfactual/mapping/seed, not independent repeated trials',
            'Frozen retention and counterfactual response alone do not establish effect learning'])

def validate(root):
    r=json.loads((root/'result.json').read_text());require(r['protocol']=='NERVA-M7-synapse-audit-v1','wrong M7 protocol')
    require(all(r['checks'].values()),'runtime checks failed')
    topology=r['initial_topology'];edges={e['id']:e for e in topology['synapses']};core=set(topology['processing_ids'])
    groups={(a,m):{} for a in ARMS for m in MAPS};snapshot_rows=0
    with (root/'weights.csv').open(newline='') as f:
        for row in csv.DictReader(f):
            snapshot_rows+=1;group=(row['arm'],row['mapping']);name=row['checkpoint'];ident=int(row['synapse']);require(group in groups and ident in edges,'unknown snapshot group/edge')
            entry=groups[group].setdefault(name,dict(time=int(row['time_us']),weights={}))
            require(entry['time']==int(row['time_us']) and ident not in entry['weights'],'duplicate/drifting snapshot')
            entry['weights'][ident]=f32(row['weight'])
    run_lookup={(run['arm'],run['mapping']):run for run in r['runs']};require(set(run_lookup)==set(groups),'missing/duplicate run')
    for group,snapshots in groups.items():
        n=r['requested_config']['base']['train_trials'];final='train_'+str(n)
        expected=['initial']+['train_'+str(i) for i in range(64,n,64)]+[final,'frozen','rest_2s','interference_1_4s']
        require(set(snapshots)==set(expected),'missing checkpoint')
        for entry in snapshots.values():require(set(entry['weights'])==set(edges),'incomplete weight snapshot')
        require(snapshots['initial']['weights']=={i:f32(e['weight']) for i,e in edges.items()},'initial weights differ')
        require(all(snapshots[k]['weights']==snapshots[final]['weights'] for k in ['frozen','rest_2s','interference_1_4s']),'frozen weights drifted')
    summaries=[];current=None;pre={};post={};ledger={};seq=0;counts=Counter();stats={};pending=[]
    pair_rows=0;event_rows=0
    def finish():
        if current is None:return
        for checkpoint in pending:require(checkpoint['weights']==ledger,'snapshot differs from event ledger')
        actual=run_lookup[current]
        for key,value in [('events',seq),('weight_updates',counts['weight']),('core_spikes',counts['core_spike']),('copy_arrivals',counts['copy_arrival'])]:require(actual[key]==value,'event/JSON total differs: '+key)
        require(current[0]!='aged_frozen' or counts['weight']==0,'frozen updates')
        summaries.extend(stats.values())
    with (root/'pairs.csv').open(newline='') as pair_file, (root/'events.csv').open(newline='') as f:
        pairs=iter(csv.DictReader(pair_file))
        for row in csv.DictReader(f):
            event_rows+=1;group=(row['arm'],row['mapping']);time=int(row['time_us']);kind=row['event']
            if group!=current:
                finish();current=group;require(group in groups,'unknown event group');pre={};post={};ledger=dict(groups[group]['initial']['weights']);seq=0;counts=Counter();stats={};pending=sorted(groups[group].values(),key=lambda x:x['time'])
            require(int(row['seq'])==seq,'missing/duplicate event sequence');seq+=1;counts[kind]+=1
            while pending and time>=pending[0]['time']:require(pending.pop(0)['weights']==ledger,'checkpoint/event weights disagree')
            mark=(time,int(row['trial']),row['phase'],row['key'],int(row['effect']))
            if kind=='arrival':
                ident=int(row['synapse']);edge=edges[ident];require((int(row['source']),int(row['target']))==(edge['pre'],edge['post']),'wrong arrival endpoints')
                pre[ident]=mark;counts['copy_arrival']+=int(edge['pre'] in [36,37])
            elif kind=='spike':post[int(row['source'])]=mark;counts['core_spike']+=int(int(row['source']) in core)
            elif kind=='weight':
                pair_rows+=1;p=next(pairs,None);require(p is not None,'missing pair row')
                ident=int(row['synapse']);edge=edges[ident];require(edge['accepted_by_stdp'],'ineligible STDP update')
                require((p['arm'],p['mapping'],int(p['seq']),int(p['synapse']))==(*group,seq-1,ident),'pair/event order differs')
                a,b=pre[ident],post[edge['post']]
                for prefix,partner in [('pre',a),('post',b)]:
                    require((int(p[prefix+'_us']),int(p[prefix+'_trial']),p[prefix+'_phase'],p[prefix+'_key'],int(p[prefix+'_effect']))==partner,'not nearest recorded partner')
                old,new=f32(row['old_weight']),f32(row['new_weight']);require(ledger[ident]==old,'old weight differs from ledger')
                require(f32(p['old_weight'])==old and f32(p['new_weight'])==new and f32(p['expected_weight'])==new,'pair weights disagree')
                delta=b[0]-a[0];require(int(p['delta_us'])==delta and abs(delta)<=100000,'invalid pair delay')
                ltp=new>old;require(old!=new and p['kind']==('ltp' if ltp else 'ltd') and (b[0] if ltp else a[0])==time and (delta>=0 if ltp else delta<=0),'wrong pair trigger')
                expected=f32(min(2,max(0,old+f32((.0001 if ltp else -.00012)*math.exp(-abs(delta)/20000)))))
                require(abs(expected-new)<=1e-7,'offline pair equation mismatch');ledger[ident]=new
                family='core_core' if edge['pre'] in core and edge['post'] in core else ('visual_core' if edge['pre']<2 else 'core_motor')
                temporal='same_trial_action_effect' if a[1]==b[1] and a[2]=='action' and b[2]=='effect' else ('cross_trial' if a[1]!=b[1] else 'other_same_trial')
                key=(*group,family,p['kind'],temporal)
                s=stats.setdefault(key,dict(arm=group[0],mapping=group[1],family=family,kind=p['kind'],temporal=temporal,updates=0,delta_weight_sum=0,abs_delay_us_sum=0))
                s['updates']+=1;s['delta_weight_sum']+=new-old;s['abs_delay_us_sum']+=abs(delta)
        finish();require(next(pairs,None) is None,'extra pair rows')
    probes=0
    for group,run in run_lookup.items():
        seen=set()
        for p in run['counterfactuals']:
            probes+=1;key=(p['condition'],p['response']['key']);require(key not in seen,'duplicate counterfactual');seen.add(key)
            require(p['same_replay_state_before_intervention'] is True and p['response']['effect_not_presented'] is True,'invalid counterfactual boundary')
            require(len(p['response']['core_counts'])==len(core) and len(p['response']['core_potentials'])==len(core),'wrong response dimension')
            counts=Counter(n for n,t in p['response']['core_events']);require([counts[n] for n in topology['processing_ids']]==p['response']['core_counts'],'spike counts/events differ')
            require(all(p['response']['action_us']<=time<=p['response']['forecast_us'] for n,time in p['response']['core_events']),'future event in response')
            require(p['response']['copy_arrivals']==(0 if p['condition']=='no_copy' else 9),'copy control differs')
        require(seen=={(c,k) for c in CONDITIONS for k in ['a','d']},'missing counterfactual')
    return r,groups,summaries,dict(event_rows=event_rows,pair_rows=pair_rows,snapshot_rows=snapshot_rows,counterfactuals=probes)

def summarize(r,groups):
    topo=r['initial_topology'];core=set(topo['processing_ids']);edges=topo['synapses'];rows=[];effects=[];counter=[]
    visual={label:{e['post'] for e in edges if e['pre']==channel} for channel,label in enumerate([-1,1])}
    action={key:{e['post'] for e in edges if e['pre']==copy} for key,copy in [('a',36),('d',37)]}
    for arm in ARMS:
        for mapping in MAPS:
            initial=groups[arm,mapping]['initial']['weights'];trained=groups[arm,mapping]['train_256']['weights'];alignment=0;eligible=0
            for family,selected in [('core_core',[e for e in edges if e['pre'] in core and e['post'] in core]),('visual_core',[e for e in edges if e['pre']<2]),('core_motor',[e for e in edges if e['pre'] in core and e['post'] not in core])]:
                deltas=[trained[e['id']]-initial[e['id']] for e in selected]
                rows.append(dict(seed=r['seed'],arm=arm,mapping=mapping,family=family,edges=len(deltas),potentiated=sum(d>0 for d in deltas),depressed=sum(d<0 for d in deltas),unchanged=sum(d==0 for d in deltas),delta_sum=sum(deltas),mean_abs_delta=statistics.mean(map(abs,deltas)),max_abs_delta=max(map(abs,deltas)),at_zero=sum(trained[e['id']]==0 for e in selected)))
            for key in ['a','d']:
                associated=(-1 if key=='a' else 1)*(1 if mapping=='normal_mapping' else -1)
                for e in edges:
                    sign=int(e['post'] in visual[associated])-int(e['post'] in visual[-associated])
                    if e['accepted_by_stdp'] and e['pre'] in action[key] and e['pre'] in core and e['post'] in core and sign:
                        alignment+=sign*(trained[e['id']]-initial[e['id']]);eligible+=1
            effects.append(dict(seed=r['seed'],arm=arm,mapping=mapping,direct_weight_alignment=alignment,eligible_action_visual_edges=eligible,
                mapping_core_weight_l1=sum(abs(groups[arm,'normal_mapping']['train_256']['weights'][e['id']]-groups[arm,'reversed_mapping']['train_256']['weights'][e['id']]) for e in edges if e['pre'] in core and e['post'] in core)))
    for run in r['runs']:
        for key in ['a','d']:
            probes={p['condition']:p for p in run['counterfactuals'] if p['response']['key']==key};sham=probes['sham']['response']
            for condition,p in probes.items():
                q=p['response'];extra=sum(count for neuron,count in zip(topo['processing_ids'],q['core_counts']) if neuron not in action[key])
                counter.append(dict(seed=r['seed'],arm=run['arm'],mapping=run['mapping'],key=key,condition=condition,weights_changed=p['weights_changed'],core_spikes=sum(q['core_counts']),spikes_outside_copy_targets=extra,
                    changed_count_neurons=sum(a!=b for a,b in zip(sham['core_counts'],q['core_counts'])),spike_events_changed=q['core_events']!=sham['core_events'],max_abs_potential_change=max(abs(a-b) for a,b in zip(sham['core_potentials'],q['core_potentials']))))
    return rows,effects,counter

def csv_out(path,rows):
    with path.open('w',newline='') as f:
        writer=csv.DictWriter(f,fieldnames=list(rows[0]));writer.writeheader();writer.writerows(rows)
def report(root):
    manifest=json.loads((root/'manifest.json').read_text());all_rows=[];all_effects=[];all_counter=[];all_pairs=[];all_edges=[];audits=[]
    for reference in manifest['references']:
        r,groups,pairs,audit=validate(root/'results'/f"seed-{reference['seed']}")
        require(r['seed']==reference['seed'],'wrong seed');source=reference_path(reference['path']);require(hashlib.sha256(source.read_bytes()).hexdigest()==reference['sha256'],'M6 reference changed')
        require(r['requested_config']==json.loads(source.read_text())['requested_config'],'reference config differs')
        rows,effects,counter=summarize(r,groups);all_rows+=rows;all_effects+=effects;all_counter+=counter
        core=set(r['initial_topology']['processing_ids'])
        for arm in ARMS:
            for edge in r['initial_topology']['synapses']:
                initial=groups[arm,'normal_mapping']['initial']['weights'][edge['id']]
                normal=groups[arm,'normal_mapping']['train_256']['weights'][edge['id']]
                reverse=groups[arm,'reversed_mapping']['train_256']['weights'][edge['id']]
                all_edges.append(dict(seed=r['seed'],arm=arm,synapse=edge['id'],pre=edge['pre'],post=edge['post'],
                    accepted_by_stdp=edge['accepted_by_stdp'],core_core=edge['pre'] in core and edge['post'] in core,
                    initial_weight=initial,normal_weight=normal,reversed_weight=reverse,normal_delta=normal-initial,
                    reversed_delta=reverse-initial,normal_minus_reversed=normal-reverse))
        all_pairs.extend(dict(seed=r['seed'],**p) for p in pairs);audits.append(dict(seed=r['seed'],**audit))
    for name,rows in [('weights-summary.csv',all_rows),('mapping-summary.csv',all_effects),('counterfactuals.csv',all_counter),('pair-summary.csv',all_pairs),('edge-comparison.csv',all_edges)]:csv_out(root/name,rows)
    contrasts=[]
    for reference in ['aged_frozen','scrambled']:
        def values(arm):return [statistics.mean(row['direct_weight_alignment'] for row in all_effects if row['arm']==arm and row['seed']==ref['seed']) for ref in manifest['references']]
        contrasts.append(dict(metric='direct_weight_alignment',reference=reference,**bootstrap_paired(values('paired'),values(reference))))
    save(root/'diagnostics.json',dict(audits=audits,total={key:sum(a[key] for a in audits) for key in ['event_rows','pair_rows','snapshot_rows','counterfactuals']},alignment_contrasts=contrasts))
    lines=['# M7 – gemessene Synapsendiagnose','', 'Explorative Replays aller vorhandenen Core-off-Seeds 400–409; keine neue Lernregel und kein neuer funktionaler Lernnachweis.','',
        '| Arm | Core-Kanten verstärkt | abgeschwächt | unverändert | Mittlere absolute Änderung | Größte absolute Änderung |', '|---|---:|---:|---:|---:|---:|']
    for arm in ARMS:
        rows=[row for row in all_rows if row['arm']==arm and row['family']=='core_core']
        lines.append(f"| {arm} | {statistics.mean(x['potentiated'] for x in rows):.2f} | {statistics.mean(x['depressed'] for x in rows):.2f} | {statistics.mean(x['unchanged'] for x in rows):.2f} | {statistics.mean(x['mean_abs_delta'] for x in rows):.6f} | {max(x['max_abs_delta'] for x in rows):.6f} |")
    lines+=['','| Eingriff, gekoppelter Arm | Core-Spikes | Spikes außerhalb der Kopie-Zielzellen | Veränderte Spikefolgen gegenüber Sham | Maximale Potenzialänderung |','|---|---:|---:|---:|---:|']
    for condition in CONDITIONS:
        rows=[row for row in all_counter if row['arm']=='paired' and row['condition']==condition]
        lines.append(f"| {condition} | {statistics.mean(x['core_spikes'] for x in rows):.3f} | {statistics.mean(x['spikes_outside_copy_targets'] for x in rows):.3f} | {sum(x['spike_events_changed'] for x in rows)}/{len(rows)} | {max(x['max_abs_potential_change'] for x in rows):.6f} |")
    lines+=['','[Gewichte](weights-summary.csv) · [Zeitpaare](pair-summary.csv) · [Belegungsvergleich](mapping-summary.csv) · [Gegenprüfungen](counterfactuals.csv) · [Datenaudit](diagnostics.json).',
        '', 'Die direkte Gewichtszuordnung ist eine aus Anschlusszellen abgeleitete topologische Diagnose, kein funktionaler Lernnachweis. Gefrorene Gewichtsretention folgt aus dem Abschalten der Plastizität und belegt keine Konsolidierung. Bootstrapintervalle sind deskriptiv.']
    (root/'summary.md').write_text('\n'.join(lines)+'\n');print('Report:',root/'summary.md')

def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('mode',choices=['plan','run','report']);p.add_argument('--output',type=Path,required=True);p.add_argument('--references',type=Path,default=Path(__file__).resolve().parents[3] / 'Messungen/Experimente/m6-core-control');p.add_argument('--jobs',type=int,default=4);args=p.parse_args();root=args.output
    if args.mode=='report':report(root);return
    root.mkdir(parents=True,exist_ok=True);manifest=plan(args.references);path=root/'manifest.json'
    if path.exists():
        existing=json.loads(path.read_text())
        require(normalized_manifest(existing)==normalized_manifest(manifest),'manifest differs; use a new directory')
        manifest=existing
    else:save(path,manifest)
    if args.mode=='plan':print('Plan:',path);return
    require(1<=args.jobs<=16,'jobs must be 1..16')
    def execute(ref):
        out=root/'results'/f"seed-{ref['seed']}"
        if (out/'result.json').exists():validate(out);return f"VALID seed {ref['seed']}"
        log=root/f"seed-{ref['seed']}.log"
        proc=subprocess.run([str(Path(__file__).resolve().parents[2] / 'Experimente/run.sh'),'m7','--reference',str(reference_path(ref['path'])),'--output',str(out)],capture_output=True,text=True);log.write_text(proc.stdout+proc.stderr)
        require(proc.returncode==0,f"M7 seed {ref['seed']}: {proc.stderr[-1200:]}");return f"OK seed {ref['seed']}"
    with ThreadPoolExecutor(max_workers=args.jobs) as pool:
        for status in pool.map(execute,manifest['references']):print(status,flush=True)
    report(root)
if __name__=='__main__':main()
