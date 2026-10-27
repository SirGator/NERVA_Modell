#!/usr/bin/env python3
"""M10: retrospective M9 pathway audit and passive frozen replay traces."""
import argparse
import csv
import hashlib
import importlib.util
import json
import os
import statistics
import subprocess
from collections import defaultdict
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

HERE = Path(__file__).resolve().parent
EXPERIMENTS = HERE.parent


def module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    result = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(result)
    return result


m9 = module("m10_m9", EXPERIMENTS / "m9/run.py")
core = module("m10_core", HERE / "core.py")
PROTOCOL = "NERVA-M10-v1"


def digest(path):
    h = hashlib.sha256()
    with path.open("rb") as stream:
        while chunk := stream.read(1024 * 1024):
            h.update(chunk)
    return h.hexdigest()


def plan(source, output, stage):
    manifest = json.loads((source / "manifest.json").read_text())
    expected_stage = "smoke" if stage == "smoke" else "confirm"
    core.require(manifest == m9.plan(expected_stage, manifest["trial_counts"]), "M10 source is not the fixed M9-v2 plan")
    inputs = {}
    for condition in manifest["conditions"]:
        for seed in manifest["seeds"]:
            path = Path("results") / condition["case_id"] / f"seed-{seed}.json"
            inputs[str(path)] = digest(source / path)
    return dict(protocol=PROTOCOL, stage=stage, date="2026-10-27",
                source=os.path.relpath(source.resolve(), output.resolve()),
                source_manifest_sha256=digest(source / "manifest.json"), inputs=inputs,
                seeds=manifest["seeds"], trial_counts=manifest["trial_counts"],
                trace_case="weight4_tau100",
                design=dict(status="Retrospective exploratory diagnosis on existing M9 seeds; not fresh confirmation",
                            observation="All four M9 conditions, all four arms, both mappings and both checkpoints; no selection of seeds",
                            trace="Combination only on every source seed; sham/Core reset/all-plastic reset; same training and frozen checkpoints",
                            measurements=["mapping-dependent Core weights", "action count differences", "intervention count/event differences", "fixed decoder margins/decisions", "signed/absolute synaptic arrival impulses", "voltages projected on cloned neurons to common phase times"],
                            voltage_limit="M9 stored-state voltages have different last-update times; common-time voltage conclusions use new replay traces only",
                            inference="Descriptive seed-based summaries, no new learning/synergy significance claim; input/reference hashes fixed before replay",
                            limits="Impulses are not conductance currents; refractory/inhibitory effects can alter the voltage response; original neuron rules unchanged"))


def load(root):
    manifest = json.loads((root / "manifest.json").read_text())
    source = (root / manifest["source"]).resolve()
    core.require(manifest == plan(source, root, manifest["stage"]), "M10 manifest/source differs")
    return manifest, source


def validate(wrapper, reference, path, source_path):
    audit = core.audit_trace(wrapper, reference)
    m9.validate(wrapper["m9_result"], reference["requested_config"], path)
    for suffix in [".details.csv", ".audit.csv", ".sensor.csv", ".copies.csv", ".branches.csv", ".propagation.csv"]:
        core.require(digest(Path(str(path) + suffix)) == digest(Path(str(source_path) + suffix)), "passive trace changed historical CSV: " + suffix)
    audit["historical_csv_byte_equal"] = True
    return audit


def write_csv(path, rows):
    with path.open("w", newline="") as stream:
        writer = csv.DictWriter(stream, fieldnames=list(rows[0]))
        writer.writeheader()
        writer.writerows(rows)


def report(root):
    manifest, source = load(root)
    comparisons, actions, weights, traces, audits = [], [], [], [], []
    for relative in manifest["inputs"]:
        path = source / relative
        result = json.loads(path.read_text())
        m9.validate(result, result["requested_config"])
        rows, a, w = core.measure(result)
        comparisons.extend(rows); actions.extend(a); weights.extend(w)
        if result["case_id"] == manifest["trace_case"]:
            target = root / "traces" / f"seed-{result['seed']}.json"
            wrapper = json.loads(target.read_text())
            audits.append(dict(seed=result["seed"], **validate(wrapper, result, target, path)))
            traces.extend(core.measure(wrapper["m9_result"], common_time=True)[0])
    for name, rows in [("weight-effects.csv", comparisons), ("action-discrimination.csv", actions),
                       ("mapping-weights.csv", weights), ("trace-effects.csv", traces)]:
        write_csv(root / name, rows)
    grouped = defaultdict(list)
    for row in comparisons:
        grouped[row["case"], row["arm"], row["stage"], row["intervention"]].append(row)
    summaries = []
    for (case, arm, stage, intervention), rows in grouped.items():
        summaries.append(dict(case=case, arm=arm, stage=stage, intervention=intervention,
                              seeds=len({r["seed"] for r in rows}), windows=len(rows),
                              count_changed=sum(r["count_l1"] > 0 for r in rows),
                              events_changed=sum(r["events_changed"] for r in rows),
                              prediction_changed=sum(r["prediction_changed"] for r in rows),
                              mean_count_l1=statistics.mean(r["count_l1"] for r in rows)))
    write_csv(root / "summary.csv", summaries)
    m9.m6.json_out(root / "diagnostics.json", dict(protocol=PROTOCOL, retrospective=True,
        source_files=len(manifest["inputs"]), source_hashes_preserved=True,
        comparisons=len(comparisons), action_pairs=len(actions), mapping_weights=len(weights),
        replay_audits=audits, phases=sum(a["phases"] for a in audits), events=sum(a["events"] for a in audits)))
    lines = ["# M10 – Signalwegdiagnose", "", "Retrospektive Ursachenanalyse vorhandener M9-Daten; kein neuer unabhängiger Lernnachweis.",
             f"Seeds: {manifest['seeds']}. Alle vier Quellbedingungen; passive Replaytraces für die Kombination.", "",
             "| Bedingung, gekoppelter Arm | Aktionsproben | Spikezählmuster nach Core-Rücksetzung geändert | Vorhersagen geändert | A/D unterschiedlich aktiv, aber gleiche vorhergesagte Klasse |",
             "|---|---:|---:|---:|---:|"]
    for case, _, _ in m9.LEVELS:
        rows = [r for r in comparisons if r["case"] == case and r["arm"] == "paired" and r["intervention"] == "restore_core"]
        pairs = [r for r in actions if r["case"] == case and r["arm"] == "paired"]
        mismatch = sum(r["action_counts_l1"] > 0 and r["same_predicted_class"] for r in pairs)
        lines.append(f"| {case} | {len(rows)} | {sum(r['count_l1'] > 0 for r in rows)} | {sum(r['prediction_changed'] for r in rows)} | {mismatch}/{len(pairs)} |")
    lines += ["", "## Passive Traces mit gemeinsamen Messzeitpunkten", "",
              "| Arm, Core-Rücksetzung, Kombination | Proben | Geänderte Eingangsimpulssummen | Geänderte Potenziale am Prognosezeitpunkt | Geänderte Spikezählmuster | Geänderte Entscheidungen |",
              "|---|---:|---:|---:|---:|---:|"]
    for arm in core.ARMS:
        rows = [r for r in traces if r["arm"] == arm and r["intervention"] == "restore_core"]
        lines.append(f"| {arm} | {len(rows)} | {sum(r['signed_input_l1'] > 0 for r in rows)} | {sum(r['forecast_voltage_linf'] > 0 for r in rows)} | {sum(r['count_l1'] > 0 for r in rows)} | {sum(r['prediction_changed'] for r in rows)} |")
    lines += ["", "Die Beobachtung verändert weder ursprüngliche M9-Ergebnisse noch deren CSV-Dateien. Alle Replay- und Ereignisaudits bestanden.",
              "", "Potenziale werden auf Neuronenkopien auf gemeinsame Phasengrenzen fortgeschrieben. Die alten gespeicherten Zustandswerte sind keine synchronen Spannungsmessungen.",
              "", "Synaptische Eingänge sind signierte Ankunftsimpulse. Ihre Summe allein bestimmt wegen Leck, Hemmung und Refraktärzeiten keine Spikezahl.",
              "", "Die Vergleiche diagnostizieren den festen Spikezähl-Ausleser. Gleichklassifikation beweist keine identischen neuronalen Antworten und keinen allgemeinen Informationsverlust.",
              "", "[Gewichtseffekte](weight-effects.csv) · [Aktionsunterschiede](action-discrimination.csv) · [Gewichtsspeicherung](mapping-weights.csv) · [Zeitlich abgeglichene Traces](trace-effects.csv) · [Audit](diagnostics.json)."]
    (root / "summary.md").write_text("\n".join(lines) + "\n")
    print("Report:", root / "summary.md", flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=["plan", "run", "report"])
    parser.add_argument("--source", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--stage", choices=["diagnostic", "smoke"], default="diagnostic")
    parser.add_argument("--jobs", type=int, default=4)
    args = parser.parse_args()
    root = args.output
    if args.mode == "report":
        report(root)
        return
    core.require(args.source is not None, "--source is required")
    core.require(1 <= args.jobs <= 16, "jobs must be 1..16")
    expected = plan(args.source, root, args.stage)
    root.mkdir(parents=True, exist_ok=True)
    path = root / "manifest.json"
    if path.exists():
        core.require(json.loads(path.read_text()) == expected, "existing M10 manifest differs")
    else:
        m9.m6.json_out(path, expected)
    if args.mode == "plan":
        print("Plan:", path)
        return
    manifest, source = load(root)
    (root / "traces").mkdir(exist_ok=True)

    def execute(seed):
        reference_path = source / "results" / manifest["trace_case"] / f"seed-{seed}.json"
        reference = json.loads(reference_path.read_text())
        target = root / "traces" / f"seed-{seed}.json"
        if target.exists():
            validate(json.loads(target.read_text()), reference, target, reference_path)
            return f"VALID M10 seed {seed}"
        proc = subprocess.run([str(EXPERIMENTS / "run.sh"), "m10", "--reference", str(reference_path), "--output", str(target)], capture_output=True, text=True)
        target.with_suffix(".log").write_text(proc.stdout + proc.stderr)
        core.require(proc.returncode == 0, f"M10 seed {seed} failed: {proc.stderr[-2000:]}")
        validate(json.loads(target.read_text()), reference, target, reference_path)
        return f"OK M10 seed {seed}"

    with ThreadPoolExecutor(max_workers=args.jobs) as pool:
        for status in pool.map(execute, manifest["seeds"]):
            print(status, flush=True)
    report(root)


if __name__ == "__main__":
    main()
