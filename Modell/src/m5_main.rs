use nerva_vm::{Result, m5};
use std::{
    env, fs,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
fn main() {
    if let Err(error) = run() {
        eprintln!("nerva-m5: {error}");
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
                    "Usage: nerva-m5 --config CONFIG.json --output RESULT.json [--raw-events]"
                );
                return Ok(());
            }
            "--config" if config.is_none() => {
                config = Some(PathBuf::from(args.next().ok_or("missing config path")?))
            }
            "--output" if output.is_none() => {
                output = Some(PathBuf::from(args.next().ok_or("missing output path")?))
            }
            "--raw-events" if !raw => raw = true,
            _ => return Err(format!("unexpected option {arg}").into()),
        }
    }
    let config = m5::Config::parse(serde_json::from_slice(&fs::read(
        config.ok_or("--config required")?,
    )?)?)?;
    let output = output.ok_or("--output required")?;
    if output.exists() {
        return Err("output exists; choose a new file".into());
    }
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    let flag = Arc::new(AtomicBool::new(false));
    let signal = flag.clone();
    ctrlc::set_handler(move || signal.store(true, Ordering::Relaxed))?;
    let result = m5::run(&config, &output, raw, &flag)?;
    use std::io::Write;
    let mut pending = output.as_os_str().to_owned();
    pending.push(".partial");
    let pending = PathBuf::from(pending);
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&pending)?;
    file.write_all(&serde_json::to_vec_pretty(&result)?)?;
    drop(file);
    fs::rename(pending, output)?;
    println!(
        "M5_COMPLETED {} seed {}",
        config.base.case_id, config.base.seed
    );
    Ok(())
}
