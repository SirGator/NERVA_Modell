import fcntl
import json
from pathlib import Path
import sqlite3
import subprocess
import sys
from tempfile import TemporaryDirectory
import unittest
import xml.etree.ElementTree as ET

HERE = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(HERE))
import harness
from contracts import ExperimentContext
from analysis.sensitivity import Sensitivity
from analysis.phase_mapping import PhaseMapping, state
from analysis.causal import Causal
from analysis.quality_diversity import QualityDiversity
from evolution import history
from planning.scheduler import prepare
from planning.spec import ResearchSpec
from test_evolution import FakeBackend, records


def result(parameters, score, seeds=(7, 19), eligible=True, quality=None):
    return dict(id=str(sorted(parameters.items())), parameters=parameters,
                evaluation=dict(eligible=eligible, quality=score if quality is None else quality, resource=100),
                observations=[dict(seed=r["job"]["seed"], intervention=r["job"]["intervention"], metrics=r["metrics"])
                              for r in records(seeds, score)])


def source():
    return dict(research=dict(name="fixture", training_seeds=[7, 19], validation_seeds=[151, 191, 229, 277, 307, 347], max_trials=400),
                base={"learning.a_plus": .04, "mechanisms.stdp": True, "mechanisms.homeostasis": True},
                studies=[dict(id="screen", method="sensitivity", options=dict(trajectories=2, axes=[
                    dict(name="learning.a_plus", min=.02, max=.08)])),
                         dict(id="mechanisms", method="causal")], validation=dict(bootstrap_samples=100))


class ResearchFixture(FakeBackend):
    def capabilities(self):
        return dict(metrics=["sequence_score", "retained_sequence_score", "firing_rate"])

    def evaluate(self, candidate, seeds, root, phase, requirements, report_only=False):
        evaluation = super().evaluate(candidate, seeds, root, phase, requirements, report_only)
        value = 2+candidate["parameters"]["learning.a_plus"]+(self.validation_shift if phase == "validate" else 0)
        return dict(result(candidate["parameters"], value, seeds), id=candidate["id"], evaluation=evaluation, records_sha256={})


class MethodTests(unittest.TestCase):
    def test_morris_recovers_linear_slopes_and_pairs_one_axis_steps(self):
        method = Sensitivity(dict(trajectories=8, levels=6, axes=[dict(name="x", min=0, max=1), dict(name="y", min=0, max=1)]))
        plan = method.propose(ExperimentContext("linear", {}, (7, 19)))
        self.assertEqual(len(plan.points), 8*3)
        results = [result(p, 2*p["x"]-3*p["y"]) for p in plan.points]
        report = method.analyze(results)
        self.assertAlmostEqual(report.findings["parameters"]["x"]["mu"], 2)
        self.assertAlmostEqual(report.findings["parameters"]["y"]["mu_star"], 3)
        self.assertAlmostEqual(report.findings["parameters"]["y"]["sigma"], 0)
        for edge in plan.design["edges"]:
            a, b = plan.points[edge["before"]], plan.points[edge["after"]]
            self.assertEqual([n for n in a if a[n] != b[n]], [edge["axis"]])
            self.assertAlmostEqual(abs(a[edge["axis"]]-b[edge["axis"]]), .6)
        self.assertEqual(plan, method.propose(ExperimentContext("linear", {}, (7, 19))))

    def test_factorial_causal_contrasts_recover_interaction(self):
        method = Causal({})
        plan = method.propose(ExperimentContext("causal", {}, (7, 19)))
        results = [result(p, 2*p["mechanisms.stdp"]+3*p["mechanisms.homeostasis"]
                          +4*p["mechanisms.stdp"]*p["mechanisms.homeostasis"]) for p in plan.points]
        report = method.analyze(results)
        self.assertEqual(report.findings["contrasts"]["stdp_without_homeostasis"]["mean"], 2)
        self.assertEqual(report.findings["contrasts"]["homeostasis_with_stdp"]["mean"], 7)
        self.assertEqual(report.findings["interaction"]["mean"], 4)
        self.assertEqual(report.status, "exploratory")

    def test_phase_map_preserves_seed_disagreement_and_threshold_states(self):
        method = PhaseMapping(dict(axes=[dict(name="x", values=[1]), dict(name="y", values=[2])]))
        plan = method.propose(ExperimentContext("map", {}, (7, 19)))
        r = result(plan.points[0], 3)
        r["observations"][4]["metrics"]["silent_fraction"] = 1
        self.assertEqual(method.analyze([r]).findings["cells"][0]["state"], "mixed")
        metrics = dict(silent_fraction=.1, firing_rate=201, persistent_fraction=0, stability=1)
        self.assertEqual(state(metrics, 200), "overactive")
        metrics.update(firing_rate=10, stability=0)
        self.assertEqual(state(metrics, 200), "unstable")

    def test_map_elites_gates_bounds_tiebreaks_and_archive_parent_mutation(self):
        method = QualityDiversity(dict(axes=[dict(name="x", values=[0, 1]), dict(name="y", values=[0, 1])]))
        method.propose(ExperimentContext("qd", {}, (7, 19)))
        parent = result(dict(x=0, y=0), 2)
        worse = result(dict(x=1, y=0), 2, quality=1)
        invalid = result(dict(x=0, y=1), 100, eligible=False)
        outside = result(dict(x=1, y=1), 5)
        archive, excluded = method.archive([parent, worse, invalid, outside])
        self.assertEqual(list(archive.values()), [parent])
        self.assertEqual({e["reason"] for e in excluded}, {"hard_requirements", "descriptor_out_of_range"})
        # From (0,0), index 1 is a one-axis neighbor; index 3 is two changes.
        self.assertEqual(method.next_index([parent], {1, 3}), 1)


class HarnessTests(unittest.TestCase):
    def test_budget_reserves_confirmation_and_defers_complete_low_priority_designs(self):
        s = source()
        s["research"]["max_trials"] = 56  # 8 baseline + 48 holdout jobs.
        manifest, _ = prepare(ResearchSpec(s), ResearchFixture())
        self.assertTrue(all(not study["scheduled"] for study in manifest["studies"]))
        self.assertEqual(manifest["maximum_trials"], 56)
        s["research"]["max_trials"] = 55
        with self.assertRaisesRegex(ValueError, "budget"):
            prepare(ResearchSpec(s), ResearchFixture())

    def test_freeze_before_holdout_integrity_resume_and_memory(self):
        with TemporaryDirectory() as temp:
            root = Path(temp)/"research"
            spec, backend = ResearchSpec(source()), ResearchFixture()
            final = harness.run(spec, root, backend, "fixture")
            self.assertEqual(final["validation_status"], "completed")
            self.assertEqual(backend.frozen, history.read(root/"frozen-finalist.json"))
            self.assertTrue(all(s == (spec.training if p == "train" else spec.validation) for p, s, _ in backend.new_evaluations))
            count = len(backend.new_evaluations)
            self.assertEqual(harness.run(spec, root, backend, "fixture", True), final)
            self.assertEqual(len(backend.new_evaluations), count)
            with sqlite3.connect(root/"research.sqlite") as db:
                self.assertEqual(db.execute("SELECT count(*) FROM reports").fetchone()[0], 2)
                self.assertEqual(db.execute("PRAGMA integrity_check").fetchone()[0], "ok")
                self.assertEqual(db.execute("SELECT DISTINCT status FROM hypotheses WHERE id LIKE 'study:%'").fetchall(), [("unresolved",)])
            path = root/"reports/mechanisms.json"
            envelope = json.loads(path.read_bytes())
            envelope["record"]["status"] = "confirmed"
            path.write_text(json.dumps(envelope))
            before = path.read_bytes()
            with self.assertRaisesRegex(ValueError, "checksum"):
                harness.run(spec, root, backend, "fixture", True)
            self.assertEqual(path.read_bytes(), before)

    def test_validation_outcomes_do_not_change_exploratory_reports(self):
        with TemporaryDirectory() as temp:
            roots = [Path(temp)/"a", Path(temp)/"b"]
            for root, shift in zip(roots, [0, -100]):
                harness.run(ResearchSpec(source()), root, ResearchFixture(shift), "fixture")
            self.assertEqual(history.read(roots[0]/"frozen-finalist.json"), history.read(roots[1]/"frozen-finalist.json"))
            self.assertEqual(history.read(roots[0]/"reports/screen.json"), history.read(roots[1]/"reports/screen.json"))

    def test_invalid_specs_and_metrics_fail_before_output(self):
        for case in range(4):
            s = source()
            if case == 0:
                s["research"]["validation_seeds"] = [7]
            if case == 1:
                s["studies"][0]["options"]["levels"] = 5
            if case == 2:
                s["studies"][0]["options"]["metric"] = "nonexistent"
            if case == 3:
                s["studies"][0]["options"]["unexpected"] = True
            with TemporaryDirectory() as temp:
                root = Path(temp)/"research"
                with self.assertRaises(ValueError):
                    harness.run(ResearchSpec(s), root, ResearchFixture(), "fixture")
                self.assertFalse(root.exists())

    def test_lock_foreign_directory_and_incomplete_report_are_rejected(self):
        with TemporaryDirectory() as temp:
            root = Path(temp)/"research"
            root.mkdir()
            with (root/".harness.lock").open("a+") as lock:
                fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
                with self.assertRaisesRegex(ValueError, "locked"):
                    harness.plan(ResearchSpec(source()), root, ResearchFixture(), "fixture")
            (root/"foreign.txt").write_text("keep")
            with self.assertRaisesRegex(ValueError, "nonempty"):
                harness.plan(ResearchSpec(source()), root, ResearchFixture(), "fixture")
            (root/"foreign.txt").unlink()
            harness.plan(ResearchSpec(source()), root, ResearchFixture(), "fixture")
            with self.assertRaisesRegex(ValueError, "completed"):
                harness.run(ResearchSpec(source()), root, ResearchFixture(), "fixture", True)


class NativeHarnessTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        subprocess.run(["cargo", "build", "--offline", "--manifest-path", str(HERE/"Cargo.toml")], check=True, capture_output=True)
        cls.binary = HERE/"target/debug/nerva-research"

    def invoke(self, mode, root, config=None):
        return subprocess.run([sys.executable, str(HERE/"run.py"), mode, str(config or HERE/"config/research_smoke.toml"),
                               "--binary", str(self.binary), "--output", str(root)], capture_output=True, text=True)

    def test_all_methods_real_runner_replay_report_and_corrupt_raw_record(self):
        with TemporaryDirectory() as temp:
            root = Path(temp)/"research"
            plan = self.invoke("research-plan", root)
            self.assertEqual(plan.returncode, 0, plan.stderr)
            self.assertFalse((root/"training").exists())
            manifest = history.read(root/"harness-manifest.json")
            self.assertTrue(all(s["scheduled"] for s in manifest["studies"]))
            run = self.invoke("research", root)
            self.assertEqual(run.returncode, 0, run.stdout+run.stderr)
            self.assertEqual(len(list((root/"reports").glob("*.json"))), 5)
            self.assertTrue((root/"maps/states.svg").exists())
            self.assertTrue((root/"maps/states.csv").exists())
            svg = ET.parse(root/"maps/states.svg").getroot()
            self.assertEqual(len(svg.findall(".//{http://www.w3.org/2000/svg}title")), 4)
            paths = list(root.glob("training/*/records/*.json"))+list(root.glob("validation/*/records/*.json"))
            before = {str(p): p.read_bytes() for p in paths}
            report = self.invoke("research-report", root)
            self.assertEqual(report.returncode, 0, report.stdout+report.stderr)
            self.assertEqual(before, {str(p): p.read_bytes() for p in paths})
            path = paths[0]
            data = json.loads(path.read_bytes())
            data["record"]["metrics"]["sequence_score"] = 999
            path.write_text(json.dumps(data))
            corrupt = self.invoke("research-report", root)
            self.assertNotEqual(corrupt.returncode, 0)
            self.assertIn("integrity", corrupt.stderr)

    def test_invalid_late_design_does_not_start_baseline(self):
        with TemporaryDirectory() as temp:
            config = Path(temp)/"bad.toml"
            config.write_text((HERE/"config/research_smoke.toml").read_text().replace("values = [1.15, 1.25]", "values = [1.15, 99.0]"))
            root = Path(temp)/"research"
            process = self.invoke("research", root, config)
            self.assertNotEqual(process.returncode, 0)
            self.assertFalse(root.exists())


if __name__ == "__main__":
    unittest.main()
