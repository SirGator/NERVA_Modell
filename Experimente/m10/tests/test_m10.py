import copy
import importlib.util
import json
import shutil
import subprocess
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory

spec=importlib.util.spec_from_file_location("m10_run",Path(__file__).resolve().parents[1]/"run.py")
m10=importlib.util.module_from_spec(spec);spec.loader.exec_module(m10)
ROOT=Path(__file__).resolve().parents[4]


class ReplayAuditTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.temp=TemporaryDirectory();cls.folder=Path(cls.temp.name)
        cls.source=cls.folder/"source";cls.output=cls.folder/"output"
        shutil.copytree(ROOT/"Messungen/Experimente/m9-v2-smoke",cls.source)
        proc=subprocess.run(["python3","NERVA_Modell/Experimente/m10/run.py","run","--stage","smoke",
                             "--source",str(cls.source),"--output",str(cls.output)],cwd=ROOT,capture_output=True,text=True)
        if proc.returncode:
            cls.temp.cleanup();raise AssertionError(proc.stdout+proc.stderr)
        cls.path=cls.output/"traces/seed-692.json"
        cls.result=json.loads(cls.path.read_text())
        cls.reference=json.loads((cls.source/"results/weight4_tau100/seed-692.json").read_text())

    @classmethod
    def tearDownClass(cls):
        cls.temp.cleanup()

    def test_fixed_plan_all_sources_and_passive_trace(self):
        manifest=json.loads((self.output/"manifest.json").read_text())
        self.assertEqual(len(manifest["inputs"]),4)
        self.assertEqual(manifest["trace_case"],"weight4_tau100")
        self.assertIn("Retrospective",manifest["design"]["status"])
        audit=m10.validate(self.result,self.reference,self.path,self.source/"results/weight4_tau100/seed-692.json")
        self.assertEqual(audit["phases"],384)
        self.assertTrue(audit["historical_csv_byte_equal"])

    def test_tampered_voltage_impulse_and_time_are_rejected(self):
        def phase(result):
            return result["m9_result"]["arms"]["paired"]["normal_mapping"]["weight_reset_probes"]["stages"]["checkpoint_probe"]["sham"][0]["trace"][0]
        for field, change, error in [
            ("potentials_after",lambda values:values.__setitem__(0,values[0]+.1),"voltage"),
            ("signed_input_per_core",lambda values:values.__setitem__(0,values[0]+.1),"input sums"),
        ]:
            result=copy.deepcopy(self.result);change(phase(result)[field])
            with self.assertRaisesRegex(ValueError,error):m10.core.audit_trace(result,self.reference)
        result=copy.deepcopy(self.result);phase(result)["end_us"]+=1
        with self.assertRaisesRegex(ValueError,"clock"):m10.core.audit_trace(result,self.reference)

    def test_modified_source_and_modified_original_features_are_rejected(self):
        path=self.source/"results/reference/seed-692.json";original=path.read_bytes()
        try:
            path.write_bytes(original+b"\n")
            with self.assertRaisesRegex(ValueError,"source differs"):m10.load(self.output)
        finally:path.write_bytes(original)
        result=copy.deepcopy(self.result)
        result["m9_result"]["arms"]["paired"]["normal_mapping"]["checkpoint_probe"]["predictions_count"][0]*=-1
        with self.assertRaisesRegex(ValueError,"replay differs"):m10.core.audit_trace(result,self.reference)

    def test_retrospective_counts_and_common_time_measurements(self):
        old=m10.core.measure(self.reference)[0]
        traced=m10.core.measure(self.result["m9_result"],common_time=True)[0]
        self.assertEqual(len(old),64)
        self.assertEqual(len(traced),64)
        self.assertTrue(all("forecast_voltage_linf" in row for row in traced))
        self.assertTrue(all("forecast_voltage_linf" not in row for row in old))
        self.assertTrue(all(row["count_l1"]==0 for row in traced if row["arm"] in ["before","aged_frozen"]))


if __name__=="__main__":unittest.main()
