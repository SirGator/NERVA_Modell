//! Frame-only sensory front end. Diagnostic features never access a world object.
use crate::Result;
use nerva::{
    io::ReceptorSignal,
    math::Position3D,
    primitives::SimTime,
    transduction::{
        ChannelSpike, PoissonSensoryTransducer, RateSensoryTransducer, SensoryTransducer,
    },
};
use nerva_qemu::{
    capture::Frame,
    signals::ChannelRate,
    vision_encoder::{Encoding, VisionEncoder, grayscale},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SensorMode {
    Pixels,
    Effect2Ch,
    Position7Ch,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpikeCoding {
    Deterministic,
    Poisson,
    OnOff,
}

impl std::str::FromStr for SensorMode {
    type Err = String;
    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        match value {
            "pixels" => Ok(Self::Pixels),
            "effect_2ch" => Ok(Self::Effect2Ch),
            "position_7ch" => Ok(Self::Position7Ch),
            _ => Err(format!(
                "unknown sensor {value:?}; expected pixels, effect_2ch or position_7ch"
            )),
        }
    }
}

impl std::str::FromStr for SpikeCoding {
    type Err = String;
    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        match value {
            "deterministic" => Ok(Self::Deterministic),
            "poisson" => Ok(Self::Poisson),
            "on_off" => Ok(Self::OnOff),
            _ => Err(format!(
                "unknown encoding {value:?}; expected deterministic, poisson or on_off"
            )),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SensorConfig {
    pub mode: SensorMode,
    pub resolution: usize,
    pub coding: SpikeCoding,
    pub max_rate_hz: f32,
}
impl Default for SensorConfig {
    fn default() -> Self {
        Self {
            mode: SensorMode::Pixels,
            resolution: 64,
            coding: SpikeCoding::Poisson,
            max_rate_hz: 120.0,
        }
    }
}
impl SensorConfig {
    pub fn validate(&self) -> Result<()> {
        if ![8, 16, 64].contains(&self.resolution) {
            return Err("sensor resolution must be 8, 16 or 64".into());
        }
        if !self.max_rate_hz.is_finite() || !(0.0..=1_000.0).contains(&self.max_rate_hz) {
            return Err("sensor maximum rate must be finite in 0..=1000 Hz".into());
        }
        Ok(())
    }
    pub fn base_channels(&self) -> usize {
        match self.mode {
            SensorMode::Pixels => self.resolution * self.resolution,
            SensorMode::Effect2Ch => 2,
            SensorMode::Position7Ch => 7,
        }
    }
    pub fn channels(&self) -> usize {
        self.base_channels()
            * if self.coding == SpikeCoding::OnOff {
                2
            } else {
                1
            }
    }
    pub(crate) fn position(&self, id: u64) -> Position3D {
        let index = id as usize % self.base_channels();
        match self.mode {
            SensorMode::Pixels => Position3D::new(
                (index % self.resolution) as f32 / (self.resolution - 1) as f32 - 0.5,
                (index / self.resolution) as f32 / (self.resolution - 1) as f32 - 0.5,
                -1.5,
            ),
            _ => Position3D::new(
                index as f32 / (self.base_channels() - 1) as f32 - 0.5,
                0.0,
                -1.5,
            ),
        }
    }
}

#[derive(Debug)]
pub struct SensorEncoder {
    config: SensorConfig,
    pixels: VisionEncoder,
    previous: Option<Vec<f32>>,
}
impl SensorEncoder {
    pub fn new(config: SensorConfig) -> Result<Self> {
        config.validate()?;
        Ok(Self {
            pixels: VisionEncoder::new(
                config.resolution,
                config.resolution,
                config.max_rate_hz,
                if config.coding == SpikeCoding::OnOff {
                    Encoding::Change
                } else {
                    Encoding::Brightness
                },
            )?,
            config,
            previous: None,
        })
    }
    pub fn config(&self) -> SensorConfig {
        self.config
    }
    /// Diagnostic features describe the presently visible frame, including the
    /// unchanged start frame. No executed key or future label is accepted here.
    pub fn features(&self, frame: &Frame) -> Result<Vec<f32>> {
        let n = self.config.resolution;
        let gray = grayscale(frame, n, n)?;
        if self.config.mode == SensorMode::Pixels {
            return Ok(gray);
        }
        // In the diagnostic world's fixed camera, the solid agent is above the
        // track. Area averages retain its brightness centroid even at 8x8.
        let mut mass = 0.0f64;
        let mut moment = 0.0f64;
        for y in 0..n * 5 / 8 {
            for x in 0..n {
                let value = f64::from(gray[y * n + x]);
                mass += value;
                moment += value * (x as f64 + 0.5) * 64.0 / n as f64;
            }
        }
        if mass <= 1e-9 {
            return Err("diagnostic sensor cannot identify an agent in the current frame".into());
        }
        let coordinate = (moment / mass - 8.0) / 8.0;
        match self.config.mode {
            SensorMode::Effect2Ch => Ok(vec![
                f32::from(coordinate < 2.5),
                f32::from(coordinate > 3.5),
            ]),
            SensorMode::Position7Ch => {
                let mut result = vec![0.0; 7];
                result[coordinate.round().clamp(0.0, 6.0) as usize] = 1.0;
                Ok(result)
            }
            SensorMode::Pixels => unreachable!(),
        }
    }
    pub fn encode(&mut self, frame: &Frame) -> Result<Vec<ChannelRate>> {
        if self.config.mode == SensorMode::Pixels {
            return self.pixels.encode(frame);
        }
        let features = self.features(frame)?;
        let mut result = Vec::with_capacity(self.config.channels());
        for (i, &value) in features.iter().enumerate() {
            if self.config.coding == SpikeCoding::OnOff {
                let change = self.previous.as_ref().map_or(0.0, |old| value - old[i]);
                result.push(ChannelRate {
                    channel: i as u64,
                    rate_hz: change.max(0.0) * self.config.max_rate_hz,
                });
                result.push(ChannelRate {
                    channel: (i + features.len()) as u64,
                    rate_hz: (-change).max(0.0) * self.config.max_rate_hz,
                });
            } else {
                result.push(ChannelRate {
                    channel: i as u64,
                    rate_hz: value * self.config.max_rate_hz,
                });
            }
        }
        self.previous = Some(features);
        Ok(result)
    }
}

#[derive(Debug)]
pub(crate) enum SpikeSource {
    Rate(RateSensoryTransducer),
    Poisson(PoissonSensoryTransducer),
}
impl SpikeSource {
    pub fn new(coding: SpikeCoding, seed: u64) -> Self {
        if coding == SpikeCoding::Deterministic {
            Self::Rate(RateSensoryTransducer::new())
        } else {
            Self::Poisson(PoissonSensoryTransducer::new(seed))
        }
    }
    pub fn push(&mut self, signal: ReceptorSignal) -> Result<()> {
        match self {
            Self::Rate(source) => source.push(signal)?,
            Self::Poisson(source) => source.push(signal)?,
        };
        Ok(())
    }
    pub fn advance_until(&mut self, until: SimTime, output: &mut Vec<ChannelSpike>) -> Result<()> {
        match self {
            Self::Rate(source) => source.advance_until(until, output)?,
            Self::Poisson(source) => source.advance_until(until, output)?,
        };
        Ok(())
    }
}
