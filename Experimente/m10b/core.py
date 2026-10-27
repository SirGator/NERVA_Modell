"""Fixed visual-axis diagnosis; no decoder fitting and no neural simulation."""
import math

EPSILON = 1e-12


def require(ok, message):
    if not ok:
        raise ValueError(message)


def analyze_pair(decoder, trials, labels):
    require(len(trials) == 2 and [q["key"] for q in trials] == ["a", "d"], "expected A/D pair")
    require(sorted(labels) == [-1, 1], "pair must contain both effect labels")
    mean, scale, centers = decoder["mean"], decoder["scale"], decoder["centers"]
    size = len(mean)
    require(size > 0 and len(scale) == size and set(centers) == {"-1", "1"}, "invalid decoder shape")
    require(all(len(c) == size for c in centers.values()), "invalid visual centroid shape")
    require(all(math.isfinite(x) for v in [mean, scale, *centers.values()] for x in v), "nonfinite decoder")
    require(all(s > 0 for s in scale), "nonpositive decoder scale")
    axis = [p-n for p, n in zip(centers["1"], centers["-1"])]
    midpoint = [(p+n)/2 for p, n in zip(centers["1"], centers["-1"])]
    norm = math.sqrt(sum(x*x for x in axis))
    features, predictions, distances = [], [], []
    for q in trials:
        require(len(q["core_counts"]) == size and all(type(x) is int and x >= 0 for x in q["core_counts"]), "invalid action counts")
        z = [(x-m)/s for x, m, s in zip(q["core_counts"], mean, scale)]
        distance = {label: sum((x-c)**2 for x, c in zip(z, centers[str(label)])) for label in [-1, 1]}
        prediction = min([-1, 1], key=lambda label: distance[label])
        require(prediction == q["prediction_count"], "original prediction differs from fixed decoder")
        features.append(z); predictions.append(prediction); distances.append(distance)
    raw = sum(p == label for p, label in zip(predictions, labels))/2
    same = predictions[0] == predictions[1]
    if norm == 0:
        return dict(raw_accuracy=raw, pair_rank_accuracy=.5, effect_axis_cosine=None,
                    score_a=0.0, score_d=0.0, common_bias=0.0, signed_effect_contrast=0.0,
                    same_class=same, aligned_but_same_class=False, correct_direction=False,
                    wrong_direction=False, projection_tie=True, axis_degenerate=True,
                    bias_dominates=True, decision_tolerance=EPSILON)
    unit = [x/norm for x in axis]
    scores = [sum((x-m)*u for x, m, u in zip(z, midpoint, unit)) for z in features]
    for score, distance in zip(scores, distances):
        require(math.isclose(distance[-1]-distance[1], 2*norm*score, rel_tol=1e-10, abs_tol=1e-9),
                "visual-axis projection differs from centroid distances")
    bias = sum(scores)/2
    contrast = labels[1]*(scores[1]-scores[0])
    delta = [d-a for a, d in zip(*features)]
    delta_norm = math.sqrt(sum(x*x for x in delta))
    tolerance = EPSILON*max(1, abs(scores[0]), abs(scores[1]), delta_norm)
    correct, wrong = contrast > tolerance, contrast < -tolerance
    cosine = max(-1.0, min(1.0, contrast/delta_norm)) if delta_norm else None
    return dict(raw_accuracy=raw, pair_rank_accuracy=1.0 if correct else 0.0 if wrong else .5,
                effect_axis_cosine=cosine, score_a=scores[0], score_d=scores[1],
                common_bias=bias, signed_effect_contrast=contrast, same_class=same,
                aligned_but_same_class=correct and same, correct_direction=correct,
                wrong_direction=wrong, projection_tie=not (correct or wrong), axis_degenerate=False,
                bias_dominates=abs(bias) >= abs(scores[1]-scores[0])/2-tolerance,
                decision_tolerance=tolerance)


def measure(result):
    rows = []
    for arm in ["before", "aged_frozen", "paired", "scrambled"]:
        for mapping in ["normal_mapping", "reversed_mapping"]:
            model = result["arms"][arm][mapping]
            for stage in ["checkpoint_probe", "rest_checkpoint_probe"]:
                for intervention in ["sham", "restore_core", "restore_all_plastic"]:
                    pair = model["weight_reset_probes"]["stages"][stage][intervention]
                    values = analyze_pair(model["weight_reset_probes"]["count_readout"], pair, model[stage]["labels"])
                    if intervention == "sham":
                        require(values["raw_accuracy"] == model[stage]["count"]["balanced_accuracy"], "raw accuracy differs from M9")
                    rows.append(dict(case=result["case_id"], seed=result["seed"], arm=arm,
                                     mapping=mapping, stage=stage, intervention=intervention, **values))
    return rows
