"""Crossed frozen-weight/recurrence audit, without fitting or generating spikes."""
import copy
import math
from collections import Counter

WEIGHTS = ["trained", "reset_core"]
GATES = ["sham", "block_exc", "block_inh", "block_both"]


def require(ok, message):
    if not ok:raise ValueError(message)


def without_crossed(result):
    result = copy.deepcopy(result)
    for arm in ["before", "aged_frozen", "paired", "scrambled"]:
        for mapping in ["normal_mapping", "reversed_mapping"]:
            result["arms"][arm][mapping].pop("crossed_probes", None)
    return result


def audit_window(q, result, blocked, m10, m10c):
    edges = {e["id"]: e for e in result["initial_synapses"]}
    polarities = {n["id"]: n["polarity"] for n in result["initial_neurons"]}
    trace = q["trace"]
    require(len(trace) == 4, "phase coverage differs")
    start, events, copies = q["intervention_us"], 0, 0
    core_events = []
    for index, (phase, duration) in enumerate(zip(trace, [50000, 10000, 10000, 30000])):
        require(phase["start_us"] == start and phase["before_us"] == start-1
                and phase["end_us"] == start+duration-1, "phase clock differs")
        if index:
            for before, after in [("potentials_before", "potentials_after"), ("refractory_until_before", "refractory_until_after")]:
                require(phase[before] == trace[index-1][after], "phase state discontinuity")
        for field in ["potentials_before", "potentials_after", "signed_input_per_core", "absolute_input_per_core"]:
            require(len(phase[field]) == 32 and all(math.isfinite(x) for x in phase[field]), "invalid phase vector")
        spikes, arrivals, signed, absolute = Counter(), Counter(), [0.0]*32, [0.0]*32
        phase_copies, previous = 0, start
        for event in phase["events"]:
            events += 1
            when = event["time_us"]
            require(previous <= when <= phase["end_us"], "event time/order differs")
            previous = when
            if event["kind"] == "arrival":
                edge = edges[event["synapse"]]
                require((edge["pre"], edge["post"]) == (event["source"], event["target"]), "arrival endpoints differ")
                require(not index or event["synapse"] not in blocked, "blocked arrival escaped")
                value = event["amplitude"]
                require(math.isfinite(value) and (value >= 0 if polarities[event["source"]] == "Excitatory" else value <= 0), "arrival polarity differs")
                if 2 <= event["target"] < 34:
                    n = event["target"]-2
                    arrivals[n] += 1; signed[n] += value; absolute[n] += abs(value)
                    phase_copies += event["source"] in [36, 37]
            elif event["kind"] == "spike":
                require(event["source"] in polarities, "unknown spike")
                if 2 <= event["source"] < 34:
                    spikes[event["source"]-2] += 1
                    if index:core_events.append([event["source"], when])
            elif event["kind"] == "external":
                require(event["target"] in polarities and math.isfinite(event["amplitude"]), "invalid external input")
            else:raise ValueError("unexpected/plastic trace event")
        require([spikes[i] for i in range(32)] == phase["core_counts"]
                and [arrivals[i] for i in range(32)] == phase["arrivals_per_core"], "phase counts differ")
        require(m10.equal(signed, phase["signed_input_per_core"]) and m10.equal(absolute, phase["absolute_input_per_core"]), "phase input sums differ")
        require(phase_copies == phase["copy_arrivals"], "phase copy count differs")
        if index:copies += phase_copies
        m10.audit_voltage(phase, result)
        start += duration
    require(q["core_events"] == core_events and q["copy_arrivals"] == copies == 9, "action events/copies differ")
    require(q["action_us"] == trace[1]["start_us"] and q["forecast_us"] == trace[-1]["end_us"], "action clock differs")
    reconstructed = m10c.reconstruct(q, result)
    return events, reconstructed["error_linf"]


def validate(wrapper, reference, previous, d):
    m10, m10c, m10d, analyze = d.m10.core, d.m10c, d.m10d.core, d.m10b.analyze_pair
    require(wrapper["protocol"] == "NERVA-M10E-v1" and wrapper["reference_reproduced"] is True
            and wrapper["sham_reproduced"] is True, "wrong M10-E contract")
    result = wrapper["m9_result"]
    require(wrapper["seed"] == reference["seed"] and wrapper["case_id"] == reference["case_id"], "source identity differs")
    require(m10.equal(without_crossed(result), reference), "parent differs from original M9")
    require(previous["protocol"] == "NERVA-M10D-v1" and previous["seed"] == reference["seed"], "M10-D identity differs")
    require(m10.equal(m10d.without_recurrence(previous["m9_result"]), reference), "M10-D source parent differs")
    windows, events, cancelled, controls, maximum = 0, 0, 0, 0, 0.0
    initial = result["initial_synapses"]
    variants = {f"{w}__{g}" for w in WEIGHTS for g in GATES}
    for arm in m10.ARMS:
        for mapping in m10.MAPS:
            model = result["arms"][arm][mapping]
            probes = model["crossed_probes"]
            trained = dict(model["trained_weights"])
            expected_edits = [[e["id"], trained[e["id"]], e["weight"]] for e in initial
                              if 2 <= e["pre"] < 34 and 2 <= e["post"] < 34 and trained[e["id"]] != e["weight"]]
            require(m10.equal(probes["count_readout"], model["weight_reset_probes"]["count_readout"]), "decoder differs")
            for stage in m10.STAGES:
                branches = probes["stages"][stage]
                require(set(branches) == variants, "crossed coverage differs")
                common = branches["trained__sham"]
                for weight in WEIGHTS:
                    for gate in GATES:
                        pair = branches[f"{weight}__{gate}"]
                        analyze(probes["count_readout"], pair, model[stage]["labels"])
                        for index, q in enumerate(pair):
                            windows += 1
                            edits = expected_edits if weight == "reset_core" else []
                            require(m10.equal(q["changed_weights"], edits), "Core reset edits differ")
                            meta = q["weight_intervention"]
                            require(meta["mode"] == weight and meta["activation_us"] == q["action_us"], "weight reset time/mode differs")
                            require(all(meta[k] is True for k in ["baseline_unmodified", "only_core_weights_changed", "neurons_and_time_preserved", "pending_unblocked_arrivals_keep_emission_amplitudes"]), "weight intervention contract failed")
                            for flag in ["effect_not_presented", "exact_replay_state_before_rest", "neurons_preserved_at_intervention", "frozen_parameters_preserved", "parent_unchanged"]:
                                require(q[flag] is True, "frozen branch contract failed")
                            require(q["trace"][0] == common[index]["trace"][0], "baseline differs")
                            for field in ["potentials_before", "refractory_until_before"]:
                                require(q["trace"][1][field] == common[index]["trace"][1][field], "action start state differs")
                            blocked = m10d.selected(result, gate)
                            g = q["recurrence_gate"]
                            require(g["blocked_synapses"] == blocked and g["activation_us"] == q["action_us"], "gate selection/time differs")
                            require(type(g["cancelled_arrivals"]) is int and g["cancelled_arrivals"] >= 0
                                    and (blocked or g["cancelled_arrivals"] == 0), "invalid cancellation count")
                            require(all(g[k] is True for k in ["only_enabled_flags_changed", "weights_and_cells_preserved", "baseline_unmodified"]), "gate contract failed")
                            cancelled += g["cancelled_arrivals"]
                            if weight == "trained":
                                old = previous["m9_result"]["arms"][arm][mapping]["recurrence_probes"]["stages"][stage][gate][index]
                                require(m10.equal({k: v for k, v in q.items() if k != "weight_intervention"}, old), "trained branch differs from M10-D")
                            elif all(e[0] in blocked for e in edits):
                                controls += 1
                                require(q["trace"] == branches[f"trained__{gate}"][index]["trace"], "blocked-weight negative control differs")
                            count, error = audit_window(q, result, blocked, m10, m10c)
                            events += count; maximum = max(maximum, error)
    require(windows == 256, "window coverage differs")
    return dict(windows=windows, phases=windows*4, events=events, cancelled_arrivals=cancelled,
                maximum_voltage_error=maximum, negative_control_windows=controls,
                original_parent_reproduced=True, all_trained_branches_reproduce_m10d=True)


def measure(result, d):
    pairs, windows = [], []
    for arm in d.m10.core.ARMS:
        for mapping in d.m10.core.MAPS:
            model = result["arms"][arm][mapping]
            decoder = model["crossed_probes"]["count_readout"]
            for stage in d.m10.core.STAGES:
                branches = model["crossed_probes"]["stages"][stage]
                for gate in GATES:
                    for weight in WEIGHTS:
                        pair = branches[f"{weight}__{gate}"]
                        values = d.m10b.analyze_pair(decoder, pair, model[stage]["labels"])
                        pairs.append(dict(seed=result["seed"], arm=arm, mapping=mapping, stage=stage, gate=gate, weight=weight,
                            **values, mean_spikes=sum(sum(q["core_counts"]) for q in pair)/2,
                            mean_active_cells=sum(sum(x > 0 for x in q["core_counts"]) for q in pair)/2,
                            correct_margin=sum(d.m10.core.margin(decoder, q["core_counts"], label)
                                               for q, label in zip(pair, model[stage]["labels"]))/2))
                    for a, b in zip(branches[f"trained__{gate}"], branches[f"reset_core__{gate}"]):
                        windows.append(dict(seed=result["seed"], arm=arm, mapping=mapping, stage=stage, gate=gate, key=a["key"],
                            count_l1=d.m10.core.l1(a["core_counts"], b["core_counts"]),
                            events_changed=a["core_events"] != b["core_events"],
                            prediction_changed=a["prediction_count"] != b["prediction_count"],
                            voltage_linf=d.m10.core.linf(a["trace"][-1]["potentials_after"], b["trace"][-1]["potentials_after"])))
    return pairs, windows
