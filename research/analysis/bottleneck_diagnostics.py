"""Evidence-driven suspicions, not automatic claims of mechanism failure."""
from contracts import AnalysisReport

STAGES = ('perception', 'processing', 'learning', 'storage', 'readout', 'behavior')


def classify(observations):
    stages = {stage:dict(status='untested', evidence=[]) for stage in STAGES}
    hypotheses = []
    if observations.get('decoder_action_control_passed') is False:
        stages['processing'] = dict(status='decoder_positive_control_unresolved', evidence=['decoder_action_control_passed'])
        hypotheses.append(dict(id='probe_basis_or_alignment',
            next_test='within_network_held_out_probe_trials_with_action_positive_control', priority=120))
    if observations.get('input_information') is True:
        stages['perception'] = dict(status='signal_observed', evidence=['input_information'])
    elif observations.get('input_information') is False:
        stages['perception'] = dict(status='suspected_bottleneck', evidence=['input_information'])
        hypotheses.append(dict(id='sensory_coding', next_test='controlled_input_injection', priority=110))
    if observations.get('core_action_information') is True:
        stages['processing'] = dict(status='action_information_observed', evidence=['core_action_information'])
    if observations.get('input_information') is True and observations.get('core_action_information') is False:
        stages['processing'] = dict(status='suspected_bottleneck', evidence=['input_information', 'core_action_information'])
        hypotheses.append(dict(id='signal_propagation', next_test='paired_input_and_core_pathway_assay', priority=105))
    if observations.get('learned_effect_information') is True:
        stages['learning'] = dict(status='learning_signal_indicated', evidence=['learned_effect_information'])
    elif observations.get('weights_changed') is True:
        stages['learning'] = dict(status='weights_change_function_unresolved', evidence=['weights_changed'])
        hypotheses.append(dict(id='selectivity_or_representation', next_test='review_existing_m11_pilot_then_targeted_timing_test', priority=100))
    if observations.get('readout_gap') is True:
        stages['readout'] = dict(status='suspected_bottleneck', evidence=['readout_gap'])
        hypotheses.append(dict(id='readout_access', next_test='frozen_external_readout_on_fresh_seeds_and_weight_reset_controls', priority=110))
    if observations.get('old_skill_lost_after_new_learning') is True:
        if observations.get('task_a_acquired') is True and observations.get('task_b_acquired') is True:
            stages['storage'] = dict(status='interference_suspected', evidence=['task_a_acquired', 'task_b_acquired', 'old_skill_lost_after_new_learning'])
            hypotheses.append(dict(id='interference', next_test='matched_duration_freeze_after_a_and_nonoverlapping_b_controls', priority=100))
        else:
            stages['storage'] = dict(status='acquisition_controls_missing', evidence=['old_skill_lost_after_new_learning'])
            if observations.get('disjoint_tasks_acquired') is True:
                stages['learning'] = dict(status='overlap_acquisition_unresolved', evidence=['disjoint_tasks_acquired', 'old_skill_lost_after_new_learning'])
                hypotheses.append(dict(id='overlap_selectivity', next_test='counterbalanced_overlap_and_context_assay', priority=110))
    if observations.get('timing_specific') is True:
        hypotheses.append(dict(id='temporal_credit_assignment', next_test='delay_sweep_with_matched_exposure', priority=95))
    if observations.get('capacity_helpful') is False:
        hypotheses.append(dict(id='capacity_or_connectivity', next_test='matched_activity_capacity_and_connectivity_assay', priority=90))
    return AnalysisReport('bottleneck_diagnostics', dict(stages=stages,
        hypotheses=sorted(hypotheses, key=lambda h:(-h['priority'], h['id'])),
        mechanism_changes_allowed=False, automatic_parameter_mutation=False,
        untested_stages=[s for s in STAGES if stages[s]['status']=='untested']), [
        'Suspicions require targeted interventions. Unknown evidence is not a failed stage.',
        'A negative decoder result cannot identify a unique causal bottleneck.',
        'New learning mechanisms are never introduced automatically.'], status='diagnostic_hypotheses')
