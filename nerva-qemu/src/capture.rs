//! QMP P6 screendumps and model-neutral RGB frames.

use std::{fs::File, io::Read, path::Path};

use serde_json::json;

use crate::{Result, qmp::Qmp};

const MAX_PIXELS: usize = 16_777_216;
const MAX_FILE: u64 = (MAX_PIXELS * 3 + 4096) as u64;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frame {
    width: usize,
    height: usize,
    rgb: Vec<u8>,
}

impl Frame {
    pub fn new(width: usize, height: usize, rgb: Vec<u8>) -> Result<Self> {
        let pixels = width.checked_mul(height).ok_or("image size overflow")?;
        if pixels == 0 || pixels > MAX_PIXELS || pixels * 3 != rgb.len() {
            return Err("invalid RGB frame dimensions or pixel count".into());
        }
        Ok(Self { width, height, rgb })
    }

    pub fn width(&self) -> usize {
        self.width
    }
    pub fn height(&self) -> usize {
        self.height
    }
    pub fn rgb(&self) -> &[u8] {
        &self.rgb
    }

    pub fn read_ppm(path: &Path) -> Result<Self> {
        let mut bytes = Vec::new();
        File::open(path)?
            .take(MAX_FILE + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() as u64 > MAX_FILE {
            return Err("screendump exceeds image size limit".into());
        }
        Self::from_ppm(&bytes)
    }

    pub fn from_ppm(bytes: &[u8]) -> Result<Self> {
        let mut cursor = 0;
        if token(bytes, &mut cursor)? != "P6" {
            return Err("expected binary P6 PPM image".into());
        }
        let width = token(bytes, &mut cursor)?.parse()?;
        let height = token(bytes, &mut cursor)?.parse()?;
        if token(bytes, &mut cursor)? != "255" {
            return Err("expected 8-bit PPM channels (maxval 255)".into());
        }
        // Consume exactly the header delimiter, never whitespace-valued pixels.
        match bytes.get(cursor) {
            Some(b'\r') if bytes.get(cursor + 1) == Some(&b'\n') => cursor += 2,
            Some(byte) if byte.is_ascii_whitespace() => cursor += 1,
            _ => return Err("missing PPM raster delimiter".into()),
        }
        Self::new(width, height, bytes[cursor..].to_vec())
    }
}

fn token<'a>(bytes: &'a [u8], cursor: &mut usize) -> Result<&'a str> {
    loop {
        while bytes.get(*cursor).is_some_and(u8::is_ascii_whitespace) {
            *cursor += 1;
        }
        if bytes.get(*cursor) != Some(&b'#') {
            break;
        }
        while bytes.get(*cursor).is_some_and(|byte| *byte != b'\n') {
            *cursor += 1;
        }
    }
    let start = *cursor;
    while bytes
        .get(*cursor)
        .is_some_and(|byte| !byte.is_ascii_whitespace() && *byte != b'#')
    {
        *cursor += 1;
    }
    if start == *cursor {
        return Err("incomplete PPM header".into());
    }
    Ok(std::str::from_utf8(&bytes[start..*cursor])?)
}

/// `path` must be an absolute path on the same host/filesystem as QEMU.
pub fn capture(qmp: &mut Qmp, path: &Path) -> Result<Frame> {
    if !path.is_absolute() {
        return Err("screendump path must be absolute".into());
    }
    let filename = path.to_str().ok_or("screendump path must be UTF-8")?;
    qmp.execute("screendump", json!({"filename": filename, "format": "ppm"}))?;
    Frame::read_ppm(path)
}
