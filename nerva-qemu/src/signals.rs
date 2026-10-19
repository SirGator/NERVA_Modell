//! Adapter-owned values. Conversion to neural-library types belongs to the host.

/// Firing rate requested by one encoded visual channel, in Hertz.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChannelRate {
    pub channel: u64,
    pub rate_hz: f32,
}

/// One motor pulse on an adapter channel, timestamped in microseconds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MotorPulse {
    pub channel: u64,
    pub at_us: u64,
    pub amplitude: f32,
}
