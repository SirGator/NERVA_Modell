"""Independent frozen recurrence-block validation and fixed-decoder measurements."""
import copy
import math

VARIANTS = ["sham", "block_exc", "block_inh", "block_both"]


def require(ok, message):
    if not ok:raise ValueError(message)


def without_recurrence(result):
    result = copy.deepcopy(result)
    for arm in ["before", "aged_frozen", "paired", "scrambled"]:
        for mapping in ["normal_mapping", "reversed_mapping"]:
            result["arms"][arm][mapping].pop("recurrence_probes", None)
    return result


def selected(result, variant):
    polarities = {n["id"]: n["polarity"] for n in result["initial_neurons"]}
    return [e["id"] for e in result["initial_synapses"]
            if 2 <= e["pre"] < 34 and 2 <= e["post"] < 34
            and (variant == "block_both" or variant == "block_exc" and polarities[e["pre"]] == "Excitatory"
                 or variant == "block_inh" and polarities[e["pre"]] == "Inhibitory")]


def validate(wrapper, reference, m10, m10c, analyze_pair):
    require(wrapper["protocol"] == "NERVA-M10D-v1" and wrapper["reference_reproduced"] is True
            and wrapper["sham_reproduced"] is True, "wrong M10-D contract")
    result = wrapper["m9_result"]
    require(wrapper["seed"] == reference["seed"] and wrapper["case_id"] == reference["case_id"], "source identity differs")
    require(m10.equal(without_recurrence(result), reference), "parent differs from original M9")
    network = {e["id"]: e for e in result["initial_synapses"]}
    polarities = {n["id"]: n["polarity"] for n in result["initial_neurons"]}
    phases, events_total, windows, cancelled = 0, 0, 0, 0
    maximum_error = 0.0
    for arm in m10.ARMS:
        for mapping in m10.MAPS:
            model = result["arms"][arm][mapping]
            probes = model["recurrence_probes"]
            require(m10.equal(probes["count_readout"], model["weight_reset_probes"]["count_readout"]), "decoder differs")
            for stage in m10.STAGES:
                branches = probes["stages"][stage]
                require(set(branches) == set(VARIANTS), "variant coverage differs")
                for variant in VARIANTS:
                    pair = branches[variant]
                    rank = analyze_pair(probes["count_readout"], pair, model[stage]["labels"])
                    if variant == "sham":
                        require(rank["raw_accuracy"] == model[stage]["count"]["balanced_accuracy"], "sham accuracy differs")
                    for index, q in enumerate(pair):
                        windows += 1
                        sham = branches["sham"][index]
                        if variant == "sham":
                            original = {k: v for k, v in q.items() if k not in ["trace", "recurrence_gate"]}
                            require(m10.equal(original, model["weight_reset_probes"]["stages"][stage]["sham"][index]), "sham probe differs")
                        require(q["changed_weights"] == [], "gate changed weights")
                        for flag in ["effect_not_presented", "exact_replay_state_before_rest", "neurons_preserved_at_intervention", "frozen_parameters_preserved", "parent_unchanged"]:
                            require(q[flag] is True, "frozen branch contract failed")
                        gate = q["recurrence_gate"]
                        blocked = selected(result, variant)
                        require(gate["blocked_synapses"] == blocked, "blocked edge selection differs")
                        require(type(gate["cancelled_arrivals"]) is int and gate["cancelled_arrivals"] >= 0, "invalid cancellation count")
                        require(blocked or gate["cancelled_arrivals"] == 0, "sham cancelled arrivals")
                        cancelled += gate["cancelled_arrivals"]
                        require(gate["activation_us"] == q["action_us"] == sham["action_us"], "gate time differs")
                        require(all(gate[k] is True for k in ["only_enabled_flags_changed", "weights_and_cells_preserved", "baseline_unmodified"]), "gate state contract failed")
                        trace = q["trace"]
                        require(len(trace) == 4 and trace[0] == sham["trace"][0], "baseline differs")
                        for field in ["potentials_before", "refractory_until_before"]:
                            require(trace[1][field] == sham["trace"][1][field], "action start state differs")
                        action_events, action_counts, copies = [], [0]*32, 0
                        start = q["intervention_us"]
                        for phase_index, (phase, duration) in enumerate(zip(trace, [50000, 10000, 10000, 30000])):
                            phases += 1
                            require(phase["start_us"] == start and phase["before_us"] == start-1
                                    and phase["end_us"] == start+duration-1, "phase clock differs")
                            if phase_index:
                                for before, after in [("potentials_before", "potentials_after"), ("refractory_until_before", "refractory_until_after")]:
                                    require(phase[before] == trace[phase_index-1][after], "phase state discontinuity")
                            for field in ["potentials_before", "potentials_after", "signed_input_per_core", "absolute_input_per_core"]:
                                require(len(phase[field]) == 32 and all(math.isfinite(v) for v in phase[field]), "invalid trace vector")
                            counts, arrivals, signed, absolute = [0]*32, [0]*32, [0.0]*32, [0.0]*32
                            spike_events, phase_copies, previous = [], 0, start
                            for event in phase["events"]:
                                events_total += 1
                                when = event["time_us"]
                                require(previous <= when <= phase["end_us"], "event time/order differs")
                                previous = when
                                if event["kind"] == "arrival":
                                    edge = network[event["synapse"]]
                                    require((edge["pre"], edge["post"]) == (event["source"], event["target"]), "arrival endpoints differ")
                                    require(not phase_index or event["synapse"] not in blocked, "blocked arrival escaped")
                                    amplitude = event["amplitude"]
                                    require(math.isfinite(amplitude) and (amplitude >= 0 if polarities[event["source"]] == "Excitatory" else amplitude <= 0), "arrival polarity differs")
                                    if 2 <= event["target"] < 34:
                                        n = event["target"]-2
                                        arrivals[n] += 1; signed[n] += amplitude; absolute[n] += abs(amplitude)
                                        phase_copies += event["source"] in [36, 37]
                                elif event["kind"] == "spike":
                                    require(event["source"] in polarities, "unknown spike")
                                    if 2 <= event["source"] < 34:
                                        counts[event["source"]-2] += 1
                                        spike_events.append([event["source"], when])
                                elif event["kind"] == "external":
                                    require(event["target"] in polarities and math.isfinite(event["amplitude"]), "unknown input")
                                else:raise ValueError("unexpected/plastic trace event")
                            require(counts == phase["core_counts"] and arrivals == phase["arrivals_per_core"], "event counts differ")
                            require(m10.equal(signed, phase["signed_input_per_core"]) and m10.equal(absolute, phase["absolute_input_per_core"]), "input sums differ")
                            require(phase_copies == phase["copy_arrivals"], "copy count differs")
                            m10.audit_voltage(phase, result)
                            if phase_index:
                                action_counts = [a+b for a, b in zip(action_counts, counts)]
                                action_events.extend(spike_events); copies += phase_copies
                            start += duration
                        require(action_counts == q["core_counts"] and action_events == q["core_events"], "action features differ")
                        require(copies == q["copy_arrivals"] == 9, "action copy changed")
                        reconstructed = m10c.reconstruct(q, result)
                        maximum_error = max(maximum_error, reconstructed["error_linf"])
    require(windows == 128 and phases == 512, "branch coverage differs")
    return dict(windows=windows, phases=phases, events=events_total, cancelled_arrivals=cancelled,
                maximum_voltage_error=maximum_error, reference_reproduced=True, sham_reproduced=True)


def measure(result, analyze_pair):
    rows = []
    for arm in ["before", "aged_frozen", "paired", "scrambled"]:
        for mapping in ["normal_mapping", "reversed_mapping"]:
            model = result["arms"][arm][mapping]
            for stage in ["checkpoint_probe", "rest_checkpoint_probe"]:
                branches = model["recurrence_probes"]["stages"][stage]
                for variant in VARIANTS:
                    pair = branches[variant]
                    values = analyze_pair(model["recurrence_probes"]["count_readout"], pair, model[stage]["labels"])
                    rows.append(dict(seed=result["seed"], arm=arm, mapping=mapping, stage=stage, variant=variant,
                        **values,
                        mean_spikes=sum(sum(q["core_counts"]) for q in pair)/2,
                        mean_active_cells=sum(sum(n > 0 for n in q["core_counts"]) for q in pair)/2,
                        max_spikes_one_cell=max(max(q["core_counts"]) for q in pair),
                        changed_count_windows=sum(q["core_counts"] != sham["core_counts"] for q, sham in zip(pair, branches["sham"])),
                        changed_predictions=sum(q["prediction_count"] != sham["prediction_count"] for q, sham in zip(pair, branches["sham"])),
                        cancelled_arrivals=sum(q["recurrence_gate"]["cancelled_arrivals"] for q in pair)))
    return rows
