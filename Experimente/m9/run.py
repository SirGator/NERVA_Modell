#!/usr/bin/env python3
"""M9: fixed 2x2 combination test, using the M8 replay and independent audit."""
import argparse
import csv
import importlib.util
import json
import statistics
import subprocess
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

EXPERIMENTS = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("nerva_m8_shared", EXPERIMENTS / "m8/run.py")
m8 = importlib.util.module_from_spec(spec)
spec.loader.exec_module(m8)
m6 = m8.m6
reset_spec = importlib.util.spec_from_file_location("nerva_m9_resets", Path(__file__).with_name("weight_resets.py"))
resets = importlib.util.module_from_spec(reset_spec)
reset_spec.loader.exec_module(resets)

PROTOCOL = "NERVA-M9-v2"
LEVELS = [
    ("reference", 1, 20000),
    ("weight4", 4, 20000),
    ("tau100", 1, 100000),
    ("weight4_tau100", 4, 100000),
]
SEEDS = {"smoke": [692], "pilot": [600, 601, 602], "confirm": list(range(610, 630))}
COUNTS = (256, 128, 128)
FAMILY_SIZE = len(LEVELS) * len(m8.PRIMARY)


def cases(seed, counts=COUNTS):
    m6.require(len(counts) == 3 and all(type(n) is int and 8 <= n <= 4096 and n % 4 == 0 for n in counts),
               "trial counts must be multiples of four in 8..4096")
    return [dict(protocol=PROTOCOL, case_id=name, seed=seed,
                 train_trials=counts[0], test_trials=counts[1], calibration_trials=counts[2],
                 condition=dict(core_weight_scale=weight, stdp_tau_us=tau))
            for name, weight, tau in LEVELS]


def plan(stage, counts=None):
    m6.require(stage in SEEDS, "unknown M9 stage")
    counts = tuple(counts) if counts is not None else ((8, 8, 8) if stage == "smoke" else COUNTS)
    m6.require(stage == "smoke" or counts == COUNTS,
               "pilot/confirm trial counts are fixed; use --stage smoke for software checks")
    return dict(
        protocol=PROTOCOL, stage=stage, seeds=SEEDS[stage], conditions=cases(0, counts),
        trial_counts=list(counts), scientific_run=stage != "smoke",
        preregistration=dict(
            date="2026-10-27",
            hypothesis="Combined recurrent propagation and longer timing sensitivity may enable learned action-effect prediction",
            design="Fixed 2x2; all four conditions carried unchanged from pilot to fresh confirmation",
            fixed="effect_2ch/32, deterministic encoding, three copies at weight0.9, Core tau20ms, action/effect50ms; homeostasis/development off",
            recurrence="Scale all initial Core->Core weights, excitatory and inhibitory, by 1 or 4; endpoints and delays fixed",
            stdp="Existing nearest-pair rule: both plus/minus tau20 or100ms, a_plus0.0001/a_minus0.00012, pair window100ms, weights0..2",
            rate_gain="Same baseline reference gain per seed; no renormalization per condition",
            controls=m6.DEFAULT_ARMS, mappings=m6.MAPS,
            revision="v2 supersedes unexecuted v1 scientific plans; v1 files retained",
            weight_resets="Sham, restore Core only, restore all plastic to condition-specific initial weights; identical replay state; intervention after rest, before 50ms baseline; unchanged checkpoint count decoder; neutral images only",
            causal_success="Behavioral criteria AND sham exceeds restore_all_plastic in each mapping, positive paired bootstrap95 lower bound and exact sign p<=0.00625, at least twenty seeds; restore_core localizes contribution descriptively",
            causal_limits="State and pending arrivals retained; establishes contribution under fixed decoder, not sufficiency or exclusive storage in weights",
            primary=m8.PRIMARY,
            checkpoint="Two separate exact A/D replays per mapping/seed; independent 2s neutral rest; no parent calibration",
            no_copy="Independent neutral-frame replay without action copies at both primary checkpoints",
            success="At least twenty seeds; visual >=80% in each mapping; count prediction above chance and all controls in both mappings; flip/dual-correct above all controls; all lower paired bootstrap95 bounds positive",
            multiplicity=dict(family_size=FAMILY_SIZE, alpha=.05 / FAMILY_SIZE,
                              rule="Every required one-sided exact paired sign test must pass; ties omitted; Bonferroni over four conditions x two endpoints"),
            secondary=["128 online trials and first16", "sequential retention/interference", "time-bin readout"],
            factorial="Descriptive seed-paired combination-minus-reference/singles and interaction Y11-Y10-Y01+Y00, separately per mapping and primary endpoint; raw paired accuracy and learning advantages over each control",
            weights="Initial and trained visual weight sums and retained fraction, per seed/arm; monitor possible LTD cost",
            limits="Extra spikes, weight alignment, or descriptive interaction alone are not learning; smoke/pilot cannot confirm; no parameter tuning or condition selection",
        ),
    )


def validate(r, config, path=None):
    counts = tuple(config[k] for k in ("train_trials", "test_trials", "calibration_trials"))
    m6.require(config in cases(config["seed"], counts), "unknown M9 condition/configuration")
    audited = m8.validate(r, config, path, protocol=PROTOCOL, condition_parameters=config["condition"])
    branches = resets.validate(r, m6.require, path)
    if path is not None:
        audited[0]["weight_reset_branches"] = branches
    return audited


def contrasts(name, values, stage):
    screen, confirmed, rows = m8.contrasts(name, values, stage, family_size=FAMILY_SIZE)
    return screen and len(values) >= 20, confirmed and len(values) >= 20, rows


def factorial_contrasts(records):
    rows = []
    for stage in m8.PRIMARY:
        for mapping in m6.MAPS:
            for control in ["raw", "before", "aged_frozen", "scrambled"]:
                def vector(case):
                    values = []
                    for r in records[case]:
                        accuracy = r["arms"]["paired"][mapping][stage]["count"]["balanced_accuracy"]
                        if control != "raw":
                            accuracy -= r["arms"][control][mapping][stage]["count"]["balanced_accuracy"]
                        values.append(accuracy)
                    return values

                a, b, c, d = [vector(name) for name, _, _ in LEVELS]
                differences = [("combination_minus_reference", d, a),
                               ("combination_minus_weight4", d, b),
                               ("combination_minus_tau100", d, c)]
                interaction = [dd - bb - cc + aa for aa, bb, cc, dd in zip(a, b, c, d)]
                differences.append(("interaction", interaction, [0] * len(a)))
                for name, left, right in differences:
                    estimate = m6.bootstrap_paired(left, right)
                    rows.append(dict(stage=stage, mapping=mapping, control=control,
                                     contrast=name, seeds=len(a), mean_difference=estimate["mean_difference"],
                                     ci_low=estimate["ci95"][0], ci_high=estimate["ci95"][1],
                                     inference="descriptive; not a learning or synergy gate"))
    return rows


def validate_matrix(records):
    for values in records.values():
        for r, baseline in zip(values, records["reference"]):
            m6.require(r["reference_normalization"] == baseline["reference_normalization"],
                       "M9 baseline normalization differs across conditions")


def report(root):
    manifest = json.loads((root / "manifest.json").read_text())
    m6.require(manifest == plan(manifest["stage"], manifest["trial_counts"]),
               "M9 manifest differs from fixed plan")
    records, summaries, weights = m8.report(
        root, validator=validate, contrast_fn=contrasts, protocol=PROTOCOL,
        label="M9", family_size=FAMILY_SIZE, matrix_validator=validate_matrix,
    )
    causal_rows = []
    for row in summaries:
        causal, extra = resets.contrasts(row["case"], records[row["case"]], m6.bootstrap_paired, m8.sign_p, .05 / FAMILY_SIZE)
        causal_rows.extend(extra)
        for stage in m8.PRIMARY:
            row[stage + "_behavior_confirmed"] = row[stage + "_confirmed"]
            row[stage + "_weight_causality_confirmed"] = causal[stage]
            row[stage + "_confirmed"] = row[stage + "_behavior_confirmed"] and causal[stage]
    factorial = factorial_contrasts(records)
    for row in weights:
        row["visual_retained_fraction"] = row["visual_trained_sum"] / row["visual_initial_sum"]
    for filename, rows in [("factorial-contrasts.csv", factorial), ("weight-summary.csv", weights), ("summary.csv", summaries), ("causal-contrasts.csv", causal_rows)]:
        with (root / filename).open("w", newline="") as stream:
            writer = csv.DictWriter(stream, fieldnames=list(rows[0]))
            writer.writeheader()
            writer.writerows(rows)
    diagnostics_path = root / "diagnostics.json"
    diagnostics = json.loads(diagnostics_path.read_text())
    diagnostics.update(scientific_run=manifest["scientific_run"],
                       stage=manifest["stage"], factorial_contrasts=factorial,
                       weight_reset_branches=sum(a["weight_reset_branches"] for a in diagnostics["audits"]),
                       causal_contrasts=causal_rows,
                       reference_normalization_matched=True)
    m6.json_out(diagnostics_path, diagnostics)
    summary_path = root / "summary.md"
    summary_path.write_text(summary_path.read_text().replace(" – Kriterien einschließlich Familienkorrektur:", " – Verhaltenskriterien einschließlich Familienkorrektur:"))
    with summary_path.open("a") as stream:
        stream.write("\nM9-v2 verlangt mindestens 20 Seeds und zusätzlich einen kontrollierten Gewichtseffekt.\n")
        for stage in m8.PRIMARY:
            passed = ", ".join(row["case"] for row in summaries if row[stage + "_confirmed"]) or "keine"
            stream.write(f"\n{stage} – Gesamtnachweis einschließlich Gewichtskausalität: {passed}.\n")
        stream.write("\n[Gewichtsrücksetzungen und gepaarte Kontraste](causal-contrasts.csv). "
                     "Der Ausleser bleibt fest; Membran-/Spurzustand und reisende Impulse bleiben erhalten.\n")
        stream.write("\n" + ("Technischer Kurzlauf; kein wissenschaftlicher Lernnachweis."
                              if manifest["stage"] == "smoke" else
                              "Pilot: keine Bestätigung; alle Bedingungen unverändert auf neuen Seeds prüfen."
                              if manifest["stage"] == "pilot" else
                              "Bestätigungsreihe mit vorab festgelegten Bedingungen und neuen Seeds.") + "\n")
        stream.write("\n[Faktorielle Kontraste](factorial-contrasts.csv) sind deskriptiv; "
                     "eine positive Interaktion allein belegt keinen Lernerfolg.\n")
        stream.write("\n| Fall | Erhaltener visueller Gewichtsanteil, gekoppelt |\n|---|---:|\n")
        for name, _, _ in LEVELS:
            retained = statistics.mean(w["visual_retained_fraction"] for w in weights
                                       if w["case"] == name and w["arm"] == "paired")
            stream.write(f"| {name} | {retained:.1%} |\n")
    return records, summaries, weights


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=["plan", "run", "report"])
    parser.add_argument("--stage", choices=list(SEEDS), default="pilot")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--jobs", type=int, default=4)
    for name in ("train", "test", "calibration"):
        parser.add_argument(f"--{name}-trials", type=int)
    args = parser.parse_args()
    root = args.output
    if args.mode == "report":
        report(root)
        return
    m6.require(1 <= args.jobs <= 16, "jobs must be 1..16")
    defaults = (8, 8, 8) if args.stage == "smoke" else COUNTS
    supplied = (args.train_trials, args.test_trials, args.calibration_trials)
    counts = tuple(default if value is None else value for default, value in zip(defaults, supplied))
    manifest = plan(args.stage, counts)
    root.mkdir(parents=True, exist_ok=True)
    path = root / "manifest.json"
    if path.exists():
        m6.require(json.loads(path.read_text()) == manifest, "manifest differs; choose new directory")
    else:
        m6.json_out(path, manifest)
    if args.mode == "plan":
        print("Plan:", path)
        return

    def execute(item):
        template, seed = item
        config = dict(template, seed=seed)
        out = root / "results" / config["case_id"] / f"seed-{seed}.json"
        if out.exists():
            validate(json.loads(out.read_text()), config, out)
            return f"VALID {config['case_id']} seed {seed}"
        cfg = root / "configs" / config["case_id"] / f"seed-{seed}.json"
        m6.json_out(cfg, config)
        out.parent.mkdir(parents=True, exist_ok=True)
        proc = subprocess.run([str(EXPERIMENTS / "run.sh"), "m9", "--config", str(cfg),
                               "--output", str(out)], capture_output=True, text=True)
        out.with_suffix(".log").write_text(proc.stdout + proc.stderr)
        m6.require(proc.returncode == 0,
                   f"failed {config['case_id']} seed {seed}: {proc.stderr[-1200:]}")
        validate(json.loads(out.read_text()), config, out)
        return f"OK {config['case_id']} seed {seed}"

    with ThreadPoolExecutor(max_workers=args.jobs) as pool:
        for status in pool.map(execute, [(c, s) for c in manifest["conditions"] for s in manifest["seeds"]]):
            print(status, flush=True)
    report(root)


if __name__ == "__main__":
    main()
