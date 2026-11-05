import copy
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
from tempfile import TemporaryDirectory
import tomllib
import unittest

HERE=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(HERE))
import localize_homeostasis as localization
from analysis.homeostasis_populations import protocol, analyze, SCOPES
from evolution import history


class PopulationProtocolTests(unittest.TestCase):
    def test_family_and_scope_matrix_are_fixed_before_data(self):
        source=tomllib.loads((HERE/'config/homeostasis_populations.toml').read_text())
        p=protocol(source)
        self.assertEqual(len(p['seeds']),24)
        self.assertEqual(p['scopes'],SCOPES)
        self.assertEqual(p['family_tests'],24)
        self.assertEqual(p['per_test_alpha'],.05/24)
        alternate=copy.deepcopy(source);alternate['parameters'][0]['values']=[1,37,4097]
        self.assertEqual(protocol(alternate)['family_tests'],18)
        for case in ['scope','duplicate','development','replay']:
            bad=copy.deepcopy(source)
            if case=='scope':bad['parameters'][1]['values']=['off','core','all']
            if case=='duplicate':bad['base']['mechanisms.homeostasis']=True
            if case=='development':bad['base']['mechanisms.development']=True
            if case=='replay':bad['experiment']['verify_replay']=False
            with self.subTest(case=case),self.assertRaises(ValueError):protocol(bad)


class NativePopulationTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        subprocess.run(['cargo','build','--offline','--manifest-path',str(HERE/'Cargo.toml')],check=True,capture_output=True)
        cls.binary=HERE/'target/debug/nerva-research'
        cls.temp=TemporaryDirectory();cls.addClassCleanup(cls.temp.cleanup)
        cls.config=Path(cls.temp.name)/'fixture.toml'
        text=(HERE/'config/homeostasis_populations.toml').read_text()
        source=tomllib.loads(text)
        cls.sizes=[int(s.strip()) for s in os.environ.get('NERVA_TEST_SIZES','17,65').split(',')]
        text=text.replace(source['experiment']['name'],'population_fixture')
        text=text.replace('seeds = '+str(source['experiment']['seeds']),'seeds = [11, 19]')
        text=text.replace('values = [17, 256, 1025, 4096]','values = '+json.dumps(cls.sizes))
        text=text.replace('max_runs = 2304','max_runs = '+str(48*len(cls.sizes)))
        text=text.replace('max_total_neurons = 100000','max_total_neurons = '+str(max(cls.sizes)+11))
        cls.config.write_text(text)
        cls.reference=Path(cls.temp.name)/'reference'
        localization.execute('plan',cls.config,cls.reference,cls.binary,audit_roots=[])
        assert not list((cls.reference/'records').glob('*.json'))
        cls.registration=(cls.reference/'preregistration.json').read_bytes()
        cls.result=localization.execute('run',cls.config,cls.reference,cls.binary,audit_roots=[])
        cls.rows=[json.loads(p.read_text())['record'] for p in (cls.reference/'records').glob('*.json')]

    def test_native_scope_audits_contrasts_and_phase_diagnostics(self):
        self.assertTrue(self.result['technical_checks_passed'])
        self.assertEqual(self.result['trials'],48*len(self.sizes))
        self.assertEqual([c['processing_neurons'] for c in self.result['cells']],self.sizes)
        for cell in self.result['cells']:
            self.assertEqual(set(cell['means']),set(SCOPES))
            self.assertEqual([c['name'] for c in cell['contrasts']],['core_minus_off','sensor_added','motor_copy_added'])
            self.assertEqual(len({m['root_sensor_rate_hz'] for m in cell['means'].values()}),1)
            self.assertEqual(cell['means']['core']['sensor_intrinsic_current'],0)
            self.assertEqual(cell['means']['core_sensor']['motor_intrinsic_current'],0)
            self.assertEqual(cell['means']['off']['core_homeostasis_updates'],0)
        self.assertEqual((self.reference/'preregistration.json').read_bytes(),self.registration)

    def test_report_preserves_raw_records_registration_and_result(self):
        with TemporaryDirectory() as temp:
            root=Path(temp)/'archive';shutil.copytree(self.reference,root)
            before={p.name:p.read_bytes() for p in (root/'records').glob('*.json')}
            result=localization.execute('report',self.config,root,self.binary,audit_roots=[])
            self.assertEqual(result,self.result)
            self.assertEqual(before,{p.name:p.read_bytes() for p in (root/'records').glob('*.json')})
            self.assertEqual((root/'preregistration.json').read_bytes(),self.registration)

    def test_excluded_population_mutation_is_rejected(self):
        rows=copy.deepcopy(self.rows)
        row=next(r for r in rows if r['job']['parameters']['homeostasis.scope']=='core' and r['job']['intervention']=='active')
        row['metrics']['sensor_homeostasis_updates']=1
        with self.assertRaisesRegex(ValueError,'populations'):analyze(rows,tomllib.loads(self.config.read_text()))

    def test_generated_input_difference_is_rejected(self):
        rows=copy.deepcopy(self.rows)
        rows[0]['details']['training_curve'][0]['root_sensor_spikes']+=1
        with self.assertRaisesRegex(ValueError,'root-generated'):analyze(rows,tomllib.loads(self.config.read_text()))

    def test_missing_data_or_late_registration_are_not_repaired(self):
        rows=copy.deepcopy(self.rows);rows.pop()
        with self.assertRaisesRegex(ValueError,'complete paired'):analyze(rows,tomllib.loads(self.config.read_text()))
        for case in ['missing','late']:
            with self.subTest(case=case),TemporaryDirectory() as temp:
                root=Path(temp)/'archive';shutil.copytree(self.reference,root)
                record=next((root/'records').glob('*.json'))
                if case=='missing':record.unlink()
                else:(root/'preregistration.json').unlink()
                with self.assertRaises(ValueError):
                    localization.execute('report' if case=='missing' else 'plan',self.config,root,self.binary,audit_roots=[])
                if case=='missing':self.assertFalse(record.exists())


if __name__=='__main__':unittest.main()
