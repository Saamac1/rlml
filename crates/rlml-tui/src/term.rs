//! The terminal loop: raw mode and the alternate screen on `/dev/tty`,
//! frames drawn by changed cells, keys and resizes read as they come
//! (Termplate's terminal layer). The terminal is restored on every exit.

use std::io;
use std::time::{Duration, Instant};

use rlml_core::model::Model;
use termplate_core::engine::InputEvent;
use termplate_core::keys::Binding;
use termplate_core::style::StyleSheet;
use termplate_term::{Screen, TtyGuard, map_event, no_color_from_env, terminal_size};

use crate::app::App;

/// Plays a new game in the terminal until the player quits. A runtime
/// error ends the game and is returned after the terminal is restored.
pub fn play(m: &Model, seed: u64) -> Result<(), String> {
    // the mouse is on unless the player's settings turn it off (D 3.2)
    let mouse = m.settings().get("mouse").and_then(|i| i.word(1)) != Some("off");
    let (guard, tty) = TtyGuard::new(mouse).map_err(|e| format!("cannot open the terminal: {e}"))?;
    let no_color = no_color_from_env(std::env::var("NO_COLOR").ok().as_deref());
    let mut screen = Screen::new(tty, StyleSheet::default(), no_color);
    let result = run(m, seed, &mut screen);
    drop(guard);
    result
}

fn run(m: &Model, seed: u64, screen: &mut Screen<std::fs::File>) -> Result<(), String> {
    let io = |e: io::Error| e.to_string();
    let start = Instant::now();
    let mut app = App::new(m, seed, terminal_size())?;
    // pads come from their own thread, as raw events (M 8.2)
    let pads = crate::pad::spawn();
    let mut pad_states: std::collections::BTreeMap<usize, crate::pad::PadState> = Default::default();
    loop {
        // in-between frames first, each shown `frame` long (M 7.2)
        for f in std::mem::take(&mut app.frames) {
            screen.draw(&f).map_err(io)?;
            std::thread::sleep(app.frame_time());
        }
        screen.draw(&app.draw()).map_err(io)?;
        if app.quit {
            return Ok(());
        }
        // every event that is waiting goes before the next frame, and the
        // pointer's moves count only at their last cell: a moving mouse
        // sends dozens of moves a second, and a frame for each would queue
        // the keys behind them
        if crossterm::event::poll(app.wait()).map_err(io)? {
            let mut moved = None;
            loop {
                match map_event(crossterm::event::read().map_err(io)?) {
                    Some(InputEvent::MouseMove { x, y }) => moved = Some((x as usize, y as usize)),
                    Some(ev) => {
                        if let Some((x, y)) = moved.take() {
                            app.pointer_at(x, y);
                        }
                        handle(&mut app, screen, ev, start.elapsed())?;
                    }
                    None => {}
                }
                if app.quit || !crossterm::event::poll(Duration::ZERO).map_err(io)? {
                    break;
                }
            }
            if let Some((x, y)) = moved {
                app.pointer_at(x, y);
            }
        }
        while let Ok((id, raw)) = pads.try_recv() {
            match raw {
                crate::pad::Raw::Connected => app.pad_connected(id),
                crate::pad::Raw::Disconnected => {
                    pad_states.remove(&id);
                    app.pad_disconnected(id);
                }
                raw => {
                    let deadzone = app.input.keys.deadzone(&app.contexts());
                    if let Some(p) = pad_states.entry(id).or_default().event(&raw, deadzone) {
                        app.pad(id, p, start.elapsed())?;
                    }
                }
            }
        }
        app.tick(start.elapsed())?;
    }
}

/// One input event other than a pointer move.
fn handle(app: &mut App<'_>, screen: &mut Screen<std::fs::File>, ev: InputEvent, now: Duration) -> Result<(), String> {
    match ev {
        InputEvent::Key(code, mods) => app.key(Binding::key(code, mods), now)?,
        InputEvent::Mouse { binding, x, y } => app.mouse_press(binding, x as usize, y as usize, now)?,
        InputEvent::MouseMove { x, y } => app.pointer_at(x as usize, y as usize),
        InputEvent::Resize(w, h) => {
            app.resize(w, h);
            screen.invalidate();
        }
    }
    Ok(())
}
