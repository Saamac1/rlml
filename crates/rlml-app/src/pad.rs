//! Pads (D 11.3, M 8.2): raw button and stick events turned into presses
//! with the place-based names of `pad` sections. A button held when
//! another is pressed makes a chord (`lb+start`), and a stick gives its
//! direction once when it crosses the deadzone. The front end's reader
//! (gilrs in the terminal) only forwards raw events, so this stays
//! testable without a device.

use std::collections::{BTreeMap, BTreeSet};

use crate::input::PadPress;

/// A pad's raw event: a button by name pressed or released, or a stick or
/// d-pad axis by name (`lstick.x`, `lstick.y`, `rstick.x`, `rstick.y`,
/// `dpad.x`, `dpad.y`) moved to a value in -1.0..=1.0, up and right
/// positive.
#[derive(Debug, Clone, PartialEq)]
pub enum Raw {
    Button { name: String, down: bool },
    Axis { name: String, value: f32 },
    Connected,
    Disconnected,
}

/// What one pad holds now.
#[derive(Debug, Clone, Default)]
pub struct PadState {
    held: BTreeSet<String>,
    /// Each axis' side past the deadzone: -1, 0 or 1.
    sides: BTreeMap<String, i8>,
}

impl PadState {
    /// The press a raw event makes, if any; `deadzone` is a share of the
    /// stick's reach.
    pub fn event(&mut self, raw: &Raw, deadzone: f32) -> Option<PadPress> {
        match raw {
            Raw::Button { name, down: true } => {
                let held: Vec<String> = self.held.iter().filter(|h| *h != name).cloned().collect();
                self.held.insert(name.clone());
                Some(PadPress { button: name.clone(), held })
            }
            Raw::Button { name, down: false } => {
                self.held.remove(name);
                None
            }
            Raw::Axis { name, value } => {
                let side = if *value > deadzone {
                    1
                } else if *value < -deadzone {
                    -1
                } else {
                    0
                };
                let was = self.sides.insert(name.clone(), side).unwrap_or(0);
                if side == 0 || side == was {
                    return None;
                }
                let (stick, axis) = name.rsplit_once('.')?;
                let dir = match (axis, side) {
                    ("x", 1) => "right",
                    ("x", _) => "left",
                    ("y", 1) => "up",
                    _ => "down",
                };
                Some(PadPress { button: format!("{stick}.{dir}"), held: self.held.iter().cloned().collect() })
            }
            Raw::Connected | Raw::Disconnected => None,
        }
    }
}

/// A gilrs event as a raw event: the reader's part that both the
/// terminal (a thread) and the browser (polled each frame) share.
#[cfg(feature = "gilrs")]
pub fn from_gilrs(ev: gilrs::EventType) -> Option<Raw> {
    use gilrs::{Axis, Button, EventType};
    let button = |b: Button| {
        Some(match b {
            Button::South => "south",
            Button::East => "east",
            Button::West => "west",
            Button::North => "north",
            Button::LeftTrigger => "lb",
            Button::RightTrigger => "rb",
            Button::LeftTrigger2 => "lt",
            Button::RightTrigger2 => "rt",
            Button::Start => "start",
            Button::Select => "select",
            Button::LeftThumb => "lstick",
            Button::RightThumb => "rstick",
            Button::DPadUp => "dpad.up",
            Button::DPadDown => "dpad.down",
            Button::DPadLeft => "dpad.left",
            Button::DPadRight => "dpad.right",
            _ => return None,
        })
    };
    Some(match ev {
        EventType::ButtonPressed(b, _) => Raw::Button { name: button(b)?.to_string(), down: true },
        EventType::ButtonReleased(b, _) => Raw::Button { name: button(b)?.to_string(), down: false },
        EventType::AxisChanged(a, value, _) => {
            let name = match a {
                Axis::LeftStickX => "lstick.x",
                Axis::LeftStickY => "lstick.y",
                Axis::RightStickX => "rstick.x",
                Axis::RightStickY => "rstick.y",
                Axis::DPadX => "dpad.x",
                Axis::DPadY => "dpad.y",
                _ => return None,
            };
            Raw::Axis { name: name.to_string(), value }
        }
        EventType::Connected => Raw::Connected,
        EventType::Disconnected => Raw::Disconnected,
        _ => return None,
    })
}
