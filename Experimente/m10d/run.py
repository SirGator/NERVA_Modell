#!/usr/bin/env python3
"""M10-D: frozen recurrence interventions on the existing M9 combination seeds."""
import argparse
import hashlib
import importlib.util
import json
import os
import statistics
import subprocess
from collections import defaultdict
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path

HERE = Path(__file__).resolve().parent
EXPERIMENTS = HERE.parent
PROJECT = EXPERIMENTS.parents[1]


def module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    result = importlib.util.module_from_spec(spec); spec.loader.exec_module(result)
    return result


core = module("m10d_core", HERE/"core.py")
m10 = module("m10d_m10", EXPERIMENTS/"m10/run.py")
m10b = module("m10d_m10b", EXPERIMENTS/"m10b/core.py")
m10c = module("m10d_m10c", EXPERIMENTS/"m10c/core.py")
m9 = m10.m9
digest = m10.digest
PROTOCOL = "NERVA-M10D-v1"
SUFFIXES = [".details.csv", ".audit.csv", ".sensor.csv", ".copies.csv", ".branches.csv", ".propagation.csv"]


def plan(source, output, stage):
    parent = json.loads((source/"manifest.json").read_text())
    core.require(parent == m9.plan("smoke" if stage == "smoke" else "confirm", parent["trial_counts"]), "source is not fixed M9-v2 plan")
    inputs = {}
    for seed in parent["seeds"]:
        relative = Path("results/weight4_tau100")/f"seed-{seed}.json"
        inputs[str(relative)] = digest(source/relative)
    return dict(protocol=PROTOCOL, stage=stage, date="2026-10-27",
        source=os.path.relpath(source.resolve(), output.resolve()),
        source_manifest_sha256=digest(source/"manifest.json"), inputs=inputs,
        seeds=parent["seeds"], trial_counts=parent["trial_counts"],
        definition=dict(status="Diagnostic intervention on known M9 checkpoints, not fresh confirmation",
            case="weight4_tau100", arms=m10.core.ARMS, mappings=m10.core.MAPS,
            stages=m10.core.STAGES, variants=core.VARIANTS,
            gate="Disable selected Core-to-Core edges and cancel their in-flight arrivals immediately after identical baseline, before action; dispose branch after forecast",
            preserved="Frozen weights, cellular state and clock at intervention; same visual count decoder, nine action-copy arrivals; all other pathways intact",
            metrics=["raw_accuracy", "pair_rank_accuracy", "signed_effect_contrast", "mean_spikes", "mean_active_cells", "max_spikes_one_cell", "changed_count_windows", "changed_predictions"],
            inference="All variants and controls; seed-paired per mapping/checkpoint descriptive bootstrap, 4000 resamples seed1234, no learning/significance gate",
            limits="Pair rank is not an independent single-action forecast; motor and non-Core feedback remain; no inference that a permanent block improves learning",
            voltage_tolerance=2e-5))


def load(root):
    manifest = json.loads((root/"manifest.json").read_text())
    source = (root/manifest["source"]).resolve()
    core.require(manifest == plan(source, root, manifest["stage"]), "M10-D manifest/source differs")
    return manifest, source


def validate(wrapper, reference, target, source_path):
    audit = core.validate(wrapper, reference, m10.core, m10c, m10b.analyze_pair)
    m9.validate(wrapper["m9_result"], reference["requested_config"], target)
    for suffix in SUFFIXES:
        core.require(digest(Path(str(target)+suffix)) == digest(Path(str(source_path)+suffix)), "historical CSV differs: "+suffix)
    return dict(**audit, historical_csv_byte_equal=True)


def contrasts(rows):
    indexed = {(r["seed"], r["arm"], r["mapping"], r["stage"], r["variant"]): r for r in rows}
    seeds = sorted({r["seed"] for r in rows})
    out = []
    for mapping in m10.core.MAPS:
        for stage in m10.core.STAGES:
            for variant in core.VARIANTS:
                for metric in ["raw_accuracy", "pair_rank_accuracy", "mean_spikes", "signed_effect_contrast"]:
                    left = [indexed[s, "paired", mapping, stage, variant][metric] for s in seeds]
                    controls = [(arm, variant) for arm in ["before", "aged_frozen", "scrambled"]]
                    if variant != "sham":controls.append(("paired", "sham"))
                    for arm, control_variant in controls:
                        right = [indexed[s, arm, mapping, stage, control_variant][metric] for s in seeds]
                        estimate = m9.m6.bootstrap_paired(left, right)
                        out.append(dict(mapping=mapping, stage=stage, variant=variant, metric=metric,
                            control_arm=arm, control_variant=control_variant, seeds=len(seeds),
                            paired=statistics.mean(left), control=statistics.mean(right),
                            delta=estimate["mean_difference"], ci_low=estimate["ci95"][0], ci_high=estimate["ci95"][1],
                            inference="descriptive known-seed contrast; no learning gate"))
    return out


def report(root):
    manifest, source = load(root)
    rows, audits = [], []
    for relative in manifest["inputs"]:
        reference = json.loads((source/relative).read_text())
        target = root/"traces"/f"seed-{reference['seed']}.json"
        wrapper = json.loads(target.read_text())
        audits.append(dict(seed=reference["seed"], **validate(wrapper, reference, target, source/relative)))
        rows.extend(core.measure(wrapper["m9_result"], m10b.analyze_pair))
    m10.write_csv(root/"pairs.csv", rows)
    m10.write_csv(root/"contrasts.csv", contrasts(rows))
    grouped = defaultdict(list)
    for row in rows:grouped[row["arm"], row["stage"], row["variant"]].append(row)
    summaries = []
    for (arm, stage, variant), values in grouped.items():
        summaries.append(dict(arm=arm, stage=stage, variant=variant, seeds=len({r["seed"] for r in values}), pairs=len(values),
            raw_accuracy=statistics.mean(r["raw_accuracy"] for r in values),
            pair_rank_accuracy=statistics.mean(r["pair_rank_accuracy"] for r in values),
            correct_direction=sum(r["correct_direction"] for r in values), wrong_direction=sum(r["wrong_direction"] for r in values),
            projection_ties=sum(r["projection_tie"] for r in values),
            mean_spikes=statistics.mean(r["mean_spikes"] for r in values),
            mean_active_cells=statistics.mean(r["mean_active_cells"] for r in values),
            max_spikes_one_cell=max(r["max_spikes_one_cell"] for r in values),
            changed_count_windows=sum(r["changed_count_windows"] for r in values),
            changed_predictions=sum(r["changed_predictions"] for r in values)))
    m10.write_csv(root/"summary.csv", summaries)
    m9.m6.json_out(root/"diagnostics.json", dict(protocol=PROTOCOL, stage=manifest["stage"], pairs=len(rows),
        replay_audits=audits, phases=sum(a["phases"] for a in audits), windows=sum(a["windows"] for a in audits),
        events=sum(a["events"] for a in audits), maximum_voltage_error=max(a["maximum_voltage_error"] for a in audits),
        cancelled_arrivals=sum(a["cancelled_arrivals"] for a in audits), original_parent_reproduced=True,
        original_sham_reproduced=True, source_hashes_preserved=True, new_decoder_fits=0, new_training_rules=0,
        new_seed_replays=len(audits), independent_confirmation=False))
    labels = dict(sham="Referenz", block_exc="Ohne rekurrente Erregung", block_inh="Ohne rekurrente Hemmung", block_both="Ohne beide")
    def get(arm, stage, variant):
        return next(r for r in summaries if (r["arm"], r["stage"], r["variant"]) == (arm, stage, variant))
    lines = ["# M10-D – Rekurrente Beiträge kontrolliert blockieren", "",
        "Technischer Pilot." if manifest["stage"] == "smoke" else "Diagnose auf denselben 20 bekannten M9-Seeds, keine unabhängige Bestätigung.", "",
        "Feste Trainingscheckpoint-Ausleser; Blockade erst nach identischer Baseline. Keine neue Lernregel und kein Refit.", "",
        "## Gekoppelt trainiert", "",
        "| Variante | Sofort: Einzelantwort | Sofort: Paar-Rangfolge | Spikes/Aktion | Nach Ruhe: Einzelantwort | Nach Ruhe: Paar-Rangfolge | Spikes/Aktion |",
        "|---|---:|---:|---:|---:|---:|---:|"]
    for variant in core.VARIANTS:
        a, b = [get("paired", stage, variant) for stage in m10.core.STAGES]
        lines.append(f"| {labels[variant]} | {a['raw_accuracy']:.2%} | {a['pair_rank_accuracy']:.2%} | {a['mean_spikes']:.3f} | {b['raw_accuracy']:.2%} | {b['pair_rank_accuracy']:.2%} | {b['mean_spikes']:.3f} |")
    lines += ["", "## Kontrollarme: Paar-Rangfolge sofort / nach Ruhe", "",
        "| Variante | Vor Training | Gealtert eingefroren | Gekoppelt | Entkoppelt |", "|---|---:|---:|---:|---:|"]
    for variant in core.VARIANTS:
        parts = [f"{get(arm, m10.core.STAGES[0], variant)['pair_rank_accuracy']:.2%} / {get(arm, m10.core.STAGES[1], variant)['pair_rank_accuracy']:.2%}" for arm in m10.core.ARMS]
        lines.append(f"| {labels[variant]} | "+" | ".join(parts)+" |")
    lines += ["", "Paar-Rangfolge nutzt beide A-/D-Antworten und deren bekannte Balance. Sie ist keine neue einzelne Vorhersage oder autonom gelernte Handlung.",
        "", "Die vollständigen Tabellen enthalten aktive Zellen, höchste Spikezahl je Zelle, ursprüngliche Vorhersagen und die Unterschiede zu Sham. Mehr Aktivität allein zählt nicht als Erfolg.",
        "", "Alle Originalergebnisse und sechs historischen CSVs sind reproduziert. Gesperrte Kanten liefern im Aktionsfenster keine Ankünfte; Aktionskopien bleiben erhalten.",
        "", "Motor-Rückkopplung und andere nicht-Core→Core-Signalwege bleiben auch bei Blockade beider rekurrenter Anteile aktiv.",
        "", "[Einzelpaare](pairs.csv) · [Deskriptive seed-gepaarte Kontraste](contrasts.csv) · [Audit](diagnostics.json)."]
    (root/"summary.md").write_text("\n".join(lines)+"\n")
    print("Report:", root/"summary.md", flush=True)


def provenance(root):
    paths = [EXPERIMENTS/"target/release/nerva-m10d", root/"manifest.json", EXPERIMENTS/"M10D_EXPERIMENT.md"]
    for folder in [PROJECT/"NERVA/src", PROJECT/"NERVA_Modell/Modell/src", EXPERIMENTS/"src"]:
        paths.extend(folder.rglob("*.rs"))
    for folder in ["m9", "m10", "m10b", "m10c", "m10d"]:
        paths.extend((EXPERIMENTS/folder).rglob("*.py"))
    result = dict(protocol=PROTOCOL, date="2026-10-27", files={os.path.relpath(p.resolve(), PROJECT): digest(p) for p in sorted(paths)})
    path = root/"provenance.json"
    if path.exists():core.require(json.loads(path.read_text()) == result, "program provenance changed")
    else:m9.m6.json_out(path, result)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=["plan", "run", "report"])
    parser.add_argument("--source", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--stage", choices=["diagnostic", "smoke"], default="diagnostic")
    parser.add_argument("--jobs", type=int, default=4)
    args = parser.parse_args()
    root = args.output
    if args.mode == "report":report(root); return
    core.require(args.source is not None, "--source required")
    core.require(1 <= args.jobs <= 16, "jobs must be 1..16")
    expected = plan(args.source, root, args.stage)
    root.mkdir(parents=True, exist_ok=True)
    path = root/"manifest.json"
    if path.exists():core.require(json.loads(path.read_text()) == expected, "existing manifest differs")
    else:m9.m6.json_out(path, expected)
    if args.mode == "plan":print("Plan:", path); return
    manifest, source = load(root)
    provenance(root)
    m9.m6.json_out(root/"execution.json", dict(jobs=args.jobs, seeds=manifest["seeds"], mode=manifest["stage"]))
    (root/"traces").mkdir(exist_ok=True)
    def execute(seed):
        reference_path = source/"results/weight4_tau100"/f"seed-{seed}.json"
        reference = json.loads(reference_path.read_text())
        target = root/"traces"/f"seed-{seed}.json"
        if not target.exists():
            proc = subprocess.run([str(EXPERIMENTS/"target/release/nerva-m10d"), "--reference", str(reference_path), "--output", str(target)], capture_output=True, text=True)
            target.with_suffix(".log").write_text(proc.stdout+proc.stderr)
            core.require(proc.returncode == 0, f"M10-D seed {seed} failed: {proc.stderr[-2000:]}")
        validate(json.loads(target.read_text()), reference, target, reference_path)
        return f"OK M10-D seed {seed}"
    with ThreadPoolExecutor(max_workers=args.jobs) as pool:
        tasks = [pool.submit(execute, seed) for seed in manifest["seeds"]]
        for task in as_completed(tasks):print(task.result(), flush=True)
    report(root)
    provenance(root)


if __name__ == "__main__":main()
