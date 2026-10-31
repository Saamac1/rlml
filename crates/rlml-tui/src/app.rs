//! The game in a terminal without the terminal: keys, time and window
//! size come in, frames go out, so it can be driven and tested with a
//! clock of its own (M 6.1.2, M 7.2, M 8).
//!
//! * On the player's turn a key's command is the player's command.
//! * Without the player's turn the world goes on a round at the `round`
//!   pace, and `pass` runs the next round at once (M 6.1.2).
//! * `idle` gives the player's turn away after its time without a key.
//! * With `presentation frame`, the moments where something visible
//!   changed become in-between frames shown that long each (M 7.2).

use std::time::Duration;

use rlml_core::model::Model;
use rlml_core::rt::run::{Runner, Status};
use rlml_core::rt::value::{CommandValue, Value};
use rlml_core::rt::world::{RtResult, World};
use termplate_core::frame::Frame;
use termplate_core::keys::{Binding, KeyCode, Mods};

use crate::draw::View;
use crate::input::{Effect, Input};

/// `500ms`, `1s`, `0500ms` (D 2.4).
pub fn time(s: &str) -> Option<Duration> {
    if let Some(ms) = s.strip_suffix("ms") {
        return ms.parse().ok().map(Duration::from_millis);
    }
    s.strip_suffix('s')?.parse().ok().map(Duration::from_secs)
}

/// `idle TIME [ai] [COMMAND]` (D 3.2).
#[derive(Debug, Clone, PartialEq)]
pub struct Idle {
    pub after: Duration,
    pub ai: bool,
    pub command: Option<String>,
}

pub struct App<'m> {
    pub m: &'m Model,
    pub r: Runner<'m>,
    pub view: View,
    pub input: Input,
    /// The key context stack, bottom first (M 8).
    pub stack: Vec<String>,
    pub lang: String,
    pub size: (usize, usize),
    pub status: Status,
    pub quit: bool,
    /// A line for the player: a key that did nothing, a menu not there yet.
    pub note: Option<String>,
    round: Option<Duration>,
    idle: Option<Idle>,
    frame: Duration,
    last_round: Duration,
    last_key: Duration,
    /// In-between frames waiting to be shown, `frame` apart.
    pub frames: Vec<Frame>,
    /// The last in-between frame, to keep only changed moments.
    last_frame: Option<Frame>,
}

impl<'m> App<'m> {
    pub fn new(m: &'m Model, seed: u64, size: (usize, usize)) -> RtResult<App<'m>> {
        let mut w = World::new_game(m, seed)?;
        w.begin()?;
        let settings = m.settings();
        let setting = |k: &str| settings.get(k).map(|i| i.rest(1).to_string());
        let idle = settings.get("idle").and_then(|i| {
            let after = time(i.word(1)?)?;
            let ai = i.tokens.iter().any(|t| t.is("ai"));
            let command = i.tokens.iter().skip(2).find(|t| !t.is("ai")).and_then(|t| m.resolve(i.namespace.as_deref(), &t.text).ok()).map(|d| d.full.clone());
            Some(Idle { after, ai, command })
        });
        let frame = m.sections.iter().filter(|b| b.kind == "presentation").flat_map(|b| b.items.iter()).find(|i| i.key() == "frame").and_then(|i| i.word(1)).and_then(time).unwrap_or_default();
        let input = Input::new(m);
        let stack = input.base_stack();
        let mut app = App {
            m,
            r: Runner::new(w),
            view: View::default(),
            input,
            stack,
            lang: setting("language").unwrap_or_default(),
            size,
            status: Status::NoPlayer { rounds: 0 },
            quit: false,
            note: None,
            round: setting("round").as_deref().and_then(time),
            idle,
            frame,
            last_round: Duration::ZERO,
            last_key: Duration::ZERO,
            frames: vec![],
            last_frame: None,
        };
        app.advance(0)?;
        Ok(app)
    }

    /// The player's turn: the scheduler waits for input, and the player is
    /// alive (it may have died since).
    pub fn players_turn(&self) -> bool {
        self.status == Status::Input && self.r.w.s.player.is_some_and(|p| self.r.w.s.get(p).alive)
    }

    /// The frame for the window now.
    pub fn draw(&mut self) -> Frame {
        let (w, h) = self.size;
        let mut f = self.view.draw(&mut self.r.w, &self.lang, w, h);
        if let Some(note) = &self.note
            && h > 0
        {
            f.text(0, h - 1, w, note, termplate_core::frame::Role::Normal);
        }
        f
    }

    pub fn resize(&mut self, w: usize, h: usize) {
        self.size = (w, h);
    }

    /// Runs the world to the player's turn or `rounds` rounds, keeping
    /// in-between frames when `presentation frame` is set (M 7.2).
    fn advance(&mut self, rounds: i64) -> RtResult<()> {
        let keep = !self.frame.is_zero();
        let (w, h) = self.size;
        let lang = self.lang.clone();
        let (view, frames, last) = (&mut self.view, &mut self.frames, &mut self.last_frame);
        self.status = self.r.run_with(rounds, &mut |world| {
            if !keep {
                return;
            }
            let f = view.draw(world, &lang, w, h);
            // only a moment where something visible changed is a frame
            if last.as_ref() != Some(&f) {
                *last = Some(f.clone());
                frames.push(f);
            }
        })?;
        Ok(())
    }

    /// A key at time `now` (M 8).
    pub fn key(&mut self, b: Binding, now: Duration) -> RtResult<()> {
        self.last_key = now;
        self.note = None;
        // ctrl+c always leaves, as in every terminal program
        if b == Binding::key(KeyCode::Char('c'), Mods { ctrl: true, ..Mods::default() }) {
            self.quit = true;
            return Ok(());
        }
        match self.input.press(self.m, &self.stack, b) {
            Effect::Command(cmd) => self.command(cmd)?,
            Effect::Engine(a) => match a.as_str() {
                // `pass` runs the next round when the player has no turn (M 6.1.2)
                "pass" if !self.players_turn() => {
                    self.last_round = now;
                    self.advance(1)?;
                }
                "pass" => {}
                "quit" | "exit" => self.quit = true,
                other => self.note = Some(format!("`{other}` is not supported yet")),
            },
            Effect::Open { menu, .. } => self.note = Some(format!("menu {menu}: menus come next")),
            Effect::Toggle(p) => self.note = Some(format!("panel {p}: panels are not supported yet")),
            Effect::Counting(n) => self.note = Some(format!("count: {n}")),
            Effect::Unbound(_) => {}
        }
        Ok(())
    }

    /// The player's command; a failed one takes no time (M 6.1.5).
    fn command(&mut self, cmd: CommandValue) -> RtResult<()> {
        if !self.players_turn() {
            return Ok(());
        }
        self.r.player(&cmd)?;
        self.advance(0)
    }

    /// The clock at `now`: rounds without the player's turn and `idle`.
    pub fn tick(&mut self, now: Duration) -> RtResult<()> {
        if !self.players_turn() {
            if let Some(round) = self.round
                && now.saturating_sub(self.last_round) >= round
            {
                self.last_round = now;
                self.advance(1)?;
            }
            return Ok(());
        }
        self.last_round = now;
        if let Some(idle) = self.idle.clone()
            && now.saturating_sub(self.last_key) >= idle.after
        {
            self.last_key = now;
            if let Some(cmd) = self.idle_command(&idle)? {
                self.command(cmd)?;
            }
        }
        Ok(())
    }

    /// `idle`: the player's ai's choice, or the named command (M 6.1.2).
    fn idle_command(&mut self, idle: &Idle) -> RtResult<Option<CommandValue>> {
        let Some(p) = self.r.w.s.player else { return Ok(None) };
        if idle.ai
            && let Some(Value::Name(ai)) = self.r.w.s.get(p).get("ai").and_then(|c| c.get("behavior")).cloned()
            && let Value::Command(cmd) = self.r.w.run_ai(&ai, p)?
        {
            return Ok(Some(cmd));
        }
        Ok(idle.command.clone().map(|name| CommandValue { name, args: vec![] }))
    }

    /// The time until the next tick matters: the round's pace or `idle`.
    pub fn wait(&self) -> Duration {
        let step = if self.players_turn() { self.idle.as_ref().map(|i| i.after) } else { self.round };
        step.map_or(Duration::from_millis(500), |s| s.min(Duration::from_millis(100)))
    }

    /// How long each in-between frame shows.
    pub fn frame_time(&self) -> Duration {
        self.frame
    }
}
