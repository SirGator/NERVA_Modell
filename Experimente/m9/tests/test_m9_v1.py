import copy
import csv
import importlib.util
import json
import subprocess
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory

spec = importlib.util.spec_from_file_location("m9_v1_run", Path(__file__).resolve().parents[1] / "v1.py")
m9 = importlib.util.module_from_spec(spec)
spec.loader.exec_module(m9)
ROOT = Path(__file__).resolve().parents[4]


class PlanTests(unittest.TestCase):
    def test_fixed_matrix_fresh_seeds_and_separate_software_stage(self):
        pilot, confirm, smoke = [m9.plan(stage) for stage in ["pilot", "confirm", "smoke"]]
        self.assertEqual(pilot["seeds"], [600, 601, 602])
        self.assertEqual(confirm["seeds"], list(range(610, 620)))
        self.assertTrue(set(pilot["seeds"]).isdisjoint(confirm["seeds"]))
        self.assertEqual(pilot["conditions"], confirm["conditions"])
        self.assertEqual({tuple(c["condition"].values()) for c in pilot["conditions"]},
                         {(1, 20000), (4, 20000), (1, 100000), (4, 100000)})
        self.assertFalse(smoke["scientific_run"])
        self.assertEqual(smoke["trial_counts"], [8, 8, 8])
        for stage in ["pilot", "confirm"]:
            with self.assertRaisesRegex(ValueError, "trial counts are fixed"):
                m9.plan(stage, (8, 8, 8))
        for counts in [(8, 10, 8), (8, 8, True), (8, 8, 8192)]:
            with self.assertRaises(ValueError):
                m9.plan("smoke", counts)

    def test_family_correction_does_not_change_m8(self):
        self.assertEqual(m9.FAMILY_SIZE, 8)
        self.assertEqual(m9.plan("confirm")["preregistration"]["multiplicity"]["alpha"], .00625)
        self.assertEqual(len(m9.m8.LEVELS), 13)
        self.assertEqual(m9.m8.plan("confirm")["seeds"], list(range(510, 520)))
        self.assertLess(m9.m8.sign_p([1] * 10), .05 / 8)
        self.assertGreater(m9.m8.sign_p([1] * 9 + [-1]), .05 / 8)
        self.assertEqual(m9.m8.sign_p([0] * 10), 1)

    def test_factorial_interaction_uses_control_adjusted_learning_advantages(self):
        records = {}
        for (name, _, _), accuracy in zip(m9.LEVELS, [.5, .5, .5, 1]):
            arms = {}
            for arm in m9.m6.DEFAULT_ARMS:
                score = accuracy if arm == "paired" else .5
                arms[arm] = {mapping: {stage: {"count": {"balanced_accuracy": score}}
                                     for stage in m9.m8.PRIMARY} for mapping in m9.m6.MAPS}
            records[name] = [dict(arms=arms)]
        rows = m9.factorial_contrasts(records)
        self.assertEqual(len(rows), 64)
        self.assertTrue(all(r["mean_difference"] == .5 for r in rows))
        # An equally good frozen combination has no training-specific interaction.
        for mapping in m9.m6.MAPS:
            for stage in m9.m8.PRIMARY:
                records["weight4_tau100"][0]["arms"]["aged_frozen"][mapping][stage]["count"]["balanced_accuracy"] = 1
        adjusted = [r for r in m9.factorial_contrasts(records) if r["control"] == "aged_frozen"]
        self.assertTrue(all(r["mean_difference"] == 0 for r in adjusted))

    def test_learning_gate_applies_m9_family_without_weakening_m8(self):
        values = []
        stage = "checkpoint_probe"
        for seed in range(10):
            arms = {}
            for arm in m9.m6.DEFAULT_ARMS:
                score = (1 if seed < 9 else .5) if arm == "paired" else 0
                pair_score = int(arm == "paired" and seed < 9)
                arms[arm] = {
                    mapping: dict(visual_decodability=1, **{stage: {
                        feature: dict(balanced_accuracy=score) for feature in ["count", "time_bins"]
                    }}) for mapping in m9.m6.MAPS
                }
                arms[arm]["mapping_comparison"] = {stage: {"count": dict(
                    flip_rate=pair_score, dual_correct_rate=pair_score)}}
            values.append(dict(arms=arms))
        screen, confirmed, rows = m9.contrasts("synthetic", values, stage)
        self.assertTrue(screen and confirmed)
        self.assertTrue(all(r["family_alpha"] == .05 / 8 for r in rows if r["feature"] != "time_bins"))
        old_screen, old_confirmed, old_rows = m9.m8.contrasts("synthetic", values, stage)
        self.assertTrue(old_screen)
        self.assertFalse(old_confirmed)  # Nine wins plus one tie: 1/512 > 0.05/26.
        self.assertTrue(all(r["family_alpha"] == .05 / 26 for r in old_rows if r["feature"] != "time_bins"))


class RuntimeAuditTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.temp = TemporaryDirectory()
        cls.root = Path(cls.temp.name) / "smoke"
        proc = subprocess.run(["python3", "NERVA_Modell/Experimente/m9/v1.py", "run", "--stage", "smoke",
                               "--jobs", "2", "--output", str(cls.root)], cwd=ROOT, capture_output=True, text=True)
        if proc.returncode:
            cls.temp.cleanup()
            raise AssertionError(proc.stdout + proc.stderr)
        cls.config = m9.cases(690, (8, 8, 8))[-1]
        cls.path = cls.root / "results/weight4_tau100/seed-690.json"
        cls.result = json.loads(cls.path.read_text())

    @classmethod
    def tearDownClass(cls):
        cls.temp.cleanup()

    def test_all_four_conditions_audited_and_smoke_cannot_confirm(self):
        diagnostics = json.loads((self.root / "diagnostics.json").read_text())
        self.assertEqual(len(diagnostics["audits"]), 4)
        self.assertEqual(diagnostics["total"]["branch_rows"], 4 * 56)
        self.assertEqual(diagnostics["exact_sign_family_alpha"], .00625)
        with (self.root / "summary.csv").open(newline="") as stream:
            rows = list(csv.DictReader(stream))
        self.assertTrue(all(row[s + "_confirmed"] == "False" for row in rows for s in m9.m8.PRIMARY))
        with (self.root / "weight-summary.csv").open(newline="") as stream:
            weights = list(csv.DictReader(stream))
        self.assertEqual(len(weights), 16)
        for row in weights:
            self.assertAlmostEqual(float(row["visual_retained_fraction"]),
                                   float(row["visual_trained_sum"]) / float(row["visual_initial_sum"]))

    def test_tampered_effective_parameter_and_propagation_are_rejected(self):
        bad = copy.deepcopy(self.result)
        bad["effective_parameters"]["stdp_tau_us"] = 20000
        with self.assertRaisesRegex(ValueError, "effective parameter differs"):
            m9.validate(bad, self.config)
        path = Path(str(self.path) + ".propagation.csv")
        original = path.read_bytes()
        try:
            with path.open(newline="") as stream:
                reader = csv.DictReader(stream)
                fields, rows = reader.fieldnames, list(reader)
            rows[0]["extra_core_spikes"] = str(int(rows[0]["extra_core_spikes"]) + 1)
            with path.open("w", newline="") as stream:
                writer = csv.DictWriter(stream, fieldnames=fields)
                writer.writeheader()
                writer.writerows(rows)
            with self.assertRaisesRegex(ValueError, "propagation/detail counts differ"):
                m9.validate(self.result, self.config, self.path)
        finally:
            path.write_bytes(original)

    def test_reference_and_singles_match_original_m8_dynamics(self):
        for (name, _, _), old_index in zip(m9.LEVELS[:3], [0, 2, 6]):
            config = m9.m8.cases(690, (8, 8, 8))[old_index]
            config["case_id"] = name
            cfg, out = Path(self.temp.name) / f"{name}.json", Path(self.temp.name) / f"{name}-m8.json"
            cfg.write_text(json.dumps(config))
            proc = subprocess.run(["./NERVA_Modell/Experimente/run.sh", "m8", "--config", str(cfg),
                                   "--output", str(out)], cwd=ROOT, capture_output=True, text=True)
            self.assertEqual(proc.returncode, 0, proc.stderr)
            old = json.loads(out.read_text())
            new_path = self.root / "results" / name / "seed-690.json"
            new = json.loads(new_path.read_text())
            for key in ["arms", "normalization", "reference_normalization", "initial_neurons", "initial_synapses", "effective_parameters"]:
                self.assertEqual(old[key], new[key], key)
            for suffix in [".details.csv", ".audit.csv", ".sensor.csv", ".copies.csv", ".branches.csv", ".propagation.csv"]:
                self.assertEqual(Path(str(out) + suffix).read_bytes(), Path(str(new_path) + suffix).read_bytes())

    def test_modified_manifest_cannot_redefine_scientific_plan(self):
        path = self.root / "manifest.json"
        original = path.read_bytes()
        try:
            manifest = json.loads(original)
            manifest["seeds"] = [610, 611]
            path.write_text(json.dumps(manifest))
            with self.assertRaisesRegex(ValueError, "manifest differs"):
                m9.report(self.root)
        finally:
            path.write_bytes(original)


if __name__ == "__main__":
    unittest.main()
