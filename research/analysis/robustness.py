"""Paired parameter stress tests; no unsupported noise/task interventions."""
from contracts import ExperimentPlan, AnalysisReport
from evolution.spec import fields
from planning.experiment_designer import axes
from .common import LIMITS, contrast


class Robustness:
    def __init__(self, options):
        fields(options, ["axes", "metric"])
        self.axes = axes(options.get("axes"))
        self.metric = options.get("metric", "retained_sequence_score")

    def propose(self, context):
        points = [dict(context.base)]
        perturbations = []
        for axis in self.axes:
            for value in axis["values"]:
                points.append(dict(context.base, **{axis["name"]: value}))
                perturbations.append(dict(parameter=axis["name"], value=value))
        return ExperimentPlan("robustness", points, dict(perturbations=perturbations, metric=self.metric))

    def analyze(self, results):
        baseline = results[0]
        tests = [dict(candidate=r["id"], parameters=r["parameters"],
                      eligible=r["evaluation"]["eligible"], checks=r["evaluation"]["checks"],
                      delta=contrast(r, baseline, self.metric)) for r in results[1:]]
        return AnalysisReport("robustness", dict(metric=self.metric, baseline=baseline["id"],
            baseline_eligible=baseline["evaluation"]["eligible"], tests=tests,
            all_stress_gates_passed=all(r["eligible"] for r in tests)), LIMITS+[
                "Only declared parameter perturbations are tested, each on a freshly constructed network.",
                "No injected noise, new sequences/tasks, interference, or continuing-plasticity forgetting test."])
