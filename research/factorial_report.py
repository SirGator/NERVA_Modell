"""Independent arithmetic validation of native direction/strength records; stdlib only."""
import math
import struct

METRICS = ("reference_verified", "recurrent_reference_verified", "requested_factorial_valid",
           "feasible_factorial_valid", "natural_gain_a", "natural_gain_b", "native_training_passes", "audit_rows")
METHODS = ("sensory_and_recurrent", "recurrent_contrast", "recurrent_two_hop")


def require(condition, message):
    if not condition:
        raise ValueError(message)


def norm(values):
    return math.sqrt(sum(x*x for x in values))


def close(a, b):
    return math.isclose(a, b, rel_tol=1e-9, abs_tol=1e-12)


def f32(value):
    return struct.unpack("f", struct.pack("f", value))[0]


def cosine(a, b):
    denominator = norm(a)*norm(b)
    return max(-1., min(1., sum(x*y for x,y in zip(a,b))/denominator)) if denominator else None


def similarity(a, b):
    return max(0., cosine(a,b) or 0.)


def margins(probe, targets):
    q = [f["core_counts"] for f in probe["frames"][3:]]
    return [similarity(q[0],targets[0])-similarity(q[0],targets[1]),
            similarity(q[1],targets[1])-similarity(q[1],targets[0])]


def ray_limit(initial, direction):
    length=norm(direction)
    if not length:
        return None
    return min((2.-w)/(d/length) if d>0 else -w/(d/length)
               for w,d in zip(initial,direction) if d)


def validate_matrix(matrix, observations, initial, natural, artificial, expected_high):
    if matrix is None:
        require(expected_high is None, "factorial matrix unexpectedly missing")
        return None
    require(expected_high is not None and close(matrix["high_strength"],expected_high), "wrong factorial high norm")
    low=norm(natural)
    require(close(matrix["natural_strength"],low) and set(matrix["arms"])=={"NN","NR","RN","RR"}, "incomplete factorial axes")
    for name,direction,strength in (("NN",natural,low),("NR",natural,expected_high),
                                    ("RN",artificial,low),("RR",artificial,expected_high)):
        arm=matrix["arms"][name]
        length=norm(direction)
        limit=ray_limit(initial,direction)
        require(close(arm["requested_norm"],strength), "wrong requested arm norm")
        require((limit is None and arm["ray_limit"] is None) or
                (limit is not None and arm["ray_limit"] is not None and close(arm["ray_limit"],limit)), "wrong ray limit")
        feasible=bool(length and strength<=limit+1e-12*max(limit,1.))
        require(arm["feasible"]==feasible, "arm feasibility differs")
        if not feasible:
            require(arm["observation"] is None and arm["faithful"] is False, "infeasible arm claims an observation")
            continue
        index=arm["observation"]
        require(type(index) is int and 0<=index<len(observations) and
                arm["matched_checkpoint"] is True and arm["only_weights_changed"] is True, "arm checkpoint missing")
        scale=arm["ray_norm"]
        require(0<=scale<=limit+1e-12 and arm["normalization_iterations"] in (0,48), "unbounded numeric normalization")
        raw=[w+(d if scale==length else scale*(d/length)) for w,d in zip(initial,direction)]
        require(all(-1e-12<=w<=2.+1e-12 for w in raw), "arm clips a mathematical direction")
        desired=[f32(min(2.,max(0.,w))) for w in raw]
        actual=[w for sid,w in observations[index]["weights"]]
        require(actual==desired, "arm installed weights differ from its normalized ray")
        delta=[w-w0 for w,w0 in zip(actual,initial)]
        magnitude=norm(delta)
        angle=cosine(delta,direction)
        relative=abs(magnitude-strength)/strength if strength else 0.
        faithful=relative<=.01 and angle is not None and angle>=.999
        require(close(arm["actual_norm"],magnitude) and close(arm["relative_norm_error"],relative) and
                ((angle is None and arm["direction_cosine"] is None) or
                 (angle is not None and arm["direction_cosine"] is not None and close(angle,arm["direction_cosine"]))) and
                close(arm["rounding_l2"],norm([a-b for a,b in zip(actual,raw)])) and arm["faithful"]==faithful,
                "arm publication fidelity differs")
    valid=bool(norm(natural) and norm(artificial) and abs(expected_high-low)>1e-6*low and
               all(a["feasible"] and a["faithful"] for a in matrix["arms"].values()))
    require(matrix["valid"]==valid, "invalid factorial comparison")
    if not valid:
        require(matrix["contrasts"] is None, "invalid normalization produced causal contrasts")
        return None
    contrasts=[]
    for cue in range(2):
        values={name:observations[a["observation"]]["completion_margins"][cue] for name,a in matrix["arms"].items()}
        nn,nr,rn,rr=(values[name] for name in ("NN","NR","RN","RR"))
        contrasts.append(dict(direction_at_natural_strength=rn-nn,direction_at_high_strength=rr-nr,
            strength_on_natural_direction=nr-nn,strength_on_reference_direction=rr-rn,interaction=rr-nr-rn+nn))
    require(len(matrix["contrasts"])==2 and all(set(a)==set(b) and all(close(a[k],b[k]) for k in a)
        for a,b in zip(contrasts,matrix["contrasts"])), "factorial effects differ")
    return contrasts


def analyze(records):
    pipeline, studies, seen = [], [], set()
    for row in sorted(records,key=lambda r:r["job"]["seed"]):
        seed, m, d=row["job"]["seed"],row["metrics"],row["details"]
        require(seed not in seen and row["job"]["intervention"]=="direction_strength", "duplicate/unknown factorial record")
        seen.add(seed)
        require(row["full_replay_checked"] is True and set(m)==set(METRICS) and
                all(type(v) in (int,float) and math.isfinite(v) for v in m.values()), "invalid/unreplayed factorial metrics")
        require(d["matched_checkpoint"] is True and d["only_weights_changed"] is True and
                d["training"]["audit"]["complete"] is True, "factorial checkpoint/audit missing")
        observations,edges=d["observations"],d["edges"]
        require(observations and d["maximum_training_passes"]==25 and
                len(observations)+1==m["native_training_passes"]<=25, "actual reconstruction cost differs")
        initial=[e[4] for e in edges]
        natural=[e[5]-e[4] for e in edges]
        signatures=set()
        targets=[f["core_counts"] for f in observations[0]["probe"]["frames"][1:3]]
        distinct=all(sum(t)>0 for t in targets) and similarity(*targets)<1.-1e-9
        require(distinct==d["fixed_targets_distinct"], "fixed targets differ")
        baseline_margin=margins(observations[0]["probe"],targets)
        maximum_balance_error=0.
        for observation in observations:
            require(len(observation["weights"])==len(edges) and all(
                sid==e[0] and 0<=w<=2 for e,(sid,w) in zip(edges,observation["weights"])), "observation weights invalid")
            signature=tuple(w for sid,w in observation["weights"])
            require(signature not in signatures, "duplicate weight vectors were simulated twice")
            signatures.add(signature)
            probe=observation["probe"]
            require(probe["complete"] is True and probe["parameters_preserved"] is True and
                    [f["image"] for f in probe["frames"]]==[4,0,1,2,3], "incomplete frozen factorial probe")
            computed=margins(probe,targets)
            require(all(close(a,b) for a,b in zip(computed,observation["completion_margins"])) and
                    all(close(a-b,g) for a,b,g in zip(computed,baseline_margin,observation["margin_gains"])), "factorial readout differs")
            require(sum(f["audit_rows"] for f in probe["frames"])==probe["audit_rows"], "factorial audit row count differs")
            for field in ("core","motor"):
                effect=any(a[field+"_event_digest"]!=b[field+"_event_digest"]
                    for a,b in zip(probe["frames"],observations[0]["probe"]["frames"]))
                require(observation[field+"_effect"]==effect, "factorial spike effect differs")
            for frame in probe["frames"]:
                for n in frame["neurons"]:
                    error=n["stored_after"]-n["stored_before"]-(n["continuous_potential_change"]+
                        n["accepted_net"]+n["integration_rounding"]+n["reset_potential_change"])
                    maximum_balance_error=max(maximum_balance_error,abs(error))
                    require(abs(error)<1e-9 and close(error,n["balance_error"]) and
                            n["batches"]==n["spikes"]+n["refractory_batches"]+n["subthreshold_batches"],
                            "factorial electrical ledger differs")
        require([w for sid,w in observations[0]["weights"]]==initial, "baseline weights differ")
        require(sum(o["probe"]["audit_rows"] for o in observations)==m["audit_rows"], "total factorial audit count differs")
        candidates=d["candidates"]
        require([(c["method"],c["dose"],c["family"]) for c in candidates]==
                [(method,dose,family) for method in METHODS for dose in (.25,1.,2.) for family in ("targeted","swapped")],
                "incomplete recurrent search grid")
        require(set(d["directions"])==set(METHODS), "search directions missing")
        references={}
        for index,c in enumerate(candidates):
            direction=d["directions"][c["method"]]
            require(len(direction)==len(edges) and all(math.isfinite(v) and abs(v)<=1.+1e-12 for v in direction),
                    "search direction invalid")
            if c["method"].startswith("recurrent_"):
                require(all(v==0. for e,v in zip(edges,direction) if not e[3].startswith("core_")),
                        "recurrent search changes sensor projections")
            observation=c["observation"]
            require(type(observation) is int and 0<=observation<len(observations), "candidate observation missing")
            o=observations[observation]
            sign=1. if c["family"]=="targeted" else -1.
            raw=[w+sign*c["dose"]*v for w,v in zip(initial,direction)]
            require([w for sid,w in o["weights"]]==[f32(min(2.,max(0.,w))) for w in raw] and
                    c["clipped_edges"]==sum(not 0<=w<=2 for w in raw), "candidate weights/clipping differ")
            eligible=distinct and all(x>1e-9 for x in o["completion_margins"]+o["margin_gains"]) and all(
                sum(f["core_counts"])>0 for f in o["probe"]["frames"][3:]) and (
                sum(o["probe"]["frames"][0]["core_counts"])<=sum(observations[0]["probe"]["frames"][0]["core_counts"]))
            require(c["candidate_passed_without_swap_control"]==eligible, "candidate control differs")
            verified=eligible and c["family"]=="targeted" and all(o["margin_gains"][cue]>
                observations[candidates[index+1]["observation"]]["margin_gains"][cue]+1e-9 for cue in range(2))
            require(c["verified_reference"]==verified, "unverified reference selected")
            if verified:
                references.setdefault(c["method"],index)
        require(d["references_by_method"]==references, "first verified reference differs")
        selected=next((references[k] for k in ("recurrent_contrast","recurrent_two_hop","sensory_and_recurrent") if k in references),None)
        recurrent=any(k.startswith("recurrent_") for k in references)
        require(m["reference_verified"]==int(selected is not None) and m["recurrent_reference_verified"]==int(recurrent), "reference metric differs")
        natural_index=d["natural_observation"]
        require(type(natural_index) is int and 0<=natural_index<len(observations) and
                [w for sid,w in observations[natural_index]["weights"]]==[e[5] for e in edges], "natural observation differs")
        require(all(close(observations[natural_index]["margin_gains"][cue],m["natural_gain_"+("a","b")[cue]])
                    for cue in range(2)), "natural gains differ")
        if selected is None:
            require(d["direction_reference"] is None and d["requested_factorial"] is None and d["feasible_factorial"] is None,
                    "factorial lacks a verified reference")
        else:
            choice=d["direction_reference"]
            require(choice["candidate_index"]==selected and choice["method"]==candidates[selected]["method"] and
                    choice["observation"]==candidates[selected]["observation"], "wrong direction reference")
            artificial=[w-w0 for w0,(sid,w) in zip(initial,observations[choice["observation"]]["weights"])]
            alignment=cosine(natural,artificial)
            stored=choice["alignment"]["cosine"]
            require((alignment is None and stored is None) or
                    (alignment is not None and stored is not None and close(alignment,stored)), "reference alignment differs")
            validate_matrix(d["requested_factorial"],observations,initial,natural,artificial,norm(artificial))
            limits=[ray_limit(initial,natural),ray_limit(initial,artificial)]
            high=min([norm(artificial),*limits]) if all(x is not None for x in limits) else None
            validate_matrix(d["feasible_factorial"],observations,initial,natural,artificial,high)
        require(m["requested_factorial_valid"]==int(d["requested_factorial"] is not None and d["requested_factorial"]["valid"]) and
                m["feasible_factorial_valid"]==int(d["feasible_factorial"] is not None and d["feasible_factorial"]["valid"]), "factorial validity metric differs")
        require(d["functional_memory_demonstrated"] is False and d["independent_confirmation"] is False, "artificial search claims learned memory")
        observation_text="Rekurrente Referenz verifiziert." if recurrent else "Keine verifizierte rekurrente Referenz im Raster."
        pipeline.append(dict(seed=seed,group="direction_strength",metrics=m,observations=[observation_text]))
        matrices={}
        for name in ("requested_factorial","feasible_factorial"):
            matrix=d[name]
            matrices[name]=None if matrix is None else dict(valid=matrix["valid"],natural_strength=matrix["natural_strength"],
                high_strength=matrix["high_strength"],contrasts=matrix["contrasts"],arms={
                    k:dict(**a,margin_gains=observations[a["observation"]]["margin_gains"] if a["observation"] is not None else None)
                    for k,a in matrix["arms"].items()})
        studies.append(dict(seed=seed,reference_method=d["direction_reference"]["method"] if selected is not None else None,
            recurrent_reference_verified=recurrent,references_by_method=references,matrices=matrices,
            actual_native_training_passes=int(m["native_training_passes"]),unique_observations=len(observations),
            maximum_balance_error=maximum_balance_error,candidates=[
                dict(**c,margin_gains=observations[c["observation"]]["margin_gains"]) for c in candidates]))
    require(pipeline, "no factorial records")
    return dict(protocol="NERVA-Direction-Strength-Report/1",suite="direction_strength",paired_seeds=len(pipeline),
        pipeline=pipeline,studies=studies,actual_native_training_passes=2*sum(s["actual_native_training_passes"] for s in studies),
        functional_memory_demonstrated=False,independent_confirmation=False)


def markdown(result, plan):
    lines=["# Richtung und Stärke unabhängig prüfen","",
        f"{result['paired_seeds']} Seeds; {result['actual_native_training_passes']} tatsächliche Trainingsdurchläufe "
        f"einschließlich Replay, Budget höchstens {plan['counts']['native_training_passes']}.","",
        "| Seed | Referenzmethode | Rekurrente Referenz | Gewünschtes 2×2 gültig | Gemeinsame zulässige Stärke gültig |",
        "| --- | --- | --- | --- | --- |"]
    for s in result["studies"]:
        requested,feasible=(s["matrices"][key] for key in ("requested_factorial","feasible_factorial"))
        lines.append(f"| {s['seed']} | {s['reference_method'] or 'offen'} | {s['recurrent_reference_verified']} | "
                     f"{bool(requested and requested['valid'])} | {bool(feasible and feasible['valid'])} |")
    lines+=["","| Seed | Matrix | Arm | Gewünschte L2-Stärke | Tatsächliche L2-Stärke | Richtungstreue | Gewinn Cue A / B |",
            "| --- | --- | --- | ---: | ---: | ---: | --- |"]
    for s in result["studies"]:
        for family,matrix in s["matrices"].items():
            if matrix is None:
                continue
            for arm,a in matrix["arms"].items():
                gains=a["margin_gains"]
                gain="nicht ausgeführt" if gains is None else f"{gains[0]:.9g} / {gains[1]:.9g}"
                lines.append(f"| {s['seed']} | {family} | {arm} | {a['requested_norm']:.9g} | "
                             f"{a.get('actual_norm','unzulässig')} | {a.get('direction_cosine','offen')} | {gain} |")
    lines+=["","NN: natürliche Richtung/natürliche Stärke; NR: natürliche Richtung/hohe Stärke;",
            "RN: Referenzrichtung/natürliche Stärke; RR: Referenzrichtung/hohe Stärke.",
            "Eine mathematisch unzulässige Skalierung wird nicht abgeschnitten. Sie bleibt als fehlende Variante sichtbar.",
            "Zusätzlich wird die hohe Stärke auf den gemeinsamen zulässigen Radius beider Richtungen beschränkt.",
            "Richtungseffekt, Stärkeneffekt und Interaktion werden nur für vollständig norm- und richtungstreue Matrizen berechnet.",
            "Die künstlichen Referenzen sind auf diesen Bildern kalibriert; sie beweisen kein natürlich gelerntes Gedächtnis.",
            "Die Suche über bestehende rekurrente Wege bleibt endlich; ein Fehlschlag beweist keine unmögliche Netzwerkfunktion.",
            "Gleiche Gewichtsvektoren nutzen dieselbe exakte Beobachtung und werden nicht erneut simuliert.",""]
    return lines
