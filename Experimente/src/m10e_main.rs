use nerva_experiments::{Result, m10e};
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
        eprintln!("nerva-m10e: {e}");
        std::process::exit(1);
    }
}
fn run() -> Result<()> {
    let mut args = env::args().skip(1);
    let mut config = None;
    let mut output = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" => {
                println!("Usage: nerva-m10e --reference M9_RESULT.json --output NEW_RESULT.json");
                return Ok(());
            }
            "--reference" if config.is_none() => {
                config = Some(PathBuf::from(args.next().ok_or("missing config")?))
            }
            "--output" if output.is_none() => {
                output = Some(PathBuf::from(args.next().ok_or("missing output")?))
            }
            _ => return Err(format!("unexpected option {arg}").into()),
        }
    }
    let reference = serde_json::from_slice(&fs::read(config.ok_or("--reference required")?)?)?;
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
    let result = m10e::run(&reference, &output, &flag)?;
    let partial = PathBuf::from(format!("{}.partial", output.display()));
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&partial)?;
    file.write_all(&serde_json::to_vec_pretty(&result)?)?;
    drop(file);
    fs::rename(partial, &output)?;
    println!(
        "M10E_COMPLETED {} seed {}",
        reference["case_id"], reference["seed"]
    );
    Ok(())
}
