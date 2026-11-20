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
            let Some(raw) = rlml_app::pad::from_gilrs(ev.event) else { continue };
            if tx.send((usize::from(ev.id), raw)).is_err() {
                return;
            }
        }
    });
    rx
}
