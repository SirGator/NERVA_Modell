"""Strict harness configuration. Methods never receive held-out seeds."""
from evolution.spec import fields, flatten, number, require
from .experiment_designer import identifier, integer
from .profiles import profile


class ResearchSpec:
    def __init__(self, source):
        fields(source, ["research", "base", "studies", "requirements", "validation"])
        r = source.get("research", {})
        fields(r, ["name", "scenario", "training_seeds", "validation_seeds", "design_seed", "max_trials"])
        require(identifier(r.get("name")), "invalid research name")
        self.name = r["name"]
        self.scenario = r.get("scenario", "m0_selection")
        p = profile(self.scenario)
        self.design_seed = integer(r.get("design_seed", 42), 0, 2**64-1, "design_seed")
        self.max_trials = integer(r.get("max_trials", 10000), 1, 100000, "max_trials")

        def seeds(key):
            values = r.get(key)
            require(isinstance(values, list) and 1 <= len(values) <= 256, "invalid "+key)
            for seed in values:
                integer(seed, 0, 2**64-1, key)
            require(len(set(values)) == len(values), "duplicate "+key)
            return values

        self.training = seeds("training_seeds")
        self.validation = seeds("validation_seeds")
        require(not set(self.training) & set(self.validation), "training/validation seeds must be disjoint")
        self.base = flatten(source.get("base", {}))
        if self.scenario == "m0_selection":
            self.base.setdefault("retention_delay_us", 2000000)
        self.studies = source.get("studies")
        require(isinstance(self.studies, list) and 1 <= len(self.studies) <= 32, "studies must contain 1..32 entries")
        names = set()
        for study in self.studies:
            fields(study, ["id", "method", "priority", "options"])
            require(identifier(study.get("id")) and study["id"] not in names, "study IDs must be valid and unique")
            names.add(study["id"])
            require(isinstance(study.get("method"), str), "study method required")
            integer(study.get("priority", 0), 0, 1000, "priority")
        self.requirements = dict(p.requirements)
        fields(source.get("requirements", {}), self.requirements)
        self.requirements.update(source.get("requirements", {}))
        require(all(number(v) and v >= 0 for v in self.requirements.values()), "invalid hard requirements")
        if self.scenario == "m0_selection":
            require(self.requirements["max_probe_rate_hz"] > 0 and self.requirements["max_event_count"] > 0, "invalid hard requirements")
        else:
            require(0 <= self.requirements["min_visual_accuracy"] <= 1
                    and all(self.requirements[k] > 0 for k in ["max_core_rate_hz", "max_observed_events", "max_synapses"]),
                    "invalid hard requirements")
        self.validation_policy = dict(bootstrap_samples=4000, alpha=.05, min_quality_gain=0., max_resource_ratio=1.25)
        fields(source.get("validation", {}), self.validation_policy)
        self.validation_policy.update(source.get("validation", {}))
        v = self.validation_policy
        integer(v["bootstrap_samples"], 100, 100000, "bootstrap_samples")
        require(all(number(v[k]) for k in ["alpha", "min_quality_gain", "max_resource_ratio"])
                and 0 < v["alpha"] < .5 and v["min_quality_gain"] >= 0 and v["max_resource_ratio"] >= 1,
                "invalid validation policy")
        self.source = source
