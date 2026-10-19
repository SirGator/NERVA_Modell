//! Bounded, synchronous QMP exchanges with IDs and asynchronous event handling.

use std::{
    io::{BufRead, BufReader, Write},
    os::unix::net::UnixStream,
    path::Path,
    time::{Duration, Instant},
};

use serde_json::{Value, json};

use crate::Result;

const MAX_MESSAGE: usize = 1_048_576;
const TIMEOUT: Duration = Duration::from_secs(5);

pub struct Qmp {
    stream: BufReader<UnixStream>,
    next_id: u64,
    healthy: bool,
}

impl Qmp {
    pub fn connect(path: &Path) -> Result<Self> {
        Self::from_stream(UnixStream::connect(path)?)
    }

    pub fn from_stream(stream: UnixStream) -> Result<Self> {
        stream.set_write_timeout(Some(TIMEOUT))?;
        let mut client = Self {
            stream: BufReader::new(stream),
            next_id: 0,
            healthy: true,
        };
        let greeting = client.read_message(Instant::now() + TIMEOUT)?;
        if !greeting.get("QMP").is_some_and(Value::is_object) {
            return Err("missing QMP greeting".into());
        }
        client.execute("qmp_capabilities", json!({}))?;
        Ok(client)
    }

    pub fn execute(&mut self, command: &str, arguments: Value) -> Result<Value> {
        if !self.healthy {
            return Err("QMP connection unusable after a previous exchange failure".into());
        }
        self.healthy = false;
        self.exchange(command, arguments)
    }

    fn exchange(&mut self, command: &str, arguments: Value) -> Result<Value> {
        let deadline = Instant::now() + TIMEOUT;
        let id = self.next_id;
        self.next_id = id.checked_add(1).ok_or("QMP request ID overflow")?;
        let request = json!({"execute": command, "arguments": arguments, "id": id});
        serde_json::to_writer(self.stream.get_mut(), &request)?;
        self.stream.get_mut().write_all(b"\r\n")?;
        self.stream.get_mut().flush()?;
        loop {
            let response = self.read_message(deadline)?;
            if response.get("event").is_some() {
                continue;
            }
            if response.get("id").and_then(Value::as_u64) != Some(id) {
                return Err("QMP response ID mismatch".into());
            }
            if let Some(error) = response.get("error") {
                self.healthy = true; // A command rejection preserves stream alignment.
                return Err(format!("QMP {command}: {error}").into());
            }
            let result = response
                .get("return")
                .cloned()
                .ok_or_else(|| "QMP response has no return value".into());
            self.healthy = result.is_ok();
            return result;
        }
    }

    fn read_message(&mut self, deadline: Instant) -> Result<Value> {
        let mut line = Vec::new();
        loop {
            let remaining = deadline
                .checked_duration_since(Instant::now())
                .filter(|duration| !duration.is_zero())
                .ok_or("QMP exchange timed out")?;
            self.stream.get_mut().set_read_timeout(Some(remaining))?;
            let buffer = self.stream.fill_buf()?;
            if buffer.is_empty() {
                return Err("QMP connection closed before a complete response".into());
            }
            let newline = buffer.iter().position(|byte| *byte == b'\n');
            let count = newline.map_or(buffer.len(), |index| index + 1);
            if line.len() + count > MAX_MESSAGE {
                return Err("QMP message exceeds 1 MiB".into());
            }
            line.extend_from_slice(&buffer[..count]);
            self.stream.consume(count);
            if newline.is_some() {
                return Ok(serde_json::from_slice(&line)?);
            }
        }
    }
}
