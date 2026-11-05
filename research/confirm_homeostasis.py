#!/usr/bin/env python3
"""Preregister, execute/resume and verify a fixed paired homeostasis contrast."""
import argparse
import contextlib
import fcntl
import hashlib
import json
from pathlib import Path
import sqlite3
import subprocess
import sys
import tomllib

from analysis.homeostasis_confirmation import analyze, protocol
from evolution import history
from evolution.spec import require

HERE = Path(__file__).resolve().parent
PROJECT = HERE.parents[1]


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def seed_values(value):
    found = set()
    if isinstance(value, dict):
        for key, item in value.items():
            if key == "seed" and type(item) is int:
                found.add(item)
            if (key == "seeds" or key.endswith("_seeds")) and isinstance(item, list):
                found.update(v for v in item if type(v) is int)
            found.update(seed_values(item))
    elif isinstance(value, list):
        for item in value:
            found.update(seed_values(item))
    return found


def audit_seeds(seeds, output, config, roots=None):
    roots = roots if roots is not None else [HERE/"results", PROJECT/"Messungen", HERE/"config"]
    seen, files = set(), {}
    for root in roots:
        for path in sorted(root.rglob("*")):
            if not path.is_file() or path == config or output == path or output in path.parents:
                continue
            if not (path.name.endswith("manifest.json") or path.name in ["preregistered-contrasts.json", "preregistration.json"]
                    or (root == HERE/"config" and path.suffix == ".toml")):
                continue
            raw = path.read_bytes()
            try:
                value = tomllib.loads(raw.decode()) if path.suffix == ".toml" else json.loads(raw)
            except (ValueError, UnicodeDecodeError) as exc:
                raise ValueError("cannot audit seed catalog: "+str(path)) from exc
            used = seed_values(value)
            seen.update(used)
            files[str(path)] = dict(sha256=hashlib.sha256(raw).hexdigest(), seed_count=len(used))
    require(not set(seeds) & seen, "confirmation seeds overlap existing or reserved seeds: "+str(sorted(set(seeds) & seen)))
    return dict(roots=[str(p) for p in roots], files=files, known_seed_count=len(seen),
                overlap=[], scope="Local research/archive manifests and configured/reserved seeds; no assertion about external archives")


def source_identity(binary):
    paths = [Path(__file__), HERE/"analysis/homeostasis_confirmation.py", HERE/"run.py",
             HERE/"evolution/history.py", HERE/"evolution/mutation.py", HERE/"evolution/spec.py",
             HERE/"planning/profiles.py"]
    return dict(binary_sha256=sha(binary), sources_sha256={str(p.relative_to(HERE)):sha(p) for p in paths})


def record_hashes(root):
    return {p.name:sha(p) for p in sorted((root/"records").glob("*.json"))}


def native(mode, config, root, binary):
    subprocess.run([sys.executable,str(HERE/"run.py"),mode,str(config),
                    "--binary",str(binary),"--output",str(root)],check=True)


def execute(mode, config, root, binary, audit_roots=None):
    source = tomllib.loads(config.read_text())
    p = protocol(source)
    registration = root/"preregistration.json"
    identity = source_identity(binary)
    if mode == "report":
        require(registration.exists(), "confirmation-report requires preregistration")
    # A parent lock allows the native planner to initialize an empty output.
    root.parent.mkdir(parents=True, exist_ok=True)
    with (root.parent/("."+root.name+".confirmation.lock")).open("a+") as lock:
        try:
            fcntl.flock(lock,fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            raise ValueError("confirmation output is locked") from None
        if registration.exists():
            registered = history.read(registration)
            require(registered["protocol"] == p and registered["identity"] == identity
                    and registered["config_sha256"] == sha(config), "confirmation protocol, source or binary differs")
            require(sha(root/"manifest.json") == registered["manifest_sha256"], "confirmation manifest differs")
            require(sha(root/"source.toml") == registered["archived_source_sha256"], "archived confirmation source differs")
        else:
            require(not record_hashes(root), "cannot preregister after trials have been executed")
            # Reserve new seeds serially across confirmation plans.
            with (HERE/"results/.confirmation-seeds.lock").open("a+") as seed_lock:
                fcntl.flock(seed_lock,fcntl.LOCK_EX | fcntl.LOCK_NB)
                audit = audit_seeds(p["seeds"],root,config,audit_roots)
                native("plan",config,root,binary)
                require(not record_hashes(root), "preregistration requires zero raw records")
                registered = dict(protocol=p,identity=identity,config_sha256=sha(config),
                    manifest_sha256=sha(root/"manifest.json"),records_at_registration=0,
                    archived_source_sha256=sha(root/"source.toml"),
                    seed_audit=audit)
                history.immutable(registration,registered)
        if mode == "plan":
            print("PREREGISTERED",registration,"family_tests=",p["family_tests"],"per_test_alpha=",p["per_test_alpha"])
            return registered
        receipt = root/"verification-receipt.json"
        if receipt.exists():
            sealed = history.read(receipt)
            require(record_hashes(root) == sealed["records_sha256"], "sealed confirmation records differ")
        if mode == "run":
            native("run",config,root,binary)
        before = record_hashes(root)
        native("report",config,root,binary)
        require(before == record_hashes(root), "report changed raw confirmation records")
        manifest = json.loads((root/"manifest.json").read_bytes())
        require(set(before) == {job["id"]+".json" for job in manifest["jobs"]}, "unexpected confirmation record files")
        records = [json.loads((root/"records"/(job["id"]+".json")).read_bytes())["record"] for job in manifest["jobs"]]
        result = analyze(records,source)
        history.immutable(root/"confirmation.json",result)
        with contextlib.closing(sqlite3.connect(root/"results.sqlite")) as db:
            require(db.execute("PRAGMA integrity_check").fetchone()[0] == "ok", "confirmation SQLite integrity failure")
            require(db.execute("SELECT count(*) FROM trials").fetchone()[0] == len(records), "confirmation catalog trial count differs")
        history.immutable(receipt,dict(records_sha256=before,record_count=len(records),new_trials_during_report=0,
            raw_records_unchanged=True,sqlite_integrity="ok",preregistration_sha256=sha(registration),
            confirmation_sha256=sha(root/"confirmation.json"),identity=identity))
        lines = ["# Homeostasis confirmation","",
            f'{len(p["seeds"])} paired seeds; {len(records)} trials; family alpha={p["alpha"]}, threshold={p["per_test_alpha"]}.',"",
            "| Core neurons | Visual accuracy off/on | Visual loss confirmed | MAE off/on | External MAE change confirmed |",
            "|---:|---:|---|---:|---|"]
        for cell in result["cells"]:
            a,b=cell["means"]["off"],cell["means"]["on"]
            lines.append(f'| {cell["processing_neurons"]} | {a["visual_accuracy"]:.4f} / {b["visual_accuracy"]:.4f} | {cell["visual_loss"]["confirmed"]} | {a["position_mae"]:.4f} / {b["position_mae"]:.4f} | {cell["external_forecast_error_change"]["confirmed"]} |')
        lines += ["",*result["limits"],""]
        history.atomic(root/"confirmation.md","\n".join(lines).encode())
        print("CONFIRMATION",root/"confirmation.md","verified_records=",len(records))
        return result


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode",choices=["plan","run","report"])
    parser.add_argument("--config",type=Path,default=HERE/"config/homeostasis_confirmation.toml")
    parser.add_argument("--output",type=Path)
    parser.add_argument("--binary",type=Path)
    args=parser.parse_args()
    source=tomllib.loads(args.config.read_text())
    protocol(source)
    root=(args.output or HERE/"results"/source["experiment"]["name"]).resolve()
    binary=args.binary.resolve() if args.binary else HERE/"target/release/nerva-research"
    if args.binary is None:
        subprocess.run(["cargo","build","--offline","--release","--manifest-path",str(HERE/"Cargo.toml")],check=True)
    (HERE/"results").mkdir(exist_ok=True)
    execute(args.mode,args.config.resolve(),root,binary)


if __name__ == "__main__":
    try:
        main()
    except (OSError,ValueError,KeyError,TypeError,sqlite3.Error,subprocess.CalledProcessError) as exc:
        raise SystemExit(f"nerva-confirm-homeostasis: {exc}")
