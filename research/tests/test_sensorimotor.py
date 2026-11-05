import hashlib
import json
import os
from pathlib import Path
import sqlite3
import subprocess
import sys
from tempfile import TemporaryDirectory
import unittest

HERE = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(HERE))
from evolution import history
from planning.profiles import SENSORIMOTOR_GROUPS, profile, sensorimotor_evaluate, sensorimotor_validate
from planning.spec import ResearchSpec
from planning.scheduler import prepare
from planning.backend import ResearchBackend
from analysis.plasticity_diagnostics import PlasticityDiagnostics
from analysis.size_environment import SizeEnvironment
from contracts import ExperimentContext
import run as frontend


def records(seeds=(11, 19), gain=.25):
    rows = []
    for seed in seeds:
        for group in SENSORIMOTOR_GROUPS:
            m = dict(prediction_accuracy=.5 if group.endswith("_frozen") else .5+gain,
                     position_mae=.5, persistence_mae=1., majority_accuracy=.5,
                     visual_accuracy=1., core_rate_hz=5., peak_core_rate_hz=10., silent_fraction=.5,
                     observed_events=1000., maximum_synapses=5000., parameters_preserved=1.,
                     weight_updates=0., homeostasis_updates=0., formed=0., pruned=0.,
                     action_copy_arrivals=0. if group.startswith("passive") else 24.)
            trial = dict(before_frame="a", after_frame="b", action_us=1, effect_us=20001)
            rows.append(dict(job=dict(seed=seed, intervention=group), metrics=m, full_replay_checked=True,
                             details=dict(initial_network_digest="same", training=[trial], probes=[trial])))
    return rows


class SensorimotorPolicyTests(unittest.TestCase):
    def test_arbitrary_positive_sizes_use_the_same_factorial_and_reject_invalid_values(self):
        chosen=[1,3,37,1025,4096,16384,65536]
        plan=PlasticityDiagnostics(dict(sizes=chosen)).propose(ExperimentContext("sizes",{},(11,)))
        self.assertEqual(len(plan.points),4*len(chosen))
        for n in chosen:
            points=[p for p in plan.points if p['processing_neurons']==n]
            self.assertEqual({(p['mechanisms.development'],p['mechanisms.homeostasis']) for p in points},
                             {(False,False),(False,True),(True,False),(True,True)})
        for invalid in [[0],[-1],[True],[1.0],[17,17],[]]:
            for cls in [PlasticityDiagnostics,SizeEnvironment]:
                with self.subTest(invalid=invalid,cls=cls),self.assertRaises(ValueError):cls(dict(sizes=invalid))

    def test_paired_control_metrics_and_gates(self):
        requirements = profile("sensorimotor_size").requirements
        result = sensorimotor_evaluate(records(), [11, 19], requirements)
        self.assertTrue(result["eligible"])
        self.assertEqual(result["quality"], .25)
        for change in ["readout", "resource", "pairing", "replay"]:
            with self.subTest(change=change):
                rows = records()
                if change == "readout": rows[0]["metrics"]["visual_accuracy"] = .1
                if change == "resource": rows[0]["metrics"]["maximum_synapses"] = 100000
                if change == "pairing": rows[0]["details"]["initial_network_digest"] = "different"
                if change == "replay": rows[0]["full_replay_checked"] = False
                result = sensorimotor_evaluate(rows, [11,19], requirements)
                self.assertFalse(result["eligible"])
                self.assertIsNone(result["quality"])
                self.assertEqual(result["raw_quality"], .25)
        with self.assertRaisesRegex(ValueError, "paired"):
            sensorimotor_evaluate(records()[:-1], [11,19], requirements)
        for condition, metric in [("passive", "action_copy_arrivals"), ("active_frozen", "weight_updates")]:
            rows=records()
            next(r for r in rows if r["job"]["intervention"]==condition)["metrics"][metric]=1
            with self.assertRaises(ValueError): sensorimotor_evaluate(rows,[11,19],requirements)

    def test_validation_is_specific_and_does_not_confirm_below_naive_baselines(self):
        seeds=list(range(16))
        requirements=profile("sensorimotor_size").requirements
        candidate=sensorimotor_evaluate(records(seeds,.4),seeds,requirements)
        baseline=sensorimotor_evaluate(records(seeds,.1),seeds,requirements)
        policy=dict(bootstrap_samples=100,alpha=.05,min_quality_gain=0,max_resource_ratio=1.25)
        self.assertTrue(sensorimotor_validate(candidate,baseline,policy)["held_out_quality_improved"])
        candidate["seed_majority_advantages"]=[-.1]*len(seeds)
        decision=sensorimotor_validate(candidate,baseline,policy)
        self.assertFalse(decision["held_out_quality_improved"])
        self.assertFalse(decision["checks"]["beats_naive_baselines"])
        baseline["seeds"]=[999]
        with self.assertRaisesRegex(ValueError,"share seeds"): sensorimotor_validate(candidate,baseline,policy)


class NativeSensorimotorTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        subprocess.run(["cargo","build","--offline","--manifest-path",str(HERE/"Cargo.toml")],check=True,capture_output=True)
        cls.binary=HERE/"target/debug/nerva-research"
        cls.sizes=[int(s.strip()) for s in os.environ.get("NERVA_TEST_SIZES","17,65").split(',')]
        SizeEnvironment(dict(sizes=cls.sizes))

    def config(self):
        return f'''[research]
name = "sensorimotor_fixture"
scenario = "sensorimotor_size"
training_seeds = [11, 19]
validation_seeds = [101, 103]
max_trials = {12*len(self.sizes)+24}
[base]
processing_neurons = {self.sizes[0]}
max_total_neurons = {max(self.sizes)+11}
max_observed_events = 10000000
train_trials = 4
test_trials = 4
[[studies]]
id = "sizes"
method = "size_environment"
[studies.options]
sizes = {json.dumps(self.sizes)}
[requirements]
min_visual_accuracy = 0.0
max_synapses = {max(self.sizes)*64+1000}
max_observed_events = 10000000
[validation]
bootstrap_samples = 100
'''

    def invoke(self,mode,root,config):
        return subprocess.run([sys.executable,str(HERE/"run.py"),mode,str(config),"--binary",str(self.binary),
                               "--output",str(root)],capture_output=True,text=True)

    def test_real_harness_budget_metrics_report_and_integrity(self):
        with TemporaryDirectory() as temp:
            temp=Path(temp)
            config=temp/"spec.toml";config.write_text(self.config())
            root=temp/"archive"
            planned=self.invoke("research-plan",root,config)
            self.assertEqual(planned.returncode,0,planned.stderr)
            manifest=history.read(root/"harness-manifest.json")
            self.assertEqual(manifest["maximum_training_trials"],12*len(self.sizes))
            self.assertEqual(manifest["reserved_validation_trials"],24)
            self.assertEqual(manifest["maximum_native_training_runs"],2*(12*len(self.sizes)+24))
            self.assertFalse((root/"training").exists())
            result=self.invoke("research",root,config)
            self.assertEqual(result.returncode,0,result.stdout+result.stderr)
            report=history.read(root/"reports/sizes.json")
            self.assertEqual([c["processing_neurons"] for c in report["findings"]["cells"]],self.sizes)
            self.assertEqual(len(report["findings"]["cells"][0]["groups"]),6)
            self.assertTrue(all(c["checks"]["matched_experience"] for c in report["findings"]["cells"]))
            final=history.read(root/"final-result.json")
            self.assertFalse(final["validation_used_for_selection"])
            self.assertEqual(final["validation_status"],"completed")
            self.assertEqual(final["candidate"]["evaluation"]["seeds"],[101,103])
            self.assertEqual(final["candidate"]["id"],history.read(root/"frozen-finalist.json")["winner"])
            paths=list(root.glob("training/*/records/*.json"))+list(root.glob("validation/*/records/*.json"))
            before={str(p):hashlib.sha256(p.read_bytes()).hexdigest() for p in paths}
            result=self.invoke("research-report",root,config)
            self.assertEqual(result.returncode,0,result.stdout+result.stderr)
            self.assertEqual(before,{str(p):hashlib.sha256(p.read_bytes()).hexdigest() for p in paths})
            self.assertEqual(history.read(root/"final-result.json"),final)
            with sqlite3.connect(root/"research.sqlite") as db:
                self.assertEqual(db.execute("PRAGMA integrity_check").fetchone()[0],"ok")
            p=paths[0];data=json.loads(p.read_bytes());data["record"]["metrics"]["prediction_accuracy"]=1.01
            p.write_text(json.dumps(data))
            result=self.invoke("research-report",root,config)
            self.assertNotEqual(result.returncode,0)
            self.assertIn("integrity",result.stderr)

    def test_invalid_late_size_and_wrong_method_fail_before_any_output(self):
        for text in [self.config().replace(f"sizes = {json.dumps(self.sizes)}","sizes = [17, 0]"),
                     self.config().replace('method = "size_environment"','method = "causal"')]:
            with TemporaryDirectory() as temp:
                temp=Path(temp);config=temp/"bad.toml";config.write_text(text);root=temp/"output"
                result=self.invoke("research",root,config)
                self.assertNotEqual(result.returncode,0)
                self.assertFalse(root.exists())
        import tomllib
        s=tomllib.loads(self.config());s["research"]["max_trials"]=35
        backend=ResearchBackend(self.binary,frontend.catalog,"sensorimotor_size")
        with self.assertRaisesRegex(ValueError,"budget"): prepare(ResearchSpec(s),backend)

    def test_same_diagnosis_and_contrasts_for_all_requested_sizes(self):
        text=self.config().replace('method = "size_environment"','method = "plasticity_diagnostics"')
        text=text.replace(f'max_trials = {12*len(self.sizes)+24}',f'max_trials = {48*len(self.sizes)+24}')
        with TemporaryDirectory() as temp:
            temp=Path(temp);config=temp/'diagnosis.toml';config.write_text(text);root=temp/'archive'
            result=self.invoke('research',root,config)
            self.assertEqual(result.returncode,0,result.stdout+result.stderr)
            report=history.read(root/'reports/sizes.json')
            self.assertEqual(report['method'],'plasticity_diagnostics')
            self.assertEqual(len(report['findings']['cells']),4*len(self.sizes))
            self.assertEqual([c['processing_neurons'] for c in report['findings']['contrasts']],self.sizes)
            for cell in report['findings']['cells']:
                flags=cell['parameters']
                active=cell['groups']['active']
                if not flags['mechanisms.development']:
                    self.assertEqual(active['formed'],0);self.assertEqual(active['pruned'],0)
                if not flags['mechanisms.homeostasis']:self.assertEqual(active['homeostasis_updates'],0)
            paths=list(root.glob('training/*/records/*.json'))+list(root.glob('validation/*/records/*.json'))
            before={str(p):p.read_bytes() for p in paths}
            result=self.invoke('research-report',root,config)
            self.assertEqual(result.returncode,0,result.stdout+result.stderr)
            self.assertEqual(before,{str(p):p.read_bytes() for p in paths})


if __name__ == "__main__": unittest.main()
