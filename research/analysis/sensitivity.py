"""Morris elementary effects on paired seed-mean responses."""
import statistics
from contracts import ExperimentPlan, AnalysisReport
from evolution.spec import fields, require
from planning.experiment_designer import axes, integer, morris
from .common import LIMITS, mean


class Sensitivity:
    def __init__(self, options):
        fields(options, ["axes", "trajectories", "levels", "metric"])
        self.axes = axes(options.get("axes"), bounds=True)
        self.trajectories = integer(options.get("trajectories", 4), 2, 64, "trajectories")
        self.levels = integer(options.get("levels", 4), 4, 20, "levels")
        require(self.levels % 2 == 0, "Morris levels must be even")
        self.metric = options.get("metric", "sequence_score")

    def propose(self, context):
        points, edges = morris(context.base, self.axes, self.trajectories, self.levels, context.design_seed)
        self.plan = ExperimentPlan("sensitivity", points, dict(edges=edges, metric=self.metric,
                                    axes=self.axes, levels=self.levels, trajectories=self.trajectories))
        return self.plan

    def analyze(self, results):
        effects = {a["name"]: [] for a in self.axes}
        bounds = {a["name"]: a["max"]-a["min"] for a in self.axes}
        for edge in self.plan.design["edges"]:
            before, after = results[edge["before"]], results[edge["after"]]
            name = edge["axis"]
            delta = (after["parameters"][name]-before["parameters"][name])/bounds[name]
            require(delta != 0, "Morris step collapsed at native precision")
            effects[name].append((mean(after, self.metric)-mean(before, self.metric))/delta)
        summary = {name: dict(mu=statistics.mean(v), mu_star=statistics.mean(abs(x) for x in v),
                              sigma=statistics.stdev(v), elementary_effects=v, trajectories=len(v))
                   for name, v in effects.items()}
        return AnalysisReport("sensitivity", dict(metric=self.metric, parameters=summary,
            ranking=sorted(summary, key=lambda n: (-summary[n]["mu_star"], n))), LIMITS+[
                "Effects use normalized parameter ranges and paired seed means; sigma mixes interactions and nonlinearity.",
                "No Sobol indices or confidence intervals are estimated."])
