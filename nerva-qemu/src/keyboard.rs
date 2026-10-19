//! Stateful key transitions; held keys are released when the session is dropped.

use crate::{
    Result,
    capture::{self, Frame},
    qmp::Qmp,
};
use serde_json::json;
use std::{collections::BTreeSet, path::Path};

// The first five channels preserve the original W/A/S/D/Space mapping.
// Names and labels refer to a German ISO keyboard; QMP uses physical US names.
macro_rules! define_keys {
    ($(($key:ident, $qcode:literal, $label:literal)),+ $(,)?) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
        #[repr(usize)]
        pub enum Key { $($key),+ }

        impl Key {
            pub const ALL: &'static [Self] = &[$(Self::$key),+];
            pub const COUNT: usize = Self::ALL.len();
            pub const fn channel(self) -> u64 { self as u64 }
            pub fn from_channel(channel: u64) -> Option<Self> {
                usize::try_from(channel).ok().and_then(|index| Self::ALL.get(index).copied())
            }
            pub const fn qcode(self) -> &'static str { match self { $(Self::$key => $qcode),+ } }
            pub const fn label(self) -> &'static str { match self { $(Self::$key => $label),+ } }
            pub const fn is_modifier(self) -> bool {
                matches!(self, Self::LeftShift | Self::RightShift | Self::LeftCtrl | Self::RightCtrl
                    | Self::LeftAlt | Self::RightAlt | Self::LeftMeta | Self::RightMeta)
            }
        }
    };
}

define_keys! {
    (W, "w", "W"),
    (A, "a", "A"),
    (S, "s", "S"),
    (D, "d", "D"),
    (Space, "spc", "Leertaste"),
    (Escape, "esc", "Esc"),
    (F1, "f1", "F1"),
    (F2, "f2", "F2"),
    (F3, "f3", "F3"),
    (F4, "f4", "F4"),
    (F5, "f5", "F5"),
    (F6, "f6", "F6"),
    (F7, "f7", "F7"),
    (F8, "f8", "F8"),
    (F9, "f9", "F9"),
    (F10, "f10", "F10"),
    (F11, "f11", "F11"),
    (F12, "f12", "F12"),
    (PrintScreen, "sysrq", "Druck"),
    (ScrollLock, "scroll_lock", "Rollen"),
    (Pause, "pause", "Pause"),
    (Caret, "grave_accent", "^"),
    (Digit1, "1", "1"),
    (Digit2, "2", "2"),
    (Digit3, "3", "3"),
    (Digit4, "4", "4"),
    (Digit5, "5", "5"),
    (Digit6, "6", "6"),
    (Digit7, "7", "7"),
    (Digit8, "8", "8"),
    (Digit9, "9", "9"),
    (Digit0, "0", "0"),
    (SharpS, "minus", "ß"),
    (Acute, "equal", "´"),
    (Backspace, "backspace", "Rücktaste"),
    (Tab, "tab", "Tab"),
    (Q, "q", "Q"),
    (E, "e", "E"),
    (R, "r", "R"),
    (T, "t", "T"),
    (Z, "y", "Z"),
    (U, "u", "U"),
    (I, "i", "I"),
    (O, "o", "O"),
    (P, "p", "P"),
    (UmlautU, "bracket_left", "Ü"),
    (Plus, "bracket_right", "+"),
    (Enter, "ret", "Enter"),
    (CapsLock, "caps_lock", "Feststelltaste"),
    (F, "f", "F"),
    (G, "g", "G"),
    (H, "h", "H"),
    (J, "j", "J"),
    (K, "k", "K"),
    (L, "l", "L"),
    (UmlautO, "semicolon", "Ö"),
    (UmlautA, "apostrophe", "Ä"),
    (Hash, "backslash", "#"),
    (LeftShift, "shift", "Shift links"),
    (LessThan, "less", "<"),
    (Y, "z", "Y"),
    (X, "x", "X"),
    (C, "c", "C"),
    (V, "v", "V"),
    (B, "b", "B"),
    (N, "n", "N"),
    (M, "m", "M"),
    (Comma, "comma", ","),
    (Period, "dot", "."),
    (Hyphen, "slash", "-"),
    (RightShift, "shift_r", "Shift rechts"),
    (LeftCtrl, "ctrl", "Strg links"),
    (LeftMeta, "meta_l", "Windows links"),
    (LeftAlt, "alt", "Alt"),
    (RightAlt, "alt_r", "AltGr"),
    (RightMeta, "meta_r", "Windows rechts"),
    (Menu, "menu", "Menü"),
    (RightCtrl, "ctrl_r", "Strg rechts"),
    (Insert, "insert", "Einfg"),
    (Home, "home", "Pos1"),
    (PageUp, "pgup", "Bild auf"),
    (Delete, "delete", "Entf"),
    (End, "end", "Ende"),
    (PageDown, "pgdn", "Bild ab"),
    (Up, "up", "Pfeil hoch"),
    (Left, "left", "Pfeil links"),
    (Down, "down", "Pfeil runter"),
    (Right, "right", "Pfeil rechts"),
    (NumLock, "num_lock", "Num"),
    (NumpadDivide, "kp_divide", "Num /"),
    (NumpadMultiply, "kp_multiply", "Num *"),
    (NumpadSubtract, "kp_subtract", "Num -"),
    (Numpad7, "kp_7", "Num 7"),
    (Numpad8, "kp_8", "Num 8"),
    (Numpad9, "kp_9", "Num 9"),
    (NumpadAdd, "kp_add", "Num +"),
    (Numpad4, "kp_4", "Num 4"),
    (Numpad5, "kp_5", "Num 5"),
    (Numpad6, "kp_6", "Num 6"),
    (Numpad1, "kp_1", "Num 1"),
    (Numpad2, "kp_2", "Num 2"),
    (Numpad3, "kp_3", "Num 3"),
    (NumpadEnter, "kp_enter", "Num Enter"),
    (Numpad0, "kp_0", "Num 0"),
    (NumpadDecimal, "kp_decimal", "Num ,"),
}

pub struct Keyboard {
    qmp: Qmp,
    held: BTreeSet<Key>,
}

impl Keyboard {
    pub fn new(qmp: Qmp) -> Self {
        Self {
            qmp,
            held: BTreeSet::new(),
        }
    }

    pub fn capture(&mut self, path: &Path) -> Result<Frame> {
        capture::capture(&mut self.qmp, path)
    }

    pub fn set_keys(&mut self, keys: impl IntoIterator<Item = Key>) -> Result<()> {
        let next: BTreeSet<_> = keys.into_iter().collect();
        let mut released: Vec<_> = self.held.difference(&next).copied().collect();
        let mut pressed: Vec<_> = next.difference(&self.held).copied().collect();
        // Release ordinary keys before modifiers; press modifiers first.
        released.sort_by_key(|key| (key.is_modifier(), *key));
        pressed.sort_by_key(|key| (!key.is_modifier(), *key));
        let events: Vec<_> = released
            .into_iter()
            .map(|key| event(key, false))
            .chain(pressed.into_iter().map(|key| event(key, true)))
            .collect();
        if !events.is_empty() {
            self.qmp
                .execute("input-send-event", json!({"events": events}))?;
        }
        self.held = next;
        Ok(())
    }

    /// Release all supported keys, also after a partially acknowledged action.
    pub fn release_all(&mut self) -> Result<()> {
        let mut keys = Key::ALL.to_vec();
        keys.sort_by_key(|key| (key.is_modifier(), *key));
        let events: Vec<_> = keys.into_iter().map(|key| event(key, false)).collect();
        self.qmp
            .execute("input-send-event", json!({"events": events}))?;
        self.held.clear();
        Ok(())
    }
}

impl Drop for Keyboard {
    fn drop(&mut self) {
        let _ = self.release_all();
    }
}

fn event(key: Key, down: bool) -> serde_json::Value {
    json!({"type": "key", "data": {"down": down, "key": {"type": "qcode", "data": key.qcode()}}})
}
