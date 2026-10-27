"""Trajectory accounting on a fixed neuron relevance axis; no new spike generation."""
import math

GROUPS = ["copy", "recurrent_exc", "recurrent_inh", "visual", "other"]
SIGNALS = ["all_input", "accepted_input", "all_leak_input", "accepted_leak_input",
           "linear_drive", "reset_loss", "residual_voltage", "spike_counts"]
EPSILON = 1e-12
VOLTAGE_TOLERANCE = 2e-5


def require(ok, message):
    if not ok:
        raise ValueError(message)


def zero():
    return [0.0]*32


def group(source, polarities):
    if source in [36, 37]:
        return "copy"
    if 2 <= source < 34:
        return "recurrent_exc" if polarities[source] == "Excitatory" else "recurrent_inh"
    return "visual" if source in [0, 1] else "other"


def coefficients(decoder):
    axis = [p-n for p, n in zip(decoder["centers"]["1"], decoder["centers"]["-1"])]
    norm = math.sqrt(sum(x*x for x in axis))
    return [x/(norm*s) for x, s in zip(axis, decoder["scale"])] if norm else zero()


def project(coeff, a, d, effect):
    terms = [effect*w*(y-x) for w, x, y in zip(coeff, a, d)]
    value = sum(terms)
    tolerance = EPSILON*max(1, sum(abs(x) for x in terms))
    return value, 1 if value > tolerance else -1 if value < -tolerance else 0, tolerance


def reconstruct(q, result):
    """Decompose observed neutral-LIF endpoint, conditional on its recorded spikes."""
    phases = q["trace"][1:]
    require(len(phases) == 3, "expected three action phases")
    start, end = phases[0]["before_us"], phases[-1]["end_us"]
    require(start == q["action_us"]-1 and end == q["forecast_us"] and end-start == 50000, "window clock differs")
    tau = result["effective_parameters"]["core_membrane_tau_us"]
    threshold = result["effective_parameters"]["core_threshold"]
    edges = {e["id"]: e for e in result["initial_synapses"]}
    polarities = {n["id"]: n["polarity"] for n in result["initial_neurons"]}
    arrivals, spikes = [{} for _ in range(32)], [set() for _ in range(32)]
    vectors = {signal: zero() for signal in SIGNALS}
    vectors["carry"] = [v*math.exp(-(end-start)/tau) for v in phases[0]["potentials_before"]]
    vectors["discarded_leak"] = zero()
    groups = {g: {s: zero() for s in ["all_input", "all_leak_input", "accepted_leak_input"]} for g in GROUPS}
    by_edge = {}
    for phase in phases:
        for event in phase["events"]:
            when = event["time_us"]
            require(start < when <= end, "event outside action window")
            if event["kind"] == "arrival" and 2 <= event["target"] < 34:
                edge = edges[event["synapse"]]
                require((edge["pre"], edge["post"]) == (event["source"], event["target"]), "edge endpoints differ")
                arrivals[event["target"]-2].setdefault(when, []).append(event)
            elif event["kind"] == "external" and 2 <= event["target"] < 34:
                raise ValueError("unexpected external Core input in fixed M9 topology")
            elif event["kind"] == "spike" and 2 <= event["source"] < 34:
                spikes[event["source"]-2].add(when)
    errors = []
    for index in range(32):
        value, time = phases[0]["potentials_before"][index], start
        refractory = phases[0]["refractory_until_before"][index]
        for when in sorted(set(arrivals[index]) | spikes[index]):
            value *= math.exp(-(when-time)/tau)
            batch = arrivals[index].get(when, [])
            accepted = when >= refractory
            for event in batch:
                amplitude = event["amplitude"]
                leak = amplitude*math.exp(-(end-when)/tau)
                kind = group(event["source"], polarities)
                ident = event["synapse"]
                entry = by_edge.setdefault(ident, dict(all_input=0.0, all_leak_input=0.0, accepted_leak_input=0.0))
                for signal, contribution in [("all_input", amplitude), ("all_leak_input", leak), ("accepted_leak_input", leak if accepted else 0.0)]:
                    vectors[signal][index] += contribution
                    groups[kind][signal][index] += contribution
                    entry[signal] += contribution
                if accepted:
                    vectors["accepted_input"][index] += amplitude
                else:
                    vectors["discarded_leak"][index] += leak
            if accepted:
                value += sum(event["amplitude"] for event in batch)
                if when in spikes[index]:
                    require(value >= threshold-VOLTAGE_TOLERANCE, "recorded spike below reconstructed threshold")
                    vectors["reset_loss"][index] += value*math.exp(-(end-when)/tau)
                    value, refractory = 0, when+5000
                else:
                    require(value < threshold+VOLTAGE_TOLERANCE, "missing recorded spike")
            else:
                require(when not in spikes[index], "recorded spike during refractory interval")
            time = when
        reconstructed = value*math.exp(-(end-time)/tau)
        observed = phases[-1]["potentials_after"][index]
        require(math.isclose(reconstructed, observed, abs_tol=VOLTAGE_TOLERANCE, rel_tol=VOLTAGE_TOLERANCE), "voltage reconstruction differs")
        require(refractory == phases[-1]["refractory_until_after"][index], "refractory reconstruction differs")
        vectors["linear_drive"][index] = vectors["carry"][index]+vectors["accepted_leak_input"][index]
        identity = vectors["linear_drive"][index]-vectors["reset_loss"][index]
        require(math.isclose(identity, reconstructed, abs_tol=1e-10, rel_tol=1e-10), "LIF accounting identity failed")
        require(math.isclose(vectors["all_leak_input"][index]-vectors["discarded_leak"][index], vectors["accepted_leak_input"][index], abs_tol=1e-10, rel_tol=1e-10), "refractory input identity failed")
        vectors["residual_voltage"][index] = observed
        vectors["spike_counts"][index] = len(spikes[index])
        require(vectors["spike_counts"][index] == q["core_counts"][index], "spikes differ from original counts")
        errors.append(abs(reconstructed-observed))
    return dict(vectors=vectors, groups=groups, edges=by_edge,
                error_linf=max(errors), action_before=phases[0]["potentials_before"])


def measure(result, pair_analyzer):
    signals, groups, edges, windows = [], [], [], []
    network = {e["id"]: e for e in result["initial_synapses"]}
    polarities = {n["id"]: n["polarity"] for n in result["initial_neurons"]}
    for arm in ["before", "aged_frozen", "paired", "scrambled"]:
        for mapping in ["normal_mapping", "reversed_mapping"]:
            model = result["arms"][arm][mapping]
            coeff = coefficients(model["weight_reset_probes"]["count_readout"])
            trained = dict(model["trained_weights"])
            for stage in ["checkpoint_probe", "rest_checkpoint_probe"]:
                effect = model[stage]["labels"][1]
                for intervention in ["sham", "restore_core", "restore_all_plastic"]:
                    pair = model["weight_reset_probes"]["stages"][stage][intervention]
                    base = dict(seed=result["seed"], arm=arm, mapping=mapping, stage=stage, intervention=intervention)
                    a, d = [reconstruct(q, result) for q in pair]
                    require(a["action_before"] == d["action_before"], "A/D initial states differ")
                    rank = pair_analyzer(model["weight_reset_probes"]["count_readout"], pair, model[stage]["labels"])
                    for q, reconstructed in zip(pair, [a, d]):
                        windows.append(dict(**base, key=q["key"], voltage_error_linf=reconstructed["error_linf"]))
                    for signal in SIGNALS:
                        value, direction, tolerance = project(coeff, a["vectors"][signal], d["vectors"][signal], effect)
                        if signal == "spike_counts":
                            require(math.isclose(value, rank["signed_effect_contrast"], abs_tol=1e-9, rel_tol=1e-10), "spike projection differs from M10-B")
                            direction = 1 if rank["correct_direction"] else -1 if rank["wrong_direction"] else 0
                        signals.append(dict(**base, signal=signal, contrast=value, direction=direction, tolerance=tolerance))
                    for kind in GROUPS:
                        values = {signal: project(coeff, a["groups"][kind][signal], d["groups"][kind][signal], effect)[0]
                                  for signal in ["all_input", "all_leak_input", "accepted_leak_input"]}
                        groups.append(dict(**base, source_group=kind, **values))
                    probe_weights = {ident: new for ident, old, new in pair[0]["changed_weights"]}
                    for ident in sorted(a["edges"].keys() | d["edges"].keys()):
                        edge = network[ident]
                        values = {signal: effect*coeff[edge["post"]-2]*(d["edges"].get(ident, {}).get(signal, 0)-a["edges"].get(ident, {}).get(signal, 0))
                                  for signal in ["all_input", "all_leak_input", "accepted_leak_input"]}
                        edges.append(dict(**base, synapse=ident, pre=edge["pre"], post=edge["post"],
                                          source_group=group(edge["pre"], polarities),
                                          readout_coefficient=coeff[edge["post"]-2], initial_weight=edge["weight"],
                                          trained_weight=trained[ident], probe_weight=probe_weights.get(ident, trained[ident]), **values))
    return signals, groups, edges, windows
