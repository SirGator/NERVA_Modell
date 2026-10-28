"""Independent candidate copies, native validation and effective-config identity."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys
from . import evaluator
from .history import atomic
from .mutation import canonical_json, normalize
from .spec import require


class InvalidCandidate(ValueError):pass


class NativeBackend:
    def __init__(self, binary, catalog_callback):
        self.binary = Path(binary)
        self.catalog_callback = catalog_callback

    def study_spec(self, parameters, seeds, name):
        return dict(experiment=dict(name=name, scenario="m0_selection", seeds=seeds, verify_replay=True, max_runs=4*len(seeds)),
                    base=parameters, measurements=dict(metrics=["sequence_score", "retained_sequence_score", "stability", "event_count"]))

    def canonical(self, parameters, spec):
        try:parameters = normalize(parameters)
        except (ValueError, OverflowError) as exc:raise InvalidCandidate(str(exc)) from exc
        source = self.study_spec(parameters, spec.training, "selection_identity")
        process = subprocess.run([str(self.binary), "plan"], input=canonical_json(source), capture_output=True, text=True)
        if process.returncode:raise InvalidCandidate(process.stderr.strip())
        plan = json.loads(process.stdout)
        effective = [job["effective_config"] for job in plan["jobs"][:4]]
        cid = "c"+hashlib.sha256(canonical_json(effective).encode()).hexdigest()
        return dict(id=cid, parameters=parameters, effective_config=effective)

    def identity(self):
        folder = Path(__file__).resolve().parent
        files = [folder.parent/"run.py", *sorted(folder.glob("*.py"))]
        return dict(binary_sha256=hashlib.sha256(self.binary.read_bytes()).hexdigest(),
                    python_version=sys.version, selection_sources={p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in files})

    def evaluate(self, candidate, seeds, root, phase, requirements, report_only=False):
        source = self.study_spec(candidate["parameters"], seeds, phase+"_"+candidate["id"])
        require(not report_only or (root/"manifest.json").exists(), "missing selection study: "+str(root))
        process = subprocess.run([str(self.binary), "report" if report_only else "run", "--output", str(root)],
                                 input=canonical_json(source), capture_output=True, text=True)
        if process.returncode:raise ValueError("selection study failed: "+process.stderr.strip())
        if not (root/"execution.log").exists():atomic(root/"execution.log", (process.stdout+process.stderr).encode())
        text = "\n".join(["[experiment]", *(f"{k} = {json.dumps(v)}" for k, v in source["experiment"].items()),
                           "", "[base]", *(json.dumps(k)+" = "+json.dumps(v) for k, v in sorted(source["base"].items())),
                           "", "[measurements]", "metrics = "+json.dumps(source["measurements"]["metrics"]), ""])
        if (root/"source.toml").exists():require((root/"source.toml").read_text() == text, "generated selection study source differs")
        else:atomic(root/"source.toml", text.encode())
        self.catalog_callback(root, self.binary)
        manifest_bytes = (root/"manifest.json").read_bytes()
        plan = json.loads(manifest_bytes)
        records = [json.loads((root/"records"/f"{job['id']}.json").read_bytes())["record"] for job in plan["jobs"]]
        assessment = evaluator.evaluate(records, seeds, requirements)
        assessment["study_manifest_sha256"] = hashlib.sha256(manifest_bytes).hexdigest()
        assessment["study_phase"] = phase
        print(f"STUDY {phase} {candidate['id'][:13]} eligible={assessment['eligible']} quality={assessment['quality']} cached={'COMPLETED' not in process.stdout}", flush=True)
        return assessment
