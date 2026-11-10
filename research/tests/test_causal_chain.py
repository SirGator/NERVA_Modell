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


class ChainPolicyTests(unittest.TestCase):
    def test_deterministic_calibrations_are_shared_and_all_work_is_budgeted(self):
        source = (HERE/"config/causal_chain_smoke.toml").read_text()
        spec = control.parse(source)
        self.assertEqual(control.design_counts(spec), (5, 38))
        self.assertEqual(control.study(spec, 7)["experiment"]["interventions"], list(control.CHAIN_GROUPS))
        self.assertEqual(control.study(spec, 19)["experiment"]["interventions"], ["C_gain"])
        spec["control"]["seeds"].append(23)
        self.assertEqual(control.design_counts(spec), (6, 50))
        for old,new in [('max_trials = 5','max_trials = 4'),
                        ('max_native_training_passes = 38','max_native_training_passes = 37'),
                        ('suite = "causal_chain"','suite = "unknown"')]:
            with self.subTest(new=new), self.assertRaises(ValueError):
                control.parse(source.replace(old,new))


class NativeChainControlTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        subprocess.run(["cargo","build","--offline","--release","--manifest-path",str(HERE/"Cargo.toml")],
                       check=True,capture_output=True)
        cls.binary=HERE/"target/release/nerva-research"

    def test_chain_resume_archived_report_and_rejection_of_incomplete_diagnoses(self):
        source=(HERE/"config/causal_chain_smoke.toml").read_text()
        spec=control.parse(source)
        with TemporaryDirectory() as directory:
            root=Path(directory)/"chain"
            with control.locked(root):
                plan=control.prepare(root,spec,source,self.binary)
            self.assertEqual(plan["counts"]["trials"],5)
            self.assertEqual(plan["counts"]["native_training_passes"],38)
            self.assertFalse((root/"seeds").exists())
            # Finish one model seed first, then resume the remaining experiments.
            control.native(root/"runner","run",control.study(spec,19),control.seed_root(root,19))
            original=next((control.seed_root(root,19)/"records").glob("*.json"))
            raw=original.read_bytes()
            with control.locked(root):
                report=control.execute(root,control.load_plan(root,source))
            self.assertEqual(original.read_bytes(),raw)
            self.assertEqual(len(report["pipeline"]),5)
            self.assertTrue(report["diagnostic_controls_passed"])
            self.assertFalse(report["functional_memory_demonstrated"])
            self.assertEqual(len(report["models"]),2)
            self.assertEqual(sum(r["metrics"]["native_training_passes"] for r in report["pipeline"])*2,38)
            self.assertEqual(control.status(root,plan)["expected"],5)
            self.assertEqual(control.status(root,plan)["cached_records"],5)
            receipt=(root/"receipt.json").read_bytes()
            archived=subprocess.run([sys.executable,str(root/"producer.py"),"report","--output",str(root)],
                                    capture_output=True,text=True)
            self.assertEqual(archived.returncode,0,archived.stderr)
            self.assertEqual((root/"receipt.json").read_bytes(),receipt)
            with sqlite3.connect(root/"control.sqlite") as db:
                self.assertEqual(db.execute("SELECT COUNT(*) FROM trials").fetchone()[0],5)
                self.assertEqual(db.execute("PRAGMA integrity_check").fetchone()[0],"ok")
            records=[json.loads(p.read_bytes())["record"] for p in (root/"seeds").glob("*/records/*.json")]
            for defect in ["missing","duplicate","unmatched_gain"]:
                broken=copy.deepcopy(records)
                if defect=="missing":
                    broken=[r for r in broken if r["job"]["intervention"]!="B_memory"]
                elif defect=="duplicate":
                    broken.append(copy.deepcopy(broken[0]))
                else:
                    next(r for r in broken if r["job"]["intervention"]=="C_gain")["details"]["data"]["doses"][0]["matched_checkpoint"]=False
                with self.subTest(defect=defect),self.assertRaises(ValueError):
                    control.analyze_chain(broken)
            failed=copy.deepcopy(records)
            next(r for r in failed if r["job"]["intervention"]=="B_memory")["metrics"]["diagnostic_controls_passed"]=0.
            invalid=control.analyze_chain(failed)
            self.assertFalse(invalid["diagnostic_controls_passed"])
            self.assertTrue(all(not m["interpretation_valid"] for m in invalid["models"]))
            self.assertNotIn("weights_before",json.dumps(report),
                             "central report must not duplicate native weight snapshots")
