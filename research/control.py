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
    require(set(control) == allowed, "control fields must be exactly "+", ".join(sorted(allowed)))
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
    trials = len(seeds)*len(GROUPS)
    require(trials <= control["max_trials"], "complete paired design exceeds max_trials")
    require(trials*4 <= control["max_native_training_passes"],
            "design including reconstruction and replay exceeds max_native_training_passes")
    return spec


def study(spec, seed):
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
    require(trials == len(GROUPS)*len(plans), "native design size differs")
    shutil.copy2(binary, root/"runner")
    shutil.copy2(Path(__file__), root/"producer.py")
    atomic(root/"source.toml", source.encode())
    project = HERE.parents[1]
    paths = [Path(__file__)]
    for folder in ("NERVA", "NERVA_Modell/Modell", "NERVA_Modell/nerva-qemu", "NERVA_Modell/research"):
        package = project/folder
        paths += sorted((package/"src").rglob("*.rs"))
        paths += [package/p for p in ("Cargo.toml", "Cargo.lock") if (package/p).is_file()]
    paths += [HERE/"build.rs"]
    with zipfile.ZipFile(root/"source-code.zip", "w", compression=zipfile.ZIP_DEFLATED) as archive:
        for path in sorted(set(paths)):
            archive.write(path, path.relative_to(project))
    model = spec["model"]
    steps = (12*model["training_repetitions"]+12*math.ceil(model["rest_us"]/50000)+12)*trials*2
    plan = dict(protocol="NERVA-Research-Control/1", specification=spec, seeds=plans,
                priorities=["chemistry_on_vs_off", "three_factor_vs_pair_stdp", "input_activity_weight_recall"],
                counts=dict(trials=trials, replay_executions=trials*2,
                            native_training_passes=trials*4, maximum_model_steps=steps),
                artifacts={name:sha(root/name) for name in
                           ("runner", "producer.py", "source.toml", "source-code.zip")},
                limits=["Exploratory fixed cue-completion assay; no independent confirmation.",
                        "No search, automatic winner selection or changes to neuronal rules.",
                        "Workers are bounded per seed; every seed includes all eight controls.",
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
    result = analyze(records)
    result["counts"] = plan["counts"]
    result["record_bytes"] = sum((root/path).stat().st_size for path in hashes)
    write_json(root/"control-report.json", result)
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
    writer.writerow(["seed", "control", *METRICS])
    for row in result["pipeline"]:
        writer.writerow([row["seed"], row["group"], *(row["metrics"][key] for key in METRICS)])
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
