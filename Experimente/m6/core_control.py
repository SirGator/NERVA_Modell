#!/usr/bin/env python3
"""Exploratory Core-homeostasis ablation, separate from the original M6 matrix."""
from argparse import ArgumentParser
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
import csv,json,statistics,subprocess
import run as m6

def make_plan():
    template=m6.cases(0)[3]
    rows=[]
    for enabled in [True,False]:
        base=dict(template['base'],case_id='core_D_'+('on' if enabled else 'off'),
            params=dict(template['base']['params'],homeostasis=enabled))
        rows.append(dict(template,base=base))
    return dict(protocol='NERVA-M6-core-control-v1',stage='exploratory follow-up after M6',seeds=list(range(400,410)),
        conditions=rows,hypothesis='Core intrinsic-current regulation suppresses burst integration after training',
        fixed='diag_D: effect_2ch/32, three copies, visual-homeostasis off, tau20ms, weight0.9, delay50ms; 256/128/128 trials',
        endpoint='Same M6 chance/control/mapping criteria, plus actual cue Core spikes; no tuning using these seeds')

def report(root):
    plan=json.loads((root/'manifest.json').read_text());records={};audits=[];rows=[];contrasts=[]
    for template in plan['conditions']:
        name=template['base']['case_id'];values=[]
        for seed in plan['seeds']:
            config=dict(template,base=dict(template['base'],seed=seed));path=root/'results'/name/f'seed-{seed}.json'
            r=json.loads(path.read_text());audits.append(dict(case=name,seed=seed,**m6.validate(r,config,path)));values.append(r)
        records[name]=values;row=dict(case=name,seeds=len(values),visual=statistics.mean(r['arms']['paired']['visual_decodability'] for r in values))
        for stage in m6.STAGES:
            row[stage]=statistics.mean(r['arms']['paired'][mapping][stage]['count']['balanced_accuracy'] for r in values for mapping in m6.MAPS)
            passed,extra=m6.learning_contrasts(name,values,stage);row[stage+'_evidence']=passed;contrasts.extend(extra)
        spikes=[];silent=0
        for seed in plan['seeds']:
            path=root/'results'/name/f'seed-{seed}.json.details.csv'
            with path.open(newline='') as f:
                for line in csv.DictReader(f):
                    if line['arm']=='paired' and line['stage']=='probe':
                        n=int(line['cue_core_spikes']);spikes.append(n);silent+=n==0
        row.update(cue_core_spikes_mean=statistics.mean(spikes),cue_windows=len(spikes),silent_cue_windows=silent);rows.append(row)
    for on,off in zip(records['core_D_on'],records['core_D_off']):
        m6.require(on['initial_network_fingerprint']==off['initial_network_fingerprint'] and on['normalization']==off['normalization'],'Core ablation changed initial network/input budget')
        for arm in ['before','aged_frozen']:m6.require(on['arms'][arm]==off['arms'][arm],'frozen Core-ablation controls differ')
    diffs={stage:m6.bootstrap_paired([
        statistics.mean(r['arms']['paired'][mapping][stage]['count']['balanced_accuracy'] for mapping in m6.MAPS) for r in records['core_D_off']],
        [statistics.mean(r['arms']['paired'][mapping][stage]['count']['balanced_accuracy'] for mapping in m6.MAPS) for r in records['core_D_on']]) for stage in m6.STAGES}
    for filename,data in [('summary.csv',rows),('prediction-contrasts.csv',contrasts)]:
        with (root/filename).open('w',newline='') as f:w=csv.DictWriter(f,fieldnames=list(data[0]));w.writeheader();w.writerows(data)
    m6.json_out(root/'diagnostics.json',dict(audits=audits,off_minus_on=diffs,initial_networks_and_frozen_controls_equal=True))
    lines=['# M6: zusätzliche Core-Homöostase-Kontrolle','', 'Explorative, getrennt geplante Folgeprüfung auf neuen Seeds 400–409. Beide Bedingungen verwenden die kombinierte Variante D.','',
        '| Core-Homöostase | Checkpoint | Online | Nach Ruhe | Nach Interferenz | Visuell | Core-Spikes im Aktionsfenster | Stille Fenster |',
        '|---|---:|---:|---:|---:|---:|---:|---:|']
    for r in rows:lines.append(f"| {r['case']} | {100*r['checkpoint_probe']:.1f}% | {100*r['probe']:.1f}% | {100*r['retained_probe']:.1f}% | {100*r['interference_probe']:.1f}% | {100*r['visual']:.1f}% | {r['cue_core_spikes_mean']:.2f} | {r['silent_cue_windows']}/{r['cue_windows']} |")
    for stage in m6.STAGES:lines+=['',stage+' – bestätigte Kriterien: '+(', '.join(r['case'] for r in rows if r[stage+'_evidence']) or 'keine')+'.']
    lines+=['','[Gepaarte Kontraste](prediction-contrasts.csv) · [Datenaudit](diagnostics.json). Deskriptive Bootstrap-Intervalle, keine Korrektur für multiple Vergleiche.']
    (root/'summary.md').write_text('\n'.join(lines)+'\n');print('Report:',root/'summary.md')

def main():
    p=ArgumentParser(description=__doc__);p.add_argument('mode',choices=['plan','run','report']);p.add_argument('--output',type=Path,required=True);p.add_argument('--jobs',type=int,default=4);args=p.parse_args();root=args.output
    if args.mode=='report':report(root);return
    root.mkdir(parents=True,exist_ok=True);plan=make_plan();manifest=root/'manifest.json'
    if manifest.exists():m6.require(json.loads(manifest.read_text())==plan,'manifest differs')
    m6.json_out(manifest,plan)
    if args.mode=='plan':print('Plan written:',manifest);return
    m6.require(1<=args.jobs<=32,'jobs must be 1..32')
    def execute(item):
        template,seed=item;config=dict(template,base=dict(template['base'],seed=seed));name=config['base']['case_id'];out=root/'results'/name/f'seed-{seed}.json'
        if out.exists():m6.validate(json.loads(out.read_text()),config);return f'VALID {name} seed {seed}'
        cfg=root/'configs'/name/f'seed-{seed}.json';m6.json_out(cfg,config);out.parent.mkdir(parents=True,exist_ok=True)
        proc=subprocess.run([str(Path(__file__).resolve().parents[2] / 'Modell/run-demo.sh'),'m6','--config',str(cfg),'--output',str(out)],text=True,capture_output=True)
        out.with_suffix('.stdout.log').write_text(proc.stdout);out.with_suffix('.stderr.log').write_text(proc.stderr)
        m6.require(proc.returncode==0,proc.stderr[-2000:]);m6.validate(json.loads(out.read_text()),config);return f'OK {name} seed {seed}'
    with ThreadPoolExecutor(max_workers=args.jobs) as pool:
        for message in pool.map(execute,[(c,s) for c in plan['conditions'] for s in plan['seeds']]):print(message,flush=True)
    report(root)
if __name__=='__main__':main()
