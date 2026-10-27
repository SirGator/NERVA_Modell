import copy
import importlib.util
import json
import shutil
import subprocess
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory

HERE = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("m10d_run_test", HERE/"run.py")
m = importlib.util.module_from_spec(spec); spec.loader.exec_module(m)


class InterventionTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.temp = TemporaryDirectory()
        cls.output = Path(cls.temp.name)/"output"
        cls.source = Path(cls.temp.name)/"source"
        source = m.PROJECT/"Messungen/Experimente/m9-v2-smoke"
        cls.source.mkdir()
        shutil.copy(source/"manifest.json", cls.source/"manifest.json")
        shutil.copytree(source/"results/weight4_tau100", cls.source/"results/weight4_tau100")
        proc = subprocess.run(["python3", str(HERE/"run.py"), "run", "--stage", "smoke",
            "--source", str(cls.source), "--output", str(cls.output)], capture_output=True, text=True)
        if proc.returncode:
            cls.temp.cleanup(); raise AssertionError(proc.stdout+proc.stderr)
        cls.wrapper = json.loads((cls.output/"traces/seed-692.json").read_text())
        cls.reference = json.loads((cls.source/"results/weight4_tau100/seed-692.json").read_text())

    @classmethod
    def tearDownClass(cls):cls.temp.cleanup()

    def validate(self, wrapper):
        return m.core.validate(wrapper, self.reference, m.m10.core, m.m10c, m.m10b.analyze_pair)

    def trial(self, wrapper, variant="block_exc"):
        return wrapper["m9_result"]["arms"]["paired"]["normal_mapping"]["recurrence_probes"]["stages"]["checkpoint_probe"][variant][0]

    def test_full_pilot_preserves_parent_and_reconstructs_all_variants(self):
        audit = json.loads((self.output/"diagnostics.json").read_text())
        self.assertEqual(audit["pairs"], 64)
        self.assertEqual(audit["windows"], 128)
        self.assertEqual(audit["phases"], 512)
        self.assertEqual(audit["new_decoder_fits"], 0)
        self.assertTrue(audit["original_parent_reproduced"])
        self.assertLess(audit["maximum_voltage_error"], 2e-5)

    def test_wrong_selection_and_escaped_arrivals_are_rejected(self):
        bad = copy.deepcopy(self.wrapper)
        self.trial(bad)["recurrence_gate"]["blocked_synapses"] = []
        with self.assertRaisesRegex(ValueError, "blocked edge selection"):
            self.validate(bad)
        bad = copy.deepcopy(self.wrapper)
        q = self.trial(bad)
        ident = q["recurrence_gate"]["blocked_synapses"][0]
        edge = next(e for e in bad["m9_result"]["initial_synapses"] if e["id"] == ident)
        q["trace"][1]["events"].insert(0, dict(kind="arrival", time_us=q["action_us"],
            synapse=ident, source=edge["pre"], target=edge["post"], amplitude=.1))
        with self.assertRaisesRegex(ValueError, "blocked arrival escaped"):
            self.validate(bad)

    def test_changed_checkpoint_and_prediction_are_rejected(self):
        bad = copy.deepcopy(self.wrapper)
        self.trial(bad)["trace"][1]["potentials_before"][0] += .01
        with self.assertRaisesRegex(ValueError, "action start state"):
            self.validate(bad)
        bad = copy.deepcopy(self.wrapper)
        self.trial(bad)["prediction_count"] *= -1
        with self.assertRaisesRegex(ValueError, "original prediction"):
            self.validate(bad)

    def test_changed_voltage_and_parent_are_rejected(self):
        bad = copy.deepcopy(self.wrapper)
        self.trial(bad)["trace"][-1]["potentials_after"][0] += .01
        with self.assertRaisesRegex(ValueError, "voltage differs"):
            self.validate(bad)
        bad = copy.deepcopy(self.wrapper)
        bad["m9_result"]["seed"] += 1
        with self.assertRaisesRegex(ValueError, "parent differs"):
            self.validate(bad)

    def test_source_changes_are_rejected(self):
        path = self.source/"results/weight4_tau100/seed-692.json"
        original = path.read_bytes()
        try:
            path.write_bytes(original+b"\n")
            with self.assertRaisesRegex(ValueError, "manifest/source differs"):m.load(self.output)
        finally:path.write_bytes(original)


if __name__ == "__main__":unittest.main()
