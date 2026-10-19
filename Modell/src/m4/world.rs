use crate::Result;
use nerva_qemu::{
    capture::Frame,
    keyboard::Key,
    test_world::{MotionWorld, PixelEnvironment},
};

pub const CANONICAL: usize = 256;
pub fn image(position: u8) -> Result<Frame> {
    let small = MotionWorld::new(position, false)?.observe()?;
    let mut rgb = vec![0; CANONICAL * CANONICAL * 3];
    for y in 0..CANONICAL {
        for x in 0..CANONICAL {
            let from = ((y / 4) * 64 + x / 4) * 3;
            let to = (y * CANONICAL + x) * 3;
            rgb[to..to + 3].copy_from_slice(&small.rgb()[from..from + 3]);
        }
    }
    Frame::new(CANONICAL, CANONICAL, rgb)
}
/// The evaluator owns this world; the model port cannot hold a reference to it.
pub struct DelayedWorld {
    world: MotionWorld,
    pending: Option<(u64, Key)>,
    pub action_us: u64,
    pub motion_us: Option<u64>,
}
impl DelayedWorld {
    pub fn new(reversed: bool) -> Result<Self> {
        Ok(Self {
            world: MotionWorld::new(3, reversed)?,
            pending: None,
            action_us: 0,
            motion_us: None,
        })
    }
    pub fn execute(&mut self, key: Key, time: u64, delay: u64) -> Result<()> {
        if self.pending.is_some() {
            return Err("world already has a pending action".into());
        }
        self.action_us = time;
        self.pending = Some((time.checked_add(delay).ok_or("world time overflow")?, key));
        Ok(())
    }
    pub fn advance(&mut self, time: u64) -> Result<()> {
        if let Some((due, key)) = self.pending
            && time >= due
        {
            self.world.act(Some(key))?;
            self.motion_us = Some(due);
            self.pending = None;
        }
        Ok(())
    }
    pub fn observe_native(&self) -> Result<Frame> {
        self.world.observe()
    }
    pub fn observe(&self) -> Result<Frame> {
        let small = self.world.observe()?;
        let mut rgb = vec![0; CANONICAL * CANONICAL * 3];
        for y in 0..CANONICAL {
            for x in 0..CANONICAL {
                let f = ((y / 4) * 64 + x / 4) * 3;
                let t = (y * CANONICAL + x) * 3;
                rgb[t..t + 3].copy_from_slice(&small.rgb()[f..f + 3]);
            }
        }
        Frame::new(CANONICAL, CANONICAL, rgb)
    }
}
pub fn fingerprint(frame: &Frame) -> u64 {
    frame.rgb().iter().fold(0xcbf29ce484222325, |h, &b| {
        (h ^ u64::from(b)).wrapping_mul(0x100000001b3)
    })
}
pub fn visible_effect(frame: &Frame) -> Result<i16> {
    use crate::model::{SensorConfig, SensorMode, sensory::SensorEncoder};
    let encoder = SensorEncoder::new(SensorConfig {
        mode: SensorMode::Effect2Ch,
        ..SensorConfig::default()
    })?;
    let feature = encoder.features(frame)?;
    match feature.as_slice() {
        [a, b] if *a > 0.0 && *b == 0.0 => Ok(-1),
        [a, b] if *b > 0.0 && *a == 0.0 => Ok(1),
        _ => Err("image has no unique left/right effect".into()),
    }
}
