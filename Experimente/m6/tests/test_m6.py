import importlib.util
from pathlib import Path
from tempfile import TemporaryDirectory
from argparse import Namespace
import unittest
spec=importlib.util.spec_from_file_location('m6_run',Path(__file__).resolve().parents[1]/'run.py')
m6=importlib.util.module_from_spec(spec);spec.loader.exec_module(m6)

def synthetic_records():
    records=[]
    for _ in range(10):
        arms={}
        for arm in m6.DEFAULT_ARMS:
            value=.9 if arm=='paired' else .5
            maps={mapping:dict(visual_decodability=.95,**{stage:{feature:{'balanced_accuracy':value} for feature in ['count','time_bins']} for stage in m6.STAGES}) for mapping in m6.MAPS}
            maps['mapping_comparison']={stage:{'count':{'flip_rate':value,'dual_correct_rate':value}} for stage in m6.STAGES}
            arms[arm]=maps
        records.append(dict(arms=arms))
    return records

class M6RunnerTests(unittest.TestCase):
    def test_immediate_evidence_is_not_hidden_by_later_forgetting(self):
        records=synthetic_records()
        for r in records:
            for mapping in m6.MAPS:r['arms']['paired'][mapping]['retained_probe']['count']['balanced_accuracy']=.5
        self.assertTrue(m6.learning_contrasts('test',records,'probe')[0])
        self.assertFalse(m6.learning_contrasts('test',records,'retained_probe')[0])
    def test_one_mapping_or_motor_discrimination_alone_does_not_establish_learning(self):
        records=synthetic_records()
        for r in records:r['arms']['paired']['reversed_mapping']['probe']['count']['balanced_accuracy']=.5
        self.assertFalse(m6.learning_contrasts('test',records,'probe')[0])
        records=synthetic_records()
        for r in records:r['arms']['paired']['mapping_comparison']['probe']['count']['dual_correct_rate']=.5
        self.assertFalse(m6.learning_contrasts('test',records,'probe')[0])
    def test_preregistered_plan_is_resumable_and_uses_unseen_confirmation_seeds(self):
        self.assertEqual(m6.plan('confirm')['seeds'],list(range(300,310)))
        self.assertEqual(len(m6.cases(0)),8)
        with TemporaryDirectory() as folder:
            args=Namespace(stage='pilot',train_trials=256,test_trials=128,calibration_trials=128,output=Path(folder),command=None,jobs=4)
            m6.run(args);before=(Path(folder)/'manifest.json').read_bytes();m6.run(args)
            self.assertEqual(before,(Path(folder)/'manifest.json').read_bytes())
if __name__=='__main__':unittest.main()
