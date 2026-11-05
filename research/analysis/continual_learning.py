"""Interpret acquisition controls before considering an interference claim."""
from dataclasses import asdict
from contracts import ExperimentPlan, AnalysisReport
from evolution.spec import require
from .bottleneck_diagnostics import classify

GROUPS = ('none', 'no_b', 'freeze_after_a', 'disjoint_b')


class ContinualLearning:
    def propose(self, context):
        return ExperimentPlan('continual_learning', [dict(seed=s,intervention=g) for s in context.seeds for g in GROUPS],
            dict(scenario='continual_sequences', stage='technical', repetitions=context.base.get('training_repetitions',24),
                 controls=list(GROUPS), success='Both A and B must be acquired before a forgetting interpretation'))

    def analyze(self, results):
        indexed = {(r['job']['seed'],r['job']['intervention']):r for r in results}
        seeds = sorted({s for s,_ in indexed})
        require(len(indexed) == len(results) and set(indexed) == {(s,g) for s in seeds for g in GROUPS},
                'continual diagnosis needs all paired seed/control records')
        require(all(r['full_replay_checked'] is True for r in results), 'continual diagnosis requires full replay')
        technical = all(all(r['metrics'][m] == 1 for m in ('stability','weights_frozen','probe_replay_identical')) for r in results)
        cases = []
        for seed in seeds:
            metrics = {g:indexed[seed,g]['metrics'] for g in GROUPS}
            cases.append(dict(seed=seed, conditions=metrics,
                target_a_acquired=metrics['none']['a_acquired'] == 1,
                target_b_acquired=metrics['none']['b_acquired'] == 1,
                disjoint_both_acquired=metrics['disjoint_b']['a_acquired'] == metrics['disjoint_b']['b_acquired'] == 1,
                loss_beyond_controls=metrics['none']['a_loss'] > max(metrics['no_b']['a_loss'],metrics['freeze_after_a']['a_loss'])))
        observations = dict(task_a_acquired=all(c['target_a_acquired'] for c in cases),
            task_b_acquired=all(c['target_b_acquired'] for c in cases),
            disjoint_tasks_acquired=all(c['disjoint_both_acquired'] for c in cases),
            old_skill_lost_after_new_learning=True if technical and all(c['loss_beyond_controls'] for c in cases) else None)
        diagnosis = asdict(classify(observations))
        assessable = technical and observations['task_a_acquired'] and observations['task_b_acquired']
        return AnalysisReport('continual_learning',dict(technical_checks_passed=technical,seeds=seeds,cases=cases,
            observations=observations,interference_assessable=assessable,
            interference_indicated=assessable and all(c['loss_beyond_controls'] for c in cases),
            bottlenecks=diagnosis, independent_confirmation=False),[
                'Two-sequence technical assay on a small complete recurrent graph; not the M9 action-effect task.',
                'Processing cells receive explicit input pulses; sensory coding is intentionally bypassed.',
                'Probes use frozen forks; A/B training remains plastic except in the declared freeze control.',
                'A score drop when B itself is not acquired is not a confirmed forgetting result.',
                'Pattern overlap can cause competing transitions; controls do not prove a unique mechanism.',
                'No new learning rule or automatic deployment.'],status='technical_assay')
