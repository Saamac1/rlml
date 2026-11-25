//! Browser keys as the terminal names them: a `KeyboardEvent`'s `key`
//! becomes the `KeyCode` that `termplate-term`'s `map_event` gives for
//! the same key from crossterm, so a game's `h j k l`, arrows, `esc` and
//! `.` work the same in both. Mouse buttons and the wheel get
//! crossterm's names too.

use std::collections::BTreeSet;

use rlml_app::input::Keys;
use termplate_core::keys::{Binding, KeyCode, Mods, Mouse};

/// The binding of a key press: `key` and `code` as the browser gives
/// them, with the modifiers held. `None` for a key the terminal has no
/// name for (a modifier alone, a dead key).
pub fn binding(key: &str, code: &str, ctrl: bool, alt: bool, shift: bool) -> Option<Binding> {
    let named = match key {
        "Enter" => Some(KeyCode::Enter),
        "Escape" | "Esc" => Some(KeyCode::Esc),
        "Tab" => Some(KeyCode::Tab),
        "Backspace" => Some(KeyCode::Backspace),
        "Delete" | "Del" => Some(KeyCode::Delete),
        "Insert" => Some(KeyCode::Insert),
        "ArrowUp" | "Up" => Some(KeyCode::Up),
        "ArrowDown" | "Down" => Some(KeyCode::Down),
        "ArrowLeft" | "Left" => Some(KeyCode::Left),
        "ArrowRight" | "Right" => Some(KeyCode::Right),
        "Home" => Some(KeyCode::Home),
        "End" => Some(KeyCode::End),
        "PageUp" => Some(KeyCode::PageUp),
        "PageDown" => Some(KeyCode::PageDown),
        "Spacebar" => Some(KeyCode::Char(' ')),
        f if f.len() >= 2 && f.starts_with('F') && f[1..].chars().all(|c| c.is_ascii_digit()) => f
            [1..]
            .parse()
            .ok()
            .filter(|n| (1..=24).contains(n))
            .map(KeyCode::F),
        _ => None,
    };
    let mods = Mods { ctrl, alt, shift };
    if let Some(k) = named {
        return Some(Binding::key(k, mods));
    }
    let mut chars = key.chars();
    let single = match (chars.next(), chars.next()) {
        (Some(c), None) if !c.is_control() => Some(c),
        _ => None,
    };
    // with ctrl or alt the browser may give the layout's other character
    // (macOS option+h is `˙`): the key's place names it, as a terminal does
    let ch = match single {
        Some(c) if !(ctrl || alt) || c.is_ascii() => c,
        _ if ctrl || alt => from_code(code)?,
        _ => return None,
    };
    Some(Binding::key(KeyCode::Char(ch), mods))
}

/// The binding of a `mousedown`: `MouseEvent.button` 0 is `click`, 1
/// `mclick`, 2 `rclick`; the others have no name.
pub fn mouse(button: i32, ctrl: bool, alt: bool, shift: bool) -> Option<Binding> {
    let m = match button {
        0 => Mouse::Click,
        1 => Mouse::MiddleClick,
        2 => Mouse::RightClick,
        _ => return None,
    };
    Some(Binding::Mouse(m, Mods { ctrl, alt, shift }))
}

/// The binding of one wheel step, `wheel.up` or `wheel.down`.
pub fn wheel(up: bool, ctrl: bool, alt: bool, shift: bool) -> Binding {
    Binding::Mouse(
        if up { Mouse::WheelUp } else { Mouse::WheelDown },
        Mods { ctrl, alt, shift },
    )
}

/// `KeyA` → `a`, `Digit1` → `1`.
fn from_code(code: &str) -> Option<char> {
    let c = code
        .strip_prefix("Key")
        .or_else(|| code.strip_prefix("Digit"))?;
    let mut chars = c.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) if c.is_ascii_alphanumeric() => Some(c.to_ascii_lowercase()),
        _ => None,
    }
}

/// Every key the game binds in any context, by name (`ctrl+q`, `K`,
/// `space`), and ctrl+c, which always leaves: the page keeps the
/// browser's own use of these keys from them.
pub fn bound(keys: &Keys) -> BTreeSet<String> {
    let mut out: BTreeSet<String> = keys
        .contexts
        .values()
        .flat_map(|c| c.rows.iter().chain(c.parts.values().flatten()))
        .map(|(b, _)| b.name())
        .collect();
    out.insert("ctrl+c".into());
    out
}
