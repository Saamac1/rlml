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
use rlml_core::rt::profile::Profile;
use rlml_core::rt::store::{AutosaveRules, Saved, Saves, Slot, Store};
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

/// The menus docked in any layout: `- v menu NAME` rows and menus on
/// tabs (D 11.1), each once.
fn docked_menus(m: &Model) -> Vec<Menu> {
    let mut out: Vec<Menu> = Vec::new();
    for def in m.defs.values().filter(|d| d.kind == rlml_core::model::Kind::Layout) {
        let Some(layout) = def.block() else { continue };
        for item in layout.sub("legend") {
            let words: Vec<&str> = item.tokens.iter().map(|t| t.text.as_str()).collect();
            let names: Vec<&str> = match words.iter().position(|w| *w == "menu" || *w == "tabs") {
                Some(i) if words[i] == "menu" => words.get(i + 1).copied().into_iter().collect(),
                Some(i) => words[i + 1..].iter().copied().take_while(|w| !w.starts_with("size")).collect(),
                None => vec![],
            };
            for n in names {
                let Ok(d) = m.resolve_kind(item.namespace.as_deref(), n, &[rlml_core::model::Kind::Menu]) else { continue };
                if !out.iter().any(|x| x.full == d.full)
                    && let Some(menu) = Menu::read(m, &d.full)
                {
                    out.push(menu);
                }
            }
        }
    }
    out
}

/// How long a held button waits before it repeats, and then between its
/// repeats (M 7.7).
const HOLD_DELAY: Duration = Duration::from_millis(350);
const HOLD_REPEAT: Duration = Duration::from_millis(140);

/// The engine's menu: a new game, the saves when there are some, and
/// leaving (M 8).
fn engine_menu(saves: bool, profile: bool) -> Menu {
    let item = |key: &str, action: &str| Item { key: key.into(), action: ItemAction::Engine(action.into()), hide: None, disabled: None, label: None };
    let mut items = vec![item("NEW_GAME", "new_game")];
    if saves {
        items.push(item("LOAD_GAME", "load_game"));
    }
    if profile {
        items.push(item("RESET_PROFILE", "reset_profile"));
    }
    items.push(item("EXIT", "exit"));
    Menu { full: ENGINE_MENU.into(), ns: None, context: "MENU".into(), is_final: true, width: None, height: None, items, verbs: false }
}

/// The list of saves `load_game` opens; a row loads its slot.
pub const LOAD_MENU: &str = "<load>";

/// A dialogue's `choice` as a menu in context `DIALOGUE` (M 7.5).
pub const DIALOGUE_MENU: &str = "<dialogue>";

/// A slot as the list of saves shows it.
fn slot_label(slot: Slot) -> String {
    match slot {
        Slot::Auto(n) => format!("auto {n}"),
        Slot::Manual(n) => format!("save {n}"),
        Slot::Quit => "quit".into(),
        Slot::Only => "game".into(),
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
    /// The log being read (`read LOG`, M 7.4): presentation, not state.
    pub reading: Option<Reading>,
    /// The window's size in pixels when the front end knows it; without
    /// it a cell counts as twice as high as wide (D 11.1).
    pub pixels: Option<(usize, usize)>,
    /// The front end tells when a mouse button is let go
    /// (`mouse_release`), so a held button repeats (M 7.7).
    pub releases: bool,
    /// The held button's action and when it repeats next.
    held: Option<(ItemAction, Duration)>,
    /// The panel a held `hold PANEL` button shows over the window.
    pub overlay: Option<String>,
    /// `settings - targeting cursor|list|auto` (D 3.2), `cursor` by default.
    targeting: String,
    /// The log's teletype (`baud`, D 9.3): the messages from `typed` on
    /// are being typed, `type_chars` characters so far, as of `type_last`.
    typed: usize,
    type_chars: f64,
    type_last: Duration,
    /// The teletype's characters a second, when a log area has `baud`.
    baud: Option<f64>,
    /// A key of the keyboard has been pressed: buttons show their keys
    /// (`<KEY>.KEYS`, D 11.5).
    pub keyboard: bool,
    /// The game's save slots, when the front end gives a store (M 5.4).
    pub saves: Option<Saves<Box<dyn Store>>>,
    /// The game's profile file, when the front end gives a store (M 5.6).
    pub profile: Option<Profile<Box<dyn Store>>>,
    /// The join screen, while it is open (M 8.2).
    pub joining: Option<crate::join::JoinScreen>,
    /// The device of the last press: it chose `new_game`, so it is seat 1.
    last_device: Device,
    /// The seats' devices came from the join screen or a join: a pad that
    /// connects does not take a seat then.
    chosen_devices: bool,
    /// A scene's field: what the player has typed (RS 9.7).
    pub field: String,
    /// The menu a scene's `open` showed, until it closes.
    scene_menu: Option<String>,
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

/// A log open to be read back (M 7.4): its name, how many lines up from
/// the newest it shows, and the height of its last page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reading {
    pub log: String,
    pub back: usize,
    pub page: usize,
    /// A modal panel (`open PANEL`, D 11.5) read the same way: `back` is
    /// then the rows scrolled down from its top.
    pub panel: bool,
    /// The scroll buttons drawn last, up and down, where something is
    /// hidden that way (M 7.4).
    pub arrows: (Option<termplate_core::layout::Rect>, Option<termplate_core::layout::Rect>),
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

/// How often the teletype's typing shows (D 9.3).
const TYPE_PACE: Duration = Duration::from_millis(30);

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
            reading: None,
            pixels: None,
            releases: false,
            held: None,
            overlay: None,
            targeting: setting("targeting").unwrap_or_else(|| "cursor".into()),
            typed: 0,
            type_chars: 0.0,
            type_last: Duration::ZERO,
            baud: None,
            keyboard: false,
            saves: None,
            profile: None,
            joining: None,
            last_device: Device::Keyboard(1),
            chosen_devices: false,
            field: String::new(),
            scene_menu: None,
        };
        app.baud = crate::draw::log_baud(&app.r.w, "");
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

    /// Saves go to `store` from now on (M 5.4); the game being played
    /// becomes a session of its own there.
    pub fn set_store(&mut self, store: Box<dyn Store>) {
        self.saves = Some(Saves::new(store, AutosaveRules::read(self.m)));
    }

    /// The profile is kept in `store` from now on (M 5.6). The next new
    /// game reads it; `restart` starts the game being played over with it.
    pub fn set_profile_store(&mut self, store: Box<dyn Store>, restart: bool) -> RtResult<()> {
        self.profile = Some(Profile::new(store, self.m));
        if restart && self.playing {
            self.games -= 1;
            self.start(self.seed)?;
        }
        Ok(())
    }

    /// The game's `seats` least and most (D 3.3).
    fn seat_range(&self) -> (u8, u8) {
        let g = self.m.named_game();
        let row = g.and_then(|g| g.items.iter().find(|i| i.key() == "seats")).map(|i| i.rest(1).to_string());
        let (lo, hi) = row.as_deref().and_then(rlml_core::rt::world::seat_range).unwrap_or((1, 1));
        (lo.min(hi).max(1), lo.max(hi).max(1))
    }

    /// A new game with `seed`: through the join screen when more than one
    /// seat may play (M 8.2), with the device that chose it as seat 1.
    fn start(&mut self, seed: u64) -> RtResult<()> {
        let range = self.seat_range();
        if range.1 > 1 {
            self.menus.clear();
            self.playing = false;
            self.joining = Some(crate::join::JoinScreen::new_game(seed, range, self.last_device));
            return Ok(());
        }
        self.start_with(seed, None)
    }

    /// The join screen's start: a new game with its seats, or the save with
    /// its seats' devices, skipped seats leaving and new ones joining.
    fn finish_join(&mut self) -> RtResult<()> {
        use crate::join::Slot as JoinSlot;
        let Some(j) = self.joining.take() else { return Ok(()) };
        match j.load {
            None => {
                let devices: Vec<Device> = j.seats.iter().filter_map(|(_, s)| if let JoinSlot::Taken(d) = s { Some(*d) } else { None }).collect();
                self.start_with(j.seed, Some(devices.len() as u8))?;
                self.seat_devices = (1..).zip(devices).collect();
            }
            Some(save) => {
                let r = Runner::load(self.m, &save)?;
                self.seed = r.w.s.game.seed;
                self.play(r)?;
                self.seat_devices.clear();
                for (n, slot) in j.seats {
                    let saved = save.now.s.seat(n).is_some();
                    match slot {
                        JoinSlot::Taken(d) if saved => {
                            self.seat_devices.insert(n, d);
                        }
                        JoinSlot::Taken(d) => {
                            if let Some(k) = self.r.join()? {
                                self.seat_devices.insert(k, d);
                            }
                        }
                        // a saved seat without a device leaves before the
                        // first input (M 8.2)
                        JoinSlot::Skipped | JoinSlot::Waiting => {
                            self.r.leave_seat(n)?;
                        }
                    }
                }
                self.advance(0)?;
            }
        }
        self.chosen_devices = true;
        Ok(())
    }

    /// A press on the join screen, by `JOIN`'s bindings only.
    fn join_press(&mut self, device: Device, press: &Press) -> RtResult<()> {
        use crate::join::JoinStep;
        let action = self.input.keys.lookup_only("JOIN", device, press).cloned();
        let Some(j) = self.joining.as_mut() else { return Ok(()) };
        let step = match action {
            Some(Action::Engine(a)) => match a.as_str() {
                "join" | "confirm" => j.join(device),
                "back" => j.back(device),
                "skip" => j.skip(),
                _ => JoinStep::Stay,
            },
            _ => JoinStep::Stay,
        };
        match step {
            JoinStep::Start => self.finish_join(),
            JoinStep::Cancel => {
                self.joining = None;
                self.quit_game();
                Ok(())
            }
            JoinStep::Stay => Ok(()),
        }
    }

    /// Starts a new game with `seed` and `seats` joined seats (`None`: the
    /// game's least) and runs it to the player's first turn (M 5.1); the
    /// menus close. The profile's snapshot is taken from its file now.
    fn start_with(&mut self, seed: u64, seats: Option<u8>) -> RtResult<()> {
        let file = match self.profile.as_mut() {
            Some(p) => {
                let (file, warning) = p.load();
                if let Some(w) = warning {
                    self.note = Some(w);
                }
                Some(file)
            }
            None => None,
        };
        let mut w = World::new_game_with(self.m, seed, seats, file.as_deref())?;
        w.begin()?;
        if let Some(saves) = self.saves.as_mut() {
            saves.new_session(self.m);
        }
        self.seed = seed;
        self.games += 1;
        self.play(Runner::new(w))
    }

    /// Goes on from a save (M 5.2): `begin` does not run.
    fn load(&mut self, slot: Slot) -> RtResult<()> {
        let Some(saves) = self.saves.as_mut() else { return Ok(()) };
        let save = saves.load(self.m, slot)?;
        // more than one seat: each saved seat asks for its device (M 8.2)
        let range = self.seat_range();
        if range.1 > 1 {
            self.menus.clear();
            self.playing = false;
            self.joining = Some(crate::join::JoinScreen::load(save, range));
            return Ok(());
        }
        let r = Runner::load(self.m, &save)?;
        self.seed = r.w.s.game.seed;
        self.play(r)
    }

    /// Plays `r` from its next turn: the view starts over and the menus
    /// close.
    fn play(&mut self, r: Runner<'m>) -> RtResult<()> {
        self.r = r;
        self.chosen_devices = false;
        self.view = View { truecolor: self.view.truecolor, ..View::default() };
        self.menus.clear();
        self.frames.clear();
        self.last_frame = None;
        self.status = Status::NoPlayer { rounds: 0 };
        self.playing = true;
        self.assign_devices();
        self.advance(0)
    }

    /// The runner's automatic saves and `delete_save(auto)`, to the store
    /// (M 5.4, M 5.5). Without a store they go nowhere.
    fn write_saves(&mut self) {
        let writes = self.r.take_profile_writes();
        if let Some(p) = self.profile.as_mut()
            && let Err(e) = p.write(&writes)
        {
            self.note = Some(format!("the profile was not written: {e}"));
        }
        let due = std::mem::take(&mut self.r.due);
        let delete = std::mem::take(&mut self.r.delete_auto);
        let Some(saves) = self.saves.as_mut() else { return };
        if delete {
            saves.delete_auto();
        }
        for save in due {
            if let Err(e) = saves.autosave(save) {
                self.note = Some(format!("the game was not saved: {e}"));
            }
        }
    }

    /// Leaving the game being played saves it by the `quit` row (M 5.4).
    fn save_on_leaving(&mut self) {
        if !self.playing {
            return;
        }
        if let Some(saves) = self.saves.as_mut()
            && let Err(e) = saves.quit(&self.r)
        {
            self.note = Some(format!("the game was not saved: {e}"));
        }
    }

    /// The player's own save (`save`, M 5.2): not on a `nosave` map, and
    /// in a permadeath game it ends the game.
    fn save_game(&mut self) {
        if !self.playing {
            return;
        }
        if self.r.w.save_blocked() {
            self.note = Some(if self.r.in_scene() { "the game cannot be saved in a scene".into() } else { "the game cannot be saved here".into() });
            return;
        }
        let Some(saves) = self.saves.as_mut() else {
            self.note = Some("saving is not available here".into());
            return;
        };
        match saves.manual(&self.r) {
            Ok(Saved::In(slot)) => self.note = Some(format!("saved: {}", slot_label(slot))),
            Ok(Saved::AndEnds) => {
                self.note = Some("saved".into());
                self.quit_game();
            }
            Err(e) => self.note = Some(format!("the game was not saved: {e}")),
        }
    }

    /// `load_game`: the saves, newest first, in a list to choose from.
    fn open_load_menu(&mut self) {
        let list = self.saves.as_ref().map(|s| s.list()).unwrap_or_default();
        if list.is_empty() {
            self.note = Some("no saves".into());
            return;
        }
        let items = list
            .iter()
            .map(|l| {
                let s = &l.save.now.s;
                let map = s.last_player_map.or(s.player.and_then(|p| s.at(p)).map(|p| p.map)).and_then(|m| s.maps.get(m)).map_or("", |m| m.name.as_str());
                let label = format!("{} · {map} · turn {}", slot_label(l.slot), s.game.turn());
                Item { key: l.slot.name(), action: ItemAction::Engine(format!("load:{}", l.slot.name())), hide: None, disabled: None, label: Some(label) }
            })
            .collect();
        let menu = Menu { full: LOAD_MENU.into(), ns: None, context: "MENU".into(), is_final: false, width: None, height: None, items, verbs: false };
        let layer = menu::Layer::open(&mut self.r.w, menu);
        self.menus.layers.push(layer);
    }

    /// Gives the seats their devices: keyboard part 1, then the pads in
    /// connection order, then the other keyboard parts (M 8.2). A pad that
    /// connects takes a seat from a keyboard part, since it is plugged in
    /// to play.
    fn assign_devices(&mut self) {
        if self.chosen_devices {
            return;
        }
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
        // a seat whose pad went is played from keyboard part 1 (M 8.2)
        if self.chosen_devices {
            self.seat_devices.retain(|_, d| *d != Device::Pad(id));
        }
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
                let has_saves = self.saves.as_ref().is_some_and(|s| !s.list().is_empty());
                let has_profile = self.profile.is_some() && !self.m.profiles.is_empty();
                let layer = menu::Layer::open(&mut self.r.w, engine_menu(has_saves, has_profile));
                self.menus.layers.push(layer);
            }
        }
    }

    /// The next game's seed: a new one each time (M 5.1).
    fn next_seed(&self) -> u64 {
        let mut s = self.seed;
        rlml_core::rt::rng::splitmix64(&mut s)
    }

    /// The key contexts now: the game's, then the open menus' (M 8); the
    /// join screen's `JOIN` alone while it is open.
    pub fn contexts(&self) -> Vec<String> {
        if self.joining.is_some() {
            return vec!["JOIN".into()];
        }
        // a scene takes `SCENE` over the game's keys (M 8)
        if self.r.in_scene() && !self.menus.is_open() {
            return self.stack.iter().cloned().chain(["SCENE".to_string()]).collect();
        }
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
        if let Some(j) = &self.joining {
            // the join screen: its lines in the window's middle (M 8.2)
            let (w, h) = self.size;
            let mut f = Frame::new(w, h);
            let lines = j.lines(&self.r.w, &self.lang);
            let width = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
            let (x, y) = (w.saturating_sub(width) / 2, h.saturating_sub(lines.len()) / 2);
            for (i, l) in lines.iter().enumerate() {
                f.text(x, y + i, w.saturating_sub(x), l, termplate_core::frame::Role::Normal);
            }
            return f;
        }
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
        self.view.portrait = self.portrait();
        self.view.pointer = self.pointer;
        self.view.enabled = self.players_turn() && !self.menus.is_open();
        self.view.typing = self.baud.map(|_| (self.typed, self.type_chars as usize));
        self.view.keyboard = self.keyboard;
        if self.reading.as_ref().is_some_and(|r| !r.panel) {
            // over a `read` log area the view keeps the log's place (D 11.4)
            let over = self.view.read_rects.iter().find(|(_, l, _)| self.reading.as_ref().is_some_and(|r| r.log == *l)).cloned();
            if self.playing && let Some((rect, _, below)) = over {
                return self.draw_reading_over(w, h, rect, below);
            }
            return self.draw_reading(w, h);
        }
        let mut f = if self.playing { self.view.draw(&mut self.r.w, &self.lang, w, h) } else { Frame::new(w, h) };
        // a scene's line over the map, under any menu (M 7.4)
        if let Some(pause) = self.r.scene_pause().cloned() {
            menu::draw_say(&mut f, &self.r.w, &self.lang, &pause, Some(&self.field));
        }
        self.menu_drawn = match self.menus.layers.last() {
            Some(layer) => menu::draw(&mut f, &self.r.w, &self.lang, layer),
            None => Default::default(),
        };
        if self.coin.is_some() && self.menus.position(COIN_MENU).is_none() {
            self.coin = None;
        }
        if let Some(name) = self.overlay.clone()
            && let Some(pb) = crate::panel::block(&self.r.w, &name)
        {
            // over the window's largest band beside the map (M 7.7)
            let over = self.beside_map(w, h);
            f.fill(over, ' ', termplate_core::frame::Role::Normal);
            let mut ui = crate::panel::PanelUi { truecolor: self.view.truecolor, enabled: true, ..Default::default() };
            let lang = self.lang.clone();
            self.r.w.holding_view(|world| crate::panel::draw(&mut f, world, &lang, pb, &name, over, &mut ui));
        }
        // a modal panel over the band beside the map, scrolled as the
        // reading view (D 11.5)
        if let Some(r) = self.reading.clone().filter(|r| r.panel) {
            let band = self.beside_map(w, h);
            f.fill(band, ' ', termplate_core::frame::Role::Normal);
            let line = |key: String| self.m.has_text(&self.lang, &key).then(|| self.r.w.text(&self.lang, &key, &[]));
            let (title, hint) = (line(format!("{}.TITLE", r.log)), line(format!("{}.READ", r.log)));
            let (top, bottom) = (title.is_some() as usize, hint.is_some() as usize);
            if let Some(t) = &title {
                f.text(band.x, band.y, band.w, t, termplate_core::frame::Role::Title);
            }
            if let Some(t) = &hint
                && band.h > 0
            {
                f.text(band.x, band.y + band.h - 1, band.w, t, termplate_core::frame::Role::Dim);
            }
            let rect = termplate_core::layout::Rect::new(band.x, band.y + top, band.w, band.h.saturating_sub(top + bottom));
            let (back, used) = self.draw_modal_panel(&mut f, &r.log, rect, r.back);
            let arrows = self.draw_arrows(&mut f, rect, back > 0, back + rect.h < used);
            self.reading = Some(Reading { back, page: rect.h, arrows, ..r });
        }
        if let Some(note) = &self.note
            && h > 0
        {
            f.text(0, h - 1, w, note, termplate_core::frame::Role::Normal);
        }
        f
    }

    /// Whether a window of `cols`×`rows` cells and `pixels` holds the
    /// layout it would choose: the browser picks its font size by this.
    pub fn fits(&self, cols: usize, rows: usize, pixels: (usize, usize)) -> bool {
        let portrait = pixels.1 > pixels.0;
        self.view.solve(&self.r.w, cols, rows, portrait).is_ok_and(|(_, s)| !s.too_small)
    }

    /// Portrait: the window is taller than wide in pixels, a cell counted
    /// as 1:2 without the pixels (D 11.1).
    fn portrait(&self) -> bool {
        match self.pixels {
            Some((pw, ph)) => ph > pw,
            None => 2 * self.size.1 > self.size.0,
        }
    }

    /// `tab NAME`: the tabs area holding NAME shows it (D 11.1).
    fn select_tab(&mut self, name: &str) {
        let found = self.view.tab_areas.iter().find_map(|(key, names)| names.iter().position(|n| n == name).map(|i| (key.clone(), i)));
        if let Some((key, i)) = found {
            self.view.tabs.insert(key.clone(), i);
            self.view.last_tabs = Some(key);
        }
    }

    /// `tab.next` and `tab.prev`: the area whose tab changed last, else
    /// the first (D 11.1).
    fn step_tab(&mut self, by: isize) {
        let area = self.view.tab_areas.iter().find(|(k, _)| Some(k) == self.view.last_tabs.as_ref()).or(self.view.tab_areas.first()).cloned();
        if let Some((key, names)) = area {
            let n = names.len() as isize;
            let now = self.view.tabs.get(&key).copied().unwrap_or(0) as isize;
            self.view.tabs.insert(key.clone(), (now + by).rem_euclid(n.max(1)) as usize);
            self.view.last_tabs = Some(key);
        }
    }

    /// A panel's button (M 7.7): its action on the player's turn, as a
    /// docked menu's row does.
    fn button_action(&mut self, action: ItemAction, now: Duration) -> RtResult<()> {
        match action {
            // without releases from the front end a second click closes it
            ItemAction::Hold(p) => self.overlay = if !self.releases && self.overlay.as_ref() == Some(&p) { None } else { Some(p) },
            ItemAction::Tab(t) => self.select_tab(&t),
            ItemAction::Read(l) => self.read_log(&l),
            // a panel is read at any time, a menu on the turn
            ItemAction::Open { menu, stack } => {
                if self.players_turn() || self.m.kind(&menu) == Some(rlml_core::model::Kind::Panel) {
                    self.open(&menu, stack);
                }
            }
            ItemAction::Engine(a) => self.engine(&a, now)?,
            ItemAction::Command(c) => {
                if self.players_turn() {
                    self.verb_hint = None;
                    self.command(CommandValue { name: c, args: vec![] })?;
                }
            }
        }
        Ok(())
    }

    /// The front end's mouse button went up: a held button stops (M 7.7).
    pub fn mouse_release(&mut self) {
        self.held = None;
        self.overlay = None;
        self.view.pressed = None;
    }

    /// `read LOG`: the log opens at its newest (M 7.4).
    fn read_log(&mut self, log: &str) {
        self.reading = Some(Reading { log: log.to_string(), back: 0, page: 1, panel: false, arrows: (None, None) });
    }

    /// A `MENU` action in the reading view.
    fn reading_action(&mut self, a: &str) {
        let page = self.reading.as_ref().map_or(1, |r| r.page.max(1)) as isize;
        match a {
            "cursor.north" => self.scroll_reading(1),
            "cursor.south" => self.scroll_reading(-1),
            "page.up" => self.scroll_reading(page),
            "page.down" => self.scroll_reading(-page),
            "first" => self.scroll_reading(isize::MAX / 2),
            "last" => self.scroll_reading(-(isize::MAX / 2)),
            "back" | "confirm" | "close" => self.reading = None,
            _ => {}
        }
    }

    /// Lines up (positive) or down; the drawing cuts it to the oldest, and
    /// a panel's to its last row.
    fn scroll_reading(&mut self, by: isize) {
        if let Some(r) = &mut self.reading {
            let by = if r.panel { by.saturating_neg() } else { by };
            r.back = (r.back as isize).saturating_add(by).max(0) as usize;
        }
    }

    /// The reading view: `<LOG>.TITLE` at the top and `<LOG>.READ` at the
    /// bottom when written, the log between (M 7.4).
    fn draw_reading(&mut self, w: usize, h: usize) -> Frame {
        let mut f = Frame::new(w, h);
        let Some(r) = self.reading.clone() else { return f };
        let line = |key: String| self.m.has_text(&self.lang, &key).then(|| self.r.w.text(&self.lang, &key, &[]));
        let (title, hint) = (line(format!("{}.TITLE", r.log)), line(format!("{}.READ", r.log)));
        let top = title.is_some() as usize;
        let bottom = hint.is_some() as usize;
        if let Some(t) = &title {
            f.text(0, 0, w, t, termplate_core::frame::Role::Title);
        }
        if let Some(t) = &hint
            && h > 0
        {
            f.text(0, h - 1, w, t, termplate_core::frame::Role::Dim);
        }
        let rect = termplate_core::layout::Rect::new(0, top, w, h.saturating_sub(top + bottom));
        let (lines, back) = crate::draw::draw_log_back(&mut f, &self.r.w, &self.lang, &r.log, rect, self.view.truecolor, crate::draw::LogDraw { back: r.back, gap: crate::draw::Gap::All, typing: None });
        let arrows = self.draw_arrows(&mut f, rect, lines > back + rect.h, back > 0);
        self.reading = Some(Reading { back, page: rect.h, arrows, ..r });
        f
    }

    /// The reading view over a `read` log area (D 11.4): the game under
    /// it, the log's messages on its rows as they were and the older ones
    /// above them over the map, an empty line between those, the title at
    /// the top. The area's `read PANEL` and the `<LOG>.READ` hint go under
    /// the log, or right of it when it reaches the bottom.
    fn draw_reading_over(&mut self, w: usize, h: usize, area: termplate_core::layout::Rect, below: Option<String>) -> Frame {
        use termplate_core::frame::Role;
        use termplate_core::layout::Rect;
        let mut f = self.view.draw(&mut self.r.w, &self.lang, w, h);
        let Some(r) = self.reading.clone() else { return f };
        let line = |key: String| self.m.has_text(&self.lang, &key).then(|| self.r.w.text(&self.lang, &key, &[]));
        let (title, hint) = (line(format!("{}.TITLE", r.log)), line(format!("{}.READ", r.log)));
        let bottom = (area.y + area.h).min(h);
        let band = Rect::new(area.x, 0, area.w, bottom);
        f.fill(band, ' ', Role::Normal);
        let top = title.is_some() as usize;
        if let Some(t) = &title {
            f.text(band.x, 0, band.w, t, Role::Title);
        }
        let rect = Rect::new(area.x, top, area.w, bottom.saturating_sub(top));
        let how = crate::draw::LogDraw { back: r.back, gap: crate::draw::Gap::Above(area.h), typing: None };
        let (lines, back) = crate::draw::draw_log_back(&mut f, &self.r.w, &self.lang, &r.log, rect, self.view.truecolor, how);
        let arrows = self.draw_arrows(&mut f, rect, lines > back + rect.h, back > 0);
        self.reading = Some(Reading { back, page: rect.h, arrows, ..r });
        let right = (area.x + area.w).min(w);
        let rest = if bottom < h { Rect::new(0, bottom, w, h - bottom) } else { Rect::new(right, 0, w - right, h) };
        if rest.w == 0 || rest.h == 0 {
            return f;
        }
        f.fill(rest, ' ', Role::Normal);
        let hint_rows = hint.is_some() as usize;
        if let Some(t) = &hint {
            f.text(rest.x, rest.y + rest.h - 1, rest.w, t, Role::Dim);
        }
        if let Some(name) = below
            && let Some(pb) = crate::panel::block(&self.r.w, &name)
        {
            let (pw, ph) = crate::panel::size(pb);
            let room = Rect::new(rest.x, rest.y, rest.w, rest.h.saturating_sub(hint_rows));
            let at = Rect::new(room.x + room.w.saturating_sub(pw) / 2, room.y + room.h.saturating_sub(ph) / 2, pw.min(room.w), ph.min(room.h));
            let mut ui = crate::panel::PanelUi { truecolor: self.view.truecolor, enabled: true, ..Default::default() };
            let lang = self.lang.clone();
            self.r.w.holding_view(|world| crate::panel::draw(&mut f, world, &lang, pb, &name, at, &mut ui));
        }
        f
    }

    /// The scroll buttons at the right of `r` (M 7.4): `↑` at the top when
    /// something is hidden above, `↓` at the bottom when below; three rows
    /// high where there is room.
    fn draw_arrows(&mut self, f: &mut Frame, r: termplate_core::layout::Rect, above: bool, below: bool) -> (Option<termplate_core::layout::Rect>, Option<termplate_core::layout::Rect>) {
        use termplate_core::layout::Rect;
        let (look, pressed) = crate::draw::button_look(&self.r.w, self.view.truecolor);
        let (bw, bh) = (5.min(r.w), if r.h >= 7 { 3 } else { 1 });
        if bw == 0 || r.h < 2 * bh {
            return (None, None);
        }
        let x = r.x + r.w - bw;
        let mut out = (None, None);
        for (show, y, label, slot) in [(above, r.y, "↑", 0), (below, r.y + r.h - bh, "↓", 1)] {
            if !show {
                continue;
            }
            let cells = Rect::new(x, y, bw, bh);
            let look = if self.view.pressed == Some(cells) { (look.0, look.1.over(pressed)) } else { look };
            crate::panel::draw_button(f, cells, label, look, false, false, false);
            if slot == 0 { out.0 = Some(cells) } else { out.1 = Some(cells) }
        }
        out
    }

    /// The window's largest band outside the map area (below, above, right
    /// or left of it), so the map stays in sight; the whole window without
    /// a map area (D 11.5, M 7.7).
    fn beside_map(&self, w: usize, h: usize) -> termplate_core::layout::Rect {
        use termplate_core::layout::Rect;
        let whole = Rect::new(0, 0, w, h);
        let Some(m) = self.view.map_rect else { return whole };
        let (right, below) = ((m.x + m.w).min(w), (m.y + m.h).min(h));
        let bands = [Rect::new(0, below, w, h - below), Rect::new(0, 0, w, m.y), Rect::new(right, 0, w - right, h), Rect::new(0, 0, m.x, h)];
        bands.into_iter().filter(|b| b.w > 0 && b.h > 0).max_by_key(|b| b.w * b.h).unwrap_or(whole)
    }

    /// A modal panel in `r` (D 11.5): drawn whole off the screen, centred
    /// across, centred down when it fits, else scrolled `back` rows from
    /// its top, cut to its last written row. Gives the `back` drawn and
    /// the rows the panel uses.
    fn draw_modal_panel(&mut self, f: &mut Frame, name: &str, r: termplate_core::layout::Rect, back: usize) -> (usize, usize) {
        let Some(pb) = crate::panel::block(&self.r.w, name) else { return (0, 0) };
        let (pw, ph) = crate::panel::size(pb);
        let mut off = Frame::new(r.w, ph);
        let whole = termplate_core::layout::Rect::new(r.w.saturating_sub(pw) / 2, 0, pw.min(r.w), ph);
        let mut ui = crate::panel::PanelUi { truecolor: self.view.truecolor, enabled: true, ..Default::default() };
        let lang = self.lang.clone();
        self.r.w.holding_view(|world| crate::panel::draw(&mut off, world, &lang, pb, name, whole, &mut ui));
        let used = (0..ph).rev().find(|&y| off.row(y).trim() != "").map_or(0, |y| y + 1);
        let (from, at) = if used <= r.h { (0, r.y + (r.h - used) / 2) } else { (back.min(used - r.h), r.y) };
        for y in 0..used.min(r.h) {
            for x in 0..r.w.min(f.width.saturating_sub(r.x)) {
                f.cells[(at + y) * f.width + r.x + x] = off.cells[(from + y) * off.width + x];
            }
        }
        (from, used)
    }

    /// The teletype types on to `now` (D 9.3): with all typed, the
    /// messages after them start from their first character.
    fn type_on(&mut self, now: Duration) {
        // a late tick (a first one, a busy page) does not skip the typing
        let since = now.saturating_sub(self.type_last).min(Duration::from_millis(100));
        self.type_last = now;
        let Some(cps) = self.baud else { return };
        let log = &self.r.w.log;
        if self.typed >= log.len() {
            self.typed = log.len();
            self.type_chars = 0.0;
            return;
        }
        self.type_chars += since.as_secs_f64() * cps;
        let left: usize = log[self.typed..].iter().map(|l| self.r.w.text(&self.lang, &l.key, &l.args).chars().count()).sum();
        if self.type_chars >= left as f64 {
            self.typed = log.len();
            self.type_chars = 0.0;
        }
    }

    /// A press shows at once what the teletype has not typed (D 9.3).
    fn typed_out(&mut self, now: Duration) {
        self.typed = self.r.w.log.len();
        self.type_chars = 0.0;
        self.type_last = now;
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
        self.write_saves();
        self.sync_scene()
    }

    /// A key at time `now` (M 8).
    pub fn key(&mut self, b: Binding, now: Duration) -> RtResult<()> {
        self.typed_out(now);
        self.keyboard = true;
        self.last_key = now;
        self.clock = now;
        self.note = None;
        // ctrl+c always leaves, as in every terminal program
        if b == Binding::key(KeyCode::Char('c'), Mods { ctrl: true, ..Mods::default() }) {
            self.quit = true;
            return Ok(());
        }
        // a scene's field takes the keys as text (RS 9.7)
        if self.field_key(&b)? {
            return self.sync_scene();
        }
        let device = Device::Keyboard(self.input.keys.part_of(&b));
        self.press(device, Press::Key(b), now)?;
        self.sync_scene()
    }

    /// A key for a scene's field: a character, `backspace`, `enter` to give
    /// the answer, `esc` to cancel it. `false` without a field.
    fn field_key(&mut self, b: &Binding) -> RtResult<bool> {
        use rlml_core::rt::world::{AskKind, Pause};
        let Some(Pause::Ask { kind, .. }) = self.r.scene_pause().cloned() else { return Ok(false) };
        if self.menus.is_open() {
            return Ok(false);
        }
        let Binding::Key(code, mods) = b else { return Ok(false) };
        match code {
            KeyCode::Char(c) if !mods.ctrl && !mods.alt => {
                let fits = match kind {
                    AskKind::Number { .. } => c.is_ascii_digit() || (*c == '-' && self.field.is_empty()),
                    AskKind::Text { max } => self.field.chars().count() < max,
                };
                if fits {
                    self.field.push(*c);
                }
            }
            KeyCode::Backspace => {
                self.field.pop();
            }
            KeyCode::Enter => self.field_answer()?,
            KeyCode::Esc => {
                self.field.clear();
                self.r.scene_answer(Value::None)?;
                self.advance(0)?;
            }
            _ => {}
        }
        Ok(true)
    }

    /// The field's text as its answer: a number in range or the text; an
    /// empty field is `none`.
    fn field_answer(&mut self) -> RtResult<()> {
        use rlml_core::rt::world::{AskKind, Pause};
        let Some(Pause::Ask { kind, .. }) = self.r.scene_pause().cloned() else { return Ok(()) };
        let v = match kind {
            _ if self.field.is_empty() => Value::None,
            AskKind::Number { .. } => match self.field.parse::<i64>() {
                Ok(n) => Value::Number(n),
                Err(_) => return Ok(()),
            },
            AskKind::Text { .. } => Value::Text(self.field.clone()),
        };
        if self.r.scene_answer(v)? {
            self.field.clear();
            self.advance(0)?;
        }
        Ok(())
    }

    /// The scene's menus as it waits: a `choice` as the dialogue menu, the
    /// menu `open` waits for, and a closed one answered (RS 9.7).
    fn sync_scene(&mut self) -> RtResult<()> {
        use rlml_core::rt::world::Pause;
        let pause = self.r.scene_pause().cloned();
        // the menu `open` showed has closed
        if let Some(name) = self.scene_menu.clone()
            && self.menus.position(&name).is_none()
            && self.reading.is_none()
        {
            self.scene_menu = None;
            if matches!(pause, Some(Pause::Open { .. })) {
                self.r.scene_closed()?;
                self.advance(0)?;
                return self.sync_scene();
            }
        }
        match pause {
            Some(Pause::Choice { scene, items, .. }) => {
                let cursor = self.menus.layers.last().filter(|l| l.menu.full == DIALOGUE_MENU).map(|l| l.cursor);
                self.menus.layers.retain(|l| l.menu.full != DIALOGUE_MENU);
                let rows = items
                    .iter()
                    .map(|i| {
                        let text = self.r.w.text(&self.lang, &format!("{scene}.{}", i.name), &i.args);
                        let label = match i.adjust {
                            Some((now, _, _)) => format!("{text}  ◂ {now} ▸"),
                            None => text,
                        };
                        Item { key: i.name.clone(), action: ItemAction::Engine("confirm".into()), hide: None, disabled: None, label: Some(label) }
                    })
                    .collect();
                let menu = Menu { full: DIALOGUE_MENU.into(), ns: None, context: "DIALOGUE".into(), is_final: true, width: None, height: None, items: rows, verbs: false };
                let mut layer = menu::Layer::open(&mut self.r.w, menu);
                for (row, item) in layer.rows.iter_mut().zip(&items) {
                    row.enabled = item.enabled;
                }
                layer.cursor = cursor.filter(|&c| c < layer.rows.len()).or_else(|| layer.rows.iter().position(|r| r.enabled)).unwrap_or(0);
                self.menus.layers.push(layer);
            }
            Some(Pause::Open { name, .. }) if self.scene_menu.is_none() => {
                self.menus.layers.retain(|l| l.menu.full != DIALOGUE_MENU);
                self.open(&name, false);
                self.scene_menu = Some(name);
            }
            Some(Pause::Open { .. }) => {}
            _ => self.menus.layers.retain(|l| l.menu.full != DIALOGUE_MENU),
        }
        Ok(())
    }

    /// A pad press at time `now` (M 8.2).
    pub fn pad(&mut self, id: usize, p: PadPress, now: Duration) -> RtResult<()> {
        self.typed_out(now);
        self.last_key = now;
        self.clock = now;
        self.note = None;
        self.pad_connected(id);
        self.press(Device::Pad(id), Press::Pad(p), now)?;
        self.sync_scene()
    }

    /// A press from a device, routed by M 8.2: `ALWAYS` from every device,
    /// a menu from the device that opened it (keyboard parts count as one
    /// there), the game's context on a turn from the awaited seat's device
    /// only; other presses are dropped.
    fn press(&mut self, device: Device, press: Press, now: Duration) -> RtResult<()> {
        self.last_device = device;
        if self.joining.is_some() {
            return self.join_press(device, &press);
        }
        // `join` from a free device adds a seat mid-game (M 8.2); in a menu,
        // a dialogue or a scene the press is dropped
        if self.playing
            && !self.menus.is_open()
            && !self.seat_devices.values().any(|d| *d == device)
            && matches!(self.input.keys.lookup_on(&self.contexts(), device, &press), Some(Action::Engine(a)) if a == "join")
        {
            if let Some(n) = self.r.join()? {
                self.seat_devices.insert(n, device);
                self.chosen_devices = true;
            }
            return self.advance(0);
        }
        let always = self.input.keys.is_always(device, &press);
        // a key closes a held panel (D 11.5)
        if self.overlay.take().is_some() {
            return Ok(());
        }
        // the reading view takes its keys by `MENU`'s bindings, which win
        // over `ALWAYS`'s, so `esc` closes it rather than pausing (M 7.4)
        if self.reading.is_some() {
            match self.input.keys.lookup_on(&["MENU".to_string()], device, &press).cloned() {
                Some(Action::Engine(a)) => {
                    self.reading_action(&a);
                    return Ok(());
                }
                _ if !always => return Ok(()),
                _ => self.reading = None,
            }
        }
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
            Effect::Read(log) => self.read_log(&log),
            Effect::Tab(t) => self.select_tab(&t),
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
        // a panel opens modal, scrolled like the reading view (D 11.5)
        if self.m.kind(menu) == Some(rlml_core::model::Kind::Panel) {
            self.reading = Some(Reading { log: menu.to_string(), back: 0, page: 1, panel: true, arrows: (None, None) });
            return;
        }
        if !self.menus.open(&mut self.r.w, menu, stack) {
            self.note = Some(format!("{menu}: list menus and panels are not supported yet"));
        }
    }

    /// An engine action from a key or a menu item (M 8, M 9.1).
    fn engine(&mut self, a: &str, now: Duration) -> RtResult<()> {
        // an `adjust` item moves with left and right (RS 9.7); a dialogue
        // has no `back`
        if let Some(layer) = self.menus.layers.last().filter(|l| l.menu.full == DIALOGUE_MENU) {
            let item = layer.rows.get(layer.cursor).map(|r| layer.menu.items[r.item].key.clone());
            match (a, item) {
                ("cursor.west" | "cursor.east", Some(item)) => {
                    self.r.scene_adjust(&item, if a == "cursor.east" { 1 } else { -1 })?;
                    return self.sync_scene();
                }
                ("back" | "close", _) => return Ok(()),
                _ => {}
            }
        }
        // a scene's own actions (RS 9.8): `rewind` comes with timelines
        if self.r.in_scene() && !self.menus.is_open() {
            match a {
                "confirm" => self.r.scene_confirm()?,
                "skip" => self.r.scene_skip()?,
                "restart" => self.r.scene_restart()?,
                "rewind" => self.note = Some("`rewind` comes with timelines".into()),
                _ => return self.engine_action(a, now),
            }
            return self.advance(0);
        }
        self.engine_action(a, now)
    }

    fn engine_action(&mut self, a: &str, now: Duration) -> RtResult<()> {
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
                // a menu fits its window, so a page goes to its end
                "first" | "page.up" => {
                    layer.move_to_end(false);
                    return Ok(());
                }
                "last" | "page.down" => {
                    layer.move_to_end(true);
                    return Ok(());
                }
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
                // a game under way ends as by `quit` (M 8)
                self.save_on_leaving();
                // the first game takes the seed given, later ones new seeds
                let seed = if self.games > 0 { self.next_seed() } else { self.seed };
                self.start(seed)?;
                self.last_round = now;
            }
            "save" => self.save_game(),
            // the seat whose device opened the menu leaves (M 8.2)
            "leave_seat" if self.playing => {
                let owner = self.menu_owner.unwrap_or(self.last_device);
                let seat = self.seat_devices.iter().find(|(_, d)| **d == owner).map(|(n, _)| *n).or_else(|| self.r.w.s.seats.first().map(|s| s.n));
                if let Some(n) = seat
                    && self.r.leave_seat(n)?
                {
                    self.seat_devices.remove(&n);
                    self.menus.clear();
                    self.advance(0)?;
                }
            }
            "reset_profile" => {
                if let Some(p) = self.profile.as_mut() {
                    p.reset();
                    self.note = Some("the profile was reset".into());
                }
            }
            "load_game" => self.open_load_menu(),
            a if a.starts_with("load:") => {
                if let Some(slot) = Slot::parse(&a["load:".len()..]) {
                    self.save_on_leaving();
                    self.load(slot)?;
                    self.last_round = now;
                }
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
            "tab.next" => self.step_tab(1),
            "tab.prev" => self.step_tab(-1),
            "quit" if self.playing => {
                self.save_on_leaving();
                self.quit_game();
            }
            "quit" | "exit" => self.quit = true,
            other => self.note = Some(format!("`{other}` is not supported yet")),
        }
        Ok(())
    }

    /// `confirm` in a menu: the item under the cursor (M 7.5).
    fn choose(&mut self, now: Duration) -> RtResult<()> {
        // a dialogue's item: chosen, or an `adjust` item raised (RS 9.7)
        if let Some(layer) = self.menus.layers.last().filter(|l| l.menu.full == DIALOGUE_MENU) {
            let Some(item) = layer.chosen().map(|i| i.key.clone()) else { return Ok(()) };
            if !self.r.scene_choose(&item)? {
                self.r.scene_adjust(&item, 1)?;
            }
            self.advance(0)?;
            return self.sync_scene();
        }
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
            ItemAction::Read(log) => {
                self.menus.clear();
                self.read_log(&log);
            }
            ItemAction::Tab(t) => {
                self.menus.clear();
                self.select_tab(&t);
            }
            ItemAction::Hold(p) => {
                self.menus.clear();
                self.overlay = Some(p);
            }
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
        self.type_on(now);
        // a held button repeats at the keys' pace (M 7.7)
        if let Some((action, next)) = self.held.clone()
            && now >= next
        {
            self.held = Some((action.clone(), now + HOLD_REPEAT));
            self.typed_out(now);
            self.last_key = now;
            if !self.menus.is_open() && self.reading.is_none() {
                self.button_action(action, now)?;
            }
        }
        // the game's time stops while a menu is open or the log is read
        // (M 7.4, M 8)
        if !self.playing || self.menus.is_open() || self.reading.is_some() {
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
        if let Some((_, next)) = &self.held {
            return next.saturating_sub(self.clock).clamp(Duration::from_millis(10), HOLD_REPEAT);
        }
        if !self.playing || self.menus.is_open() || self.reading.is_some() {
            return Duration::from_millis(500);
        }
        // the teletype types on at the frames' pace (D 9.3)
        if self.baud.is_some() && self.typed < self.r.w.log.len() {
            return TYPE_PACE;
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
            ItemAction::Read(log) => {
                self.read_log(&log);
                return Ok(());
            }
            ItemAction::Tab(t) => {
                self.select_tab(&t);
                return Ok(());
            }
            ItemAction::Hold(p) => {
                self.overlay = Some(p);
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
                        let action = match &it.action {
                            ItemAction::Command(c) => Action::Command(c.clone()),
                            ItemAction::Engine(a) => Action::Engine(a.clone()),
                            ItemAction::Open { menu, stack } => Action::Open { menu: menu.clone(), stack: *stack },
                            ItemAction::Read(l) => Action::Read(l.clone()),
                            ItemAction::Tab(t) => Action::Tab(t.clone()),
                            ItemAction::Hold(p) => Action::Engine(format!("hold {p}")),
                        };
                        let key = self.input.keys.shortcut_for(&contexts, device, &action).unwrap_or_default();
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
        self.mouse_press_on(b, x, y, now)?;
        self.sync_scene()
    }

    fn mouse_press_on(&mut self, b: Binding, x: usize, y: usize, now: Duration) -> RtResult<()> {
        if !self.mouse {
            return Ok(());
        }
        self.pointer = Some((x, y));
        self.typed_out(now);
        self.last_key = now;
        self.clock = now;
        // a tap or a click confirms a scene's line or gives its field's
        // answer (M 7.4)
        if self.r.in_scene() && !self.menus.is_open() && b == Binding::mouse(termplate_core::keys::Mouse::Click) {
            if matches!(self.r.scene_pause(), Some(rlml_core::rt::world::Pause::Ask { .. })) {
                self.field_answer()?;
            } else {
                self.r.scene_confirm()?;
                self.advance(0)?;
            }
            return self.sync_scene();
        }
        // a held panel shows until the button goes up, or the next click
        if self.overlay.is_some() && !self.releases {
            self.overlay = None;
            return Ok(());
        }
        // in the reading view the wheel scrolls and a click closes (M 7.4)
        if self.reading.is_some() {
            use termplate_core::keys::Mouse;
            match b {
                Binding::Mouse(Mouse::WheelUp, _) => self.scroll_reading(3),
                Binding::Mouse(Mouse::WheelDown, _) => self.scroll_reading(-3),
                Binding::Mouse(Mouse::Click, _) => {
                    // the scroll buttons scroll half a view, elsewhere a
                    // click closes (M 7.4)
                    let (up, down) = self.reading.as_ref().map_or((None, None), |r| r.arrows);
                    let half = self.reading.as_ref().map_or(1, |r| (r.page / 2).max(1)) as isize;
                    match (up.filter(|r| r.contains(x, y)), down.filter(|r| r.contains(x, y))) {
                        (Some(r), _) => {
                            self.scroll_reading(half);
                            self.view.pressed = self.releases.then_some(r);
                        }
                        (_, Some(r)) => {
                            self.scroll_reading(-half);
                            self.view.pressed = self.releases.then_some(r);
                        }
                        _ => self.reading = None,
                    }
                }
                Binding::Mouse(Mouse::RightClick, _) => self.reading = None,
                _ => {}
            }
            return Ok(());
        }
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
        // a tab title chooses its tab, at any time (D 11.1)
        let click = b == Binding::mouse(termplate_core::keys::Mouse::Click);
        if click && let Some((key, i, _)) = self.view.tab_rects.iter().find(|(_, _, r)| r.contains(x, y)).cloned() {
            self.view.tabs.insert(key.clone(), i);
            self.view.last_tabs = Some(key);
            return Ok(());
        }
        // a click on a `read` log opens it to be read (D 11.4)
        if click && let Some((_, log, _)) = self.view.read_rects.iter().find(|(r, _, _)| r.contains(x, y)).cloned() {
            self.read_log(&log);
            return Ok(());
        }
        // a panel's button does its action, and held it repeats (M 7.7)
        if click && let Some((cells, action)) = self.view.buttons.iter().find(|(r, _)| r.contains(x, y)).cloned() {
            if self.releases {
                self.view.pressed = Some(cells);
            }
            if self.releases && !matches!(action, ItemAction::Hold(_) | ItemAction::Open { .. }) {
                self.held = Some((action.clone(), now + HOLD_DELAY));
            }
            return self.button_action(action, now);
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
            Some(Action::Read(log)) => self.read_log(&log),
            Some(Action::Tab(t)) => self.select_tab(&t),
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
