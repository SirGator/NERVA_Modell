"""Paired 2x2 interventions on the existing runtime mechanism switches."""
import statistics
from contracts import ExperimentPlan, AnalysisReport
from evolution.spec import fields
from planning.experiment_designer import factorial
from .common import LIMITS, contrast


class Causal:
    def __init__(self, options):
        fields(options, ["metric"])
        self.metric = options.get("metric", "sequence_score")

    def propose(self, context):
        self.axes = [dict(name="mechanisms.stdp", values=[False, True]),
                     dict(name="mechanisms.homeostasis", values=[False, True])]
        return ExperimentPlan("causal", factorial(context.base, self.axes), dict(axes=self.axes, metric=self.metric))

    def analyze(self, results):
        off, homeo, stdp, both = results
        comparisons = dict(stdp_without_homeostasis=contrast(stdp, off, self.metric),
            stdp_with_homeostasis=contrast(both, homeo, self.metric),
            homeostasis_without_stdp=contrast(homeo, off, self.metric),
            homeostasis_with_stdp=contrast(both, stdp, self.metric))
        interaction = [a-b for a, b in zip(comparisons["stdp_with_homeostasis"]["per_seed"],
                                          comparisons["stdp_without_homeostasis"]["per_seed"])]
        return AnalysisReport("causal", dict(metric=self.metric, contrasts=comparisons,
            interaction=dict(mean=statistics.mean(interaction), per_seed=interaction),
            candidates=[r["id"] for r in results]), LIMITS+[
                "Paired controlled total effects include downstream mechanism changes; no isolated biological mechanism claim.",
                "Contrasts are descriptive; confirmation requires a preregistered contrast on fresh seeds."])
