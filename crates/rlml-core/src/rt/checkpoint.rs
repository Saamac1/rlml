//! Checkpoints in memory (M 5.3, RS 10.2): `checkpoint()` takes a
//! snapshot at the end of the action that called it, and `rollback()`
//! loads the last one there. Checkpoint 0 is the state after `begin`.
//!
//! A snapshot holds the game's state (M 3.1) and what the runner keeps
//! between actions: the stream, sounds and hearing, the last shared view,
//! the waiting commands and the wakes already announced. The log's
//! messages are presentation and stay as they are, and so do travels and
//! orders, which are dropped (M 8.1).

use std::collections::{BTreeMap, BTreeSet};

use super::eval::Suspended;
use super::rng::Rng;
use super::run::Runner;
use super::state::{Seat, State, Wake};
use super::value::{EntityId, MapId, Sound};
use super::world::{Noise, Point};

#[derive(Debug, Clone, PartialEq)]
pub struct Snapshot {
    pub s: State,
    pub rng: Rng,
    pub(crate) sounds: Vec<Noise>,
    pub(crate) sounds_made: u64,
    pub(crate) heard_mark: BTreeMap<EntityId, u64>,
    pub(crate) heard: BTreeMap<EntityId, Vec<Sound>>,
    pub(crate) last_view: Option<(MapId, BTreeSet<(i32, i32)>)>,
    pub(crate) switch: Option<(MapId, MapId)>,
    pub(crate) waiting: BTreeMap<EntityId, Suspended>,
    pub(crate) woke: BTreeMap<EntityId, Wake>,
}

impl<'m> Runner<'m> {
    /// The game as it is now, between actions.
    pub fn snapshot(&self) -> Snapshot {
        let w = &self.w;
        Snapshot {
            s: w.s.clone(),
            rng: w.rng.clone(),
            sounds: w.sounds.clone(),
            sounds_made: w.sounds_made,
            heard_mark: w.heard_mark.clone(),
            heard: w.heard.clone(),
            last_view: w.last_view.clone(),
            switch: w.switch,
            waiting: self.waiting.clone(),
            woke: self.woke.clone(),
        }
    }

    /// Loads `snap`. Seats joined after it stay joined and empty, and
    /// devices do not change (M 5.3).
    pub fn restore(&mut self, snap: &Snapshot) {
        let later: Vec<Seat> = self.w.s.seats.iter().filter(|s| snap.s.seat(s.n).is_none()).map(|s| Seat { n: s.n, ..Seat::default() }).collect();
        let w = &mut self.w;
        // profile writes stay: `rollback` does not undo them (M 5.6)
        let profile = std::mem::take(&mut w.s.profile);
        w.s = snap.s.clone();
        w.s.profile = profile;
        for seat in later {
            let i = w.s.seats.iter().position(|s| s.n > seat.n).unwrap_or(w.s.seats.len());
            w.s.seats.insert(i, seat);
        }
        w.rng = snap.rng.clone();
        w.sounds = snap.sounds.clone();
        w.sounds_made = snap.sounds_made;
        w.heard_mark = snap.heard_mark.clone();
        w.heard = snap.heard.clone();
        w.last_view = snap.last_view.clone();
        w.switch = snap.switch;
        self.waiting = snap.waiting.clone();
        self.woke = snap.woke.clone();
        self.travels.clear();
        self.orders.clear();
        self.awaiting = None;
    }

    /// The action end's steps 2–4 (M 6.1.7): `rollback` or `checkpoint`,
    /// the last of them called deciding, then `delete_save(checkpoint)`.
    /// A rollback loads its own state, so the rest is skipped.
    pub(crate) fn save_calls(&mut self) {
        let calls = std::mem::take(&mut self.w.saves);
        let checkpoint = calls.point == Some(Point::Checkpoint);
        match calls.point {
            Some(Point::Rollback) => match self.checkpoint.clone() {
                Some(snap) => {
                    self.restore(&snap);
                    return;
                }
                None => self.w.warnings.push("`rollback()` without a checkpoint does nothing: `delete_save` removed it (M 5.5)".into()),
            },
            Some(Point::Checkpoint) => self.checkpoint = Some(Box::new(self.snapshot())),
            None => {}
        }
        if calls.delete_checkpoint {
            self.checkpoint = None;
        }
        if calls.delete_auto {
            self.delete_auto = true;
            self.auto_off = true;
        }
        // step 5: one automatic save for every request of the action
        let rules = &self.rules;
        let due = calls.autosave || checkpoint && rules.on_checkpoint || calls.arrived && rules.on_arrive;
        if due && !self.auto_off {
            self.due.push(self.save());
        }
    }
}
