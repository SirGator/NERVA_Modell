//! Host construction choices using NERVA's public tissue mechanisms.
//! Cell physics, field interpolation and synaptic mechanisms live in the library.
use super::{ModelConfig, random::next_random};
use crate::Result;
pub use nerva::cell_profiles::{AreaField, AreaWeights, connection_affinity, electrical_profile};
use nerva::{
    core::{Network, NeuronId, Synapse},
    neuromodulation::{ReceptorProfile, ReleaseProfile},
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Architecture {
    #[default]
    Cortical,
    Hippocampal,
}
impl Architecture {
    pub fn name(self) -> &'static str {
        match self {
            Self::Cortical => "cortical",
            Self::Hippocampal => "hippocampal",
        }
    }
}
pub(crate) fn field_for_config(config: &ModelConfig) -> AreaField {
    let side = (config.processing_neurons as f64).cbrt().ceil() as u64;
    let extent = side.saturating_sub(1) as f32 * 0.06 + 0.01;
    AreaField::new(-extent, extent, config.area_transition_width).expect("validated model geometry")
}
/// Weighted sampling without replacement, from existing geometric candidates.
pub(crate) fn order_candidates(
    network: &Network,
    source: NeuronId,
    candidates: &mut [NeuronId],
    field: AreaField,
    rng: &mut u64,
) {
    let neuron = network.neuron(source).expect("known source");
    let pre = field.at(neuron.position()).expect("validated position");
    let mut keys = std::collections::BTreeMap::new();
    for &target in candidates.iter() {
        let post = network.neuron(target).expect("known target");
        let distance = neuron.position().distance_to(post.position());
        let preference = connection_affinity(
            pre,
            field.at(post.position()).expect("validated position"),
            neuron.polarity(),
        )
        .expect("convex influences")
            * (-distance / 0.25).exp();
        let draw = ((next_random(rng) >> 11) as f64 + 0.5) / ((1u64 << 53) as f64);
        keys.insert(target, -draw.ln() / f64::from(preference));
    }
    candidates.sort_by(|a, b| keys[a].total_cmp(&keys[b]).then(a.cmp(b)));
}
pub(crate) fn chemical_profiles(
    area: AreaWeights,
    index: u64,
    seed: u64,
) -> (ReceptorProfile, ReleaseProfile) {
    let mut state = seed ^ index.wrapping_mul(0xd6e8_feb8_6659_fd93);
    let variation = 0.75 + (next_random(&mut state) % 10_001) as f32 * 0.00005;
    let mut receptors = nerva::cell_profiles::receptor_profile(area).expect("convex influences");
    receptors.dopamine_d1_like *= variation;
    receptors.dopamine_d2_like *= variation;
    receptors.acetylcholine *= variation;
    receptors.noradrenaline *= variation;
    // Existing cells remain part of the shared plastic recurrent network.
    let source = (index.wrapping_mul(29).wrapping_add(seed)) % 32;
    let release = ReleaseProfile {
        dopamine: if source == 0 { 0.001 } else { 0. },
        acetylcholine: if source == 11 { 0.001 } else { 0. },
        noradrenaline: if source == 22 { 0.001 } else { 0. },
    };
    (receptors, release)
}
pub(crate) fn configure_synapse(
    edge: &mut Synapse,
    network: &Network,
    field: AreaField,
) -> Result<()> {
    let pre = network.neuron(edge.pre()).expect("known pre");
    let post = network.neuron(edge.post()).expect("known post");
    edge.set_short_term(nerva::cell_profiles::short_term_profile(
        field.at(pre.position())?,
        field.at(post.position())?,
        pre.polarity(),
    )?)?;
    Ok(())
}
