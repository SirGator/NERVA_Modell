use nerva_vm::{Result, m6};
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
        eprintln!("nerva-m6: {error}");
        std::process::exit(1);
    }
}
fn run() -> Result<()> {
    let mut args = env::args().skip(1);
    let mut config = None;
    let mut output = None;
    let mut raw = false;
    let mut intrinsic = false;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" => {
                println!(
                    "Usage: nerva-m6 --config CONFIG.json --output RESULT.json [--raw-events]\n       nerva-m6 --intrinsic-audit --config CONFIG.json --output RESULT.json"
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
            "--intrinsic-audit" if !intrinsic => intrinsic = true,
            _ => return Err(format!("unexpected option {arg}").into()),
        }
    }
    let config = m6::Config::parse(serde_json::from_slice(&fs::read(
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
    let result = if intrinsic {
        m6::mechanisms::run(&config, &flag)?
    } else {
        m6::run(&config, &output, raw, &flag)?
    };
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
        "M6_COMPLETED {} seed {}",
        config.base.case_id, config.base.seed
    );
    Ok(())
}
