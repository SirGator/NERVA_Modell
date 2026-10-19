use nerva_experiments::{
    Result,
    sensorimotor::{SensorimotorConfig, run_sensorimotor},
};
use nerva_qemu::vision_encoder::Encoding;
use std::{
    env, fs,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

fn main() {
    if let Err(error) = run() {
        eprintln!("nerva-sensorimotor: {error}");
        std::process::exit(1);
    }
}
fn run() -> Result<()> {
    let mut config = SensorimotorConfig::default();
    let mut output = None;
    let mut args = env::args().skip(1);
    while let Some(option) = args.next() {
        if matches!(option.as_str(), "help" | "--help" | "-h") {
            println!(
                "Usage: nerva-sensorimotor [OPTIONS]\n  --seeds 0,1,2,3,4,5,6,7,8,9\n  --explore-steps N (64), --probe-steps N (64), multiples of eight\n  --processing N (32), --jobs N (4), --output NEW_DIRECTORY\n  --encoding brightness|change, --initial-drive F (2.5)\n  --pruning-min-us N (300000), --inhibition-weight F (2.0)\n\nSeparate motor and goal-free action/effect tests. Random tie breaking is a control only."
            );
            return Ok(());
        }
        let value = args.next().ok_or("missing option value")?;
        match option.as_str() {
            "--seeds" => {
                config.seeds = value
                    .split(',')
                    .map(str::parse)
                    .collect::<std::result::Result<_, _>>()?
            }
            "--seed" => config.seeds = vec![value.parse()?],
            "--explore-steps" => config.explore_steps = value.parse()?,
            "--probe-steps" => config.probe_steps = value.parse()?,
            "--processing" => config.model.processing_neurons = value.parse()?,
            "--jobs" => config.jobs = value.parse()?,
            "--initial-drive" => config.model.initial_structural_drive = value.parse()?,
            "--pruning-min-us" => config.model.pruning_min_below_us = value.parse()?,
            "--inhibition-weight" => config.model.motor_inhibition_weight = value.parse()?,
            "--encoding" => {
                config.model.encoding = match value.as_str() {
                    "brightness" => Encoding::Brightness,
                    "change" => Encoding::Change,
                    _ => return Err("encoding must be brightness or change".into()),
                }
            }
            "--output" if output.is_none() => output = Some(PathBuf::from(value)),
            _ => return Err(format!("unexpected option: {option}").into()),
        }
    }
    config.validate()?;
    let output = output.unwrap_or_else(|| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
            "../../Messungen/Experimente/{}-sensorimotor",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    });
    fs::create_dir_all(output.parent().unwrap_or_else(|| std::path::Path::new(".")))?;
    fs::create_dir(&output)?;
    let cancelled = Arc::new(AtomicBool::new(false));
    let signal = Arc::clone(&cancelled);
    ctrlc::set_handler(move || signal.store(true, Ordering::Relaxed))?;
    let report = run_sensorimotor(&config, &output, &cancelled)?;
    println!(
        "SENSORIMOTOR_COMPLETED: isolated motor mapping: {}; controlled effect learning evidence: {}. Output: {}",
        report["isolated_motor_mapping_all_passed"],
        report["controlled_effect_learning_evidence"],
        output.display()
    );
    Ok(())
}
