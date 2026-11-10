//! Research orchestration; no neuron, environment or learning rule lives here.
pub mod collector;
mod readout;
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

/// The same FNV digest of Debug output without allocating the entire string.
/// Large network snapshots and audit streams can be hashed in bounded memory.
pub fn fingerprint_debug(value: &impl std::fmt::Debug) -> String {
    struct Digest(u64);
    impl std::fmt::Write for Digest {
        fn write_str(&mut self, s: &str) -> std::fmt::Result {
            for byte in s.bytes() {
                self.0 = (self.0 ^ u64::from(byte)).wrapping_mul(0x100000001b3);
            }
            Ok(())
        }
    }
    use std::fmt::Write;
    let mut digest = Digest(0xcbf29ce484222325);
    write!(&mut digest, "{value:?}").expect("digest writer is infallible");
    format!("{:016x}", digest.0)
}
