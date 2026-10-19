import json
import csv
import sys
import unittest
from argparse import Namespace
from pathlib import Path
from tempfile import TemporaryDirectory
sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
from runner import make_manifest,bootstrap_paired,validate_record,do_report

class TestRunner(unittest.TestCase):
    def report_fixture(self, root, include_details=True):
        plan=make_manifest('confirm');plan['cases']=plan['cases'][:1]
        (root/'manifest.json').write_text(json.dumps(plan))
        for seed in plan['seeds']:
            arms={}
            for arm in plan['arms']:
                performance=.9 if arm=='paired' else .5
                arms[arm]=dict(balanced_accuracy=performance,visual_decodability=1.,action_selection_rate=0.,test_count=256)
                if include_details:
                    for mapping in ['normal_mapping','reversed_mapping']:
                        arms[arm][mapping]=dict(visual_decodability=1.,**{
                            stage:dict(count=dict(balanced_accuracy=performance))
                            for stage in ['probe','retained_probe','interference_probe']})
            record=dict(protocol=plan['protocol'],case_id=plan['cases'][0]['id'],seed=seed,arms=arms,
                        checks={key:True for key in ['same_initial_network','matched_experience','no_leakage','frozen_probe','balanced_labels']})
            path=root/'results'/record['case_id']/f'seed-{seed:02d}.json'
            path.parent.mkdir(parents=True,exist_ok=True);path.write_text(json.dumps(record))
        return plan
    def report_row(self, root):
        do_report(Namespace(output=root))
        with (root/'summary.csv').open() as file:
            return next(csv.DictReader(file))
    def test_manifest(self):
        plan=make_manifest('pilot')
        self.assertEqual(len(plan['cases']),20)
        self.assertEqual(plan['seeds'],[0,1,2])
        self.assertEqual(len(set(c['id'] for c in plan['cases'])),20)
        self.assertEqual(make_manifest('confirm')['seeds'],list(range(100,110)))
    def test_bootstrap(self):
        self.assertGreater(bootstrap_paired([.8,.9,.7],[.5,.6,.4])['ci95'][0],0)
    def test_validation_prevents_missing_controls(self):
        with self.assertRaises(ValueError):validate_record(dict(seed=0,case_id='x',protocol='NERVA-M4-v1',arms={}),0,'x')
    def test_report_rejects_empty_data_as_measurement(self):
        with TemporaryDirectory() as directory:
            root=Path(directory);(root/'manifest.json').write_text(json.dumps(make_manifest('pilot')))
            do_report(Namespace(output=root))
            self.assertFalse((root/'summary.csv').exists())
    def test_gate_has_only_simple_deterministic_sensors_and_two_delays(self):
        gate=make_manifest('gate')
        self.assertEqual(len(gate['cases']),4)
        self.assertEqual({c['config']['sensor'] for c in gate['cases']},{'effect_2ch','position_7ch'})
        self.assertEqual({c['config']['action_effect_delay_ms'] for c in gate['cases']},{8,20})
        self.assertTrue(all(c['config']['encoding']=='deterministic' for c in gate['cases']))
    def test_confirmation_needs_both_mappings_and_retention(self):
        with TemporaryDirectory() as directory:
            root=Path(directory);plan=self.report_fixture(root)
            self.assertEqual(self.report_row(root)['m4_criteria_met'],'True')
            for seed in plan['seeds']:
                path=root/'results'/plan['cases'][0]['id']/f'seed-{seed:02d}.json'
                record=json.loads(path.read_text())
                record['arms']['paired']['reversed_mapping']['retained_probe']['count']['balanced_accuracy']=.5
                path.write_text(json.dumps(record))
            self.assertEqual(self.report_row(root)['m4_criteria_met'],'False')
    def test_immediate_aggregates_without_mapping_data_do_not_confirm_learning(self):
        with TemporaryDirectory() as directory:
            root=Path(directory);self.report_fixture(root,include_details=False)
            row=self.report_row(root)
            self.assertEqual(row['preliminary_all_three_controls'],'True')
            self.assertEqual(row['m4_criteria_met'],'False')

if __name__=='__main__':unittest.main()
