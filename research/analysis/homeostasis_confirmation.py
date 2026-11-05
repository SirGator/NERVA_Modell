"""Preregistered paired homeostasis contrasts; no selection or neural updates."""
import math
import statistics

from evolution.mutation import SplitMix64
from evolution.spec import flatten, require
from planning.profiles import SENSORIMOTOR_GROUPS

ALPHA = .05
BOOTSTRAP_SAMPLES = 4000


def protocol(source):
    e = source.get("experiment", {})
    name=e.get("name")
    require(isinstance(name,str) and 1<=len(name)<=80
            and all(c.isascii() and (c.isalnum() or c in "-_") for c in name), "invalid confirmation name")
    require(e.get("scenario") == "sensorimotor_size", "confirmation requires sensorimotor_size")
    require(e.get("verify_replay") is True, "confirmation requires complete replay")
    require(e.get("interventions") == SENSORIMOTOR_GROUPS, "confirmation requires all six controls")
    seeds = e.get("seeds")
    require(isinstance(seeds, list) and 1 <= len(seeds) <= 256
            and all(type(s) is int and 0 <= s < 2**64 for s in seeds)
            and len(set(seeds)) == len(seeds), "invalid confirmation seeds")
    axes = source.get("parameters", [])
    require(len(axes) == 2 and {a["name"] for a in axes} == {"processing_neurons", "mechanisms.homeostasis"},
            "confirmation requires exactly size and homeostasis axes")
    by_name = {a["name"]: a["values"] for a in axes}
    sizes = by_name["processing_neurons"]
    require(isinstance(sizes, list) and sizes and len(sizes) <= 256
            and all(type(n) is int and n > 0 for n in sizes) and len(set(sizes)) == len(sizes), "invalid sizes")
    require(by_name["mechanisms.homeostasis"] == [False, True]
            and all(type(v) is bool for v in by_name["mechanisms.homeostasis"]), "homeostasis axis must be [false, true]")
    base = flatten(source.get("base", {}))
    require(base.get("mechanisms.development") is False, "development must be disabled")
    require("mechanisms.homeostasis" not in base and "processing_neurons" not in base, "do not duplicate axis parameters in base")
    require({"visual_accuracy", "position_mae"} <= set(source.get("measurements", {}).get("metrics", [])), "missing primary measurements")
    return dict(protocol="NERVA-homeostasis-confirmation-v1", sizes=sizes, seeds=seeds,
                primary_group="active", alpha=ALPHA, family_tests=2*len(sizes),
                per_test_alpha=ALPHA/(2*len(sizes)), bootstrap_samples=BOOTSTRAP_SAMPLES,
                sign_test="Exact binomial sign tests over seeds; ties remain in n as non-wins. Strict p < alpha/(2*K).",
                visual_accuracy="One-sided: homeostasis-off minus homeostasis-on > 0; positive sample mean also required.",
                position_mae="Two-sided: homeostasis-on minus homeostasis-off differs from zero; sample mean must agree with sign direction.",
                decision="Bonferroni across both primary endpoints and every size; no seed exclusion or finalist selection.",
                bootstrap="Unadjusted percentile 95% intervals of seed-mean differences, descriptive only; not a decision gate.",
                interpretation="Visual decoding and recorded external forecast MAE in this fixed assay. Poor visual calibration is an outcome, not an exclusion; MAE does not isolate latent neural prediction.",
                technical_checks="All six groups, full replay, frozen measurement, no development, switch audits, finite/budgeted metrics, identical initial networks/frames/timing across arms and matched frozen endpoints.")


def tail(n, wins):
    return sum(math.comb(n, k) for k in range(wins, n+1))/2**n


def endpoint(differences, two_sided, threshold, bootstrap_seed):
    n = len(differences)
    wins = sum(d > 0 for d in differences)
    losses = sum(d < 0 for d in differences)
    p = min(1., 2*min(tail(n, wins), tail(n, losses))) if two_sided else tail(n, wins)
    mean = statistics.mean(differences)
    direction_ok = (wins > losses and mean > 0) or (losses > wins and mean < 0) if two_sided else mean > 0
    rng = SplitMix64(bootstrap_seed)
    samples = sorted(statistics.mean(differences[rng.next() % n] for _ in range(n))
                     for _ in range(BOOTSTRAP_SAMPLES))
    return dict(mean_difference=mean, median_difference=statistics.median(differences),
                positive=wins, negative=losses, ties=n-wins-losses, pairs=n,
                sign_p=p, p_threshold=threshold, two_sided=two_sided,
                bootstrap_ci95=[samples[int(.025*len(samples))], samples[int(.975*len(samples))]],
                bootstrap_seed=bootstrap_seed, confirmed=p < threshold and direction_ok,
                per_seed_differences=differences)


def experience(details):
    return [[(p["before_frame"], p["after_frame"], p["action_us"], p["effect_us"])
             for p in details[stage]] for stage in ["training", "probes"]]


def analyze(records, source):
    p = protocol(source)
    base = flatten(source.get("base", {}))
    indexed = {}
    for record in records:
        job = record["job"]
        params = job["parameters"]
        n, h = params["processing_neurons"], params["mechanisms.homeostasis"]
        require(type(n) is int and type(h) is bool, "confirmation coordinates have invalid types")
        key = (n, job["seed"], h, job["intervention"])
        require(key not in indexed, "duplicate confirmation trial")
        indexed[key] = record
    expected = {(n, s, h, g) for n in p["sizes"] for s in p["seeds"] for h in [False, True] for g in SENSORIMOTOR_GROUPS}
    require(set(indexed) == expected, "confirmation lacks the complete paired design")
    for (n, seed, h, group), r in indexed.items():
        m = r["metrics"]
        require(all(type(v) in [int, float] and math.isfinite(v) for v in m.values()), "nonfinite confirmation metrics")
        require(all(k in m for k in ["visual_accuracy", "position_mae", "prediction_accuracy", "parameters_preserved",
            "formed", "pruned", "homeostasis_updates", "weight_updates", "action_copy_arrivals", "observed_events",
            "total_neurons", "processing_neurons", "initial_synapses", "final_synapses", "core_rate_hz"]), "missing confirmation metrics")
        require(0 <= m["visual_accuracy"] <= 1 and 0 <= m["prediction_accuracy"] <= 1 and 0 <= m["position_mae"] <= 6,
                "confirmation metric outside range")
        require(r["full_replay_checked"] is True and m["parameters_preserved"] == 1, "confirmation lacks verified frozen measurement")
        require(m["processing_neurons"] == n and m["total_neurons"] == n+11, "confirmation size differs")
        require(0 <= m["observed_events"] <= base.get("max_observed_events", 2000000) and m["core_rate_hz"] >= 0
                and m["total_neurons"] <= base.get("max_total_neurons", 100000), "confirmation budget exceeded")
        require(m["formed"] == m["pruned"] == 0 and m["initial_synapses"] == m["final_synapses"], "development changed structure")
        frozen = group.endswith("_frozen")
        require(r["job"]["effective_config"]["development_enabled"] is False, "development switch differs")
        require(r["job"]["effective_config"]["homeostasis_enabled"] is (h and not frozen), "homeostasis switch differs")
        require(r["job"]["effective_config"]["learning_enabled"] is (not frozen), "STDP switch differs")
        if frozen:
            require(m["weight_updates"] == m["homeostasis_updates"] == 0, "frozen control changed parameters")
        elif h:
            require(m["homeostasis_updates"] > 0, "enabled homeostasis produced no updates")
        else:
            require(m["homeostasis_updates"] == 0, "disabled homeostasis produced updates")
        require(m["action_copy_arrivals"] == 0 if group.startswith("passive") else m["action_copy_arrivals"] > 0,
                "feedback condition differs")
        for probe in r["details"]["probes"]:
            require(probe["forecast_us"] < probe["effect_us"], "future leakage in confirmation")
    cells = []
    for index, n in enumerate(p["sizes"]):
        for seed in p["seeds"]:
            rows = [indexed[n, seed, h, g] for h in [False, True] for g in SENSORIMOTOR_GROUPS]
            require(len({r["details"]["initial_network_digest"] for r in rows}) == 1, "unpaired initial networks")
            require(all(experience(r["details"]) == experience(rows[0]["details"]) for r in rows), "unpaired sensory experience")
            for group in ["active_frozen", "passive_frozen", "decoupled_frozen"]:
                a, b = (indexed[n, seed, h, group]["metrics"] for h in [False, True])
                require(all(a[k] == b[k] for k in ["visual_accuracy", "position_mae", "prediction_accuracy", "core_rate_hz"]),
                        "matched frozen control endpoints differ")
        off = [indexed[n, s, False, "active"]["metrics"] for s in p["seeds"]]
        on = [indexed[n, s, True, "active"]["metrics"] for s in p["seeds"]]
        visual = endpoint([a["visual_accuracy"]-b["visual_accuracy"] for a,b in zip(off,on)], False, p["per_test_alpha"], 20261105+2*index)
        mae = endpoint([b["position_mae"]-a["position_mae"] for a,b in zip(off,on)], True, p["per_test_alpha"], 20261106+2*index)
        cells.append(dict(processing_neurons=n, seeds=p["seeds"],
            means={name:dict(visual_accuracy=statistics.mean(m["visual_accuracy"] for m in values),
                position_mae=statistics.mean(m["position_mae"] for m in values),
                prediction_accuracy=statistics.mean(m["prediction_accuracy"] for m in values)) for name,values in [("off",off),("on",on)]},
            visual_loss=visual, external_forecast_error_change=mae,
            both_active_readouts_pass_half_accuracy=sum(a["visual_accuracy"] >= .5 and b["visual_accuracy"] >= .5 for a,b in zip(off,on)),
            per_seed=[dict(seed=s,off=a,on=b) for s,a,b in zip(p["seeds"],off,on)]))
    return dict(protocol=p, technical_checks_passed=True, trials=len(records), cells=cells,
                limits=["Confirmation is specific to these fixed assay endpoints and sizes, not general cognition or new tasks.",
                        f'All {len(p["seeds"])} seeds remain in the tests, including functionally weak or silent networks.',
                        "External forecast error combines representation, visual decoder and pre-effect activity; poor calibration prevents an isolated neural-prediction claim.",
                        "Other feedback groups are matched controls and descriptive evidence, not additional confirmatory hypotheses."])
