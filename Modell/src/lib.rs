//! Running visual neural model, shared by the VM application and experiments.
mod direct;
pub mod model;
pub mod visual_vm;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub mod m4;
pub mod m5;
pub mod m6;
pub mod readout;
