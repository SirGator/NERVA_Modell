import copy
import json
from pathlib import Path
import sqlite3
import subprocess
import sys
from tempfile import TemporaryDirectory
import unittest

HERE=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(HERE))
import control


class PrecisionPolicyTests(unittest.TestCase):
    def test_complete_paired_design_and_reconstruction_cost(self):
        source=(HERE/"config/precision_smoke.toml").read_text()
        spec=control.parse(source)
        self.assertEqual(control.design_counts(spec),(13,48))
        self.assertEqual(len(control.study(spec,7)["experiment"]["interventions"]),7)
        self.assertEqual(len(control.study(spec,19)["experiment"]["interventions"]),6)
        for old,new in [('max_trials = 13','max_trials = 12'),
                        ('max_native_training_passes = 48','max_native_training_passes = 47')]:
            with self.subTest(new=new),self.assertRaises(ValueError):
                control.parse(source.replace(old,new))


class NativePrecisionTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        subprocess.run(["cargo","build","--offline","--release","--manifest-path",str(HERE/"Cargo.toml")],
                       check=True,capture_output=True)
        cls.binary=HERE/"target/release/nerva-research"

    def test_precision_pipeline_resume_conservation_and_invalid_controls(self):
        source=(HERE/"config/precision_smoke.toml").read_text();spec=control.parse(source)
        with TemporaryDirectory() as directory:
            root=Path(directory)/"precision"
            with control.locked(root):
                plan=control.prepare(root,spec,source,self.binary)
                report=control.execute(root,plan)
            self.assertEqual(plan["counts"]["trials"],13)
            self.assertEqual(plan["counts"]["native_training_passes"],48)
            self.assertEqual(len(report["pipeline"]),12)
            self.assertEqual(len(report["contrasts"]),8)
            self.assertFalse(report["functional_memory_demonstrated"])
            self.assertFalse(report["exploratory_association_advantage"])
            calibration=report["numeric_calibration"]["cases"]
            self.assertEqual(calibration[1]["published"],0)
            self.assertGreater(calibration[3]["published"],0)
            self.assertLess(abs(calibration[3]["effective_mass_error"]),1e-15)
            self.assertGreater(report["control_means"]["direct"]["rounding_error_mass"],
                               report["control_means"]["accumulated"]["rounding_error_mass"])
            self.assertLess(report["control_means"]["accumulated"]["mass_balance_max_error"],1e-12)
            receipt=(root/"receipt.json").read_bytes()
            with control.locked(root):
                control.execute(root,control.load_plan(root,source),report_only=True)
            self.assertEqual((root/"receipt.json").read_bytes(),receipt)
            with sqlite3.connect(root/"control.sqlite") as db:
                self.assertEqual(db.execute("SELECT COUNT(*) FROM trials").fetchone()[0],12)
                self.assertTrue(db.execute("SELECT value FROM metadata WHERE key='numeric_calibration'").fetchone())
            records=[json.loads(p.read_bytes())["record"] for p in (root/"seeds").glob("*/records/*.json")]
            for defect in ["missing","input","balance","frozen","calibration"]:
                broken=copy.deepcopy(records)
                model=next(r for r in broken if r["job"]["intervention"]=="accumulated")
                if defect=="missing":
                    broken.remove(model)
                elif defect=="input":
                    model["details"]["training"]["input_digest"]="different"
                elif defect=="balance":
                    model["metrics"]["mass_balance_max_error"]=1
                elif defect=="frozen":
                    next(r for r in broken if r["job"]["intervention"]=="accumulated_frozen")["metrics"]["pending_mass"]=1
                else:
                    next(r for r in broken if r["job"]["intervention"]=="precision_numeric")["details"]["controls_passed"]=False
                with self.subTest(defect=defect),self.assertRaises(ValueError):
                    control.analyze_precision(broken)
