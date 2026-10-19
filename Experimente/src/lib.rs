//! Controlled research protocols and external readouts; depends on the running model.
pub mod agency;
pub mod experiment;
pub mod m3;
pub mod m7;
pub mod m8;
pub mod protocol;
mod random;
pub mod retention;
pub mod sensorimotor;
pub mod serial_model;

pub use nerva_vm::{Result, model};
