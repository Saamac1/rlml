//! The terminal loop: raw mode and the alternate screen on `/dev/tty`,
//! frames drawn by changed cells, keys and resizes read as they come
//! (Termplate's terminal layer). The terminal is restored on every exit.

use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use rlml_app::app::App;
pub use rlml_app::app::{DevFiles, Reloader};
pub use rlml_app::debug::DebugSetup;
use rlml_app::debug::DebugView;
use rlml_app::events::{Event, Feed};
use rlml_core::model::Model;
use rlml_core::rt::debug::{Debugger, Step};
use termplate_core::engine::InputEvent;
use termplate_core::keys::Binding;
use termplate_core::style::StyleSheet;
use termplate_term::{Screen, TtyGuard, map_event, no_color_from_env, terminal_size};

/// Plays a new game in the terminal until the player quits. A runtime
/// error ends the game and is returned after the terminal is restored.
/// `log_out` gets the last game's input log (M 4.4), also when a runtime
/// error ends it. The game's data folder `data` keeps its saves in
/// `saves/` and its profile (D 3.2).
pub fn play<'m>(
    m: &'m Model,
    seed: u64,
    log_out: Option<&std::path::Path>,
    data: Option<&std::path::Path>,
    dev: Option<Reloader<'m>>,
    debug: Option<DebugSetup>,
) -> Result<(), String> {
    // the mouse is on unless the player's settings turn it off (D 3.2)
    let mouse = m.settings().get("mouse").and_then(|i| i.word(1)) != Some("off");
    let (guard, tty) =
        TtyGuard::new(mouse).map_err(|e| format!("cannot open the terminal: {e}"))?;
    let no_color = no_color_from_env(std::env::var("NO_COLOR").ok().as_deref());
    let mut screen = Screen::new(tty, StyleSheet::default(), no_color);
    let mut app = App::new(m, seed, terminal_size())?;
    // development mode: `f12` opens the inspector and `f5` reloads
    app.dev = dev.is_some();
    app.reloader = dev;
    // breakpoints stop the game in a view of their own (M 5.7)
    let redraw = Arc::new(AtomicBool::new(false));
    if let Some(setup) = debug {
        let mut d = Debugger::new(stop_hook(setup.source, no_color, redraw.clone()));
        for spec in &setup.breaks {
            d.add(m, spec)?;
        }
        app.debugger = Some(d.shared());
    }
    if let Some(dir) = data {
        app.set_store(Box::new(crate::saves::DiskStore::new(&dir.join("saves"))));
        // the first game started before the profile was there
        app.set_profile_store(Box::new(crate::saves::DiskStore::plain(dir)), true)?;
    }
    let result = run(&mut app, &mut screen, &redraw);
    drop(guard);
    if let Some(path) = log_out {
        let written =
            app.r.input_log().write().and_then(|t| {
                std::fs::write(path, t).map_err(|e| format!("{}: {e}", path.display()))
            });
        if let Err(e) = written {
            eprintln!("the input log was not written: {e}");
        }
    }
    result
}

/// The debugger's hook in the terminal: the stop's view on a screen of its
/// own, keys read until one continues or steps. The game's screen is drawn
/// whole after it.
fn stop_hook(
    source: rlml_app::debug::Source,
    no_color: bool,
    redraw: Arc<AtomicBool>,
) -> rlml_core::rt::debug::Hook {
    Box::new(move |w, stop| {
        let Ok(tty) = std::fs::OpenOptions::new().write(true).open("/dev/tty") else {
            return Step::Continue;
        };
        let mut screen = Screen::new(tty, StyleSheet::default(), no_color);
        let mut view = DebugView::new(stop.clone(), source(&stop.file));
        redraw.store(true, Ordering::Relaxed);
        loop {
            let (cw, ch) = terminal_size();
            if screen.draw(&view.draw(cw, ch)).is_err() {
                return Step::Continue;
            }
            match crossterm::event::read().map(map_event) {
                Ok(Some(InputEvent::Key(code, mods))) => {
                    if let Some(step) = view.key(&Binding::key(code, mods), w) {
                        return step;
                    }
                }
                Ok(Some(InputEvent::Resize(..))) => screen.invalidate(),
                Ok(_) => {}
                Err(_) => return Step::Continue,
            }
        }
    })
}

fn run(
    app: &mut App,
    screen: &mut Screen<std::fs::File>,
    redraw: &AtomicBool,
) -> Result<(), String> {
    let io = |e: io::Error| e.to_string();
    let start = Instant::now();
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
        // a stop drew over the game
        if redraw.swap(false, Ordering::Relaxed) {
            screen.invalidate();
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
        if feed.apply(app, events, start.elapsed())? {
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
