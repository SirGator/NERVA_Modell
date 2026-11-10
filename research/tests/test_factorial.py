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
import factorial_report as report


class FactorialPolicyTests(unittest.TestCase):
    def test_complete_search_and_normalization_budget(self):
        source=(HERE/"config/direction_strength_smoke.toml").read_text()
        spec=control.parse(source)
        self.assertEqual(control.design_counts(spec),(2,100))
        self.assertEqual(control.study(spec,7)["experiment"]["scenario"],"hippocampal_direction_strength")
        for old,new in [("max_trials = 2","max_trials = 1"),
                        ("max_native_training_passes = 100","max_native_training_passes = 99")]:
            with self.subTest(new=new),self.assertRaises(ValueError):
                control.parse(source.replace(old,new))

    def test_valid_exact_factorial_separates_direction_strength_and_interaction(self):
        observations=[]
        arms={}
        for name,weights,actual,gain in [
            ("NN",[1.125,1.],.125,.1),("NR",[1.25,1.],.25,.2),
            ("RN",[1.,1.125],.125,.3),("RR",[1.,1.25],.25,.5)]:
            arms[name]=dict(requested_norm=actual,ray_limit=1.,feasible=True,ray_norm=actual,
                normalization_iterations=0,observation=len(observations),matched_checkpoint=True,only_weights_changed=True,
                actual_norm=actual,relative_norm_error=0.,direction_cosine=1.,rounding_l2=0.,faithful=True)
            observations.append(dict(weights=list(enumerate(weights)),completion_margins=[gain,gain]))
        expected=dict(direction_at_natural_strength=.2,direction_at_high_strength=.3,
            strength_on_natural_direction=.1,strength_on_reference_direction=.2,interaction=.1)
        matrix=dict(natural_strength=.125,high_strength=.25,arms=arms,valid=True,contrasts=[expected,expected])
        actual=report.validate_matrix(matrix,observations,[1.,1.],[.125,0.],[0.,.25],.25)
        self.assertAlmostEqual(actual[0]["interaction"],.1)
        broken=copy.deepcopy(matrix)
        broken["arms"]["RN"]["actual_norm"]=.2
        with self.assertRaises(ValueError):
            report.validate_matrix(broken,observations,[1.,1.],[.125,0.],[0.,.25],.25)


class NativeFactorialTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        subprocess.run(["cargo","build","--offline","--release","--manifest-path",str(HERE/"Cargo.toml")],
                       check=True,capture_output=True)
        cls.binary=HERE/"target/release/nerva-research"

    def test_replay_archive_cache_and_rejection_of_confounded_comparisons(self):
        source=(HERE/"config/direction_strength_smoke.toml").read_text()
        spec=control.parse(source)
        with TemporaryDirectory() as directory:
            root=Path(directory)/"factorial"
            with control.locked(root):
                plan=control.prepare(root,spec,source,self.binary)
                result=control.execute(root,plan)
            self.assertEqual(plan["counts"]["native_training_passes"],100)
            self.assertLess(result["actual_native_training_passes"],100)
            self.assertTrue(result["studies"][0]["reference_method"])
            self.assertIsNone(result["studies"][1]["reference_method"])
            self.assertFalse(result["functional_memory_demonstrated"])
            receipt=(root/"receipt.json").read_bytes()
            archived=subprocess.run([sys.executable,str(root/"producer.py"),"report","--output",str(root)],
                                    capture_output=True,text=True)
            self.assertEqual(archived.returncode,0,archived.stderr)
            self.assertEqual(receipt,(root/"receipt.json").read_bytes())
            with sqlite3.connect(root/"control.sqlite") as db:
                self.assertEqual(db.execute("SELECT COUNT(*) FROM trials").fetchone()[0],2)
            records=[json.loads(p.read_bytes())["record"] for p in (root/"seeds").glob("*/records/*.json")]
            for defect in ("missing_grid","false_reference","false_norm","wrong_ray","duplicate","incomplete",
                           "clipped_as_feasible","unreplayed","wrong_natural","scope"):
                broken=copy.deepcopy(records)
                row=next(r for r in broken if r["job"]["seed"]==7)
                d=row["details"]
                if defect=="missing_grid":
                    d["candidates"].pop()
                elif defect=="false_reference":
                    d["candidates"][1]["verified_reference"]=True
                elif defect=="false_norm":
                    d["feasible_factorial"]["arms"]["RN"]["actual_norm"]*=2
                elif defect=="wrong_ray":
                    d["feasible_factorial"]["arms"]["RR"]["ray_norm"]*=.8
                elif defect=="duplicate":
                    d["observations"].append(copy.deepcopy(d["observations"][0]))
                    row["metrics"]["native_training_passes"]+=1
                elif defect=="incomplete":
                    d["observations"][0]["probe"]["complete"]=False
                elif defect=="clipped_as_feasible":
                    d["requested_factorial"]["arms"]["NR"]["feasible"]=True
                elif defect=="unreplayed":
                    row["full_replay_checked"]=False
                elif defect=="wrong_natural":
                    d["natural_observation"]=0
                else:
                    sensory=next(i for i,e in enumerate(d["edges"]) if e[3]=="sensory_to_core")
                    d["directions"]["recurrent_two_hop"][sensory]=1.
                with self.subTest(defect=defect),self.assertRaises(ValueError):
                    report.analyze(broken)
            (root/"factorial_report.py").write_text("# changed\n")
            with self.assertRaisesRegex(ValueError,"artifact checksum differs"):
                control.load_plan(root,source)
