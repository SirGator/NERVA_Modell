from pathlib import Path
import sys
import unittest
sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
from multilevel_report import stored_potential_difference,replay_summary,verify_responses

class MultilevelReportTests(unittest.TestCase):
    def test_input_ledger_difference_is_not_mistaken_for_a_membrane_difference(self):
        a={"frames":[{"neurons":[{"id":1,"stored_after":0.,"stored_after_time_us":10,"positive_input":1.}]}]}
        b={"frames":[{"neurons":[{"id":1,"stored_after":0.,"stored_after_time_us":10,"positive_input":2.}]}]}
        result=stored_potential_difference(a,b)
        self.assertEqual(result["different_stored_potentials"],0)
        self.assertEqual(result["compared_neuron_frames"],1)
        b["frames"][0]["neurons"][0]["stored_after_time_us"]=20
        self.assertEqual(stored_potential_difference(a,b)["different_sample_times"],1)

    def test_pair_requests_remain_unavailable_and_opposing_mass_is_kept(self):
        row={"initial_weight":1.,"weight":1.,"positive_actual":.2,"negative_actual":-.2,
             "weight_changes":2,"positive_tag_integral":0.,"negative_tag_integral":0.,
             "positive_requested":None,"negative_requested":None,"residual":0.}
        result=replay_summary([row])
        self.assertEqual(result["weight_l1"],0.)
        self.assertEqual(result["negative_actual_mass_share"],.5)
        self.assertIsNone(result["positive_requested"])

    def test_cue_gain_is_recomputed_from_counts_and_tampering_is_rejected(self):
        def frames(a,b):
            return [{"core_counts":v} for v in ([0,0],[1,0],[0,1],a,b)]
        rows=[{"probe":{"frames":frames([0,0],[0,0])},"completion_margins":[0.,0.],"margin_gains":[0.,0.]},
              {"probe":{"frames":frames([1,0],[0,1])},"completion_margins":[1.,1.],"margin_gains":[1.,1.]}]
        verify_responses(rows)
        rows[1]["margin_gains"][0]=0.
        with self.assertRaisesRegex(ValueError,"margin gain differs"):
            verify_responses(rows)

if __name__=="__main__":unittest.main()
