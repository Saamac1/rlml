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

#[derive(Debug, Clone, PartialEq, Default)]
pub struct State {
    pub entities: Vec<Instance>,
    pub game: Game,
    pub maps: Vec<MapState>,
    pub wakes: Vec<Wake>,
    pub next_seq: u64,
    pub decks: Vec<(String, DeckState)>,
    /// The last `control` target (RS 10.1).
    pub player: Option<EntityId>,
    /// The map the player was last on; the active map without a player
    /// (M 6.1.8).
    pub last_player_map: Option<MapId>,
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
    pub fn at(&self, e: EntityId) -> Option<Pos> {
        let mut cur = e;
        for _ in 0..=self.entities.len() {
            match &self.entities[cur].place {
                Place::Map(p) => return Some(*p),
                Place::In { holder, .. } => cur = *holder,
                Place::Nowhere => return None,
            }
        }
        None
    }

    /// Schedules a wake on the actor's map clock; an actor has at most one
    /// wake, so an earlier one is replaced (M 6.1.2).
    pub fn schedule(&mut self, entity: EntityId, at: i64) {
        self.unschedule(entity);
        let seq = self.next_seq;
        self.next_seq += 1;
        self.wakes.push(Wake { entity, at, seq });
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

    /// The active map: the player's, or where the player last was, or the
    /// start map (M 6.1.8).
    pub fn active_map(&self) -> Option<MapId> {
        self.player.and_then(|p| self.at(p)).map(|p| p.map).or(self.last_player_map).or_else(|| self.game.start.as_deref().and_then(|s| self.map_id(s)))
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
