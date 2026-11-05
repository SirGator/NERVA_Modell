import copy
import json
from pathlib import Path
import shutil
import subprocess
import sys
from tempfile import TemporaryDirectory
import tomllib
import unittest

HERE=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(HERE))
import confirm_homeostasis as confirmation
from analysis.homeostasis_confirmation import endpoint, protocol, analyze
from evolution import history


class ConfirmationPolicyTests(unittest.TestCase):
    def test_exact_sign_tests_keep_ties_and_both_endpoint_directions(self):
        positive=endpoint([.25]*24,False,.00625,7)
        self.assertEqual(positive['sign_p'],2**-24)
        self.assertTrue(positive['confirmed'])
        self.assertEqual(positive['bootstrap_ci95'],[.25,.25])
        self.assertFalse(endpoint([-.25]*24,False,.00625,7)['confirmed'])
        tied=endpoint([.25]*12+[0.]*12,False,.00625,7)
        self.assertEqual(tied['pairs'],24)
        self.assertEqual(tied['ties'],12)
        self.assertFalse(tied['confirmed'])
        negative=endpoint([-.25]*24,True,.00625,7)
        self.assertTrue(negative['confirmed'])
        self.assertEqual(negative['sign_p'],2**-23)
        self.assertEqual(negative['mean_difference'],-.25)

    def test_protocol_binds_family_and_development(self):
        source=tomllib.loads((HERE/'config/homeostasis_confirmation.toml').read_text())
        p=protocol(source)
        self.assertEqual(len(p['seeds']),24)
        self.assertEqual(p['sizes'],[17,256,1025,4096])
        self.assertEqual(p['family_tests'],8)
        self.assertEqual(p['per_test_alpha'],.00625)
        for case in ['development','replay','controls','axis','name']:
            bad=copy.deepcopy(source)
            if case=='development':bad['base']['mechanisms.development']=True
            if case=='replay':bad['experiment']['verify_replay']=False
            if case=='controls':bad['experiment']['interventions'].pop()
            if case=='axis':bad['parameters'][1]['values']=[0,1]
            if case=='name':bad['experiment']['name']='../outside'
            with self.subTest(case=case),self.assertRaises(ValueError):protocol(bad)

    def test_seed_audit_includes_reserved_seeds_and_rejects_overlap(self):
        with TemporaryDirectory() as temp:
            temp=Path(temp);archive=temp/'catalog';archive.mkdir()
            (archive/'harness-manifest.json').write_text(json.dumps(dict(record=dict(specification=dict(
                research=dict(training_seeds=[11],validation_seeds=[19]))))))
            result=confirmation.audit_seeds([29],temp/'new',temp/'input.toml',[archive])
            self.assertEqual(result['known_seed_count'],2)
            with self.assertRaisesRegex(ValueError,'overlap'):
                confirmation.audit_seeds([19],temp/'new',temp/'input.toml',[archive])


class NativeConfirmationTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        subprocess.run(['cargo','build','--offline','--manifest-path',str(HERE/'Cargo.toml')],check=True,capture_output=True)
        cls.binary=HERE/'target/debug/nerva-research'
        cls.temp=TemporaryDirectory();cls.addClassCleanup(cls.temp.cleanup)
        cls.config=Path(cls.temp.name)/'fixture.toml'
        source=tomllib.loads((HERE/'config/homeostasis_confirmation.toml').read_text())
        original=(HERE/'config/homeostasis_confirmation.toml').read_text()
        text=original.replace(source['experiment']['name'],'confirmation_fixture')
        text=text.replace('seeds = '+str(source['experiment']['seeds']),'seeds = [11, 19]')
        text=text.replace('values = [17, 256, 1025, 4096]','values = [17, 65]')
        text=text.replace('max_runs = 1152','max_runs = 48')
        cls.config.write_text(text)
        cls.reference=Path(cls.temp.name)/'reference'
        confirmation.execute('plan',cls.config,cls.reference,cls.binary,audit_roots=[])
        assert not list((cls.reference/'records').glob('*.json'))
        cls.registration=(cls.reference/'preregistration.json').read_bytes()
        confirmation.execute('run',cls.config,cls.reference,cls.binary,audit_roots=[])

    def test_preregistration_native_execution_and_readonly_reanalysis(self):
        with TemporaryDirectory() as temp:
            root=Path(temp)/'archive';shutil.copytree(self.reference,root)
            self.assertEqual((root/'preregistration.json').read_bytes(),self.registration)
            receipt=history.read(root/'verification-receipt.json')
            self.assertEqual(receipt['record_count'],48)
            self.assertEqual(history.read(root/'preregistration.json')['records_at_registration'],0)
            before={p.name:p.read_bytes() for p in (root/'records').glob('*.json')}
            result=confirmation.execute('report',self.config,root,self.binary,audit_roots=[])
            self.assertEqual(result['protocol']['family_tests'],4)
            self.assertEqual(result['protocol']['seeds'],[11,19])
            self.assertEqual(before,{p.name:p.read_bytes() for p in (root/'records').glob('*.json')})
            self.assertEqual(history.read(root/'verification-receipt.json'),receipt)
            self.assertEqual((root/'preregistration.json').read_bytes(),self.registration)

    def test_confirmatory_analysis_does_not_exclude_poor_readouts(self):
        rows=[json.loads(p.read_text())['record'] for p in (self.reference/'records').glob('*.json')]
        for r in rows:
            h=r['job']['parameters']['mechanisms.homeostasis']
            if not r['job']['intervention'].endswith('_frozen'):
                r['metrics']['visual_accuracy']=0. if h else .25
        result=analyze(rows,tomllib.loads(self.config.read_text()))
        self.assertTrue(result['technical_checks_passed'])
        self.assertTrue(all(c['both_active_readouts_pass_half_accuracy']==0 for c in result['cells']))
        self.assertTrue(all(c['visual_loss']['pairs']==2 for c in result['cells']))
        rows.pop()
        with self.assertRaisesRegex(ValueError,'complete paired'):analyze(rows,tomllib.loads(self.config.read_text()))

    def test_changed_sealed_record_and_late_registration_are_rejected(self):
        for case in ['changed','late']:
            with self.subTest(case=case),TemporaryDirectory() as temp:
                root=Path(temp)/'archive';shutil.copytree(self.reference,root)
                record=next((root/'records').glob('*.json'))
                if case=='changed':record.write_text('{}')
                else:(root/'preregistration.json').unlink()
                before=record.read_bytes()
                with self.assertRaises(ValueError):
                    confirmation.execute('report' if case=='changed' else 'plan',self.config,root,self.binary,audit_roots=[])
                self.assertEqual(record.read_bytes(),before)


if __name__=='__main__':unittest.main()
