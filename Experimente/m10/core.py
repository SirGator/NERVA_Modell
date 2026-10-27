"""Read-only M10 measurements and independent event/decoder validation."""
import copy
import math
from collections import Counter

ARMS = ["before", "aged_frozen", "paired", "scrambled"]
MAPS = ["normal_mapping", "reversed_mapping"]
STAGES = ["checkpoint_probe", "rest_checkpoint_probe"]
INTERVENTIONS = ["restore_core", "restore_all_plastic"]


def require(ok, message):
    if not ok:
        raise ValueError(message)


def equal(a, b):
    if isinstance(a, bool) or isinstance(b, bool):
        return type(a) is type(b) and a == b
    if isinstance(a, (int, float)) and isinstance(b, (int, float)):
        return a == b if isinstance(a, int) and isinstance(b, int) else math.isclose(a, b, rel_tol=8 * 2**-52, abs_tol=8 * 2**-52)
    if isinstance(a, dict) and isinstance(b, dict):
        return a.keys() == b.keys() and all(equal(a[k], b[k]) for k in a)
    if isinstance(a, list) and isinstance(b, list):
        return len(a) == len(b) and all(equal(x, y) for x, y in zip(a, b))
    return a == b


def without_trace(result):
    result = copy.deepcopy(result)
    for arm in ARMS:
        for mapping in MAPS:
            for stage in STAGES:
                for trials in result["arms"][arm][mapping]["weight_reset_probes"]["stages"][stage].values():
                    for trial in trials:
                        trial.pop("trace", None)
    return result


def l1(a, b):
    return sum(abs(x-y) for x, y in zip(a, b))


def linf(a, b):
    return max(abs(x-y) for x, y in zip(a, b))


def audit_voltage(phase, result):
    """Independent neutral-LIF reconstruction, allowing f32 accumulation error."""
    tolerance = 2e-5
    tau = result["effective_parameters"]["core_membrane_tau_us"]
    threshold = result["effective_parameters"]["core_threshold"]
    require(len(phase["refractory_until_before"]) == 32 and len(phase["refractory_until_after"]) == 32,
            "invalid trace refractory state")
    inputs_by_neuron, spikes_by_neuron = [{} for _ in range(32)], [set() for _ in range(32)]
    for event in phase["events"]:
        if event["kind"] in ["arrival", "external"] and 2 <= event["target"] < 34:
            inputs_by_neuron[event["target"]-2].setdefault(event["time_us"], []).append(event["amplitude"])
        if event["kind"] == "spike" and 2 <= event["source"] < 34:
            spikes_by_neuron[event["source"]-2].add(event["time_us"])
    for index in range(32):
        inputs, spikes = inputs_by_neuron[index], spikes_by_neuron[index]
        value, time = phase["potentials_before"][index], phase["before_us"]
        refractory = phase["refractory_until_before"][index]
        for when in sorted(set(inputs) | spikes):
            value *= math.exp(-(when-time)/tau)
            if when >= refractory:
                value += sum(inputs.get(when, []))
                if when in spikes:
                    require(value >= threshold-tolerance, "trace spike below reconstructed threshold")
                    value, refractory = 0, when+5000
                else:
                    require(value < threshold+tolerance, "trace missing reconstructed spike")
            else:
                require(when not in spikes, "trace spike during refractory interval")
            time = when
        value *= math.exp(-(phase["end_us"]-time)/tau)
        require(math.isclose(value, phase["potentials_after"][index], rel_tol=tolerance, abs_tol=tolerance),
                "trace voltage differs from impulses/leak/refractory reconstruction")
        require(refractory == phase["refractory_until_after"][index], "trace refractory state differs from spikes")


def margin(decoder, counts, label):
    features = [(x-m)/s for x, m, s in zip(counts, decoder["mean"], decoder["scale"])]
    distance = {y: sum((x-c)**2 for x, c in zip(features, decoder["centers"][str(y)])) for y in [-1, 1]}
    return distance[-label] - distance[label]


def measure(result, common_time=False):
    """One row per frozen weight comparison; stored voltages are explicitly named."""
    rows, actions, weights = [], [], []
    edges = {e["id"]: e for e in result["initial_synapses"]}
    for arm in ARMS:
        normal = dict(result["arms"][arm][MAPS[0]]["trained_weights"])
        reversed_weights = dict(result["arms"][arm][MAPS[1]]["trained_weights"])
        weights.append(dict(case=result["case_id"], seed=result["seed"], arm=arm,
                            core_mapping_weight_l1=sum(abs(normal[i]-reversed_weights[i]) for i, e in edges.items()
                                                       if 2 <= e["pre"] < 34 and 2 <= e["post"] < 34)))
        for mapping in MAPS:
            model = result["arms"][arm][mapping]
            decoder = model["weight_reset_probes"]["count_readout"]
            for stage in STAGES:
                probes = model["weight_reset_probes"]["stages"][stage]
                a, d = probes["sham"]
                actions.append(dict(case=result["case_id"], seed=result["seed"], arm=arm, mapping=mapping, stage=stage,
                                    action_counts_l1=l1(a["core_counts"], d["core_counts"]),
                                    same_predicted_class=a["prediction_count"] == d["prediction_count"]))
                for intervention in INTERVENTIONS:
                    for index, (sham, reset) in enumerate(zip(probes["sham"], probes[intervention])):
                        label = model[stage]["labels"][index]
                        row = dict(case=result["case_id"], seed=result["seed"], arm=arm, mapping=mapping,
                                   stage=stage, key=sham["key"], intervention=intervention,
                                   changed_weights=len(reset["changed_weights"]),
                                   count_l1=l1(sham["core_counts"], reset["core_counts"]),
                                   events_changed=sham["core_events"] != reset["core_events"],
                                   prediction_changed=sham["prediction_count"] != reset["prediction_count"],
                                   stored_state_voltage_linf=linf(sham["core_potentials"], reset["core_potentials"]),
                                   sham_margin=margin(decoder, sham["core_counts"], label),
                                   reset_margin=margin(decoder, reset["core_counts"], label))
                        if common_time:
                            row["forecast_voltage_linf"] = linf(sham["trace"][-1]["potentials_after"], reset["trace"][-1]["potentials_after"])
                            row["signed_input_l1"] = sum(l1(a["signed_input_per_core"], b["signed_input_per_core"]) for a, b in zip(sham["trace"], reset["trace"]))
                            row["absolute_input_l1"] = sum(l1(a["absolute_input_per_core"], b["absolute_input_per_core"]) for a, b in zip(sham["trace"], reset["trace"]))
                        rows.append(row)
    return rows, actions, weights


def audit_trace(wrapper, reference):
    require(wrapper["protocol"] == "NERVA-M10-v1" and wrapper["reference_reproduced"] is True, "wrong M10 replay contract")
    result = wrapper["m9_result"]
    require(equal(without_trace(result), reference), "M10 replay differs from source M9")
    edges = {e["id"]: e for e in result["initial_synapses"]}
    polarities = {n["id"]: n["polarity"] for n in result["initial_neurons"]}
    phases, events_count = 0, 0
    for arm in ARMS:
        for mapping in MAPS:
            for stage in STAGES:
                for trials in result["arms"][arm][mapping]["weight_reset_probes"]["stages"][stage].values():
                    for q in trials:
                        require(len(q["trace"]) == 4, "trace phase coverage differs")
                        start = q["intervention_us"]
                        action_events, action_counts = [], [0] * 32
                        previous_voltage = None
                        for index, (phase, duration) in enumerate(zip(q["trace"], [50000, 10000, 10000, 30000])):
                            phases += 1
                            require(phase["start_us"] == start and phase["before_us"] == start-1 and phase["end_us"] == start+duration-1, "trace clock differs")
                            for field in ["potentials_before", "potentials_after", "signed_input_per_core", "absolute_input_per_core"]:
                                require(len(phase[field]) == 32 and all(math.isfinite(v) for v in phase[field]), "invalid trace vector")
                            require(len(phase["arrivals_per_core"]) == 32 and len(phase["core_counts"]) == 32, "invalid trace counters")
                            if previous_voltage is not None:
                                require(phase["potentials_before"] == previous_voltage, "trace voltage discontinuity between phases")
                            previous_voltage = phase["potentials_after"]
                            counts, arrivals, signed, absolute = [0]*32, [0]*32, [0.0]*32, [0.0]*32
                            spikes = []
                            copy_arrivals, previous = 0, start
                            for event in phase["events"]:
                                events_count += 1
                                time = event["time_us"]
                                require(previous <= time <= phase["end_us"], "trace event outside phase or unordered")
                                previous = time
                                if event["kind"] == "arrival":
                                    edge = edges[event["synapse"]]
                                    require((edge["pre"], edge["post"]) == (event["source"], event["target"]), "trace edge endpoints differ")
                                    amplitude = event["amplitude"]
                                    require(math.isfinite(amplitude) and (amplitude >= 0 if polarities[event["source"]] == "Excitatory" else amplitude <= 0), "trace amplitude/polarity differs")
                                    if 2 <= event["target"] < 34:
                                        n = event["target"]-2
                                        arrivals[n] += 1; signed[n] += amplitude; absolute[n] += abs(amplitude)
                                        copy_arrivals += event["source"] in [36, 37]
                                elif event["kind"] == "spike":
                                    require(event["source"] in polarities, "unknown trace spike")
                                    if 2 <= event["source"] < 34:
                                        counts[event["source"]-2] += 1
                                        spikes.append([event["source"], time])
                                elif event["kind"] == "external":
                                    require(event["target"] in polarities and math.isfinite(event["amplitude"]), "unknown trace input")
                                else:
                                    raise ValueError("unexpected/plastic event in frozen trace")
                            require(counts == phase["core_counts"] and arrivals == phase["arrivals_per_core"], "trace counts differ from events")
                            require(equal(signed, phase["signed_input_per_core"]) and equal(absolute, phase["absolute_input_per_core"]), "trace input sums differ from events")
                            require(copy_arrivals == phase["copy_arrivals"], "trace copy arrivals differ")
                            audit_voltage(phase, result)
                            if index:
                                action_events.extend(spikes)
                                action_counts = [a+b for a, b in zip(action_counts, counts)]
                            start += duration
                        require(action_events == q["core_events"] and action_counts == q["core_counts"], "trace differs from original action features")
    require(phases == 384, "trace branch count differs")
    return dict(phases=phases, events=events_count, reference_reproduced=True)
