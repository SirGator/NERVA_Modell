//! Area-averaged grayscale perception converted to model-independent rates.

use crate::{Result, capture::Frame, signals::ChannelRate};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Encoding {
    Brightness,
    Change,
}

#[derive(Debug)]
pub struct VisionEncoder {
    width: usize,
    height: usize,
    max_rate_hz: f32,
    mode: Encoding,
    previous: Option<Vec<f32>>,
}

impl VisionEncoder {
    pub fn new(width: usize, height: usize, max_rate_hz: f32, mode: Encoding) -> Result<Self> {
        if width == 0
            || height == 0
            || width.checked_mul(height).is_none_or(|count| count > 65_536)
            || !max_rate_hz.is_finite()
            || !(0.0..=1_000_000.0).contains(&max_rate_hz)
        {
            return Err("invalid vision dimensions or maximum rate".into());
        }
        Ok(Self {
            width,
            height,
            max_rate_hz,
            mode,
            previous: None,
        })
    }

    pub fn channel_count(&self) -> usize {
        self.width * self.height * if self.mode == Encoding::Change { 2 } else { 1 }
    }

    /// Convert a frame to rates. The host supplies simulation time and spike
    /// generation. Change mode has separate ON/OFF channels and a silent first
    /// frame. Source sizes may change between frames.
    pub fn encode(&mut self, frame: &Frame) -> Result<Vec<ChannelRate>> {
        let gray = grayscale(frame, self.width, self.height)?;
        let mut rates = Vec::with_capacity(self.channel_count());
        for (index, &brightness) in gray.iter().enumerate() {
            let (on, off) = match self.mode {
                Encoding::Brightness => (brightness, None),
                Encoding::Change => {
                    let change = self
                        .previous
                        .as_ref()
                        .map_or(0.0, |previous| brightness - previous[index]);
                    (change.max(0.0), Some((-change).max(0.0)))
                }
            };
            rates.push(ChannelRate {
                channel: index as u64,
                rate_hz: on * self.max_rate_hz,
            });
            if let Some(off) = off {
                rates.push(ChannelRate {
                    channel: (index + gray.len()) as u64,
                    rate_hz: off * self.max_rate_hz,
                });
            }
        }
        self.previous = Some(gray);
        Ok(rates)
    }
}

/// Average each source rectangle to preserve small visible features when shrinking.
pub fn grayscale(frame: &Frame, width: usize, height: usize) -> Result<Vec<f32>> {
    if width == 0 || height == 0 || width > frame.width() || height > frame.height() {
        return Err("grayscale target must fit inside the source image".into());
    }
    let mut output = Vec::with_capacity(width * height);
    for y in 0..height {
        for x in 0..width {
            let x0 = x * frame.width() / width;
            let x1 = (x + 1) * frame.width() / width;
            let y0 = y * frame.height() / height;
            let y1 = (y + 1) * frame.height() / height;
            let mut sum = 0.0;
            for sy in y0..y1 {
                for sx in x0..x1 {
                    let offset = (sy * frame.width() + sx) * 3;
                    let rgb = &frame.rgb()[offset..offset + 3];
                    sum += (0.2126 * f32::from(rgb[0])
                        + 0.7152 * f32::from(rgb[1])
                        + 0.0722 * f32::from(rgb[2]))
                        / 255.0;
                }
            }
            output.push((sum / ((x1 - x0) * (y1 - y0)) as f32).clamp(0.0, 1.0));
        }
    }
    Ok(output)
}
