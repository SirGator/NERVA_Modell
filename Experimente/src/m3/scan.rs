use super::{M3Config, copy, readout, stop};
use crate::{Result, model::Model, random::shuffle};
use nerva_qemu::{
    keyboard::Key,
    test_world::{MotionWorld, PixelEnvironment},
};
use serde_json::{Value, json};
use std::{fs, io::Write, path::Path, sync::atomic::AtomicBool};

pub fn run(config: &M3Config, output: &Path, flag: &AtomicBool) -> Result<Vec<Value>> {
    let mut csv = fs::File::create_new(output.join("motor-scan.csv"))?;
    writeln!(
        csv,
        "seed,setting,repeat,position,a_spikes,d_spikes,first_a_us,first_d_us,selected,positive_arrivals,negative_arrivals,refractory_arrivals"
    )?;
    let settings = [
        ("baseline", 1.0, 10_000, false, 2.0),
        ("weight_025", 0.25, 10_000, false, 2.0),
        ("weight_050", 0.5, 10_000, false, 2.0),
        ("refractory_5ms", 1.0, 5_000, false, 2.0),
        ("refractory_20ms", 1.0, 20_000, false, 2.0),
        ("inhibition_05", 1.0, 10_000, true, 0.5),
        ("inhibition_2", 1.0, 10_000, true, 2.0),
        ("inhibition_4", 1.0, 10_000, true, 4.0),
    ];
    let mut results = Vec::new();
    for &seed in &config.seeds {
        for (name, scale, refractory, competition, inhibition) in settings {
            stop(flag)?;
            let mut base = config.model.clone().frozen();
            base.seed = seed;
            base.motor_weight_scale *= scale;
            base.motor_refractory_us = refractory;
            base.motor_competition = competition;
            base.motor_inhibition_weight = inhibition;
            let mut model = Model::from_config(base)?;
            let checkpoint = model.network().clone();
            let mut random = seed ^ 0x1af5bc79;
            let mut counts = Vec::new();
            let mut timing = Vec::new();
            let mut labels = Vec::new();
            let mut total = 0usize;
            let mut selected = 0;
            let mut arrivals = 0;
            let mut during = 0;
            for repeat in 0..config.scan_repeats {
                let mut positions: Vec<u8> = (0..7).collect();
                shuffle(&mut positions, &mut random);
                for position in positions {
                    stop(flag)?;
                    copy(&mut model, None)?;
                    let step = model.step(&MotionWorld::new(position, false)?.observe()?)?;
                    super::frozen_step(&step)?;
                    let a = step.motor_counts[Key::A as usize];
                    let d = step.motor_counts[Key::D as usize];
                    counts.push(vec![a as f64, d as f64]);
                    let mut bins = vec![0.0; 20];
                    for pulse in &step.motor_events {
                        let channel = if pulse.channel == Key::A.channel() {
                            0
                        } else {
                            1
                        };
                        let bin = ((pulse.at_us - step.start_us) / 5_000).min(9) as usize;
                        bins[channel * 10 + bin] += 1.0;
                    }
                    timing.push(bins);
                    labels.push(i16::from(position));
                    total += a + d;
                    selected += usize::from(step.action.is_some());
                    let first = |key: Key| {
                        step.motor_events
                            .iter()
                            .find(|pulse| pulse.channel == key.channel())
                            .map(|p| (p.at_us - step.start_us).to_string())
                            .unwrap_or_default()
                    };
                    let inputs = [
                        &step.motor_inputs[Key::A as usize],
                        &step.motor_inputs[Key::D as usize],
                    ];
                    let positive = inputs.iter().map(|s| s.positive_arrivals).sum::<usize>();
                    let negative = inputs.iter().map(|s| s.negative_arrivals).sum::<usize>();
                    let in_ref = inputs
                        .iter()
                        .map(|s| s.arrivals_during_refractory)
                        .sum::<usize>();
                    arrivals += positive + negative;
                    during += in_ref;
                    writeln!(
                        csv,
                        "{seed},{name},{repeat},{position},{a},{d},{},{},{},{positive},{negative},{in_ref}",
                        first(Key::A),
                        first(Key::D),
                        step.action.is_some()
                    )?;
                }
            }
            if !crate::retention::learned_parameters_equal(&checkpoint, model.network()) {
                return Err("motor scan changed frozen parameters".into());
            }
            results.push(json!({"seed":seed,"setting":name,"motor_weight_scale":config.model.motor_weight_scale*scale,"motor_refractory_us":refractory,
            "competition":competition,"inhibition_weight":inhibition,"observations":labels.len(),"mean_motor_spikes":total as f64/labels.len() as f64,
            "selection_rate":selected as f64/labels.len() as f64,"arrival_refractory_fraction":during as f64/arrivals.max(1) as f64,
            "input_position_from_motor_counts":readout::chronological(&counts,&labels),"input_position_from_motor_timing":readout::chronological(&timing,&labels),"parameters_retained":true}));
        }
    }
    Ok(results)
}
