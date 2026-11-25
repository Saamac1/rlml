//! The game as the worker drives it: the terminal loop's steps without
//! its waiting, which the worker does with timers.
//!
//! 1. `next_frame` while it gives in-between frames, one each
//!    `frame_time` (M 7.2), then `draw`;
//! 2. events go in with `push` as they come, and wait for `step`;
//! 3. `step` when an event came or `wait` is over: the events, then a
//!    tick. Back to 1.

use std::collections::{BTreeSet, VecDeque};
use std::time::Duration;

use rlml_app::app::App;
use rlml_app::events::{Event, Feed};
use rlml_core::model::Model;
use rlml_core::rt::world::RtResult;
use termplate_core::frame::Frame;
use termplate_core::style::StyleSheet;

pub struct Game {
    m: &'static Model,
    seed: u64,
    pub app: App<'static>,
    feed: Feed,
    events: Vec<Event>,
    frames: VecDeque<Frame>,
    /// The frame the page shows now.
    shown: Option<Frame>,
    sheet: StyleSheet,
    /// The data folder, once the worker has read it (`store.rs`).
    journal: Option<crate::store::Journal>,
}

impl Game {
    /// A new game in a window of `size` cells. The model lives as long
    /// as the page.
    pub fn new(m: &'static Model, seed: u64, size: (usize, usize)) -> RtResult<Game> {
        let mut app = App::new(m, seed, size)?;
        app.view.truecolor = true;
        Ok(Game {
            m,
            seed,
            app,
            feed: Feed::default(),
            events: vec![],
            frames: VecDeque::new(),
            shown: None,
            sheet: StyleSheet::default(),
            journal: None,
        })
    }

    /// The game's name, which names its data folder (D 3.2).
    pub fn name(&self) -> String {
        self.m
            .named_game()
            .map(|g| g.name.clone())
            .unwrap_or_default()
    }

    /// The data folder's files as the worker read them: saves in `saves/`
    /// and the profile. The game being played starts over with the
    /// profile (M 5.1).
    pub fn storage(&mut self, files: Vec<(String, String)>) -> RtResult<()> {
        let j = crate::store::Journal::new(files);
        self.attach(&j, true)?;
        self.journal = Some(j);
        self.shown = None;
        Ok(())
    }

    fn attach(&mut self, j: &crate::store::Journal, restart: bool) -> RtResult<()> {
        self.app.set_store(Box::new(j.store("saves/")));
        self.app.set_profile_store(Box::new(j.store("")), restart)
    }

    /// What the game wrote to its data folder since the last call, for the
    /// worker to write to OPFS: `(path, Some(text))` or `(path, None)` to
    /// remove.
    pub fn store_changes(&mut self) -> Vec<(String, Option<String>)> {
        self.journal
            .as_ref()
            .map(|j| j.take_changes())
            .unwrap_or_default()
    }

    /// A game over: the browser cannot close its tab, so a new game
    /// starts in the same window.
    pub fn restart(&mut self) -> RtResult<()> {
        let size = self.app.size;
        let (pixels, releases) = (self.app.pixels, self.app.releases);
        let shown = self.shown.take();
        let journal = self.journal.take();
        *self = Game::new(self.m, self.seed, size)?;
        self.app.pixels = pixels;
        self.app.releases = releases;
        self.shown = shown;
        if let Some(j) = journal {
            // the saves and the profile stay with the page
            self.attach(&j, true)?;
            self.journal = Some(j);
        }
        Ok(())
    }

    pub fn push(&mut self, ev: Event) {
        self.events.push(ev);
    }

    /// The events since the last step, then a tick, at `now`.
    pub fn step(&mut self, now: Duration) -> RtResult<()> {
        self.input(now)?;
        self.tick(now)
    }

    /// A step's first half: the events since the last step.
    pub fn input(&mut self, now: Duration) -> RtResult<()> {
        let events = std::mem::take(&mut self.events);
        if self.feed.apply(&mut self.app, events, now)? {
            self.shown = None;
        }
        Ok(())
    }

    /// A step's second half: the world's time goes on (M 6.1.2).
    pub fn tick(&mut self, now: Duration) -> RtResult<()> {
        self.app.tick(now)
    }

    /// Events wait for the next step.
    pub fn has_events(&self) -> bool {
        !self.events.is_empty()
    }

    /// The next in-between frame's changed cells, if one is waiting.
    pub fn next_frame(&mut self) -> Option<Vec<u32>> {
        self.frames.extend(self.app.frames.drain(..));
        let f = self.frames.pop_front()?;
        Some(self.show(f))
    }

    /// The frame of now's changed cells.
    pub fn draw(&mut self) -> Vec<u32> {
        self.frames.extend(self.app.frames.drain(..));
        self.frames.clear();
        let f = self.app.draw();
        self.show(f)
    }

    fn show(&mut self, f: Frame) -> Vec<u32> {
        let out = crate::cells::encode(&self.sheet, self.shown.as_ref(), &f);
        self.shown = Some(f);
        out
    }

    pub fn quit(&self) -> bool {
        self.app.quit
    }

    /// The time until a step matters without an event.
    pub fn wait(&self) -> Duration {
        self.app.wait()
    }

    pub fn frame_time(&self) -> Duration {
        self.app.frame_time()
    }

    pub fn bound_keys(&self) -> BTreeSet<String> {
        crate::keys::bound(&self.app.input.keys)
    }

    /// The log's lines as the player reads them, oldest first.
    pub fn log_lines(&self) -> Vec<String> {
        let w = &self.app.r.w;
        w.log
            .iter()
            .map(|l| w.text(&self.app.lang, &l.key, &l.args))
            .collect()
    }
}
