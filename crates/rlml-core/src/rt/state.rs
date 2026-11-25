//! The game's state (M 3.1): instances and where they are, the `game`
//! record, loaded maps, the wake queue and map clocks (M 6.1.2, M 6.1.8)
//! and deck state (D 6.2). The random streams come with the generator.
//!
//! Every order the specification fixes is kept here (M 4.2): instances in
//! creation order, collection members by slot, wakes by moment and then
//! sequence number.

use super::value::{Area, Card, ComponentValue, EntityId, MapId, Pos, Value};

/// Where an instance is: in a cell of a map or in another instance's
/// collection field (D 5.2). A destroyed or not yet placed instance is
/// nowhere.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Place {
    Nowhere,
    Map(Pos),
    In { holder: EntityId, component: String, field: String, slot: usize },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Instance {
    /// The definition it was created from (`e.kind`, D 5.2).
    pub kind: String,
    /// Components in the order they were added, the engine's own by their
    /// lower-case names.
    pub components: Vec<ComponentValue>,
    pub place: Place,
    /// A destroyed instance keeps its handle and name (RS 4).
    pub alive: bool,
}

impl Instance {
    pub fn get(&self, component: &str) -> Option<&ComponentValue> {
        self.components.iter().find(|c| c.name == component)
    }

    pub fn get_mut(&mut self, component: &str) -> Option<&mut ComponentValue> {
        self.components.iter_mut().find(|c| c.name == component)
    }

    pub fn has(&self, component: &str) -> bool {
        self.get(component).is_some()
    }

    /// Adds a component, or replaces one with the same name in its place
    /// (M 9.5).
    pub fn set(&mut self, c: ComponentValue) {
        match self.components.iter_mut().find(|x| x.name == c.name) {
            Some(old) => *old = c,
            None => self.components.push(c),
        }
    }

    pub fn remove(&mut self, component: &str) -> Option<ComponentValue> {
        let i = self.components.iter().position(|c| c.name == component)?;
        Some(self.components.remove(i))
    }
}

/// The `game` record (D 3.3, D 5.5): the engine's fields and the authors'
/// components.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Game {
    pub start: Option<String>,
    pub at: Area,
    pub seed: u64,
    /// Elapsed microsteps (M 6.1.1).
    pub time: i64,
    pub call_depth: u32,
    pub loop_steps: u64,
    pub components: Vec<ComponentValue>,
}

impl Game {
    /// `game.turn` (M 6.1.1).
    pub fn turn(&self) -> i64 {
        self.time.div_euclid(120)
    }
}

/// A loaded map (M 3.1, D 8).
#[derive(Debug, Clone, PartialEq)]
pub struct MapState {
    pub name: String,
    pub width: i32,
    pub height: i32,
    /// Terrain layer names; one unnamed default layer without `terrain`.
    pub layers: Vec<String>,
    /// Tiles per layer, row by row; `None` is an empty layer.
    pub tiles: Vec<Vec<Option<String>>>,
    /// Cell labels and their cells in reading order (D 8.5).
    pub labels: Vec<(String, Vec<(i32, i32)>)>,
    pub wires: Vec<((i32, i32), String)>,
    pub vars: Vec<(String, Value)>,
    /// Cells the player remembers (M 7.2).
    pub remembered: Vec<(i32, i32)>,
    pub transient: bool,
    /// `game.time` when the map was first loaded: its clock starts there
    /// (M 6.1.8).
    pub loaded_at: i64,
    /// Microsteps the map has been frozen in total.
    pub frozen_total: i64,
    /// `game.time` when the map was last frozen, while it is frozen.
    pub frozen_at: Option<i64>,
}

impl MapState {
    pub fn new(name: &str, width: i32, height: i32, layers: Vec<String>, now: i64) -> MapState {
        let cells = (width * height) as usize;
        MapState {
            name: name.to_string(),
            width,
            height,
            tiles: layers.iter().map(|_| vec![None; cells]).collect(),
            layers,
            labels: vec![],
            wires: vec![],
            vars: vec![],
            remembered: vec![],
            transient: false,
            loaded_at: now,
            frozen_total: 0,
            frozen_at: None,
        }
    }

    pub fn inside(&self, x: i32, y: i32) -> bool {
        (0..self.width).contains(&x) && (0..self.height).contains(&y)
    }

    fn index(&self, x: i32, y: i32) -> usize {
        (y * self.width + x) as usize
    }

    pub fn tile(&self, layer: usize, x: i32, y: i32) -> Option<&str> {
        if !self.inside(x, y) {
            return None;
        }
        self.tiles.get(layer)?.get(self.index(x, y))?.as_deref()
    }

    pub fn set_tile(&mut self, layer: usize, x: i32, y: i32, tile: Option<String>) {
        if self.inside(x, y) {
            let i = self.index(x, y);
            self.tiles[layer][i] = tile;
        }
    }

    /// Adds a cell to a label group; cells stay in reading order (D 8.5).
    pub fn add_label(&mut self, label: &str, x: i32, y: i32) {
        let i = match self.labels.iter().position(|(l, _)| l == label) {
            Some(i) => i,
            None => {
                self.labels.push((label.to_string(), vec![]));
                self.labels.len() - 1
            }
        };
        let cells = &mut self.labels[i].1;
        if !cells.contains(&(x, y)) {
            cells.push((x, y));
            cells.sort_by_key(|(x, y)| (*y, *x));
        }
    }

    pub fn label(&self, label: &str) -> &[(i32, i32)] {
        self.labels.iter().find(|(l, _)| l == label).map(|(_, c)| c.as_slice()).unwrap_or(&[])
    }

    /// The map's own clock: `game.time` less the time it was frozen
    /// (M 6.1.8).
    pub fn clock(&self, now: i64) -> i64 {
        let frozen_now = self.frozen_at.map(|t| now - t).unwrap_or(0);
        now - self.frozen_total - frozen_now
    }

    pub fn freeze(&mut self, now: i64) {
        if self.frozen_at.is_none() {
            self.frozen_at = Some(now);
        }
    }

    pub fn thaw(&mut self, now: i64) {
        if let Some(t) = self.frozen_at.take() {
            self.frozen_total += now - t;
        }
    }
}

/// A wake: the moment on the actor's map clock and the sequence number
/// (M 6.1.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Wake {
    pub entity: EntityId,
    pub at: i64,
    pub seq: u64,
}

/// A shared deck's draw pile and discard pile (D 6.2).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DeckState {
    pub draw: Vec<Card>,
    pub discard: Vec<Card>,
}

/// A seat's number, 1…8 (M 6.1.10).
pub type SeatId = u8;

/// A joined seat: its members in the order they joined and the one it
/// controls (M 6.1.10). Devices are not state.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Seat {
    pub n: SeatId,
    pub members: Vec<EntityId>,
    pub active: Option<EntityId>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct State {
    pub entities: Vec<Instance>,
    pub game: Game,
    pub maps: Vec<MapState>,
    pub wakes: Vec<Wake>,
    pub next_seq: u64,
    pub decks: Vec<(String, DeckState)>,
    /// The member whose turn is on, or who had the last one (M 6.1.10);
    /// in a one-seat game the last `control` target (RS 10.1).
    pub player: Option<EntityId>,
    /// The joined seats by number (M 6.1.10).
    pub seats: Vec<Seat>,
    /// `party each`: every member waits for its seat's input (D 3.3).
    pub party_each: bool,
    /// The actor whose command runs now; its wake is out of the queue
    /// meanwhile (M 6.1.5). Only the scheduler sets it, between actions.
    pub acting: Option<EntityId>,
    /// `leash on`, by default when more than one seat may join (D 3.3).
    pub leash: bool,
    /// The game's `seats a..b` (D 3.3), locked for the session.
    pub seat_range: (SeatId, SeatId),
    /// The map the player was last on; the active map without a player
    /// (M 6.1.8).
    pub last_player_map: Option<MapId>,
    /// The session's view of the profile (M 5.6): its snapshot at the
    /// start and the session's own writes.
    pub profile: Vec<ComponentValue>,
    /// The camera's target from `follow(camera, e)`, or `None` for the
    /// default: the group, or `player` (M 7.1).
    pub camera: Option<EntityId>,
}

impl State {
    pub fn map_id(&self, name: &str) -> Option<MapId> {
        self.maps.iter().position(|m| m.name == name)
    }

    /// A new, unplaced instance.
    pub fn create(&mut self, kind: &str) -> EntityId {
        self.entities.push(Instance { kind: kind.to_string(), components: vec![], place: Place::Nowhere, alive: true });
        self.entities.len() - 1
    }

    pub fn get(&self, e: EntityId) -> &Instance {
        &self.entities[e]
    }

    pub fn get_mut(&mut self, e: EntityId) -> &mut Instance {
        &mut self.entities[e]
    }

    /// Living instances in a cell, in creation order (M 4.2).
    pub fn entities_at(&self, p: Pos) -> Vec<EntityId> {
        (0..self.entities.len()).filter(|&e| self.entities[e].alive && self.entities[e].place == Place::Map(p)).collect()
    }

    /// Living instances on a map, in creation order.
    pub fn entities_on(&self, map: MapId) -> Vec<EntityId> {
        (0..self.entities.len()).filter(|&e| self.entities[e].alive && matches!(self.entities[e].place, Place::Map(p) if p.map == map)).collect()
    }

    /// Members of a collection field by slot, a stack's members in creation
    /// order (D 5.4).
    pub fn members(&self, holder: EntityId, component: &str, field: &str) -> Vec<EntityId> {
        let mut out: Vec<(usize, EntityId)> = (0..self.entities.len())
            .filter_map(|e| match &self.entities[e].place {
                Place::In { holder: h, component: c, field: f, slot } if self.entities[e].alive && *h == holder && c == component && f == field => Some((*slot, e)),
                _ => None,
            })
            .collect();
        out.sort();
        out.into_iter().map(|(_, e)| e).collect()
    }

    /// The smallest free slot of a collection field (D 5.4).
    pub fn free_slot(&self, holder: EntityId, component: &str, field: &str) -> usize {
        let used: Vec<usize> = self
            .members(holder, component, field)
            .into_iter()
            .filter_map(|e| match &self.entities[e].place {
                Place::In { slot, .. } => Some(*slot),
                _ => None,
            })
            .collect();
        (0..).find(|s| !used.contains(s)).unwrap()
    }

    /// `e.at`: the cell, or in a collection the topmost holder's cell
    /// (M 3.4).
    /// The cell `e` is in, or for an instance in a collection its top
    /// holder's: where the camera follows it (M 7.1).
    pub fn outer_at(&self, e: EntityId) -> Option<Pos> {
        let mut x = e;
        for _ in 0..=self.entities.len() {
            match &self.get(x).place {
                Place::Map(p) => return Some(*p),
                Place::In { holder, .. } => x = *holder,
                Place::Nowhere => return None,
            }
        }
        None
    }

    pub fn at(&self, e: EntityId) -> Option<Pos> {
        let mut cur = e;
        for _ in 0..=self.entities.len() {
            match &self.entities[cur].place {
                Place::Map(p) => return Some(*p),
                // `game`'s collections are not on a map (D 5.5)
                Place::In { holder, .. } if *holder >= self.entities.len() => return None,
                Place::In { holder, .. } => cur = *holder,
                Place::Nowhere => return None,
            }
        }
        None
    }

    /// Schedules a wake on the actor's map clock; an actor has at most one
    /// wake, so an earlier one is replaced (M 6.1.2).
    pub fn schedule(&mut self, entity: EntityId, at: i64) {
        let seq = self.next_seq;
        self.next_seq += 1;
        self.restore(Wake { entity, at, seq });
    }

    /// Puts a wake back as it was, its sequence number kept: a kept turn
    /// keeps its place in the moment's order (M 6.1.5).
    pub fn restore(&mut self, w: Wake) {
        self.unschedule(w.entity);
        self.wakes.push(w);
    }

    pub fn unschedule(&mut self, entity: EntityId) {
        self.wakes.retain(|w| w.entity != entity);
    }

    pub fn wake_of(&self, entity: EntityId) -> Option<Wake> {
        self.wakes.iter().find(|w| w.entity == entity).copied()
    }

    /// The next wake of actors on `map`: the smallest moment, then the
    /// smallest sequence number (M 6.1.2).
    pub fn next_wake(&self, map: MapId) -> Option<Wake> {
        self.wakes.iter().filter(|w| self.at(w.entity).is_some_and(|p| p.map == map)).min_by_key(|w| (w.at, w.seq)).copied()
    }

    pub fn deck(&self, name: &str) -> Option<&DeckState> {
        self.decks.iter().find(|(n, _)| n == name).map(|(_, d)| d)
    }

    pub fn deck_mut(&mut self, name: &str) -> Option<&mut DeckState> {
        self.decks.iter_mut().find(|(n, _)| n == name).map(|(_, d)| d)
    }

    /// The active map: the player's, or a living member's, or where the
    /// player last was, or the start map (M 6.1.8). Several active maps
    /// (M 6.1.10) are not supported yet: the group shares the player's.
    pub fn active_map(&self) -> Option<MapId> {
        self.player
            .filter(|&p| self.get(p).alive)
            .and_then(|p| self.at(p))
            .or_else(|| self.players().into_iter().find_map(|e| self.at(e)))
            .map(|p| p.map)
            .or(self.last_player_map)
            .or_else(|| self.game.start.as_deref().and_then(|s| self.map_id(s)))
    }

    // ---- seats (M 6.1.10) ----

    pub fn seat(&self, n: SeatId) -> Option<&Seat> {
        self.seats.iter().find(|s| s.n == n)
    }

    fn seat_mut(&mut self, n: SeatId) -> Option<&mut Seat> {
        self.seats.iter_mut().find(|s| s.n == n)
    }

    /// Joins seat `n`; the seats stay in number order. Joining a joined
    /// seat does nothing.
    pub fn join_seat(&mut self, n: SeatId) {
        if self.seat(n).is_none() {
            let i = self.seats.iter().position(|s| s.n > n).unwrap_or(self.seats.len());
            self.seats.insert(i, Seat { n, members: vec![], active: None });
        }
    }

    /// The lowest free seat number.
    pub fn free_seat(&self) -> SeatId {
        (1..=u8::MAX).find(|&n| self.seat(n).is_none()).unwrap_or(u8::MAX)
    }

    /// Frees seat `n`: its members lose `seat` (M 8.2).
    pub fn part_seat(&mut self, n: SeatId) {
        if let Some(i) = self.seats.iter().position(|s| s.n == n) {
            for e in self.seats.remove(i).members {
                self.get_mut(e).remove("seat");
            }
        }
    }

    /// The seat `e` is a member of.
    pub fn seat_of(&self, e: EntityId) -> Option<SeatId> {
        self.seats.iter().find(|s| s.members.contains(&e)).map(|s| s.n)
    }

    pub fn is_member(&self, e: EntityId) -> bool {
        self.seat_of(e).is_some()
    }

    /// The one way membership changes (M 6.1.10): `e` joins seat `n` with
    /// the engine component `seat` (D 5.3), or with `None` leaves its seat
    /// and loses the component. A seat that is not joined changes nothing.
    /// A controlled member that moves to a seat without a living controlled
    /// member stays controlled there, and its old seat's next living member
    /// takes control. `player` is not touched here: the scheduler sets it
    /// when a member waits for input, and `control` on a body swap.
    pub fn set_seat(&mut self, e: EntityId, n: Option<SeatId>) {
        // a seat that is not joined changes nothing; callers report it
        if n.is_some_and(|n| self.seat(n).is_none()) || (n.is_some() && self.seat_of(e) == n) {
            return;
        }
        let old = self.seat_of(e);
        let was_active = old.is_some_and(|o| self.seat(o).is_some_and(|s| s.active == Some(e)));
        let free = n.is_some_and(|n| self.active(n).is_none());
        self.remove_member(e);
        if let Some(n) = n
            && let Some(s) = self.seat_mut(n)
        {
            s.members.push(e);
            if was_active && free {
                s.active = Some(e);
            }
            self.get_mut(e).set(ComponentValue { name: "seat".into(), fields: vec![("n".into(), Value::Number(n as i64))] });
        }
        // a controlled member that leaves its seat, for whatever reason,
        // hands control to the seat's next living member (M 6.1.10)
        if was_active
            && let Some(o) = old
            && let Some(next) = self.seat_members(o).first().copied()
        {
            self.set_active(o, next);
        }
    }

    /// A seat number from a logic or data value, if it is a valid number.
    pub fn seat_number(v: &Value) -> Option<SeatId> {
        v.as_number().and_then(|n| SeatId::try_from(n).ok())
    }

    /// `e` leaves its seat and loses `seat`; `set_seat(e, None)`.
    fn remove_member(&mut self, e: EntityId) {
        for s in &mut self.seats {
            s.members.retain(|&m| m != e);
            if s.active == Some(e) {
                s.active = None;
            }
        }
        self.get_mut(e).remove("seat");
    }

    pub fn set_active(&mut self, n: SeatId, e: EntityId) {
        if let Some(s) = self.seat_mut(n) {
            s.active = Some(e);
        }
    }

    /// A seat's living members in joining order.
    pub fn seat_members(&self, n: SeatId) -> Vec<EntityId> {
        self.seat(n).map(|s| s.members.iter().copied().filter(|&e| self.get(e).alive).collect()).unwrap_or_default()
    }

    /// `players`: every seat's living members, by seat and then joining
    /// order (M 6.1.10).
    pub fn players(&self) -> Vec<EntityId> {
        self.seats.iter().flat_map(|s| s.members.iter().copied()).filter(|&e| self.get(e).alive).collect()
    }

    /// A seat's living controlled member.
    pub fn active(&self, n: SeatId) -> Option<EntityId> {
        self.seat(n).and_then(|s| s.active).filter(|&e| self.get(e).alive)
    }

    /// `controlled(e)` (M 6.1.10): does `e` wait for its seat's input on
    /// its turns? The seat's controlled member does, with `party each`
    /// every living member, and with `party switch` any member while the
    /// controlled one cannot act (dead, without a wake or off the map),
    /// since control moves to whoever wakes first: the scheduler's rule.
    pub fn controlled(&self, e: EntityId) -> bool {
        if !self.get(e).alive {
            return false;
        }
        let Some(n) = self.seat_of(e) else { return false };
        let active = self.active(n);
        // the member whose command runs has its wake out of the queue
        // meanwhile; it can still act
        let cannot = |a: EntityId| self.at(a).is_none() || (self.wake_of(a).is_none() && self.acting != Some(a));
        self.party_each || active == Some(e) || active.is_none_or(cannot)
    }

    /// An actor: a member, or an instance with `ai` (M 6.1.10).
    pub fn is_actor(&self, e: EntityId) -> bool {
        self.is_member(e) || self.get(e).has("ai")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: i32, y: i32) -> Pos {
        Pos { map: 0, x, y }
    }

    fn state() -> State {
        let mut s = State::default();
        s.maps.push(MapState::new("CAVE", 4, 3, vec![String::new()], 0));
        s
    }

    #[test]
    fn instances_in_cells_and_creation_order() {
        let mut s = state();
        let a = s.create("RAT");
        let b = s.create("RAT");
        s.get_mut(b).place = Place::Map(p(1, 1));
        s.get_mut(a).place = Place::Map(p(1, 1));
        assert_eq!(s.entities_at(p(1, 1)), [a, b], "creation order, not placing order");
        s.get_mut(a).alive = false;
        assert_eq!(s.entities_at(p(1, 1)), [b]);
        assert_eq!(s.get(a).kind, "RAT", "a dead handle keeps its kind");
    }

    #[test]
    fn collections_by_slot() {
        let mut s = state();
        let hero = s.create("HERO");
        s.get_mut(hero).place = Place::Map(p(2, 0));
        let (x, y, z) = (s.create("A"), s.create("B"), s.create("C"));
        for (e, slot) in [(x, 2), (y, 0), (z, 1)] {
            s.get_mut(e).place = Place::In { holder: hero, component: "PACK".into(), field: "items".into(), slot };
        }
        assert_eq!(s.members(hero, "PACK", "items"), [y, z, x]);
        s.get_mut(z).alive = false;
        assert_eq!(s.free_slot(hero, "PACK", "items"), 1, "the smallest free slot");
        assert_eq!(s.at(x), Some(p(2, 0)), "a member is at its holder's cell");
    }

    #[test]
    fn wakes_by_moment_then_sequence() {
        let mut s = state();
        let (a, b, c) = (s.create("A"), s.create("B"), s.create("C"));
        for e in [a, b, c] {
            s.get_mut(e).place = Place::Map(p(0, 0));
        }
        s.schedule(a, 120);
        s.schedule(b, 60);
        s.schedule(c, 60);
        assert_eq!(s.next_wake(0).unwrap().entity, b, "same moment: scheduled first runs first");
        s.schedule(b, 200);
        assert_eq!(s.wakes.len(), 3, "one wake per actor");
        assert_eq!(s.next_wake(0).unwrap().entity, c);
    }

    #[test]
    fn map_clock_stops_while_frozen() {
        let mut m = MapState::new("CAVE", 1, 1, vec![String::new()], 0);
        assert_eq!(m.clock(500), 500);
        m.freeze(500);
        assert_eq!(m.clock(800), 500, "frozen");
        m.thaw(800);
        assert_eq!(m.clock(830), 530, "goes on from where it stopped (M 6.1.8)");
    }

    #[test]
    fn labels_in_reading_order() {
        let mut m = MapState::new("CAVE", 4, 3, vec![String::new()], 0);
        m.add_label("WEST", 0, 2);
        m.add_label("WEST", 0, 0);
        m.add_label("WEST", 0, 1);
        assert_eq!(m.label("WEST"), [(0, 0), (0, 1), (0, 2)]);
        m.set_tile(0, 1, 1, Some("FLOOR".into()));
        assert_eq!(m.tile(0, 1, 1), Some("FLOOR"));
        assert_eq!(m.tile(0, 9, 9), None);
    }
}
