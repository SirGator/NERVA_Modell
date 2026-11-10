//! Native fixed calibration; accumulator state is measured rather than emulated.
use crate::{Result, error};
use nerva::{
    config::{LearningConfig, NeuronConfig},
    core::{ModulationState, Neuron, NeuronId, Polarity, SimTime, Synapse, SynapseId},
    learning::{PairStdp, PlasticityRule, ThreeFactorConfig, WeightUpdateMode},
    math::Position3D,
    primitives::Weight,
};
use serde_json::{Value, json};
fn run_case(mode: WeightUpdateMode, parts: u64) -> Result<Value> {
    let mut post = Neuron::new(
        NeuronId(2),
        Position3D::ORIGIN,
        Polarity::Excitatory,
        None,
        NeuronConfig::default(),
        SimTime::ZERO,
    )
    .map_err(super::super::fail)?;
    post.set_modulation(
        SimTime::ZERO,
        ModulationState {
            learning_signal: 1.,
            ..Default::default()
        },
    )
    .map_err(super::super::fail)?;
    let mut edge = Synapse::new(
        SynapseId(7),
        NeuronId(1),
        NeuronId(2),
        Weight::new(0.5).unwrap(),
        1000,
        true,
    )
    .map_err(super::super::fail)?;
    let mut rule = PairStdp::new(&LearningConfig {
        a_plus: 0.0001,
        a_minus: 0.00012,
        min_weight: 0.,
        max_weight: 2.,
        ..Default::default()
    })
    .with_three_factor(ThreeFactorConfig::default())
    .map_err(super::super::fail)?
    .with_weight_update_mode(mode);
    rule.on_pre_arrival(&mut edge, &post, SimTime(1000));
    rule.on_post_spike(&post, std::slice::from_mut(&mut edge), SimTime(2000));
    let mut requested = 0.;
    let mut clipped = 0.;
    let mut rounding = 0.;
    for index in 1..=parts {
        let time = SimTime(2000 + 100000 * index / parts);
        let d = rule
            .delayed_update_diagnostics(&edge, &post, time, true)
            .ok_or_else(|| error("missing precision diagnostics"))?;
        requested += d.unrounded_delta;
        clipped += d.clipped_delta;
        rounding += d.rounding_error;
        rule.advance_time(&mut edge, &post, time, true);
    }
    let actual = f64::from(edge.weight().get()) - 0.5;
    let residual = edge.learning_weight_residual();
    Ok(
        json!({"mode":format!("{mode:?}"),"partitions":parts,"duration_us":100000,
        "requested":requested,"published":actual,"residual":residual,"clipped":clipped,"rounding_error":rounding,
        "balance_error":requested-actual-residual-clipped-rounding,"effective_mass_error":requested-actual-residual-clipped}),
    )
}
pub(super) fn run() -> Result<Value> {
    let mut cases = Vec::new();
    for mode in [WeightUpdateMode::DirectF32, WeightUpdateMode::Accumulated] {
        for parts in [1, 1000] {
            cases.push(run_case(mode, parts)?);
        }
    }
    let passed = cases[1]["published"] == 0.
        && cases[3]["published"].as_f64().unwrap() > 0.
        && [2, 3]
            .iter()
            .all(|&i| cases[i]["effective_mass_error"].as_f64().unwrap().abs() < 1e-15);
    if !passed {
        return Err(error("precision numeric positive controls failed"));
    }
    Ok(
        json!({"numeric_calibration":true,"controls_passed":passed,"cases":cases,
        "functional_memory_demonstrated":false,"limits":"One 100ms native isolated integration versus1000 partitions. Tag dynamics remain f32; only publication differs."}),
    )
}
