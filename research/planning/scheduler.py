"""Prioritize complete designs; reserve holdout cost before exploration."""
from dataclasses import asdict
from contracts import ExperimentContext
from evolution.spec import require
from analysis.sensitivity import Sensitivity
from analysis.phase_mapping import PhaseMapping
from analysis.causal import Causal
from analysis.robustness import Robustness
from analysis.quality_diversity import QualityDiversity


METHODS = dict(sensitivity=Sensitivity, phase_mapping=PhaseMapping, causal=Causal,
               robustness=Robustness, quality_diversity=QualityDiversity)


def prepare(spec, backend):
    capabilities = backend.capabilities()
    baseline = backend.canonical(spec.base, spec)
    candidates = {baseline["id"]: baseline}
    studies, methods = [], {}
    # A finalist and baseline, each with all four controls, even if identical.
    reserved = 2*len(spec.validation)*4
    used = len(spec.training)*4  # Baseline is always evaluated.
    require(used+reserved <= spec.max_trials, "budget cannot cover baseline plus held-out confirmation")
    scheduled = {baseline["id"]}
    for study in sorted(spec.studies, key=lambda s: (-s.get("priority", 0), s["id"])):
        require(study["method"] in METHODS, "unknown research method: "+study["method"])
        method = METHODS[study["method"]](study.get("options", {}))
        context = ExperimentContext(study["id"], dict(spec.base), tuple(spec.training), spec.design_seed)
        plan = method.propose(context)
        metrics = [plan.design["metric"]] if "metric" in plan.design else []
        if study["method"] == "quality_diversity":
            metrics += [d["metric"] for d in method.descriptors]
        require(all(isinstance(m, str) and m in capabilities["metrics"] for m in metrics), "unknown analysis metric")
        canonical = [backend.canonical(p, spec) for p in plan.points]
        if study["method"] == "quality_diversity":
            method.points = [c["parameters"] for c in canonical]
        # Validate all candidate configurations, including budget-skipped designs.
        if study["method"] in ["phase_mapping", "quality_diversity"]:
            require(len({c["id"] for c in canonical}) == len(canonical), "duplicate effective design points after native rounding")
        if study["method"] == "sensitivity":
            for edge in plan.design["edges"]:
                require(canonical[edge["before"]]["id"] != canonical[edge["after"]]["id"],
                        "Morris step collapsed at native precision")
        for candidate in canonical:
            cid = candidate["id"]
            if cid in candidates:
                require(candidates[cid]["effective_config"] == candidate["effective_config"], "candidate identity collision")
            else:
                candidates[cid] = candidate
        new_ids = {c["id"] for c in canonical}-scheduled
        # Adaptive MAP-Elites can choose any prevalidated point; charge a safe
        # upper bound without assuming it will hit previously evaluated points.
        if study["method"] == "quality_diversity":
            count = min(len(new_ids), method.limit)
        else:
            count = len(new_ids)
        cost = count*len(spec.training)*4
        accepted = used+reserved+cost <= spec.max_trials
        entry = dict(id=study["id"], priority=study.get("priority", 0), plan=asdict(plan),
                     candidates=canonical, scheduled=accepted, maximum_additional_trials=cost,
                     reason="scheduled" if accepted else "insufficient_budget_for_complete_design")
        studies.append(entry)
        methods[study["id"]] = method
        if accepted:
            used += cost
            # Adaptive studies may not visit their entire candidate universe.
            if study["method"] != "quality_diversity":
                scheduled.update(new_ids)
    return dict(protocol="NERVA-Harness-v1", specification=spec.source, baseline=baseline,
                studies=studies, maximum_training_trials=used, reserved_validation_trials=reserved,
                maximum_trials=used+reserved, maximum_native_training_runs=4*(used+reserved),
                budget_scope="Study jobs, each with two endpoints and full replay; four native training runs per job",
                identity=backend.identity()), methods
