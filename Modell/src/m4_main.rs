use nerva_vm::{
    Result,
    m4::{config::M4Config, run},
};
use std::{
    env, fs,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
fn main() {
    if let Err(error) = main_run() {
        eprintln!("nerva-m4: {error}");
        std::process::exit(1);
    }
}
fn main_run() -> Result<()> {
    let mut args = env::args().skip(1);
    let mut config = None;
    let mut output = None;
    let mut raw = false;
    let mut mechanisms = false;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" => {
                println!(
                    "Usage: nerva-m4 --config CONFIG.json --output RESULT.json [--raw-events]\n       nerva-m4 --mechanistic-audit --output RESULT.json\nRuns the NERVA-M4-v1 contract. Sidecars: .details.csv, .audit.csv, optional .events.csv."
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
            "--mechanistic-audit" if !mechanisms => mechanisms = true,
            _ => return Err(format!("unexpected option {arg}").into()),
        }
    }
    let output = output.ok_or("--output is required")?;
    if mechanisms {
        if config.is_some() || raw {
            return Err("mechanistic audit does not accept --config or --raw-events".into());
        }
        if output.exists() {
            return Err("output already exists".into());
        }
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent)?;
        }
        let result = nerva_vm::m4::mechanisms::run(&output)?;
        publish(&output, &result)?;
        println!("M4_MECHANISMS_COMPLETED");
        return Ok(());
    }
    let config = M4Config::parse(serde_json::from_slice(&fs::read(
        config.ok_or("--config is required")?,
    )?)?)?;
    if output.exists() {
        return Err("output already exists; choose a new file".into());
    }
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    let flag = Arc::new(AtomicBool::new(false));
    let signal = Arc::clone(&flag);
    ctrlc::set_handler(move || signal.store(true, Ordering::Relaxed))?;
    let result = run(&config, &output, raw, &flag)?;
    publish(&output, &result)?;
    println!("M4_COMPLETED {} seed {}", config.case_id, config.seed);
    Ok(())
}

fn publish(path: &std::path::Path, result: &serde_json::Value) -> Result<()> {
    use std::io::Write;
    let mut pending = path.as_os_str().to_owned();
    pending.push(".partial");
    let pending = PathBuf::from(pending);
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&pending)?;
    file.write_all(&serde_json::to_vec_pretty(result)?)?;
    drop(file);
    fs::rename(pending, path)?;
    Ok(())
}
