import importlib.util
import json
from pathlib import Path
from tempfile import TemporaryDirectory
from argparse import Namespace
import unittest

spec=importlib.util.spec_from_file_location('m5_run',Path(__file__).resolve().parents[1]/'run.py')
m5=importlib.util.module_from_spec(spec);spec.loader.exec_module(m5)

class M5RunnerTests(unittest.TestCase):
    def test_preregistered_plan_can_resume_and_uses_fresh_confirmation_seeds(self):
        with TemporaryDirectory() as folder:
            args=Namespace(stage='pilot',train_trials=256,test_trials=128,calibration_trials=128,
                           output=Path(folder),command=None,jobs=4)
            m5.run(args);before=(Path(folder)/'manifest.json').read_bytes();m5.run(args)
            self.assertEqual(before,(Path(folder)/'manifest.json').read_bytes())
        self.assertEqual(m5.plan('confirm')['seeds'],list(range(200,210)))
        self.assertEqual(len(m5.cases(0)),16)
    def test_duplicate_motor_controls_cannot_replace_both_actions_and_frozen_arms(self):
        config=m5.cases(0)[0]
        record=dict(protocol='NERVA-M5-v1',requested_config=config,seed=0,case_id=config['base']['case_id'],
            checks={key:True for key in ['same_initial_network','identical_neutral_images','fixed_copy_pulse_count_and_timing','frozen_control_preserved']},
            observations=[dict(action='a',learning=True,generated_and_emitted_visual_spikes=0,copy_spikes=3,copy_arrivals=9,core_weight_update_count=0)]*6)
        with self.assertRaises(ValueError):m5.validate(record,config)

if __name__=='__main__':unittest.main()
