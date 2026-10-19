use nerva_experiments::{Result, m7};
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
        eprintln!("nerva-m7: {error}");
        std::process::exit(1);
    }
}
fn run() -> Result<()> {
    let mut args = env::args().skip(1);
    let mut reference = None;
    let mut output = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" => {
                println!(
                    "Usage: nerva-m7 --reference M6_CORE_OFF_RESULT.json --output NEW_DIRECTORY"
                );
                return Ok(());
            }
            "--reference" if reference.is_none() => {
                reference = Some(PathBuf::from(args.next().ok_or("missing reference")?))
            }
            "--output" if output.is_none() => {
                output = Some(PathBuf::from(args.next().ok_or("missing output")?))
            }
            _ => return Err(format!("unexpected option {arg}").into()),
        }
    }
    let reference: serde_json::Value =
        serde_json::from_slice(&fs::read(reference.ok_or("--reference required")?)?)?;
    m7::validate_reference(&reference)?;
    let output = output.ok_or("--output required")?;
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::create_dir(&output)?;
    let flag = Arc::new(AtomicBool::new(false));
    let signal = flag.clone();
    ctrlc::set_handler(move || signal.store(true, Ordering::Relaxed))?;
    m7::run(&reference, &output, &flag)?;
    println!("M7_COMPLETED seed {}", reference["seed"]);
    Ok(())
}
