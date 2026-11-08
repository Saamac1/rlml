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
//! * Menus stop the game's time while open (M 7.5, M 8). With a `title`
//!   menu the game starts there; `quit` returns to it, or without one to
//!   the engine's menu of a new game and leaving.

use std::collections::BTreeMap;
use std::time::Duration;

use rlml_core::model::Model;
use rlml_core::rt::run::{Runner, Status};
use rlml_core::rt::state::SeatId;
use rlml_core::rt::value::{CommandValue, Value};
use rlml_core::rt::world::{RtResult, World};
use termplate_core::frame::Frame;
use termplate_core::keys::{Binding, KeyCode, Mods};

use crate::draw::View;
use crate::input::{Action, Device, Effect, Input, PadPress, Press};
use crate::menu::{self, ItemAction, Item, Menu, Menus};

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

/// The engine's menu when the game has no `title` (M 8): a new game or
/// leaving.
pub const ENGINE_MENU: &str = "ENGINE";

fn engine_menu() -> Menu {
    let item = |key: &str, action: &str| Item { key: key.into(), action: ItemAction::Engine(action.into()), hide: None, disabled: None };
    Menu {
        full: ENGINE_MENU.into(),
        ns: None,
        context: "MENU".into(),
        is_final: true,
        width: None,
        height: None,
        items: vec![item("NEW_GAME", "new_game"), item("EXIT", "exit")],
    }
}

pub struct App<'m> {
    pub m: &'m Model,
    pub r: Runner<'m>,
    pub view: View,
    pub input: Input,
    /// The game's key context stack under the menus, bottom first (M 8).
    pub stack: Vec<String>,
    pub menus: Menus,
    /// False before the first game and after `quit`: only menus show.
    pub playing: bool,
    /// The seed of the game being played; a new game takes the next.
    pub seed: u64,
    /// Games started.
    pub games: u32,
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
    /// The time of the last `tick`, and the member whose turn started
    /// when, for the turn marker's blink (M 7.1).
    clock: Duration,
    turn_start: Option<(rlml_core::rt::value::EntityId, Duration)>,
    /// Pads by connection id, in the order they connected (M 8.2).
    pub pads: Vec<usize>,
    /// Each seat's device. Until the join screen exists, seat 1 takes
    /// keyboard part 1, the next seats the pads in connection order and
    /// then the other keyboard parts; a seat without a device is played
    /// from keyboard part 1 (M 8.2).
    pub seat_devices: BTreeMap<SeatId, Device>,
    /// The device that opened the menus on top, which drives them (M 8.2).
    menu_owner: Option<Device>,
    /// `settings - mouse on|off` (D 3.2), on by default.
    pub mouse: bool,
    /// The pointer's screen cell, while the mouse is over the window.
    pub pointer: Option<(usize, usize)>,
    /// When the last travel step ran, for the travel's pace.
    last_step: Duration,
}

/// The least time between two travel steps, so a travel shows even
/// without `presentation frame` (M 8.1).
const STEP_PACE: Duration = Duration::from_millis(80);

/// The turn marker flashes on for this long when a turn starts, then
/// blinks with this period, half on and half off.
const MARK_FLASH: Duration = Duration::from_millis(600);
const MARK_BLINK: Duration = Duration::from_millis(800);

impl<'m> App<'m> {
    /// A new game with `seed`, or the game's `title` menu when it has one.
    pub fn new(m: &'m Model, seed: u64, size: (usize, usize)) -> RtResult<App<'m>> {
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
            r: Runner::new(World::new(m, seed)),
            view: View::default(),
            input,
            stack,
            menus: Menus::default(),
            playing: false,
            seed,
            games: 0,
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
            clock: Duration::ZERO,
            turn_start: None,
            pads: vec![],
            seat_devices: BTreeMap::new(),
            menu_owner: None,
            mouse: setting("mouse").as_deref() != Some("off"),
            pointer: None,
            last_step: Duration::ZERO,
        };
        match app.title() {
            Some(title) => {
                app.menus.open(&mut app.r.w, &title, false);
            }
            None => app.start(seed)?,
        }
        Ok(app)
    }

    /// The game's own start menu (`title`, D 3.3).
    fn title(&self) -> Option<String> {
        let g = self.m.named_game()?;
        let item = g.items.iter().find(|i| i.key() == "title")?;
        self.m.resolve_kind(g.namespace.as_deref(), item.word(1)?, &[rlml_core::model::Kind::Menu]).ok().map(|d| d.full.clone())
    }

    /// Starts a new game with `seed` and runs it to the player's first turn
    /// (M 5.1); the menus close.
    fn start(&mut self, seed: u64) -> RtResult<()> {
        let mut w = World::new_game(self.m, seed)?;
        w.begin()?;
        self.r = Runner::new(w);
        self.seed = seed;
        self.view = View::default();
        self.menus.clear();
        self.frames.clear();
        self.last_frame = None;
        self.status = Status::NoPlayer { rounds: 0 };
        self.playing = true;
        self.games += 1;
        self.assign_devices();
        self.advance(0)
    }

    /// Gives the seats their devices: keyboard part 1, then the pads in
    /// connection order, then the other keyboard parts (M 8.2). A pad that
    /// connects takes a seat from a keyboard part, since it is plugged in
    /// to play.
    fn assign_devices(&mut self) {
        let parts = self.input.keys.keyboard_parts();
        let devices = [Device::Keyboard(1)].into_iter().chain(self.pads.iter().map(|&p| Device::Pad(p))).chain(parts.into_iter().filter(|&p| p != 1).map(Device::Keyboard));
        self.seat_devices = self.r.w.s.seats.iter().map(|s| s.n).zip(devices).collect();
    }

    /// A pad connected or came back (M 8.2): the seats take their devices
    /// again, so the pad plays the first seat after seat 1 that no pad has.
    pub fn pad_connected(&mut self, id: usize) {
        if !self.pads.contains(&id) {
            self.pads.push(id);
            self.assign_devices();
        }
    }

    pub fn pad_disconnected(&mut self, id: usize) {
        self.pads.retain(|&p| p != id);
        self.assign_devices();
    }

    /// Does `device` play seat `n`? Its own device does, and keyboard part
    /// 1 plays a seat that has none.
    pub fn drives(&self, device: Device, n: SeatId) -> bool {
        match self.seat_devices.get(&n) {
            Some(d) => *d == device,
            None => device == Device::Keyboard(1),
        }
    }

    /// `quit`: back to the `title` menu, or to the engine's (M 8).
    fn quit_game(&mut self) {
        self.playing = false;
        self.menus.clear();
        match self.title() {
            Some(title) => {
                self.menus.open(&mut self.r.w, &title, false);
            }
            None => {
                let layer = menu::Layer::open(&mut self.r.w, engine_menu());
                self.menus.layers.push(layer);
            }
        }
    }

    /// The next game's seed: a new one each time (M 5.1).
    fn next_seed(&self) -> u64 {
        let mut s = self.seed;
        rlml_core::rt::rng::splitmix64(&mut s)
    }

    /// The key contexts now: the game's, then the open menus' (M 8).
    pub fn contexts(&self) -> Vec<String> {
        self.stack.iter().cloned().chain(self.menus.contexts().map(String::from)).collect()
    }

    /// The player's turn: the scheduler waits for input, and the player is
    /// alive (it may have died since).
    pub fn players_turn(&self) -> bool {
        self.playing && self.status == Status::Input && self.r.w.s.player.is_some_and(|p| self.r.w.s.get(p).alive)
    }

    /// The frame for the window now.
    /// The member to mark now: with more than one member, the one whose
    /// input is awaited, flashing when its turn starts and then blinking
    /// (M 7.1).
    fn turn_marker(&mut self) -> Option<rlml_core::rt::value::EntityId> {
        let p = self.r.w.s.player.filter(|_| self.players_turn() && self.r.w.s.players().len() > 1);
        let Some(p) = p else {
            self.turn_start = None;
            return None;
        };
        let start = match self.turn_start {
            Some((e, t)) if e == p => t,
            _ => {
                self.turn_start = Some((p, self.clock));
                self.clock
            }
        };
        let since = self.clock.saturating_sub(start);
        let on = since < MARK_FLASH || (since - MARK_FLASH).as_millis() % MARK_BLINK.as_millis() < MARK_BLINK.as_millis() / 2;
        on.then_some(p)
    }

    pub fn draw(&mut self) -> Frame {
        self.view.marked = self.turn_marker();
        self.view.hover = self.pointer_cell().and_then(|p| self.r.w.hover_text(&self.lang, p));
        let (w, h) = self.size;
        let mut f = if self.playing { self.view.draw(&mut self.r.w, &self.lang, w, h) } else { Frame::new(w, h) };
        if let Some(layer) = self.menus.layers.last() {
            menu::draw(&mut f, &self.r.w, &self.lang, layer);
        }
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
        self.clock = now;
        self.note = None;
        // ctrl+c always leaves, as in every terminal program
        if b == Binding::key(KeyCode::Char('c'), Mods { ctrl: true, ..Mods::default() }) {
            self.quit = true;
            return Ok(());
        }
        let device = Device::Keyboard(self.input.keys.part_of(&b));
        self.press(device, Press::Key(b), now)
    }

    /// A pad press at time `now` (M 8.2).
    pub fn pad(&mut self, id: usize, p: PadPress, now: Duration) -> RtResult<()> {
        self.last_key = now;
        self.clock = now;
        self.note = None;
        self.pad_connected(id);
        self.press(Device::Pad(id), Press::Pad(p), now)
    }

    /// A press from a device, routed by M 8.2: `ALWAYS` from every device,
    /// a menu from the device that opened it (keyboard parts count as one
    /// there), the game's context on a turn from the awaited seat's device
    /// only; other presses are dropped.
    fn press(&mut self, device: Device, press: Press, now: Duration) -> RtResult<()> {
        let always = self.input.keys.is_always(device, &press);
        if !always {
            let same = |a: Device, b: Device| matches!((a, b), (Device::Keyboard(_), Device::Keyboard(_))) || a == b;
            if self.menus.is_open() {
                if self.menu_owner.is_some_and(|o| !same(o, device)) {
                    return Ok(());
                }
            } else if self.players_turn()
                && let Some(n) = self.r.awaiting()
                && !self.drives(device, n)
            {
                return Ok(());
            }
        }
        // any press of the travelling seat stops its travel and does
        // nothing else (M 8.1)
        if !always
            && !self.menus.is_open()
            && self.players_turn()
            && let Some(n) = self.r.awaiting()
            && self.r.travelling(n)
        {
            self.r.stop_travel(n);
            return Ok(());
        }
        let contexts = self.contexts();
        match self.input.press_on(self.m, &contexts, device, press) {
            Effect::Command(cmd) => self.command(cmd)?,
            Effect::Engine(a) => self.engine(&a, now)?,
            Effect::Open { menu, stack, always } => {
                // the `ALWAYS` key of an open menu closes it, so pause is a
                // toggle; it does not close a `final` menu (M 8)
                match self.menus.position(&menu) {
                    Some(i) if always && !self.menus.layers[i].menu.is_final => self.menus.layers.truncate(i),
                    _ => {
                        if !self.menus.is_open() {
                            self.menu_owner = Some(device);
                        }
                        self.open(&menu, stack)
                    }
                }
            }
            Effect::Toggle(p) => self.note = Some(format!("panel {p}: panels are not supported yet")),
            Effect::Counting(n) => self.note = Some(format!("count: {n}")),
            Effect::Unbound => {}
        }
        Ok(())
    }

    fn open(&mut self, menu: &str, stack: bool) {
        if !self.menus.open(&mut self.r.w, menu, stack) {
            self.note = Some(format!("{menu}: list menus and panels are not supported yet"));
        }
    }

    /// An engine action from a key or a menu item (M 8, M 9.1).
    fn engine(&mut self, a: &str, now: Duration) -> RtResult<()> {
        if let Some(layer) = self.menus.top() {
            match a {
                "cursor.north" => {
                    layer.move_by(-1);
                    return Ok(());
                }
                "cursor.south" => {
                    layer.move_by(1);
                    return Ok(());
                }
                "confirm" => return self.choose(now),
                _ => {}
            }
        }
        match a {
            // `pass` runs the next round when the player has no turn (M 6.1.2)
            "pass" if self.playing && !self.menus.is_open() && !self.players_turn() => {
                self.last_round = now;
                self.advance(1)?;
            }
            "pass" => {}
            "back" => self.menus.back(),
            "close" => self.menus.close(),
            "resume" => self.menus.close(),
            "new_game" => {
                // the first game takes the seed given, later ones new seeds
                let seed = if self.games > 0 { self.next_seed() } else { self.seed };
                self.start(seed)?;
                self.last_round = now;
            }
            "travel_to" if self.players_turn() => {
                if let (Some(n), Some(p)) = (self.r.awaiting(), self.pointer_cell())
                    && self.r.travel_to(n, p)?
                {
                    self.travel_step(now)?;
                }
            }
            "follow" if self.players_turn() => {
                if let Some(n) = self.r.awaiting() {
                    if self.r.follow(n, None)? {
                        self.travel_step(now)?;
                    } else {
                        self.note = Some("follow: nobody to follow".into());
                    }
                }
            }
            "travel_to" | "follow" => {}
            "quit" if self.playing => self.quit_game(),
            "quit" | "exit" => self.quit = true,
            other => self.note = Some(format!("`{other}` is not supported yet")),
        }
        Ok(())
    }

    /// `confirm` in a menu: the item under the cursor (M 7.5).
    fn choose(&mut self, now: Duration) -> RtResult<()> {
        let Some(item) = self.menus.top().and_then(|l| l.chosen().cloned()) else { return Ok(()) };
        match item.action {
            ItemAction::Open { menu, stack } => self.open(&menu, stack),
            ItemAction::Engine(a) => self.engine(&a, now)?,
            // a command from a menu is the player's command with its cost;
            // the menus close for it
            ItemAction::Command(c) => {
                self.menus.close();
                if !self.menus.is_open() {
                    self.command(CommandValue { name: c, args: vec![] })?;
                }
            }
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
        self.clock = now;
        // the game's time stops while a menu is open (M 8)
        if !self.playing || self.menus.is_open() {
            self.last_round = now;
            self.last_key = now;
            return Ok(());
        }
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
        // a travel steps at its pace; `idle` waits meanwhile (M 8.1)
        if let Some(n) = self.r.awaiting()
            && self.r.travelling(n)
        {
            self.last_key = now;
            if self.frames.is_empty() && now.saturating_sub(self.last_step) >= self.frame.max(STEP_PACE) {
                self.travel_step(now)?;
            }
            return Ok(());
        }
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
        if !self.playing || self.menus.is_open() {
            return Duration::from_millis(500);
        }
        if self.players_turn() && self.r.awaiting().is_some_and(|n| self.r.travelling(n)) {
            return self.frame.max(STEP_PACE);
        }
        let step = if self.players_turn() { self.idle.as_ref().map(|i| i.after) } else { self.round };
        // the turn marker blinks, so a group's turn redraws often enough
        let step = if self.players_turn() && self.r.w.s.players().len() > 1 { Some(step.map_or(MARK_BLINK / 4, |s| s.min(MARK_BLINK / 4))) } else { step };
        step.map_or(Duration::from_millis(500), |s| s.min(Duration::from_millis(100)))
    }

    /// The map cell under the pointer, on the active map.
    pub fn pointer_cell(&self) -> Option<rlml_core::rt::value::Pos> {
        let (x, y) = self.pointer?;
        let map = self.r.w.s.active_map()?;
        crate::draw::cell_at(&self.view, map, x, y)
    }

    /// The pointer moved to screen cell (x, y): the hover follows it.
    pub fn pointer_at(&mut self, x: usize, y: usize) {
        if self.mouse {
            self.pointer = Some((x, y));
        }
    }

    /// A mouse press at screen cell (x, y) and time `now` (M 8.1, M 8.2).
    /// The mouse is shared and follows the turn: on a turn its press is the
    /// awaited seat's, read from keyboard part 1's rows, since the mouse
    /// has no part of its own. Menus take the mouse later.
    pub fn mouse_press(&mut self, b: Binding, x: usize, y: usize, now: Duration) -> RtResult<()> {
        if !self.mouse {
            return Ok(());
        }
        self.pointer = Some((x, y));
        self.last_key = now;
        self.clock = now;
        self.note = None;
        let contexts = self.contexts();
        let press = Press::Key(b);
        let always = self.input.keys.is_always(Device::Keyboard(1), &press);
        if !always && (self.menus.is_open() || !self.players_turn()) {
            return Ok(());
        }
        if !always
            && let Some(n) = self.r.awaiting()
            && self.r.travelling(n)
        {
            self.r.stop_travel(n);
            return Ok(());
        }
        match self.input.keys.lookup_on(&contexts, Device::Keyboard(1), &press).cloned() {
            Some(Action::Command(c)) => self.command(CommandValue { name: c, args: vec![] })?,
            Some(Action::Engine(a)) => self.engine(&a, now)?,
            Some(Action::Open { menu, stack }) => {
                if !self.menus.is_open() {
                    self.menu_owner = self.r.awaiting().and_then(|n| self.seat_devices.get(&n).copied()).or(Some(Device::Keyboard(1)));
                }
                self.open(&menu, stack)
            }
            Some(Action::Toggle(p)) => self.note = Some(format!("panel {p}: panels are not supported yet")),
            None => {}
        }
        Ok(())
    }

    /// One step of the awaited seat's travel, then the world to the next
    /// turn (M 8.1).
    fn travel_step(&mut self, now: Duration) -> RtResult<()> {
        self.last_step = now;
        self.r.step_travel()?;
        self.advance(0)
    }

    /// How long each in-between frame shows.
    pub fn frame_time(&self) -> Duration {
        self.frame
    }
}
