//! Finite-window spike counting with optional first-spike tie breaking.

use crate::{Result, keyboard::Key, signals::MotorPulse};
use std::collections::VecDeque;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdRule {
    Count,
    CountThenFirstSpike,
    RandomTie,
}

/// Pure A/D rule for controlled comparisons. Randomness is
/// supplied by the experiment and only used for nonempty ties; silence stays idle.
pub fn decode_ad(
    pulses: &[MotorPulse],
    start_us: u64,
    end_us: u64,
    rule: AdRule,
    draw: u64,
) -> Result<Option<Key>> {
    if start_us > end_us {
        return Err("invalid A/D observation window".into());
    }
    let mut counts = [0usize; 2];
    let mut first = [None; 2];
    for pulse in pulses {
        let index = match Key::from_channel(pulse.channel) {
            Some(Key::A) => 0,
            Some(Key::D) => 1,
            _ => return Err("A/D decoder received another key".into()),
        };
        if pulse.at_us < start_us
            || pulse.at_us > end_us
            || !pulse.amplitude.is_finite()
            || pulse.amplitude <= 0.0
        {
            return Err("invalid A/D motor pulse".into());
        }
        counts[index] += 1;
        first[index] = Some(first[index].map_or(pulse.at_us, |old: u64| old.min(pulse.at_us)));
    }
    if counts[0] > counts[1] {
        return Ok(Some(Key::A));
    }
    if counts[1] > counts[0] {
        return Ok(Some(Key::D));
    }
    if counts[0] == 0 {
        return Ok(None);
    }
    Ok(match rule {
        AdRule::Count => None,
        AdRule::RandomTie => Some(if draw.is_multiple_of(2) {
            Key::A
        } else {
            Key::D
        }),
        AdRule::CountThenFirstSpike => match first[0].cmp(&first[1]) {
            std::cmp::Ordering::Less => Some(Key::A),
            std::cmp::Ordering::Greater => Some(Key::D),
            _ => None,
        },
    })
}

#[derive(Debug)]
pub struct MotorDecoder {
    window_us: u64,
    minimum_spikes: usize,
    events: VecDeque<MotorPulse>,
    last_horizon: Option<u64>,
    timing_ties: bool,
}

impl MotorDecoder {
    pub fn new(window_us: u64, minimum_spikes: usize) -> Result<Self> {
        if window_us == 0 || minimum_spikes == 0 {
            return Err("motor window and spike threshold must be positive".into());
        }
        Ok(Self {
            window_us,
            minimum_spikes,
            events: VecDeque::new(),
            last_horizon: None,
            timing_ties: false,
        })
    }

    /// Resolve nonempty count ties using a unique earliest first pulse.
    /// Exact first-pulse ties still abstain; modifiers keep their own channels.
    pub fn with_timing_ties(mut self, enabled: bool) -> Self {
        self.timing_ties = enabled;
        self
    }

    fn winner(&self, counts: &[usize; Key::COUNT], include: impl Fn(Key) -> bool) -> Option<Key> {
        if let Some(key) = unique_winner(counts, &include) {
            return Some(key);
        }
        if !self.timing_ties {
            return None;
        }
        let maximum = Key::ALL
            .iter()
            .copied()
            .filter(|&key| include(key))
            .map(|key| counts[key as usize])
            .max()?;
        if maximum == 0 {
            return None;
        }
        let first = self
            .events
            .iter()
            .filter(|pulse| {
                Key::from_channel(pulse.channel)
                    .is_some_and(|key| include(key) && counts[key as usize] == maximum)
            })
            .map(|pulse| pulse.at_us)
            .min()?;
        let mut keys = Key::ALL.iter().copied().filter(|&key| {
            include(key)
                && counts[key as usize] == maximum
                && self
                    .events
                    .iter()
                    .any(|pulse| pulse.channel == key.channel() && pulse.at_us == first)
        });
        let key = keys.next()?;
        if keys.next().is_some() {
            None
        } else {
            Some(key)
        }
    }

    /// Events must be complete, ordered, and newer than the preceding horizon.
    /// The counting interval is `(until - window, until]`.
    pub fn decode(&mut self, outputs: &[MotorPulse], until_us: u64) -> Result<Option<Key>> {
        let counts = self.count(outputs, until_us)?;
        Ok(self.winner(&counts, |_| true))
    }

    /// One ordinary key plus independently active modifiers. Ordinary-key ties
    /// use the configured rule; silence also releases all modifiers.
    pub fn decode_keys(&mut self, outputs: &[MotorPulse], until_us: u64) -> Result<Vec<Key>> {
        let counts = self.count(outputs, until_us)?;
        let mut keys: Vec<_> = Key::ALL
            .iter()
            .copied()
            .filter(|key| key.is_modifier() && counts[*key as usize] > 0)
            .collect();
        if let Some(key) = self.winner(&counts, |key| !key.is_modifier()) {
            keys.push(key);
        }
        Ok(keys)
    }

    fn count(&mut self, outputs: &[MotorPulse], until_us: u64) -> Result<[usize; Key::COUNT]> {
        let until = until_us;
        if self.last_horizon.is_some_and(|last| until < last) {
            return Err("motor horizon went backwards".into());
        }
        let mut last_event = self.last_horizon;
        for event in outputs {
            if event.channel >= Key::ALL.len() as u64
                || !event.amplitude.is_finite()
                || event.amplitude <= 0.0
                || event.at_us > until
                || self.last_horizon.is_some_and(|last| event.at_us <= last)
                || last_event.is_some_and(|last| event.at_us < last)
            {
                return Err("invalid or out-of-order motor output".into());
            }
            last_event = Some(event.at_us);
        }
        self.events.extend(outputs.iter().copied());
        if let Some(cutoff) = until.checked_sub(self.window_us) {
            while self
                .events
                .front()
                .is_some_and(|event| event.at_us <= cutoff)
            {
                self.events.pop_front();
            }
        }
        let mut counts = [0usize; Key::COUNT];
        for event in &self.events {
            counts[event.channel as usize] += 1;
        }
        self.last_horizon = Some(until);
        for count in &mut counts {
            if *count < self.minimum_spikes {
                *count = 0;
            }
        }
        Ok(counts)
    }
}

/// Stable German ISO-105 channels; the original W/A/S/D/Space retain IDs 0–4.
pub fn key_channel(key: Key) -> u64 {
    key.channel()
}

fn unique_winner(counts: &[usize; Key::COUNT], include: impl Fn(Key) -> bool) -> Option<Key> {
    let eligible = || Key::ALL.iter().copied().filter(|key| include(*key));
    let maximum = eligible()
        .map(|key| counts[key as usize])
        .max()
        .unwrap_or(0);
    if maximum == 0 {
        return None;
    }
    let mut winners = eligible().filter(|key| counts[*key as usize] == maximum);
    let first = winners.next()?;
    if winners.next().is_some() {
        None
    } else {
        Some(first)
    }
}
