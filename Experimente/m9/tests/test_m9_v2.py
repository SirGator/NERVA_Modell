import copy
import importlib.util
import json
import subprocess
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory

spec = importlib.util.spec_from_file_location("m9_v2", Path(__file__).resolve().parents[1] / "run.py")
m9 = importlib.util.module_from_spec(spec)
spec.loader.exec_module(m9)
ROOT = Path(__file__).resolve().parents[4]


class V2PlanTests(unittest.TestCase):
    def test_twenty_confirmation_seeds_and_old_plan_not_reinterpreted(self):
        plan = m9.plan("confirm")
        self.assertEqual(plan["protocol"], "NERVA-M9-v2")
        self.assertEqual(plan["seeds"], list(range(610, 630)))
        self.assertTrue(set(plan["seeds"]).isdisjoint(m9.plan("pilot")["seeds"]))
        old = json.loads((ROOT / "Messungen/Experimente/m9-confirm/manifest.json").read_text())
        self.assertEqual(old["protocol"], "NERVA-M9-v1")
        self.assertEqual(old["seeds"], list(range(610, 620)))
        with self.assertRaisesRegex(ValueError, "trial counts are fixed"):
            m9.plan("confirm", (8, 8, 8))

    def test_behavioral_gate_requires_twenty_even_with_perfect_scores(self):
        arms = {}
        stage = "checkpoint_probe"
        for arm in m9.m6.DEFAULT_ARMS:
            value = int(arm == "paired")
            arms[arm] = {mapping: dict(visual_decodability=1, **{stage: {
                feature: dict(balanced_accuracy=value) for feature in ["count", "time_bins"]
            }}) for mapping in m9.m6.MAPS}
            arms[arm]["mapping_comparison"] = {stage: {"count": dict(flip_rate=value, dual_correct_rate=value)}}
        sample = dict(arms=arms)
        self.assertFalse(m9.contrasts("synthetic", [sample] * 10, stage)[1])
        self.assertTrue(m9.contrasts("synthetic", [sample] * 20, stage)[1])


class V2AuditTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.temp = TemporaryDirectory()
        cls.root = Path(cls.temp.name)
        proc = subprocess.run(["python3", "NERVA_Modell/Experimente/m9/run.py", "run", "--stage", "smoke",
                               "--jobs", "2", "--output", str(cls.root)], cwd=ROOT, capture_output=True, text=True)
        if proc.returncode:
            cls.temp.cleanup()
            raise AssertionError(proc.stdout + proc.stderr)
        cls.config = m9.cases(692, (8, 8, 8))[-1]
        cls.path = cls.root / "results/weight4_tau100/seed-692.json"
        cls.result = json.loads(cls.path.read_text())

    @classmethod
    def tearDownClass(cls):
        cls.temp.cleanup()

    def test_all_causal_branches_audited_and_smoke_cannot_confirm(self):
        diagnostics = json.loads((self.root / "diagnostics.json").read_text())
        self.assertEqual(diagnostics["weight_reset_branches"], 4 * 96)
        audit, _ = m9.validate(self.result, self.config, self.path)
        self.assertEqual(audit["weight_reset_branches"], 96)
        self.assertIn("Gewichtskausalität", (self.root / "summary.md").read_text())
        self.assertIn("Gesamtnachweis einschließlich Gewichtskausalität: keine", (self.root / "summary.md").read_text())

    def test_tampered_edits_events_decoder_and_replay_are_rejected(self):
        bad = copy.deepcopy(self.result)
        q = bad["arms"]["paired"]["normal_mapping"]["weight_reset_probes"]["stages"]["checkpoint_probe"]["restore_all_plastic"][0]
        self.assertTrue(q["changed_weights"])
        q["changed_weights"][0][2] += .01
        with self.assertRaisesRegex(ValueError, "reset weight edits differ"):
            m9.validate(bad, self.config)
        for field, value, error in [("core_counts", [999] * 32, "counts differ"),
                                    ("parent_unchanged", False, "invariant failed"),
                                    ("forecast_us", 999, "clock differs")]:
            bad = copy.deepcopy(self.result)
            q = bad["arms"]["paired"]["normal_mapping"]["weight_reset_probes"]["stages"]["checkpoint_probe"]["sham"][0]
            q[field] = value
            with self.assertRaisesRegex(ValueError, error):
                m9.validate(bad, self.config)
        bad = copy.deepcopy(self.result)
        bad["arms"]["paired"]["normal_mapping"]["weight_reset_probes"]["count_readout"]["scale"][0] = 0
        with self.assertRaisesRegex(ValueError, "decoder scale"):
            m9.validate(bad, self.config)

    def test_causality_requires_loss_in_both_mappings_and_twenty_seeds(self):
        result = copy.deepcopy(self.result)
        for arm in m9.m6.DEFAULT_ARMS:
            for mapping in m9.m6.MAPS:
                model = result["arms"][arm][mapping]
                for stage in m9.m8.PRIMARY:
                    for intervention in m9.resets.INTERVENTIONS:
                        for trial, label in zip(model["weight_reset_probes"]["stages"][stage][intervention], model[stage]["labels"]):
                            trial["prediction_count"] = -label if arm == "paired" and intervention == "restore_all_plastic" else label
        args = (m9.m6.bootstrap_paired, m9.m8.sign_p, .05 / 8)
        self.assertTrue(all(m9.resets.contrasts("synthetic", [result] * 20, *args)[0].values()))
        self.assertFalse(any(m9.resets.contrasts("synthetic", [result] * 10, *args)[0].values()))
        for stage in m9.m8.PRIMARY:
            model = result["arms"]["paired"]["reversed_mapping"]
            for trial, label in zip(model["weight_reset_probes"]["stages"][stage]["restore_all_plastic"], model[stage]["labels"]):
                trial["prediction_count"] = label
        self.assertFalse(any(m9.resets.contrasts("synthetic", [result] * 20, *args)[0].values()))


if __name__ == "__main__":
    unittest.main()
