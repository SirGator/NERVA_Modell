"""Independent audit and paired tests for frozen M9 weight counterfactuals."""
import csv
import json
import math
from collections import Counter
from pathlib import Path

INTERVENTIONS = ["sham", "restore_core", "restore_all_plastic"]


def validate(result, require, path=None):
    edges = {edge["id"]: edge for edge in result["initial_synapses"]}
    core = set(range(2, 34))
    propagation = {}
    if path is not None:
        with Path(str(path) + ".propagation.csv").open(newline="") as stream:
            for row in csv.DictReader(stream):
                if row["stage"] in ["checkpoint_probe", "rest_checkpoint_probe"]:
                    propagation[row["arm"], row["mapping"] + "_mapping", row["stage"], row["key"]] = json.loads(row["core_events"])
    branches = 0
    for arm in ["before", "aged_frozen", "paired", "scrambled"]:
        for mapping in ["normal_mapping", "reversed_mapping"]:
            model = result["arms"][arm][mapping]
            probes = model["weight_reset_probes"]
            decoder = probes["count_readout"]
            require(set(decoder) == {"mean", "scale", "centers"}, "invalid reset decoder")
            require(set(decoder["centers"]) == {"-1", "1"}, "reset decoder classes differ")
            vectors = [decoder["mean"], decoder["scale"], *decoder["centers"].values()]
            require(all(len(v) == 32 and all(type(x) in (int, float) and math.isfinite(x) for x in v) for v in vectors), "invalid reset decoder vector")
            require(all(x > 0 for x in decoder["scale"]), "invalid reset decoder scale")
            require(set(probes["stages"]) == {"checkpoint_probe", "rest_checkpoint_probe"}, "reset stages differ")
            trained = dict(model["trained_weights"])
            for stage, rest in [("checkpoint_probe", 0), ("rest_checkpoint_probe", 2000000)]:
                conditions = probes["stages"][stage]
                require(set(conditions) == set(INTERVENTIONS), "reset interventions differ")
                for intervention in INTERVENTIONS:
                    trials = conditions[intervention]
                    require(len(trials) == 2 and [q["key"] for q in trials] == ["a", "d"], "reset action coverage differs")
                    expected = []
                    for ident, edge in edges.items():
                        selected = (intervention == "restore_core" and edge["pre"] in core and edge["post"] in core) or (intervention == "restore_all_plastic" and edge["plastic"])
                        if selected and trained[ident] != edge["weight"]:
                            expected.append([ident, trained[ident], edge["weight"]])
                    for index, q in enumerate(trials):
                        branches += 1
                        require(q["checkpoint_us"] == model["checkpoint_us"] and q["rest_us"] == rest, "reset checkpoint differs")
                        require(q["intervention_us"] == model["checkpoint_us"] + rest and q["action_us"] == q["intervention_us"] + 50000 and q["forecast_us"] == q["action_us"] + 49999, "reset clock differs")
                        require(all(q[key] is True for key in ["exact_replay_state_before_rest", "neurons_preserved_at_intervention", "frozen_parameters_preserved", "parent_unchanged", "effect_not_presented"]), "reset state invariant failed")
                        require(q["copy_arrivals"] == 9, "reset copy arrivals differ")
                        require(q["changed_weights"] == expected, "reset weight edits differ from trained/initial weights")
                        require(all(n in core and q["action_us"] <= t <= q["forecast_us"] for n, t in q["core_events"]), "invalid reset event")
                        counts = Counter(n for n, _ in q["core_events"])
                        require(q["core_counts"] == [counts[n] for n in range(2, 34)], "reset counts differ from events")
                        require(len(q["core_potentials"]) == 32 and all(math.isfinite(v) for v in q["core_potentials"]), "invalid reset potentials")
                        features = [(x - mean) / scale for x, mean, scale in zip(q["core_counts"], decoder["mean"], decoder["scale"])]
                        prediction = min([-1, 1], key=lambda label: sum((x - center) ** 2 for x, center in zip(features, decoder["centers"][str(label)])))
                        require(q["prediction_count"] == prediction, "reset prediction differs from decoder/events")
                        if intervention == "sham":
                            require(prediction == model[stage]["predictions_count"][index], "reset sham differs from primary prediction")
                            if path is not None:
                                require(q["core_events"] == propagation[arm, mapping, stage, q["key"]], "reset sham differs from primary events")
                        if not expected:
                            sham = conditions["sham"][index]
                            require(all(q[key] == sham[key] for key in ["core_events", "core_counts", "core_potentials", "prediction_count"]), "unchanged reset differs from sham")
    require(branches == 96, "reset branch count differs")
    return branches


def accuracy(result, arm, mapping, stage, intervention):
    model = result["arms"][arm][mapping]
    trials = model["weight_reset_probes"]["stages"][stage][intervention]
    return sum(q["prediction_count"] == label for q, label in zip(trials, model[stage]["labels"])) / 2


def contrasts(name, values, bootstrap, sign_p, alpha):
    rows, passed = [], {stage: len(values) >= 20 for stage in ["checkpoint_probe", "rest_checkpoint_probe"]}
    for stage in passed:
        for mapping in ["normal_mapping", "reversed_mapping"]:
            for arm in ["before", "aged_frozen", "paired", "scrambled"]:
                sham = [accuracy(r, arm, mapping, stage, "sham") for r in values]
                for intervention in INTERVENTIONS[1:]:
                    reset = [accuracy(r, arm, mapping, stage, intervention) for r in values]
                    estimate = bootstrap(sham, reset)
                    p = sign_p([a - b for a, b in zip(sham, reset)])
                    required = arm == "paired" and intervention == "restore_all_plastic"
                    if required:
                        passed[stage] = passed[stage] and estimate["ci95"][0] > 0 and p <= alpha
                    rows.append(dict(case=name, stage=stage, mapping=mapping, arm=arm,
                                     intervention=intervention, seeds=len(values), sham=sum(sham) / len(sham),
                                     reset=sum(reset) / len(reset), delta=estimate["mean_difference"],
                                     ci_low=estimate["ci95"][0], ci_high=estimate["ci95"][1],
                                     one_sided_sign_p=p, family_alpha=alpha if required else "",
                                     required_for_weight_causality=required))
    return passed, rows
