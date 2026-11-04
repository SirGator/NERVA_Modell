import statistics
from evolution.spec import require


def values(result, metric, group="none"):
    rows = result["observations"]
    selected = [r for r in rows if r["intervention"] == group]
    require(selected and all(metric in r["metrics"] for r in selected), "missing analysis metric: "+metric)
    return [r["metrics"][metric] for r in selected]


def mean(result, metric, group="none"):
    return statistics.mean(values(result, metric, group))


def contrast(left, right, metric):
    a = {r["seed"]: r["metrics"][metric] for r in left["observations"] if r["intervention"] == "none"}
    b = {r["seed"]: r["metrics"][metric] for r in right["observations"] if r["intervention"] == "none"}
    require(a.keys() == b.keys() and a, "contrast requires paired seeds")
    differences = [a[s]-b[s] for s in sorted(a)]
    return dict(mean=statistics.mean(differences), per_seed=differences,
                seeds=sorted(a), min=min(differences), max=max(differences))


LIMITS = ["Exploratory M0 results on the declared seeds; no independent task evidence.",
          "Retention uses frozen weights after quiet delay; ongoing plasticity and interference are untested."]
