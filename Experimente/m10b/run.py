#!/usr/bin/env python3
"""M10-B: effect alignment versus common decoder bias in existing M9 responses."""
import argparse
import csv
import hashlib
import importlib.util
import json
import os
import statistics
from collections import defaultdict
from pathlib import Path

HERE = Path(__file__).resolve().parent


def module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    result = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(result)
    return result


core = module("m10b_core", HERE / "core.py")
m9 = module("m10b_m9", HERE.parent / "m9/run.py")
PROTOCOL = "NERVA-M10B-v1"
STAGES = ["checkpoint_probe", "rest_checkpoint_probe"]
MAPS = ["normal_mapping", "reversed_mapping"]


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def plan(source, output, stage):
    source_manifest = json.loads((source / "manifest.json").read_text())
    expected = "smoke" if stage == "smoke" else "confirm"
    core.require(source_manifest == m9.plan(expected, source_manifest["trial_counts"]), "source is not the fixed M9-v2 plan")
    inputs = {}
    for case in source_manifest["conditions"]:
        for seed in source_manifest["seeds"]:
            relative = Path("results") / case["case_id"] / f"seed-{seed}.json"
            inputs[str(relative)] = digest(source / relative)
    return dict(protocol=PROTOCOL, stage=stage, date="2026-10-27",
                source=os.path.relpath(source.resolve(), output.resolve()),
                source_manifest_sha256=digest(source / "manifest.json"), inputs=inputs,
                seeds=source_manifest["seeds"],
                definition=dict(
                    status="Retrospective exploratory extension of M10; existing M9 seeds, no fresh confirmation",
                    scope="All four conditions, all four arms, both mappings/endpoints and all three existing frozen weight interventions",
                    decoder="Existing visual mean/scale/centroids only; no refit, no action-label training, no neural simulation",
                    axis="Unit vector from negative to positive standardized visual centroid; scores relative to visual midpoint",
                    bias="Mean A/D projection; effect contrast is projection(D)-projection(A), signed by D's actual effect label",
                    centered_rank="Known balanced A/D pair is assigned one negative and one positive effect by score order; ties score0.5",
                    epsilon=core.EPSILON,
                    tie_rule="abs(effect contrast)<=epsilon*max(1,abs(scoreA),abs(scoreD),norm(zD-zA))",
                    metrics=["original single-action accuracy", "balanced-pair rank accuracy", "signed visual-axis cosine", "common bias", "aligned but same original class"],
                    controls="Compare trained paired arm with chance/before/aged_frozen/scrambled and both weight resets, separately per mapping and endpoint",
                    inference="Seed-paired descriptive bootstrap intervals only; no new significance or learning confirmation gate",
                    limits="Pair ranking has access to both actions from identical states and assumes one example of each effect; it is not an independent single-action forecast or a learned behavior"))


def load(root):
    manifest = json.loads((root / "manifest.json").read_text())
    source = (root / manifest["source"]).resolve()
    core.require(manifest == plan(source, root, manifest["stage"]), "M10-B manifest/source differs")
    return manifest, source


def write_csv(path, rows):
    with path.open("w", newline="") as stream:
        writer = csv.DictWriter(stream, fieldnames=list(rows[0]))
        writer.writeheader(); writer.writerows(rows)


def contrasts(rows):
    indexed = {(r["case"], r["seed"], r["arm"], r["mapping"], r["stage"], r["intervention"]): r for r in rows}
    out = []
    for case, _, _ in m9.LEVELS:
        seeds = sorted({r["seed"] for r in rows if r["case"] == case})
        for mapping in MAPS:
            for stage in STAGES:
                for metric in ["raw_accuracy", "pair_rank_accuracy"]:
                    paired = [indexed[case, seed, "paired", mapping, stage, "sham"][metric] for seed in seeds]
                    for reference in ["chance", "before", "aged_frozen", "scrambled", "restore_core", "restore_all_plastic"]:
                        if reference == "chance":
                            control = [.5]*len(seeds)
                        else:
                            arm = "paired" if reference.startswith("restore_") else reference
                            intervention = reference if reference.startswith("restore_") else "sham"
                            control = [indexed[case, seed, arm, mapping, stage, intervention][metric] for seed in seeds]
                        estimate = m9.m6.bootstrap_paired(paired, control)
                        out.append(dict(case=case, mapping=mapping, stage=stage, metric=metric,
                                        reference=reference, seeds=len(seeds), paired=statistics.mean(paired),
                                        control=statistics.mean(control), delta=estimate["mean_difference"],
                                        ci_low=estimate["ci95"][0], ci_high=estimate["ci95"][1],
                                        inference="descriptive; no learning or significance gate"))
    return out


def report(root):
    manifest, source = load(root)
    rows = []
    for relative in manifest["inputs"]:
        result = json.loads((source / relative).read_text())
        m9.validate(result, result["requested_config"])
        rows.extend(core.measure(result))
    write_csv(root / "pairs.csv", rows)
    grouped = defaultdict(list)
    for row in rows:
        grouped[row["case"], row["arm"], row["stage"], row["intervention"]].append(row)
    summaries = []
    for (case, arm, stage, intervention), values in grouped.items():
        cosines = [r["effect_axis_cosine"] for r in values if r["effect_axis_cosine"] is not None]
        summaries.append(dict(case=case, arm=arm, stage=stage, intervention=intervention,
            seeds=len({r["seed"] for r in values}), pairs=len(values),
            raw_accuracy=statistics.mean(r["raw_accuracy"] for r in values),
            pair_rank_accuracy=statistics.mean(r["pair_rank_accuracy"] for r in values),
            mean_effect_axis_cosine=statistics.mean(cosines) if cosines else "",
            same_class=sum(r["same_class"] for r in values),
            aligned_but_same_class=sum(r["aligned_but_same_class"] for r in values),
            correct_direction=sum(r["correct_direction"] for r in values),
            wrong_direction=sum(r["wrong_direction"] for r in values),
            projection_ties=sum(r["projection_tie"] for r in values)))
    write_csv(root / "summary.csv", summaries)
    comparisons = contrasts(rows)
    write_csv(root / "contrasts.csv", comparisons)
    diagnostics = dict(protocol=PROTOCOL, retrospective=True, source_files=len(manifest["inputs"]),
                       pairs=len(rows), source_hashes_preserved=True,
                       original_predictions_reconstructed=True, original_accuracy_reproduced=True,
                       neural_runs=0, action_label_fits=0, epsilon=core.EPSILON,
                       degenerate_axes=sum(r["axis_degenerate"] for r in rows))
    m9.m6.json_out(root / "diagnostics.json", diagnostics)
    def selected(case, stage, arm="paired", intervention="sham"):
        return next(r for r in summaries if r["case"] == case and r["stage"] == stage and r["arm"] == arm and r["intervention"] == intervention)
    lines = ["# M10-B – Wirkungsausrichtung und gemeinsamer Decoderbias", "",
             "Retrospektive Diagnose vorhandener M9-Antworten. Kein neuer neuronaler Lauf, kein neu trainierter Ausleser, kein unabhängiger Lernnachweis.", "",
             "Die Paar-Rangfolge nutzt beide A-/D-Antworten und die bekannte Balance. Sie ist keine einzelne Handlungswirkungsvorhersage.", "",
             "| Bedingung, gekoppelter Arm | Sofort: Einzelantwort | Sofort: Paar-Rangfolge | Nach Ruhe: Einzelantwort | Nach Ruhe: Paar-Rangfolge |",
             "|---|---:|---:|---:|---:|"]
    for case, _, _ in m9.LEVELS:
        a, b = [selected(case, stage) for stage in STAGES]
        lines.append(f"| {case} | {a['raw_accuracy']:.2%} | {a['pair_rank_accuracy']:.2%} | {b['raw_accuracy']:.2%} | {b['pair_rank_accuracy']:.2%} |")
    lines += ["", "## Kombination: Ausrichtung und ursprüngliche Gleichklassifikation", "",
              "| Zeitpunkt | Paare | Richtige Richtung, aber gleiche Klasse | Richtige Richtung insgesamt | Falsche Richtung | Projektionsgleichstand |",
              "|---|---:|---:|---:|---:|---:|"]
    for stage in STAGES:
        row = selected("weight4_tau100", stage)
        lines.append(f"| {stage} | {row['pairs']} | {row['aligned_but_same_class']} | {row['correct_direction']} | {row['wrong_direction']} | {row['projection_ties']} |")
    lines += ["", "## Kontrollen der Paar-Rangfolge, Kombination", "",
              "| Arm/Eingriff | Sofort | Nach Ruhe |", "|---|---:|---:|"]
    for arm, intervention in [("before", "sham"), ("aged_frozen", "sham"), ("paired", "sham"), ("scrambled", "sham"), ("paired", "restore_core"), ("paired", "restore_all_plastic")]:
        a, b = [selected("weight4_tau100", stage, arm, intervention) for stage in STAGES]
        lines.append(f"| {arm}/{intervention} | {a['pair_rank_accuracy']:.2%} | {b['pair_rank_accuracy']:.2%} |")
    lines += ["", "Die Zentrierung entfernt einen gemeinsamen A-/D-Projektionsoffset. Richtige Richtung trotz gleicher Klasse diagnostiziert einen Schwellen-/Biasbeitrag; falsche Richtung oder Gleichstand wird dadurch nicht behoben.",
              "", "[Alle Paare und Projektionen](pairs.csv) · [Seed-gepaarte deskriptive Kontraste](contrasts.csv) · [Audit](diagnostics.json).",
              "", "Intervalle sind deskriptiv. Dieselben bereits untersuchten Seeds werden wiederverwendet; daraus wird kein neuer Lernnachweis abgeleitet."]
    (root / "summary.md").write_text("\n".join(lines)+"\n")
    print("Report:", root / "summary.md")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=["plan", "run", "report"])
    parser.add_argument("--source", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--stage", choices=["diagnostic", "smoke"], default="diagnostic")
    args = parser.parse_args()
    if args.mode == "report":
        report(args.output); return
    core.require(args.source is not None, "--source required")
    expected = plan(args.source, args.output, args.stage)
    args.output.mkdir(parents=True, exist_ok=True)
    path = args.output / "manifest.json"
    if path.exists():
        core.require(json.loads(path.read_text()) == expected, "existing M10-B manifest differs")
    else:
        m9.m6.json_out(path, expected)
    if args.mode == "plan":
        print("Plan:", path)
    else:
        report(args.output)


if __name__ == "__main__":
    main()
