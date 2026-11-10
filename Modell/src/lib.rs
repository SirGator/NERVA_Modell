//! Running visual neural model, shared by the VM application and research harness.
mod direct;
pub mod model;
pub mod visual_vm;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
