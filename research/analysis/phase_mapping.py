"""Two-dimensional parameter maps with explicit per-seed state labels."""
from contracts import ExperimentPlan, AnalysisReport
from evolution.spec import fields, number, require
from planning.experiment_designer import axes, factorial
from .common import LIMITS, mean


def state(metrics, max_rate):
    if metrics["silent_fraction"] >= 1 or metrics["firing_rate"] == 0:
        return "collapse"
    if metrics["persistent_fraction"] > 0 or metrics["firing_rate"] > max_rate:
        return "overactive"
    if metrics["stability"] != 1:
        return "unstable"
    return "stable"


class PhaseMapping:
    def __init__(self, options):
        fields(options, ["axes", "max_rate_hz"])
        self.axes = axes(options.get("axes"), dimensions=2)
        self.max_rate = options.get("max_rate_hz", 200.)
        require(number(self.max_rate) and self.max_rate > 0, "max_rate_hz must be positive")

    def propose(self, context):
        return ExperimentPlan("phase_mapping", factorial(context.base, self.axes),
                              dict(axes=self.axes, max_rate_hz=self.max_rate))

    def analyze(self, results):
        cells = []
        for result in results:
            states = {str(r["seed"]): state(r["metrics"], self.max_rate)
                      for r in result["observations"] if r["intervention"] == "none"}
            labels = set(states.values())
            cells.append(dict(candidate=result["id"], coordinates=[result["parameters"][a["name"]] for a in self.axes],
                state=next(iter(labels)) if len(labels) == 1 else "mixed", seed_states=states,
                sequence_score=mean(result, "sequence_score"), firing_rate=mean(result, "firing_rate"),
                eligible=result["evaluation"]["eligible"]))
        return AnalysisReport("phase_mapping", dict(axes=self.axes, cells=cells), LIMITS+[
            "State labels are operational thresholds at sampled points, not proven dynamical phases or boundaries."])
