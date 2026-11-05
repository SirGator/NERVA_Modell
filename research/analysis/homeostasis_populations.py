"""Fixed adjacent population contrasts with observed input/current diagnostics."""
import math
import statistics

from analysis.homeostasis_confirmation import endpoint, experience
from evolution.spec import flatten, require
from planning.profiles import SENSORIMOTOR_GROUPS

SCOPES = ["off", "core", "core_sensor", "all"]
POPULATIONS = ["sensor", "core", "motor", "copy"]
CONTRASTS = [("core_minus_off", "off", "core"),
             ("sensor_added", "core", "core_sensor"),
             ("motor_copy_added", "core_sensor", "all")]
DIAGNOSTICS = ["sensor_rate_hz", "root_sensor_rate_hz", "core_rate_hz",
               "training_sensor_rate_hz", "training_core_rate_hz",
               "measurement_sensor_rate_hz", "measurement_core_rate_hz"]
DIAGNOSTICS += [p+"_intrinsic_current" for p in POPULATIONS]
DIAGNOSTICS += [p+"_homeostasis_updates" for p in POPULATIONS]


def protocol(source):
    e = source.get("experiment", {})
    name = e.get("name")
    require(isinstance(name, str) and 1 <= len(name) <= 80
            and all(c.isascii() and (c.isalnum() or c in "-_") for c in name), "invalid population study name")
    require(e.get("scenario") == "sensorimotor_size" and e.get("verify_replay") is True,
            "population study requires sensorimotor_size with full replay")
    require(e.get("interventions") == SENSORIMOTOR_GROUPS, "population study needs all six controls")
    seeds = e.get("seeds")
    require(isinstance(seeds, list) and 1 <= len(seeds) <= 256
            and all(type(s) is int and 0 <= s < 2**64 for s in seeds)
            and len(set(seeds)) == len(seeds), "invalid population study seeds")
    axes = source.get("parameters", [])
    require(len(axes) == 2 and {a["name"] for a in axes} == {"processing_neurons", "homeostasis.scope"},
            "population study needs exactly size and scope axes")
    values = {a["name"]:a["values"] for a in axes}
    sizes = values["processing_neurons"]
    require(isinstance(sizes, list) and 1 <= len(sizes) <= 256
            and all(type(n) is int and n > 0 for n in sizes) and len(set(sizes)) == len(sizes), "invalid population study sizes")
    require(values["homeostasis.scope"] == SCOPES, "scopes must be off/core/core_sensor/all")
    base = flatten(source.get("base", {}))
    require(base.get("mechanisms.development") is False, "population study requires development off")
    require(not {"mechanisms.homeostasis", "homeostasis.scope", "processing_neurons"} & base.keys(),
            "do not duplicate scope/size switches in base")
    require({"visual_accuracy", "position_mae", *DIAGNOSTICS} <= set(source.get("measurements", {}).get("metrics", [])),
            "missing population study measurements")
    return dict(protocol="NERVA-homeostasis-populations-v1", sizes=sizes, seeds=seeds, scopes=SCOPES,
        primary_group="active", contrasts=[dict(name=n,low=a,high=b) for n,a,b in CONTRASTS],
        primary_endpoints=["visual_accuracy", "position_mae"], alpha=.05,
        family_tests=6*len(sizes), per_test_alpha=.05/(6*len(sizes)),
        tests="Two-sided exact conservative sign tests over seeds for all adjacent contrasts and both endpoints; ties stay in n. Sample mean must agree with sign direction. Strict p < alpha/(6*K).",
        differences="High scope minus low scope. Negative visual-accuracy difference is loss; positive MAE difference is worse.",
        bootstrap="4000 deterministic percentile bootstrap draws of paired seed means; unadjusted 95% intervals are descriptive only.",
        diagnostics="Root-generated spikes and actual sensor-neuron spikes are separate; rates per neuron/channel and second. Intrinsic currents are stored homeostatic model currents after training. Population update counts audit scope membership.",
        technical_checks="Complete controls/replay, frozen measurement, no development, exact scope membership, identical initial nets/frames/timing/root-spike counts per observation window, identical frozen endpoints. No seed exclusion for poor readouts.",
        limits="Conditional total intervention effects of adding populations; not isolated mediation, equivalence or general cognition. External MAE includes visual decoding quality.")


def root_signature(record):
    d = record["details"]
    return [(w["phase"], w["start_us"], w["end_us"], w["root_sensor_spikes"])
            for w in d["training_curve"]+d["measurement_windows"]]


def analyze(records, source):
    p = protocol(source)
    base = flatten(source["base"])
    indexed = {}
    for r in records:
        job = r["job"]; n = job["parameters"]["processing_neurons"]; scope = job["parameters"]["homeostasis.scope"]
        require(type(n) is int and isinstance(scope, str), "invalid population coordinates")
        key = n,job["seed"],scope,job["intervention"]
        require(key not in indexed, "duplicate population trial")
        indexed[key] = r
    require(set(indexed) == {(n,s,h,g) for n in p["sizes"] for s in p["seeds"] for h in SCOPES for g in SENSORIMOTOR_GROUPS},
            "population study lacks complete paired design")
    for (n,seed,scope,group),r in indexed.items():
        m = r["metrics"]; config = r["job"]["effective_config"]
        needed = {"visual_accuracy", "position_mae", "prediction_accuracy", "parameters_preserved", "formed", "pruned",
                  "initial_synapses", "final_synapses", "homeostasis_updates", "weight_updates", "action_copy_arrivals",
                  "processing_neurons", "total_neurons", "observed_events", *DIAGNOSTICS}
        require(needed <= m.keys() and all(type(v) in [int,float] and math.isfinite(v) for v in m.values()), "invalid population metrics")
        require(0 <= m["visual_accuracy"] <= 1 and 0 <= m["prediction_accuracy"] <= 1 and 0 <= m["position_mae"] <= 6, "invalid endpoint ranges")
        require(m["processing_neurons"] == n and m["total_neurons"] == n+11, "population size mismatch")
        require(r["full_replay_checked"] is True and m["parameters_preserved"] == 1, "missing replay/frozen checks")
        require(config["development_enabled"] is False and config["homeostasis_scope"] == scope, "scope configuration differs")
        frozen = group.endswith("_frozen")
        require(config["learning_enabled"] is (not frozen)
                and config["homeostasis_enabled"] is (not frozen and scope != "off"), "population switches differ")
        require(m["formed"] == m["pruned"] == 0 and m["initial_synapses"] == m["final_synapses"], "structure changed")
        require(0 <= m["observed_events"] <= base.get("max_observed_events",2000000)
                and m["total_neurons"] <= base.get("max_total_neurons",100000), "population budget exceeded")
        require(all(m[k] >= 0 for k in DIAGNOSTICS if not k.endswith("intrinsic_current")), "negative population count/rate")
        allowed = {"off":[False]*4,"core":[False,True,False,False],"core_sensor":[True,True,False,False],"all":[True]*4}[scope]
        if frozen: allowed = [False]*4
        currents = r["details"]["intrinsic_currents"]
        require(currents["trained"] == currents["final"], "measurement changed intrinsic currents")
        for i,pop in enumerate(POPULATIONS):
            c = currents["trained"][pop]; initial = currents["initial"][pop]
            require(c["count"] == [7,n,2,2][i] and initial["count"] == c["count"], "population count differs")
            require(initial["mean"] == initial["min"] == initial["max"] == 0, "initial currents differ")
            require(all(type(c[k]) in [int,float] and math.isfinite(c[k]) for k in ["mean","min","max"])
                    and -10 <= c["min"] <= c["mean"] <= c["max"] <= 20, "invalid intrinsic current")
            require(m[pop+"_intrinsic_current"] == c["mean"], "current summary differs")
            updates = m[pop+"_homeostasis_updates"]
            require(updates > 0 if allowed[i] else updates == 0, "updates outside requested populations or missing intervention")
            if not allowed[i]: require(c["min"] == c["max"] == 0, "excluded population current changed")
        require(sum(m[pop+"_homeostasis_updates"] for pop in POPULATIONS) == m["homeostasis_updates"], "update count mismatch")
        if frozen: require(m["weight_updates"] == 0, "frozen weights changed")
        require(m["action_copy_arrivals"] == 0 if group.startswith("passive") else m["action_copy_arrivals"] > 0, "feedback mismatch")
        require(all(q["forecast_us"] < q["effect_us"] for q in r["details"]["probes"]), "future leakage")
    cells = []
    for i,n in enumerate(p["sizes"]):
        for seed in p["seeds"]:
            rows = [indexed[n,seed,h,g] for h in SCOPES for g in SENSORIMOTOR_GROUPS]
            require(len({r["details"]["initial_network_digest"] for r in rows}) == 1, "unpaired initial networks")
            require(all(experience(r["details"]) == experience(rows[0]["details"]) for r in rows), "unpaired experience")
            require(all(root_signature(r) == root_signature(rows[0]) for r in rows), "root-generated inputs differ")
            for group in ["active_frozen","passive_frozen","decoupled_frozen"]:
                ms = [indexed[n,seed,h,group]["metrics"] for h in SCOPES]
                require(all(all(m[k] == ms[0][k] for k in ["visual_accuracy","position_mae","prediction_accuracy","sensor_rate_hz","core_rate_hz"]) for m in ms),
                        "frozen scope controls differ")
        active = {h:[indexed[n,s,h,"active"]["metrics"] for s in p["seeds"]] for h in SCOPES}
        summaries = {h:{k:statistics.mean(m[k] for m in rows) for k in ["visual_accuracy","position_mae","prediction_accuracy",*DIAGNOSTICS]}
                     for h,rows in active.items()}
        contrasts = []
        for j,(name,low,high) in enumerate(CONTRASTS):
            endpoints = {metric:endpoint([b[metric]-a[metric] for a,b in zip(active[low],active[high])],True,
                p["per_test_alpha"],20261106+6*i+2*j+k) for k,metric in enumerate(p["primary_endpoints"])}
            contrasts.append(dict(name=name,low=low,high=high,endpoints=endpoints,
                both_visual_readouts_pass_half_accuracy=sum(a["visual_accuracy"] >= .5 and b["visual_accuracy"] >= .5 for a,b in zip(active[low],active[high]))))
        cells.append(dict(processing_neurons=n,seeds=p["seeds"],means=summaries,contrasts=contrasts,
            per_seed=[dict(seed=s,scopes={h:active[h][k] for h in SCOPES}) for k,s in enumerate(p["seeds"])]))
    return dict(protocol=p,technical_checks_passed=True,trials=len(records),cells=cells,
        limits=["Confirmations refer to fixed external assay endpoints, not general cognition or an isolated latent prediction mechanism.",
                "Sensor addition is conditional on regulated core; motor/copy addition is conditional on regulated core and sensor cells.",
                "Rate/current diagnostics are descriptive and do not prove mediation or distinguish sensor intrinsic-current changes from downstream STDP effects.",
                "No functional readout gate excludes seeds; non-confirmation does not establish equivalence."])
