#!/usr/bin/env python3
"""M10-E: fixed Core-weight restoration crossed with four recurrent gates."""
import argparse
import csv
import json
import os
import statistics
import subprocess
from collections import defaultdict
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path
from types import SimpleNamespace
import importlib.util

HERE = Path(__file__).resolve().parent
EXPERIMENTS = HERE.parent
PROJECT = EXPERIMENTS.parents[1]


def module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    result = importlib.util.module_from_spec(spec); spec.loader.exec_module(result)
    return result


core = module("m10e_core", HERE/"core.py")
m10d = module("m10e_previous", EXPERIMENTS/"m10d/run.py")
m10, m10b, m10c, m9 = m10d.m10, m10d.m10b, m10d.m10c, m10d.m9
D = SimpleNamespace(m10=m10, m10b=m10b, m10c=m10c, m10d=m10d)
digest = m10.digest
PROTOCOL = "NERVA-M10E-v1"
METRICS = ["raw_accuracy", "pair_rank_accuracy", "signed_effect_contrast", "mean_spikes", "correct_margin"]


def plan(source, recurrence_source, output, stage):
    base = m10d.plan(source, output, stage)
    previous, _ = m10d.load(recurrence_source)
    core.require(previous["stage"] == stage and previous["seeds"] == base["seeds"]
                 and previous["inputs"] == base["inputs"], "M10-D/M9 source plans differ")
    old_hashes = {f"traces/seed-{s}.json": digest(recurrence_source/"traces"/f"seed-{s}.json") for s in base["seeds"]}
    return dict(protocol=PROTOCOL, stage=stage, date="2026-10-27",
        source=base["source"], source_manifest_sha256=base["source_manifest_sha256"], inputs=base["inputs"],
        recurrence_source=os.path.relpath(recurrence_source.resolve(), output.resolve()),
        recurrence_manifest_sha256=digest(recurrence_source/"manifest.json"), recurrence_traces=old_hashes,
        seeds=base["seeds"], trial_counts=base["trial_counts"],
        definition=dict(status="Exploratory crossed intervention on known M9 seeds; not fresh confirmation",
            weights=core.WEIGHTS, gates=core.GATES, arms=m10.core.ARMS, mappings=m10.core.MAPS, stages=m10.core.STAGES,
            intervention="Core weights restored after identical neutral baseline, then gates applied immediately before action",
            state="Same cellular/refractory state and fixed visual count decoder; unblocked pending inputs retain emission amplitude; other pathways remain",
            difference_from_m10="Earlier reset was before baseline; these reset branches are not direct replicas of earlier reset branches",
            metrics=METRICS+["count_l1", "events_changed", "prediction_changed", "voltage_linf"],
            weight_effect="trained minus reset_core, same gate/seed/arm/mapping/stage",
            interaction="weight effect under a gate minus weight effect under sham; paired/control weight-effect contrasts",
            inference="Descriptive seed-paired bootstrap, 4000 resamples seed1234, per mapping/stage; no learning/significance gate",
            negative_control="If every changed edge is blocked, action traces must be identical between weight states",
            validation="Original M9 result and CSVs unchanged; all four trained branches reproduce archived M10-D; neutral LIF tolerance2e-5"))


def load(root):
    manifest = json.loads((root/"manifest.json").read_text())
    source = (root/manifest["source"]).resolve()
    previous = (root/manifest["recurrence_source"]).resolve()
    core.require(manifest == plan(source, previous, root, manifest["stage"]), "M10-E manifest/source differs")
    return manifest, source, previous


def validate(wrapper, reference, previous, target, source_path):
    audit = core.validate(wrapper, reference, previous, D)
    m9.validate(wrapper["m9_result"], reference["requested_config"], target)
    for suffix in m10d.SUFFIXES:
        core.require(digest(Path(str(target)+suffix)) == digest(Path(str(source_path)+suffix)), "historical CSV differs: "+suffix)
    return dict(**audit, historical_csv_byte_equal=True)


def contrasts(rows):
    index = {(r["seed"], r["arm"], r["mapping"], r["stage"], r["gate"], r["weight"]): r for r in rows}
    seeds = sorted({r["seed"] for r in rows})
    def effect(arm, mapping, stage, gate, metric):
        return [index[s, arm, mapping, stage, gate, "trained"][metric]-index[s, arm, mapping, stage, gate, "reset_core"][metric] for s in seeds]
    out = []
    for mapping in m10.core.MAPS:
        for stage in m10.core.STAGES:
            for gate in core.GATES:
                for metric in METRICS:
                    paired = effect("paired", mapping, stage, gate, metric)
                    comparisons = [("weight_effect", arm, effect(arm, mapping, stage, gate, metric), [0]*len(seeds)) for arm in m10.core.ARMS]
                    comparisons += [("paired_minus_control_weight_effect", arm, paired, effect(arm, mapping, stage, gate, metric))
                                    for arm in ["before", "aged_frozen", "scrambled"]]
                    if gate != "sham":
                        comparisons += [("gate_minus_sham_weight_effect", arm, effect(arm, mapping, stage, gate, metric), effect(arm, mapping, stage, "sham", metric)) for arm in m10.core.ARMS]
                    for kind, arm, left, right in comparisons:
                        estimate = m9.m6.bootstrap_paired(left, right)
                        out.append(dict(mapping=mapping, stage=stage, gate=gate, metric=metric, kind=kind, arm=arm,
                            seeds=len(seeds), delta=estimate["mean_difference"], ci_low=estimate["ci95"][0], ci_high=estimate["ci95"][1],
                            inference="descriptive known-seed contrast; no learning gate"))
    return out


def report(root):
    manifest, source, previous_source = load(root)
    pairs, windows, audits = [], [], []
    for relative in manifest["inputs"]:
        reference = json.loads((source/relative).read_text())
        old = json.loads((previous_source/"traces"/f"seed-{reference['seed']}.json").read_text())
        target = root/"traces"/f"seed-{reference['seed']}.json"
        wrapper = json.loads(target.read_text())
        audits.append(dict(seed=reference["seed"], **validate(wrapper, reference, old, target, source/relative)))
        a, b = core.measure(wrapper["m9_result"], D)
        pairs.extend(a); windows.extend(b)
    m10.write_csv(root/"pairs.csv", pairs)
    m10.write_csv(root/"weight-effects.csv", windows)
    m10.write_csv(root/"contrasts.csv", contrasts(pairs))
    grouped = defaultdict(list)
    for row in pairs:grouped[row["arm"], row["stage"], row["gate"], row["weight"]].append(row)
    summaries = []
    for (arm, stage, gate, weight), values in grouped.items():
        summaries.append(dict(arm=arm, stage=stage, gate=gate, weight=weight,
            seeds=len({r["seed"] for r in values}), pairs=len(values),
            **{metric: statistics.mean(r[metric] for r in values) for metric in METRICS},
            mean_active_cells=statistics.mean(r["mean_active_cells"] for r in values)))
    m10.write_csv(root/"summary.csv", summaries)
    groups = defaultdict(list)
    for r in windows:groups[r["arm"], r["stage"], r["gate"]].append(r)
    changes = [dict(arm=arm, stage=stage, gate=gate, windows=len(values),
        count_changed=sum(r["count_l1"] > 0 for r in values), events_changed=sum(r["events_changed"] for r in values),
        predictions_changed=sum(r["prediction_changed"] for r in values),
        voltage_changed=sum(r["voltage_linf"] > 0 for r in values), mean_count_l1=statistics.mean(r["count_l1"] for r in values))
        for (arm, stage, gate), values in groups.items()]
    m10.write_csv(root/"weight-effect-summary.csv", changes)
    m9.m6.json_out(root/"diagnostics.json", dict(protocol=PROTOCOL, stage=manifest["stage"], pairs=len(pairs),
        windows=sum(a["windows"] for a in audits), phases=sum(a["phases"] for a in audits), events=sum(a["events"] for a in audits),
        negative_control_windows=sum(a["negative_control_windows"] for a in audits),
        maximum_voltage_error=max(a["maximum_voltage_error"] for a in audits),
        original_parent_reproduced=True, all_trained_branches_reproduce_m10d=True, source_hashes_preserved=True,
        new_decoder_fits=0, new_training_rules=0, known_seed_replays=len(audits), independent_confirmation=False,
        replay_audits=audits))
    def get(stage, gate, weight):return next(r for r in summaries if (r["arm"], r["stage"], r["gate"], r["weight"]) == ("paired", stage, gate, weight))
    lines = ["# M10-E – Core-Gewichte und Rekurrenz gekreuzt", "",
        "Technischer Pilot." if manifest["stage"] == "smoke" else "Diagnose auf denselben 20 bekannten Seeds, keine unabhängige Bestätigung.", "",
        "Gewichtsreset und Sperre erst nach identischer Baseline; feste Ausleser. Nicht gesperrte unterwegs befindliche Eingänge behalten ihre Emissionsamplitude.", "",
        "## Gekoppelter Arm, trainiert / zurückgesetzt", "",
        "| Sperre | Sofort: Einzelvorhersage | Nach Ruhe: Einzelvorhersage | Sofort: Paar-Rangfolge | Nach Ruhe: Paar-Rangfolge |", "|---|---:|---:|---:|---:|"]
    for gate in core.GATES:
        values=[]
        for metric in ["raw_accuracy", "pair_rank_accuracy"]:
            for stage in m10.core.STAGES:
                a, b = [get(stage, gate, weight)[metric] for weight in core.WEIGHTS]
                values.append(f"{a:.2%} / {b:.2%}")
        lines.append(f"| {gate} | "+" | ".join(values)+" |")
    lines += ["", "## Änderungen durch Core-Rücksetzung im gekoppelten Arm", "",
        "| Zeitpunkt / Sperre | Fenster | Anderes Spikezählmuster | Andere Ereignisse | Andere Vorhersage | Anderes Restpotenzial |", "|---|---:|---:|---:|---:|---:|"]
    for r in changes:
        if r["arm"] == "paired":
            lines.append(f"| {r['stage']} / {r['gate']} | {r['windows']} | {r['count_changed']} | {r['events_changed']} | {r['predictions_changed']} | {r['voltage_changed']} |")
    lines += ["", "Die Paar-Rangfolge nutzt beide Antworten und ihre bekannte Balance, keine neue einzelne Wirkungsvorhersage. Mehr Aktivität und Gewichtseinfluss allein sind kein Lernerfolg.",
        "", "Alle vier trainierten Zweige reproduzieren M10-D. Rücksetzungszweige unterscheiden sich methodisch von den früheren Rücksetzungen vor der Baseline.",
        "", "[Alle Paare und Kontrollarme](pairs.csv) · [Gewichtseffekte](weight-effects.csv) · [Seed-gepaarte Kontraste und Wechselwirkungen](contrasts.csv) · [Audit](diagnostics.json)."]
    (root/"summary.md").write_text("\n".join(lines)+"\n")
    print("Report:", root/"summary.md", flush=True)


def provenance(root):
    paths = [EXPERIMENTS/"target/release/nerva-m10e", root/"manifest.json", EXPERIMENTS/"M10E_EXPERIMENT.md"]
    for folder in [PROJECT/"NERVA/src", PROJECT/"NERVA_Modell/Modell/src", EXPERIMENTS/"src"]:paths.extend(folder.rglob("*.rs"))
    for folder in ["m9", "m10", "m10b", "m10c", "m10d", "m10e"]:paths.extend((EXPERIMENTS/folder).rglob("*.py"))
    result = dict(protocol=PROTOCOL, date="2026-10-27", files={os.path.relpath(p.resolve(), PROJECT): digest(p) for p in sorted(paths)})
    path = root/"provenance.json"
    if path.exists():core.require(json.loads(path.read_text()) == result, "program provenance changed")
    else:m9.m6.json_out(path, result)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=["plan", "run", "report"])
    parser.add_argument("--source", type=Path)
    parser.add_argument("--recurrence-source", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--stage", choices=["diagnostic", "smoke"], default="diagnostic")
    parser.add_argument("--jobs", type=int, default=4)
    args = parser.parse_args(); root = args.output
    if args.mode == "report":report(root); return
    core.require(args.source is not None and args.recurrence_source is not None, "both sources required")
    core.require(1 <= args.jobs <= 16, "jobs must be 1..16")
    expected = plan(args.source, args.recurrence_source, root, args.stage)
    root.mkdir(parents=True, exist_ok=True); path = root/"manifest.json"
    if path.exists():core.require(json.loads(path.read_text()) == expected, "existing manifest differs")
    else:m9.m6.json_out(path, expected)
    if args.mode == "plan":print("Plan:", path); return
    manifest, source, previous_source = load(root)
    provenance(root)
    m9.m6.json_out(root/"execution.json", dict(jobs=args.jobs, seeds=manifest["seeds"], stage=manifest["stage"]))
    (root/"traces").mkdir(exist_ok=True)
    def execute(seed):
        reference_path = source/"results/weight4_tau100"/f"seed-{seed}.json"
        reference = json.loads(reference_path.read_text())
        previous = json.loads((previous_source/"traces"/f"seed-{seed}.json").read_text())
        target = root/"traces"/f"seed-{seed}.json"
        if not target.exists():
            proc = subprocess.run([str(EXPERIMENTS/"target/release/nerva-m10e"), "--reference", str(reference_path), "--output", str(target)], capture_output=True, text=True)
            target.with_suffix(".log").write_text(proc.stdout+proc.stderr)
            core.require(proc.returncode == 0, f"M10-E seed {seed} failed: {proc.stderr[-2000:]}")
        validate(json.loads(target.read_text()), reference, previous, target, reference_path)
        return f"OK M10-E seed {seed}"
    with ThreadPoolExecutor(max_workers=args.jobs) as pool:
        tasks = [pool.submit(execute, seed) for seed in manifest["seeds"]]
        for task in as_completed(tasks):print(task.result(), flush=True)
    report(root); provenance(root)


if __name__ == "__main__":main()
