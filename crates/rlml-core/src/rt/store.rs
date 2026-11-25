//! Save slots (M 5.4, D 3.4): where saves go, which are kept and when the
//! engine saves on its own.
//!
//! The core does not use the file system: saves go to a [`Store`], which
//! the front end gives (a folder, the browser's storage, or [`MemStore`]
//! in tests). [`Saves`] keeps the slots by the game's `autosave` section:
//!
//! * `slots 0`: no automatic saves; the player's own saves have `manual`
//!   slots and `quit` one slot of its own.
//! * `slots 1`: permadeath, one slot every save replaces; the player's own
//!   save goes there and ends the game.
//! * `slots N`: a stack of N automatic saves, the newest on top and the
//!   oldest dropping off, with the player's own slots beside it.
//!
//! A save's stamp orders the slots by when they were written, so loading
//! an older save and going on leaves the newer ones in the stack.

use std::collections::BTreeMap;

use super::run::Runner;
use super::save::Save;
use super::world::RtResult;
use crate::model::Model;

/// Where saves are kept, by name. A write replaces the whole save at once:
/// a crash in the middle leaves the old one (M 5.4).
pub trait Store {
    fn read(&self, name: &str) -> Option<String>;
    fn write(&mut self, name: &str, text: &str) -> Result<(), String>;
    fn remove(&mut self, name: &str);
    fn list(&self) -> Vec<String>;
}

/// Saves in memory, for tests and front ends without storage.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MemStore(pub BTreeMap<String, String>);

impl Store for MemStore {
    fn read(&self, name: &str) -> Option<String> {
        self.0.get(name).cloned()
    }

    fn write(&mut self, name: &str, text: &str) -> Result<(), String> {
        self.0.insert(name.to_string(), text.to_string());
        Ok(())
    }

    fn remove(&mut self, name: &str) {
        self.0.remove(name);
    }

    fn list(&self) -> Vec<String> {
        self.0.keys().cloned().collect()
    }
}

impl<S: Store + ?Sized> Store for Box<S> {
    fn read(&self, name: &str) -> Option<String> {
        (**self).read(name)
    }

    fn write(&mut self, name: &str, text: &str) -> Result<(), String> {
        (**self).write(name, text)
    }

    fn remove(&mut self, name: &str) {
        (**self).remove(name)
    }

    fn list(&self) -> Vec<String> {
        (**self).list()
    }
}

/// The game's `autosave` section (D 3.4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutosaveRules {
    pub slots: u32,
    pub manual: u32,
    /// `quit save`: leaving the game saves it.
    pub quit_save: bool,
    /// `on input every N`; `on input` alone is every 1.
    pub on_input: Option<u32>,
    pub on_scene: bool,
    pub on_checkpoint: bool,
    pub on_arrive: bool,
}

impl Default for AutosaveRules {
    fn default() -> AutosaveRules {
        AutosaveRules { slots: 0, manual: 3, quit_save: true, on_input: None, on_scene: false, on_checkpoint: false, on_arrive: false }
    }
}

impl AutosaveRules {
    /// The rules from the data; without the section there is no automatic
    /// saving, and the player's saves and leaving keep the defaults.
    pub fn read(m: &Model) -> AutosaveRules {
        let mut r = AutosaveRules::default();
        for (_, item) in m.section_rows("autosave") {
            let (_, item) = item.marker();
            let num = |i: usize| item.word(i).and_then(|w| w.parse::<u32>().ok());
            match (item.key(), item.word(1)) {
                ("slots", _) => r.slots = num(1).unwrap_or(0),
                ("manual", _) => r.manual = num(1).unwrap_or(3),
                ("quit", w) => r.quit_save = w != Some("none"),
                ("on", Some("input")) => r.on_input = Some(if item.word(2) == Some("every") { num(3).unwrap_or(1).max(1) } else { 1 }),
                ("on", Some("scene")) => r.on_scene = true,
                ("on", Some("checkpoint")) => r.on_checkpoint = true,
                ("on", Some("arrive")) => r.on_arrive = true,
                _ => {}
            }
        }
        r
    }
}

/// A save's place.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Slot {
    /// The automatic stack's slot, from 1.
    Auto(u32),
    /// The player's own slot, from 1.
    Manual(u32),
    /// `slots 0`: leaving's own slot.
    Quit,
    /// `slots 1`: the only slot.
    Only,
}

impl Slot {
    pub fn name(self) -> String {
        match self {
            Slot::Auto(n) => format!("auto-{n}"),
            Slot::Manual(n) => format!("manual-{n}"),
            Slot::Quit => "quit".into(),
            Slot::Only => "game".into(),
        }
    }

    pub fn parse(name: &str) -> Option<Slot> {
        let num = |p: &str| name.strip_prefix(p).and_then(|n| n.parse().ok()).filter(|n| *n > 0);
        match name {
            "quit" => Some(Slot::Quit),
            "game" => Some(Slot::Only),
            _ => num("auto-").map(Slot::Auto).or_else(|| num("manual-").map(Slot::Manual)),
        }
    }
}

/// A save in a slot, as a list of saves shows it.
#[derive(Debug, Clone, PartialEq)]
pub struct Listed {
    pub slot: Slot,
    pub save: Save,
}

/// What the player's own save did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Saved {
    /// Saved in this slot.
    In(Slot),
    /// `slots 1`: saved in the only slot, and the game ends (M 5.4).
    AndEnds,
}

/// The game's save slots in a store.
pub struct Saves<S: Store> {
    pub store: S,
    pub rules: AutosaveRules,
    /// The session the saves now made belong to: `delete_save(auto)`
    /// removes its automatic saves.
    pub session: u64,
    /// `delete_save(auto)` ended automatic saving for this session.
    pub auto_off: bool,
}

impl<S: Store> Saves<S> {
    pub fn new(store: S, rules: AutosaveRules) -> Saves<S> {
        let mut s = Saves { store, rules, session: 0, auto_off: false };
        s.session = s.next_stamp();
        s
    }

    /// Every save in the store, newest first.
    pub fn list(&self) -> Vec<Listed> {
        let mut out: Vec<Listed> = self
            .store
            .list()
            .into_iter()
            .filter_map(|name| {
                let slot = Slot::parse(&name)?;
                let save = Save::read(&self.store.read(&name)?).ok()?;
                Some(Listed { slot, save })
            })
            .collect();
        out.sort_by_key(|l| std::cmp::Reverse(l.save.stamp));
        out
    }

    fn next_stamp(&self) -> u64 {
        self.list().iter().map(|l| l.save.stamp).max().map_or(1, |s| s + 1)
    }

    /// A new game starts a session of its own, with the data's `slots`
    /// locked for it (M 5.4).
    pub fn new_session(&mut self, m: &Model) {
        self.rules = AutosaveRules::read(m);
        self.session = self.next_stamp();
        self.auto_off = false;
    }

    fn put(&mut self, slot: Slot, mut save: Save) -> RtResult<()> {
        save.stamp = self.next_stamp();
        save.session = self.session;
        save.slots = self.rules.slots;
        self.store.write(&slot.name(), &save.write())
    }

    /// The slot of `kind` to write: the first free one, or the oldest.
    fn slot_in(&self, count: u32, kind: fn(u32) -> Slot) -> Slot {
        let list = self.list();
        let taken = |s: Slot| list.iter().find(|l| l.slot == s);
        (1..=count)
            .map(kind)
            .find(|&s| taken(s).is_none())
            .unwrap_or_else(|| (1..=count).map(kind).min_by_key(|&s| taken(s).map_or(0, |l| l.save.stamp)).unwrap_or(kind(1)))
    }

    /// An automatic save the runner took at an action's end (M 5.4).
    pub fn autosave(&mut self, save: Save) -> RtResult<()> {
        if self.auto_off {
            return Ok(());
        }
        match self.rules.slots {
            0 => Ok(()),
            1 => self.put(Slot::Only, save),
            n => {
                let slot = self.slot_in(n, Slot::Auto);
                self.put(slot, save)
            }
        }
    }

    /// The player's own save (`save`, M 5.2). A `nosave` map and a scene
    /// stop it before it gets here.
    pub fn manual(&mut self, r: &Runner) -> RtResult<Saved> {
        if self.rules.slots == 1 {
            self.put(Slot::Only, r.save())?;
            return Ok(Saved::AndEnds);
        }
        if self.rules.manual == 0 {
            return Err("this game has no saves of the player's own (`manual 0`)".into());
        }
        let slot = self.slot_in(self.rules.manual, Slot::Manual);
        self.put(slot, r.save())?;
        Ok(Saved::In(slot))
    }

    /// Leaving the game (`quit`, `new_game`): saves by the `quit` row.
    pub fn quit(&mut self, r: &Runner) -> RtResult<Option<Slot>> {
        if !self.rules.quit_save || r.w.save_blocked() {
            return Ok(None);
        }
        match self.rules.slots {
            0 => self.put(Slot::Quit, r.save()).map(|_| Some(Slot::Quit)),
            // after `delete_save(auto)`, leaving a permadeath game saves nothing
            1 if self.auto_off => Ok(None),
            1 => self.put(Slot::Only, r.save()).map(|_| Some(Slot::Only)),
            n => {
                let slot = self.slot_in(n, Slot::Auto);
                self.put(slot, r.save()).map(|_| Some(slot))
            }
        }
    }

    /// `delete_save(auto)`: this session's automatic saves go, and no more
    /// are made (M 5.5).
    pub fn delete_auto(&mut self) {
        for l in self.list() {
            if matches!(l.slot, Slot::Auto(_) | Slot::Only) && l.save.session == self.session {
                self.store.remove(&l.slot.name());
            }
        }
        self.auto_off = true;
    }

    /// The save in `slot`, made the session's: its saves go on from it,
    /// with its locked `slots`.
    pub fn load(&mut self, m: &Model, slot: Slot) -> RtResult<Save> {
        let text = self.store.read(&slot.name()).ok_or_else(|| format!("no save in {}", slot.name()))?;
        let save = Save::read(&text)?;
        // the `on` rows come from the data, `slots` from the session
        self.rules = AutosaveRules::read(m);
        self.session = save.session;
        self.rules.slots = save.slots;
        self.auto_off = false;
        Ok(save)
    }
}
