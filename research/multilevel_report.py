#!/usr/bin/env python3
"""Independently check and summarize a completed multilevel native record."""
import argparse
from collections import Counter
import hashlib
import json
import math
from pathlib import Path


def require(condition, message):
    if not condition:
        raise ValueError(message)


def norm(vector):
    return math.sqrt(math.fsum(v*v for v in vector))


def cosine(a, b):
    aa = math.fsum(x*x for x in a)
    bb = math.fsum(x*x for x in b)
    return math.fsum(x*y for x, y in zip(a,b))/math.sqrt(aa*bb) if aa and bb else 0.


def verify_responses(observations):
    baseline = observations[0]["probe"]["frames"]
    targets = [baseline[1]["core_counts"], baseline[2]["core_counts"]]
    def margins(frames):
        return [cosine(frames[i+3]["core_counts"], targets[i])-cosine(frames[i+3]["core_counts"], targets[1-i]) for i in range(2)]
    base = margins(baseline)
    for row in observations:
        actual = margins(row["probe"]["frames"])
        gains = [a-b for a,b in zip(actual,base)]
        require(all(abs(a-b)<1e-12 for a,b in zip(actual,row["completion_margins"])), "native completion margin differs")
        require(all(abs(a-b)<1e-12 for a,b in zip(gains,row["margin_gains"])), "native margin gain differs")


def weights(row):
    return {int(i): w for i, w in row["weights"]}


def stored_potential_difference(a, b):
    # A full electrical-ledger comparison can include input changes without a Vm change.
    # This stricter measurement compares only native stored membrane potentials.
    compared = 0
    different = 0
    different_times = 0
    for left, right in zip(a["frames"], b["frames"]):
        right_nodes = {n["id"]: n for n in right["neurons"]}
        for node in left["neurons"]:
            if node["id"] in right_nodes:
                other = right_nodes[node["id"]]
                compared += 1
                different += node["stored_after"] != other["stored_after"]
                different_times += node.get("stored_after_time_us") != other.get("stored_after_time_us")
    return {"compared_neuron_frames": compared, "different_stored_potentials": different, "different_sample_times": different_times}


def replay_summary(rows):
    positive = math.fsum(r["positive_actual"] for r in rows)
    negative = math.fsum(r["negative_actual"] for r in rows)
    positive_tag = math.fsum(r["positive_tag_integral"] for r in rows)
    negative_tag = math.fsum(r["negative_tag_integral"] for r in rows)
    requested_available = all(r["positive_requested"] is not None for r in rows)
    return {"weight_l2": norm([r["weight"]-r["initial_weight"] for r in rows]),
            "weight_l1": math.fsum(abs(r["weight"]-r["initial_weight"]) for r in rows),
            "weight_changes": sum(r["weight_changes"] for r in rows),
            "positive_actual_mass": positive, "negative_actual_mass": negative,
            "negative_actual_mass_share": -negative/(positive-negative) if positive-negative else None,
            "positive_tag_integral": positive_tag, "negative_tag_integral": negative_tag,
            "negative_tag_integral_share": -negative_tag/(positive_tag-negative_tag) if positive_tag-negative_tag else None,
            "positive_requested": math.fsum(r["positive_requested"] for r in rows) if requested_available else None,
            "negative_requested": math.fsum(r["negative_requested"] for r in rows) if requested_available else None,
            "pending_residual_l1": math.fsum(abs(r["residual"]) for r in rows)}


def summarize(record):
    require(record["full_replay_checked"] is True, "full native replay required")
    d = record["details"]
    require(d["offline"]["verified_original_local_state"] is True, "offline reconstruction failed")
    initial = {int(row[0]): row[3] for row in d["B_training"]["weights_before"]}
    native = {int(row[0]): row[3] for row in d["B_training"]["weights_after"]}
    residuals = dict(d["B_training"]["weight_residuals_after"])
    for row in d["offline"]["original"]["synapses"]:
        require(row["weight"] == native[row["synapse_id"]], "offline original weight differs")
        require(row["residual"] == residuals[row["synapse_id"]], "offline original residual differs")
    observations = d["matched_observations"]
    verify_responses(observations)
    baseline = observations[0]
    reference = observations[d["reference"]["observation"]]
    swap = observations[d["reference"]["swap_observation"]]
    verified = (reference["eligible"] and all(a > b+1e-9 for a, b in zip(reference["margin_gains"], swap["margin_gains"])))
    require(verified == d["reference"]["verified"], "reference specificity gate differs")
    source_indices = {"B_original": d["B_natural_observation"], "offline_constant": d["offline_constant_observation"],
                      "offline_pair": d["offline_pair_observation"], "E_reference": d["reference"]["observation"]}
    arms = {}
    for name, arm in d["live"].items():
        require(arm["initial_network_digest"] == d["initial_network_digest"], "initial network differs")
        require(arm["training"]["input_digest"] == d["B_training"]["input_digest"], "generated input differs")
        source_indices[name] = arm["matched_B_checkpoint_observation"]
        own = arm["own_checkpoint_response"]
        selected = set(arm["history"]["selection"]["neurons"])
        expected_spikes = Counter((t, i) for t, i in d["offline"]["post_spikes"] if i in selected)
        actual_spikes = Counter((e["time_us"], e["neuron_id"]) for e in arm["history"]["events"] if e["mechanism"] == "spike" and e["phase"] == "training")
        spike_comparison = {"selected_neurons": sorted(selected), "original_spikes": sum(expected_spikes.values()),
                            "live_spikes": sum(actual_spikes.values()), "identical": actual_spikes == expected_spikes,
                            "original_only_examples": sorted((expected_spikes-actual_spikes).elements())[:6],
                            "live_only_examples": sorted((actual_spikes-expected_spikes).elements())[:6]}
        arms[name] = {"weight_paths": arm["training"]["weight_changes"],
                      "gain_vs_own_reset": own["margin_gains"], "core_effect": own["core_effect"], "motor_effect": own["motor_effect"],
                      "offline_vs_live_different_weights": arm["offline_vs_live_different_weights"],
                      "offline_vs_live_l2": arm["offline_vs_live_l2"],
                      "training_core_spikes": arm["training"]["core_spikes"], "selected_spike_history_comparison": spike_comparison}
    normalized = {}
    for name, levels in d["normalized"].items():
        source = weights(observations[source_indices[name]])
        direction = [source[i]-initial[i] for i in initial]
        normalized[name] = {}
        for level, plan in levels.items():
            result = {"executed": plan["observation"] is not None, "faithful": plan["faithful"], "requested_norm": plan["requested_norm"]}
            if plan["observation"] is not None:
                row = observations[plan["observation"]]
                installed = weights(row)
                vector = [installed[i]-initial[i] for i in initial]
                magnitude = norm(vector)
                error = abs(magnitude-plan["requested_norm"])/plan["requested_norm"] if plan["requested_norm"] else 0.
                cosine = math.fsum(a*b for a, b in zip(vector, direction))/(magnitude*norm(direction)) if magnitude*norm(direction) else None
                require(error <= .010000001 and cosine is not None and cosine >= .999-1e-12, "unfaithful normalized arm was executed")
                require(row["only_weights_changed"] is True, "missing matched checkpoint verification")
                result.update(actual_norm=magnitude, relative_norm_error=error, direction_cosine=cosine,
                              margin_gains=row["margin_gains"], core_effect=row["core_effect"], motor_effect=row["motor_effect"],
                              stored_potential_comparison=stored_potential_difference(row["probe"], baseline["probe"]))
            else:
                result["reason"] = plan["reason"] or "f32_publication_fidelity"
            normalized[name][level] = result
    samples = d["offline"]["chemical_samples"]
    chemical = {"samples": len(samples), "negative_learning_factor_samples": sum(s[5]<0 for s in samples),
                "minimum_learning_factor": min(s[5] for s in samples), "maximum_learning_factor": max(s[5] for s in samples),
                "maximum_dopamine": max(s[2] for s in samples)}
    return {"protocol": "NERVA-Multilevel-Summary/1", "seed": record["job"]["seed"], "metrics": record["metrics"],
            "input_digest": d["B_training"]["input_digest"], "offline_original_verified": True, "native_chemical_observations": chemical,
            "offline": {key: replay_summary(d["offline"][key]["synapses"]) for key in ("original", "constant_positive", "immediate_pair")},
            "live": arms, "reference": {**d["reference"], "margin_gains": reference["margin_gains"], "swap_margin_gains": swap["margin_gains"]},
            "natural_gains": observations[d["B_natural_observation"]]["margin_gains"],
            "matched_raw_gains": {name: observations[index]["margin_gains"] for name, index in source_indices.items()},
            "normalized": normalized, "common_high_norm": d["common_high_norm"],
            "local_replay_events": {"clocks": len(d["offline"]["clock_times_us"]), "chemicals": len(d["offline"]["chemical_samples"]),
                                    "pre_arrivals": len(d["offline"]["pre_arrivals"]), "post_spikes": len(d["offline"]["post_spikes"])},
            "selected_synapses": d["B_history"]["selection"]["synapses"],
            "limits": [d["limits"], "The reference recipe uses these images; a new seed tests protocol transfer, not independent task learning.",
                       "Unexecuted bounded/quantized comparisons do not establish a direction contrast."]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("root", type=Path)
    args = parser.parse_args()
    try:
        files = sorted((args.root/"records").glob("*.json"))
        require(len(files) == 1, "expected one completed seed record")
        raw = files[0].read_bytes()
        result = summarize(json.loads(raw)["record"])
        result["record_sha256"] = hashlib.sha256(raw).hexdigest()
        print(json.dumps(result, indent=2, sort_keys=True, allow_nan=False))
    except (OSError, ValueError, KeyError) as exc:
        parser.exit(1, f"multilevel-report: {exc}\n")


if __name__ == "__main__":
    main()
