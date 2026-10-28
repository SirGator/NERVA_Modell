import contextlib
import fcntl
import hashlib
import json
import os
from pathlib import Path
import sqlite3
import tempfile
from .mutation import canonical_json
from .spec import require


def atomic(path, data):
    path.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.NamedTemporaryFile(dir=path.parent, delete=False) as stream:
        temporary = Path(stream.name); stream.write(data); stream.flush(); os.fsync(stream.fileno())
    try:os.replace(temporary, path)
    finally:temporary.unlink(missing_ok=True)


def read(path):
    envelope = json.loads(path.read_bytes())
    require(hashlib.sha256(canonical_json(envelope["record"]).encode()).hexdigest() == envelope["sha256"], "selection history checksum differs: "+str(path))
    return envelope["record"]


def immutable(path, record):
    if path.exists():require(read(path) == record, "selection history differs: "+str(path))
    else:atomic(path, (json.dumps(dict(sha256=hashlib.sha256(canonical_json(record).encode()).hexdigest(), record=record), indent=2, sort_keys=True)+"\n").encode())


@contextlib.contextmanager
def output(root, manifest, report_only):
    if report_only:require((root/"selection-manifest.json").exists(), "selection-report requires an existing search")
    root.mkdir(parents=True, exist_ok=True)
    with (root/".selection.lock").open("a+") as lock:
        try:fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:raise ValueError("selection output is locked") from None
        path = root/"selection-manifest.json"
        if not path.exists():
            require(not any(p.name != ".selection.lock" for p in root.iterdir()), "nonempty selection output has no matching manifest")
        immutable(path, manifest)
        yield


def catalog(root, candidates, generations, final=None):
    descriptor, name = tempfile.mkstemp(prefix="selection-", suffix=".sqlite", dir=root); os.close(descriptor)
    temporary = Path(name)
    try:
        with sqlite3.connect(temporary) as db:
            db.execute("PRAGMA foreign_keys=ON")
            db.executescript("""
                CREATE TABLE metadata(key TEXT PRIMARY KEY,value TEXT NOT NULL);
                CREATE TABLE candidates(id TEXT PRIMARY KEY,parameters TEXT NOT NULL,effective_config TEXT NOT NULL,evaluation TEXT);
                CREATE TABLE generations(number INTEGER PRIMARY KEY,record TEXT NOT NULL);
                CREATE TABLE lineage(generation INTEGER NOT NULL,parent TEXT,child TEXT,mutation TEXT,status TEXT NOT NULL);
                CREATE TABLE elites(generation INTEGER NOT NULL,candidate_id TEXT NOT NULL REFERENCES candidates(id),position INTEGER NOT NULL,PRIMARY KEY(generation,candidate_id));
                CREATE TABLE validation(role TEXT PRIMARY KEY,candidate_id TEXT NOT NULL REFERENCES candidates(id),evaluation TEXT NOT NULL);
            """)
            db.execute("INSERT INTO metadata VALUES (?,?)", ("manifest", canonical_json(read(root/"selection-manifest.json"))))
            for candidate in candidates.values():
                db.execute("INSERT INTO candidates VALUES (?,?,?,?)", (candidate["id"], canonical_json(candidate["parameters"]), canonical_json(candidate["effective_config"]), canonical_json(candidate.get("evaluation"))))
            for generation in generations:
                db.execute("INSERT INTO generations VALUES (?,?)", (generation["number"], canonical_json(generation)))
                for edge in generation["proposals"]:
                    db.execute("INSERT INTO lineage VALUES (?,?,?,?,?)", (generation["number"], edge.get("parent"), edge.get("child"), canonical_json(edge.get("mutation")), edge["status"]))
                for i, cid in enumerate(generation["elites"]):db.execute("INSERT INTO elites VALUES (?,?,?)", (generation["number"], cid, i))
            if final:
                db.execute("INSERT INTO metadata VALUES (?,?)", ("final", canonical_json(final)))
                for role in ["candidate", "baseline"]:
                    if role in final:db.execute("INSERT INTO validation VALUES (?,?,?)", (role, final[role]["id"], canonical_json(final[role]["evaluation"])))
            require(db.execute("PRAGMA integrity_check").fetchone()[0] == "ok", "selection SQLite integrity failure")
        os.replace(temporary, root/"selection.sqlite")
    finally:temporary.unlink(missing_ok=True)
