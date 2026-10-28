#!/usr/bin/env python3
"""TOML frontend and SQLite catalog for the independent Rust research runner."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import sqlite3
import subprocess
import sys
import tempfile

try:
    import tomllib
except ImportError:
    raise SystemExit("nerva-research requires Python 3.11 or newer (tomllib)")

HERE = Path(__file__).resolve().parent


def atomic_text(path, content):
    with tempfile.NamedTemporaryFile(mode="w", encoding="utf-8", dir=path.parent, delete=False) as stream:
        temporary = Path(stream.name)
        stream.write(content); stream.flush(); os.fsync(stream.fileno())
    try:os.replace(temporary, path)
    finally:temporary.unlink(missing_ok=True)


def catalog(root, binary):
    manifest_bytes = (root/"manifest.json").read_bytes()
    manifest = json.loads(manifest_bytes)
    summary = json.loads((root/"summary.json").read_text())
    handle, filename = tempfile.mkstemp(prefix="catalog-", suffix=".sqlite", dir=root)
    os.close(handle)
    temporary = Path(filename)
    try:
        with sqlite3.connect(temporary) as db:
            db.execute("PRAGMA foreign_keys=ON")
            db.executescript("""
                CREATE TABLE metadata (key TEXT PRIMARY KEY, value TEXT NOT NULL);
                CREATE TABLE variants (id TEXT PRIMARY KEY, parameters TEXT NOT NULL, decision TEXT NOT NULL);
                CREATE TABLE trials (id TEXT PRIMARY KEY, variant_id TEXT NOT NULL REFERENCES variants(id),
                    seed TEXT NOT NULL, intervention TEXT NOT NULL, effective_config TEXT NOT NULL,
                    details TEXT NOT NULL, full_replay_checked INTEGER NOT NULL,
                    wall_time_seconds REAL NOT NULL, raw_sha256 TEXT NOT NULL);
                CREATE TABLE metrics (trial_id TEXT NOT NULL REFERENCES trials(id), name TEXT NOT NULL,
                    value REAL NOT NULL, PRIMARY KEY(trial_id,name));
                CREATE INDEX metric_lookup ON metrics(name,value);
                CREATE INDEX trial_lookup ON trials(variant_id,seed,intervention);
            """)
            provenance = dict(
                manifest_sha256=hashlib.sha256(manifest_bytes).hexdigest(),
                source_sha256=hashlib.sha256((root/"source.toml").read_bytes()).hexdigest(),
                frontend_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),
                python_version=sys.version.split()[0],
                statement="Local result catalog; no automatic selection or independent confirmation")
            for key, value in dict(manifest=manifest, catalog_provenance=provenance).items():
                db.execute("INSERT INTO metadata VALUES (?,?)", (key, json.dumps(value, sort_keys=True)))
            decisions = {v["id"]: v["decision"] for v in summary["variants"]}
            for variant in manifest["variants"]:
                db.execute("INSERT INTO variants VALUES (?,?,?)", (variant["id"], json.dumps(variant["parameters"], sort_keys=True), json.dumps(decisions[variant["id"]], sort_keys=True)))
            for job in manifest["jobs"]:
                raw = (root/"records"/f"{job['id']}.json").read_bytes()
                record = json.loads(raw)["record"]
                if record["job"] != job:raise ValueError("catalog job identity differs")
                db.execute("INSERT INTO trials VALUES (?,?,?,?,?,?,?,?,?)", (
                    job["id"], job["variant"], str(job["seed"]), job["intervention"],
                    json.dumps(job["effective_config"], sort_keys=True), json.dumps(record["details"], sort_keys=True),
                    int(record["full_replay_checked"]), record["wall_time_seconds"], hashlib.sha256(raw).hexdigest()))
                db.executemany("INSERT INTO metrics VALUES (?,?,?)", [(job["id"], name, value) for name, value in record["metrics"].items()])
            if db.execute("PRAGMA integrity_check").fetchone()[0] != "ok":raise ValueError("SQLite integrity check failed")
        os.replace(temporary, root/"results.sqlite")
        atomic_text(root/"catalog-provenance.json", json.dumps(provenance, indent=2, sort_keys=True)+"\n")
    finally:temporary.unlink(missing_ok=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=["plan", "run", "report", "list", "select-plan", "select", "selection-report"])
    parser.add_argument("config", type=Path, nargs="?")
    parser.add_argument("--output", type=Path)
    parser.add_argument("--binary", type=Path, help="Use an explicitly supplied research binary instead of building")
    args = parser.parse_args()
    if args.mode != "list" and args.config is None:parser.error("a TOML config is required")
    if args.mode == "list" and (args.config is not None or args.output is not None):parser.error("list accepts no config/output")
    selection_mode = args.mode in ["select-plan", "select", "selection-report"]
    if args.mode != "list":
        source_text = args.config.read_text(encoding="utf-8")
        spec = tomllib.loads(source_text)
        payload = json.dumps(spec, allow_nan=False)
        # Rust validates identifiers and all variants before creating the directory.
        experiment = spec.get("selection" if selection_mode else "experiment", {})
        default_name = experiment.get("name", "unnamed") if isinstance(experiment, dict) else "unnamed"
        output = args.output or HERE/"results"/str(default_name)
        archived = output/"source.toml"
        if archived.exists() and tomllib.loads(archived.read_text()) != spec:
            raise ValueError("archived TOML differs from the requested input")
    if selection_mode:
        from evolution.spec import SelectionSpec
        selection_spec = SelectionSpec(spec)
    if args.binary is None:
        subprocess.run(["cargo", "build", "--offline", "--release", "--manifest-path", str(HERE/"Cargo.toml")], check=True)
        binary = HERE/"target/release/nerva-research"
    else:binary = args.binary.resolve()
    if args.mode == "list":
        subprocess.run([str(binary), "list"], check=True); return
    if selection_mode:
        from evolution import generation, history
        from evolution.population import NativeBackend
        backend = NativeBackend(binary, catalog)
        if args.mode == "select-plan":
            plan = generation.plan(selection_spec, backend)
            manifest = generation.make_manifest(selection_spec, backend, plan["baseline"])
            with history.output(output, manifest, False):
                if not (output/"source.toml").exists():atomic_text(output/"source.toml", source_text)
                history.immutable(output/"selection-plan.json", plan)
            print(json.dumps(plan, indent=2, sort_keys=True))
        else:
            result = generation.run(selection_spec, output, backend, source_text, args.mode == "selection-report")
            print("SELECTION", output/"selection-summary.md", "held_out_quality_improved=", result["held_out_quality_improved"])
        return
    command = [str(binary), args.mode, "--output", str(output)]
    if args.mode == "plan":
        process = subprocess.run(command, input=payload, text=True, capture_output=True)
        if process.returncode:raise ValueError(process.stderr.strip())
        plan = json.loads(process.stdout)
        counts = plan["counts"]
        print(f"PLAN {counts['variants']} variants × {counts['seeds']} seeds × {counts['interventions']} controls = {counts['trials']} trials ({counts['full_executions']} executions with replay)")
    else:subprocess.run(command, input=payload, text=True, check=True)
    # Native manifests bind normalized input; preserve the first TOML source text.
    source_path = output/"source.toml"
    if source_path.exists():
        if tomllib.loads(source_path.read_text()) != spec:raise ValueError("archived TOML differs from the manifest input")
    else:atomic_text(source_path, source_text)
    if args.mode != "plan":
        catalog(output, binary)
        print("DATABASE", output/"results.sqlite")
    else:print("MANIFEST", output/"manifest.json")


if __name__ == "__main__":
    try:main()
    except (OSError, ValueError, TypeError, subprocess.CalledProcessError, sqlite3.Error) as exc:
        raise SystemExit(f"nerva-research: {exc}")
