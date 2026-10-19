import csv,importlib.util,json,subprocess,unittest
from pathlib import Path
from tempfile import TemporaryDirectory
spec=importlib.util.spec_from_file_location('m7',Path(__file__).resolve().parents[1]/'run.py');m7=importlib.util.module_from_spec(spec);spec.loader.exec_module(m7)
ROOT=Path(__file__).resolve().parents[4]
class M7AuditTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.temp=TemporaryDirectory();cls.root=Path(cls.temp.name)
        old=json.loads((ROOT/'Messungen/Experimente/m6-core-control/configs/core_D_off/seed-400.json').read_text())
        old['base'].update(case_id='m7_python_test',train_trials=8,test_trials=8,calibration_trials=8)
        cls.cfg=cls.root/'config.json';cls.cfg.write_text(json.dumps(old));cls.reference=cls.root/'reference.json';cls.output=cls.root/'m7'
        for command in [['./NERVA_Modell/Modell/run-demo.sh','m6','--config',str(cls.cfg),'--output',str(cls.reference)],['./NERVA_Modell/Experimente/run.sh','m7','--reference',str(cls.reference),'--output',str(cls.output)]]:
            proc=subprocess.run(command,cwd=ROOT,capture_output=True,text=True);assert proc.returncode==0,proc.stderr
    @classmethod
    def tearDownClass(cls):cls.temp.cleanup()
    def test_valid_neural_replay_and_wrong_nearest_partner(self):
        _,_,_,audit=m7.validate(self.output);self.assertEqual(audit['counterfactuals'],60);self.assertGreater(audit['pair_rows'],0)
        path=self.output/'pairs.csv';original=path.read_bytes()
        try:
            with path.open(newline='') as f:reader=csv.DictReader(f);fields=reader.fieldnames;rows=list(reader)
            rows[0]['pre_us']=str(int(rows[0]['pre_us'])+1)
            with path.open('w',newline='') as f:writer=csv.DictWriter(f,fieldnames=fields);writer.writeheader();writer.writerows(rows)
            with self.assertRaisesRegex(ValueError,'nearest recorded partner'):m7.validate(self.output)
        finally:path.write_bytes(original)
    def test_corrupted_retention_weight_is_rejected(self):
        path=self.output/'weights.csv';original=path.read_bytes()
        try:
            with path.open(newline='') as f:reader=csv.DictReader(f);fields=reader.fieldnames;rows=list(reader)
            row=next(row for row in rows if row['checkpoint']=='rest_2s');row['weight']=str(float(row['weight'])+.1)
            with path.open('w',newline='') as f:writer=csv.DictWriter(f,fieldnames=fields);writer.writeheader();writer.writerows(rows)
            with self.assertRaisesRegex(ValueError,'frozen weights drifted'):m7.validate(self.output)
        finally:path.write_bytes(original)
if __name__=='__main__':unittest.main()
