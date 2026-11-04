"""Finite-domain MAP-Elites with archive-parent, single-axis mutations."""
import math
from contracts import ExperimentPlan, AnalysisReport
from evolution.mutation import SplitMix64
from evolution.spec import fields, number, require
from planning.experiment_designer import axes, factorial, integer
from .common import LIMITS, mean


class QualityDiversity:
    def __init__(self, options):
        fields(options, ["axes", "descriptors", "max_evaluations"])
        self.axes = axes(options.get("axes"))
        self.descriptors = options.get("descriptors", [
            dict(metric="firing_rate", min=0., max=200., bins=4),
            dict(metric="retained_sequence_score", min=-3., max=3., bins=4)])
        require(isinstance(self.descriptors, list) and len(self.descriptors) == 2, "MAP-Elites needs two descriptors")
        for d in self.descriptors:
            fields(d, ["metric", "min", "max", "bins"])
            require(isinstance(d.get("metric"), str) and number(d.get("min"))
                    and number(d.get("max")) and d["min"] < d["max"], "invalid descriptor range")
            integer(d.get("bins"), 1, 64, "descriptor bins")
        require(self.descriptors[0]["metric"] != self.descriptors[1]["metric"], "duplicate descriptor")
        self.limit = integer(options.get("max_evaluations", 32), 1, 4096, "max_evaluations")

    def propose(self, context):
        self.rng = SplitMix64(context.design_seed)
        points = factorial(context.base, self.axes)
        self.points = points
        self.limit = min(self.limit, len(points))
        return ExperimentPlan("quality_diversity", points,
                              dict(axes=self.axes, descriptors=self.descriptors, max_evaluations=self.limit))

    def archive(self, results):
        archive, excluded = {}, []
        for result in results:
            if not result["evaluation"]["eligible"]:
                excluded.append(dict(candidate=result["id"], reason="hard_requirements"))
                continue
            descriptors = [mean(result, d["metric"]) for d in self.descriptors]
            if any(not d["min"] <= v <= d["max"] for d, v in zip(self.descriptors, descriptors)):
                excluded.append(dict(candidate=result["id"], reason="descriptor_out_of_range"))
                continue
            cell = tuple(min(d["bins"]-1, math.floor((v-d["min"])/(d["max"]-d["min"])*d["bins"]))
                         for d, v in zip(self.descriptors, descriptors))
            key = lambda r: (-r["evaluation"]["quality"], r["evaluation"]["resource"], r["id"])
            if cell not in archive or key(result) < key(archive[cell]):
                archive[cell] = result
        return archive, excluded

    def next_index(self, results, remaining):
        """Choose an archive parent, then an untested one-axis neighbor."""
        archive, _ = self.archive(results)
        choices = sorted(remaining)
        if archive:
            parent = archive[sorted(archive)[self.rng.next() % len(archive)]]
            neighbors = [i for i in choices if sum(self.points[i][a["name"]] != parent["parameters"][a["name"]]
                                                  for a in self.axes) == 1]
            if neighbors:
                choices = neighbors
        return choices[self.rng.next() % len(choices)]

    def analyze(self, results):
        archive, excluded = self.archive(results)
        cells = [dict(cell=list(cell), candidate=r["id"], quality=r["evaluation"]["quality"],
                      resource=r["evaluation"]["resource"],
                      descriptors=[mean(r, d["metric"]) for d in self.descriptors])
                 for cell, r in sorted(archive.items())]
        return AnalysisReport("quality_diversity", dict(descriptors=self.descriptors, cells=cells,
            coverage=len(cells)/math.prod(d["bins"] for d in self.descriptors),
            evaluations=len(results), excluded=excluded), LIMITS+[
                "Finite configured parameter domain; parents are archive elites and mutations change one axis.",
                "Empty archive or exhausted local neighbors trigger uniform sampling of untested configurations."])
