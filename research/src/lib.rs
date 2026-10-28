//! Research orchestration; no neuron, environment or learning rule lives here.
pub mod collector;
pub mod runner;
pub mod scenario;
pub mod scenarios;
pub mod spec;
pub use nerva::experiment::ExperimentError;
pub type Result<T> = std::result::Result<T, ExperimentError>;

pub fn error(message: impl Into<String>) -> ExperimentError {
    ExperimentError::new("research", message)
}

/// Explicitly non-cryptographic, deterministic fingerprint for accidental corruption.
/// Plans additionally compare their complete structured contents when resumed.
pub fn fingerprint(bytes: &[u8]) -> String {
    let hash = bytes.iter().fold(0xcbf29ce484222325_u64, |h, &b| {
        (h ^ b as u64).wrapping_mul(0x100000001b3)
    });
    format!("{hash:016x}")
}
