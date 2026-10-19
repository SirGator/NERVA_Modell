use nerva_experiments::{
    Result,
    m3::{M3Config, run_m3},
};
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
        eprintln!("nerva-m3: {error}");
        std::process::exit(1);
    }
}
fn run() -> Result<()> {
    let mut config = M3Config::default();
    let mut output = None;
    let mut args = env::args().skip(1);
    while let Some(option) = args.next() {
        if matches!(option.as_str(), "help" | "--help" | "-h") {
            println!(
                "Usage: nerva-m3 [OPTIONS]\n --seeds 0,1,2,3,4,5,6,7,8,9 --jobs N (4)\n --train-trials N (128), --probe-trials N (64), --calibration-trials N (32)\n --phase-us N (8000), --retention-windows N (40), --scan-repeats N (8)\n --processing N (32), --output NEW_DIRECTORY\nTrials are balanced in blocks of four; no goal or reward. Primary readout is calibrated on images only."
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
            "--jobs" => config.jobs = value.parse()?,
            "--train-trials" => config.train_trials = value.parse()?,
            "--probe-trials" => config.probe_trials = value.parse()?,
            "--calibration-trials" => config.calibration_trials = value.parse()?,
            "--phase-us" => config.phase_us = value.parse()?,
            "--retention-windows" => config.retention_windows = value.parse()?,
            "--scan-repeats" => config.scan_repeats = value.parse()?,
            "--processing" => config.model.processing_neurons = value.parse()?,
            "--output" if output.is_none() => output = Some(PathBuf::from(value)),
            _ => return Err(format!("unexpected option: {option}").into()),
        }
    }
    config.validate()?;
    let output = output.unwrap_or_else(|| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
            "../../Messungen/Experimente/{}-m3",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    });
    fs::create_dir_all(output.parent().unwrap_or_else(|| std::path::Path::new(".")))?;
    fs::create_dir(&output)?;
    let flag = Arc::new(AtomicBool::new(false));
    let signal = Arc::clone(&flag);
    ctrlc::set_handler(move || signal.store(true, Ordering::Relaxed))?;
    let report = run_m3(&config, &output, &flag)?;
    println!(
        "M3_COMPLETED: criteria met: {}. Output: {}",
        report["m3_criteria_met"],
        output.display()
    );
    Ok(())
}
