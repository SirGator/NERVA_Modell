"""Scenario-specific endpoints and policies; no neural learning happens here."""
from dataclasses import dataclass
import hashlib
import json
import math
import statistics

from evolution import evaluator
from evolution.mutation import SplitMix64
from evolution.spec import require

SENSORIMOTOR_GROUPS = ["active", "active_frozen", "passive", "passive_frozen", "decoupled", "decoupled_frozen"]
SENSORIMOTOR_LIMITS = [
    "Prescribed balanced A/D exploration and diagnostic frame-only position encoding, not autonomous goal pursuit.",
    "Visual-only external decoding of pre-effect activity; labels never feed local neural learning.",
    "Passive removes executed-action copies during training and probe; decoupled training is probed on coupled dynamics.",
    "Exploratory results on the declared seeds and one persistent discrete world; no new-task, memory or general cognition claim.",
]


def sensorimotor_evaluate(records, seeds, requirements):
    indexed = {(r["job"]["seed"], r["job"]["intervention"]): r for r in records}
    require(len(indexed) == len(records) and set(indexed) == {(s, g) for s in seeds for g in SENSORIMOTOR_GROUPS},
            "sensorimotor study lacks paired seeds/controls")
    checks = dict(reproducibility=True, activity=True, resource_limits=True, visual_readout=True, matched_experience=True)
    reasons = []
    required = ["prediction_accuracy", "position_mae", "persistence_mae", "majority_accuracy", "visual_accuracy", "core_rate_hz", "peak_core_rate_hz",
                "silent_fraction", "observed_events", "maximum_synapses", "parameters_preserved",
                "weight_updates", "homeostasis_updates", "formed", "pruned", "action_copy_arrivals"]
    for (seed, group), record in indexed.items():
        m = record["metrics"]
        require(all(type(m.get(k)) in [int, float] and math.isfinite(m[k]) for k in required),
                "invalid or missing sensorimotor metrics")
        require(all(0 <= m[k] <= 1 for k in ["prediction_accuracy", "visual_accuracy", "silent_fraction"])
                and m["position_mae"] >= 0, "sensorimotor metric out of range")
        flags = dict(
            reproducibility=record["full_replay_checked"] is True and m["parameters_preserved"] == 1,
            activity=0 < m["core_rate_hz"] and m["silent_fraction"] < 1
                     and 0 <= m["peak_core_rate_hz"] <= requirements["max_core_rate_hz"],
            resource_limits=0 <= m["observed_events"] <= requirements["max_observed_events"]
                            and 0 < m["maximum_synapses"] <= requirements["max_synapses"],
            visual_readout=m["visual_accuracy"] >= requirements["min_visual_accuracy"],
        )
        if group.endswith("_frozen"):
            require(all(m[k] == 0 for k in ["weight_updates", "homeostasis_updates", "formed", "pruned"]),
                    "no-learning control changed parameters")
        if group.startswith("passive"):
            require(m["action_copy_arrivals"] == 0, "passive model received action copies")
        for name, passed in flags.items():
            checks[name] &= passed
            if not passed:
                reasons.append(dict(seed=seed, intervention=group, requirement=name))
    pairing={}
    for seed in seeds:
        rows = [indexed[seed, g]["details"] for g in SENSORIMOTOR_GROUPS]
        initial = {r["initial_network_digest"] for r in rows}
        # All conditions have exactly the same image trajectory and timing.
        def sensory(row):
            return [[(p["before_frame"], p["after_frame"], p["action_us"], p["effect_us"])
                     for p in row[stage]] for stage in ["training", "probes"]]
        matched = len(initial) == 1 and all(sensory(r) == sensory(rows[0]) for r in rows)
        checks["matched_experience"] &= matched
        if not matched:
            reasons.append(dict(seed=seed, requirement="matched_experience"))
        pairing[str(seed)]=dict(initial_network=rows[0]["initial_network_digest"],
            sensory_sha256=hashlib.sha256(json.dumps(sensory(rows[0]),sort_keys=True).encode()).hexdigest())
    gains = [indexed[s, "active"]["metrics"]["prediction_accuracy"]
             - indexed[s, "active_frozen"]["metrics"]["prediction_accuracy"] for s in seeds]
    majority = [indexed[s, "active"]["metrics"]["prediction_accuracy"]
                - indexed[s, "active"]["metrics"]["majority_accuracy"] for s in seeds]
    persistence = [indexed[s, "active"]["metrics"]["persistence_mae"]
                   - indexed[s, "active"]["metrics"]["position_mae"] for s in seeds]
    raw = statistics.mean(gains)
    eligible = all(checks.values())
    return dict(eligible=eligible, checks=checks, rejection_reasons=reasons,
                quality=raw if eligible else None, raw_quality=raw, seeds=seeds, seed_quality=gains,
                seed_majority_advantages=majority, seed_persistence_advantages=persistence,
                pairing_fingerprints=pairing,
                resource=statistics.mean(indexed[s, "active"]["metrics"]["observed_events"] for s in seeds),
                quality_scope="Active prediction-accuracy gain over matched control with all plasticity disabled; not an aggregate cognition score",
                resource_scope="Count of observed native events, not FLOPs, memory usage or wall time")


def sensorimotor_validate(candidate, baseline, policy):
    require(candidate["seeds"] == baseline["seeds"], "validation candidates must share seeds")
    differences = [a-b for a, b in zip(candidate["seed_quality"], baseline["seed_quality"])]
    rng = SplitMix64(1234)
    samples = sorted(statistics.mean(differences[rng.next() % len(differences)] for _ in differences)
                     for _ in range(policy["bootstrap_samples"]))
    interval = [samples[int(len(samples)*.025)], samples[min(len(samples)-1, int(len(samples)*.975))]]
    p_control = evaluator.sign_p(candidate["seed_quality"])
    p_baseline = evaluator.sign_p(differences)
    delta = candidate["raw_quality"] - baseline["raw_quality"]
    checks = dict(candidate_requirements=candidate["eligible"], baseline_requirements=baseline["eligible"],
                  quality_gain=delta > policy["min_quality_gain"], bootstrap_lower_positive=interval[0] > 0,
                  paired_tests=max(p_control, p_baseline) < policy["alpha"]/2,
                  positive_control_advantage=candidate["raw_quality"] > 0,
                  beats_naive_baselines=statistics.mean(candidate["seed_majority_advantages"]) > 0
                                        and statistics.mean(candidate["seed_persistence_advantages"]) > 0,
                  resource_ratio=candidate["resource"] <= baseline["resource"]*policy["max_resource_ratio"])
    return dict(held_out_quality_improved=all(checks.values()), checks=checks, quality_delta=delta,
                bootstrap_ci95=interval, bootstrap_samples=policy["bootstrap_samples"], bootstrap_seed=1234,
                sign_p_control=p_control, sign_p_baseline=p_baseline, sign_threshold=policy["alpha"]/2,
                limits="One frozen finalist versus baseline on held-out seeds of the same sensorimotor assay; no independent-task claim")


@dataclass(frozen=True)
class Profile:
    groups: tuple
    study_metrics: tuple
    native_runs_per_job: int
    methods: tuple
    requirements: dict
    scope: str
    assess: object
    validate: object


PROFILES = {
    "m0_selection": Profile(tuple(evaluator.GROUPS),
        ("sequence_score", "retained_sequence_score", "stability", "event_count"), 4,
        ("sensitivity", "phase_mapping", "causal", "robustness", "quality_diversity"),
        dict(min_retained_score=1., max_retention_loss=0., max_probe_rate_hz=200., max_event_count=100000),
        "M0 task", evaluator.evaluate, evaluator.validate),
    "sensorimotor_size": Profile(tuple(SENSORIMOTOR_GROUPS),
        ("prediction_accuracy", "position_mae", "core_rate_hz", "observed_events", "final_synapses"), 2,
        ("size_environment", "plasticity_diagnostics"),
        dict(max_core_rate_hz=200., max_observed_events=2000000, max_synapses=40000, min_visual_accuracy=.5),
        "sensorimotor size/feedback assay", sensorimotor_evaluate, sensorimotor_validate),
}


def profile(name):
    require(isinstance(name, str) and name in PROFILES, "unsupported research scenario: " + str(name))
    return PROFILES[name]
