import copy
import hashlib
import json
from pathlib import Path
import sqlite3
import subprocess
import sys
from tempfile import TemporaryDirectory
import unittest

HERE = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(HERE))
from evolution import evaluator, generation, history, mutation, selection
from evolution.spec import SelectionSpec


def source():
    return dict(selection=dict(name="synthetic_selection", generations=3, elite_size=2, children_per_parent=1,
        max_candidates=8, training_seeds=[7,19], validation_seeds=[151,191,229,277,307,347]),
        base={"learning.a_plus":1.0}, mutations=[dict(name="learning.a_plus", factors=[1.5], min=.1, max=4.0)],
        validation=dict(bootstrap_samples=100))


def records(seeds, score, stable=True, retained=None, events=100):
    rows = []
    for seed in seeds:
        for group in evaluator.GROUPS:
            value = score if group in ["none","disable_homeostasis"] else 0
            delayed = (score if retained is None else retained) if group in ["none","disable_homeostasis"] else 0
            m = dict(sequence_score=value, retained_sequence_score=delayed, event_count=events)
            for prefix in ["", "retained_"]:
                m.update({prefix+"stability":int(stable), prefix+"frozen_replay_identical":1,
                    prefix+"weights_frozen":1, prefix+"firing_rate":10, prefix+"rates_finite":1,
                    prefix+"silent_fraction":.1, prefix+"persistent_fraction":0})
            rows.append(dict(job=dict(seed=seed, intervention=group), metrics=m, full_replay_checked=True))
    return rows


class FakeBackend:
    def __init__(self, validation_shift=0):self.new_evaluations=[];self.validation_shift=validation_shift
    def identity(self):return {"fixture":"deterministic-synthetic-not-neural"}
    def canonical(self, parameters, spec):
        parameters=mutation.normalize(parameters)
        cid="c"+hashlib.sha256(mutation.canonical_json(parameters).encode()).hexdigest()
        return dict(id=cid, parameters=parameters, effective_config=dict(parameters))
    def evaluate(self, candidate, seeds, root, phase, requirements, report_only=False):
        if phase=="validate":
            self.frozen=history.read(root.parent.parent/"frozen-finalist.json")
            assert self.frozen["training_only"]
        path=root/"fixture.json"
        if path.exists():return history.read(path)
        if report_only:raise ValueError("missing synthetic trial")
        self.new_evaluations.append((phase,list(seeds),candidate["id"]))
        value=2+candidate["parameters"]["learning.a_plus"]+(self.validation_shift if phase=="validate" else 0)
        assessment=evaluator.evaluate(records(seeds,value),seeds,requirements)
        history.immutable(path,assessment)
        return assessment


class PolicyTests(unittest.TestCase):
    def test_spec_rejects_overlap_code_mutation_and_unknown_policy(self):
        for case in range(5):
            s=source()
            if case==0:s["selection"]["validation_seeds"]=[7]
            if case==1:s["selection"]["training_seeds"]=[7,7]
            if case==2:s["mutations"][0]["name"]="network.connection_probability"
            if case==3:s["requirements"]={"ignore_stability":True}
            if case==4:s["mutations"][0]["factors"]=[0]
            with self.assertRaises(ValueError):SelectionSpec(s)

    def test_deterministic_mutations_and_machine_precision_bounds(self):
        spec=SelectionSpec(source());parent=FakeBackend().canonical(spec.base,spec)
        self.assertEqual(mutation.children(parent,spec,0),mutation.children(parent,spec,0))
        self.assertEqual(spec.base["learning.a_plus"],1.0)
        s=source();s["base"]["learning.a_plus"]=.1;s["mutations"]=[dict(name="learning.a_plus",values=[.2],min=.1,max=.2)]
        spec=SelectionSpec(s);parent=FakeBackend().canonical(spec.base,spec)
        self.assertEqual(len(mutation.children(parent,spec,0)[0]),1)

    def test_hard_gates_precede_quality_and_pareto_selection_preserves_tradeoffs(self):
        req=SelectionSpec(source()).requirements
        rejected=evaluator.evaluate(records([7,19],100,stable=False),[7,19],req)
        self.assertFalse(rejected["eligible"]);self.assertIsNone(rejected["quality"])
        self.assertFalse(evaluator.evaluate(records([7,19],3,retained=1),[7,19],req)["eligible"])
        candidates=[]
        for cid,q,cost,x in [("a",3,300,1),("b",2,100,4),("c",1,200,2)]:
            candidates.append(dict(id=cid,parameters={"learning.a_plus":x},evaluation=dict(eligible=True,quality=q,resource=cost)))
        candidates.append(dict(id="unstable",parameters={"learning.a_plus":3},evaluation=rejected))
        chosen,fronts=selection.select(candidates,2,source()["mutations"])
        self.assertEqual(chosen,["a","b"]);self.assertEqual(fronts,{"a":0,"b":0})

    def test_held_out_validation_requires_real_paired_gain_and_gates(self):
        spec=SelectionSpec(source());seeds=spec.validation
        baseline=evaluator.evaluate(records(seeds,2),seeds,spec.requirements)
        better=evaluator.evaluate(records(seeds,3),seeds,spec.requirements)
        self.assertTrue(evaluator.validate(better,baseline,spec.validation_policy)["held_out_quality_improved"])
        self.assertFalse(evaluator.validate(baseline,baseline,spec.validation_policy)["held_out_quality_improved"])
        bad=evaluator.evaluate(records(seeds,5,stable=False),seeds,spec.requirements)
        self.assertFalse(evaluator.validate(bad,baseline,spec.validation_policy)["held_out_quality_improved"])


class GenerationTests(unittest.TestCase):
    def test_evolution_improves_fixture_freezes_before_holdout_and_resumes(self):
        with TemporaryDirectory() as temporary:
            root=Path(temporary)/"search";s=source();spec=SelectionSpec(s);backend=FakeBackend()
            original=copy.deepcopy(spec.source)
            result=generation.run(spec,root,backend,"synthetic fixture\n")
            self.assertTrue(result["held_out_quality_improved"])
            self.assertGreater(result["candidate"]["parameters"]["learning.a_plus"],spec.base["learning.a_plus"])
            self.assertEqual(spec.source,original)
            self.assertEqual(len(list((root/"generations").glob("*.json"))),3)
            for phase,seeds,_ in backend.new_evaluations:self.assertEqual(seeds,spec.training if phase=="train" else spec.validation)
            before=len(backend.new_evaluations)
            self.assertEqual(generation.run(spec,root,backend,"synthetic fixture\n",True),result)
            self.assertEqual(len(backend.new_evaluations),before)
            with sqlite3.connect(root/"selection.sqlite") as db:
                self.assertEqual(db.execute("SELECT count(*) FROM generations").fetchone()[0],3)
                self.assertEqual(db.execute("SELECT count(*) FROM validation").fetchone()[0],2)

    def test_validation_outcome_does_not_change_training_history(self):
        with TemporaryDirectory() as temporary:
            spec=SelectionSpec(source());a=Path(temporary)/"good";b=Path(temporary)/"bad"
            first=generation.run(spec,a,FakeBackend(),"fixture")
            second=generation.run(spec,b,FakeBackend(-100),"fixture")
            self.assertTrue(first["held_out_quality_improved"]);self.assertFalse(second["held_out_quality_improved"])
            self.assertEqual(first["frozen"],second["frozen"])
            self.assertEqual([p.read_bytes() for p in sorted((a/"generations").glob("*.json"))],
                             [p.read_bytes() for p in sorted((b/"generations").glob("*.json"))])

    def test_no_eligible_population_stops_without_validation(self):
        with TemporaryDirectory() as temporary:
            s=source();s["requirements"]={"min_retained_score":100}
            backend=FakeBackend();result=generation.run(SelectionSpec(s),Path(temporary)/"search",backend,"fixture")
            self.assertEqual(result["validation_status"],"not_run_no_eligible_candidate")
            self.assertFalse(any(phase=="validate" for phase,_,_ in backend.new_evaluations))

    def test_candidate_budget_and_history_corruption_are_enforced(self):
        with TemporaryDirectory() as temporary:
            s=source();s["selection"]["max_candidates"]=2;spec=SelectionSpec(s);root=Path(temporary)/"search";backend=FakeBackend()
            result=generation.run(spec,root,backend,"fixture")
            self.assertEqual(result["training_candidates"],2)
            self.assertEqual(result["frozen"]["stop_reason"],"candidate_budget")
            path=root/"frozen-finalist.json";value=json.loads(path.read_bytes());value["record"]["winner"]="changed";path.write_text(json.dumps(value))
            with self.assertRaisesRegex(ValueError,"checksum differs"):generation.run(spec,root,backend,"fixture",True)


class NativeSelectionTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        subprocess.run(["cargo","build","--offline","--manifest-path",str(HERE/"Cargo.toml")],check=True,capture_output=True)
        cls.binary=HERE/"target/debug/nerva-research"

    def invoke(self,mode,root):
        return subprocess.run([sys.executable,str(HERE/"run.py"),mode,str(HERE/"config/selection_smoke.toml"),
            "--binary",str(self.binary),"--output",str(root)],capture_output=True,text=True)

    def test_native_selection_plan_run_sqlite_and_resume(self):
        with TemporaryDirectory() as temporary:
            root=Path(temporary)/"search"
            plan=self.invoke("select-plan",root)
            self.assertEqual(plan.returncode,0,plan.stderr)
            self.assertFalse((root/"training").exists())
            run=self.invoke("select",root)
            self.assertEqual(run.returncode,0,run.stdout+run.stderr)
            result=history.read(root/"final-result.json")
            self.assertFalse(result["original_model_modified"])
            self.assertFalse(result["validation_used_for_selection"])
            self.assertEqual(result["validation_status"],"completed")
            with sqlite3.connect(root/"selection.sqlite") as db:
                self.assertEqual(db.execute("SELECT count(*) FROM generations").fetchone()[0],2)
                self.assertEqual(db.execute("PRAGMA foreign_key_check").fetchall(),[])
            paths=list(root.glob("training/*/records/*.json"))+list(root.glob("validation/*/records/*.json"))
            before={str(p):p.read_bytes() for p in paths}
            report=self.invoke("selection-report",root)
            self.assertEqual(report.returncode,0,report.stdout+report.stderr)
            self.assertEqual(before,{str(p):p.read_bytes() for p in paths})
            self.assertEqual(history.read(root/"final-result.json"),result)
            path=paths[0];record=json.loads(path.read_bytes());record["record"]["metrics"]["sequence_score"]=999;path.write_text(json.dumps(record))
            corrupt=self.invoke("select",root)
            self.assertNotEqual(corrupt.returncode,0)
            self.assertIn("integrity",corrupt.stderr)


if __name__=="__main__":unittest.main()
