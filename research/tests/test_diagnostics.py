import copy
import csv
import hashlib
import json
from pathlib import Path
import sqlite3
import subprocess
import sys
from tempfile import TemporaryDirectory
import unittest

HERE = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(HERE))
from analysis.bottleneck_diagnostics import classify
from diagnostics import archive, decoder, runner
from diagnostics.spec import DiagnosisSpec
from evolution import history
from planning.diagnostic_controller import advance


def fixture(seed):
    result = dict(protocol='NERVA-M9-v2', seed=seed, case_id='fixture',
        checks={k:True for k in ['same_initial_network','same_actions_across_mappings','no_leakage',
                                'matched_experience','frozen_probe','balanced_labels',
                                'parent_not_calibrated_or_reset','zero_calibration_gap_on_parent']},
        initial_neurons=[dict(id=i, position=[0,0,0]) for i in [2,3]],
        initial_synapses=[dict(id=0, pre=2, post=3, weight=.1)], arms={})
    for arm in archive.ARMS:
        result['arms'][arm] = {}
        for mapping in archive.MAPPINGS:
            stages = {}
            for stage in archive.STAGES:
                start = 10 if stage == archive.STAGES[0] else 2000010
                trials = []
                for key in ['a','d']:
                    action = -1 if key == 'a' else 1
                    effect = action if mapping == archive.MAPPINGS[0] else -action
                    encoded = effect if arm == 'paired' else action
                    nid = 2 if encoded < 0 else 3
                    trials.append(dict(key=key, action_us=start, forecast_us=start+49999,
                        core_counts=[int(nid==2),int(nid==3)],core_events=[[nid,start+1000]],
                        prediction_count=1,changed_weights=[],effect_not_presented=True,
                        exact_replay_state_before_rest=True,frozen_parameters_preserved=True,
                        neurons_preserved_at_intervention=True,parent_unchanged=True))
                stages[stage] = dict(sham=trials)
            result['arms'][arm][mapping] = dict(trained_weights=[[0,.3 if arm=='paired' else .1]],
                                                weight_reset_probes=dict(stages=stages))
    return result


def setup_archive(folder, test_seeds=(3,4)):
    archive_root = folder/'archive'
    records = []
    for seed in [1,2,*test_seeds]:
        relative = f'results/fixture/seed-{seed}.json'
        path = archive_root/relative; path.parent.mkdir(parents=True,exist_ok=True)
        raw = json.dumps(fixture(seed)).encode();path.write_bytes(raw)
        records.append([relative,len(raw),hashlib.sha256(raw).hexdigest()])
    with (archive_root/'raw-sha256.csv').open('w',newline='') as stream:
        writer = csv.writer(stream);writer.writerow(['path','bytes','sha256']);writer.writerows(records)
    source = dict(diagnosis=dict(name='synthetic_diagnosis',archive='archive',case='fixture',
        training_seeds=[1,2],test_seeds=list(test_seeds),max_features=10))
    return DiagnosisSpec(source,folder)


class DecoderTests(unittest.TestCase):
    def test_closed_form_regularized_solution_and_constant_features(self):
        model = decoder.fit([[-1.],[1.]],[-1,1],1.)
        self.assertAlmostEqual(model['coefficients'][0],.5)
        self.assertEqual([decoder.predict(model,[-3]),decoder.predict(model,[3])],[-1,1])
        constant = decoder.fit([[0.],[0.]],[-1,1],1.)
        self.assertEqual(constant['coefficients'],[0.])

    def test_holdout_labels_and_extreme_values_do_not_change_fitted_models(self):
        with TemporaryDirectory() as temp:
            spec=setup_archive(Path(temp))
            _, method, rows=runner.prepare(spec)
            initial=copy.deepcopy(method.fit_models(rows))
            for row in rows:
                if row['partition']=='test':
                    row['effect'] *= -1
                    row['features']['counts']=[10**8*x for x in row['features']['counts']]
            self.assertEqual(initial,method.fit_models(rows))
            self.assertEqual(initial['paired:counts:effect']['means'],[.5,.5])

    def test_action_copy_alone_is_chance_and_learning_signal_needs_controls(self):
        with TemporaryDirectory() as temp:
            spec=setup_archive(Path(temp),range(3,15))
            _, method, rows=runner.prepare(spec);method.fit_models(rows)
            report=method.analyze(rows)
            scores={s['id']:s for s in report.findings['scores']}
            self.assertEqual(scores['paired:counts:effect']['test']['checkpoint_probe']['mean'],1.)
            self.assertEqual(scores['before:counts:effect']['test']['checkpoint_probe']['mean'],.5)
            self.assertEqual(scores['paired:action_only:effect']['test']['checkpoint_probe']['mean'],.5)
            self.assertTrue(any(c['readout_gap_indicated'] for c in report.findings['comparisons']))
            for row in rows:
                row['features']['counts']=[1 if row['action'] < 0 else 0,1 if row['action'] > 0 else 0]
                row['features']['temporal']=row['features']['counts']*5
            method.fit_models(rows)
            self.assertIsNone(method.analyze(rows).findings['observations']['learned_effect_information'])


class ArchiveTests(unittest.TestCase):
    def test_bad_seals_overlap_post_effect_events_and_confirmation_claim_are_rejected(self):
        with TemporaryDirectory() as temp:
            folder=Path(temp);spec=setup_archive(folder)
            data=fixture(1)
            trial=data['arms']['paired']['normal_mapping']['weight_reset_probes']['stages']['checkpoint_probe']['sham'][0]
            trial['core_events'][0][1]=trial['forecast_us']+1
            with self.assertRaisesRegex(ValueError,'pre-effect'):
                archive.extract(data,spec,'train',1)
            data=fixture(1)
            data['arms']['paired']['normal_mapping']['weight_reset_probes']['stages']['checkpoint_probe']['sham'][0]['effect_not_presented']=False
            with self.assertRaisesRegex(ValueError,'unsafe'):
                archive.extract(data,spec,'train',1)
            for key,value in [('test_seeds',[1,3]),('stage','confirmation')]:
                source=copy.deepcopy(spec.source);source['diagnosis'][key]=value
                with self.assertRaises(ValueError):DiagnosisSpec(source,folder)
            path=spec.archive/'results/fixture/seed-1.json';path.write_bytes(path.read_bytes()+b' ')
            with self.assertRaisesRegex(ValueError,'integrity'):
                runner.prepare(spec)

    def test_plan_run_resume_report_and_tampering(self):
        with TemporaryDirectory() as temp:
            folder=Path(temp);spec=setup_archive(folder);root=folder/'output'
            runner.plan(spec,root,'fixture')
            self.assertFalse((root/'frozen-decoders.json').exists())
            result=runner.run(spec,root,'fixture')
            self.assertFalse(result['independent_confirmation'])
            self.assertEqual(result['new_neural_trials'],0)
            self.assertEqual(len(result['report']['findings']['scores']),20)
            before=(root/'frozen-decoders.json').read_bytes()
            self.assertEqual(runner.run(spec,root,'fixture',True),result)
            self.assertEqual(before,(root/'frozen-decoders.json').read_bytes())
            with sqlite3.connect(root/'diagnosis.sqlite') as db:
                self.assertEqual(db.execute('SELECT count(*) FROM decoders').fetchone()[0],20)
                self.assertEqual(db.execute('PRAGMA integrity_check').fetchone()[0],'ok')
            envelope=json.loads(before);envelope['record']['training_seeds']=[3,4]
            (root/'frozen-decoders.json').write_text(json.dumps(envelope))
            with self.assertRaisesRegex(ValueError,'checksum'):
                runner.run(spec,root,'fixture',True)


class ControllerTests(unittest.TestCase):
    def test_unknown_stage_is_not_failure_and_interference_requires_both_acquisitions(self):
        report=classify({})
        self.assertEqual(report.findings['untested_stages'],['perception','processing','learning','storage','readout','behavior'])
        self.assertEqual(report.findings['hypotheses'],[])
        missing=classify(dict(old_skill_lost_after_new_learning=True))
        self.assertEqual(missing.findings['stages']['storage']['status'],'acquisition_controls_missing')
        full=classify(dict(old_skill_lost_after_new_learning=True,task_a_acquired=True,task_b_acquired=True))
        self.assertEqual(full.findings['hypotheses'][0]['id'],'interference')
        self.assertFalse(full.findings['mechanism_changes_allowed'])
        weak=classify(dict(decoder_action_control_passed=False,weights_changed=True))
        self.assertEqual(weak.findings['hypotheses'][0]['id'],'probe_basis_or_alignment')

    def test_stop_rules_do_not_promote_uninformative_pilots(self):
        self.assertEqual(advance('pilot',True,False)['action'],'stop_uninformative_hypothesis')
        self.assertFalse(advance('pilot',True,True,True,False)['run_confirmation'])
        self.assertTrue(advance('pilot',True,True,True,True)['run_confirmation'])
        self.assertEqual(advance('technical',False)['action'],'repair_assay')


class NativeContinualTests(unittest.TestCase):
    def test_native_continual_run_report_and_acquisition_gating(self):
        subprocess.run(['cargo','build','--offline','--manifest-path',str(HERE/'Cargo.toml')],check=True,capture_output=True)
        with TemporaryDirectory() as temp:
            root=Path(temp)/'continual'
            command=[sys.executable,str(HERE/'run.py'),'run',str(HERE/'config/diagnostic_continual_smoke.toml'),
                     '--binary',str(HERE/'target/debug/nerva-research'),'--output',str(root)]
            process=subprocess.run(command,capture_output=True,text=True)
            self.assertEqual(process.returncode,0,process.stdout+process.stderr)
            report=history.read(root/'continual-analysis.json')['findings']
            self.assertTrue(report['technical_checks_passed'])
            self.assertTrue(report['observations']['task_a_acquired'])
            self.assertTrue(report['observations']['disjoint_tasks_acquired'])
            self.assertFalse(report['observations']['task_b_acquired'])
            self.assertFalse(report['interference_assessable'])
            self.assertEqual(report['bottlenecks']['findings']['hypotheses'][0]['id'],'overlap_selectivity')
            paths=list((root/'records').glob('*.json'))
            before={p.name:p.read_bytes() for p in paths}
            command[2]='report'
            process=subprocess.run(command,capture_output=True,text=True)
            self.assertEqual(process.returncode,0,process.stderr)
            self.assertEqual(before,{p.name:p.read_bytes() for p in paths})


if __name__=='__main__':unittest.main()
