import copy
import json
from pathlib import Path
import sqlite3
import subprocess
import sys
from tempfile import TemporaryDirectory
import unittest

HERE = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(HERE))
import control


def rows():
    result = []
    for seed in [11, 19]:
        for group in control.GROUPS:
            metrics = {name: 0. for name in control.METRICS}
            metrics.update(matched_checkpoints=1., sensory_spikes=10.,
                           sensor_to_core_arrivals=10., training_core_spikes=5.)
            details = dict(initial_network_digest="shared",
                           matched_checkpoint=True, only_weights_changed=True,
                           training=dict(input_digest="same", audit=dict(complete=True, tag_events=0,
                                                                         chemical_samples=0)))
            result.append(dict(job=dict(seed=seed, intervention=group), metrics=metrics,
                               details=details, full_replay_checked=True))
    return result


class ControlPolicyTests(unittest.TestCase):
    def test_budgets_cover_full_design_reconstructions_and_replay(self):
        source = (HERE/"config/hippocampal_control_smoke.toml").read_text()
        spec = control.parse(source)
        self.assertEqual(spec["control"]["max_native_training_passes"], 64)
        for before, after in [('max_trials = 16', 'max_trials = 15'),
                              ('max_native_training_passes = 64', 'max_native_training_passes = 63'),
                              ('workers = 2', 'workers = 9'),
                              ('seeds = [7, 19]', 'seeds = [7, 7]'),
                              ('processing_neurons = 16', 'processing_neurons = true')]:
            with self.subTest(after=after), self.assertRaises(ValueError):
                control.parse(source.replace(before, after))
        with self.assertRaises(ValueError):
            control.parse(source+"\nunknown = 1\n")

    def test_silence_ties_incomplete_controls_and_unmatched_starts(self):
        report = control.analyze(rows())
        self.assertEqual(len(report["priorities"]), 8)
        self.assertFalse(report["functional_memory_demonstrated"])
        self.assertTrue(all(c["p_one_sided"] == 1. for c in report["priorities"]))
        self.assertTrue(all(not c["exploratory_threshold_met"] for c in report["priorities"]))
        for problem in ["missing", "duplicate", "initial", "input", "replay", "audit"]:
            broken = copy.deepcopy(rows())
            if problem == "missing":
                broken.pop()
            elif problem == "duplicate":
                broken.append(copy.deepcopy(broken[0]))
            elif problem == "initial":
                broken[0]["details"]["initial_network_digest"] = "different"
            elif problem == "input":
                broken[0]["details"]["training"]["input_digest"] = "different"
            elif problem == "replay":
                broken[0]["full_replay_checked"] = False
            else:
                broken[0]["details"]["training"]["audit"]["complete"] = False
            with self.subTest(problem=problem), self.assertRaises(ValueError):
                control.analyze(broken)

    def test_bottleneck_hints_separate_rounding_from_absent_factor(self):
        row = rows()[0]
        row["metrics"].update(requested_updates=4., requests_without_change=4.,
                              zero_factor_integrations=7., weight_l1=.1)
        audit = dict(tag_events=4, chemical_samples=8)
        hints = control.hints(row["metrics"], audit)
        self.assertIn("nonzero_requests_without_actual_change", hints)
        self.assertNotIn("eligibility_without_nonzero_weight_request", hints)
        self.assertIn("net_weight_changes_without_core_spike_effect_in_recall", hints)
        self.assertIn("no_nonzero_motor_factor_observed", hints)

    def test_positive_and_negative_directions_use_paired_seed_differences(self):
        positive = []
        template = rows()[:len(control.GROUPS)]
        for seed in range(16):
            for original in template:
                row = copy.deepcopy(original)
                row["job"]["seed"] = seed
                if row["job"]["intervention"] == "three_on":
                    row["metrics"].update(weight_gain_a=.2, weight_gain_b=.2)
                positive.append(row)
        report = control.analyze(positive)
        self.assertTrue(all(c["exploratory_threshold_met"] for c in report["priorities"]))
        self.assertFalse(report["independent_confirmation"])
        for row in positive:
            if row["job"]["intervention"] == "three_on":
                row["metrics"].update(weight_gain_a=-.2, weight_gain_b=-.2)
        self.assertTrue(all(not c["exploratory_threshold_met"] for c in control.analyze(positive)["priorities"]))

    def test_wall_deadline_and_output_lock_are_enforced(self):
        with self.assertRaises(ValueError):
            control.remaining(0)
        with TemporaryDirectory() as directory:
            root = Path(directory)
            with control.locked(root), self.assertRaises(ValueError):
                with control.locked(root):
                    pass


class NativeControlTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        subprocess.run(["cargo", "build", "--offline", "--release", "--manifest-path",
                        str(HERE/"Cargo.toml")], check=True, capture_output=True)
        cls.binary = HERE/"target/release/nerva-research"

    def test_partial_resume_report_raw_hashes_and_archived_producer(self):
        source = (HERE/"config/hippocampal_control_smoke.toml").read_text()
        spec = control.parse(source)
        with TemporaryDirectory() as directory:
            root = Path(directory)/"control"
            with control.locked(root):
                plan = control.prepare(root, spec, source, self.binary)
            self.assertEqual(plan["counts"]["native_training_passes"], 64)
            self.assertFalse((root/"seeds").exists(), "plan must not simulate")
            self.assertFalse((root/"receipt.json").exists())
            seed = spec["control"]["seeds"][0]
            control.native(root/"runner", "run", control.study(spec, seed), control.seed_root(root, seed))
            records = control.seed_root(root, seed)/"records"
            missing = next(records.glob("*.json"))
            missing.unlink()  # Interrupted archive, without a completed receipt.
            existing = {p: p.read_bytes() for p in records.glob("*.json")}
            with control.locked(root):
                control.execute(root, control.load_plan(root, source))
            for path, content in existing.items():
                self.assertEqual(path.read_bytes(), content, "resume overwrote a complete record")
            self.assertTrue(missing.exists())
            receipt = (root/"receipt.json").read_bytes()
            with control.locked(root):
                control.execute(root, control.load_plan(root, source), report_only=True)
            self.assertEqual((root/"receipt.json").read_bytes(), receipt)
            with sqlite3.connect(root/"control.sqlite") as db:
                self.assertEqual(db.execute("SELECT COUNT(*) FROM trials").fetchone()[0], 16)
                self.assertEqual(db.execute("PRAGMA integrity_check").fetchone()[0], "ok")
            producer = subprocess.run([sys.executable, str(root/"producer.py"), "status",
                                       "--output", str(root)], capture_output=True, text=True)
            self.assertEqual(producer.returncode, 0, producer.stderr)
            self.assertEqual(json.loads(producer.stdout)["cached_records"], 16)
            with self.assertRaises(ValueError):
                control.load_plan(root, source+"\n")
            missing.write_bytes(missing.read_bytes()+b"\n")
            with self.assertRaisesRegex(ValueError, "raw record differs"):
                control.load_plan(root, source)

    def test_foreign_output_and_broken_native_records_are_rejected(self):
        source = (HERE/"config/hippocampal_control_smoke.toml").read_text()
        spec = control.parse(source)
        with TemporaryDirectory() as directory:
            root = Path(directory)
            (root/"foreign.txt").write_text("keep")
            with control.locked(root), self.assertRaises(ValueError):
                control.prepare(root, spec, source, self.binary)
            self.assertEqual((root/"foreign.txt").read_text(), "keep")
            target = root/"native"
            control.native(self.binary, "run", control.study(spec, 7), target)
            path = next((target/"records").glob("*.json"))
            record = json.loads(path.read_bytes())
            record["record"]["metrics"]["weight_gain_a"] = 999.
            path.write_text(json.dumps(record))
            with self.assertRaisesRegex(ValueError, "integrity"):
                control.native(self.binary, "report", control.study(spec, 7), target)
