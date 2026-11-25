//! The join screen, context `JOIN` (M 8.2): which device plays which
//! seat before a game of more than one seat starts or a save of one goes
//! on.
//!
//! * New game: the device that chose `new_game` is seat 1, and other
//!   devices join with `join` and get the next numbers. `back` from a
//!   joined device takes its seat away, and `confirm` (the same key as
//!   `join` on a joined device) starts the game once at least the game's
//!   `seats` least have joined.
//! * A save: the screen asks a device for each saved seat in number order.
//!   `skip`, or `confirm` before a seat has one, leaves that seat out: it
//!   leaves the game as by `part` before the first input. New seats may
//!   join after the saved ones, up to the game's most.

use rlml_core::rt::save::Save;
use rlml_core::rt::state::SeatId;
use rlml_core::rt::world::World;

use crate::input::Device;

/// A seat on the screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Slot {
    /// A saved seat that waits for a device.
    Waiting,
    /// A saved seat that was skipped: it leaves the game.
    Skipped,
    Taken(Device),
}

#[derive(Debug, Clone)]
pub struct JoinScreen {
    /// The seats by number: a new game's from 1, a save's saved seats and
    /// then the new ones.
    pub seats: Vec<(SeatId, Slot)>,
    /// The save going on, or `None` for a new game.
    pub load: Option<Save>,
    /// The new game's seed.
    pub seed: u64,
    pub least: u8,
    pub most: u8,
}

/// What a press on the screen does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JoinStep {
    /// Nothing to do yet.
    Stay,
    /// The screen closes with no game: every seat left a new game.
    Cancel,
    /// The game starts with these seats.
    Start,
}

impl JoinScreen {
    /// A new game's screen, with `first` as seat 1.
    pub fn new_game(seed: u64, (least, most): (u8, u8), first: Device) -> JoinScreen {
        JoinScreen {
            seats: vec![(1, Slot::Taken(first))],
            load: None,
            seed,
            least,
            most,
        }
    }

    /// A save's screen: each saved seat waits for a device.
    pub fn load(save: Save, (least, most): (u8, u8)) -> JoinScreen {
        let seats = save
            .now
            .s
            .seats
            .iter()
            .map(|s| (s.n, Slot::Waiting))
            .collect();
        JoinScreen {
            seats,
            load: Some(save),
            seed: 0,
            least,
            most,
        }
    }

    fn holds(&self, device: Device) -> Option<usize> {
        self.seats
            .iter()
            .position(|(_, s)| *s == Slot::Taken(device))
    }

    fn taken(&self) -> usize {
        self.seats
            .iter()
            .filter(|(_, s)| matches!(s, Slot::Taken(_)))
            .count()
    }

    /// `join` from `device`: a free device takes the next waiting saved
    /// seat or a new one; a joined device confirms.
    pub fn join(&mut self, device: Device) -> JoinStep {
        if self.holds(device).is_some() {
            return self.confirm();
        }
        if let Some(i) = self.seats.iter().position(|(_, s)| *s == Slot::Waiting) {
            self.seats[i].1 = Slot::Taken(device);
        } else if self
            .seats
            .iter()
            .filter(|(_, s)| *s != Slot::Skipped)
            .count()
            < self.most as usize
        {
            let n = (1..=self.most)
                .find(|n| !self.seats.iter().any(|(m, _)| m == n))
                .unwrap_or(self.most);
            self.seats.push((n, Slot::Taken(device)));
            self.seats.sort_by_key(|(n, _)| *n);
        }
        JoinStep::Stay
    }

    /// `back` from `device`: its seat goes. A new game's seats are
    /// numbered again from 1; a saved seat waits again.
    pub fn back(&mut self, device: Device) -> JoinStep {
        let Some(i) = self.holds(device) else {
            return JoinStep::Stay;
        };
        let saved = self
            .load
            .as_ref()
            .is_some_and(|s| s.now.s.seat(self.seats[i].0).is_some());
        if saved {
            self.seats[i].1 = Slot::Waiting;
            return JoinStep::Stay;
        }
        self.seats.remove(i);
        if self.load.is_none() {
            for (k, (n, _)) in self.seats.iter_mut().enumerate() {
                *n = k as SeatId + 1;
            }
            if self.seats.is_empty() {
                return JoinStep::Cancel;
            }
        }
        JoinStep::Stay
    }

    /// `skip`: the next waiting saved seat leaves.
    pub fn skip(&mut self) -> JoinStep {
        if let Some(i) = self.seats.iter().position(|(_, s)| *s == Slot::Waiting) {
            self.seats[i].1 = Slot::Skipped;
        }
        JoinStep::Stay
    }

    /// `confirm`: the game starts once enough seats have a device; saved
    /// seats still waiting are skipped.
    pub fn confirm(&mut self) -> JoinStep {
        if self.taken() < self.least.max(1) as usize {
            return JoinStep::Stay;
        }
        for (_, s) in self.seats.iter_mut() {
            if *s == Slot::Waiting {
                *s = Slot::Skipped;
            }
        }
        JoinStep::Start
    }

    /// The screen's lines: a title, a line a seat and the keys' hint.
    pub fn lines(&self, w: &World, lang: &str) -> Vec<String> {
        let text = |key: &str, n: Option<usize>, fallback: &str| {
            let args: Vec<(String, rlml_core::rt::value::Value)> = n
                .map(|n| {
                    (
                        "N".to_string(),
                        rlml_core::rt::value::Value::Number(n as i64),
                    )
                })
                .into_iter()
                .collect();
            if w.m.has_text(lang, key) {
                w.text(lang, key, &args)
            } else {
                fallback.replace("{N}", &n.map(|n| n.to_string()).unwrap_or_default())
            }
        };
        let mut out = vec![
            text("ENGINE.JOIN.TITLE", None, "Join the game"),
            String::new(),
        ];
        for (n, slot) in &self.seats {
            let what = match slot {
                Slot::Taken(Device::Keyboard(p)) => {
                    text("ENGINE.JOIN.KEYBOARD", Some(*p as usize), "keyboard {N}")
                }
                Slot::Taken(Device::Pad(p)) => text("ENGINE.JOIN.PAD", Some(*p + 1), "pad {N}"),
                Slot::Waiting => text("ENGINE.JOIN.WAITING", None, "waiting for a device"),
                Slot::Skipped => text("ENGINE.JOIN.SKIPPED", None, "skipped"),
            };
            out.push(format!(
                "{}  {what}",
                text("ENGINE.JOIN.SEAT", Some(*n as usize), "Seat {N}")
            ));
        }
        out.push(String::new());
        out.push(text(
            "ENGINE.JOIN.HINT",
            None,
            "Join from a free device, start with confirm",
        ));
        out
    }
}
