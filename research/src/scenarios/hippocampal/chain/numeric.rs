//! Native isolated local-rule arithmetic; the f64 accumulator is diagnostic only.
use super::{Outcome, cell, flag};
use crate::{Result, error};
use nerva::{
    config::LearningConfig,
    core::{ModulationState, NeuronId, SimTime, Synapse, SynapseId},
    learning::{PairStdp, PlasticityRule, ThreeFactorConfig},
    primitives::Weight,
};
use serde_json::{Value, json};

fn case(
    name: &str,
    weight: f32,
    signal: f32,
    elapsed: u64,
    parts: u64,
    tagged: bool,
    enabled: bool,
) -> Result<Value> {
    let mut post = cell(2, 0.1)?;
    post.set_modulation(
        SimTime::ZERO,
        ModulationState {
            learning_signal: signal,
            ..Default::default()
        },
    )
    .map_err(super::fail)?;
    let mut edge = Synapse::new(
        SynapseId(7),
        NeuronId(1),
        NeuronId(2),
        Weight::new(weight).map_err(super::fail)?,
        1000,
        true,
    )
    .map_err(super::fail)?;
    let mut rule = PairStdp::new(&LearningConfig {
        a_plus: 0.0001,
        a_minus: 0.00012,
        min_weight: 0.,
        max_weight: 2.,
        ..Default::default()
    })
    .with_three_factor(ThreeFactorConfig::default())
    .map_err(super::fail)?;
    if tagged {
        rule.on_pre_arrival(&mut edge, &post, SimTime(1000));
        rule.on_post_spike(&post, std::slice::from_mut(&mut edge), SimTime(2000));
    } else {
        rule.advance_time(&mut edge, &post, SimTime(2000), false);
    }
    let tag = edge.learning_eligibility();
    let ulp = f32::from_bits(weight.to_bits() + 1) - weight;
    let mut requested = 0.;
    let mut actual = 0.;
    let mut unchanged = 0;
    let mut clipped = 0;
    let mut rounded = 0;
    let mut reference = f64::from(weight);
    for index in 1..=parts {
        let at = SimTime(2000 + elapsed * index / parts);
        let before = edge.weight().get();
        let update = rule
            .delayed_update_diagnostics(&edge, &post, at, enabled)
            .ok_or_else(|| error("missing delayed arithmetic"))?;
        let cast = update.unrounded_delta as f32;
        let sum = before + cast;
        let bound = sum.clamp(0., 2.);
        rule.advance_time(&mut edge, &post, at, enabled);
        let after = edge.weight().get();
        if after != bound {
            return Err(error(
                "native update differs from independently staged f32 add/clamp",
            ));
        }
        requested += update.unrounded_delta;
        actual += f64::from(after) - f64::from(before);
        reference = (reference + update.unrounded_delta).clamp(0., 2.);
        unchanged += usize::from(update.unrounded_delta != 0. && before == after);
        clipped += usize::from(sum != bound);
        rounded += usize::from(update.unrounded_delta != 0. && sum == before && sum == bound);
    }
    Ok(
        json!({"case":name,"initial_weight":f64::from(weight),"tag_after_pair":f64::from(tag),
        "held_factor":f64::from(signal),"enabled":enabled,"elapsed_us":elapsed,"partitions":parts,
        "initial_positive_ulp":f64::from(ulp),"requested_signed_sum":requested,"actual_signed_sum":actual,
        "unchanged_requests":unchanged,"clipped_steps":clipped,"rounded_away_steps":rounded,
        "final_weight":f64::from(edge.weight().get()),"diagnostic_f64_accumulator":reference,
        "f64_accumulator_is_not_installed":true}),
    )
}
pub(super) fn run() -> Result<Outcome> {
    let cases = vec![
        case("representable", 0.5, 1., 100000, 1, true, true)?,
        case("sub_ulp_interval", 0.5, 1., 1, 1, true, true)?,
        case("tiny_factor", 0.5, 1e-8, 100000, 1, true, true)?,
        case("zero_factor", 0.5, 0., 100000, 1, true, true)?,
        case("upper_bound", 2., 1., 100000, 1, true, true)?,
        case("lower_bound", 0., -1., 100000, 1, true, true)?,
        case("no_tag", 0.5, 1., 100000, 1, false, true)?,
        case("frozen", 0.5, 1., 100000, 1, true, false)?,
    ];
    let single = case("one_interval", 0.5, 1., 100000, 1, true, true)?;
    let split = case("1000_intervals", 0.5, 1., 100000, 1000, true, true)?;
    let passed = cases[0]["actual_signed_sum"].as_f64().unwrap() > 0.
        && [1, 2]
            .iter()
            .all(|&i| cases[i]["rounded_away_steps"].as_u64().unwrap() > 0)
        && [3, 6, 7]
            .iter()
            .all(|&i| cases[i]["actual_signed_sum"] == 0.)
        && [4, 5]
            .iter()
            .all(|&i| cases[i]["clipped_steps"].as_u64().unwrap() > 0);
    Ok(Outcome {
        stages: [
            -1.,
            -1.,
            flag(cases[0]["tag_after_pair"].as_f64().unwrap() > 0.),
            flag(passed),
            -1.,
            -1.,
        ],
        passed,
        rows: 1009,
        training: 0,
        details: json!({"cases":cases,"partition_comparison":[single,split],
            "scope":"Isolated native PairStdp hooks and delayed arithmetic, no actor or runtime replacement.",
            "limits":"The same held factor and duration are used; f32 tag decay may itself depend on partitioning. Requested mass is measured separately in both branches."}),
    })
}
