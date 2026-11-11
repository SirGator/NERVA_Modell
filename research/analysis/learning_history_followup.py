"""Read-only, reproducible analysis of paired Learning History controls."""
import argparse
from collections import Counter
import hashlib
import json
import math
from pathlib import Path

GROUPS = ("three_on", "three_off", "pair_on", "pair_off", "frozen_on")
SYNAPSES = (52, 57, 360)


def require(condition, message):
    if not condition:
        raise ValueError(message)


def sign(value):
    return "positive" if value > 0 else "negative" if value < 0 else "zero"


def negative_share(positive, negative):
    mass = positive - negative
    return -negative / mass if mass else None


def summarize_synapse(history, synapse):
    edge = next(s for s in history["synapse_summary"] if s["synapse_id"] == synapse)
    require(synapse in history["selection"]["synapses"], "synapse detail was not recorded")
    events = [e for e in history["events"] if e["synapse_id"] == synapse and e["phase"] == "training"]
    traces = [e for e in events if e["stream"] == "learning"]
    changes = [e for e in traces if e["data"]["actual_delta"] != 0]
    require(len(changes) == edge["weight_events"], "selected history omitted weight changes")
    require(math.isclose(sum(e["data"]["actual_delta"] for e in changes),
                         edge["weight_after"] - edge["weight_before"], abs_tol=1e-12),
            "weight ledger does not balance")
    delayed = [e for e in traces if e["mechanism"] == "three_factor"]
    cases = Counter()
    residual_direction_differences = 0
    for event in delayed:
        data = event["data"]
        # This assay uses the native ThreeFactorConfig::default learning_rate=1.
        require(math.isclose(data["requested_delta"], data["tag_integral_seconds"] * data["held_learning_signal"],
                             rel_tol=1e-12, abs_tol=1e-30), "native three-factor product differs")
        cases[f'tag_{sign(data["tag_integral_seconds"])}__factor_{sign(data["held_learning_signal"])}__request_{sign(data["requested_delta"])}'] += 1
        residual_direction_differences += sign(data["requested_delta"]) != sign(data["actual_delta"])
    windows = [w for w in history["delayed_windows"] if w["synapse_id"] == synapse]
    totals = {key: sum(w[key] for w in windows) for key in
              ("positive_tag_integral", "negative_tag_integral", "positive_requested", "negative_requested")}
    earliest = traces[:4]
    first_change = changes[:1]
    strongest = [max(delayed, key=lambda e: abs(e["data"]["requested_delta"]))] if delayed else []
    examples = sorted({e["id"]: e for e in earliest + first_change + strongest}.values(), key=lambda e: (e["time_us"], e["id"]))
    return {"edge": edge, "net_delta": edge["weight_after"] - edge["weight_before"],
            "published_delayed_sign_cases": dict(cases), "residual_direction_differences": residual_direction_differences,
            "all_delayed_integrations": totals,
            "negative_tag_integral_share": negative_share(totals["positive_tag_integral"], totals["negative_tag_integral"]),
            "negative_requested_share": negative_share(totals["positive_requested"], totals["negative_requested"]),
            "examples": examples}


def summarize_record(record):
    require(record["full_replay_checked"] is True, "full replay is required")
    require(record["details"]["observation_preserves_full_state"] is True, "observation changed replay state")
    history = record["details"]["history"]
    require(history["complete"] is True, "incomplete history")
    edges = history["synapse_summary"]
    positive = sum(s["positive_actual"] for s in edges)
    negative = sum(s["negative_actual"] for s in edges)
    chemistry = {}
    for neuron in (84, 91):
        samples = [e["data"] for e in history["events"] if e["phase"] == "training"
                   and e["mechanism"] == "neuromodulation" and e["neuron_id"] == neuron]
        chemistry[str(neuron)] = {key: {"min": min(s[key] for s in samples), "max": max(s[key] for s in samples)}
                                 for key in ("learning_signal", "dopamine", "acetylcholine", "noradrenaline")} if samples else None
    return {"metrics": record["metrics"], "weight_paths": record["details"]["training"]["weight_changes"],
            "positive_actual_mass": positive, "negative_actual_mass": negative,
            "negative_actual_mass_share": negative_share(positive, negative),
            "strengthened_synapses": sum(s["weight_after"] > s["weight_before"] for s in edges),
            "weakened_synapses": sum(s["weight_after"] < s["weight_before"] for s in edges),
            "opposing_synapses": sum(s["positive_actual"] > 0 and s["negative_actual"] < 0 for s in edges),
            "opposing_mass": sum(s["opposing_update_mass"] for s in edges),
            "chemistry": chemistry, "synapses": {str(s): summarize_synapse(history, s) for s in SYNAPSES},
            "motor_factor_samples": record["details"]["training"]["audit"]["nonzero_motor_factor_samples"]}


def analyze(records):
    groups = {}
    for record in records:
        group = record["job"]["intervention"]
        require(group not in groups, "duplicate control")
        groups[group] = record
    require(set(groups) == set(GROUPS), "expected all five fixed controls")
    require(len({r["job"]["seed"] for r in records}) == 1, "controls must share one seed")
    require(len({r["details"]["training"]["input_digest"] for r in records}) == 1, "input schedules differ")
    require(len({json.dumps(r["details"]["training"]["weights_before"], sort_keys=True) for r in records}) == 1,
            "initial weights or synapse identities differ")
    require(len({json.dumps(r["details"]["history"]["neurons"], sort_keys=True) for r in records}) == 1,
            "selected initial neuron parameters differ")
    return {"protocol": "NERVA-Learning-History-Followup/1", "seed": records[0]["job"]["seed"],
            "initial_weights_topology_selected_neurons_and_inputs_match": True,
            "groups": {name: summarize_record(groups[name]) for name in GROUPS},
            "limits": ["Post-hoc selection of the three largest seed-7 weight changes; no independent confirmation.",
                       "Sign cases cover published delayed updates. Window integrals include no-change integrations.",
                       "Native eligibility-integral × held-factor arithmetic is checked with the unchanged default learning rate.",
                       "Unweighted eligibility integrals are descriptive arithmetic, not a simulated chemical knockout.",
                       "Controls can change neural trajectories; they do not isolate an individual mechanism from a common intermediate state."]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("root", type=Path)
    args = parser.parse_args()
    try:
        paths = sorted((args.root / "records").glob("*.json"))
        records = [json.loads(p.read_text())["record"] for p in paths]
        result = analyze(records)
        result["records_sha256"] = {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in paths}
        provenance = json.loads((args.root / "pre-run-provenance.json").read_text())
        for name, digest in provenance["artifacts_sha256"].items():
            require(hashlib.sha256((args.root / name).read_bytes()).hexdigest() == digest, "pre-run artifact changed: " + name)
        result["pre_run_provenance"] = provenance
        print(json.dumps(result, indent=2, sort_keys=True, allow_nan=False))
    except (ValueError, KeyError, StopIteration, OSError) as exc:
        parser.exit(1, f"history-followup: {exc}\n")


if __name__ == "__main__":
    main()
