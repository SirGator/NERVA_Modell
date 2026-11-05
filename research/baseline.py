#!/usr/bin/env python3
"""Run or verify the fixed M0 reference and seal its records with SHA-256."""
import argparse
import hashlib
import json
from pathlib import Path
import sqlite3
import subprocess
import sys
import tomllib

from evolution.history import immutable, read

HERE = Path(__file__).resolve().parent
CONFIG = HERE / "config/phase1_baseline.toml"
RECEIPT = "baseline-receipt.json"


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def record_hashes(root):
    return {p.name: sha256(p) for p in sorted((root / "records").glob("*.json"))}


def check(condition, message):
    if not condition:
        raise ValueError(message)


def describe(root, binary):
    """Called only after the native report has checked all records and replay flags."""
    manifest = json.loads((root / "manifest.json").read_text())
    expected = tomllib.loads(CONFIG.read_text())
    check(manifest["spec"] == expected, "archive is not the fixed baseline protocol")
    hashes = record_hashes(root)
    jobs = manifest["jobs"]
    check(set(hashes) == {job["id"] + ".json" for job in jobs}, "baseline record set differs")
    metrics = expected["measurements"]["metrics"]
    for job in jobs:
        record = json.loads((root / "records" / (job["id"] + ".json")).read_text())["record"]
        check(record["full_replay_checked"] is True, "baseline needs full replay")
        check(set(record["metrics"]) == set(metrics), "baseline metric contract differs")
    with sqlite3.connect(root / "results.sqlite") as db:
        check(db.execute("PRAGMA integrity_check").fetchone()[0] == "ok", "SQLite integrity failure")
        check(db.execute("SELECT count(*) FROM trials").fetchone()[0] == len(jobs), "SQLite trial count differs")
    return dict(
        protocol="NERVA-M0-baseline-v1",
        config_sha256=sha256(CONFIG),
        manifest_sha256=sha256(root / "manifest.json"),
        source_sha256=sha256(root / "source.toml"),
        binary_sha256=sha256(binary),
        engine_build_fnv1a64=manifest["engine_build_fnv1a64"],
        verifier_sha256=sha256(Path(__file__)),
        records_sha256=hashes,
        counts=manifest["counts"],
        metrics=metrics,
        decision=json.loads((root / "summary.json").read_text())["variants"][0]["decision"],
        scope="Unchanged nine-neuron M0 reference on reused regression seeds. Not a sensorimotor model baseline or independent confirmation.",
        verification="Native report verified all records; SHA-256 unchanged after report; SQLite integrity_check=ok.",
    )


def execute(mode, root, binary):
    receipt = root / RECEIPT
    if mode == "verify":
        check(receipt.is_file(), "verify needs an existing baseline receipt")
    sealed = None
    if receipt.exists():
        sealed = read(receipt)
        # Fail before rebuilding derived outputs if an already sealed record changed.
        check(record_hashes(root) == sealed["records_sha256"], "sealed baseline records differ")
        check(sha256(root / "manifest.json") == sealed["manifest_sha256"], "sealed manifest differs")
        check(sha256(root / "source.toml") == sealed["source_sha256"], "sealed source differs")
        check(sha256(binary) == sealed["binary_sha256"], "sealed binary differs")
        check(sha256(CONFIG) == sealed["config_sha256"], "sealed config differs")
        check(sha256(Path(__file__)) == sealed["verifier_sha256"], "sealed verifier differs")
    command = [sys.executable, str(HERE / "run.py")]
    options = [str(CONFIG), "--binary", str(binary), "--output", str(root)]
    if mode == "run" and sealed is None:
        subprocess.run(command + ["run"] + options, check=True)
    before = record_hashes(root)
    # Report cannot start missing trials and validates the current native build.
    subprocess.run(command + ["report"] + options, check=True)
    check(before == record_hashes(root), "report modified baseline records")
    description = describe(root, binary)
    if mode == "verify":
        check(description == sealed, "baseline receipt differs")
    # immutable also validates the SHA-256 envelope when the receipt exists.
    immutable(receipt, description)
    return description


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=["run", "verify"])
    parser.add_argument("--output", type=Path, default=HERE / "results/phase1_m0_baseline_20261105")
    parser.add_argument("--binary", type=Path)
    args = parser.parse_args()
    binary = args.binary.resolve() if args.binary else HERE / "target/release/nerva-research"
    if args.binary is None:
        subprocess.run(["cargo", "build", "--offline", "--release", "--manifest-path", str(HERE / "Cargo.toml")], check=True)
    result = execute(args.mode, args.output.resolve(), binary)
    print("BASELINE", args.output / RECEIPT, "trials=", result["counts"]["trials"])


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, KeyError, TypeError, subprocess.CalledProcessError, sqlite3.Error) as exc:
        raise SystemExit(f"nerva-baseline: {exc}")
