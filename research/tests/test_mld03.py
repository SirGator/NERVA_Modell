import copy
from pathlib import Path
import sys
import tempfile
import unittest
sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
from mld03_analysis import scan, electrical
from mld03_study import parse, leaf


class Mld03Tests(unittest.TestCase):
    def probe(self):
        return {'complete':True,'parameters_preserved':True,'frames':[
            {'image':i,'start_us':0,'end_us':50,'spike_times':[[1,10]],'source_spike_counts':[[1,1]],
             'core_counts':[1],'motor_counts':[0],
             'integration_boundaries':[[10,1,.5,0.,1.,.5,True,False,0]]}
            for i in [4,0,1,2,3,5,6,7,8]]}

    def test_corrupted_spike_times_and_threshold_crossings_are_rejected(self):
        pop={'sensor':[0],'core':[1],'motor':[2]}
        p=self.probe();scan(p,pop)
        broken=copy.deepcopy(p);broken['frames'][3]['spike_times'][0][1]=11
        with self.assertRaisesRegex(ValueError,'missing core spike boundary'):scan(broken,pop)
        broken=copy.deepcopy(p);broken['frames'][3]['integration_boundaries'][0][4]=1.1
        with self.assertRaisesRegex(ValueError,'threshold crossing'):scan(broken,pop)

    def test_electrical_pairing_does_not_confuse_different_sample_times(self):
        a=self.probe();b=self.probe()
        a['frames'][0]['integration_boundaries'][0][2]=.75
        d=electrical(a,b)
        self.assertEqual(d['potential_before_delta_linf'],.25)
        a['frames'][0]['integration_boundaries'][0][0]=11
        d=electrical(a,b)
        self.assertEqual(d['potential_before_delta_linf'],0.)
        self.assertEqual(d['unmatched_or_ambiguous_boundaries'],1)

    def test_preregistered_followup_is_bounded_and_leaf_uses_correct_sections(self):
        source=(Path(__file__).resolve().parents[1]/'config/mld03_20261112.toml').read_text()
        with tempfile.TemporaryDirectory() as directory:
            path=Path(directory)/'spec.toml';path.write_text(source);spec=parse(path)
            generated=leaf(17027,spec['model'])
            self.assertIn('hippocampal_mld03',generated)
            self.assertIn('["excitability", "population"]',generated)
            for changed in [source.replace('560','559'),source.replace('workers = 2','workers = 3'),source.replace('31033','17027')]:
                path.write_text(changed)
                with self.assertRaises(ValueError):parse(path)


if __name__=='__main__':unittest.main()
