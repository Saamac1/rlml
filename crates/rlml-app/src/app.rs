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
use rlml_core::rt::travel::OrderGoal;
use rlml_core::rt::verbs::{Ask, Target};
use rlml_core::rt::value::{CommandValue, Value};
use rlml_core::rt::world::{RtResult, World};
use termplate_core::frame::Frame;
use termplate_core::keys::{Binding, KeyCode, Mods};

use crate::cursor::{self, Cursor};
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

/// The menus docked in the layout: its `- v menu NAME` rows (D 11.1).
fn docked_menus(m: &Model) -> Vec<Menu> {
    let Some(layout) = m.layout() else { return vec![] };
    layout
        .sub("legend")
        .filter_map(|item| {
            let words: Vec<&str> = item.tokens.iter().map(|t| t.text.as_str()).collect();
            let i = words.iter().position(|w| *w == "menu")?;
            let full = m.resolve(item.namespace.as_deref(), words.get(i + 1)?).ok()?.full.clone();
            Menu::read(m, &full)
        })
        .collect()
}

fn engine_menu() -> Menu {
    let item = |key: &str, action: &str| Item { key: key.into(), action: ItemAction::Engine(action.into()), hide: None, disabled: None, label: None };
    Menu {
        full: ENGINE_MENU.into(),
        ns: None,
        context: "MENU".into(),
        is_final: true,
        width: None,
        height: None,
        items: vec![item("NEW_GAME", "new_game"), item("EXIT", "exit")],
        verbs: false,
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
    /// The seats' look cursors (M 8.1), and the seat that moved one last.
    pub cursors: BTreeMap<SeatId, Cursor>,
    last_cursor: Option<SeatId>,
    /// The menus docked in the layout (D 11.1), in legend order.
    pub docked: Vec<Menu>,
    /// The chosen verb and what clicks have filled (M 8.1).
    pub verb: Option<Verb>,
    /// The docked menu that has the keys, `focus MENU` (D 11.3).
    pub focus: Option<Focus>,
    /// Why the last click fitted no parameter, for the hover until the
    /// pointer moves.
    verb_hint: Option<String>,
    /// The open verb coin's verbs (M 8.1), and where the top menu was drawn.
    coin: Option<Coin>,
    menu_drawn: crate::menu::Drawn,
    /// The player's command whose parameters are being asked (M 8).
    pub asking: Option<Asking>,
    /// `settings - targeting cursor|list|auto` (D 3.2), `cursor` by default.
    targeting: String,
}

/// The asked parameters' list menu as a layer.
pub const ASK_MENU: &str = "<ask>";

/// A player's command being asked for its parameters (M 8): presentation,
/// not state; the command runs when all are given.
#[derive(Debug, Clone, PartialEq)]
pub struct Asking {
    pub seat: SeatId,
    pub command: String,
    pub args: Vec<(Option<String>, Value)>,
    pub ui: AskUi,
}

/// How the parameter is being asked.
#[derive(Debug, Clone, PartialEq)]
pub enum AskUi {
    None,
    /// A list menu (`from`, or `aim` with `targeting list`): the choices.
    List { name: String, choices: Vec<Value> },
    /// The aiming cursor, context `AIM`.
    Aim { name: String, pos: rlml_core::rt::value::Pos, range: i64, candidates: Vec<rlml_core::rt::value::Pos> },
    /// A direction, context `DIRECTION`.
    Direction { name: String, pos: bool },
}

/// The verb coin's name as a menu layer.
pub const COIN_MENU: &str = "<verbs>";

/// An open verb coin (M 8.1): its rows' commands, each with the
/// parameter the cell fills.
#[derive(Debug, Clone, PartialEq)]
pub struct Coin {
    pub seat: SeatId,
    pub entries: Vec<(String, (String, Value))>,
    /// Opened from the look cursor: the verb's other targets come from it.
    pub keys: bool,
}

/// A verb chosen from a docked menu (M 8.1): presentation, not state.
#[derive(Debug, Clone, PartialEq)]
pub struct Verb {
    pub seat: SeatId,
    pub command: String,
    pub menu: String,
    pub item: usize,
    pub filled: Vec<(String, Value)>,
}

/// A docked menu with the keys of a seat (D 11.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Focus {
    pub seat: SeatId,
    pub menu: String,
    pub row: usize,
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
            cursors: BTreeMap::new(),
            last_cursor: None,
            docked: docked_menus(m),
            verb: None,
            focus: None,
            verb_hint: None,
            coin: None,
            menu_drawn: Default::default(),
            asking: None,
            targeting: setting("targeting").unwrap_or_else(|| "cursor".into()),
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
        self.view = View { truecolor: self.view.truecolor, ..View::default() };
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
        self.expire_verb();
        self.view.hover = match &self.verb_hint {
            Some(h) => Some(h.clone()),
            None => self.hover_cell().and_then(|p| self.r.w.hover_text(&self.lang, p)),
        };
        self.view.cursors = self.cursor_marks();
        self.view.docked = self.docked_view();
        self.view.sentence = self.sentence();
        let (w, h) = self.size;
        let mut f = if self.playing { self.view.draw(&mut self.r.w, &self.lang, w, h) } else { Frame::new(w, h) };
        self.menu_drawn = match self.menus.layers.last() {
            Some(layer) => menu::draw(&mut f, &self.r.w, &self.lang, layer),
            None => Default::default(),
        };
        if self.coin.is_some() && self.menus.position(COIN_MENU).is_none() {
            self.coin = None;
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
        // a seat's own cursor goes before the turn's routing: it works on
        // other seats' turns too (M 8.2)
        let travelling_seat = self.r.awaiting().filter(|&n| self.players_turn() && self.r.travelling(n) && self.drives(device, n));
        if !always && !self.menus.is_open() && self.ask_press(device, &press)? {
            return Ok(());
        }
        // a list's letter chooses its row (M 7.5)
        if !always
            && self.menus.layers.last().is_some_and(|l| l.menu.full == ASK_MENU)
            // in a menu the keyboard's parts count as one (M 8.2)
            && self.asking.as_ref().is_some_and(|a| self.drives(device, a.seat) || matches!((device, self.menu_owner), (Device::Keyboard(_), Some(Device::Keyboard(_)))))
            && let Press::Key(Binding::Key(KeyCode::Char(c), _)) = &press
            && let Some(AskUi::List { choices, .. }) = self.asking.as_ref().map(|a| &a.ui)
        {
            let letters = self.letters(choices);
            if let Some(i) = letters.iter().position(|l| l == c) {
                if let Some(top) = self.menus.top()
                    && let Some(r) = top.rows.iter().position(|r| r.item == i)
                {
                    top.cursor = r;
                }
                return self.choose(now);
            }
        }
        if !always && travelling_seat.is_none() && !self.menus.is_open() && self.focus_press(device, &press, now)? {
            return Ok(());
        }
        if !always && !self.menus.is_open() && self.playing && travelling_seat.is_none() && self.cursor_press(device, &press, now)? {
            return Ok(());
        }
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
            && self.drives(device, n)
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
            Effect::Focus(menu) => {
                if self.players_turn()
                    && let Some(n) = self.r.awaiting()
                    && self.docked.iter().any(|d| d.full == menu)
                {
                    self.focus = Some(Focus { seat: n, menu, row: 0 });
                }
            }
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
            "back" if self.menus.layers.last().is_some_and(|l| l.menu.full == ASK_MENU) => self.cancel_ask(),
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
            "look" if self.players_turn() => {
                if let Some(n) = self.r.awaiting() {
                    self.open_cursor(n, false);
                }
            }
            "verbs" if self.players_turn() => {
                if let (Some(n), Some(p)) = (self.r.awaiting(), self.pointer_cell()) {
                    self.open_coin(n, p, false)?;
                }
            }
            "travel_to" | "follow" | "look" | "verbs" => {}
            "quit" if self.playing => self.quit_game(),
            "quit" | "exit" => self.quit = true,
            other => self.note = Some(format!("`{other}` is not supported yet")),
        }
        Ok(())
    }

    /// `confirm` in a menu: the item under the cursor (M 7.5).
    fn choose(&mut self, now: Duration) -> RtResult<()> {
        if self.menus.layers.last().is_some_and(|l| l.menu.full == COIN_MENU) {
            return self.choose_coin(now);
        }
        if self.menus.layers.last().is_some_and(|l| l.menu.full == ASK_MENU) {
            let i = self.menus.layers.last().and_then(|l| l.rows.get(l.cursor)).map(|r| r.item);
            self.menus.layers.pop();
            let choice = match (&self.asking, i) {
                (Some(Asking { ui: AskUi::List { choices, .. }, .. }), Some(i)) => choices.get(i).cloned(),
                _ => None,
            };
            return match choice {
                Some(v) => self.answer(v),
                None => {
                    self.cancel_ask();
                    Ok(())
                }
            };
        }
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
        let Some(n) = self.r.awaiting() else { return Ok(()) };
        self.asking = Some(Asking { seat: n, command: cmd.name, args: cmd.args, ui: AskUi::None });
        self.ask_next()
    }

    /// Asks the command's next parameter, or runs it when all are given
    /// (M 8): `from` in a list menu, `aim` by `targeting`, `ask` as a
    /// direction. A command that fails before asking takes no time.
    fn ask_next(&mut self) -> RtResult<()> {
        loop {
            let (Some(a), Some(actor)) = (self.asking.clone(), self.r.w.s.player) else { return Ok(()) };
            if !self.players_turn() || self.r.awaiting() != Some(a.seat) {
                self.asking = None;
                return Ok(());
            }
            match self.r.w.next_ask(actor, &a.command, &a.args)? {
                Ask::Ready => {
                    self.asking = None;
                    self.r.player(&CommandValue { name: a.command, args: a.args })?;
                    return self.advance(0);
                }
                Ask::Fails => {
                    self.asking = None;
                    return Ok(());
                }
                Ask::Given(name, v) => self.asking.as_mut().unwrap().args.push((Some(name), v)),
                Ask::From { name, members } => {
                    let choices = members.into_iter().map(Value::Entity).collect();
                    return self.ask_list(name, choices);
                }
                Ask::Aim { name, range, candidates } => {
                    if self.targeting == "auto" && !candidates.is_empty() {
                        self.asking.as_mut().unwrap().args.push((Some(name), Value::Pos(candidates[0])));
                        continue;
                    }
                    if self.targeting == "list" && !candidates.is_empty() {
                        return self.ask_list(name, candidates.into_iter().map(Value::Pos).collect());
                    }
                    let pos = candidates.first().copied().or_else(|| self.r.w.s.at(actor)).unwrap();
                    self.asking.as_mut().unwrap().ui = AskUi::Aim { name, pos, range, candidates };
                    return Ok(());
                }
                Ask::Direction { name, pos } => {
                    self.asking.as_mut().unwrap().ui = AskUi::Direction { name, pos };
                    return Ok(());
                }
                Ask::Field { .. } => {
                    self.asking = None;
                    self.note = Some("number and text fields are not supported yet".into());
                    return Ok(());
                }
            }
        }
    }

    /// The asked parameter's list menu (M 7.5): entities by name and their
    /// letters, cells by what is seen there.
    fn ask_list(&mut self, name: String, choices: Vec<Value>) -> RtResult<()> {
        let letters = self.letters(&choices);
        let items = choices
            .iter()
            .zip(&letters)
            .enumerate()
            .map(|(i, (v, l))| {
                let text = match v {
                    Value::Pos(p) => self.r.w.hover_text(&self.lang, *p).unwrap_or_else(|| format!("{},{}", p.x, p.y)),
                    v => self.r.w.name_text(&self.lang, v),
                };
                Item { key: i.to_string(), action: ItemAction::Engine("confirm".into()), hide: None, disabled: None, label: Some(format!("{l}) {text}")) }
            })
            .collect();
        let menu = Menu { full: ASK_MENU.into(), ns: None, context: "MENU".into(), is_final: false, width: None, height: None, items, verbs: false };
        let layer = menu::Layer::open(&mut self.r.w, menu);
        self.menus.layers.push(layer);
        let seat = self.asking.as_ref().map(|a| a.seat).unwrap_or(1);
        self.menu_owner = Some(self.seat_devices.get(&seat).copied().unwrap_or(Device::Keyboard(1)));
        if let Some(a) = self.asking.as_mut() {
            a.ui = AskUi::List { name, choices };
        }
        Ok(())
    }

    /// A list's letters (M 7.5): the members' slot letters when all are in
    /// the same collection, else a–z as shown.
    fn letters(&self, choices: &[Value]) -> Vec<char> {
        use rlml_core::rt::state::Place;
        let places: Vec<Option<(usize, String, String, usize)>> = choices
            .iter()
            .map(|v| match v.as_entity().map(|e| &self.r.w.s.get(e).place) {
                Some(Place::In { holder, component, field, slot }) => Some((*holder, component.clone(), field.clone(), *slot)),
                _ => None,
            })
            .collect();
        let same = places.iter().all(|p| p.is_some()) && places.windows(2).all(|w| w[0].as_ref().map(|p| (&p.0, &p.1, &p.2)) == w[1].as_ref().map(|p| (&p.0, &p.1, &p.2)));
        if same && !places.is_empty() {
            return places.iter().map(|p| (b'a' + (p.as_ref().unwrap().3 % 26) as u8) as char).collect();
        }
        (0..choices.len()).map(|i| (b'a' + (i % 26) as u8) as char).collect()
    }

    /// The asked parameter's answer: the next one is asked.
    fn answer(&mut self, v: Value) -> RtResult<()> {
        let Some(a) = self.asking.as_mut() else { return Ok(()) };
        let name = match std::mem::replace(&mut a.ui, AskUi::None) {
            AskUi::List { name, .. } | AskUi::Aim { name, .. } | AskUi::Direction { name, .. } => name,
            AskUi::None => return Ok(()),
        };
        a.args.push((Some(name), v));
        self.ask_next()
    }

    /// `back` while asking: the command goes without taking time (M 8).
    fn cancel_ask(&mut self) {
        self.asking = None;
        if self.menus.layers.last().is_some_and(|l| l.menu.full == ASK_MENU) {
            self.menus.layers.pop();
        }
    }

    /// A press while aiming or choosing a direction (M 8): the `AIM` or
    /// `DIRECTION` context of the asking seat's device takes it.
    fn ask_press(&mut self, device: Device, press: &Press) -> RtResult<bool> {
        let Some(a) = self.asking.clone() else { return Ok(false) };
        if !self.drives(device, a.seat) {
            return Ok(false);
        }
        let ctx = match a.ui {
            AskUi::Aim { .. } => "AIM",
            AskUi::Direction { .. } => "DIRECTION",
            _ => return Ok(false),
        };
        let Some(Action::Engine(act)) = self.input.keys.lookup_on(&[ctx.to_string()], device, press).cloned() else { return Ok(true) };
        self.ask_action(&act)?;
        Ok(true)
    }

    fn ask_action(&mut self, act: &str) -> RtResult<()> {
        let Some(a) = self.asking.clone() else { return Ok(()) };
        let Some(at) = self.r.w.s.player.and_then(|p| self.r.w.s.at(p)) else { return Ok(()) };
        match (&a.ui, act) {
            (_, "back") => self.cancel_ask(),
            (AskUi::Aim { pos, range, candidates, name }, _) => match act {
                "confirm" => self.answer(Value::Pos(*pos))?,
                "next" | "prev" if !candidates.is_empty() => {
                    let i = candidates.iter().position(|c| c == pos);
                    let k = match (act, i) {
                        ("next", Some(i)) => (i + 1) % candidates.len(),
                        ("prev", Some(i)) => (i + candidates.len() - 1) % candidates.len(),
                        ("prev", None) => candidates.len() - 1,
                        _ => 0,
                    };
                    self.asking.as_mut().unwrap().ui = AskUi::Aim { name: name.clone(), pos: candidates[k], range: *range, candidates: candidates.clone() };
                }
                _ => {
                    if let Some(d) = cursor::dir_of(act) {
                        let p = cursor::moved(&self.r.w, *pos, d);
                        // the aiming cursor stays in range (M 8)
                        if at.distance(p) <= *range {
                            self.asking.as_mut().unwrap().ui = AskUi::Aim { name: name.clone(), pos: p, range: *range, candidates: candidates.clone() };
                        }
                    }
                }
            },
            (AskUi::Direction { pos, .. }, _) => {
                if let Some(d) = cursor::dir_of(act) {
                    let pos = *pos;
                    self.answer(if pos { Value::Pos(at.step(d)) } else { Value::Dir(d) })?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// A mouse press while asking (M 8.1): a click aims at its cell or
    /// gives the direction towards it; `rclick` goes back.
    fn ask_mouse(&mut self, b: &Binding) -> RtResult<bool> {
        let Some(a) = self.asking.clone() else { return Ok(false) };
        if !matches!(a.ui, AskUi::Aim { .. } | AskUi::Direction { .. }) {
            return Ok(false);
        }
        if *b == Binding::mouse(termplate_core::keys::Mouse::RightClick) {
            self.cancel_ask();
            return Ok(true);
        }
        if *b != Binding::mouse(termplate_core::keys::Mouse::Click) {
            return Ok(true);
        }
        let (Some(cell), Some(at)) = (self.pointer_cell(), self.r.w.s.player.and_then(|p| self.r.w.s.at(p))) else { return Ok(true) };
        match a.ui {
            AskUi::Aim { range, .. } if at.distance(cell) <= range => self.answer(Value::Pos(cell))?,
            AskUi::Direction { pos, .. } => {
                if let Some(d) = rlml_core::rt::verbs::toward(at, cell) {
                    self.answer(if pos { Value::Pos(at.step(d)) } else { Value::Dir(d) })?;
                }
            }
            _ => {}
        }
        Ok(true)
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
        if !self.mouse {
            return;
        }
        if self.pointer != Some((x, y)) {
            self.verb_hint = None;
        }
        self.pointer = Some((x, y));
        // the pointer highlights a menu's row (M 8.1)
        if self.menus.is_open() {
            if let Some(i) = self.menu_drawn.row_at(x, y)
                && let Some(top) = self.menus.top()
                && top.rows.get(i).is_some_and(|r| r.enabled)
            {
                top.cursor = i;
            }
            return;
        }
        // the pointer aims, in range (M 8.1)
        if let (Some(cell), Some(at)) = (self.pointer_cell(), self.r.w.s.player.and_then(|p| self.r.w.s.at(p)))
            && let Some(Asking { ui: AskUi::Aim { pos, range, .. }, .. }) = self.asking.as_mut()
            && at.distance(cell) <= *range
        {
            *pos = cell;
            return;
        }
        // with the awaited seat's cursor open, the pointer is that cursor (M 8.1)
        if let (Some(n), Some(p)) = (self.r.awaiting().filter(|_| self.players_turn()), self.pointer_cell())
            && let Some(c) = self.cursors.get_mut(&n)
        {
            c.pos = p;
            self.last_cursor = Some(n);
        }
    }

    /// The seat a device plays now: the awaited seat if it does, else the
    /// first seat it plays (M 8.2).
    pub fn seat_of(&self, device: Device) -> Option<SeatId> {
        if let Some(n) = self.r.awaiting().filter(|&n| self.drives(device, n)) {
            return Some(n);
        }
        self.r.w.s.seats.iter().map(|s| s.n).find(|&n| self.drives(device, n))
    }

    /// The member a seat's cursor starts at: its controlled one, else the
    /// camera's middle.
    fn cursor_start(&self, n: SeatId) -> Option<rlml_core::rt::value::Pos> {
        let s = &self.r.w.s;
        if let Some(p) = s.active(n).and_then(|e| s.at(e)) {
            return Some(p);
        }
        let map = s.active_map()?;
        let (cx, cy) = self.view.camera.get(&map).copied().unwrap_or((0, 0));
        let r = self.view.map_rect?;
        Some(rlml_core::rt::value::Pos { map, x: cx + r.w as i32 / 2, y: cy + r.h as i32 / 2 })
    }

    fn open_cursor(&mut self, n: SeatId, stays: bool) {
        if let Some(pos) = self.cursor_start(n) {
            self.cursors.insert(n, Cursor { pos, stays });
            self.last_cursor = Some(n);
        }
    }

    /// A press for a seat's cursor (M 8.1, M 8.2): `cursor` opens or closes
    /// the seat's staying cursor at any time, and with a cursor open the
    /// `LOOK` context takes the seat's presses, also on other seats' turns.
    /// Returns whether the press was the cursor's.
    fn cursor_press(&mut self, device: Device, press: &Press, now: Duration) -> RtResult<bool> {
        let Some(n) = self.seat_of(device) else { return Ok(false) };
        if self.cursors.contains_key(&n) {
            let look = vec!["LOOK".to_string()];
            if let Some(Action::Engine(a)) = self.input.keys.lookup_on(&look, device, press).cloned() {
                self.cursor_action(n, &a, now)?;
                return Ok(true);
            }
        }
        let contexts = self.contexts();
        if self.input.keys.lookup_on(&contexts, device, press) == Some(&Action::Engine("cursor".into())) {
            if self.cursors.remove(&n).is_none() {
                self.open_cursor(n, true);
            } else {
                self.r.cancel_order(n);
            }
            return Ok(true);
        }
        // a seat with its cursor open is in the cursor's context: its other
        // presses do nothing (the `LOOK` context is modal)
        Ok(self.cursors.contains_key(&n))
    }

    /// A `LOOK` action of seat `n`'s cursor (M 8.1).
    fn cursor_action(&mut self, n: SeatId, a: &str, now: Duration) -> RtResult<()> {
        let Some(c) = self.cursors.get(&n).copied() else { return Ok(()) };
        let my_turn = self.players_turn() && self.r.awaiting() == Some(n);
        self.last_cursor = Some(n);
        match a {
            _ if cursor::dir_of(a).is_some() => {
                let p = cursor::moved(&self.r.w, c.pos, cursor::dir_of(a).unwrap());
                self.cursors.insert(n, Cursor { pos: p, ..c });
            }
            "next" | "prev" => {
                let from = self.cursor_start(n).unwrap_or(c.pos);
                let cells = cursor::visible(&self.r.w, from);
                if !cells.is_empty() {
                    let i = cells.iter().position(|p| *p == c.pos);
                    let k = match (a, i) {
                        ("next", Some(i)) => (i + 1) % cells.len(),
                        ("prev", Some(i)) => (i + cells.len() - 1) % cells.len(),
                        ("prev", None) => cells.len() - 1,
                        _ => 0,
                    };
                    self.cursors.insert(n, Cursor { pos: cells[k], ..c });
                }
            }
            "verbs" if my_turn => self.open_coin(n, c.pos, true)?,
            "confirm" if my_turn && self.verb.as_ref().is_some_and(|v| v.seat == n) => {
                self.verb_target(Target::Cell(c.pos), now)?;
            }
            "back" if self.verb.as_ref().is_some_and(|v| v.seat == n) => {
                self.undo_verb();
                if self.verb.is_none() && !c.stays {
                    self.cursors.remove(&n);
                }
            }
            "confirm" if my_turn => {
                if !c.stays {
                    self.cursors.remove(&n);
                }
                if self.r.travel_to(n, c.pos)? {
                    self.travel_step(now)?;
                }
            }
            "follow" if my_turn => {
                let target = cursor::actor_at(&self.r.w, c.pos);
                if !c.stays {
                    self.cursors.remove(&n);
                }
                if self.r.follow(n, target)? {
                    self.travel_step(now)?;
                } else {
                    self.note = Some("follow: nobody to follow".into());
                }
            }
            // on another seat's turn: an order for the seat's next one
            "confirm" if c.stays => self.r.give_order(n, OrderGoal::Cell(c.pos)),
            "follow" if c.stays => self.r.give_order(n, OrderGoal::Follow(cursor::actor_at(&self.r.w, c.pos))),
            "back" => {
                if !self.r.cancel_order(n) {
                    self.cursors.remove(&n);
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// The verb coin's candidates for seat `n` (M 8.1): the docked verbs
    /// menus' commands, or without one the commands bound in the game's
    /// base context for the seat's device, `noverb` ones left out.
    fn coin_candidates(&self, n: SeatId) -> Vec<String> {
        let from_menus: Vec<String> = self
            .docked
            .iter()
            .filter(|d| d.verbs)
            .flat_map(|d| d.items.iter())
            .filter_map(|it| match &it.action {
                ItemAction::Command(c) => Some(c.clone()),
                _ => None,
            })
            .collect();
        if self.docked.iter().any(|d| d.verbs) {
            return from_menus;
        }
        let device = self.seat_devices.get(&n).copied().unwrap_or(Device::Keyboard(1));
        let Some(ctx) = self.input.keys.base.as_ref().and_then(|b| self.input.keys.contexts.get(b)) else { return vec![] };
        let actions: Vec<&Action> = match device {
            Device::Keyboard(p) => ctx.parts.get(&p).unwrap_or(&ctx.rows).iter().map(|(_, a)| a).collect(),
            Device::Pad(_) => ctx.pad.iter().map(|(_, a)| a).collect(),
        };
        let mut out: Vec<String> = Vec::new();
        for a in actions {
            if let Action::Command(c) = a
                && !out.contains(c)
                && !self.r.w.is_noverb(c)
            {
                out.push(c.clone());
            }
        }
        out
    }

    /// A verb's row text: its docked menu's `<MENU>.<ITEM>`, else the
    /// command's `<COMMAND>.NAME`, else its name; and its shortcut.
    fn verb_label(&self, n: SeatId, command: &str) -> String {
        let docked = self.docked.iter().filter(|d| d.verbs).find_map(|d| d.items.iter().find(|it| it.action == ItemAction::Command(command.to_string())).map(|it| format!("{}.{}", d.full, it.key)));
        let key = docked.unwrap_or_else(|| format!("{command}.NAME"));
        let label = if self.m.has_text(&self.lang, &key) { self.r.w.text(&self.lang, &key, &[]) } else { command.rsplit('.').next().unwrap_or(command).to_string() };
        let device = self.seat_devices.get(&n).copied().unwrap_or(Device::Keyboard(1));
        match self.input.keys.shortcut(&self.contexts(), device, command) {
            Some(k) => format!("{label}  {k}"),
            None => label,
        }
    }

    /// `verbs`: the verb coin at cell `p` for seat `n` (M 8.1), next to the
    /// cell. Nothing opens when no verb fits.
    fn open_coin(&mut self, n: SeatId, p: rlml_core::rt::value::Pos, keys: bool) -> RtResult<()> {
        let Some(actor) = self.r.w.s.player else { return Ok(()) };
        let candidates = self.coin_candidates(n);
        let entries = self.r.w.coin(actor, &candidates, Target::Cell(p))?;
        if entries.is_empty() {
            return Ok(());
        }
        let items = entries
            .iter()
            .map(|(c, _)| Item { key: c.clone(), action: ItemAction::Command(c.clone()), hide: None, disabled: None, label: Some(self.verb_label(n, c)) })
            .collect();
        let menu = Menu { full: COIN_MENU.into(), ns: None, context: "MENU".into(), is_final: false, width: None, height: None, items, verbs: true };
        let mut layer = menu::Layer::open(&mut self.r.w, menu);
        layer.at = self.screen_cell(p);
        self.menus.layers.push(layer);
        self.menu_owner = Some(self.seat_devices.get(&n).copied().unwrap_or(Device::Keyboard(1)));
        self.coin = Some(Coin { seat: n, entries, keys });
        Ok(())
    }

    /// The coin's row chosen: its parameter is filled, and the command runs
    /// or, with more to fill, becomes the chosen verb (M 8.1).
    fn choose_coin(&mut self, now: Duration) -> RtResult<()> {
        let i = self.menus.layers.last().and_then(|l| l.rows.get(l.cursor)).map(|r| r.item);
        let coin = self.coin.take();
        self.menus.back();
        let (Some(coin), Some(i)) = (coin, i) else { return Ok(()) };
        let Some((command, fill)) = coin.entries.get(i).cloned() else { return Ok(()) };
        let filled = vec![fill];
        if self.r.w.verb_open(&command, &filled) {
            let (menu, item) = self
                .docked
                .iter()
                .filter(|d| d.verbs)
                .find_map(|d| d.items.iter().position(|it| it.action == ItemAction::Command(command.clone())).map(|k| (d.full.clone(), k)))
                .unwrap_or_default();
            if coin.keys {
                let args = filled.into_iter().map(|(n, v)| (Some(n), v)).collect();
                return self.command(CommandValue { name: command, args });
            }
            self.verb = Some(Verb { seat: coin.seat, command, menu, item, filled });
            return Ok(());
        }
        let _ = now;
        let args = filled.into_iter().map(|(n, v)| (Some(n), v)).collect();
        self.command(CommandValue { name: command, args })
    }

    /// Where the top menu's window was last drawn.
    pub fn menu_drawn_window(&self) -> Option<termplate_core::layout::Rect> {
        self.menu_drawn.window
    }

    /// The screen cell of map cell `p`, from the last frame.
    fn screen_cell(&self, p: rlml_core::rt::value::Pos) -> Option<(usize, usize)> {
        let r = self.view.map_rect?;
        let (cx, cy) = *self.view.camera.get(&p.map)?;
        let (sx, sy) = (p.x - cx, p.y - cy);
        (sx >= 0 && sy >= 0 && (sx as usize) < r.w && (sy as usize) < r.h).then(|| (r.x + sx as usize, r.y + sy as usize))
    }

    /// A press for the focused docked menu (D 11.3): `MENU`'s keys move,
    /// choose and go back; other presses pass through to the game.
    fn focus_press(&mut self, device: Device, press: &Press, now: Duration) -> RtResult<bool> {
        let Some(f) = self.focus.clone() else { return Ok(false) };
        if !self.drives(device, f.seat) {
            return Ok(false);
        }
        let rows = self.docked.iter().find(|d| d.full == f.menu).map_or(0, |d| d.items.len());
        let menu = vec!["MENU".to_string()];
        let Some(Action::Engine(a)) = self.input.keys.lookup_on(&menu, device, press).cloned() else { return Ok(false) };
        match a.as_str() {
            "cursor.north" => self.focus = Some(Focus { row: f.row.saturating_sub(1), ..f }),
            "cursor.south" => self.focus = Some(Focus { row: (f.row + 1).min(rows.saturating_sub(1)), ..f }),
            "confirm" => {
                self.focus = None;
                self.choose_verb(f.seat, &f.menu, f.row, true, now)?;
            }
            "back" => self.focus = None,
            _ => return Ok(false),
        }
        Ok(true)
    }

    /// Seat `n` chose a docked menu's row (M 8.1): a command without
    /// clickable parameters runs now, another becomes the chosen verb. From
    /// the keys, its targets are chosen with the look cursor.
    fn choose_verb(&mut self, n: SeatId, menu: &str, row: usize, keys: bool, now: Duration) -> RtResult<()> {
        if !self.players_turn() || self.r.awaiting() != Some(n) {
            return Ok(());
        }
        let Some(item) = self.docked.iter().find(|d| d.full == menu).and_then(|d| d.items.get(row)).cloned() else { return Ok(()) };
        let command = match item.action {
            ItemAction::Command(c) => c,
            ItemAction::Engine(a) => return self.engine(&a, now),
            ItemAction::Open { menu, stack } => {
                self.open(&menu, stack);
                return Ok(());
            }
        };
        self.verb_hint = None;
        if !self.r.w.verb_open(&command, &[]) {
            self.verb = None;
            return self.command(CommandValue { name: command, args: vec![] });
        }
        // from the keys, the parameters are asked as for a key (M 8.1)
        if keys {
            self.verb = None;
            return self.command(CommandValue { name: command, args: vec![] });
        }
        self.verb = Some(Verb { seat: n, command, menu: menu.to_string(), item: row, filled: vec![] });
        Ok(())
    }

    /// A click or the cursor's `confirm` on `t` for the chosen verb: it
    /// fills the first free parameter it fits, and when no clickable one is
    /// left the command runs (M 8.1). A target that fits none says so in
    /// the hover and takes no time.
    fn verb_target(&mut self, t: Target, _now: Duration) -> RtResult<()> {
        let (Some(v), Some(p)) = (self.verb.clone(), self.r.w.s.player) else { return Ok(()) };
        match self.r.w.fill_verb(p, &v.command, &v.filled, t)? {
            Some(fill) => {
                let mut filled = v.filled.clone();
                filled.push(fill);
                if self.r.w.verb_open(&v.command, &filled) {
                    self.verb = Some(Verb { filled, ..v });
                } else {
                    self.verb = None;
                    if self.cursors.get(&v.seat).is_some_and(|c| !c.stays) {
                        self.cursors.remove(&v.seat);
                    }
                    let args = filled.into_iter().map(|(n, v)| (Some(n), v)).collect();
                    self.command(CommandValue { name: v.command, args })?;
                }
            }
            None => {
                let key = format!("{}.NO_TARGET", v.command);
                let has = self.m.texts(&self.lang).contains_key(&key);
                self.verb_hint = Some(if has { self.r.w.text(&self.lang, &key, &[]) } else { "–".to_string() });
            }
        }
        Ok(())
    }

    /// `rclick` or `back` with a verb: the last filled parameter goes, and
    /// with none filled the verb (M 8.1).
    fn undo_verb(&mut self) {
        self.verb_hint = None;
        match &mut self.verb {
            Some(v) if !v.filled.is_empty() => {
                v.filled.pop();
            }
            _ => self.verb = None,
        }
    }

    /// The verb and the focus belong to the turn: they go when it ends.
    fn expire_verb(&mut self) {
        let turn = self.r.awaiting().filter(|_| self.players_turn());
        if self.verb.as_ref().is_some_and(|v| Some(v.seat) != turn) {
            self.verb = None;
            self.verb_hint = None;
        }
        if self.focus.as_ref().is_some_and(|f| Some(f.seat) != turn) {
            self.focus = None;
        }
    }

    /// The docked menus as drawn now: each row's text and the shortcut of
    /// the seat whose turn it is (M 8.1).
    fn docked_view(&self) -> BTreeMap<String, crate::draw::Docked> {
        let turn = self.r.awaiting().filter(|_| self.players_turn());
        let device = turn.and_then(|n| self.seat_devices.get(&n).copied()).unwrap_or(Device::Keyboard(1));
        let contexts = self.contexts();
        let pointed = self.pointer.and_then(|(x, y)| crate::draw::docked_at(&self.view, x, y));
        self.docked
            .iter()
            .map(|d| {
                let rows = d
                    .items
                    .iter()
                    .map(|it| {
                        let key = match &it.action {
                            ItemAction::Command(c) => self.input.keys.shortcut(&contexts, device, c).unwrap_or_default(),
                            _ => String::new(),
                        };
                        crate::draw::DockRow { label: self.r.w.text(&self.lang, &format!("{}.{}", d.full, it.key), &[]), key, enabled: turn.is_some() }
                    })
                    .collect();
                let docked = crate::draw::Docked {
                    rows,
                    pointed: pointed.as_ref().filter(|(m, _)| *m == d.full).map(|(_, i)| *i),
                    focused: self.focus.as_ref().filter(|f| f.menu == d.full).map(|f| f.row),
                    chosen: self.verb.as_ref().filter(|v| v.menu == d.full).map(|v| v.item),
                };
                (d.full.clone(), docked)
            })
            .collect()
    }

    /// The sentence line (M 8.1): the chosen verb's `<COMMAND>.SENTENCE`
    /// with the filled parameters and the target under the pointer or the
    /// cursor where it would go; without the text, the verb's label and
    /// the hover.
    fn sentence(&mut self) -> Option<String> {
        let v = self.verb.clone()?;
        let mut args: Vec<(String, Value)> = v.filled.clone();
        if let (Some(cell), Some(p)) = (self.hover_cell(), self.r.w.s.player)
            && let Ok(Some(preview)) = self.r.w.fill_verb(p, &v.command, &v.filled, Target::Cell(cell))
        {
            args.push(preview);
        }
        let key = format!("{}.SENTENCE", v.command);
        if self.m.has_text(&self.lang, &key) {
            // a parameter not filled shows empty
            for p in self.r.w.verb_params(&v.command) {
                if !args.iter().any(|(n, _)| *n == p.name) {
                    args.push((p.name, Value::Text(String::new())));
                }
            }
            return Some(self.r.w.text(&self.lang, &key, &args));
        }
        let label = self.docked.iter().find(|d| d.full == v.menu).and_then(|d| d.items.get(v.item)).map(|it| self.r.w.text(&self.lang, &format!("{}.{}", v.menu, it.key), &[])).unwrap_or(v.command.clone());
        let hover = self.hover_cell().and_then(|p| self.r.w.hover_text(&self.lang, p));
        Some(match hover {
            Some(h) => format!("{label} {h}"),
            None => label,
        })
    }

    /// The cell the hover describes: the awaited seat's cursor, else the
    /// cursor moved last, else the pointer's cell (M 8.1).
    fn hover_cell(&self) -> Option<rlml_core::rt::value::Pos> {
        if let Some(Asking { ui: AskUi::Aim { pos, .. }, .. }) = &self.asking {
            return Some(*pos);
        }
        let awaited = self.r.awaiting().filter(|_| self.players_turn()).and_then(|n| self.cursors.get(&n));
        let last = self.last_cursor.and_then(|n| self.cursors.get(&n));
        awaited.or(last).map(|c| c.pos).or_else(|| self.pointer_cell())
    }

    /// The cells to mark on the map: open cursors and the targets of
    /// orders (M 7.1).
    fn cursor_marks(&self) -> Vec<rlml_core::rt::value::Pos> {
        let mut out: Vec<_> = self.cursors.values().map(|c| c.pos).collect();
        if let Some(Asking { ui: AskUi::Aim { pos, .. }, .. }) = &self.asking {
            out.push(*pos);
        }
        // the mouse pointer's map cell, when no menu is over the map (M 8.1)
        if !self.menus.is_open()
            && let Some(p) = self.pointer_cell()
            && !out.contains(&p)
        {
            out.push(p);
        }
        for o in self.r.orders.values() {
            match o.goal {
                OrderGoal::Cell(p) => out.push(p),
                OrderGoal::Follow(Some(e)) => out.extend(self.r.w.s.at(e)),
                OrderGoal::Follow(None) => {}
            }
        }
        out
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
        let press = Press::Key(b.clone());
        let always = self.input.keys.is_always(Device::Keyboard(1), &press);
        // menus take the mouse without bindings (M 8.1): a click chooses the
        // row under it, and a click outside the window is `back`
        if !always && self.menus.is_open() {
            if b == Binding::mouse(termplate_core::keys::Mouse::Click) {
                match self.menu_drawn.row_at(x, y) {
                    Some(i) => {
                        if let Some(top) = self.menus.top() {
                            top.cursor = i;
                        }
                        self.choose(now)?;
                    }
                    None if !self.menu_drawn.inside(x, y) => self.engine("back", now)?,
                    None => {}
                }
            } else if b == Binding::mouse(termplate_core::keys::Mouse::RightClick) && !self.menu_drawn.inside(x, y) {
                self.engine("back", now)?;
            }
            return Ok(());
        }
        if !always && !self.players_turn() {
            return Ok(());
        }
        if !always && self.ask_mouse(&b)? {
            return Ok(());
        }
        if !always
            && let Some(n) = self.r.awaiting()
            && self.r.travelling(n)
        {
            self.r.stop_travel(n);
            return Ok(());
        }
        if !always && let Some(n) = self.r.awaiting() {
            let click = b == Binding::mouse(termplate_core::keys::Mouse::Click);
            let rclick = b == Binding::mouse(termplate_core::keys::Mouse::RightClick);
            // a docked menu's row chooses its verb (M 8.1)
            if click && let Some((menu, row)) = crate::draw::docked_at(&self.view, x, y) {
                self.focus = None;
                return self.choose_verb(n, &menu, row, false, now);
            }
            if self.verb.as_ref().is_some_and(|v| v.seat == n) {
                if click && let Some(p) = self.pointer_cell() {
                    return self.verb_target(Target::Cell(p), now);
                }
                if rclick {
                    self.undo_verb();
                    return Ok(());
                }
            }
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
            Some(Action::Focus(_)) | None => {}
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
