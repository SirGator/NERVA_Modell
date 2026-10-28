"""Hard gates precede Pareto ranking; no weighted universal fitness."""
import math


def dominates(a, b):
    return a["quality"] >= b["quality"] and a["resource"] <= b["resource"] and (a["quality"] > b["quality"] or a["resource"] < b["resource"])


def distance(a, b, mutations):
    terms = []
    for m in mutations:
        x, y = a["parameters"][m["name"]], b["parameters"][m["name"]]
        terms.append(float(x != y) if type(x) is bool else abs(x-y)/max(m["max"]-m["min"], 1e-12))
    return math.sqrt(sum(t*t for t in terms))


def select(candidates, size, mutations):
    remaining = [dict(c, **c["evaluation"]) for c in candidates if c["evaluation"]["eligible"]]
    selected, fronts, rank = [], {}, 0
    while remaining and len(selected) < size:
        front = [c for c in remaining if not any(dominates(other, c) for other in remaining if other["id"] != c["id"])]
        for c in front:fronts[c["id"]] = rank
        available = list(front)
        while available and len(selected) < size:
            if not selected:choice = min(available, key=lambda c: (-c["quality"], c["resource"], c["id"]))
            else:choice = min(available, key=lambda c: (-min(distance(c, s, mutations) for s in selected), -c["quality"], c["resource"], c["id"]))
            selected.append(choice); available.remove(choice)
        front_ids = {c["id"] for c in front}
        remaining = [c for c in remaining if c["id"] not in front_ids]; rank += 1
    return [c["id"] for c in selected], fronts
