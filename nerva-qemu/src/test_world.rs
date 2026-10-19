//! Small deterministic pixel/keyboard environment. No NERVA types or policy.
//! `Assessment` belongs to the evaluator; agents receive only `Frame`.

use crate::{Result, capture::Frame, keyboard::Key};

pub const POSITIONS: u8 = 7;

pub trait PixelEnvironment {
    fn observe(&self) -> Result<Frame>;
    /// One pulse advances one cell. None is an explicit idle action.
    fn act(&mut self, key: Option<Key>) -> Result<()>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EpisodeSpec {
    pub start: u8,
    pub target: u8,
    pub reversed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Assessment {
    pub reached: bool,
    pub distance: u8,
}

pub struct MiniWorld {
    position: u8,
    target: u8,
    reversed: bool,
}

impl MiniWorld {
    pub fn new(spec: EpisodeSpec) -> Result<Self> {
        if spec.start >= POSITIONS || spec.target >= POSITIONS || spec.start == spec.target {
            return Err("mini-world needs two different positions in 0..7".into());
        }
        Ok(Self {
            position: spec.start,
            target: spec.target,
            reversed: spec.reversed,
        })
    }

    pub fn assess(&self) -> Assessment {
        Assessment {
            reached: self.position == self.target,
            distance: self.position.abs_diff(self.target),
        }
    }
}

impl PixelEnvironment for MiniWorld {
    fn observe(&self) -> Result<Frame> {
        render(self.position, Some(self.target))
    }

    fn act(&mut self, key: Option<Key>) -> Result<()> {
        let direction: i8 = match key {
            Some(Key::A) => -1,
            Some(Key::D) => 1,
            None => 0,
            _ => return Err("mini-world accepts only A, D or no key".into()),
        };
        if self.assess().reached {
            return Err("episode has already reached its target".into());
        }
        let direction = if self.reversed { -direction } else { direction };
        self.position = (self.position as i8 + direction).clamp(0, POSITIONS as i8 - 1) as u8;
        Ok(())
    }
}

/// Goal-free seven-position world for motor/effect tests.
pub struct MotionWorld {
    position: u8,
    reversed: bool,
}
impl MotionWorld {
    pub fn new(start: u8, reversed: bool) -> Result<Self> {
        if start >= POSITIONS {
            return Err("motion-world position must be in 0..7".into());
        }
        Ok(Self {
            position: start,
            reversed,
        })
    }
}
impl PixelEnvironment for MotionWorld {
    fn observe(&self) -> Result<Frame> {
        render(self.position, None)
    }
    fn act(&mut self, key: Option<Key>) -> Result<()> {
        let direction: i8 = match key {
            Some(Key::A) => -1,
            Some(Key::D) => 1,
            None => 0,
            _ => return Err("motion-world accepts A/D or idle only".into()),
        };
        let direction = if self.reversed { -direction } else { direction };
        self.position = (self.position as i8 + direction).clamp(0, POSITIONS as i8 - 1) as u8;
        Ok(())
    }
}
fn render(position: u8, target: Option<u8>) -> Result<Frame> {
    let mut rgb = vec![0; 64 * 64 * 3];
    let mut pixel = |x: usize, y: usize, value: u8| {
        rgb[(y * 64 + x) * 3..(y * 64 + x) * 3 + 3].fill(value);
    };
    for x in 4..61 {
        pixel(x, 42, 32);
    }
    for position in 0..POSITIONS {
        pixel(8 + usize::from(position) * 8, 44, 64);
    }
    // Hollow goal and solid agent remain distinguishable in grayscale.
    if let Some(target) = target {
        let goal = 8 + usize::from(target) * 8;
        for y in 22..36 {
            for x in goal - 3..=goal + 3 {
                if matches!(y, 22 | 35) || x == goal - 3 || x == goal + 3 {
                    pixel(x, y, 255);
                }
            }
        }
    }
    let agent = 8 + usize::from(position) * 8;
    for y in 26..32 {
        for x in agent - 2..agent + 2 {
            pixel(x, y, 255);
        }
    }
    Frame::new(64, 64, rgb)
}
