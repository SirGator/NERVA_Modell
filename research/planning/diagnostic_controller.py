"""Two tracks and staged stopping rules, without automatic mechanism edits."""


def portfolio():
    return [
        dict(id='representation', track='A', priority=100, stage='retrospective_pilot', status='executable',
             method='diagnostic_decoder', config='config/diagnosis_m9.toml', max_new_trials=0),
        dict(id='continual_learning', track='B', priority=95, stage='technical', status='executable',
             config='config/diagnostic_continual_smoke.toml', method='continual_sequences', maximum_trials=8,
             protocol=['untrained A/B probes', 'train A', 'frozen fork probe A/B', 'train B with plasticity', 'frozen fork probe A/B'],
             controls=['matched-duration no-B', 'freeze after A', 'untrained', 'nonoverlapping B'],
             requirements=['A acquired', 'B acquired', 'paired seeds', 'read-only probes', 'matched durations']),
        dict(id='timing', track='A', priority=90, stage='technical', status='task_adapter_required',
             protocol='Vary action-effect delay with identical actions/exposure; measure the pre-effect representation and weight-dependent readout'),
        dict(id='competition', track='A', priority=85, stage='pilot', status='existing_pilot_to_review',
             evidence='Messungen/Experimente/m11-pilot/',
             stop_rule='No further competition variants without informative weight-dependent functional gain'),
        dict(id='scaling', track='B', priority=70, stage='technical', status='task_adapter_required',
             protocol='Vary cells, connections and overlapping patterns; separate usable learning from firing activity'),
    ]


def advance(stage, technical_checks, informative=False, effect_supported=False, fresh_seeds=False):
    if not technical_checks:
        return dict(action='repair_assay', next_stage='technical', run_confirmation=False)
    if stage == 'technical':
        return dict(action='small_controlled_pilot', next_stage='pilot', run_confirmation=False)
    if stage == 'pilot' and not informative:
        return dict(action='stop_uninformative_hypothesis', next_stage=None, run_confirmation=False)
    if stage == 'pilot' and not effect_supported:
        return dict(action='target_discriminating_control', next_stage='pilot', run_confirmation=False)
    if stage == 'pilot' and effect_supported and fresh_seeds:
        return dict(action='freeze_criteria_then_confirm', next_stage='confirmation', run_confirmation=True)
    if stage == 'pilot':
        return dict(action='reserve_unused_confirmation_seeds', next_stage=None, run_confirmation=False)
    return dict(action='record_scoped_result', next_stage=None, run_confirmation=False)
