import copy
import importlib.util
import json
import shutil
import subprocess
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory

spec = importlib.util.spec_from_file_location("m10b_run", Path(__file__).resolve().parents[1] / "run.py")
m10b = importlib.util.module_from_spec(spec)
spec.loader.exec_module(m10b)
ROOT = Path(__file__).resolve().parents[4]


class GeometryTests(unittest.TestCase):
    def decoder(self):
        return dict(mean=[10, 0], scale=[2, 1], centers={"-1": [-1, 0], "1": [1, 0]})

    def pair(self, a, d, predictions):
        return [dict(key=key, core_counts=counts, prediction_count=prediction)
                for key, counts, prediction in zip(["a", "d"], [a, d], predictions)]

    def test_common_bias_can_hide_correct_effect_direction(self):
        natural = self.pair([8, 0], [12, 0], [-1, 1])
        biased = self.pair([14, 0], [18, 0], [1, 1])
        a = m10b.core.analyze_pair(self.decoder(), natural, [-1, 1])
        b = m10b.core.analyze_pair(self.decoder(), biased, [-1, 1])
        self.assertEqual(a["raw_accuracy"], 1)
        self.assertEqual(b["raw_accuracy"], .5)
        self.assertEqual(a["pair_rank_accuracy"], b["pair_rank_accuracy"])
        self.assertEqual(b["pair_rank_accuracy"], 1)
        self.assertTrue(b["aligned_but_same_class"] and b["bias_dominates"])

    def test_distinct_action_counts_can_be_orthogonal_to_visual_effect(self):
        pair = self.pair([14, 0], [14, 10], [1, 1])
        result = m10b.core.analyze_pair(self.decoder(), pair, [-1, 1])
        self.assertEqual(result["pair_rank_accuracy"], .5)
        self.assertEqual(result["effect_axis_cosine"], 0)
        self.assertTrue(result["projection_tie"])

    def test_reversed_mapping_requires_reversed_direction(self):
        pair = self.pair([8, 0], [12, 0], [-1, 1])
        result = m10b.core.analyze_pair(self.decoder(), pair, [1, -1])
        self.assertEqual(result["pair_rank_accuracy"], 0)
        self.assertEqual(result["effect_axis_cosine"], -1)
        self.assertTrue(result["wrong_direction"])

    def test_degenerate_axis_and_original_prediction_integrity(self):
        decoder = self.decoder()
        decoder["centers"]["1"] = decoder["centers"]["-1"][:]
        result = m10b.core.analyze_pair(decoder, self.pair([8, 0], [12, 0], [-1, -1]), [-1, 1])
        self.assertTrue(result["axis_degenerate"])
        self.assertEqual(result["pair_rank_accuracy"], .5)
        with self.assertRaisesRegex(ValueError, "original prediction differs"):
            m10b.core.analyze_pair(self.decoder(), self.pair([8, 0], [12, 0], [1, 1]), [-1, 1])


class SourceAuditTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.temp = TemporaryDirectory()
        cls.source = Path(cls.temp.name) / "source"
        cls.output = Path(cls.temp.name) / "output"
        shutil.copytree(ROOT / "Messungen/Experimente/m9-v2-smoke", cls.source)
        proc = subprocess.run(["python3", "NERVA_Modell/Experimente/m10b/run.py", "run", "--stage", "smoke",
                               "--source", str(cls.source), "--output", str(cls.output)], cwd=ROOT, capture_output=True, text=True)
        if proc.returncode:
            cls.temp.cleanup()
            raise AssertionError(proc.stdout + proc.stderr)

    @classmethod
    def tearDownClass(cls):
        cls.temp.cleanup()

    def test_all_conditions_controls_and_interventions_without_new_fits(self):
        diagnostics = json.loads((self.output / "diagnostics.json").read_text())
        self.assertEqual(diagnostics["source_files"], 4)
        self.assertEqual(diagnostics["pairs"], 192)
        self.assertEqual(diagnostics["neural_runs"], 0)
        self.assertEqual(diagnostics["action_label_fits"], 0)
        self.assertTrue(diagnostics["original_accuracy_reproduced"])
        self.assertIn("keine einzelne Handlungswirkungsvorhersage", (self.output / "summary.md").read_text())

    def test_source_changes_are_rejected(self):
        path = self.source / "results/reference/seed-692.json"
        original = path.read_bytes()
        try:
            path.write_bytes(original+b"\n")
            with self.assertRaisesRegex(ValueError, "manifest/source differs"):
                m10b.load(self.output)
        finally:
            path.write_bytes(original)


if __name__ == "__main__":
    unittest.main()
