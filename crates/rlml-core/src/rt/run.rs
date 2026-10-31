//! The scheduler (M 6.1): microsteps and rounds, the wake queue and the
//! course of a turn.
//!
//! Time moves only on the active map (M 6.1.8), and sparsely: the
//! scheduler jumps to the next moment that has something, a wake or a
//! round boundary (M 6.1.1). At a moment the phases run in a fixed order:
//! the round (only on a boundary, not at 0), then the wakes by moment and
//! sequence number (M 6.1.4). Timers and background scenes are not
//! supported yet.

use std::collections::BTreeMap;

use super::eval::{BodyKind, Env, Outcome, Scope, Suspended};
use super::value::{CommandValue, EntityId, MapId, Value};
use super::world::{DEFAULT_COST, RtResult, World, delay};

/// What the scheduler waits for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    /// The player's turn: a command is needed (M 6.1.5 step 2).
    Input,
    /// There is no player, or the player has no wake; `rounds` rounds went
    /// by (M 6.1.2).
    NoPlayer { rounds: i64 },
}

pub struct Runner<'m> {
    pub w: World<'m>,
    /// Commands waiting at `wait`, by actor (M 6.1.6).
    pub waiting: BTreeMap<EntityId, Suspended>,
    /// The player's `wake` has been sent for this turn.
    woke: bool,
}

fn engine_env(name: &str) -> Env {
    Env::new(BodyKind::Handler, name, None, "<engine>")
}

impl<'m> Runner<'m> {
    pub fn new(w: World<'m>) -> Runner<'m> {
        Runner { w, waiting: BTreeMap::new(), woke: false }
    }

    /// The moment on a map's clock now.
    fn clock(&self, map: MapId) -> i64 {
        self.w.s.maps[map].clock(self.w.s.game.time)
    }

    /// Runs until the player's turn. Without a player, at most `rounds`
    /// rounds go by (M 6.1.2: the world goes on at the `round` pace).
    pub fn run(&mut self, rounds: i64) -> RtResult<Status> {
        self.run_with(rounds, &mut |_| {})
    }

    /// `run`, calling `seen` after every actor's turn and every round, for
    /// the presentation's in-between frames (M 7.2).
    pub fn run_with(&mut self, rounds: i64, seen: &mut dyn FnMut(&mut World)) -> RtResult<Status> {
        let start_turn = self.w.s.game.turn();
        loop {
            let Some(map) = self.w.s.active_map() else { return Ok(Status::NoPlayer { rounds: 0 }) };
            let now = self.clock(map);
            // phase 4: the wakes of this moment, by sequence number
            if let Some(wake) = self.w.s.next_wake(map)
                && wake.at <= now
            {
                if Some(wake.entity) == self.w.s.player && !self.waiting.contains_key(&wake.entity) {
                    if !self.woke {
                        self.woke = true;
                        self.wake_event(wake.entity)?;
                        // `wake` may end the turn: the player died or stopped acting
                        if self.w.s.wake_of(wake.entity) != Some(wake) {
                            self.woke = false;
                            continue;
                        }
                    }
                    return Ok(Status::Input);
                }
                self.turn(wake.entity)?;
                seen(&mut self.w);
                continue;
            }
            // nothing more at this moment: the active map changes (M 6.7)
            if self.w.switch.is_some() {
                self.w.apply_switch();
                continue;
            }
            let played = self.w.s.game.turn() - start_turn;
            let has_player = self.w.s.player.is_some_and(|p| self.w.s.get(p).alive && self.w.s.wake_of(p).is_some());
            if !has_player && played >= rounds {
                return Ok(Status::NoPlayer { rounds: played });
            }
            // the next moment: the nearest wake or round boundary (M 6.1.1)
            let boundary = (self.w.s.game.time.div_euclid(120) + 1) * 120;
            let next_wake = self.w.s.next_wake(map).map(|w| self.w.s.game.time + (w.at - now));
            let next = next_wake.map_or(boundary, |t| t.min(boundary));
            self.w.s.game.time = next;
            if next == boundary {
                self.round()?;
                seen(&mut self.w);
            }
        }
    }

    /// Phase 1, the round: `turn` to the active map's instances in creation
    /// order, then once to the general handlers with `game` as subject
    /// (M 6.1.4).
    fn round(&mut self) -> RtResult<()> {
        let Some(map) = self.w.s.active_map() else { return Ok(()) };
        if self.w.s.entities_on(map).iter().any(|&e| self.w.s.get(e).has("timer")) {
            return Err("timers are not supported yet (M 6.1.4 phase 2)".into());
        }
        let mut env = engine_env("turn");
        env.map = Some(map);
        for e in self.w.s.entities_on(map) {
            self.w.emit_to("turn", vec![Value::Entity(e)], &env, Scope::Components)?;
        }
        self.w.emit_to("turn", vec![], &env, Scope::General)?;
        self.end_action()
    }

    /// The start of a wake: `hear` for new sounds, then `wake` (M 6.8,
    /// M 6.1.5).
    fn wake_event(&mut self, e: EntityId) -> RtResult<()> {
        self.w.hear_events(e)?;
        if !self.w.s.get(e).alive {
            return Ok(());
        }
        let mut env = engine_env("wake");
        env.map = self.w.s.at(e).map(|p| p.map);
        self.w.emit("wake", vec![Value::Entity(e)], &env)
    }

    fn end_action(&mut self) -> RtResult<()> {
        self.w.end_action()?;
        let env = engine_env("<action end>");
        self.w.flush_created(&env)
    }

    /// An actor's turn that is not the player's choice (M 6.1.5): `wake`,
    /// the ai's choice or the waiting command, the run and the next wake.
    fn turn(&mut self, e: EntityId) -> RtResult<()> {
        self.w.s.unschedule(e);
        if !self.w.s.get(e).alive {
            self.waiting.remove(&e);
            return Ok(());
        }
        self.wake_event(e)?;
        // the actor may have died or stopped acting in `wake`
        let is_player = self.w.s.player == Some(e);
        if !self.w.s.get(e).alive || (!is_player && !self.w.s.get(e).has("ai")) || self.w.s.at(e).is_none() {
            self.waiting.remove(&e);
            return self.end_action();
        }
        let outcome = match self.waiting.remove(&e) {
            Some(state) => self.w.continue_command(e, state, true)?,
            None => {
                let ai = self.w.s.get(e).get("ai").and_then(|c| c.get("behavior")).cloned();
                let choice = match ai {
                    Some(Value::Name(behavior)) => self.w.run_ai(&behavior, e)?,
                    _ => Value::None,
                };
                match choice {
                    Value::Command(cmd) => self.w.start_command(e, &cmd)?,
                    // `none` rests for the default cost (M 6.1.5)
                    _ => Outcome::Done { cost: DEFAULT_COST },
                }
            }
        };
        self.finish(e, outcome, is_player)?;
        self.end_action()
    }

    /// Schedules after a command (M 6.1.5): success waits for the cost's
    /// delay, an ai's failure loses the turn, a wait comes back later.
    fn finish(&mut self, e: EntityId, outcome: Outcome, player: bool) -> RtResult<bool> {
        let Some(at) = self.w.s.at(e) else { return Ok(true) };
        if !self.w.s.get(e).alive {
            return Ok(true);
        }
        let now = self.clock(at.map);
        let speed = self.w.speed(e);
        Ok(match outcome {
            Outcome::Done { cost } => {
                if player && cost == 0 {
                    // a free action: the player acts again now, without a new wake
                    self.w.s.schedule(e, now);
                    self.woke = true;
                    return Ok(true);
                }
                if let Some(d) = delay(cost, speed) {
                    self.w.s.schedule(e, now + d);
                }
                true
            }
            Outcome::Failed { .. } if player => {
                // the player's failure costs nothing; time already waited
                // stays spent (M 6.1.5)
                self.w.s.schedule(e, now);
                self.woke = true;
                false
            }
            Outcome::Failed { .. } => {
                if let Some(d) = delay(DEFAULT_COST, speed) {
                    self.w.s.schedule(e, now + d);
                }
                false
            }
            Outcome::Waiting { delay, state } => {
                self.waiting.insert(e, *state);
                self.w.s.schedule(e, now + delay);
                true
            }
        })
    }

    /// The player's command on its turn. Returns whether it succeeded; a
    /// failure takes no time, and the player is asked again (M 6.1.5).
    pub fn player(&mut self, cmd: &CommandValue) -> RtResult<bool> {
        let p = self.w.s.player.ok_or("no player")?;
        let Some(wake) = self.w.s.wake_of(p) else { return Err("the player has no turn now".into()) };
        let map = self.w.s.at(p).ok_or("the player is not on a map")?.map;
        if wake.at > self.clock(map) {
            return Err("it is not the player's turn".into());
        }
        self.w.s.unschedule(p);
        self.woke = false;
        let outcome = self.w.start_command(p, cmd)?;
        let ok = self.finish(p, outcome, true)?;
        self.end_action()?;
        Ok(ok)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delays_follow_m_6_1_3() {
        assert_eq!(delay(120, 10), Some(120));
        assert_eq!(delay(120, 15), Some(80));
        assert_eq!(delay(120, 7), Some(172), "rounded up");
        assert_eq!(delay(0, 10), Some(1), "at least 1");
        assert_eq!(delay(120, 0), None, "speed 0: no wake");
    }
}
