"""External linear probes of archived M9 activity, with seed-grouped holdout."""
import statistics
from contracts import AnalysisReport, ExperimentPlan
from diagnostics import archive, decoder
from evolution.evaluator import sign_p
from evolution.spec import require


def accuracy(rows, predictions, target):
    by_seed = {}
    for row, predicted in zip(rows, predictions):
        by_seed.setdefault(row['seed'], []).append((row[target], predicted))
    scores = {}
    for seed, pairs in by_seed.items():
        require({y for y, _ in pairs} == {-1, 1}, 'accuracy requires both classes per seed')
        scores[str(seed)] = statistics.mean(statistics.mean(int(y == p) for y, p in pairs if y == label)
                                             for label in (-1, 1))
    return dict(mean=statistics.mean(scores.values()), per_seed=scores, seeds=len(scores), rows=len(rows))


def difference(a, b):
    require(a.keys() == b.keys(), 'diagnostic comparison needs paired seeds')
    values = [a[s]-b[s] for s in sorted(a, key=int)]
    return dict(mean=statistics.mean(values), sign_p=sign_p(values), differences=values,
                wins=sum(x > 0 for x in values), losses=sum(x < 0 for x in values), ties=sum(x == 0 for x in values))


class DiagnosticDecoder:
    def __init__(self, spec):
        self.spec = spec

    def propose(self, context):
        require(tuple(self.spec.training) == context.seeds, 'decoder context differs from training partition')
        return ExperimentPlan('diagnostic_decoder', archive.source_files(self.spec), dict(
            case=self.spec.case, training_seeds=self.spec.training, test_seeds=self.spec.test,
            features=['counts', 'temporal', 'action_only'], ridge=self.spec.ridge, time_bins=self.spec.bins,
            max_new_trials=0, stage='retrospective_pilot',
            fitting='Immediate training probes only; one frozen scaler/decoder per arm, feature family and target',
            testing='Both mappings of held-out seeds; same frozen decoder before and after quiet delay'))

    def fit_models(self, rows):
        models = {}
        for arm in archive.ARMS:
            selected = [r for r in rows if r['partition'] == 'train' and r['arm'] == arm and r['stage'] == archive.STAGES[0]]
            require({r['seed'] for r in selected} == set(self.spec.training), 'missing training seeds')
            for family in ('counts', 'temporal', 'action_only'):
                for target in (('effect',) if family == 'action_only' else ('effect', 'action')):
                    models[f'{arm}:{family}:{target}'] = decoder.fit([r['features'][family] for r in selected],
                                                                    [r[target] for r in selected], self.spec.ridge)
        self.models = models
        return models

    def analyze(self, rows):
        require(hasattr(self, 'models'), 'decoders must be frozen before analysis')
        scores = []
        for key, model in sorted(self.models.items()):
            arm, family, target = key.split(':')
            train = [r for r in rows if r['partition'] == 'train' and r['arm'] == arm and r['stage'] == archive.STAGES[0]]
            tests = {}
            for stage in archive.STAGES:
                selected = [r for r in rows if r['partition'] == 'test' and r['arm'] == arm and r['stage'] == stage]
                require({r['seed'] for r in selected} == set(self.spec.test), 'missing test seeds')
                tests[stage] = accuracy(selected, [decoder.predict(model, r['features'][family]) for r in selected], target)
            scores.append(dict(id=key, arm=arm, features=family, target=target,
                training=accuracy(train, [decoder.predict(model, r['features'][family]) for r in train], target), test=tests))
        indexed = {r['id']:r for r in scores}
        fixed = {}
        for arm in archive.ARMS:
            fixed[arm] = {}
            for stage in archive.STAGES:
                selected = [r for r in rows if r['partition'] == 'test' and r['arm'] == arm and r['stage'] == stage]
                fixed[arm][stage] = accuracy(selected, [r['fixed_prediction'] for r in selected], 'effect')
        # All 16 screening comparisons are shown, without choosing a decoder on
        # test performance or treating previously examined seeds as fresh data.
        threshold = .05/16
        comparisons = []
        for family in ('counts', 'temporal'):
            for stage in archive.STAGES:
                paired = indexed[f'paired:{family}:effect']['test'][stage]
                controls = [indexed[f'{arm}:{family}:effect']['test'][stage] for arm in ('before', 'aged_frozen', 'scrambled')]
                control_max = {s:max(c['per_seed'][s] for c in controls) for s in paired['per_seed']}
                chance = {s:.5 for s in paired['per_seed']}
                above = difference(paired['per_seed'], chance)
                gain = difference(paired['per_seed'], control_max)
                gap = difference(paired['per_seed'], fixed['paired'][stage]['per_seed'])
                action = indexed[f'paired:{family}:action']['test'][stage]
                action_signal = difference(action['per_seed'], chance)
                learned = above['mean'] > 0 and above['sign_p'] < threshold and gain['mean'] > 0 and gain['sign_p'] < threshold
                comparisons.append(dict(features=family, stage=stage, paired_accuracy=paired['mean'],
                    above_chance=above, gain_over_controls=gain, external_minus_fixed=gap,
                    core_action_information=action_signal, screening_sign_threshold=threshold,
                    learned_effect_signal_indicated=learned,
                    readout_gap_indicated=learned and gap['mean'] > 0 and gap['sign_p'] < threshold))
        observations = dict(input_information=None,
            decoder_action_control_passed=any(c['core_action_information']['mean'] > 0
                and c['core_action_information']['sign_p'] < threshold for c in comparisons),
            core_action_information=True if any(c['core_action_information']['mean'] > 0
                and c['core_action_information']['sign_p'] < threshold for c in comparisons) else None,
            learned_effect_information=True if any(c['learned_effect_signal_indicated'] for c in comparisons) else None,
            readout_gap=True if any(c['readout_gap_indicated'] for c in comparisons) else None,
            weights_changed=any(r['core_weights_changed'] > 0 for r in rows if r['arm'] == 'paired'),
            old_skill_lost_after_new_learning=None, timing_specific=None, capacity_helpful=None)
        return AnalysisReport('diagnostic_decoder', dict(scores=scores, comparisons=comparisons,
            fixed_decoder=fixed, observations=observations, rows=len(rows), frozen_models=len(self.models),
            test_seed_count=len(self.spec.test), new_neural_trials=0), [
            'Retrospective seed-held-out pilot on already studied M9 data; not independent prospective confirmation.',
            'Negative linear decoding does not prove absence of information; nonlinear codes and cross-seed alignment remain alternatives.',
            'Action-only and untrained/frozen/scrambled arms control action-copy and pre-existing decodability.',
            'Both mappings are present in each split; mapping, labels, fixed predictions and post-effect activity are excluded from features.',
            'After-rest predictions use the same frozen immediate-training decoder, without refitting.',
            'Frozen quiet retention does not test continual learning, interference, generalization to new tasks or scaling.',
            'No NERVA weights, learning rules or deployed readout are modified.'], status='retrospective_pilot')
