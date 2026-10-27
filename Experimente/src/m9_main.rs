use nerva_experiments::{Result, m9};
use std::{
    env, fs,
    io::Write,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
fn main() {
    if let Err(e) = run() {
        eprintln!("nerva-m9: {e}");
        std::process::exit(1);
    }
}
fn run() -> Result<()> {
    let mut args = env::args().skip(1);
    let mut config = None;
    let mut output = None;
    let mut raw = false;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" => {
                println!(
                    "Usage: nerva-m9 --config CONFIG.json --output NEW_RESULT.json [--raw-events]"
                );
                return Ok(());
            }
            "--config" if config.is_none() => {
                config = Some(PathBuf::from(args.next().ok_or("missing config")?))
            }
            "--output" if output.is_none() => {
                output = Some(PathBuf::from(args.next().ok_or("missing output")?))
            }
            "--raw-events" if !raw => raw = true,
            _ => return Err(format!("unexpected option {arg}").into()),
        }
    }
    let config = m9::parse(serde_json::from_slice(&fs::read(
        config.ok_or("--config required")?,
    )?)?)?;
    let output = output.ok_or("--output required")?;
    if output.exists() {
        return Err("output exists".into());
    }
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    let flag = Arc::new(AtomicBool::new(false));
    let signal = flag.clone();
    ctrlc::set_handler(move || signal.store(true, Ordering::Relaxed))?;
    let result = m9::run(&config, &output, raw, &flag)?;
    let partial = PathBuf::from(format!("{}.partial", output.display()));
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&partial)?;
    file.write_all(&serde_json::to_vec_pretty(&result)?)?;
    drop(file);
    fs::rename(partial, &output)?;
    println!(
        "M9_COMPLETED {} seed {}",
        config.experiment.engine.base.case_id, config.experiment.engine.base.seed
    );
    Ok(())
}
