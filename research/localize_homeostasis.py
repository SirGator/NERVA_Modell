#!/usr/bin/env python3
"""Preregister, run/resume and verify the population-localization protocol."""
import argparse
import contextlib
import fcntl
import json
from pathlib import Path
import sqlite3
import subprocess
import tomllib

import confirm_homeostasis as common
from analysis.homeostasis_populations import analyze, protocol
from evolution import history
from evolution.spec import require

HERE = Path(__file__).resolve().parent


def identity(binary):
    paths = [Path(__file__),HERE/"analysis/homeostasis_populations.py",HERE/"confirm_homeostasis.py",
             HERE/"analysis/homeostasis_confirmation.py",HERE/"run.py",HERE/"planning/profiles.py",
             HERE/"evolution/history.py",HERE/"evolution/mutation.py",HERE/"evolution/spec.py"]
    return dict(binary_sha256=common.sha(binary),sources_sha256={str(p.relative_to(HERE)):common.sha(p) for p in paths})


def execute(mode, config, root, binary, audit_roots=None):
    source = tomllib.loads(config.read_text())
    p = protocol(source)
    code = identity(binary)
    registration = root/"preregistration.json"
    if mode == "report": require(registration.exists(),"population report requires preregistration")
    root.parent.mkdir(parents=True,exist_ok=True)
    with (root.parent/("."+root.name+".confirmation.lock")).open("a+") as lock:
        try: fcntl.flock(lock,fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError: raise ValueError("population output is locked") from None
        if registration.exists():
            registered = history.read(registration)
            require(registered["protocol"] == p and registered["identity"] == code
                    and registered["config_sha256"] == common.sha(config),"population protocol, source or binary differs")
            require(registered["manifest_sha256"] == common.sha(root/"manifest.json")
                    and registered["archived_source_sha256"] == common.sha(root/"source.toml"),"population archive differs")
        else:
            require(not common.record_hashes(root),"cannot preregister population study after trials")
            with (HERE/"results/.confirmation-seeds.lock").open("a+") as seed_lock:
                fcntl.flock(seed_lock,fcntl.LOCK_EX | fcntl.LOCK_NB)
                audit = common.audit_seeds(p["seeds"],root,config,audit_roots)
                common.native("plan",config,root,binary)
                require(not common.record_hashes(root),"population preregistration requires zero records")
                history.immutable(registration,dict(protocol=p,identity=code,config_sha256=common.sha(config),
                    manifest_sha256=common.sha(root/"manifest.json"),archived_source_sha256=common.sha(root/"source.toml"),
                    records_at_registration=0,seed_audit=audit))
        if mode == "plan":
            print("PREREGISTERED",registration,"family_tests=",p["family_tests"],"per_test_alpha=",p["per_test_alpha"])
            return p
        receipt = root/"verification-receipt.json"
        if receipt.exists(): require(common.record_hashes(root) == history.read(receipt)["records_sha256"],"sealed population records differ")
        if mode == "run": common.native("run",config,root,binary)
        before = common.record_hashes(root)
        common.native("report",config,root,binary)
        require(before == common.record_hashes(root),"population report changed records")
        manifest = json.loads((root/"manifest.json").read_bytes())
        require(set(before) == {j["id"]+".json" for j in manifest["jobs"]},"unexpected population record files")
        records = [json.loads((root/"records"/(j["id"]+".json")).read_bytes())["record"] for j in manifest["jobs"]]
        result = analyze(records,source)
        history.immutable(root/"population-result.json",result)
        with contextlib.closing(sqlite3.connect(root/"results.sqlite")) as db:
            require(db.execute("PRAGMA integrity_check").fetchone()[0] == "ok","population SQLite integrity failure")
            require(db.execute("SELECT count(*) FROM trials").fetchone()[0] == len(records),"population catalog count differs")
        history.immutable(receipt,dict(records_sha256=before,record_count=len(records),new_trials_during_report=0,
            raw_records_unchanged=True,sqlite_integrity="ok",identity=code,
            preregistration_sha256=common.sha(registration),analysis_sha256=common.sha(root/"population-result.json")))
        lines = ["# Homeostasis population localization","",
            f'{len(p["seeds"])} paired seeds; {len(records)} trials; {p["family_tests"]} primary tests; alpha/test={p["per_test_alpha"]}.',"",
            "| Core | Scope | Visual accuracy | Position MAE | Sensor rate Hz | Core rate Hz | Sensor current | Core current |",
            "|---:|---|---:|---:|---:|---:|---:|---:|"]
        for cell in result["cells"]:
            for scope,m in cell["means"].items():
                lines.append(f'| {cell["processing_neurons"]} | {scope} | {m["visual_accuracy"]:.4f} | {m["position_mae"]:.4f} | {m["sensor_rate_hz"]:.4f} | {m["core_rate_hz"]:.4f} | {m["sensor_intrinsic_current"]:.4f} | {m["core_intrinsic_current"]:.4f} |')
        lines += ["",*result["limits"],""]
        history.atomic(root/"population-summary.md","\n".join(lines).encode())
        print("POPULATION RESULT",root/"population-summary.md","verified_records=",len(records))
        return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode",choices=["plan","run","report"])
    parser.add_argument("--config",type=Path,default=HERE/"config/homeostasis_populations.toml")
    parser.add_argument("--output",type=Path)
    parser.add_argument("--binary",type=Path)
    args = parser.parse_args()
    source = tomllib.loads(args.config.read_text())
    protocol(source)
    root = (args.output or HERE/"results"/source["experiment"]["name"]).resolve()
    binary = args.binary.resolve() if args.binary else HERE/"target/release/nerva-research"
    if args.binary is None: subprocess.run(["cargo","build","--offline","--release","--manifest-path",str(HERE/"Cargo.toml")],check=True)
    (HERE/"results").mkdir(exist_ok=True)
    execute(args.mode,args.config.resolve(),root,binary)


if __name__ == "__main__":
    try: main()
    except (OSError,ValueError,KeyError,TypeError,sqlite3.Error,subprocess.CalledProcessError) as exc:
        raise SystemExit(f"nerva-localize-homeostasis: {exc}")
