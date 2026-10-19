//! QEMU display and keyboard adapters, independent of any neural topology.
//!
//! Perception is limited to captured pixels. No guest memory, serial task
//! messages, positions, labels, or rewards are exposed to the neural model.

#[cfg(not(unix))]
compile_error!("The QMP Unix socket adapter requires a Unix host.");

pub mod capture;
pub mod keyboard;
pub mod motor_decoder;
pub mod qmp;
pub mod signals;
pub mod test_world;
pub mod vision_encoder;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
