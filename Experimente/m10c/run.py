#!/usr/bin/env python3
"""M10-C: exact impulse/leak/refractory/reset accounting relative to the fixed count readout."""
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


core = module("m10c_core", HERE/"core.py")
m10 = module("m10c_m10", HERE.parent/"m10/run.py")
m10b = module("m10c_m10b", HERE.parent/"m10b/run.py")
PROTOCOL = "NERVA-M10C-v1"


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def plan(source, output, stage):
    parent, reference = m10.load(source)
    core.require(parent["stage"] == "diagnostic", "M10-C requires the complete M10 diagnostic source")
    seeds = parent["seeds"][:1] if stage == "smoke" else parent["seeds"]
    hashes = {}
    for seed in seeds:
        relative = Path("traces")/f"seed-{seed}.json"
        hashes[str(relative)] = digest(source/relative)
    return dict(protocol=PROTOCOL, stage=stage, date="2026-10-27",
                source=os.path.relpath(source.resolve(), output.resolve()),
                source_manifest_sha256=digest(source/"manifest.json"), traces=hashes, seeds=seeds,
                definition=dict(
                    status="Retrospective exploratory M10 trajectory accounting; no fresh confirmation",
                    scope="Combination, all four arms/mappings/checkpoints and three frozen weight interventions; every M10 seed",
                    axis="Same fixed neuron relevance coefficients as M10-B count decoder: unit standardized visual centroid difference divided by count feature scale",
                    metrics=core.SIGNALS,
                    groups=core.GROUPS,
                    decomposition="V(end)=decayed initial state + all leak-weighted inputs - discarded refractory inputs - leak-weighted pre-reset voltage losses",
                    timing="Only the 50ms action window, common A/D state verified; spikes are observed/forced, not newly generated",
                    epsilon=core.EPSILON, voltage_tolerance=core.VOLTAGE_TOLERANCE,
                    directions="Sign of effect-corrected A/D contrast on the fixed count-readout neuron weights; not modality-specific class predictions",
                    inference="Descriptive seed-grouped counts; no new significance/learning gate",
                    limits="Current/voltage projections use count-readout neuron relevance, not separately fitted visual input/voltage templates; endpoint voltage is post-reset residual, not a stage before spiking; algebraic components condition on observed recurrence"))


def load(root):
    manifest = json.loads((root/"manifest.json").read_text())
    source = (root/manifest["source"]).resolve()
    core.require(manifest == plan(source, root, manifest["stage"]), "M10-C manifest/source differs")
    return manifest, source


def write_csv(path, rows):
    with path.open("w", newline="") as stream:
        writer = csv.DictWriter(stream, fieldnames=list(rows[0]))
        writer.writeheader(); writer.writerows(rows)


def report(root):
    manifest, source = load(root)
    parent, reference_root = m10.load(source)
    signals, groups, edges, windows, audits = [], [], [], [], []
    for relative in manifest["traces"]:
        wrapper = json.loads((source/relative).read_text())
        reference_path = reference_root/"results"/wrapper["case_id"]/f"seed-{wrapper['seed']}.json"
        reference = json.loads(reference_path.read_text())
        audits.append(dict(seed=wrapper["seed"], **m10.core.audit_trace(wrapper, reference)))
        result = wrapper["m9_result"]
        measured = core.measure(result, m10b.core.analyze_pair)
        for destination, values in zip([signals, groups, edges, windows], measured):
            destination.extend(values)
    # Independent additivity check after projection and source/edge grouping.
    ident_fields = ["seed", "arm", "mapping", "stage", "intervention"]
    grouped_signals, grouped_sources, grouped_edges = defaultdict(dict), defaultdict(list), defaultdict(list)
    for row in signals:
        grouped_signals[tuple(row[k] for k in ident_fields)][row["signal"]] = row
    for row in groups:grouped_sources[tuple(row[k] for k in ident_fields)].append(row)
    for row in edges:grouped_edges[tuple(row[k] for k in ident_fields)].append(row)
    for ident, values in grouped_signals.items():
        for signal in ["all_input", "all_leak_input", "accepted_leak_input"]:
            expected = values[signal]["contrast"]
            for part in [grouped_sources[ident], grouped_edges[ident]]:
                core.require(abs(sum(r[signal] for r in part)-expected) <= 1e-8*max(1, abs(expected)), "group/edge projection sum differs")
    transitions = []
    links = [("all_input", "accepted_input"), ("accepted_input", "accepted_leak_input"), ("accepted_leak_input", "spike_counts")]
    for ident, values in grouped_signals.items():
        for before, after in links:
            transitions.append(dict(**dict(zip(ident_fields, ident)), before=before, after=after,
                                    before_direction=values[before]["direction"], after_direction=values[after]["direction"]))
    for name, rows in [("signals.csv", signals), ("sources.csv", groups), ("edges.csv", edges),
                       ("windows.csv", windows), ("transitions.csv", transitions)]:
        write_csv(root/name, rows)
    summaries = []
    buckets = defaultdict(list)
    for row in signals:buckets[row["arm"], row["stage"], row["intervention"], row["signal"]].append(row)
    for (arm, stage, intervention, signal), rows in buckets.items():
        summaries.append(dict(arm=arm, stage=stage, intervention=intervention, signal=signal,
                              seeds=len({r["seed"] for r in rows}), pairs=len(rows),
                              positive=sum(r["direction"] == 1 for r in rows), negative=sum(r["direction"] == -1 for r in rows),
                              zero=sum(r["direction"] == 0 for r in rows), mean_contrast=statistics.mean(r["contrast"] for r in rows)))
    write_csv(root/"summary.csv", summaries)
    m10.m9.m6.json_out(root/"diagnostics.json", dict(protocol=PROTOCOL, retrospective=True,
        trace_files=len(audits), windows=len(windows), pairs=len(grouped_signals), signals=len(signals),
        source_rows=len(groups), edge_rows=len(edges), trajectory_audits=audits,
        max_voltage_reconstruction_error=max(r["voltage_error_linf"] for r in windows),
        new_neural_runs=0, new_decoder_fits=0, group_edge_sums_verified=True,
        original_count_axis_reproduced=True, source_hashes_preserved=True))
    labels = dict(all_input="Alle Eingangsimpulse", accepted_input="Nach Refraktärfilter",
                  all_leak_input="Leckgewichtete Eingänge", accepted_leak_input="Leckgewichtete angenommene Eingänge",
                  linear_drive="Antrieb vor Resetabzug", reset_loss="Resetabzug", residual_voltage="Restpotenzial nach Reset",
                  spike_counts="Spikezählmuster / M10-B-Achse")
    lines = ["# M10-C – Signalbeiträge entlang fester Auslesergewichte", "",
             "Retrospektive algebraische Diagnose aufgezeichneter M10-Trajektorien, keine neue Simulation und kein Lernnachweis.", "",
             "Eingänge und Potenziale werden auf die neuronale Relevanz des vorhandenen Spikezähl-Auslesers projiziert. Dies sind keine eigenständig kalibrierten Eingangs-/Potenzialklassifikatoren.", "",
             "## Gekoppelter Arm, unveränderte Gewichte", "",
             "| Signal | Sofort: positiv / negativ / null | Nach Ruhe: positiv / negativ / null |", "|---|---:|---:|"]
    for signal in core.SIGNALS:
        rows = [next(r for r in summaries if r["arm"] == "paired" and r["intervention"] == "sham" and r["signal"] == signal and r["stage"] == stage)
                for stage in ["checkpoint_probe", "rest_checkpoint_probe"]]
        lines.append(f"| {labels[signal]} | {rows[0]['positive']} / {rows[0]['negative']} / {rows[0]['zero']} | {rows[1]['positive']} / {rows[1]['negative']} / {rows[1]['zero']} |")
    lines += ["", "## Übergänge, gekoppelter Arm, unveränderte Gewichte", "",
              "| Zeitpunkt / Übergang | Positiv → negativ | Negativ → positiv |", "|---|---:|---:|"]
    for stage in ["checkpoint_probe", "rest_checkpoint_probe"]:
        for before, after in links:
            rows = [r for r in transitions if r["arm"] == "paired" and r["intervention"] == "sham" and r["stage"] == stage and r["before"] == before and r["after"] == after]
            lines.append(f"| {stage}: {before} → {after} | {sum(r['before_direction'] == 1 and r['after_direction'] == -1 for r in rows)} | {sum(r['before_direction'] == -1 and r['after_direction'] == 1 for r in rows)} |")
    lines += ["", "Positive/negative Vorzeichen betreffen nur die feste Neuronengewichtung. Unterschiedliche physikalische Einheiten werden nicht als vergleichbare Stärken oder Genauigkeiten behandelt.",
              "", "Das Endpotenzial ist ein Restzustand nach den beobachteten Spikes und Resets. Der Resetabzug kann seine Richtung umkehren, obwohl Spikes erzeugt wurden.",
              "", "Die linearen Terme sind eine Bilanz der beobachteten wiederkehrenden Eingänge, keine Resimulation eines Netzwerks ohne Reset oder Hemmung.",
              "", "[Signale](signals.csv) · [Quellgruppen](sources.csv) · [Synaptische Einzelbeiträge](edges.csv) · [Übergänge](transitions.csv) · [Audit](diagnostics.json)."]
    (root/"summary.md").write_text("\n".join(lines)+"\n")
    print("Report:", root/"summary.md")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=["plan", "run", "report"])
    parser.add_argument("--source", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--stage", choices=["diagnostic", "smoke"], default="diagnostic")
    args = parser.parse_args()
    if args.mode == "report":report(args.output); return
    core.require(args.source is not None, "--source required")
    expected = plan(args.source, args.output, args.stage)
    args.output.mkdir(parents=True, exist_ok=True)
    path = args.output/"manifest.json"
    if path.exists():core.require(json.loads(path.read_text()) == expected, "existing M10-C manifest differs")
    else:m10.m9.m6.json_out(path, expected)
    if args.mode == "plan":print("Plan:", path)
    else:report(args.output)


if __name__ == "__main__":main()
