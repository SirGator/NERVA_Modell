#!/usr/bin/env python3
"""Independent MLD-02 checks: native counts, installed weights, local events and 2x2 contrasts."""
import argparse
from collections import Counter, defaultdict
import hashlib
import json
import math
from pathlib import Path

EPS=1e-9

def require(ok,message):
    if not ok: raise ValueError(message)

def norm(v): return math.sqrt(math.fsum(x*x for x in v))
def cosine(a,b):
    scale=norm(a)*norm(b)
    return math.fsum(x*y for x,y in zip(a,b))/scale if scale else 0.

def margins(frames,targets):
    return [cosine(f['core_counts'],targets[i%2])-cosine(f['core_counts'],targets[1-i%2]) for i,f in enumerate(frames[3:])]

def response(row,baseline,targets):
    frames=row['probe']['frames']; require(len(frames)==9,'nine fixed probe images required')
    require([f['image'] for f in frames]==[4,0,1,2,3,5,6,7,8],'probe images/order differ')
    require(len(row['margins'])==len(row['gains'])==6,'six cue measurements required')
    m=margins(frames,targets); old=margins(baseline['frames'],targets);g=[a-b for a,b in zip(m,old)]
    require(all(abs(a-b)<1e-12 for a,b in zip(m,row['margins'])),'cue margins differ from raw counts')
    require(all(abs(a-b)<1e-12 for a,b in zip(g,row['gains'])),'cue gains differ from raw counts')
    distinct=all(sum(t)>0 for t in targets) and cosine(*targets)<1-EPS
    quiet=sum(frames[0]['core_counts'])<=sum(baseline['frames'][0]['core_counts'])
    def gate(indices):return distinct and quiet and all(m[i]>EPS and g[i]>EPS and sum(frames[i+3]['core_counts'])>0 for i in indices)
    for key,indices in [('eligible',range(2)),('novel_eligible',range(2,6)),('all_cues_eligible',range(6))]:
        require(row[key]==gate(indices),'incorrect success gate: '+key)
    compared=different=different_times=0
    for left,right in zip(frames,baseline['frames']):
        old={n['id']:n for n in right['neurons']}
        for n in left['neurons']:
            if n['id'] in old:
                compared+=1;different+=n['stored_after']!=old[n['id']]['stored_after']
                different_times+=n['stored_after_time_us']!=old[n['id']]['stored_after_time_us']
    require(row['potential_effect']==(different>0),'stored potential flag differs')
    return {'stored_potentials':{'compared':compared,'different':different,'different_sample_times':different_times},'margins':m,'gains':g,'original_success':gate(range(2)),'novel_success':gate(range(2,6)),
            'all_cues_success':gate(range(6)),'core_effect':row['core_effect'],'motor_effect':row['motor_effect'],
            'potential_effect':row['potential_effect']}

def vector(row,initial):
    w=dict(row['weights']);require(set(w)==set(initial),'weight identities differ')
    require(all(0<=v<=2 for v in w.values()),'weight bounds violated')
    return [w[i]-initial[i] for i in initial]

def local_information(offline,reference,lesions):
    edges={s['synapse_id']:s for s in offline['original']['synapses']}
    incoming=defaultdict(list)
    for sid,s in edges.items(): incoming[s['target_id']].append(sid)
    pre={};post={};positive=Counter();negative=Counter();pre_count=Counter();post_count=Counter()
    events=[(t,0,index,sid) for index,(t,sid) in enumerate(offline['pre_arrivals'])]
    events += [(t,1,index,n) for index,(t,n) in enumerate(offline['post_spikes'])]
    for time,kind,_,identity in sorted(events):
        if kind==0:
            s=edges[identity];pre_count[identity]+=1
            if s['plastic'] and s['enabled'] and s['pre_excitatory']:
                if s['target_id'] in post and 0<=time-post[s['target_id']]<=100000:negative[identity]+=1
                pre[identity]=time
        else:
            post_count[identity]+=1
            for sid in incoming[identity]:
                s=edges[sid]
                if s['plastic'] and s['enabled'] and s['pre_excitatory'] and sid in pre and 0<=time-pre[sid]<=100000:positive[sid]+=1
            post[identity]=time
    necessities={l['synapse_id']:l['necessary_for_this_reference_gate'] for l in lesions}
    rows=[]
    for sid,desired in reference.items():
        if desired==0:continue
        s=edges[sid];allowed=s['plastic'] and s['enabled'] and s['pre_excitatory']
        requested=s['positive_requested']+s['negative_requested']
        rows.append({'synapse_id':sid,'source_id':s['source_id'],'target_id':s['target_id'],'reference_delta':desired,
                     'native_delta':s['weight']-s['initial_weight'],'rule_accepts':allowed,'pre_arrivals':pre_count[sid],
                     'post_spikes':post_count[s['target_id']],'positive_pair_opportunities':positive[sid],
                     'negative_pair_opportunities':negative[sid],'no_usable_pair_opportunity':allowed and positive[sid]+negative[sid]==0,
                     'native_requested':requested,'opposite_requested_direction':requested*desired<0,
                     'necessary_for_this_reference_gate':necessities.get(sid)})
    return rows

def cancellation(rows):
    absolute=math.fsum(s['positive_actual']-s['negative_actual'] for s in rows)
    opposed=math.fsum(min(s['positive_actual'],-s['negative_actual']) for s in rows)
    return {'absolute_update_mass':absolute,'opposing_mass':opposed,'cancelled_fraction':2*opposed/absolute if absolute else None,
            'opposing_synapses':sum(s['positive_actual']>0 and s['negative_actual']<0 for s in rows)}

def summarize_atlas(record):
    require(record['full_replay_checked'] is True,'complete replay required')
    d=record['details'];require(d['offline']['verified_original_local_state'] is True,'offline replay not verified')
    rows=d['observations'];initial={int(e[0]):e[4] for e in d['edges']};ids=list(initial)
    targets=[rows[0]['probe']['frames'][1]['core_counts'],rows[0]['probe']['frames'][2]['core_counts']]
    summaries=[response(r,rows[0]['probe'],targets) for r in rows]
    native={int(s[0]):s[3] for s in d['B_training']['weights_after']};residual=dict(d['B_training']['weight_residuals_after'])
    for s in d['offline']['original']['synapses']:
        require(s['weight']==native[s['synapse_id']] and s['residual']==residual[s['synapse_id']],'offline native state differs')
    normalized={}
    for name,levels in d['normalized'].items():
        direction=d['directions'][name];normalized[name]={}
        for level,p in levels.items():
            r={'executed':p['observation'] is not None,'reason':p['reason'],'requested_norm':p['requested_norm']}
            if p['observation'] is not None:
                require(p['faithful'] is True,'unfaithful norm installation executed')
                actual=vector(rows[p['observation']],initial);size=norm(actual)
                rel=abs(size-p['requested_norm'])/p['requested_norm'] if p['requested_norm'] else 0.
                require(rel<=.010000001 and cosine(actual,direction)>=.999-1e-12,'independent norm/direction check failed')
                r.update(summaries[p['observation']],actual_norm=size,relative_norm_error=rel,direction_cosine=cosine(actual,direction))
            normalized[name][level]=r
    ref=d['reference'];a=ref['observation'];b=ref['swap']
    verified=summaries[a]['original_success'] and all(x>y+EPS for x,y in zip(summaries[a]['gains'][:2],summaries[b]['gains'][:2]))
    require(verified==ref['verified'],'reference decision differs')
    rw=dict(rows[a]['weights']);reference={i:rw[i]-initial[i] for i in ids}
    for lesion in d['lesions']:
        lw=dict(rows[lesion['observation']]['weights']);sid=lesion['synapse_id']
        require(all(lw[i]==(initial[i] if i==sid else rw[i]) for i in ids),'single-edge ablation changed extra weights')
        require(lesion['necessary_for_this_reference_gate']==(not summaries[lesion['observation']]['original_success']),'necessity gate differs')
    cases={}
    for name,c in d['cases'].items():
        own=response(c['own'],c['reset_probe'],targets)
        require(c['matched_training_state'] is True,'unmatched live checkpoint')
        require(name=='D' or c['training']['input_digest']==d['B_training']['input_digest'],'unexpected input mismatch')
        audit=c['training']['audit']
        cases[name]={'own':own,'matched':summaries[c['observation']],'weight_changes':audit['weight_events'],
                     'blocked_positive':audit['blocked_positive'],'blocked_negative':audit['blocked_negative'],
                     'weight_paths':c['training']['weight_changes']}
    require(d['cases']['D']['training']['input_digest']!=d['B_training']['input_digest'],'distractor did not change images')
    cases['B']={'matched':summaries[d['B_observation']],'weight_changes':d['B_training']['audit']['weight_events'],
                'weight_paths':d['B_training']['weight_changes']}
    samples=d['offline']['chemical_samples'];chem={'samples':len(samples),'negative_learning_factors':sum(s[5]<0 for s in samples)}
    def paired(name,level='reference_strength'):
        v=normalized.get(name,{}).get(level,{})
        return v if v.get('executed') else None
    b_high,e_high=paired('B'),paired('E')
    p_high,n_high=paired('positive_contributions'),paired('negative_contributions')
    g_off,c_off=paired('offline_G'),paired('offline_C')
    sign_advantage=(p_high['all_cues_success'] and not n_high['all_cues_success'] and all(x>y+EPS for x,y in zip(p_high['gains'],b_high['gains']))) if p_high and n_high and b_high else None
    chemical_distribution=any(abs(x-y)>EPS for x,y in zip(g_off['gains'],c_off['gains'])) if g_off and c_off else None
    strength_insufficient=(e_high['all_cues_success'] and not b_high['original_success'] and not cases['G']['own']['original_success']) if b_high and e_high else None
    core=summaries[d['native_core_observation']];d_plan=d['distractor_at_native_core_strength']
    comparison=summaries[d_plan['observation']] if d_plan['observation'] is not None else None
    memory=core['all_cues_success'] and summaries[d['B_observation']]['all_cues_success'] and not cases['D']['own']['all_cues_success'] and comparison is not None and all(x>y+EPS for x,y in zip(core['gains'],comparison['gains']))
    require(float(memory)==record['metrics']['novel_core_memory_candidate'],'memory control gate differs')
    opportunities=local_information(d['offline'],reference,d['lesions'])
    return {'section':'atlas','seed':record['job']['seed'],'metrics':record['metrics'],'reference':{**ref,'response':summaries[a]},
            'cases':cases,'normalized':normalized,'common_norm':d['common_norm'],'native_core':core,
            'native_chemical_observations':chem,'local_information':opportunities,'lesions':d['lesions'],'lesions_truncated':d['lesions_truncated'],
            'cancellation':{name:cancellation(d['offline'][name]['synapses']) for name in ['original','gain64','constant_positive','pair' if 'pair' in d['offline'] else 'immediate_pair','positive','negative']},
            'decisions':{'positive_contribution_direction_advantage':sign_advantage,'chemical_distribution_changes_function_at_equal_norm':chemical_distribution,
                         'strength_alone_insufficient_at_reference_norm':strength_insufficient,'novel_core_memory_candidate':memory,
                         'potential_without_spike_effect':summaries[d['B_observation']]['potential_effect'] and not summaries[d['B_observation']]['core_effect'] and not summaries[d['B_observation']]['motor_effect'],
                         'reference_rule_excluded_edges':sum(not s['rule_accepts'] for s in opportunities) if verified else None,
                         'reference_edges_without_usable_pair':sum(s['no_usable_pair_opportunity'] for s in opportunities) if verified else None},
            'limits':['Reference support alone is not necessity; only recorded ablations test necessity at that configuration.',
                      'Null decisions mean missing/invalid matched controls, not negative findings. New cues share a fixed probe sequence with rest, not hard state resets.',
                      'G is a fixed 64-fold diagnostic intervention, not a test of every possible strength. Pair ignores the chemical learning factor.']}

def summarize_interaction(record):
    require(record['full_replay_checked'] is True,'interaction replay missing')
    d=record['details'];require(d['shared_checkpoint_verified'] is True,'shared development not checked')
    require(d['development']['formed']+d['development']['pruned']>0,'no developed structure')
    arms={}
    for name,a in d['arms'].items():
        require(a['common_checkpoint_verified'] is True and a['training']['topology_preserved'] is True,'common fixed structure violated')
        require(a['training']['weights_before']==d['arms']['H0_K0']['training']['weights_before'],'starting weights differ')
        own=response(a['own_weight_effect'],a['reset_probe'],d['fixed_targets'])
        require(all(abs(x-y)<1e-12 for x,y in zip(own['margins'],a['margins'])),'interaction margins differ')
        tr=a['training'];before=dict(tr['weights_before']);after=dict(tr['weights_after']);ledger={s['synapse_id']:s for s in tr['synapse_ledger']}
        require(all(abs(after[i]-w-ledger.get(i,{}).get('total_delta',0.))<1e-12 for i,w in before.items()),'interaction weight balance failed')
        require(all(abs(s['local_learning_delta']+s['competition_delta']-s['total_delta'])<1e-12 for s in ledger.values()),'mechanism attribution does not balance')
        arms[name]={'own':own,'normalizations':tr['normalizations'],'homeostasis_changes':sum(n['changes'] for n in tr['homeostasis_by_neuron']),
                    'weight_changes':tr['native_weight_changes'],'mass_balance_error':tr['mass_balance_error'],'pool_count':len(tr['pool_membership_and_budgets'])}
    require(d['arms']['H0_K1']['training']['pool_membership_and_budgets']==d['arms']['H1_K1']['training']['pool_membership_and_budgets'],'competition pools/budgets differ between arms')
    interaction=[arms['H1_K1']['own']['margins'][i]-arms['H1_K0']['own']['margins'][i]-arms['H0_K1']['own']['margins'][i]+arms['H0_K0']['own']['margins'][i] for i in range(6)]
    require(all(abs(x-y)<1e-12 for x,y in zip(interaction,d['interaction'])),'2x2 contrast differs')
    return {'section':'interaction','seed':record['job']['seed'],'development':d['development'],'arms':arms,'interaction':interaction,
            'nonadditive_at_declared_epsilon':[abs(v)>EPS for v in interaction],
            'limits':['Total mechanism interaction includes cellular and synaptic effects. Deterministic per-seed contrast is not a population significance test.']}

def analyze_file(path):
    raw=Path(path).read_bytes();r=json.loads(raw)['record']
    out=summarize_atlas(r) if r['details']['section']=='atlas' else summarize_interaction(r)
    out['record_sha256']=hashlib.sha256(raw).hexdigest();return out

def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('record',type=Path);a=p.parse_args()
    try:print(json.dumps(analyze_file(a.record),indent=2,sort_keys=True,allow_nan=False))
    except (OSError,ValueError,KeyError,TypeError) as e:p.exit(1,'mld02-analysis: '+str(e)+'\n')
if __name__=='__main__':main()
