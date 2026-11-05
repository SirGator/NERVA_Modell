"""Reuse native studies and their independent hard-gate evaluator."""
import hashlib
import json
from pathlib import Path
import struct
import subprocess
from evolution.population import NativeBackend, InvalidCandidate
from evolution.spec import require
from .profiles import profile


class ResearchBackend(NativeBackend):
    def __init__(self, binary, catalog_callback, scenario="m0_selection"):
        super().__init__(binary, catalog_callback)
        self.scenario = scenario
        self.profile = profile(scenario)

    def study_spec(self, parameters, seeds, name):
        return dict(experiment=dict(name=name, scenario=self.scenario, seeds=seeds,
                                   interventions=list(self.profile.groups), verify_replay=True,
                                   max_runs=len(self.profile.groups)*len(seeds)),
                    base=parameters, measurements=dict(metrics=list(self.profile.study_metrics)))

    def assess(self, records, seeds, requirements):
        return self.profile.assess(records, seeds, requirements)

    def capabilities(self):
        process = subprocess.run([str(self.binary), "list"], capture_output=True, text=True)
        require(process.returncode == 0, "cannot inspect native scenarios: "+process.stderr)
        return next(s for s in json.loads(process.stdout)["scenarios"] if s["name"] == self.scenario)

    def canonical(self, parameters, spec):
        # Native numeric parameters use f32; preserve integers and booleans.
        try:
            parameters = {k: struct.unpack("f", struct.pack("f", v))[0] if type(v) is float else v
                          for k, v in parameters.items()}
        except (OverflowError, struct.error) as exc:
            raise InvalidCandidate(str(exc)) from exc
        return super().canonical(parameters, spec)

    def identity(self):
        identity = super().identity()
        root = Path(__file__).resolve().parents[1]
        files = [root/"contracts.py", root/"harness.py"]
        for folder in ["analysis", "planning", "research_memory"]:
            files.extend(sorted((root/folder).glob("*.py")))
        identity["research_sources"] = {str(p.relative_to(root)): hashlib.sha256(p.read_bytes()).hexdigest()
                                        for p in files}
        return identity

    def evaluate(self, candidate, seeds, root, phase, requirements, report_only=False):
        evaluation = super().evaluate(candidate, seeds, root, phase, requirements, report_only)
        manifest = json.loads((root/"manifest.json").read_bytes())
        observations = []
        hashes = {}
        for job in manifest["jobs"]:
            raw = (root/"records"/f"{job['id']}.json").read_bytes()
            record = json.loads(raw)["record"]
            observations.append(dict(seed=job["seed"], intervention=job["intervention"], metrics=record["metrics"]))
            hashes[job["id"]] = hashlib.sha256(raw).hexdigest()
        return dict(id=candidate["id"], parameters=candidate["parameters"], evaluation=evaluation,
                    observations=observations, records_sha256=hashes)
