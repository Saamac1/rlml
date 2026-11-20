//! The terminal loop: raw mode and the alternate screen on `/dev/tty`,
//! frames drawn by changed cells, keys and resizes read as they come
//! (Termplate's terminal layer). The terminal is restored on every exit.

use std::io;
use std::time::{Duration, Instant};

use rlml_app::app::App;
use rlml_app::events::{Event, Feed};
use rlml_core::model::Model;
use termplate_core::engine::InputEvent;
use termplate_core::keys::Binding;
use termplate_core::style::StyleSheet;
use termplate_term::{Screen, TtyGuard, map_event, no_color_from_env, terminal_size};

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
    app.view.truecolor = std::env::var("COLORTERM").is_ok_and(|v| v == "truecolor" || v == "24bit");
    // pads come from their own thread, as raw events (M 8.2)
    let pads = crate::pad::spawn();
    let mut feed = Feed::default();
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
        // every event that is waiting goes before the next frame
        let mut events = Vec::new();
        if crossterm::event::poll(app.wait()).map_err(io)? {
            loop {
                events.extend(map_event(crossterm::event::read().map_err(io)?).map(event));
                if !crossterm::event::poll(Duration::ZERO).map_err(io)? {
                    break;
                }
            }
        }
        events.extend(pads.try_iter().map(|(id, raw)| Event::Pad(id, raw)));
        if feed.apply(&mut app, events, start.elapsed())? {
            screen.invalidate();
        }
        app.tick(start.elapsed())?;
    }
}

/// A terminal event in the app's terms.
fn event(ev: InputEvent) -> Event {
    match ev {
        InputEvent::Key(code, mods) => Event::Key(Binding::key(code, mods)),
        InputEvent::Mouse { binding, x, y } => Event::Press { binding, x, y },
        InputEvent::MouseMove { x, y } => Event::Move { x, y },
        InputEvent::Resize(w, h) => Event::Resize(w, h),
    }
}
