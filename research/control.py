#!/usr/bin/env python3
"""Bounded control center for paired hippocampal experiments; Python standard library."""
import argparse
from concurrent.futures import ThreadPoolExecutor, as_completed
import contextlib
import csv
import fcntl
import hashlib
import io
import json
import math
import os
from pathlib import Path
import re
import shutil
import sqlite3
import subprocess
import sys
import tempfile
import time
import tomllib
import zipfile
import factorial_report

HERE = Path(__file__).resolve().parent
GROUPS = ("three_on", "three_off", "pair_on", "pair_off", "frozen_on", "frozen_off",
          "three_unpaired", "pair_unpaired")
METRICS = ("completion_a", "completion_b", "acquisition_a", "acquisition_b",
           "weight_gain_a", "weight_gain_b", "separation", "all_responses_active",
           "weight_l1", "sensory_spikes", "sensor_to_core_arrivals", "training_core_spikes",
           "requested_updates", "requests_without_change", "actual_update_mass",
           "zero_factor_integrations", "motor_factor_samples", "recall_core_effect",
           "recall_motor_effect", "recall_potential_effect", "matched_checkpoints")
# Four primary cue comparisons plus the frozen and unpaired checks for three_on.
CONTRASTS = ((1, "three_on", "three_off"), (2, "three_on", "pair_on"),
             (3, "three_on", "frozen_on"), (3, "three_on", "three_unpaired"))
CHAIN_GROUPS = ("A_numeric", "B_memory", "C_gain", "D_local")
CHAIN_METRICS = ("input_discriminable", "representation_discriminable", "local_tag_available",
                 "persistent_weight_change", "native_recall_effect", "artificial_recall_effect",
                 "diagnostic_controls_passed", "audit_complete", "native_training_passes", "audit_rows")
PRECISION_GROUPS = ("direct", "accumulated", "direct_frozen", "accumulated_frozen",
                    "direct_unpaired", "accumulated_unpaired")
PRECISION_METRICS = METRICS + ("requested_mass", "published_mass", "pending_mass", "clipped_mass",
    "rounding_error_mass", "mass_balance_max_error", "signed_requested_mass", "signed_published_mass", "signed_pending_mass")
DIAGNOSIS_METRICS = ("reference_verified", "natural_core_effect", "natural_motor_effect",
    "natural_margin_gain_a", "natural_margin_gain_b", "matched_checkpoints", "audit_rows", "native_training_passes")


def suite(spec):
    return spec["control"].get("suite", "association")


def design_counts(spec):
    seeds = len(spec["control"]["seeds"])
    if suite(spec) == "direction_strength":
        return seeds, seeds*50
    if suite(spec) == "direction_path":
        return seeds, seeds*28
    if suite(spec) == "precision":
        return seeds*len(PRECISION_GROUPS)+1, seeds*len(PRECISION_GROUPS)*4
    if suite(spec) == "causal_chain":
        # A/B/D are deterministic calibrations, shared once rather than counted as new seed evidence.
        return seeds+3, 2*(7+6*seeds)
    return seeds*len(GROUPS), seeds*len(GROUPS)*4


def require(condition, message):
    if not condition:
        raise ValueError(message)


def sha(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def atomic(path, data):
    path.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.NamedTemporaryFile(dir=path.parent, delete=False) as stream:
        temporary = Path(stream.name)
        stream.write(data)
        stream.flush()
        os.fsync(stream.fileno())
    try:
        os.replace(temporary, path)
    finally:
        temporary.unlink(missing_ok=True)


def write_json(path, value):
    atomic(path, (json.dumps(value, indent=2, sort_keys=True, allow_nan=False)+"\n").encode())


def read_json(path):
    return json.loads(path.read_bytes())


def integer(value, low, high, name):
    require(type(value) is int and low <= value <= high, f"{name} must be in {low}..{high}")
    return value


def parse(source):
    spec = tomllib.loads(source)
    require(set(spec) == {"control", "model"}, "expected only [control] and [model]")
    control, model = spec["control"], spec["model"]
    require(isinstance(control, dict) and isinstance(model, dict), "control/model must be TOML tables")
    allowed = {"name", "seeds", "workers", "max_trials", "max_native_training_passes", "max_wall_seconds"}
    require(set(control)-{"suite"} == allowed, "control fields must be "+", ".join(sorted(allowed))+" and optional suite")
    require(suite(spec) in ("association", "causal_chain", "precision", "direction_path", "direction_strength"), "unknown control suite")
    require(set(model) == {"processing_neurons", "training_repetitions", "rest_us"},
            "model requires processing_neurons, training_repetitions and rest_us")
    require(isinstance(control["name"], str) and re.fullmatch(r"[A-Za-z0-9_-]{1,80}", control["name"]),
            "invalid control name")
    seeds = control["seeds"]
    require(isinstance(seeds, list) and 1 <= len(seeds) <= 64, "use 1..64 explicit paired seeds")
    for seed in seeds:
        integer(seed, 0, 2**64-1, "seed")
    require(len(set(seeds)) == len(seeds), "duplicate seeds")
    integer(control["workers"], 1, 8, "workers")
    integer(control["max_wall_seconds"], 1, 86400, "max_wall_seconds")
    integer(control["max_trials"], 1, 512, "max_trials")
    integer(control["max_native_training_passes"], 1, 2048, "max_native_training_passes")
    integer(model["processing_neurons"], 16, 256, "processing_neurons")
    integer(model["training_repetitions"], 1, 16, "training_repetitions")
    integer(model["rest_us"], 100000, 20000000, "rest_us")
    trials, training_passes = design_counts(spec)
    require(trials <= control["max_trials"], "complete paired design exceeds max_trials")
    require(training_passes <= control["max_native_training_passes"],
            "design including reconstruction and replay exceeds max_native_training_passes")
    return spec


def study(spec, seed):
    if suite(spec) == "direction_strength":
        return dict(experiment=dict(name=f"factorial_s{seed}", scenario="hippocampal_direction_strength",
            seeds=[seed], interventions=["direction_strength"], verify_replay=True, max_runs=1),
            base=spec["model"], measurements=dict(metrics=list(factorial_report.METRICS)))
    if suite(spec) == "direction_path":
        return dict(experiment=dict(name=f"diagnosis_s{seed}", scenario="hippocampal_direction_path",
            seeds=[seed], interventions=["direction_path"], verify_replay=True, max_runs=1),
            base=spec["model"], measurements=dict(metrics=list(DIAGNOSIS_METRICS)))
    if suite(spec) == "precision":
        groups = (("precision_numeric",) if seed == spec["control"]["seeds"][0] else ()) + PRECISION_GROUPS
        return dict(experiment=dict(name=f"precision_s{seed}", scenario="hippocampal_precision",
            seeds=[seed], interventions=list(groups), verify_replay=True, max_runs=len(groups)),
            base=spec["model"], measurements=dict(metrics=list(PRECISION_METRICS)))
    if suite(spec) == "causal_chain":
        groups = CHAIN_GROUPS if seed == spec["control"]["seeds"][0] else ("C_gain",)
        return dict(experiment=dict(name=f"chain_s{seed}", scenario="hippocampal_causal_chain",
                                    seeds=[seed], interventions=list(groups), verify_replay=True,
                                    max_runs=len(groups)), base=spec["model"],
                    measurements=dict(metrics=list(CHAIN_METRICS)))
    return dict(experiment=dict(name=f"association_s{seed}", scenario="hippocampal_association",
                                seeds=[seed], interventions=list(GROUPS),
                                verify_replay=True, max_runs=len(GROUPS)),
                base=spec["model"], measurements=dict(metrics=list(METRICS)))


def native(binary, mode, spec, output=None, timeout=None, log=None):
    command = [str(binary), mode]
    if output is not None:
        command += ["--output", str(output)]
    process = subprocess.run(command, input=json.dumps(spec), text=True, capture_output=True, timeout=timeout)
    if log is not None:
        atomic(log, (process.stdout+process.stderr).encode())
    require(process.returncode == 0, f"native {mode} failed: "+process.stderr[-4000:])
    return json.loads(process.stdout) if mode == "plan" else None


@contextlib.contextmanager
def locked(root):
    root.mkdir(parents=True, exist_ok=True)
    with (root/".control.lock").open("a+") as lock:
        try:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            raise ValueError("control output is locked") from None
        yield


def prepare(root, spec, source, binary=None):
    require(not (root/"control-plan.json").exists(), "control plan already exists")
    require(not any(p.name != ".control.lock" for p in root.iterdir()),
            "refusing nonempty output without a control plan")
    if binary is None:
        subprocess.run(["cargo", "build", "--offline", "--release", "--manifest-path", str(HERE/"Cargo.toml")],
                       check=True)
        binary = HERE/"target/release/nerva-research"
    binary = binary.resolve()
    plans = [native(binary, "plan", study(spec, seed)) for seed in spec["control"]["seeds"]]
    trials = sum(p["counts"]["trials"] for p in plans)
    expected_trials, training_passes = design_counts(spec)
    require(trials == expected_trials, "native design size differs")
    if suite(spec) in ("causal_chain", "precision", "direction_path", "direction_strength"):
        native_passes = 2*sum(job["effective_config"]["training_passes_per_execution"]
                              for plan in plans for job in plan["jobs"])
        require(native_passes == training_passes, "native reconstruction/replay cost differs")
    shutil.copy2(binary, root/"runner")
    shutil.copy2(Path(__file__), root/"producer.py")
    shutil.copy2(HERE/"factorial_report.py", root/"factorial_report.py")
    atomic(root/"source.toml", source.encode())
    project = HERE.parents[1]
    paths = [Path(__file__), HERE/"factorial_report.py"]
    for folder in ("NERVA", "NERVA_Modell/Modell", "NERVA_Modell/nerva-qemu", "NERVA_Modell/research"):
        package = project/folder
        paths += sorted((package/"src").rglob("*.rs"))
        paths += [package/p for p in ("Cargo.toml", "Cargo.lock") if (package/p).is_file()]
    paths += [HERE/"build.rs"]
    with zipfile.ZipFile(root/"source-code.zip", "w", compression=zipfile.ZIP_DEFLATED) as archive:
        for path in sorted(set(paths)):
            archive.write(path, path.relative_to(project))
    model = spec["model"]
    if suite(spec) == "direction_strength":
        steps = 50*(12*model["training_repetitions"]+5*(math.ceil(model["rest_us"]/50000)+1))*trials
    elif suite(spec) == "direction_path":
        steps = 28*(12*model["training_repetitions"]+5*(math.ceil(model["rest_us"]/50000)+1))*trials
    elif suite(spec) == "causal_chain":
        steps = (36*model["training_repetitions"]+20*math.ceil(model["rest_us"]/50000)+20)*len(plans)*2
    elif suite(spec) == "precision":
        steps = (12*model["training_repetitions"]+12*math.ceil(model["rest_us"]/50000)+12)*(trials-1)*2
    else:
        steps = (12*model["training_repetitions"]+12*math.ceil(model["rest_us"]/50000)+12)*trials*2
    plan = dict(protocol="NERVA-Research-Control/1", specification=spec, seeds=plans,
                suite=suite(spec),
                priorities=["direction_strength_factorial", "recurrent_relay_search"] if suite(spec) == "direction_strength" else
                    ["verified_artificial_direction", "exact_recall_path"] if suite(spec) == "direction_path" else
                    ["retained_update_mass", "native_spike_effect", "controlled_association_gain"] if suite(spec) == "precision" else
                    list(CHAIN_GROUPS) if suite(spec) == "causal_chain" else
                    ["chemistry_on_vs_off", "three_factor_vs_pair_stdp", "input_activity_weight_recall"],
                counts=dict(trials=trials, replay_executions=trials*2,
                            native_training_passes=training_passes, maximum_model_steps=steps),
                artifacts={name:sha(root/name) for name in
                           ("runner", "producer.py", "factorial_report.py", "source.toml", "source-code.zip")},
                limits=["Exploratory fixed cue-completion assay; no independent confirmation.",
                        "Direction/path and direction/strength use finite artificial searches; first verified dose per method, in-sample only.",
                        "Direction/strength training and step counts are upper bounds; duplicate weight vectors reuse exact observations.",
                        "Workers are bounded per seed; each experiment retains its complete internal controls.",
                        "The wall budget covers native execution and verification, not compilation."])
    write_json(root/"control-plan.json", plan)
    atomic(root/"control-plan.sha256", (sha(root/"control-plan.json")+"\n").encode())
    return plan


def load_plan(root, source=None):
    path = root/"control-plan.json"
    require(path.is_file(), "control plan missing; use control plan first")
    require(sha(path) == (root/"control-plan.sha256").read_text().strip(), "control plan checksum differs")
    plan = read_json(path)
    require(plan["protocol"] == "NERVA-Research-Control/1", "unknown control protocol")
    for name, digest in plan["artifacts"].items():
        require(sha(root/name) == digest, "archived artifact checksum differs: "+name)
    require(sha(Path(__file__)) == plan["artifacts"]["producer.py"],
            "controller changed; use the archived producer.py or a new output")
    if source is not None:
        require(source == (root/"source.toml").read_text(), "source TOML differs; use a new output")
    if (root/"receipt.json").exists():
        for relative, digest in read_json(root/"receipt.json")["records_sha256"].items():
            require(sha(root/relative) == digest, "completed raw record differs: "+relative)
    return plan


def seed_root(root, seed):
    return root/"seeds"/f"s{seed}"


def remaining(deadline):
    seconds = deadline-time.monotonic()
    require(seconds > 0, "control wall-time budget exhausted; completed records can be resumed")
    return seconds


def execute(root, plan, report_only=False):
    spec = plan["specification"]
    deadline = time.monotonic()+spec["control"]["max_wall_seconds"]
    def one(seed):
        target = seed_root(root, seed)
        native(root/"runner", "report" if report_only else "run", study(spec, seed), target,
               remaining(deadline), root/"logs"/f"s{seed}.log")
        return seed
    failures = []
    # Every child has the same absolute deadline. Native execution has no VM children.
    with ThreadPoolExecutor(max_workers=min(spec["control"]["workers"], len(spec["control"]["seeds"]))) as pool:
        futures = {pool.submit(one, seed):seed for seed in spec["control"]["seeds"]}
        for future in as_completed(futures):
            seed = futures[future]
            try:
                future.result()
                print(f"VERIFIED seed={seed}", flush=True)
            except (OSError, ValueError, subprocess.TimeoutExpired) as exc:
                failures.append(f"seed={seed}: {exc}")
    require(not failures, "\n".join(failures))
    return report(root, plan)


def hints(metrics, audit):
    result = []
    if metrics["sensory_spikes"] == 0:
        result.append("no_generated_input_spikes")
    elif metrics["sensor_to_core_arrivals"] == 0:
        result.append("no_sensory_arrivals_at_core")
    elif metrics["training_core_spikes"] == 0:
        result.append("no_core_spikes_during_training")
    if audit["tag_events"] and metrics["requested_updates"] == 0 and metrics["zero_factor_integrations"] > 0:
        result.append("eligibility_without_nonzero_weight_request")
    if metrics["requests_without_change"] > 0:
        result.append("nonzero_requests_without_actual_change")
    if metrics["weight_l1"] > 0 and metrics["recall_core_effect"] == 0:
        result.append("net_weight_changes_without_core_spike_effect_in_recall")
    if metrics["recall_core_effect"] and not metrics["recall_motor_effect"]:
        result.append("core_spike_effect_without_motor_spike_effect")
    if audit["chemical_samples"] and metrics["motor_factor_samples"] == 0:
        result.append("no_nonzero_motor_factor_observed")
    return result


def analyze(records):
    paired = {}
    for row in records:
        seed, group = row["job"]["seed"], row["job"]["intervention"]
        require(group in GROUPS and group not in paired.setdefault(seed, {}), "duplicate/unknown control")
        require(row["full_replay_checked"] is True, "full replay required")
        require(set(row["metrics"]) == set(METRICS), "metric contract differs")
        require(all(type(v) in (int, float) and math.isfinite(v) for v in row["metrics"].values()),
                "invalid metric")
        require(row["details"]["matched_checkpoint"] is True and row["details"]["only_weights_changed"] is True,
                "matched-state weight control missing")
        require(row["details"]["training"]["audit"]["complete"] is True, "incomplete learning audit")
        paired[seed][group] = row
    require(paired and all(set(groups) == set(GROUPS) for groups in paired.values()), "all eight controls required per seed")
    for groups in paired.values():
        require(len({r["details"]["initial_network_digest"] for r in groups.values()}) == 1,
                "initial networks differ")
        require(len({r["details"]["training"]["input_digest"] for g,r in groups.items()
                     if not g.endswith("unpaired")}) == 1, "paired inputs differ")
    contrasts = []
    family_tests = len(CONTRASTS)*2
    for priority, a, b in CONTRASTS:
        for cue in ("a", "b"):
            differences = [groups[a]["metrics"]["weight_gain_"+cue]-groups[b]["metrics"]["weight_gain_"+cue]
                           for _,groups in sorted(paired.items())]
            n = len(differences)
            wins = sum(d > 0 for d in differences)
            # Ties stay in the full denominator as nonwins; conservative one-sided sign test.
            p = sum(math.comb(n, k) for k in range(wins, n+1))/2**n
            mean = sum(differences)/n
            completion = [groups[a]["metrics"]["completion_"+cue]-groups[b]["metrics"]["completion_"+cue]
                          for _,groups in sorted(paired.items())]
            contrasts.append(dict(priority=priority, a=a, b=b, cue=cue, seeds=sorted(paired),
                                  differences=differences, mean=mean, wins=wins,
                                  completion_differences=completion, completion_mean=sum(completion)/n,
                                  ties=sum(d == 0 for d in differences), p_one_sided=p,
                                  family_tests=family_tests, threshold=0.05/family_tests,
                                  exploratory_threshold_met=mean > 0 and p < 0.05/family_tests))
    observations = []
    for seed, groups in sorted(paired.items()):
        for group in GROUPS:
            row = groups[group]
            observations.append(dict(seed=seed, group=group, metrics=row["metrics"],
                observations=hints(row["metrics"], row["details"]["training"]["audit"])))
    means = {group:{metric:sum(groups[group]["metrics"][metric] for groups in paired.values())/len(paired)
                   for metric in METRICS} for group in GROUPS}
    return dict(protocol="NERVA-Research-Control-Report/1", priorities=contrasts, control_means=means,
                pipeline=observations, paired_seeds=len(paired),
                functional_memory_demonstrated=False, independent_confirmation=False,
                architecture="Technically integrated hippocampal profiles",
                infrastructure="Paired controls, replay, bounded audits and resume",
                demonstrated_learning="Open; consult the measured contrasts",
                limits=["Observed bottleneck hints are not proof of information absence.",
                        "Chemistry changes excitability and transmission as well as learning.",
                        "Weight-reset recall isolates the effect of final weights at the same trained checkpoint.",
                        "Cue completion is a fixed external readout, not autonomous behavior or validated cognition."])


def analyze_chain(records):
    by_group = {name:[] for name in CHAIN_GROUPS}
    seen = set()
    pipeline = []
    for row in records:
        seed, group = row["job"]["seed"], row["job"]["intervention"]
        require(group in by_group and (seed, group) not in seen, "duplicate/unknown chain experiment")
        seen.add((seed, group))
        metrics = row["metrics"]
        require(set(metrics) == set(CHAIN_METRICS), "chain metric contract differs")
        require(all(type(v) in (int, float) and math.isfinite(v) for v in metrics.values()),
                "invalid chain metric")
        require(row["full_replay_checked"] is True and metrics["audit_complete"] == 1,
                "chain requires complete observations and replay")
        require(row["details"]["functional_memory_demonstrated"] is False, "artificial diagnosis is not task learning")
        by_group[group].append(row)
        pipeline.append(dict(seed=seed, group=group, metrics=metrics,
                             observations=[] if metrics["diagnostic_controls_passed"] == 1 else
                             ["positive_or_negative_diagnostic_control_failed"]))
    require(all(len(by_group[g]) == 1 for g in ("A_numeric", "B_memory", "D_local")) and by_group["C_gain"],
            "one shared A/B/D calibration and at least one model seed required")
    common_seed = by_group["A_numeric"][0]["job"]["seed"]
    require(all(by_group[g][0]["job"]["seed"] == common_seed for g in ("B_memory", "D_local")),
            "shared calibrations must belong to the same reference seed")
    require(common_seed in {r["job"]["seed"] for r in by_group["C_gain"]}, "reference model seed missing")
    data = {g:by_group[g][0]["details"]["data"] for g in ("A_numeric", "B_memory", "D_local")}
    controls_ok = all(row["metrics"]["diagnostic_controls_passed"] == 1 for row in records)
    models = []
    stage_names = ("input_discriminable", "representation_discriminable", "local_tag_available",
                   "persistent_weight_change", "native_recall_effect")
    for row in sorted(by_group["C_gain"], key=lambda r:r["job"]["seed"]):
        d = row["details"]["data"]
        require([r["gain"] for r in d["doses"]] == [0, 1, 10, 100, 1000], "gain grid differs")
        require(all(r["matched_checkpoint"] is True and r["only_weights_changed"] is True for r in d["doses"]),
                "gain intervention changed unmatched state")
        unresolved = next((key for key in stage_names if row["metrics"][key] == 0), None)
        models.append(dict(seed=row["job"]["seed"], first_unresolved_observed_stage=unresolved,
                           interpretation_valid=controls_ok, metrics=row["metrics"],
                           first_sampled_core_effect_gain=d["first_sampled_core_effect_gain"],
                           doses=[{key:value for key,value in r.items() if key != "recall"} for r in d["doses"]],
                           bench_weight_curve=d["bench_weight_curve"]))
    # Large native checkpoints, weights and recall vectors stay in the raw records.
    return dict(protocol="NERVA-Causal-Chain-Report/1", suite="causal_chain",
                calibrations=data, models=models, pipeline=pipeline,
                paired_seeds=len(models), diagnostic_controls_passed=controls_ok,
                functional_memory_demonstrated=False, independent_confirmation=False,
                limits=["A/B/D are shared deterministic mechanism calibrations, not independent seed evidence.",
                        "Artificial microcircuit success does not certify the hippocampal pathway.",
                        "A finite gain grid gives sampled effects and clipping, not a universal or monotone threshold.",
                        "Zero requested signal, numerical rounding and bounds are separately measured."])


def chain_markdown(result, plan):
    lines = ["# Kausale Diagnosekette A–D", "",
             f"{len(result['models'])} Modellseeds, gemeinsame A/B/D-Kalibrierung; "
             f"{plan['counts']['trials']} Records mit vollständigem Replay.", "",
             f"Diagnosekontrollen bestanden: **{result['diagnostic_controls_passed']}**.",
             "Künstlich garantierte Fähigkeiten werden getrennt vom natürlichen Lernen ausgewertet.", "",
             "## A – Numerische Plastizität", "",
             "| Fall | Faktor | Tag | Angeforderte Summe | Tatsächliche Summe | Weggerundet | Begrenzt |",
             "| --- | ---: | ---: | ---: | ---: | ---: | ---: |"]
    numeric = result["calibrations"]["A_numeric"]
    for row in numeric["cases"]+numeric["partition_comparison"]:
        lines.append(f"| {row['case']} | {row['held_factor']:.6g} | {row['tag_after_pair']:.6g} | "
                     f"{row['requested_signed_sum']:.9g} | {row['actual_signed_sum']:.9g} | "
                     f"{row['rounded_away_steps']} | {row['clipped_steps']} |")
    lines += ["", "Der f64-Akkumulator ist eine externe Diagnose und ersetzt keine Lernregel.",
              "Bei der Partitionierung werden angeforderte und tatsächliche Masse in beiden Ästen separat gemessen.", "",
              "## B – Künstliches Gedächtnis", "",
              "| Kontrolle | Gewicht | Cue-Zelle | B feuert | Gewicht erhalten |", "| --- | ---: | ---: | --- | --- |"]
    for row in result["calibrations"]["B_memory"]["cases"]:
        lines.append(f"| {row['control']} | {row['weight']} | {row['cue_neuron']} | {row['target_fired']} | {row['weight_retained']} |")
    lines += ["", "Drei-Zell-Testnetz; die Zielzelle erhält keinen direkten Cue. Dies ist eine künstlich gesetzte Verbindung, kein Lernerfolg.", "",
              "## C – Wirksamkeitsschwelle", "",
              "| Seed | Gain | Installierte Änderung L1 | Begrenzte Kanten | Core-Spikes anders | Motor-Spikes anders |",
              "| --- | ---: | ---: | ---: | --- | --- |"]
    for model in result["models"]:
        for row in model["doses"]:
            lines.append(f"| {model['seed']} | {row['gain']} | {row['installed_l1_from_initial']:.9g} | "
                         f"{row['clipped_edges']} | {row['core_effect']} | {row['motor_effect']} |")
    lines += ["", "Gain 0 stellt Anfangsgewichte her; Gain 1 enthält native Lerndeltas; größere Gains sind künstliche Eingriffe.",
              "Alle Äste starten mit demselben vollständigen trainierten Zustand. Kein adaptives Nachsuchen.", "",
              "## D – Lokale Informationsverfügbarkeit", "",
              "| Kontrolle | Pre-Ankünfte | Post-Spikes | Tag vor Puls | Nichtnull-Anfragen | Gewichtsdelta |",
              "| --- | --- | --- | ---: | ---: | ---: |"]
    for row in result["calibrations"]["D_local"]["cases"]:
        lines.append(f"| {row['control']} | {row['pre_arrivals_us']} | {row['post_spikes_us']} | "
                     f"{row['tag_before_release']:.9g} | {row['nonzero_requests']} | {row['weight_delta']:.9g} |")
    lines += ["", "Prä-/Post-Zeiten und Faktoren werden am gleichen nativen Lernort gemessen.",
              "Ein später Puls kann noch einen winzigen Tagrest treffen; Überlappung ist von wirksamer Speicherung zu unterscheiden.", "",
              "## Beobachtete Grenze im Hippocampus-Modell", "",
              "| Seed | Erste nicht bestandene beobachtete Stufe | Künstlicher Gain mit erster Core-Wirkung |",
              "| --- | --- | --- |"]
    for model in result["models"]:
        stage = model["first_unresolved_observed_stage"] if model["interpretation_valid"] else "Diagnosekontrolle fehlgeschlagen"
        lines.append(f"| {model['seed']} | {stage} | {model['first_sampled_core_effect_gain']} |")
    lines += ["", "Stufen mit Messwert -1 wurden nicht geprüft. Diese Diagnose ist keine unabhängige Bestätigung.",
              "Ein funktionaler Lern- oder Gedächtnisnachweis bleibt offen.", ""]
    return lines


def analyze_precision(records):
    calibration = [r for r in records if r["job"]["intervention"] == "precision_numeric"]
    require(len(calibration) == 1 and calibration[0]["full_replay_checked"] is True,
            "one replay-verified precision calibration required")
    numeric = calibration[0]["details"]
    require(numeric["controls_passed"] is True, "numeric accumulation controls failed")
    paired = {}
    pipeline = []
    for row in records:
        group = row["job"]["intervention"]
        if group == "precision_numeric":
            continue
        seed = row["job"]["seed"]
        require(group in PRECISION_GROUPS and group not in paired.setdefault(seed, {}), "duplicate/unknown precision control")
        require(row["full_replay_checked"] is True, "precision requires full replay")
        m = row["metrics"]
        require(set(m) == set(PRECISION_METRICS) and
                all(type(v) in (int,float) and math.isfinite(v) for v in m.values()), "invalid precision metrics")
        require(row["details"]["matched_checkpoint"] is True and row["details"]["only_weights_changed"] is True,
                "precision checkpoint/weight control missing")
        require(row["details"]["training"]["audit"]["complete"] is True and m["mass_balance_max_error"] < 1e-12,
                "incomplete or unbalanced precision audit")
        if group.endswith("frozen"):
            require(m["weight_l1"] == 0 and m["pending_mass"] == 0 and m["published_mass"] == 0,
                    "frozen precision control changed weights or carry")
        paired[seed][group] = row
        pipeline.append(dict(seed=seed,group=group,metrics=m,
                             observations=hints(m,row["details"]["training"]["audit"])))
    require(paired and all(set(g) == set(PRECISION_GROUPS) for g in paired.values()),
            "all six precision controls required per seed")
    effects = []
    for seed,groups in sorted(paired.items()):
        require(len({r["details"]["initial_network_digest"] for r in groups.values()}) == 1,
                "precision initial networks differ")
        starting_weights=[r["details"]["training"]["weights_before"] for r in groups.values()]
        require(all(w == starting_weights[0] for w in starting_weights), "precision starting weights differ")
        require(len({r["details"]["training"]["input_digest"] for g,r in groups.items() if not g.endswith("unpaired")}) == 1,
                "paired precision inputs differ")
        direct,accumulated = groups["direct"],groups["accumulated"]
        def changed(field):
            a=direct["details"]["learned_recall"]["observations"]
            b=accumulated["details"]["learned_recall"]["observations"]
            return any(x[field]!=y[field] for x,y in zip(a,b))
        effects.append(dict(seed=seed, core_spikes_changed=changed("core_event_digest"),
                            motor_spikes_changed=changed("motor_event_digest"),
                            potentials_changed=changed("stored_potentials"),
                            direct=direct["metrics"],accumulated=accumulated["metrics"]))
    contrasts = []
    for label,a,b in (("accumulated_minus_direct","accumulated","direct"),
                       ("accumulated_minus_frozen","accumulated","accumulated_frozen"),
                       ("accumulated_minus_unpaired","accumulated","accumulated_unpaired"),
                       ("positive_accumulated_weight_gain","accumulated",None)):
        for cue in ("a","b"):
            differences=[g[a]["metrics"]["weight_gain_"+cue]-(g[b]["metrics"]["weight_gain_"+cue] if b else 0.)
                         for _,g in sorted(paired.items())]
            n=len(differences);wins=sum(d>0 for d in differences)
            p=sum(math.comb(n,k) for k in range(wins,n+1))/2**n
            mean=sum(differences)/n
            contrasts.append(dict(comparison=label,cue=cue,mean=mean,differences=differences,
                                  wins=wins,ties=sum(d==0 for d in differences),p_one_sided=p,
                                  threshold=0.05/8,exploratory_threshold_met=mean>0 and p<0.05/8))
    means={group:{key:sum(g[group]["metrics"][key] for g in paired.values())/len(paired)
                  for key in PRECISION_METRICS} for group in PRECISION_GROUPS}
    advantage=all(c["exploratory_threshold_met"] for c in contrasts) and all(
        g["accumulated"]["metrics"]["all_responses_active"] == 1 for g in paired.values())
    return dict(protocol="NERVA-Precision-Report/1",suite="precision",numeric_calibration=numeric,
                pipeline=pipeline,paired_seeds=len(paired),control_means=means,effects=effects,contrasts=contrasts,
                exploratory_association_advantage=advantage,functional_memory_demonstrated=False,
                independent_confirmation=False,
                limits=["Only numerical publication differs; eligibility still uses f32.",
                        "Gross requested, published, clipped and rounding-error mass are separate quantities.",
                        "Opposing signed updates may cancel; counts of unchanged requests are not lost mass.",
                        "Two known seeds provide an engineering comparison, not independent confirmation."])


def precision_markdown(result, plan):
    lines=["# Erhalt kleiner Lernbeiträge: Direktupdates gegen Akkumulator","",
           f"{result['paired_seeds']} gepaarte Modellseeds; {plan['counts']['trials']} Records mit vollständigem Replay.","",
           "## 1. Numerische Kalibrierung","",
           "| Modus | Intervalle | Angefordert | Sichtbar publiziert | Restbetrag | Nicht erhaltene Masse |",
           "| --- | ---: | ---: | ---: | ---: | ---: |"]
    for c in result["numeric_calibration"]["cases"]:
        lines.append(f"| {c['mode']} | {c['partitions']} | {c['requested']:.12g} | {c['published']:.12g} | "
                     f"{c['residual']:.12g} | {c['effective_mass_error']:.12g} |")
    lines+=["","## 2. Änderungsmasse im Modell (Kontrollmittel)","",
            "| Kontrolle | Angeforderte Bruttomasse | Publizierte Bruttomasse | Rest L1 | Begrenzte Masse | Rundungsfehler L1 |",
            "| --- | ---: | ---: | ---: | ---: | ---: |"]
    for group,m in result["control_means"].items():
        lines.append(f"| {group} | {m['requested_mass']:.9g} | {m['published_mass']:.9g} | "
                     f"{m['pending_mass']:.9g} | {m['clipped_mass']:.9g} | {m['rounding_error_mass']:.9g} |")
    lines+=["","Bruttomassen dürfen wegen Vorzeichenwechseln nicht mit Netto-Endgewichtsänderungen gleichgesetzt werden.",
            "Ein nicht sichtbarer Einzelbeitrag kann im Akkumulator erhalten sein.","",
            "## 3. Native Spikewirkung und Assoziation","",
            "| Seed | Core-Spikes durch Präzision geändert | Motor-Spikes geändert | Direkter Gewichtseffekt A/B | Akkumulierter Gewichtseffekt A/B |",
            "| --- | --- | --- | --- | --- |"]
    for e in result["effects"]:
        d,a=e["direct"],e["accumulated"]
        lines.append(f"| {e['seed']} | {e['core_spikes_changed']} | {e['motor_spikes_changed']} | "
                     f"{d['weight_gain_a']:.9g} / {d['weight_gain_b']:.9g} | {a['weight_gain_a']:.9g} / {a['weight_gain_b']:.9g} |")
    lines+=["","## Kontrollierte Assoziationsvergleiche","",
            "| Vergleich | Cue | Mittlere Differenz | Positive Seeds | p (explorativ) |",
            "| --- | --- | ---: | ---: | ---: |"]
    for c in result["contrasts"]:
        lines.append(f"| {c['comparison']} | {c['cue']} | {c['mean']:.9g} | {c['wins']}/{result['paired_seeds']} | {c['p_one_sided']:.6g} |")
    lines+=["",f"Explorative Kriterien für Assoziationsvorteil erfüllt: **{result['exploratory_association_advantage']}**.",
            "Acht vorab feste einseitige Tests; Gleichstände bleiben im Nenner. Keine unabhängige Bestätigung.",
            "Ein funktionaler Gedächtnisnachweis wird daraus nicht abgeleitet.",""]
    return lines


def analyze_diagnosis(records):
    pipeline, diagnoses, seen = [], [], set()
    def close(a, b):
        return math.isclose(a, b, rel_tol=1e-9, abs_tol=1e-12)
    def similarity(a, b):
        aa, bb = sum(x*x for x in a), sum(x*x for x in b)
        return min(1., max(0., sum(x*y for x,y in zip(a,b))/math.sqrt(aa*bb))) if aa and bb else 0.
    def margins(probe, targets):
        counts = [f["core_counts"] for f in probe["frames"]]
        return [similarity(counts[3],targets[0])-similarity(counts[3],targets[1]),
                similarity(counts[4],targets[1])-similarity(counts[4],targets[0])]
    def path_summary(probe):
        neurons = [n for f in probe["frames"][3:] for n in f["neurons"]]
        return {key:sum(n[key] for n in neurons) for key in (
            "batches","spikes","refractory_batches","subthreshold_batches","excitation_cancelled_batches",
            "positive_input","negative_input","continuous_potential_change","reset_potential_change")}
    def trace_difference(base, branch, changed):
        output = []
        for original, actual in zip(base["frames"][3:],branch["frames"][3:]):
            old_edges = {e["id"]:e for e in original["synapses"]}
            new_edges = {e["id"]:e for e in actual["synapses"]}
            active = changed & (old_edges.keys() | new_edges.keys())
            differences = []
            for sid in sorted(old_edges.keys() | new_edges.keys()):
                old, new = old_edges.get(sid,{}), new_edges.get(sid,{})
                delta = sum(new.get(k,0)-old.get(k,0) for k in ("positive_amplitude","negative_amplitude"))
                if old != new:
                    differences.append(dict(synapse_id=sid,changed_weight=sid in changed,
                        delta_signed_amplitude=delta,before=old,after=new))
            old_nodes = {n["id"]:n for n in original["neurons"]}
            new_nodes = {n["id"]:n for n in actual["neurons"]}
            node_ids = old_nodes.keys() | new_nodes.keys()
            output.append(dict(image=actual["image"],changed_weight_edges=len(changed),
                changed_weight_edges_with_arrivals=len(active),arrival_summary_changed_edges=len(differences),
                changed_weight_edges_with_changed_arrivals=sum(e["changed_weight"] for e in differences),
                spike_count_changed_neurons=sum(old_nodes.get(i,{}).get("spikes",0)!=new_nodes.get(i,{}).get("spikes",0) for i in node_ids),
                threshold_gap_changed_neurons=sum(old_nodes.get(i,{}).get("minimum_eligible_threshold_gap")!=
                    new_nodes.get(i,{}).get("minimum_eligible_threshold_gap") for i in node_ids),
                largest_arrival_differences=sorted(differences,key=lambda e:(-abs(e["delta_signed_amplitude"]),e["synapse_id"]))[:32],
                limits="Aggregate arrival differences can include divergent spike histories; unchanged sums do not prove identical timing."))
        return output
    for row in sorted(records,key=lambda r:r["job"]["seed"]):
        seed, m, d = row["job"]["seed"], row["metrics"], row["details"]
        require(seed not in seen and row["job"]["intervention"] == "direction_path", "duplicate/unknown diagnosis record")
        seen.add(seed)
        require(row["full_replay_checked"] is True and set(m) == set(DIAGNOSIS_METRICS), "diagnosis needs full replay and metrics")
        require(all(type(v) in (float,int) and math.isfinite(v) for v in m.values()), "invalid diagnosis metric")
        require(d["matched_checkpoint"] is True and d["only_weights_changed"] is True and m["matched_checkpoints"] == 1,
                "diagnosis checkpoint control missing")
        require(d["training"]["audit"]["complete"] is True and m["native_training_passes"] == 14, "diagnosis training audit/cost differs")
        candidates = d["candidates"]
        require([(c["scope"],c["dose"],c["family"]) for c in candidates] ==
                [(scope,dose,family) for scope in ("sensory_and_recurrent","recurrent_only")
                 for dose in (.25,1.,2.) for family in ("targeted","swapped")], "incomplete fixed candidate grid")
        probes = [d["baseline"],d["natural"],*(c["probe"] for c in candidates)]
        for probe in probes:
            require(probe["complete"] is True and probe["parameters_preserved"] is True, "incomplete/learning recall audit")
            require([f["image"] for f in probe["frames"]] == [4,0,1,2,3], "wrong diagnosis probes")
            require(sum(f["audit_rows"] for f in probe["frames"]) == probe["audit_rows"], "electrical row count differs")
            for frame in probe["frames"]:
                for n in frame["neurons"]:
                    error = n["stored_after"]-n["stored_before"]-(n["continuous_potential_change"]+n["accepted_net"]+
                            n["integration_rounding"]+n["reset_potential_change"])
                    require(abs(error)<1e-9 and close(error,n["balance_error"]), "recall potential ledger differs")
                    require(n["batches"] == n["spikes"]+n["refractory_batches"]+n["subthreshold_batches"],
                            "integration classifications overlap or are incomplete")
        require(sum(p["audit_rows"] for p in probes)==m["audit_rows"], "total electrical budget differs")
        targets = [f["core_counts"] for f in d["baseline"]["frames"][1:3]]
        distinct = all(sum(t)>0 for t in targets) and similarity(*targets)<1.-1e-9
        require(distinct == d["fixed_targets_distinct"], "fixed target separation differs")
        base_margin, natural_margin = margins(d["baseline"],targets), margins(d["natural"],targets)
        require(all(close(a,b) for a,b in zip(base_margin,d["baseline_margins"])) and
                all(close(a,b) for a,b in zip(natural_margin,d["natural_margins"])), "fixed readout differs")
        for cue in range(2):
            require(close(natural_margin[cue]-base_margin[cue],m["natural_margin_gain_"+("a","b")[cue]]),
                    "natural margin gain differs")
        for metric, field in (("natural_core_effect","core_event_digest"),("natural_motor_effect","motor_event_digest")):
            require(m[metric] == int(any(a[field]!=b[field] for a,b in
                zip(d["natural"]["frames"],d["baseline"]["frames"]))), "natural spike effect differs")
        first = None
        for index,c in enumerate(candidates):
            require(c["matched_checkpoint"] is True and c["only_weights_changed"] is True, "candidate checkpoint missing")
            actual_margin = margins(c["probe"],targets)
            gain = [a-b for a,b in zip(actual_margin,base_margin)]
            require(all(close(a,b) for a,b in zip(actual_margin,c["completion_margins"])) and
                    all(close(a,b) for a,b in zip(gain,c["margin_gains"])), "candidate readout differs")
            eligible = distinct and all(x>1e-9 for x in actual_margin+gain) and all(
                sum(f["core_counts"])>0 for f in c["probe"]["frames"][3:]) and (
                sum(c["probe"]["frames"][0]["core_counts"])<=sum(d["baseline"]["frames"][0]["core_counts"]))
            require(eligible == c["candidate_passed_without_swap_control"], "candidate controls differ")
            # Recalculate alignment from the archived per-edge initial, natural and installed weights.
            require(len(c["weights"]) == len(d["edges"]), "candidate weights missing")
            pairs = []
            for e,(sid,w) in zip(d["edges"],c["weights"]):
                require(sid==e[0] and 0<=w<=2, "candidate edge identity/range differs")
                if c["scope"]=="recurrent_only" and not e[3].startswith("core_"):
                    require(w==e[4], "recurrent-only candidate changed an input connection")
                pairs.append((e[5]-e[4],w-e[4]))
            aa,bb = sum(a*a for a,b in pairs),sum(b*b for a,b in pairs)
            cosine = sum(a*b for a,b in pairs)/math.sqrt(aa*bb) if aa and bb else None
            stored = c["alignment"]["cosine"]
            require((cosine is None and stored is None) or (cosine is not None and stored is not None and close(cosine,stored)),
                    "direction alignment differs")
            verified = eligible and index%2==0 and all(
                gain[cue]>candidates[index+1]["margin_gains"][cue]+1e-9 for cue in range(2))
            require(c["verified_reference"] == verified, "reference lacks swapped-target control")
            if verified and (first is None or (index>=6 and first<6)):
                first=index
        require(d["reference_verified"] == (first is not None) and m["reference_verified"] == int(first is not None),
                "reference status differs")
        if first is None:
            require(d["direction_reference"] is None, "unverified direction must remain unresolved")
            observation = "Keine verifizierte Referenz im festen Suchraum; Ursachenentscheidung offen."
        else:
            require(d["direction_reference"]["candidate_index"]==first and
                    d["direction_reference"]["scope"]==candidates[first]["scope"] and
                    d["direction_reference"]["alignment"]==candidates[first]["alignment"], "wrong selected reference")
            observation = "Künstlicher Abrufpfad verifiziert; Richtungswinkel ist eine lokale Diagnose, kein Lernnachweis."
        require(d["functional_memory_demonstrated"] is False and d["independent_confirmation"] is False,
                "artificial fitting cannot certify learned memory")
        pipeline.append(dict(seed=seed,group="direction_path",metrics=m,observations=[observation]))
        diagnoses.append(dict(seed=seed,reference_verified=first is not None,
            reference_scope=candidates[first]["scope"] if first is not None else None,
            references_by_scope={scope:next((index for index,c in enumerate(candidates)
                if c["scope"]==scope and c["verified_reference"]),None)
                for scope in ("sensory_and_recurrent","recurrent_only")},
            selected_candidate=first,direction_reference=d["direction_reference"],
            natural_margin_gains=[m["natural_margin_gain_a"],m["natural_margin_gain_b"]],
            paths={name:path_summary(p) for name,p in (("baseline",d["baseline"]),("natural",d["natural"]))},
            natural_path_difference=trace_difference(d["baseline"],d["natural"],{e[0] for e in d["edges"] if e[4]!=e[5]}),
            reference_path_difference=trace_difference(d["baseline"],candidates[first]["probe"],
                {e[0] for e,(_,w) in zip(d["edges"],candidates[first]["weights"]) if e[4]!=w}) if first is not None else None,
            candidates=[{key:c[key] for key in ("scope","family","dose","installed_l1","margin_gains","verified_reference","alignment")}
                        for c in candidates]))
    require(pipeline, "no direction/path records")
    return dict(protocol="NERVA-Direction-Path-Report/1",suite="direction_path",paired_seeds=len(pipeline),
                pipeline=pipeline,diagnoses=diagnoses,functional_memory_demonstrated=False,independent_confirmation=False)


def diagnosis_markdown(result, plan):
    lines = ["# Richtungs- und Abrufpfadanalyse", "",
        f"{result['paired_seeds']} Seeds; {plan['counts']['native_training_passes']} Trainingsdurchläufe einschließlich Replay.",
        "", "| Seed | Verifizierte Referenz | Pfad | Natürliche Wirkung Cue A / B | Cosinus zur Referenz |",
        "| --- | --- | --- | ---: | ---: |"]
    for d in result["diagnoses"]:
        direction=d["direction_reference"]
        angle=direction["alignment"]["cosine"] if direction else None
        gains=d["natural_margin_gains"]
        lines.append(f"| {d['seed']} | {'ja' if d['reference_verified'] else 'offen'} | {d['reference_scope'] or 'offen'} | "
                     f"{gains[0]:.9g} / {gains[1]:.9g} | {angle if angle is not None else 'nicht definiert'} |")
    lines += ["", "| Seed | Pfadumfang | Erste gültige Dosis |", "| --- | --- | ---: |"]
    for d in result["diagnoses"]:
        for scope,index in d["references_by_scope"].items():
            dose=d["candidates"][index]["dose"] if index is not None else "keine im Raster"
            lines.append(f"| {d['seed']} | {scope} | {dose} |")
    lines += ["", "Nur verbesserte, richtige Ergänzung beider Cues gegen Anfangsgewichte und vertauschte Zielzuordnung zählt als Referenz.",
        "Die Vollmuster des Anfangsgewichts-Abrufs bleiben als Zielvektoren fest. Ohne Cue darf die Core-Spikezahl nicht steigen.",
        "Die künstliche Suche nutzt Kalibrieraktivität; sie liefert keine unabhängige Bestätigung und keinen natürlichen Lernnachweis.",
        "Rekurrente Eingriffe werden getrennt von Sensor→Core-Eingriffen geprüft; eine gültige rekurrente Referenz hat Vorrang.",
        "Unterschiedliche Änderungsnormen und Clipping sind dokumentiert; der Richtungswinkel allein beweist keinen Lernregelfehler.",
        "", "| Seed | Zweig | Cue-Batches | Spikes | Refraktär | Unterschwellig | Durch Hemmung unter Schwelle |",
        "| --- | --- | ---: | ---: | ---: | ---: | ---: |"]
    for d in result["diagnoses"]:
        for name,p in d["paths"].items():
            lines.append(f"| {d['seed']} | {name} | {p['batches']} | {p['spikes']} | {p['refractory_batches']} | "
                         f"{p['subthreshold_batches']} | {p['excitation_cancelled_batches']} |")
    lines += ["", "| Seed | Cue | Geänderte Gewichte mit Ankunft | Veränderte Ankunftssummen | Andere Schwellenabstände | Andere Spikezahlen |",
              "| --- | --- | ---: | ---: | ---: | ---: |"]
    for d in result["diagnoses"]:
        for p in d["natural_path_difference"]:
            lines.append(f"| {d['seed']} | {p['image']} | {p['changed_weight_edges_with_arrivals']}/{p['changed_weight_edges']} | "
                         f"{p['arrival_summary_changed_edges']} | {p['threshold_gap_changed_neurons']} | {p['spike_count_changed_neurons']} |")
    lines += ["", "Rohrecords enthalten alle Synapsen-/Neuronensummen je Probe sowie maximal 128 zeitgenaue Integrationsbeispiele.",
        "Kontinuierliche Potenzialänderung enthält Leck, intrinsische Ströme und Modulation. Reset und Rundungsrest sind separat bilanziert.",
        "Hemmungszählung: Erregung allein würde am selben Zustand die Schwelle erreichen; dies ersetzt keinen simulierten Knockout.",
        "Eine erfolglose begrenzte Gewichtssuche beweist weder einen defekten Netzwerkpfad noch eine falsche Lernregel.",
        "Unterschiedliche Zweige können im Verlauf der festen Probensequenz unterschiedliche Vorgeschichten entwickeln.", ""]
    return lines



def report(root, plan):
    records, hashes = [], {}
    for seed_plan in plan["seeds"]:
        seed = seed_plan["jobs"][0]["seed"]
        target = seed_root(root, seed)
        require(read_json(target/"manifest.json") == seed_plan, "seed manifest differs")
        for job in seed_plan["jobs"]:
            path = target/"records"/(job["id"]+".json")
            row = read_json(path)["record"]
            require(row["job"] == job, "record job differs")
            records.append(row)
            hashes[path.relative_to(root).as_posix()] = sha(path)
    family = suite(plan["specification"])
    result = {"direction_strength":factorial_report.analyze,"direction_path":analyze_diagnosis,"precision":analyze_precision,"causal_chain":analyze_chain,"association":analyze}[family](records)
    result["counts"] = plan["counts"]
    result["record_bytes"] = sum((root/path).stat().st_size for path in hashes)
    write_json(root/"control-report.json", result)
    if family == "direction_strength":
        lines = factorial_report.markdown(result, plan)
    elif family == "direction_path":
        lines = diagnosis_markdown(result, plan)
    elif family == "precision":
        lines = precision_markdown(result,plan)
    elif family == "causal_chain":
        lines = chain_markdown(result, plan)
    else:
        lines = ["# Hippocampus: Forschungskontrolle", "",
                 f"{result['paired_seeds']} gepaarte Seeds; {plan['counts']['trials']} Records, vollständiger Replay.", "",
                 "| Priorität | Vergleich | Cue | Abrufdifferenz | Gewichtswirkung A − B | Positive Seeds | p (explorativ) |",
                 "| --- | --- | --- | ---: | ---: | ---: | ---: |"]
        for contrast in result["priorities"]:
            lines.append(f"| {contrast['priority']} | {contrast['a']} − {contrast['b']} | {contrast['cue']} | "
                         f"{contrast['completion_mean']:.9g} | {contrast['mean']:.9g} | "
                         f"{contrast['wins']}/{len(contrast['seeds'])} | {contrast['p_one_sided']:.6g} |")
        lines += ["", "Gewichtswirkung = Ergänzungskontrast mit gelernten Gewichten minus demselben Abruf mit Anfangsgewichten.",
                  "Acht vorab feste Tests auf Gewichtswirkung; Gleichstände bleiben im Nenner. Keine unabhängige Bestätigung.", "",
                  "## Kontrollen im Mittel", "", "| Kontrolle | Ergänzung A | Ergänzung B | Netto-Gewichtsänderung L1 | Core-Spikes im Training |",
                  "| --- | ---: | ---: | ---: | ---: |"]
        for group, metrics in result["control_means"].items():
            lines.append(f"| {group} | {metrics['completion_a']:.9g} | {metrics['completion_b']:.9g} | "
                         f"{metrics['weight_l1']:.9g} | {metrics['training_core_spikes']:.9g} |")
        lines += ["", "## Beobachtete Engpässe", "", "| Seed | Kontrolle | Beobachtungen |", "| --- | --- | --- |"]
        for row in result["pipeline"]:
            lines.append(f"| {row['seed']} | {row['group']} | {', '.join(row['observations']) or 'Kein Engpass durch diese Regeln lokalisiert'} |")
        lines += ["", "Diese Hinweise lokalisieren beobachtete Grenzen im festen Assay; sie beweisen keine Informationsabwesenheit.",
                  "Ein funktionaler Gedächtnisnachweis bleibt offen.", ""]
    atomic(root/"CONTROL_REPORT.md", "\n".join(lines).encode())
    output = io.StringIO()
    writer = csv.writer(output)
    metrics = {"direction_strength":factorial_report.METRICS,"direction_path":DIAGNOSIS_METRICS,"precision":PRECISION_METRICS,"causal_chain":CHAIN_METRICS,"association":METRICS}[family]
    writer.writerow(["seed", "control", *metrics])
    for row in result["pipeline"]:
        writer.writerow([row["seed"], row["group"], *(row["metrics"][key] for key in metrics)])
    atomic(root/"control-metrics.csv", output.getvalue().encode())
    descriptor, filename = tempfile.mkstemp(prefix="control-", suffix=".sqlite", dir=root)
    os.close(descriptor)
    temporary = Path(filename)
    try:
        with sqlite3.connect(temporary) as db:
            db.execute("CREATE TABLE trials(seed TEXT, control TEXT, metrics TEXT, observations TEXT, PRIMARY KEY(seed,control))")
            for row in result["pipeline"]:
                db.execute("INSERT INTO trials VALUES(?,?,?,?)", (str(row["seed"]),row["group"],
                           json.dumps(row["metrics"],sort_keys=True),json.dumps(row["observations"])))
            db.execute("CREATE TABLE metadata(key TEXT PRIMARY KEY,value TEXT)")
            db.execute("INSERT INTO metadata VALUES(?,?)", ("plan_sha256",sha(root/"control-plan.json")))
            if family == "precision":
                db.execute("INSERT INTO metadata VALUES(?,?)", ("numeric_calibration",json.dumps(result["numeric_calibration"],sort_keys=True)))
            require(db.execute("PRAGMA integrity_check").fetchone()[0] == "ok", "control SQLite integrity failure")
        os.replace(temporary, root/"control.sqlite")
    finally:
        temporary.unlink(missing_ok=True)
    receipt = dict(plan_sha256=sha(root/"control-plan.json"), records_sha256=hashes,
                   statement="Native report verified record integrity; SHA-256 binds this completed snapshot.")
    write_json(root/"receipt.json", receipt)
    return result


def status(root, plan):
    counts = {}
    for seed in plan["specification"]["control"]["seeds"]:
        records = seed_root(root, seed)/"records"
        counts[str(seed)] = len(list(records.glob("*.json")))
    return dict(expected=plan["counts"]["trials"], cached_records=sum(counts.values()), by_seed=counts,
                verified_snapshot=(root/"receipt.json").is_file(),
                limits="File counts are progress indicators; run/report performs native record validation.")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=["plan", "run", "report", "status"])
    parser.add_argument("config", type=Path, nargs="?")
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    require(args.config is not None or args.output is not None, "a config or --output is required")
    source = args.config.read_text() if args.config else None
    spec = parse(source) if source is not None else None
    root = (args.output or HERE/"results"/spec["control"]["name"]).resolve()
    if args.mode == "status":
        print(json.dumps(status(root, load_plan(root, source)), indent=2))
        return
    with locked(root):
        if (root/"control-plan.json").exists():
            plan = load_plan(root, source)
        else:
            require(args.mode in ("plan", "run") and spec is not None, "plan/config required")
            plan = prepare(root, spec, source)
        if args.mode == "plan":
            print(json.dumps(plan["counts"], indent=2))
        else:
            execute(root, plan, args.mode == "report")
            print("REPORT", root/"CONTROL_REPORT.md")


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, KeyError, subprocess.SubprocessError) as exc:
        print("nerva-control:", exc, file=sys.stderr)
        raise SystemExit(1)
