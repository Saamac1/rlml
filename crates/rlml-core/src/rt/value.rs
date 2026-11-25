//! Runtime values (RS 2.6). Each value matches a type of
//! [`crate::rs::types::Ty`]; equality is RogueScript's `==`.
//!
//! Positions and areas carry their map, so cells of different maps are
//! never equal (RS 2.6). An area keeps its cells in reading order: top to
//! bottom, left to right (M 4.2).

use std::fmt;

/// Index of a loaded map in [`super::state::State::maps`].
pub type MapId = usize;

/// Index of an instance in [`super::state::State::entities`]; instances are
/// numbered in creation order and a handle stays valid after death (RS 4).
pub type EntityId = usize;

/// The eight directions and `none` (D 2.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Dir {
    North,
    Northeast,
    East,
    Southeast,
    South,
    Southwest,
    West,
    Northwest,
    None,
}

impl Dir {
    /// The fixed order of M 9.3 and D 2.4.
    pub const ALL: [Dir; 8] = [
        Dir::North,
        Dir::Northeast,
        Dir::East,
        Dir::Southeast,
        Dir::South,
        Dir::Southwest,
        Dir::West,
        Dir::Northwest,
    ];

    pub fn parse(s: &str) -> Option<Dir> {
        Some(match s {
            "north" => Dir::North,
            "northeast" => Dir::Northeast,
            "east" => Dir::East,
            "southeast" => Dir::Southeast,
            "south" => Dir::South,
            "southwest" => Dir::Southwest,
            "west" => Dir::West,
            "northwest" => Dir::Northwest,
            "none" => Dir::None,
            _ => return Option::None,
        })
    }

    /// Column and row offset; north is up (row −1).
    pub fn delta(self) -> (i32, i32) {
        match self {
            Dir::North => (0, -1),
            Dir::Northeast => (1, -1),
            Dir::East => (1, 0),
            Dir::Southeast => (1, 1),
            Dir::South => (0, 1),
            Dir::Southwest => (-1, 1),
            Dir::West => (-1, 0),
            Dir::Northwest => (-1, -1),
            Dir::None => (0, 0),
        }
    }

    /// `-d` is the opposite direction, and `-none` is `none` (RS 5.3).
    pub fn opposite(self) -> Dir {
        match self {
            Dir::None => Dir::None,
            d => Dir::ALL[(Dir::ALL.iter().position(|x| *x == d).unwrap() + 4) % 8],
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Dir::North => "north",
            Dir::Northeast => "northeast",
            Dir::East => "east",
            Dir::Southeast => "southeast",
            Dir::South => "south",
            Dir::Southwest => "southwest",
            Dir::West => "west",
            Dir::Northwest => "northwest",
            Dir::None => "none",
        }
    }
}

/// A relative direction (D 2.4): the number of eighth turns clockwise from
/// front, or `None`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Side(pub Option<u8>);

impl Side {
    const NAMES: [&'static str; 8] = [
        "front",
        "frontright",
        "right",
        "backright",
        "back",
        "backleft",
        "left",
        "frontleft",
    ];

    pub fn parse(s: &str) -> Option<Side> {
        if s == "none" {
            return Some(Side(Option::None));
        }
        Side::NAMES
            .iter()
            .position(|n| *n == s)
            .map(|i| Side(Some(i as u8)))
    }

    pub fn opposite(self) -> Side {
        Side(self.0.map(|i| (i + 4) % 8))
    }

    /// The map direction when facing `facing` (`facing` not `none`).
    pub fn turn(self, facing: Dir) -> Dir {
        match (self.0, Dir::ALL.iter().position(|d| *d == facing)) {
            (Some(s), Some(f)) => Dir::ALL[(f + s as usize) % 8],
            _ => Dir::None,
        }
    }

    pub fn name(self) -> &'static str {
        self.0.map(|i| Side::NAMES[i as usize]).unwrap_or("none")
    }
}

/// A cell of a map.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Pos {
    pub map: MapId,
    pub x: i32,
    pub y: i32,
}

impl Pos {
    pub fn step(self, d: Dir) -> Pos {
        let (dx, dy) = d.delta();
        Pos {
            x: self.x + dx,
            y: self.y + dy,
            ..self
        }
    }

    /// Chebyshev distance (D 8.8); cells of different maps are infinitely
    /// far apart.
    pub fn distance(self, other: Pos) -> i64 {
        if self.map != other.map {
            return i64::MAX;
        }
        ((self.x - other.x).abs().max((self.y - other.y).abs())) as i64
    }

    fn order(self) -> (MapId, i32, i32) {
        (self.map, self.y, self.x)
    }
}

/// A set of cells of one map in reading order (RS 6).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Area {
    cells: Vec<Pos>,
}

impl Area {
    pub fn new(mut cells: Vec<Pos>) -> Area {
        cells.sort_by_key(|p| p.order());
        cells.dedup();
        Area { cells }
    }

    pub fn one(p: Pos) -> Area {
        Area { cells: vec![p] }
    }

    pub fn cells(&self) -> &[Pos] {
        &self.cells
    }

    pub fn len(&self) -> usize {
        self.cells.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    pub fn contains(&self, p: Pos) -> bool {
        self.cells
            .binary_search_by_key(&p.order(), |c| c.order())
            .is_ok()
    }

    /// `a and b`.
    pub fn and(&self, other: &Area) -> Area {
        Area {
            cells: self
                .cells
                .iter()
                .copied()
                .filter(|p| other.contains(*p))
                .collect(),
        }
    }

    /// `a or b`.
    pub fn or(&self, other: &Area) -> Area {
        Area::new(
            self.cells
                .iter()
                .chain(other.cells.iter())
                .copied()
                .collect(),
        )
    }

    /// `a - b`.
    pub fn minus(&self, other: &Area) -> Area {
        Area {
            cells: self
                .cells
                .iter()
                .copied()
                .filter(|p| !other.contains(*p))
                .collect(),
        }
    }
}

/// A card: an entity name, an optional count and field overrides (D 6.4,
/// RS 2.8).
#[derive(Debug, Clone, PartialEq)]
pub struct Card {
    /// Full name of the entity, or `None` for a `none` card.
    pub entity: Option<String>,
    pub count: Option<Box<Value>>,
    /// Overrides by component: a one-field component's value or a
    /// component value.
    pub overrides: Vec<(String, Value)>,
}

/// A component value (`HP(now: 3)`, `ai(behavior: HUNTER)`, RS 2.8).
#[derive(Debug, Clone, PartialEq)]
pub struct ComponentValue {
    /// Full name, or the engine component's vocabulary name.
    pub name: String,
    pub fields: Vec<(String, Value)>,
}

impl ComponentValue {
    pub fn get(&self, field: &str) -> Option<&Value> {
        self.fields.iter().find(|(f, _)| f == field).map(|(_, v)| v)
    }

    pub fn set(&mut self, field: &str, v: Value) {
        match self.fields.iter_mut().find(|(f, _)| f == field) {
            Some((_, old)) => *old = v,
            Option::None => self.fields.push((field.to_string(), v)),
        }
    }
}

/// A command as a value: the command's full name and its arguments
/// (RS 2.8, RS 7.2).
#[derive(Debug, Clone, PartialEq)]
pub struct CommandValue {
    pub name: String,
    pub args: Vec<(Option<String>, Value)>,
}

/// A heard sound (M 6.8).
#[derive(Debug, Clone, PartialEq)]
pub struct Sound {
    pub kind: String,
    pub at: Pos,
    pub source: Option<EntityId>,
    pub level: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Number(i64),
    /// `NdM+K`.
    Dice {
        n: u32,
        sides: u32,
        bonus: i64,
    },
    /// `a..b`; `None` is an open upper end.
    Range(i64, Option<i64>),
    Percent(i64),
    Bool(bool),
    Text(String),
    /// A full name: an entity's kind, a map, a tile, a text key, …
    Name(String),
    Dir(Dir),
    Side(Side),
    Pos(Pos),
    Area(Area),
    Entity(EntityId),
    Card(Card),
    /// Entities, cards, names or sounds in order (RS 2.6).
    Collection(Vec<Value>),
    List(Vec<Value>),
    /// Key–value pairs in insertion order.
    Dict(Vec<(Value, Value)>),
    /// A shared deck by full name.
    Deck(String),
    Table(String),
    Command(CommandValue),
    /// A scene with its arguments, as `start` and `wait scene` take it
    /// (RS 9).
    Scene(CommandValue),
    Component(ComponentValue),
    Sound(Sound),
    /// Presentation time in milliseconds.
    Time(u64),
    /// A background scene's handle, from `start` (RS 9.9).
    Handle(u64),
    Size(u32, u32),
    All,
    None,
}

impl Value {
    pub fn is_none(&self) -> bool {
        matches!(self, Value::None)
    }

    /// A truth value; anything else is a runtime error (RS 5.3).
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => Option::None,
        }
    }

    pub fn as_number(&self) -> Option<i64> {
        match self {
            Value::Number(n) => Some(*n),
            _ => Option::None,
        }
    }

    pub fn as_entity(&self) -> Option<EntityId> {
        match self {
            Value::Entity(e) => Some(*e),
            _ => Option::None,
        }
    }

    /// The members of a set: collection, list, area cells, range numbers or
    /// dictionary keys (RS 5.3).
    pub fn members(&self) -> Option<Vec<Value>> {
        Some(match self {
            Value::Collection(v) | Value::List(v) => v.clone(),
            Value::Area(a) => a.cells().iter().map(|p| Value::Pos(*p)).collect(),
            Value::Range(a, Some(b)) => (*a..=*b).map(Value::Number).collect(),
            Value::Dict(pairs) => pairs.iter().map(|(k, _)| k.clone()).collect(),
            _ => return Option::None,
        })
    }

    /// Type name for runtime error messages.
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Number(_) => "number",
            Value::Dice { .. } => "dice",
            Value::Range(..) => "range",
            Value::Percent(_) => "percent",
            Value::Bool(_) => "bool",
            Value::Text(_) => "text",
            Value::Name(_) => "name",
            Value::Dir(_) => "dir",
            Value::Side(_) => "side",
            Value::Pos(_) => "pos",
            Value::Area(_) => "area",
            Value::Entity(_) => "entity",
            Value::Card(_) => "card",
            Value::Collection(_) => "collection",
            Value::List(_) => "list",
            Value::Dict(_) => "dict",
            Value::Deck(_) => "deck",
            Value::Table(_) => "table",
            Value::Command(_) => "command",
            Value::Scene(_) => "scene",
            Value::Component(_) => "component",
            Value::Sound(_) => "sound",
            Value::Time(_) => "time",
            Value::Handle(_) => "handle",
            Value::Size(..) => "size",
            Value::All => "all",
            Value::None => "none",
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Number(n) => write!(f, "{n}"),
            Value::Dice { n, sides, bonus: 0 } => write!(f, "{n}d{sides}"),
            Value::Dice { n, sides, bonus } => write!(f, "{n}d{sides}{bonus:+}"),
            Value::Range(a, Some(b)) => write!(f, "{a}..{b}"),
            Value::Range(a, Option::None) => write!(f, "{a}.."),
            Value::Percent(p) => write!(f, "{p}%"),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Text(t) => write!(f, "\"{t}\""),
            Value::Name(n) | Value::Deck(n) | Value::Table(n) => f.write_str(n),
            Value::Dir(d) => f.write_str(d.name()),
            Value::Side(s) => f.write_str(s.name()),
            Value::Pos(p) => write!(f, "pos({}, {})", p.x, p.y),
            Value::Area(a) => write!(f, "area of {}", a.len()),
            Value::Entity(e) => write!(f, "entity #{e}"),
            Value::Card(c) => write!(f, "card {}", c.entity.as_deref().unwrap_or("none")),
            Value::Collection(v) => write!(f, "collection of {}", v.len()),
            Value::List(v) => write!(f, "list of {}", v.len()),
            Value::Dict(v) => write!(f, "dict of {}", v.len()),
            Value::Command(c) | Value::Scene(c) => write!(f, "{}(…)", c.name),
            Value::Component(c) => write!(f, "{}(…)", c.name),
            Value::Sound(s) => write!(f, "sound {}", s.kind),
            Value::Time(ms) => write!(f, "{ms}ms"),
            Value::Handle(h) => write!(f, "scene#{h}"),
            Value::Size(w, h) => write!(f, "{w}x{h}"),
            Value::All => f.write_str("all"),
            Value::None => f.write_str("none"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: i32, y: i32) -> Pos {
        Pos { map: 0, x, y }
    }

    #[test]
    fn directions() {
        assert_eq!(Dir::North.opposite(), Dir::South);
        assert_eq!(Dir::Northwest.opposite(), Dir::Southeast);
        assert_eq!(Dir::None.opposite(), Dir::None);
        assert_eq!(p(2, 2).step(Dir::Northeast), p(3, 1));
        assert_eq!(Side::parse("right").unwrap().turn(Dir::North), Dir::East);
        assert_eq!(Side::parse("back").unwrap().turn(Dir::East), Dir::West);
        assert_eq!(Side::parse("front").unwrap().opposite().name(), "back");
    }

    #[test]
    fn distance_is_chebyshev_within_a_map() {
        assert_eq!(p(0, 0).distance(p(3, 1)), 3);
        assert_eq!(p(0, 0).distance(Pos { map: 1, x: 0, y: 0 }), i64::MAX);
    }

    #[test]
    fn areas_keep_reading_order() {
        let a = Area::new(vec![p(1, 1), p(0, 1), p(5, 0), p(0, 1)]);
        assert_eq!(a.cells(), [p(5, 0), p(0, 1), p(1, 1)]);
        let b = Area::new(vec![p(0, 1), p(2, 2)]);
        assert_eq!(a.and(&b).cells(), [p(0, 1)]);
        assert_eq!(a.or(&b).cells(), [p(5, 0), p(0, 1), p(1, 1), p(2, 2)]);
        assert_eq!(a.minus(&b).cells(), [p(5, 0), p(1, 1)]);
        assert!(a.contains(p(1, 1)) && !a.contains(p(2, 2)));
    }

    #[test]
    fn members_and_equality() {
        assert_eq!(Value::Range(1, Some(3)).members().unwrap().len(), 3);
        assert_eq!(
            Value::Range(0, Some(-1)).members().unwrap().len(),
            0,
            "an empty range (RS 2.6)"
        );
        assert_ne!(
            Value::Pos(p(0, 0)),
            Value::Pos(Pos { map: 1, x: 0, y: 0 }),
            "cells belong to their map"
        );
        assert_eq!(Value::Entity(3), Value::Entity(3));
    }
}
