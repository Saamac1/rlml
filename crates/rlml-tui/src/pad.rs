//! The pads' reader (D 11.3, M 8.2): gilrs events as raw events, which
//! `rlml_app::pad::PadState` turns into presses.

use rlml_app::pad::Raw;

/// The pads' reading thread: gilrs events as raw events by pad id, sent
/// to the terminal loop. Without pad support (no permission, no backend)
/// the thread ends and the game plays from the keyboard.
pub fn spawn() -> std::sync::mpsc::Receiver<(usize, Raw)> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let Ok(mut gilrs) = gilrs::Gilrs::new() else { return };
        for (id, _) in gilrs.gamepads() {
            if tx.send((usize::from(id), Raw::Connected)).is_err() {
                return;
            }
        }
        loop {
            let Some(ev) = gilrs.next_event_blocking(Some(std::time::Duration::from_millis(100))) else { continue };
            let Some(raw) = raw(ev.event) else { continue };
            if tx.send((usize::from(ev.id), raw)).is_err() {
                return;
            }
        }
    });
    rx
}

fn raw(ev: gilrs::EventType) -> Option<Raw> {
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
