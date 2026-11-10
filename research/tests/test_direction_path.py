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


class DirectionPolicyTests(unittest.TestCase):
    def test_complete_search_including_reconstruction_and_replay_is_budgeted(self):
        source=(HERE/"config/direction_path_smoke.toml").read_text()
        spec=control.parse(source)
        self.assertEqual(control.design_counts(spec),(2,56))
        self.assertEqual(control.study(spec,7)["experiment"]["interventions"],["direction_path"])
        for old,new in [("max_trials = 2","max_trials = 1"),
                        ("max_native_training_passes = 56","max_native_training_passes = 55")]:
            with self.subTest(new=new),self.assertRaises(ValueError):
                control.parse(source.replace(old,new))


class NativeDirectionTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        subprocess.run(["cargo","build","--offline","--release","--manifest-path",str(HERE/"Cargo.toml")],
                       check=True,capture_output=True)
        cls.binary=HERE/"target/release/nerva-research"

    def test_native_success_and_unresolved_search_resume_and_reject_invalid_evidence(self):
        source=(HERE/"config/direction_path_smoke.toml").read_text()
        spec=control.parse(source)
        with TemporaryDirectory() as directory:
            root=Path(directory)/"diagnosis"
            with control.locked(root):
                plan=control.prepare(root,spec,source,self.binary)
                result=control.execute(root,plan)
            self.assertEqual(plan["counts"]["native_training_passes"],56)
            self.assertEqual(len(result["pipeline"]),2)
            self.assertTrue(result["diagnoses"][0]["reference_verified"])
            self.assertFalse(result["diagnoses"][1]["reference_verified"])
            self.assertIsNone(result["diagnoses"][1]["direction_reference"])
            self.assertFalse(result["functional_memory_demonstrated"])
            receipt=(root/"receipt.json").read_bytes()
            archived=subprocess.run([sys.executable,str(root/"producer.py"),"report","--output",str(root)],
                                    capture_output=True,text=True)
            self.assertEqual(archived.returncode,0,archived.stderr)
            self.assertEqual(receipt,(root/"receipt.json").read_bytes())
            with sqlite3.connect(root/"control.sqlite") as db:
                self.assertEqual(db.execute("SELECT COUNT(*) FROM trials").fetchone()[0],2)
            records=[json.loads(p.read_bytes())["record"] for p in (root/"seeds").glob("*/records/*.json")]
            for defect in ("grid","checkpoint","ledger","reference","alignment","readout","replay","duplicate"):
                broken=copy.deepcopy(records)
                row=next(r for r in broken if r["job"]["seed"]==7)
                d=row["details"]
                if defect=="grid":
                    d["candidates"].pop()
                elif defect=="checkpoint":
                    d["candidates"][0]["only_weights_changed"]=False
                elif defect=="ledger":
                    d["natural"]["frames"][3]["neurons"][0]["stored_after"]+=1
                elif defect=="reference":
                    d["candidates"][1]["verified_reference"]=True
                elif defect=="alignment":
                    d["candidates"][0]["alignment"]["cosine"]=.999
                elif defect=="readout":
                    d["baseline_margins"][0]+=1
                elif defect=="replay":
                    row["full_replay_checked"]=False
                else:
                    broken.append(copy.deepcopy(row))
                with self.subTest(defect=defect),self.assertRaises(ValueError):
                    control.analyze_diagnosis(broken)

