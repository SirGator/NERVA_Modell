import copy
import importlib.util
import json
import shutil
import subprocess
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory

HERE = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("m10e_test_run", HERE/"run.py")
m = importlib.util.module_from_spec(spec); spec.loader.exec_module(m)


class CrossedTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.temp = TemporaryDirectory()
        cls.output = Path(cls.temp.name)/"output"
        cls.source = Path(cls.temp.name)/"source"
        source = m.PROJECT/"Messungen/Experimente/m9-v2-smoke"
        previous_source = m.PROJECT/"Messungen/Experimente/m10d-smoke"
        cls.source.mkdir()
        shutil.copy(source/"manifest.json", cls.source/"manifest.json")
        shutil.copytree(source/"results/weight4_tau100", cls.source/"results/weight4_tau100")
        proc = subprocess.run(["python3", str(HERE/"run.py"), "run", "--stage", "smoke",
            "--source", str(cls.source), "--recurrence-source", str(previous_source), "--output", str(cls.output)], capture_output=True, text=True)
        if proc.returncode:
            cls.temp.cleanup(); raise AssertionError(proc.stdout+proc.stderr)
        cls.wrapper = json.loads((cls.output/"traces/seed-692.json").read_text())
        cls.reference = json.loads((cls.source/"results/weight4_tau100/seed-692.json").read_text())
        cls.previous = json.loads((previous_source/"traces/seed-692.json").read_text())

    @classmethod
    def tearDownClass(cls):cls.temp.cleanup()

    def validate(self, wrapper):return m.core.validate(wrapper, self.reference, self.previous, m.D)

    def trial(self, wrapper, variant="reset_core__sham"):
        return wrapper["m9_result"]["arms"]["paired"]["normal_mapping"]["crossed_probes"]["stages"]["checkpoint_probe"][variant][0]

    def test_full_pilot_and_archived_m10d_reproduction(self):
        audit = json.loads((self.output/"diagnostics.json").read_text())
        self.assertEqual(audit["pairs"], 128)
        self.assertEqual(audit["windows"], 256)
        self.assertEqual(audit["phases"], 1024)
        self.assertTrue(audit["all_trained_branches_reproduce_m10d"])
        self.assertGreater(audit["negative_control_windows"], 0)
        self.assertEqual(audit["new_decoder_fits"], 0)

    def test_wrong_reset_list_is_rejected(self):
        bad = copy.deepcopy(self.wrapper)
        q = self.trial(bad)
        self.assertTrue(q["changed_weights"])
        q["changed_weights"][0][2] += .01
        with self.assertRaisesRegex(ValueError, "Core reset edits differ"):self.validate(bad)

    def test_wrong_reset_time_or_changed_start_state_is_rejected(self):
        bad = copy.deepcopy(self.wrapper)
        self.trial(bad)["weight_intervention"]["activation_us"] -= 1
        with self.assertRaisesRegex(ValueError, "weight reset time"):self.validate(bad)
        bad = copy.deepcopy(self.wrapper)
        self.trial(bad)["trace"][1]["potentials_before"][0] += .01
        with self.assertRaisesRegex(ValueError, "action start state"):self.validate(bad)

    def test_negative_control_and_fixed_prediction_are_checked(self):
        bad = copy.deepcopy(self.wrapper)
        self.trial(bad, "reset_core__block_both")["trace"][-1]["potentials_after"][0] += .01
        with self.assertRaisesRegex(ValueError, "negative control differs"):self.validate(bad)
        bad = copy.deepcopy(self.wrapper)
        self.trial(bad)["prediction_count"] *= -1
        with self.assertRaisesRegex(ValueError, "original prediction"):self.validate(bad)

    def test_altered_trained_branch_is_rejected_against_m10d(self):
        bad = copy.deepcopy(self.wrapper)
        self.trial(bad, "trained__sham")["trace"][-1]["potentials_after"][0] += .01
        with self.assertRaisesRegex(ValueError, "trained branch differs from M10-D"):self.validate(bad)

    def test_source_and_previous_trace_manifest_changes_are_rejected(self):
        path = self.source/"results/weight4_tau100/seed-692.json"
        original = path.read_bytes()
        try:
            path.write_bytes(original+b"\n")
            with self.assertRaisesRegex(ValueError, "source plans differ"):m.load(self.output)
        finally:path.write_bytes(original)
        path = self.output/"manifest.json"
        original = path.read_bytes()
        try:
            value = json.loads(original)
            value["recurrence_traces"]["traces/seed-692.json"] = "changed"
            path.write_text(json.dumps(value))
            with self.assertRaisesRegex(ValueError, "manifest/source differs"):m.load(self.output)
        finally:path.write_bytes(original)


if __name__ == "__main__":unittest.main()
