//! Input as a front end reads it (M 7, M 8): every event waiting goes to
//! the app before the next frame, and the pointer's moves count only at
//! their last cell. A moving mouse sends dozens of moves a second, and a
//! frame for each would queue the keys behind them. The terminal and the
//! browser both feed their events through here.

use std::collections::BTreeMap;
use std::time::Duration;

use rlml_core::rt::world::RtResult;
use termplate_core::keys::Binding;

use crate::app::App;
use crate::pad::{PadState, Raw};

/// One input event, in cells.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    Key(Binding),
    /// A mouse button or wheel at a cell.
    Press { binding: Binding, x: usize, y: usize },
    /// The pointer moved to a cell.
    Move { x: usize, y: usize },
    /// The window's new size in cells.
    Resize(usize, usize),
    /// A pad's raw event by pad id.
    Pad(usize, Raw),
}

/// What the front end keeps between batches: each pad's held buttons and
/// stick sides.
#[derive(Debug, Clone, Default)]
pub struct Feed {
    pads: BTreeMap<usize, PadState>,
}

impl Feed {
    /// Gives `app` a batch of events read since the last frame, in order,
    /// with each run of pointer moves cut to its last cell; the batch ends
    /// early when the app quits. True when the window changed size, so
    /// the front end draws the whole next frame.
    pub fn apply(&mut self, app: &mut App<'_>, events: impl IntoIterator<Item = Event>, now: Duration) -> RtResult<bool> {
        let mut resized = false;
        let mut moved = None;
        for ev in events {
            if app.quit {
                return Ok(resized);
            }
            if let Event::Move { x, y } = ev {
                moved = Some((x, y));
                continue;
            }
            if let Some((x, y)) = moved.take() {
                app.pointer_at(x, y);
            }
            match ev {
                Event::Key(b) => app.key(b, now)?,
                Event::Press { binding, x, y } => app.mouse_press(binding, x, y, now)?,
                Event::Move { .. } => {}
                Event::Resize(w, h) => {
                    app.resize(w, h);
                    resized = true;
                }
                Event::Pad(id, Raw::Connected) => app.pad_connected(id),
                Event::Pad(id, Raw::Disconnected) => {
                    self.pads.remove(&id);
                    app.pad_disconnected(id);
                }
                Event::Pad(id, raw) => {
                    let deadzone = app.input.keys.deadzone(&app.contexts());
                    if let Some(p) = self.pads.entry(id).or_default().event(&raw, deadzone) {
                        app.pad(id, p, now)?;
                    }
                }
            }
        }
        if let Some((x, y)) = moved
            && !app.quit
        {
            app.pointer_at(x, y);
        }
        Ok(resized)
    }
}
