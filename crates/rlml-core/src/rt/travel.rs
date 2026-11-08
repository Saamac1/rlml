//! Travel by click, following and orders (M 8.1, M 8.2).
//!
//! A travel is not a command and not state: it lives in the runner and
//! drives the seat's turns with ordinary commands, the game's `walk` and
//! `rest` or its `follow_ai`. The front end calls `step_travel` while the
//! awaited seat travels and stops the travel on any press. A step runs as
//! the seat's input, so costs, events and failures go as with keys.

use std::collections::BTreeSet;

use super::run::Runner;
use super::state::SeatId;
use super::value::{CommandValue, Dir, EntityId, MapId, Pos, Value};
use super::world::RtResult;
use crate::model::{Kind, Model};

/// Where a travel goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Goal {
    /// `travel_to`: a cell.
    Cell(Pos),
    /// `follow`: a moving leader.
    Follow(EntityId),
}

/// A seat's travel (M 8.1).
#[derive(Debug, Clone, PartialEq)]
pub struct Travel {
    /// The member that travels; the travel ends when another one has the turn.
    pub member: EntityId,
    pub goal: Goal,
    /// The actors in the shared view at the last step.
    seen: BTreeSet<EntityId>,
    /// An order's actors when it was given: new ones cancel it before the
    /// second step.
    order: Option<BTreeSet<EntityId>>,
    /// Steps taken.
    pub steps: u32,
}

/// An order given on another seat's turn, run on the seat's next one.
#[derive(Debug, Clone, PartialEq)]
pub struct Order {
    pub goal: OrderGoal,
    seen: BTreeSet<EntityId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderGoal {
    Cell(Pos),
    /// Follow this entity, or the `leader` query's one.
    Follow(Option<EntityId>),
}

/// Why a travel ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stop {
    /// At the cell.
    Arrived,
    /// The walking command failed.
    Failed,
    /// No route through known cells.
    NoRoute,
    /// An actor came into the shared view.
    NewActor,
    /// An order went stale: an actor not seen when it was given.
    Stale,
    /// The game's `walk_stop` said so.
    Game,
    /// The leader died or is on another map.
    LeaderGone,
    /// Another member has the turn, or the seat has none.
    TurnLost,
}

/// The `game` fields of travel (D 3.3), as full names.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TravelFields {
    pub walk: Option<String>,
    pub walk_stop: Option<String>,
    pub leader: Option<String>,
    pub rest: Option<String>,
    pub follow_distance: i64,
    pub follow_ai: Option<String>,
}

impl TravelFields {
    pub fn read(m: &Model) -> TravelFields {
        let game = m.named_game();
        let ns = game.and_then(|g| g.namespace.as_deref());
        let field = |key: &str, kinds: &[Kind]| -> Option<String> {
            let name = game?.items.iter().find(|i| i.key() == key)?.word(1)?.to_string();
            m.resolve_kind(ns, &name, kinds).ok().map(|d| d.full.clone())
        };
        let distance = game.and_then(|g| g.items.iter().find(|i| i.key() == "follow_distance")).and_then(|i| i.word(1)).and_then(|w| w.parse().ok());
        TravelFields {
            walk: field("walk", &[Kind::Command]),
            walk_stop: field("walk_stop", &[Kind::Query]),
            leader: field("leader", &[Kind::Query]),
            rest: field("rest", &[Kind::Command]),
            follow_distance: distance.unwrap_or(1).max(1),
            follow_ai: field("follow_ai", &[Kind::Ai]),
        }
    }

    /// Can `follow` run: steps from the ai, or `walk` and `rest`.
    pub fn can_follow(&self) -> bool {
        self.follow_ai.is_some() || self.walk.is_some() && self.rest.is_some()
    }
}

/// The direction from `a` to the next cell `b`.
fn dir_to(a: Pos, b: Pos) -> Option<Dir> {
    Dir::ALL.into_iter().find(|&d| a.step(d) == b)
}

impl<'m> Runner<'m> {
    /// The member whose input seat `n` is awaited for now.
    fn turn_member(&self, n: SeatId) -> Option<EntityId> {
        (self.awaiting() == Some(n)).then_some(self.w.s.player?).filter(|&p| self.w.s.seat_of(p) == Some(n))
    }

    /// The actors in the shared view on `map`, members not counted (M 8.1:
    /// an actor coming into view stops a travel).
    pub fn actors_in_view(&self, map: MapId) -> BTreeSet<EntityId> {
        let Some(view) = self.w.shared_view(map) else { return BTreeSet::new() };
        self.w
            .s
            .entities_on(map)
            .into_iter()
            .filter(|&e| self.w.s.get(e).alive && self.w.s.is_actor(e) && !self.w.s.is_member(e))
            .filter(|&e| self.w.s.at(e).is_some_and(|p| view.contains(&(p.x, p.y))))
            .collect()
    }

    /// The cells the players see or remember on `map`: a route passes only
    /// these (M 8.1).
    fn known_cells(&self, map: MapId) -> BTreeSet<(i32, i32)> {
        let mut known: BTreeSet<(i32, i32)> = self.w.s.maps[map].remembered.iter().copied().collect();
        if let Some(view) = self.w.shared_view(map) {
            known.extend(view);
        }
        known
    }

    fn new_travel(&self, member: EntityId, goal: Goal, order: Option<BTreeSet<EntityId>>) -> Option<Travel> {
        let map = self.w.s.at(member)?.map;
        Some(Travel { member, goal, seen: self.actors_in_view(map), order, steps: 0 })
    }

    /// Does seat `n` travel now, or has it an order for its turn?
    pub fn travelling(&self, n: SeatId) -> bool {
        self.travels.contains_key(&n) || self.orders.contains_key(&n)
    }

    /// Cancels seat `n`'s order, if it has one (`back`, M 8.1).
    pub fn cancel_order(&mut self, n: SeatId) -> bool {
        self.orders.remove(&n).is_some()
    }

    /// Ends seat `n`'s travel and order: any press does (M 8.1).
    pub fn stop_travel(&mut self, n: SeatId) {
        self.travels.remove(&n);
        self.orders.remove(&n);
    }

    /// `travel_to` on seat `n`'s turn (M 8.1). Nothing happens without
    /// `walk`, on the member's own cell or without a route.
    pub fn travel_to(&mut self, n: SeatId, to: Pos) -> RtResult<bool> {
        self.start(n, OrderGoal::Cell(to), None)
    }

    /// `follow` on seat `n`'s turn (M 8.1): `target`, or the `leader`
    /// query's entity.
    pub fn follow(&mut self, n: SeatId, target: Option<EntityId>) -> RtResult<bool> {
        self.start(n, OrderGoal::Follow(target), None)
    }

    /// An order from seat `n` on another seat's turn (M 8.1). A new order
    /// replaces the old one.
    pub fn give_order(&mut self, n: SeatId, goal: OrderGoal) {
        let map = self.w.s.active(n).into_iter().chain(self.w.s.seat_members(n)).find_map(|e| self.w.s.at(e)).map(|p| p.map);
        let seen = map.map(|m| self.actors_in_view(m)).unwrap_or_default();
        self.orders.insert(n, Order { goal, seen });
    }

    fn start(&mut self, n: SeatId, goal: OrderGoal, order: Option<BTreeSet<EntityId>>) -> RtResult<bool> {
        let Some(member) = self.turn_member(n) else { return Err(format!("seat {n} has no turn now")) };
        let Some(at) = self.w.s.at(member) else { return Ok(false) };
        let goal = match goal {
            OrderGoal::Cell(to) => {
                if self.fields.walk.is_none() || to == at {
                    return Ok(false);
                }
                let known = self.known_cells(at.map);
                if self.w.route_within(at, &[to], Some(member), Some(&known)).is_empty() {
                    return Ok(false);
                }
                Goal::Cell(to)
            }
            OrderGoal::Follow(target) => {
                if !self.fields.can_follow() {
                    return Ok(false);
                }
                let target = match target {
                    Some(t) => Some(t),
                    None => self.leader_of(member)?,
                };
                let Some(t) = target else { return Ok(false) };
                let same_map = self.w.s.at(t).is_some_and(|p| p.map == at.map);
                if t == member || !self.w.s.get(t).alive || !same_map || self.would_circle(member, t) {
                    return Ok(false);
                }
                Goal::Follow(t)
            }
        };
        match self.new_travel(member, goal, order) {
            Some(t) => {
                self.travels.insert(n, t);
                Ok(true)
            }
            None => Ok(false),
        }
    }

    /// The `leader` query's entity for `who` (D 3.3).
    fn leader_of(&mut self, who: EntityId) -> RtResult<Option<EntityId>> {
        let Some(q) = self.fields.leader.clone() else { return Ok(None) };
        let mut env = super::eval::Env::new(super::eval::BodyKind::Handler, "follow", None, "<engine>");
        env.map = self.w.s.at(who).map(|p| p.map);
        let v = self.w.call_function(&q, vec![(None, Value::Entity(who))], &mut env)?;
        Ok(v.as_entity())
    }

    /// Would `member` following `target` close a ring of followers (M 8.1)?
    fn would_circle(&self, member: EntityId, target: EntityId) -> bool {
        let mut t = target;
        for _ in 0..=self.travels.len() {
            if t == member {
                return true;
            }
            match self.travels.values().find(|tr| tr.member == t).map(|tr| tr.goal) {
                Some(Goal::Follow(next)) => t = next,
                _ => return false,
            }
        }
        false
    }

    /// One step of the awaited seat's travel, as its input. `None`: the
    /// step ran and the travel goes on; `Some(stop)`: it ended, and the
    /// seat's input is awaited. An order starts here on its seat's turn.
    pub fn step_travel(&mut self) -> RtResult<Option<Stop>> {
        let Some(n) = self.awaiting() else { return Ok(None) };
        if !self.travels.contains_key(&n)
            && let Some(o) = self.orders.remove(&n)
            && !self.start(n, o.goal, Some(o.seen))?
        {
            return Ok(Some(Stop::NoRoute));
        }
        let Some(mut t) = self.travels.remove(&n) else { return Ok(None) };
        let stop = self.step(n, &mut t)?;
        if stop.is_none() {
            self.travels.insert(n, t);
        }
        Ok(stop)
    }

    fn step(&mut self, n: SeatId, t: &mut Travel) -> RtResult<Option<Stop>> {
        if self.turn_member(n) != Some(t.member) {
            return Ok(Some(Stop::TurnLost));
        }
        let Some(at) = self.w.s.at(t.member) else { return Ok(Some(Stop::TurnLost)) };
        let ai = matches!(t.goal, Goal::Follow(_)) && self.fields.follow_ai.is_some();
        // an actor not seen at the last step, or for an order's second step
        // one not seen when the order was given (M 8.1)
        let seen = self.actors_in_view(at.map);
        if t.steps == 1
            && let Some(o) = &t.order
            && !seen.is_subset(o)
        {
            return Ok(Some(Stop::Stale));
        }
        if !ai && !seen.is_subset(&t.seen) {
            return Ok(Some(Stop::NewActor));
        }
        t.seen = seen;
        let cmd = match t.goal {
            Goal::Cell(to) => {
                if at == to {
                    return Ok(Some(Stop::Arrived));
                }
                let known = self.known_cells(at.map);
                let Some(&next) = self.w.route_within(at, &[to], Some(t.member), Some(&known)).first() else { return Ok(Some(Stop::NoRoute)) };
                self.walk_step(t.member, at, next)?
            }
            Goal::Follow(leader) => {
                let Some(lat) = self.w.s.at(leader).filter(|p| p.map == at.map && self.w.s.get(leader).alive) else { return Ok(Some(Stop::LeaderGone)) };
                if let Some(behavior) = self.fields.follow_ai.clone() {
                    if self.walk_stops(t.member, at)? {
                        return Ok(Some(Stop::Game));
                    }
                    match self.w.run_ai_with(&behavior, t.member, vec![Value::Entity(leader)])? {
                        Value::Command(c) => Some(c),
                        _ => self.rest(),
                    }
                } else if at.distance(lat) <= self.fields.follow_distance {
                    self.rest()
                } else {
                    let near: Vec<Pos> = Dir::ALL.into_iter().map(|d| lat.step(d)).filter(|&p| p != at).collect();
                    let known = self.known_cells(at.map);
                    match self.w.route_within(at, &near, Some(t.member), Some(&known)).first() {
                        Some(&next) => self.walk_step(t.member, at, next)?,
                        None => self.rest(),
                    }
                }
            }
        };
        let Some(cmd) = cmd else { return Ok(Some(Stop::Game)) };
        t.steps += 1;
        if !self.input(n, &cmd)? {
            return Ok(Some(Stop::Failed));
        }
        Ok(None)
    }

    fn rest(&self) -> Option<CommandValue> {
        self.fields.rest.clone().map(|name| CommandValue { name, args: vec![] })
    }

    /// The `walk` command towards `next`, or `None` when `walk_stop` stops
    /// the travel first.
    fn walk_step(&mut self, who: EntityId, at: Pos, next: Pos) -> RtResult<Option<CommandValue>> {
        if self.walk_stops(who, next)? {
            return Ok(None);
        }
        let (Some(walk), Some(d)) = (self.fields.walk.clone(), dir_to(at, next)) else { return Ok(None) };
        Ok(Some(CommandValue { name: walk, args: vec![(None, Value::Dir(d))] }))
    }

    /// The game's `walk_stop(who, next)` before a step (D 3.3).
    fn walk_stops(&mut self, who: EntityId, next: Pos) -> RtResult<bool> {
        let Some(q) = self.fields.walk_stop.clone() else { return Ok(false) };
        let mut env = super::eval::Env::new(super::eval::BodyKind::Handler, "walk_stop", None, "<engine>");
        env.map = Some(next.map);
        let v = self.w.call_function(&q, vec![(None, Value::Entity(who)), (None, Value::Pos(next))], &mut env)?;
        Ok(matches!(v, Value::Bool(true)))
    }
}
