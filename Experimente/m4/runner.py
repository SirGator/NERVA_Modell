#!/usr/bin/env python3
"""NERVA M4 orchestrator, integrated from the supplied suite contract.

The supplied root-level package disappeared during integration. This copy
preserves its 20 conditions and public runner functions, adds parallel jobs,
and the newly requested initial 2/7-channel gate at 8/20 ms.
It never simulates neurons or fills missing results.
"""
import argparse
from concurrent.futures import ThreadPoolExecutor
import csv
import json
import math
import random
import shlex
import statistics
import subprocess
from pathlib import Path

MEASUREMENTS = Path(__file__).resolve().parents[3] / 'Messungen/Experimente'

DEFAULT_ARMS = ["before", "aged_frozen", "paired", "scrambled"]

def cases():
    base = dict(sensor="pixels", resolution=16, encoding="deterministic", processing=128,
                action_effect_delay_ms=8, motor_feedback=True, stdp=True, homeostasis=True,
                development=False, initial_drive=0, pruning_min_us=5000000,
                normalization="equal_input_current")
    rows = []
    def add(name, family, hypothesis, **overrides):
        rows.append(dict(id=name, family=family, hypothesis=hypothesis, config=dict(base, **overrides)))
    add("a01_minimal_2ch", "sensor", "Two explicit visible left/right channels", sensor="effect_2ch", resolution=0, processing=32)
    add("a02_position_7ch", "sensor", "Visible position code", sensor="position_7ch", resolution=0, processing=32)
    add("a03_pixels_8", "sensor", "Raw 8x8 image", resolution=8, processing=32)
    add("a04_pixels_16", "sensor", "Raw 16x16 image, small core", processing=32)
    add("a05_pixels_64", "sensor", "Raw 64x64 image, small core", resolution=64, processing=32)
    add("b01_core_128", "capacity", "Core 128 at 16x16")
    add("b02_core_256", "capacity", "Core 256 at 16x16", processing=256)
    add("b03_full_256", "capacity", "Core 256 at 64x64", resolution=64, processing=256)
    add("c01_poisson", "encoding", "Stochastic sensory coding", encoding="poisson")
    add("c02_on_off", "encoding", "ON/OFF change coding", encoding="on_off")
    add("d01_delay_2", "timing", "Actual action-effect delay 2 ms", action_effect_delay_ms=2)
    add("d02_delay_20", "timing", "Actual action-effect delay 20 ms", action_effect_delay_ms=20)
    add("d03_delay_50", "timing", "Actual action-effect delay 50 ms", action_effect_delay_ms=50)
    add("e01_no_feedback", "causality", "No motor feedback", motor_feedback=False)
    add("e02_no_stdp", "causality", "No synaptic learning", stdp=False)
    add("e03_with_homeostasis_off", "plasticity", "No homeostasis", homeostasis=False)
    add("e04_with_development", "plasticity", "Structural plasticity with zero start drive", development=True)
    add("e05_early_pruning", "plasticity", "Pruning grace 300 ms", development=True, pruning_min_us=300000)
    add("e06_start_drive", "plasticity", "Preloaded initial drive 2.5", development=True, initial_drive=2.5)
    add("f01_reference_64_32_poisson", "reference", "M3-like sensory complexity with the declared M4 normalizer", resolution=64, processing=32, encoding="poisson")
    return rows

def gate_cases():
    originals = cases()[:2]
    extra = []
    for case, name in zip(originals, ["g01_minimal_2ch_delay20", "g02_position_7ch_delay20"]):
        extra.append(dict(case, id=name, family="initial_gate", config=dict(case["config"], action_effect_delay_ms=20)))
    return originals + extra

def make_manifest(stage):
    if stage not in ("gate", "pilot", "confirm"):
        raise ValueError("unknown stage")
    return dict(protocol="NERVA-M4-v1", stage=stage,
                seeds=list(range(100,110)) if stage=="confirm" else [0,1,2],
                arms=DEFAULT_ARMS, cases=gate_cases() if stage=="gate" else cases(),
                checks=["No future sensory/action label leakage", "Bit-identical initial networks",
                        "Paired equals frozen replay; scrambled preserves action stream and image marginals",
                        "Independent calibration and probe streams", "Frozen probe", "Balanced labels",
                        "Measured expected and actual input current", "Controlled actions separate from decoder proposals"])

def json_out(path, obj):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(obj, indent=2, sort_keys=True, ensure_ascii=False)+"\n", encoding="utf-8")

def bootstrap_paired(x, y, seed=1234, n=4000):
    if not x or len(x)!=len(y):
        raise ValueError("paired vectors must have equal nonzero length")
    differences=[a-b for a,b in zip(x,y)]
    rng=random.Random(seed)
    means=sorted(sum(rng.choice(differences) for _ in differences)/len(differences) for _ in range(n))
    return dict(mean_difference=statistics.mean(differences), ci95=[means[int(n*.025)], means[min(n-1,int(n*.975))]])

def validate_record(obj, expected_seed, expected_case):
    if obj.get("seed")!=expected_seed or obj.get("case_id")!=expected_case:
        raise ValueError("wrong seed/case ID in result")
    if obj.get("protocol")!="NERVA-M4-v1":
        raise ValueError("wrong protocol")
    arms=obj.get("arms",{})
    if not set(DEFAULT_ARMS).issubset(arms):
        raise ValueError("missing control arms")
    for name in DEFAULT_ARMS:
        for key in ("balanced_accuracy","visual_decodability","action_selection_rate"):
            value=arms[name].get(key)
            if type(value) not in (int,float) or not math.isfinite(value) or not 0<=value<=1:
                raise ValueError(f"bad {name}.{key}")
        if type(arms[name].get("test_count")) is not int or arms[name]["test_count"]<1:
            raise ValueError(f"bad {name}.test_count")
    for check in ("same_initial_network","matched_experience","no_leakage","frozen_probe","balanced_labels"):
        if obj.get("checks",{}).get(check) is not True:
            raise ValueError("failed/missing audit check: "+check)
    return obj

def do_plan(args):
    manifest=make_manifest(args.stage)
    json_out(args.output,manifest)
    print(f"Created {args.output}: {len(manifest['cases'])} conditions, {len(manifest['seeds'])} paired seeds")

def do_run(args):
    root=args.output
    plan=make_manifest(args.stage)
    plan["trial_counts"]=dict(train_trials=args.train_trials,test_trials=args.test_trials,calibration_trials=args.calibration_trials)
    if args.cases:
        selected=set(args.cases.split(','))
        available={c['id']:c for c in cases()+gate_cases()[2:]}
        unknown=selected-set(available)
        if unknown:
            raise ValueError("Unknown case IDs: "+", ".join(sorted(unknown)))
        plan['cases']=[c for name,c in available.items() if name in selected]
    if args.jobs<1 or args.jobs>32:
        raise ValueError("jobs must be 1..32")
    root.mkdir(parents=True,exist_ok=True)
    manifest_path=root/'manifest.json'
    if manifest_path.exists() and json.loads(manifest_path.read_text())!=plan:
        raise ValueError("existing output has a different manifest; choose a new directory")
    json_out(manifest_path,plan)
    if not args.command:
        print("Manifest written. Supply --command to execute NERVA.")
        return
    def execute(item):
        case,seed=item
        out=root/'results'/case['id']/f'seed-{seed:02d}.json'
        if out.exists() and not args.force:
            record=validate_record(json.loads(out.read_text()),seed,case['id'])
            expected=dict(protocol='NERVA-M4-v1',case_id=case['id'],seed=seed,params=case['config'],arms=DEFAULT_ARMS,
                          train_trials=args.train_trials,test_trials=args.test_trials,calibration_trials=args.calibration_trials)
            if record.get('requested_config')!=expected:
                raise ValueError('Existing result has different or unverified input configuration; choose a new output directory')
            return f"VALID {case['id']} seed {seed}"
        if args.force and out.exists():
            raise ValueError("--force requires a fresh output directory to retain old measurement sidecars")
        cfg=root/'configs'/case['id']/f'seed-{seed:02d}.json'
        json_out(cfg,dict(protocol='NERVA-M4-v1',case_id=case['id'],seed=seed,params=case['config'],arms=DEFAULT_ARMS,
                          train_trials=args.train_trials,test_trials=args.test_trials,calibration_trials=args.calibration_trials))
        out.parent.mkdir(parents=True,exist_ok=True)
        command=args.command.replace('{config}',shlex.quote(str(cfg))).replace('{output}',shlex.quote(str(out)))
        proc=subprocess.run(command,shell=True,text=True,capture_output=True)
        (out.with_suffix('.stdout.log')).write_text(proc.stdout,encoding='utf-8')
        (out.with_suffix('.stderr.log')).write_text(proc.stderr,encoding='utf-8')
        if proc.returncode:
            raise RuntimeError(f"Failed {case['id']} seed {seed}: {proc.stderr[-2500:]}")
        if not out.exists():
            raise RuntimeError(f"Runner did not produce {out}")
        validate_record(json.loads(out.read_text()),seed,case['id'])
        return f"OK {case['id']} seed {seed}"
    work=[(case,seed) for case in plan['cases'] for seed in plan['seeds']]
    with ThreadPoolExecutor(max_workers=args.jobs) as pool:
        for message in pool.map(execute,work):
            print(message,flush=True)
    do_report(argparse.Namespace(output=root))

def do_report(args):
    root=args.output
    path=root/'manifest.json'
    if not path.exists():
        raise SystemExit('No manifest at '+str(path))
    plan=json.loads(path.read_text());rows=[]
    for case in plan['cases']:
        records=[]
        for seed in plan['seeds']:
            path=root/'results'/case['id']/f'seed-{seed:02d}.json'
            if path.exists():
                records.append(validate_record(json.loads(path.read_text()),seed,case['id']))
        if not records:
            continue
        values=lambda arm:[record['arms'][arm]['balanced_accuracy'] for record in records]
        paired=values('paired');before=values('before');aged=values('aged_frozen');scrambled=values('scrambled')
        differences={name:bootstrap_paired(paired,values(name)) for name in ('before','aged_frozen','scrambled')}
        row=dict(case=case['id'],family=case['family'],n_seeds=len(records),paired_balanced_accuracy=statistics.mean(paired),before_balanced_accuracy=statistics.mean(before),aged_balanced_accuracy=statistics.mean(aged),scrambled_balanced_accuracy=statistics.mean(scrambled),
                 paired_visual_decodability=statistics.mean(r['arms']['paired']['visual_decodability'] for r in records),paired_action_selection=statistics.mean(r['arms']['paired']['action_selection_rate'] for r in records))
        for name,short in [('before','before'),('aged_frozen','aged'),('scrambled','scrambled')]:
            diff=differences[name];row[f'delta_vs_{short}']=diff['mean_difference'];row[f'{short}_ci_low'],row[f'{short}_ci_high']=diff['ci95']
        row['replicated']=len(records)==len(plan['seeds'])
        row['preliminary_all_three_controls']=row['replicated'] and all(d['ci95'][0]>0 for d in differences.values()) and row['paired_visual_decodability']>=.8
        criteria=row['replicated'] and len(records)>=10
        for mapping in ['normal_mapping','reversed_mapping']:
            for stage in ['probe','retained_probe','interference_probe']:
                for reference in ['before','aged_frozen','scrambled']:
                    if not all(all(mapping in r['arms'][arm] and stage in r['arms'][arm][mapping] for arm in DEFAULT_ARMS) for r in records):
                        criteria=False
                        continue
                    p=[r['arms']['paired'][mapping][stage]['count']['balanced_accuracy'] for r in records]
                    q=[r['arms'][reference][mapping][stage]['count']['balanced_accuracy'] for r in records]
                    criteria=criteria and bootstrap_paired(p,q)['ci95'][0]>0
            if all(mapping in r['arms']['paired'] for r in records):
                criteria=criteria and statistics.mean(r['arms']['paired'][mapping]['visual_decodability'] for r in records)>=.8
            else:
                criteria=False
        row['m4_criteria_met']=criteria
        rows.append(row)
    if not rows:
        print('No run data yet. No NERVA experiments were executed.')
        return
    with (root/'summary.csv').open('w',newline='',encoding='utf-8') as file:
        writer=csv.DictWriter(file,fieldnames=list(rows[0]));writer.writeheader();writer.writerows(rows)
    lines=['# NERVA M4 – result summary','',f"Stage: {plan['stage']}. Measurements come from the linked NERVA model.",'',
           '| Case | Seeds | Before | Aged | Paired | Scrambled | Visual check | Δ paired−aged (95% CI) | Δ paired−scrambled (95% CI) |',
           '|---|---:|---:|---:|---:|---:|---:|---:|---:|']
    for r in rows:
        pct=lambda x:f'{100*x:.1f}%'
        lines.append(f"| {r['case']} | {r['n_seeds']} | {pct(r['before_balanced_accuracy'])} | {pct(r['aged_balanced_accuracy'])} | {pct(r['paired_balanced_accuracy'])} | {pct(r['scrambled_balanced_accuracy'])} | {pct(r['paired_visual_decodability'])} | {pct(r['delta_vs_aged'])} [{pct(r['aged_ci_low'])}, {pct(r['aged_ci_high'])}] | {pct(r['delta_vs_scrambled'])} [{pct(r['scrambled_ci_low'])}, {pct(r['scrambled_ci_high'])}] |")
    lines+=['','Three seeds are diagnostic, not confirmation. Chance accuracy is 50%. Positive immediate scores alone do not establish learning: both mappings, retention, visual validation and input-current deviations must be examined. Incomplete cases are not failure evidence.','',
            'The 20-case matrix is available; the gate adds only the two explicitly requested diagnostic 20-ms conditions.']
    (root/'summary.md').write_text('\n'.join(lines)+'\n',encoding='utf-8')
    print('Report:',root/'summary.csv',root/'summary.md')

def main():
    parser=argparse.ArgumentParser(description=__doc__);sub=parser.add_subparsers(dest='cmd',required=True)
    plan=sub.add_parser('plan');plan.add_argument('--stage',choices=['gate','pilot','confirm'],default='pilot');plan.add_argument('--output',type=Path,default=MEASUREMENTS / 'm4-manifest.json')
    run=sub.add_parser('run');run.add_argument('--stage',choices=['gate','pilot','confirm'],default='gate');run.add_argument('--output',type=Path,default=MEASUREMENTS / 'm4-results');run.add_argument('--command');run.add_argument('--cases');run.add_argument('--train-trials',type=int,default=256);run.add_argument('--test-trials',type=int,default=128);run.add_argument('--calibration-trials',type=int,default=128);run.add_argument('--jobs',type=int,default=4);run.add_argument('--force',action='store_true')
    report=sub.add_parser('report');report.add_argument('--output',type=Path,default=MEASUREMENTS / 'm4-results')
    args=parser.parse_args()
    if args.cmd=='run' and args.command and ('{config}' not in args.command or '{output}' not in args.command):
        parser.error('--command requires {config} and {output}')
    {'plan':do_plan,'run':do_run,'report':do_report}[args.cmd](args)

if __name__=='__main__':main()
