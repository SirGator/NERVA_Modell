import math

FLOAT_PARAMETERS = {
    "learning.a_plus", "learning.a_minus", "learning.tau_plus_us", "learning.tau_minus_us",
    "learning.homeostasis.target_rate_hz", "learning.homeostasis.target_input_rate",
    "learning.homeostasis.intrinsic_adjustment_rate", "learning.homeostasis.structural_adjustment_rate",
    "neuron.membrane_tau_us", "neuron.threshold",
}
INTEGER_PARAMETERS = {"learning.stdp_window_us", "learning.homeostasis.update_interval_us", "neuron.refractory_period_us"}
BOOL_PARAMETERS = {"mechanisms.stdp", "mechanisms.homeostasis"}


def require(ok, message):
    if not ok:raise ValueError(message)


def fields(value, allowed):
    require(isinstance(value, dict), "expected a table")
    require(not (set(value)-set(allowed)), "unknown selection fields: "+str(sorted(set(value)-set(allowed))))


def number(value):
    return type(value) in [int, float] and math.isfinite(value)


def flatten(value, prefix=""):
    require(isinstance(value, dict), "base must be a table")
    result = {}
    for name, item in value.items():
        key = prefix+name
        if isinstance(item, dict):
            nested = flatten(item, key+".")
        else:nested = {key: item}
        require(not result.keys() & nested.keys(), "duplicate flattened base parameter")
        result.update(nested)
    return result


class SelectionSpec:
    def __init__(self, source):
        fields(source, ["selection", "base", "mutations", "requirements", "validation"])
        s = source.get("selection", {})
        fields(s, ["name", "scenario", "generations", "elite_size", "children_per_parent", "max_candidates", "mutation_seed", "training_seeds", "validation_seeds", "retention_delay_us"])
        self.source = source
        self.name = s.get("name")
        require(isinstance(self.name, str) and 1 <= len(self.name) <= 80 and all(c.isascii() and (c.isalnum() or c in "-_") for c in self.name), "invalid selection name")
        require(s.get("scenario", "m0_selection") == "m0_selection", "selection currently requires the m0_selection adapter")
        def integer(name, default, low, high):
            n = s.get(name, default)
            require(type(n) is int and low <= n <= high, f"{name} must be {low}..{high}")
            return n
        self.generations = integer("generations", 4, 1, 100)
        self.elite_size = integer("elite_size", 2, 1, 32)
        self.children = integer("children_per_parent", 4, 1, 64)
        self.max_candidates = integer("max_candidates", 64, 1, 1000)
        require(self.elite_size <= self.max_candidates, "elite_size exceeds candidate budget")
        self.mutation_seed = integer("mutation_seed", 42, 0, 2**64-1)
        self.delay = integer("retention_delay_us", 2_000_000, 1, 10_000_000)
        def seeds(name):
            values = s.get(name)
            require(isinstance(values, list) and values and len(values) <= 256 and all(type(x) is int and 0 <= x < 2**64 for x in values), "invalid "+name)
            require(len(set(values)) == len(values), "duplicate "+name)
            return values
        self.training = seeds("training_seeds")
        self.validation = seeds("validation_seeds")
        require(not set(self.training) & set(self.validation), "training/validation seeds must be disjoint")
        self.base = flatten(source.get("base", {}))
        require("retention_delay_us" not in self.base, "retention delay belongs to the fixed selection protocol")
        self.base["retention_delay_us"] = self.delay
        self.mutations = source.get("mutations", [])
        require(isinstance(self.mutations, list) and 1 <= len(self.mutations) <= 32, "mutations must contain 1..32 axes")
        names = set()
        for mutation in self.mutations:
            fields(mutation, ["name", "factors", "values", "min", "max"])
            name = mutation.get("name")
            require(name in FLOAT_PARAMETERS | INTEGER_PARAMETERS | BOOL_PARAMETERS, "only S1 parameters and the two existing mechanism flags may mutate")
            require(name not in names and name in self.base, "each mutation needs a unique axis and an explicit baseline value")
            names.add(name)
            if name in BOOL_PARAMETERS:
                require(type(self.base[name]) is bool, "mechanism baseline must be boolean")
                require(set(mutation) == {"name", "values"} and isinstance(mutation["values"], list) and mutation["values"]
                        and all(type(v) is bool for v in mutation["values"]), "mechanism mutations require boolean choices")
            else:
                require(number(self.base[name]), "numeric mutation baseline required")
                require(number(mutation.get("min")) and number(mutation.get("max")) and 0 <= mutation["min"] <= mutation["max"], "finite nonnegative mutation bounds required")
                require(mutation["min"] <= self.base[name] <= mutation["max"], "baseline outside mutation bounds")
                require(("factors" in mutation) != ("values" in mutation), "choose mutation factors or values")
                choices = mutation.get("factors", mutation.get("values"))
                require(isinstance(choices, list) and choices and len(choices) <= 64 and all(number(x) for x in choices), "finite mutation choices required")
                require("factors" not in mutation or all(x > 0 for x in choices), "mutation factors must be positive")
                if "values" in mutation:require(all(mutation["min"] <= x <= mutation["max"] for x in choices), "mutation value outside bounds")
                if name in INTEGER_PARAMETERS:
                    require(type(self.base[name]) is int and "values" in mutation and all(type(x) is int for x in choices), "integer axes require integer value choices")
            choices = mutation.get("values", mutation.get("factors"))
            require(len({str(v) for v in choices}) == len(choices), "duplicate mutation choices")
        req = source.get("requirements", {})
        fields(req, ["min_retained_score", "max_retention_loss", "max_probe_rate_hz", "max_event_count"])
        self.requirements = dict(min_retained_score=1.0, max_retention_loss=0.0, max_probe_rate_hz=200.0, max_event_count=100_000)
        self.requirements.update(req)
        require(all(number(v) for v in self.requirements.values()), "finite requirements required")
        require(self.requirements["min_retained_score"] >= 0 and self.requirements["max_retention_loss"] >= 0
                and self.requirements["max_probe_rate_hz"] > 0 and self.requirements["max_event_count"] > 0, "invalid hard requirements")
        validation = source.get("validation", {})
        fields(validation, ["bootstrap_samples", "alpha", "min_quality_gain", "max_resource_ratio"])
        self.validation_policy = dict(bootstrap_samples=4000, alpha=.05, min_quality_gain=0.0, max_resource_ratio=1.25)
        self.validation_policy.update(validation)
        v = self.validation_policy
        require(type(v["bootstrap_samples"]) is int and 100 <= v["bootstrap_samples"] <= 100_000, "bootstrap_samples must be 100..100000")
        require(all(number(v[k]) for k in ["alpha", "min_quality_gain", "max_resource_ratio"])
                and 0 < v["alpha"] < .5 and v["min_quality_gain"] >= 0 and v["max_resource_ratio"] >= 1, "invalid validation policy")

    def budget(self):
        maximum = min(self.max_candidates, 1+self.children+(self.generations-1)*self.elite_size*self.children)
        jobs = maximum*len(self.training)*4+2*len(self.validation)*4
        return dict(maximum_training_candidates=maximum, maximum_study_jobs=jobs,
                    maximum_native_training_runs=jobs*4, generations=self.generations,
                    note="Each study job includes immediate/retained endpoints and full replay; each native training also has two frozen internal probes")
