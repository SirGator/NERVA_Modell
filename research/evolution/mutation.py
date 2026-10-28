import hashlib
import json
import struct
from .spec import FLOAT_PARAMETERS


def canonical_json(value):return json.dumps(value, sort_keys=True, separators=(",", ":"), allow_nan=False)


def normalize(genome):
    return {name: struct.unpack("f", struct.pack("f", float(value)))[0] if name in FLOAT_PARAMETERS else value for name, value in genome.items()}


class SplitMix64:
    def __init__(self, seed):self.state = seed
    def next(self):
        self.state = (self.state+0x9e3779b97f4a7c15) & (2**64-1)
        value = self.state
        value = ((value ^ (value >> 30))*0xbf58476d1ce4e5b9) & (2**64-1)
        value = ((value ^ (value >> 27))*0x94d049bb133111eb) & (2**64-1)
        return value ^ (value >> 31)


def children(parent, spec, generation):
    alternatives, rejected, seen = [], [], set()
    for mutation in spec.mutations:
        name = mutation["name"]
        for choice in mutation.get("values", mutation.get("factors")):
            value = parent["parameters"][name]*choice if "factors" in mutation else choice
            change = dict(name=name, operator="multiply" if "factors" in mutation else "set", argument=choice)
            if "min" in mutation and not mutation["min"] <= value <= mutation["max"]:
                rejected.append(dict(mutation=change, status="out_of_bounds"));continue
            genome = dict(parent["parameters"]); genome[name] = value
            try:genome = normalize(genome)
            except (OverflowError, struct.error):
                rejected.append(dict(mutation=change, status="numeric_overflow"));continue
            if "min" in mutation and not normalize({name: mutation["min"]})[name] <= genome[name] <= normalize({name: mutation["max"]})[name]:
                rejected.append(dict(mutation=change, status="rounded_out_of_bounds"));continue
            key = canonical_json(genome)
            if genome == parent["parameters"] or key in seen:
                rejected.append(dict(mutation=change, status="unchanged_or_duplicate"));continue
            seen.add(key); alternatives.append(dict(parameters=genome, mutation=change))
    alternatives.sort(key=lambda row: canonical_json(row["parameters"]))
    seed = int.from_bytes(hashlib.sha256(f"{spec.mutation_seed}:{generation}:{parent['id']}".encode()).digest()[:8], "big")
    rng = SplitMix64(seed)
    for i in range(len(alternatives)-1, 0, -1):
        j = rng.next() % (i+1); alternatives[i], alternatives[j] = alternatives[j], alternatives[i]
    return alternatives[:spec.children], rejected
