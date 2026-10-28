import math
import statistics
from .mutation import SplitMix64
from .spec import require

GROUPS = ["none", "disable_stdp", "shuffle_input", "disable_homeostasis"]


def evaluate(records, seeds, requirements):
    indexed = {}
    for record in records:
        key = (record["job"]["seed"], record["job"]["intervention"])
        require(key not in indexed, "duplicate selection trial")
        indexed[key] = record
    require(set(indexed) == {(s, g) for s in seeds for g in GROUPS}, "selection study lacks paired seeds/controls")
    checks = dict(stability=True, reproducibility=True, noncollapse=True, memory=True, resource_limits=True)
    reasons = []
    for (seed, group), record in indexed.items():
        m = record["metrics"]
        required = ["sequence_score", "retained_sequence_score", "event_count"]
        required += [prefix+name for prefix in ["", "retained_"] for name in ["stability", "frozen_replay_identical", "weights_frozen", "firing_rate", "rates_finite", "silent_fraction", "persistent_fraction"]]
        require(all(type(m.get(k)) in [int, float] and math.isfinite(m[k]) for k in required), "invalid or missing selection metrics")
        flags = dict(
            stability=all(m[p+"stability"] == 1 and m[p+"persistent_fraction"] == 0 for p in ["", "retained_"]),
            reproducibility=record["full_replay_checked"] is True and all(m[p+"frozen_replay_identical"] == m[p+"weights_frozen"] == 1 for p in ["", "retained_"]),
            noncollapse=all(m[p+"silent_fraction"] < 1 and m[p+"rates_finite"] == 1 for p in ["", "retained_"]),
            resource_limits=0 <= m["event_count"] <= requirements["max_event_count"] and all(0 <= m[p+"firing_rate"] <= requirements["max_probe_rate_hz"] for p in ["", "retained_"]))
        if group == "none":
            flags["memory"] = m["retained_sequence_score"] >= requirements["min_retained_score"] and m["sequence_score"]-m["retained_sequence_score"] <= requirements["max_retention_loss"]
        for key, passed in flags.items():
            checks[key] &= passed
            if not passed:reasons.append(dict(seed=seed, intervention=group, requirement=key))
    advantages = []
    for seed in seeds:
        candidate = indexed[seed, "none"]["metrics"]
        advantages.append([candidate[p+"sequence_score"]-indexed[seed, g]["metrics"][p+"sequence_score"]
                           for p in ["", "retained_"] for g in ["disable_stdp", "shuffle_input"]])
    quality = min(statistics.mean(row[i] for row in advantages) for i in range(4))
    eligible = all(checks.values())
    return dict(eligible=eligible, checks=checks, rejection_reasons=reasons, quality=quality if eligible else None,
        raw_quality=quality, resource=statistics.mean(indexed[s, "none"]["metrics"]["event_count"] for s in seeds),
        seeds=seeds, seed_advantages=advantages, seed_quality=[min(row) for row in advantages],
        memory_scope="Extra frozen-probe delay; no test of ongoing-plasticity forgetting or independent tasks",
        resource_scope="Deterministic event-log count proxy, not wall time or total hardware cost")


def sign_p(differences):
    wins = sum(x > 0 for x in differences); n = sum(x != 0 for x in differences)
    return sum(math.comb(n, k) for k in range(wins, n+1))/2**n if n else 1.0


def quality(rows):return min(statistics.mean(row[i] for row in rows) for i in range(4))


def validate(candidate, baseline, policy):
    require(candidate["seeds"] == baseline["seeds"], "validation candidates must share seeds")
    n = len(candidate["seeds"]); rng = SplitMix64(1234); deltas = []
    for _ in range(policy["bootstrap_samples"]):
        indices = [rng.next() % n for _ in range(n)]
        deltas.append(quality([candidate["seed_advantages"][i] for i in indices])-quality([baseline["seed_advantages"][i] for i in indices]))
    deltas.sort()
    interval = [deltas[int(len(deltas)*.025)], deltas[min(len(deltas)-1, int(len(deltas)*.975))]]
    p_fixed = sign_p([min(row[0], row[2]) for row in candidate["seed_advantages"]])
    p_random = sign_p([min(row[1], row[3]) for row in candidate["seed_advantages"]])
    p_baseline = sign_p([a-b for a, b in zip(candidate["seed_quality"], baseline["seed_quality"])])
    delta = candidate["raw_quality"]-baseline["raw_quality"]
    resource_ok = candidate["resource"] <= baseline["resource"]*policy["max_resource_ratio"]
    checks = dict(candidate_requirements=candidate["eligible"], baseline_requirements=baseline["eligible"],
        quality_gain=delta > policy["min_quality_gain"], bootstrap_lower_positive=interval[0] > 0,
        paired_tests=all(p < policy["alpha"]/3 for p in [p_fixed, p_random, p_baseline]),
        positive_control_advantage=candidate["raw_quality"] > 0, resource_ratio=resource_ok)
    return dict(held_out_quality_improved=all(checks.values()), checks=checks,
        quality_delta=delta, bootstrap_ci95=interval, bootstrap_samples=policy["bootstrap_samples"], bootstrap_seed=1234,
        sign_p_fixed=p_fixed, sign_p_random=p_random, sign_p_baseline=p_baseline,
        sign_threshold=policy["alpha"]/3,
        resource_ratio=candidate["resource"]/baseline["resource"] if baseline["resource"] else None,
        limits="One frozen finalist versus baseline on held-out seeds of the same M0 task. No independent task or general NERVA learning claim; no automatic deployment.")
