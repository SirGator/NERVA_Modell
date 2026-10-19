#!/usr/bin/env python3
"""Offline M4 diagnostics; reads completed measurements and never calls the model.

Writes all paired contrasts, per-condition signal/current summaries and an
independent audit of the trial CSV. Missing records or broken audits abort.
Confidence intervals are descriptive, without multiple-comparison correction.
"""
import argparse
from collections import Counter, defaultdict
import csv
import json
import math
from pathlib import Path
import statistics

from runner import DEFAULT_ARMS, bootstrap_paired, json_out, validate_record

MAPPINGS = ("normal_mapping", "reversed_mapping")
STAGES = ("probe", "retained_probe", "interference_probe")
REFERENCES = ("before", "aged_frozen", "scrambled")


def require(condition, message):
    if not condition:
        raise ValueError(message)


def audit_details(path, record):
    config = record["requested_config"]
    delay = config["params"]["action_effect_delay_ms"] * 1000
    groups = defaultdict(list)
    forecasts = defaultdict(lambda: Counter(windows=0, silent_windows=0, core_spikes=0))
    with Path(str(path) + ".details.csv").open(newline="") as file:
        for row in csv.DictReader(file):
            arm, mapping, stage = row["arm"], row["mapping"], row["stage"]
            require(arm in DEFAULT_ARMS and mapping in ("normal", "reversed") and
                    (stage in STAGES or (stage == "training" and arm != "before")),
                    f"unexpected trial group: {path}")
            action, motion, forecast, image = (int(row[key]) for key in
                ("action_us", "motion_us", "forecast_us", "effect_image_us"))
            require(action <= forecast < motion == image and motion - action == delay,
                    f"causal boundary failed: {path}, {arm}, {mapping}, {stage}")
            expected_copies = 3 if config["params"]["motor_feedback"] else 0
            require(int(row["copy_arrivals_before_image"]) == expected_copies,
                    f"missing/unexpected action copies: {path}")
            if expected_copies:
                require(action <= int(row["first_copy_arrival_us"]) <= forecast,
                        f"action copy outside cue: {path}")
            spikes = int(row["cue_core_spikes"])
            if spikes:
                require(action <= int(row["first_cue_spike_us"]) <=
                        int(row["last_cue_spike_us"]) <= forecast,
                        f"Core spike outside prediction window: {path}")
            else:
                require(not row["first_cue_spike_us"] and not row["last_cue_spike_us"],
                        f"empty cue has spike timestamps: {path}")
            if stage != "training" or arm != "scrambled":
                require(row["shown_effect"] == row["actual_effect"],
                        f"effect image differs from physical action: {path}")
            groups[arm, mapping, stage].append(row)
            if stage in STAGES:
                forecasts[arm]["windows"] += 1
                forecasts[arm]["silent_windows"] += int(spikes == 0)
                forecasts[arm]["core_spikes"] += spikes
    for arm in DEFAULT_ARMS:
        for mapping in ("normal", "reversed"):
            for stage in STAGES + (("training",) if arm != "before" else ()):
                rows = groups[arm, mapping, stage]
                n = config["train_trials"] if stage == "training" else config["test_trials"]
                require(len(rows) == n, f"missing/duplicate trial rows: {path}, {arm}, {mapping}, {stage}")
                require(sorted(int(row["trial"]) for row in rows) == list(range(n)),
                        f"missing/duplicate trial IDs: {path}")
                require(Counter(row["key"] for row in rows) == {"a": n // 2, "d": n // 2},
                        f"unbalanced actions: {path}")
                require(Counter(row["shown_effect"] for row in rows) == {"-1": n // 2, "1": n // 2},
                        f"unbalanced images: {path}")
        for mapping in MAPPINGS:
            m = record["arms"][arm][mapping]
            require(m["parameters_preserved"] is True, f"freeze failed: {path}")
            for stage, stats in m["stage_diagnostics"].items():
                require(stage == "training" or not (stats["weight_updates"] or stats["formed"] or stats["pruned"]),
                        f"frozen evaluation changed parameters: {path}")
                require(config["params"]["stdp"] or not stats["weight_updates"], f"STDP-off updates: {path}")
                require(config["params"]["development"] or not (stats["formed"] or stats["pruned"]),
                        f"development-off graph changes: {path}")
                require(config["params"]["motor_feedback"] or not
                        (stats["action_copy_arrivals"] or stats["motor_feedback_arrivals"]),
                        f"feedback-off arrivals: {path}")
    for mapping in ("normal", "reversed"):
        a = groups["aged_frozen", mapping, "training"]
        b = groups["paired", mapping, "training"]
        c = groups["scrambled", mapping, "training"]
        fields = ("trial", "key", "action_us", "motion_us", "forecast_us", "effect_image_us",
                  "shown_effect", "actual_effect", "start_frame_hash", "effect_frame_hash")
        require([[row[k] for k in fields] for row in a] == [[row[k] for k in fields] for row in b],
                f"paired/frozen replay differs: {path}")
        require([(row["key"], row["action_us"]) for row in b] ==
                [(row["key"], row["action_us"]) for row in c], f"scrambled action stream differs: {path}")
        require(Counter((row["key"], row["shown_effect"]) for row in c) ==
                {(key, effect): config["train_trials"] // 4 for key in ("a", "d") for effect in ("-1", "1")},
                f"scrambled association not balanced: {path}")
    phase_rows = 0
    with Path(str(path) + ".audit.csv").open(newline="") as file:
        for row in csv.DictReader(file):
            phase_rows += 1
            weight, homeostasis, formed, pruned = (int(row[k]) for k in
                ("weight_updates", "homeostasis_updates", "formed", "pruned"))
            require(row["stage"] == "training" or not (weight or homeostasis or formed or pruned),
                    f"evaluation changed plastic parameters: {path}")
            require(row["arm"] not in ("before", "aged_frozen") or not (weight or homeostasis or formed or pruned),
                    f"frozen control changed plastic parameters: {path}")
            require(config["params"]["stdp"] or not weight, f"STDP-off phase updates: {path}")
            require(config["params"]["homeostasis"] or not homeostasis, f"homeostasis-off phase updates: {path}")
            require(config["params"]["development"] or not (formed or pruned), f"development-off phase updates: {path}")
            require(all(math.isfinite(float(row[k])) for k in
                        ("core_membrane_min", "core_membrane_mean", "core_membrane_max")),
                    f"nonfinite membrane state: {path}")
    require(phase_rows > 0, f"missing phase audit: {path}")
    return dict(trial_rows=sum(map(len, groups.values())), phase_rows=phase_rows, forecasts=dict(forecasts))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True, help="Completed runner output directory")
    args = parser.parse_args()
    plan = json.loads((args.output / "manifest.json").read_text())
    summaries, contrasts, audits = [], [], []
    for case in plan["cases"]:
        records = []
        cues = {arm: Counter() for arm in DEFAULT_ARMS}
        for seed in plan["seeds"]:
            path = args.output / "results" / case["id"] / f"seed-{seed:02d}.json"
            record = validate_record(json.loads(path.read_text()), seed, case["id"])
            expected = dict(protocol=plan["protocol"], case_id=case["id"], seed=seed,
                            params=case["config"], arms=plan["arms"], **plan["trial_counts"])
            require(record["requested_config"] == expected, f"configuration differs from manifest: {path}")
            audit = audit_details(path, record)
            audits.append(dict(case=case["id"], seed=seed, **audit))
            for arm in DEFAULT_ARMS:
                cues[arm].update(audit["forecasts"][arm])
            records.append(record)
        get = lambda arm, field: statistics.mean(r["arms"][arm][m][field] for r in records for m in MAPPINGS)
        train = lambda arm, field: statistics.mean(r["arms"][arm][m]["stage_diagnostics"]["training"][field]
                                                   for r in records for m in MAPPINGS)
        initial = lambda field: statistics.mean(r["arms"]["paired"][m]["initial_network"][field]
                                                for r in records for m in MAPPINGS)
        trained = lambda field: statistics.mean(r["arms"]["paired"][m]["trained_network"][field]
                                                for r in records for m in MAPPINGS)
        current = {arm: train(arm, "sensor_to_core_current_mean_per_neuron_second") for arm in ("aged_frozen", "paired", "scrambled")}
        zero_fraction = statistics.mean(r["normalization"]["zero_input_core_cells"] /
                                        case["config"]["processing"] for r in records)
        means = {}
        for stage in STAGES:
            for feature in ("count", "time_bins"):
                values = lambda arm, mapping: [statistics.mean(r["arms"][arm][m][stage][feature]["balanced_accuracy"]
                                                  for m in MAPPINGS) if mapping == "both" else
                                                  r["arms"][arm][mapping][stage][feature]["balanced_accuracy"] for r in records]
                means[stage + "_" + feature] = statistics.mean(values("paired", "both"))
                for mapping in ("both",) + MAPPINGS:
                    for reference in REFERENCES:
                        p, q = values("paired", mapping), values(reference, mapping)
                        diff = bootstrap_paired(p, q)
                        contrasts.append(dict(case=case["id"], n_seeds=len(records), mapping=mapping,
                            stage=stage, feature=feature, reference=reference,
                            paired=statistics.mean(p), control=statistics.mean(q),
                            delta=diff["mean_difference"], ci_low=diff["ci95"][0], ci_high=diff["ci95"][1]))
        summaries.append(dict(case=case["id"], n_seeds=len(records), **means,
            visual_by_arm={arm: get(arm, "visual_decodability") for arm in DEFAULT_ARMS},
            sensor_current_target=10.0, measured_training_sensor_current=current,
            initial_core_fraction_without_expected_visual_current=zero_fraction,
            paired_weight_updates=train("paired", "weight_updates"), paired_formed=get("paired", "formed"),
            paired_pruned=get("paired", "pruned"), initial_weight_sum=initial("weight_sum"),
            trained_weight_sum=trained("weight_sum"), weights_at_zero=trained("weights_at_zero"),
            weights_at_upper_bound=trained("weights_at_upper_bound"), forecasts=cues))
    # Publish derived reports only after validating the complete requested matrix.
    json_out(args.output / "diagnostics.json", dict(stage=plan["stage"], cases=summaries,
        audits=audits, validated_records=len(audits), validated_trial_rows=sum(a["trial_rows"] for a in audits),
        validated_phase_rows=sum(a["phase_rows"] for a in audits),
        note="Paired seed bootstrap, descriptive intervals, no multiple-comparison correction. Count is primary; time bins are secondary. Stored membrane snapshots are not a common-time decoder."))
    with (args.output / "contrasts.csv").open("w", newline="") as file:
        writer = csv.DictWriter(file, fieldnames=list(contrasts[0]))
        writer.writeheader()
        writer.writerows(contrasts)
    print(f"Audited {len(audits)} records / {sum(a['trial_rows'] for a in audits)} trial rows; wrote diagnostics.json and contrasts.csv")


if __name__ == "__main__":
    main()
