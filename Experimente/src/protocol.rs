//! Fixed-size serial frames; no allocations based on guest-controlled lengths.

use std::io::{self, Read, Write};

use crate::{Result, serial_model::VmModel};

pub const REQUEST_MAGIC: u8 = 0xa5;
pub const RESPONSE_MAGIC: u8 = 0x5a;
pub const PREDICT: u8 = 0;
pub const TRAIN: u8 = 1;
pub const FINISH: u8 = 0xff;
pub const NO_ACTION: u8 = 0xff;

/// Requests: [A5, operation, observation 0..3, teacher 0..3].
/// Replies: [5A, status (0=OK, 1=invalid), action (FF=none), operation].
/// FINISH requires zero payload and is acknowledged before the server returns.
/// EOF before FINISH is an error, including a truncated frame.
pub fn serve(stream: &mut (impl Read + Write), model: &mut VmModel) -> Result<usize> {
    let mut count = 0;
    loop {
        let mut request = [0; 4];
        stream.read_exact(&mut request)?;
        let [magic, operation, observation, teacher] = request;
        let valid = magic == REQUEST_MAGIC
            && match operation {
                PREDICT => observation < 4 && teacher == 0,
                TRAIN => observation < 4 && teacher < 4,
                FINISH => observation == 0 && teacher == 0,
                _ => false,
            };
        let action = if valid && operation != FINISH {
            count += 1;
            model
                .step(observation, (operation == TRAIN).then_some(teacher))?
                .unwrap_or(NO_ACTION)
        } else {
            NO_ACTION
        };
        stream.write_all(&[RESPONSE_MAGIC, u8::from(!valid), action, operation])?;
        stream.flush()?;
        if valid && operation == FINISH {
            return Ok(count);
        }
        if !valid {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "invalid VM request").into());
        }
    }
}
