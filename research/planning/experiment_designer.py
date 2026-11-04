"""Bounded designs using only Python's standard library."""
import itertools
from evolution.mutation import SplitMix64
from evolution.spec import fields, number, require


def identifier(value):
    return (isinstance(value, str) and 1 <= len(value) <= 80
            and all(c.isascii() and (c.isalnum() or c in "-_") for c in value))


def integer(value, low, high, label):
    require(type(value) is int and low <= value <= high, f"{label} must be {low}..{high}")
    return value


def axes(source, *, bounds=False, dimensions=None):
    require(isinstance(source, list) and 1 <= len(source) <= 32, "axes must contain 1..32 entries")
    if dimensions is not None:
        require(len(source) == dimensions, f"expected {dimensions} axes")
    names = set()
    for axis in source:
        fields(axis, ["name", "min", "max"] if bounds else ["name", "values"])
        name = axis.get("name")
        require(isinstance(name, str) and name and name not in names, "axes need unique parameter names")
        names.add(name)
        if bounds:
            require(number(axis.get("min")) and number(axis.get("max"))
                    and axis["min"] < axis["max"], "finite increasing bounds required")
        else:
            values = axis.get("values")
            require(isinstance(values, list) and 1 <= len(values) <= 256
                    and all(type(v) is bool or number(v) for v in values), "finite axis values required")
            require(len({(type(v), v) for v in values}) == len(values), "duplicate axis values")
    return source


def factorial(base, dimensions, limit=4096):
    size = 1
    for axis in dimensions:
        size *= len(axis["values"])
    require(size <= limit, "design exceeds point limit")
    return [dict(base, **dict(zip([a["name"] for a in dimensions], values)))
            for values in itertools.product(*(a["values"] for a in dimensions))]


def shuffle(items, rng):
    for i in range(len(items)-1, 0, -1):
        j = rng.next() % (i+1)
        items[i], items[j] = items[j], items[i]


def morris(base, dimensions, trajectories, levels, seed):
    """Vanilla Morris: random OAT paths, even p, delta=p/(2*(p-1))."""
    rng = SplitMix64(seed)
    jump = levels // 2
    points, edges = [], []
    for trajectory in range(trajectories):
        direction = [1 if rng.next() % 2 else -1 for _ in dimensions]
        x = [rng.next() % (levels-jump) for _ in dimensions]
        x = [v if d > 0 else v+jump for v, d in zip(x, direction)]
        order = list(range(len(dimensions)))
        shuffle(order, rng)

        def point():
            return dict(base, **{a["name"]: a["min"]+(a["max"]-a["min"])*v/(levels-1)
                                 for a, v in zip(dimensions, x)})

        points.append(point())
        for axis_index in order:
            before = len(points)-1
            x[axis_index] += jump*direction[axis_index]
            points.append(point())
            edges.append(dict(trajectory=trajectory, axis=dimensions[axis_index]["name"],
                              before=before, after=len(points)-1))
    return points, edges
