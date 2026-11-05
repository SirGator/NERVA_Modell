"""Paired size/feedback comparisons, with distinct measured endpoints."""
import statistics
from contracts import ExperimentPlan, AnalysisReport
from evolution.spec import fields, require
from planning.profiles import SENSORIMOTOR_GROUPS, SENSORIMOTOR_LIMITS


def sizes(options):
    values=options.get("sizes",[256,1024])
    require(isinstance(values,list) and 1<=len(values)<=256
            and all(type(s) is int and s>0 for s in values)
            and len(set(values))==len(values), "sizes must be unique positive integers (1..256 entries)")
    return values


class SizeEnvironment:
    def __init__(self, options):
        fields(options, ["sizes"])
        self.sizes = sizes(options)

    def propose(self, context):
        return ExperimentPlan("size_environment", [dict(context.base, processing_neurons=s) for s in self.sizes],
                              dict(sizes=self.sizes, metric="prediction_accuracy"))

    def analyze(self, results):
        cells = []
        for result in results:
            indexed = {(r["seed"], r["intervention"]): r["metrics"] for r in result["observations"]}
            seeds = sorted({s for s, _ in indexed})
            require(set(indexed) == {(s, g) for s in seeds for g in SENSORIMOTOR_GROUPS}, "missing sensorimotor observations")
            metrics = ["prediction_accuracy", "position_mae", "persistence_mae", "majority_accuracy", "visual_accuracy",
                       "core_rate_hz", "peak_core_rate_hz", "silent_fraction", "observed_events", "total_neurons",
                       "initial_synapses", "final_synapses", "maximum_synapses", "core_inhibitory_fraction",
                       "weight_updates", "homeostasis_updates", "formed", "pruned", "action_copy_arrivals"]
            groups = {g: {m: statistics.mean(indexed[s, g][m] for s in seeds) for m in metrics} for g in SENSORIMOTOR_GROUPS}
            gains = {g: [indexed[s, g]["prediction_accuracy"]-indexed[s, g+"_frozen"]["prediction_accuracy"]
                         for s in seeds] for g in ["active", "passive", "decoupled"]}
            cells.append(dict(candidate=result["id"], processing_neurons=result["parameters"]["processing_neurons"],
                parameters=result["parameters"],
                seeds=seeds, groups=groups, eligible=result["evaluation"]["eligible"],
                checks=result["evaluation"]["checks"],
                learning_gains={g: dict(mean=statistics.mean(v), per_seed=v) for g, v in gains.items()},
                active_minus_passive_gain=[a-b for a,b in zip(gains["active"], gains["passive"])],
                active_minus_decoupled_gain=[a-b for a,b in zip(gains["active"], gains["decoupled"])],
                per_seed=[dict(seed=s, groups={g:indexed[s,g] for g in SENSORIMOTOR_GROUPS}) for s in seeds]))
        return AnalysisReport("size_environment", dict(cells=cells), SENSORIMOTOR_LIMITS + [
            "Rate bounds operationalize activity stability; they do not prove dynamical stability.",
            "Learning arms use their declared growth/homeostasis switches; no-learning controls disable all plasticity.",
            "Fixed sparse wiring rules are reused; neuron size also changes spatial geometry and E/I rounding."])
