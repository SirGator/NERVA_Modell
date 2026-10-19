use super::{config::M4Config, world};
use crate::{
    Result,
    model::{Model, SensorMode, sensory::SensorEncoder},
};
use nerva::math::try_distance_attenuation;
use serde_json::{Value, json};

pub const TARGET_CURRENT: f64 = 10.0;
/// A fixed gain chosen using initial weights and purely visual anchor images.
/// No trial labels, learning outcome, action or evaluation score selects it.
pub fn gain(model: &Model, config: &M4Config) -> Result<(f32, Value)> {
    let channels = config.sensor.channels();
    let n = model.processing_ids().len();
    let mut transfers = vec![vec![0.0; n]; channels];
    for edge in model.network().synapses() {
        if edge.pre().0 < channels as u64
            && let Ok(target) = model.processing_ids().binary_search(&edge.post())
        {
            let from = model.network().neuron(edge.pre()).unwrap().position();
            let to = model.network().neuron(edge.post()).unwrap().position();
            transfers[edge.pre().0 as usize][target] += f64::from(
                edge.weight().get() * try_distance_attenuation(from.distance_to(to), 3.0)?,
            );
        }
    }
    let mut currents = vec![0.0; n];
    for position in [2, 4] {
        let mut encoder = SensorEncoder::new(config.sensor)?;
        encoder.encode(&world::image(3)?)?;
        let mut phases = vec![(3, 50_000), (3, config.delay_us), (position, 50_000)];
        let mut padding = super::TRIAL_US - 100_000 - config.delay_us;
        while padding > 0 {
            let duration = padding.min(50_000);
            phases.push((3, duration));
            padding -= duration;
        }
        for (observed, duration) in phases {
            for rate in encoder.encode(&world::image(observed)?)? {
                for (total, &transfer) in currents.iter_mut().zip(&transfers[rate.channel as usize])
                {
                    *total += f64::from(rate.rate_hz) * transfer * duration as f64
                        / super::TRIAL_US as f64
                        / 2.0;
                }
            }
        }
    }
    let mean = currents.iter().sum::<f64>() / n as f64;
    if !mean.is_finite() || mean <= 0.0 {
        return Err("normalization has zero visual input current".into());
    }
    let gain = (TARGET_CURRENT / mean) as f32;
    for current in &mut currents {
        *current *= f64::from(gain);
    }
    Ok((
        gain,
        json!({"mode":"equal_input_current","definition":"initial expected sensory-to-Core absolute current, averaged across Core cells and the entire fixed trial, including neutral baseline/cue/padding and two equally weighted visual outcomes", "target_mean_current_per_core_second":TARGET_CURRENT,
        "fixed_rate_gain":gain,"expected_current_per_core_second":currents,"zero_input_core_cells":currents.iter().filter(|&&v|v==0.0).count(),
        "normalizes_population_mean_only":true,"sensor_mode_is_diagnostic":config.sensor.mode!=SensorMode::Pixels,
        "limits":"Per-cell distribution remains sparse. Poisson refractory losses and learned weight drift are measured, not corrected using test outcomes. ON/OFF has no static current."}),
    ))
}
