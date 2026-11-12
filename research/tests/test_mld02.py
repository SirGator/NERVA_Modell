import copy
from pathlib import Path
import sys
import tempfile
import unittest
sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
from mld02_analysis import response,local_information,cancellation
from mld02_study import parse

class Mld02Tests(unittest.TestCase):
    def test_novel_cues_cannot_inherit_success_from_original_cues(self):
        order=[4,0,1,2,3,5,6,7,8]
        baseline={'frames':[{'image':k,'core_counts':([1,0] if i==1 else [0,1] if i==2 else [0,0]),'neurons':[]} for i,k in enumerate(order)]}
        candidate=copy.deepcopy(baseline)
        for i in range(3,9):candidate['frames'][i]['core_counts']=[1,0] if (i-3)%2==0 else [0,1]
        row={'probe':candidate,'margins':[1.]*6,'gains':[1.]*6,'eligible':True,'novel_eligible':True,'all_cues_eligible':True,'core_effect':True,'motor_effect':False,'potential_effect':False}
        self.assertTrue(response(row,baseline,[[1,0],[0,1]])['all_cues_success'])
        candidate['frames'][5]['core_counts']=[0,0]
        with self.assertRaisesRegex(ValueError,'cue margins differ'):response(row,baseline,[[1,0],[0,1]])

    def test_local_pair_opportunities_respect_phase_order_and_polarity(self):
        def edge(i,excit):return {'synapse_id':i,'source_id':i,'target_id':20,'plastic':True,'enabled':True,'pre_excitatory':excit,'positive_requested':.1,'negative_requested':-.2,'weight':.4,'initial_weight':.5}
        tape={'original':{'synapses':[edge(1,True),edge(2,False)]},'pre_arrivals':[[1000,1],[1500,2],[3000,1]],'post_spikes':[[1000,20],[2000,20]]}
        rows=local_information(tape,{1:.1,2:.1},[])
        self.assertEqual(rows[0]['positive_pair_opportunities'],2)
        self.assertEqual(rows[0]['negative_pair_opportunities'],1)
        self.assertTrue(rows[0]['opposite_requested_direction'])
        self.assertFalse(rows[1]['rule_accepts'])
        self.assertEqual(rows[1]['positive_pair_opportunities'],0)
        self.assertIsNone(rows[0]['necessary_for_this_reference_gate'])

    def test_cancellation_is_within_synapses_not_between_unrelated_edges(self):
        r=cancellation([{'positive_actual':1.,'negative_actual':0.},{'positive_actual':0.,'negative_actual':-1.}])
        self.assertEqual(r['cancelled_fraction'],0.)
        r=cancellation([{'positive_actual':1.,'negative_actual':-1.}])
        self.assertEqual(r['cancelled_fraction'],1.)

    def test_underbudget_duplicate_and_unbounded_plans_are_rejected(self):
        source=(Path(__file__).resolve().parents[1]/'config/mld02_20261112.toml').read_text()
        with tempfile.TemporaryDirectory() as directory:
            p=Path(directory)/'spec.toml';p.write_text(source)
            self.assertEqual(len(parse(p)['study']['confirmation_seeds']),3)
            for modified in [source.replace('980','979'),source.replace('workers = 2','workers = 3'),source.replace('31033','17027')]:
                p.write_text(modified)
                with self.assertRaises(ValueError):parse(p)

if __name__=='__main__':unittest.main()
