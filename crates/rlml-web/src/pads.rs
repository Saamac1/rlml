//! Pads in the browser (M 8.2): gilrs reads the Gamepad API on the page,
//! which only has `navigator.getGamepads()` outside the worker, and its
//! raw events go to the worker as words, where `events::Feed` turns them
//! into presses as in the terminal.
//!
//! `ID c` connected, `ID d` disconnected, `ID b NAME 1|0` a button down
//! or up, `ID a NAME VALUE` an axis.

use rlml_app::pad::Raw;

pub fn encode(id: usize, raw: &Raw) -> String {
    match raw {
        Raw::Connected => format!("{id} c"),
        Raw::Disconnected => format!("{id} d"),
        Raw::Button { name, down } => format!("{id} b {name} {}", *down as u8),
        Raw::Axis { name, value } => format!("{id} a {name} {value}"),
    }
}

pub fn decode(s: &str) -> Option<(usize, Raw)> {
    let mut w = s.split(' ');
    let id = w.next()?.parse().ok()?;
    let raw = match (w.next()?, w.next(), w.next()) {
        ("c", None, None) => Raw::Connected,
        ("d", None, None) => Raw::Disconnected,
        ("b", Some(name), Some(down)) => Raw::Button { name: name.to_string(), down: down == "1" },
        ("a", Some(name), Some(value)) => Raw::Axis { name: name.to_string(), value: value.parse().ok()? },
        _ => return None,
    };
    w.next().is_none().then_some((id, raw))
}
