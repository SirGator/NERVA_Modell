import csv,importlib.util,json,subprocess,unittest
from pathlib import Path
from tempfile import TemporaryDirectory
spec=importlib.util.spec_from_file_location('m8_run',Path(__file__).resolve().parents[1]/'run.py');m8=importlib.util.module_from_spec(spec);spec.loader.exec_module(m8)
ROOT=Path(__file__).resolve().parents[4]
class M8PlanTests(unittest.TestCase):
    def test_new_seeds_all_conditions_and_exact_sign_family_gate(self):
        self.assertEqual(len(m8.LEVELS),13);self.assertEqual(m8.plan('pilot')['seeds'],[500,501,502]);self.assertEqual(m8.plan('confirm')['seeds'],list(range(510,520)))
        self.assertLess(m8.sign_p([1]*10),.05/26)
        self.assertGreater(m8.sign_p([1]*9+[-1]),.05/26)
        self.assertGreater(m8.sign_p([1]*9+[0]),.05/26)
        self.assertEqual(m8.sign_p([0]*10),1)
class M8DataAuditTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.temp=TemporaryDirectory();cls.root=Path(cls.temp.name);cls.config=m8.cases(7,(8,8,8))[0];cfg=cls.root/'config.json';cfg.write_text(json.dumps(cls.config));cls.path=cls.root/'result.json'
        proc=subprocess.run(['./NERVA_Modell/Experimente/run.sh','m8','--config',str(cfg),'--output',str(cls.path)],cwd=ROOT,capture_output=True,text=True);assert proc.returncode==0,proc.stderr
    @classmethod
    def tearDownClass(cls):cls.temp.cleanup()
    def test_valid_runtime_and_fabricated_extra_spike_count_rejected(self):
        r=json.loads(self.path.read_text());audit,_=m8.validate(r,self.config,self.path);self.assertEqual(audit['branch_rows'],56)
        path=Path(str(self.path)+'.propagation.csv');before=path.read_bytes()
        try:
            with path.open(newline='') as f:reader=csv.DictReader(f);fields=reader.fieldnames;rows=list(reader)
            rows[0]['extra_core_spikes']=str(int(rows[0]['extra_core_spikes'])+1)
            with path.open('w',newline='') as f:w=csv.DictWriter(f,fieldnames=fields);w.writeheader();w.writerows(rows)
            with self.assertRaisesRegex(ValueError,'propagation/detail counts differ'):m8.validate(r,self.config,self.path)
        finally:path.write_bytes(before)
    def test_no_copy_control_cannot_include_unknown_neuron(self):
        r=json.loads(self.path.read_text());q=r['arms']['paired']['normal_mapping']['no_copy_probes']['checkpoint_probe'];q['core_events'].append([1,q['start_us']])
        with self.assertRaisesRegex(ValueError,'invalid no-copy event'):m8.validate(r,self.config)
if __name__=='__main__':unittest.main()
