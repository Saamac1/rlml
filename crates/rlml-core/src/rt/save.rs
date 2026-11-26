//! The save file (M 5.2): a snapshot of the game between actions, its
//! last checkpoint and the log's messages, as text the engine writes and
//! reads itself. The form is the engine's own and not RLML.
//!
//! Every value is a form in parentheses, words and numbers between
//! spaces and texts in quotes; `~` is an empty option. Each top-level
//! part is on a line of its own, so a save reads in a diff:
//!
//! ```text
//! rlml-save 1
//! game MINI
//! time 360
//! now (snapshot
//! (state (game ~ (area) 7 360 256 1000000 (...)) ...
//! ```
//!
//! Map and entity numbers are kept as they are: a save holds every
//! loaded map and every instance, dead ones too, in their order.

use std::collections::{BTreeMap, BTreeSet};

use super::checkpoint::Snapshot;
use super::eval::{BodyKind, Env, Suspended};
use super::input::InputLog;
use super::path::{LoopState, PathLoop};
use super::rng::Rng;
use super::run::Runner;
use super::state::{
    Background, DeckState, Game, Instance, MapState, Place, QueuedScene, Seat, State, Wake,
};
use super::timeline::{LoopRun, Steps, TlState, Tween};
use super::value::{Area, Card, CommandValue, ComponentValue, Dir, Pos, Side, Sound, Value};
use super::world::{LogEntry, Noise, RtResult, World};
use crate::model::Model;

/// A save: the game now, its checkpoint and its messages.
#[derive(Debug, Clone, PartialEq)]
pub struct Save {
    /// The `game` block's name; a save of another game does not load.
    pub game: String,
    /// When it was written among the store's saves: later is larger.
    pub stamp: u64,
    /// The session it belongs to (M 5.5: `delete_save(auto)`).
    pub session: u64,
    /// The `slots` locked for the session (M 5.4).
    pub slots: u32,
    pub now: Snapshot,
    pub checkpoint: Option<Snapshot>,
    /// The log's messages, presentation for the reading view (M 5.2).
    pub log: Vec<LogEntry>,
}

// ---- writing ----

/// Text being written: tokens with a space between them.
#[derive(Default)]
pub struct Out {
    text: String,
    /// No space before the next token.
    fresh: bool,
}

impl Out {
    fn token(&mut self, t: &str) {
        if !self.fresh && !self.text.is_empty() && !self.text.ends_with('\n') {
            self.text.push(' ');
        }
        self.text.push_str(t);
        self.fresh = false;
    }

    fn open(&mut self, tag: &str) {
        self.token("(");
        self.text.push_str(tag);
    }

    fn close(&mut self) {
        self.text.push(')');
    }

    fn word(&mut self, w: &str) {
        self.token(w);
    }

    fn text(&mut self, s: &str) {
        let mut q = String::from("\"");
        for c in s.chars() {
            match c {
                '"' => q.push_str("\\\""),
                '\\' => q.push_str("\\\\"),
                '\n' => q.push_str("\\n"),
                c => q.push(c),
            }
        }
        q.push('"');
        self.token(&q);
    }

    fn line(&mut self) {
        self.text.push('\n');
    }
}

/// A value the save holds.
pub trait Put {
    fn put(&self, w: &mut Out);
}

/// A value read back.
pub trait Get: Sized {
    fn get(r: &mut In) -> RtResult<Self>;
}

macro_rules! numbers {
    ($($t:ty),*) => {$(
        impl Put for $t {
            fn put(&self, w: &mut Out) {
                w.word(&self.to_string());
            }
        }
        impl Get for $t {
            fn get(r: &mut In) -> RtResult<Self> {
                let t = r.word()?;
                t.parse().map_err(|_| r.err(format!("a number, not `{t}`")))
            }
        }
    )*};
}
numbers!(i64, u64, u32, u8, usize, i32);

impl Put for bool {
    fn put(&self, w: &mut Out) {
        w.word(if *self { "yes" } else { "no" });
    }
}

impl Get for bool {
    fn get(r: &mut In) -> RtResult<Self> {
        match r.word()?.as_str() {
            "yes" => Ok(true),
            "no" => Ok(false),
            t => Err(r.err(format!("yes or no, not `{t}`"))),
        }
    }
}

impl Put for String {
    fn put(&self, w: &mut Out) {
        w.text(self);
    }
}

impl Get for String {
    fn get(r: &mut In) -> RtResult<Self> {
        r.text()
    }
}

impl<T: Put> Put for Option<T> {
    fn put(&self, w: &mut Out) {
        match self {
            Some(v) => v.put(w),
            None => w.word("~"),
        }
    }
}

impl<T: Get> Get for Option<T> {
    fn get(r: &mut In) -> RtResult<Self> {
        if r.peek() == Some(&Tok::Word("~".into())) {
            r.next();
            return Ok(None);
        }
        T::get(r).map(Some)
    }
}

impl<T: Put> Put for Box<T> {
    fn put(&self, w: &mut Out) {
        (**self).put(w)
    }
}

impl<T: Get> Get for Box<T> {
    fn get(r: &mut In) -> RtResult<Self> {
        T::get(r).map(Box::new)
    }
}

impl<T: Put> Put for Vec<T> {
    fn put(&self, w: &mut Out) {
        w.open("");
        for v in self {
            v.put(w);
        }
        w.close();
    }
}

impl<T: Get> Get for Vec<T> {
    fn get(r: &mut In) -> RtResult<Self> {
        r.open("")?;
        let mut out = Vec::new();
        while !r.at_close() {
            out.push(T::get(r)?);
        }
        r.close()?;
        Ok(out)
    }
}

impl<A: Put, B: Put> Put for (A, B) {
    fn put(&self, w: &mut Out) {
        self.0.put(w);
        self.1.put(w);
    }
}

impl<A: Get, B: Get> Get for (A, B) {
    fn get(r: &mut In) -> RtResult<Self> {
        Ok((A::get(r)?, B::get(r)?))
    }
}

impl<K: Put, V: Put> Put for BTreeMap<K, V> {
    fn put(&self, w: &mut Out) {
        w.open("");
        for (k, v) in self {
            k.put(w);
            v.put(w);
        }
        w.close();
    }
}

impl<K: Get + Ord, V: Get> Get for BTreeMap<K, V> {
    fn get(r: &mut In) -> RtResult<Self> {
        r.open("")?;
        let mut out = BTreeMap::new();
        while !r.at_close() {
            let k = K::get(r)?;
            out.insert(k, V::get(r)?);
        }
        r.close()?;
        Ok(out)
    }
}

impl<T: Put> Put for BTreeSet<T> {
    fn put(&self, w: &mut Out) {
        w.open("");
        for v in self {
            v.put(w);
        }
        w.close();
    }
}

impl<T: Get + Ord> Get for BTreeSet<T> {
    fn get(r: &mut In) -> RtResult<Self> {
        Ok(Vec::<T>::get(r)?.into_iter().collect())
    }
}

/// A form `(tag a b …)` of fields written in order.
macro_rules! form {
    ($t:ty, $tag:literal, $($f:ident),*) => {
        impl Put for $t {
            fn put(&self, w: &mut Out) {
                w.open($tag);
                $(self.$f.put(w);)*
                w.close();
            }
        }
        impl Get for $t {
            fn get(r: &mut In) -> RtResult<Self> {
                r.open($tag)?;
                let v = Self { $($f: Get::get(r)?),* };
                r.close()?;
                Ok(v)
            }
        }
    };
}

form!(Pos, "pos", map, x, y);
form!(Card, "card", entity, count, overrides);
form!(ComponentValue, "comp", name, fields);
form!(CommandValue, "cmd", name, args);
form!(Sound, "sound", kind, at, source, level);
form!(Noise, "noise", at, kind, source, time);
form!(LogEntry, "msg", key, args, speech);
form!(QueuedScene, "queued", call, opener, map);
form!(
    Background, "bg", handle, call, opener, map, scope_map, bind, realtime, base, st
);
form!(TlState, "tl", next, loops, gates, tweens, clock);
form!(LoopRun, "track", round, next, stopped);
form!(Tween, "tween", start, span, done, steps);

impl Put for Steps {
    fn put(&self, w: &mut Out) {
        match self {
            Steps::Move { e, route } => {
                w.open("move");
                (e, route).put(w);
            }
            Steps::Tiles { cells, tile, layer } => {
                w.open("tiles");
                (cells, (tile, layer)).put(w);
            }
        }
        w.close();
    }
}

impl Get for Steps {
    fn get(r: &mut In) -> RtResult<Self> {
        let tag = r.peek_tag().unwrap_or("").to_string();
        r.open(&tag)?;
        let s = match tag.as_str() {
            "move" => {
                let (e, route) = Get::get(r)?;
                Steps::Move { e, route }
            }
            "tiles" => {
                let (cells, (tile, layer)) = Get::get(r)?;
                Steps::Tiles { cells, tile, layer }
            }
            t => return Err(r.err(format!("no tween `{t}`"))),
        };
        r.close()?;
        Ok(s)
    }
}
form!(Wake, "wake", entity, at, seq);
form!(DeckState, "deck", draw, discard);
form!(Seat, "seat", n, members, active);
form!(Instance, "e", kind, components, place, alive);
form!(
    Game, "game", start, at, seed, time, call_depth, loop_steps, components
);
form!(PathLoop, "loop", loop_no, state);

impl Put for Dir {
    fn put(&self, w: &mut Out) {
        w.word(self.name());
    }
}

impl Get for Dir {
    fn get(r: &mut In) -> RtResult<Self> {
        let t = r.word()?;
        Dir::parse(&t).ok_or_else(|| r.err(format!("a direction, not `{t}`")))
    }
}

impl Put for Side {
    fn put(&self, w: &mut Out) {
        w.word(self.name());
    }
}

impl Get for Side {
    fn get(r: &mut In) -> RtResult<Self> {
        let t = r.word()?;
        Side::parse(&t).ok_or_else(|| r.err(format!("a side, not `{t}`")))
    }
}

impl Put for Area {
    fn put(&self, w: &mut Out) {
        w.open("area");
        // an area's cells are of one map, usually
        let map = self.cells().first().map_or(0, |p| p.map);
        map.put(w);
        for p in self.cells() {
            if p.map == map {
                (p.x, p.y).put(w);
            } else {
                p.put(w);
            }
        }
        w.close();
    }
}

impl Get for Area {
    fn get(r: &mut In) -> RtResult<Self> {
        r.open("area")?;
        let map = usize::get(r)?;
        let mut cells = Vec::new();
        while !r.at_close() {
            if r.at_open() {
                cells.push(<Pos as Get>::get(r)?);
            } else {
                let (x, y) = <(i32, i32)>::get(r)?;
                cells.push(Pos { map, x, y });
            }
        }
        r.close()?;
        Ok(Area::new(cells))
    }
}

impl Put for Rng {
    fn put(&self, w: &mut Out) {
        w.open("rng");
        for s in self.state() {
            s.put(w);
        }
        w.close();
    }
}

impl Get for Rng {
    fn get(r: &mut In) -> RtResult<Self> {
        r.open("rng")?;
        let s = [u64::get(r)?, u64::get(r)?, u64::get(r)?, u64::get(r)?];
        r.close()?;
        Ok(Rng::from_state(s))
    }
}

impl Put for Place {
    fn put(&self, w: &mut Out) {
        match self {
            Place::Nowhere => w.word("nowhere"),
            Place::Map(p) => p.put(w),
            Place::In {
                holder,
                component,
                field,
                slot,
            } => {
                w.open("in");
                (holder, (component, (field, slot))).put(w);
                w.close();
            }
        }
    }
}

impl Get for Place {
    fn get(r: &mut In) -> RtResult<Self> {
        if r.peek() == Some(&Tok::Word("nowhere".into())) {
            r.next();
            return Ok(Place::Nowhere);
        }
        match r.peek_tag() {
            Some("pos") => Ok(Place::Map(<Pos as Get>::get(r)?)),
            _ => {
                r.open("in")?;
                let (holder, (component, (field, slot))) = Get::get(r)?;
                r.close()?;
                Ok(Place::In {
                    holder,
                    component,
                    field,
                    slot,
                })
            }
        }
    }
}

impl Put for Value {
    fn put(&self, w: &mut Out) {
        let one = |w: &mut Out, tag: &str, v: &dyn Put| {
            w.open(tag);
            v.put(w);
            w.close();
        };
        match self {
            Value::Number(n) => one(w, "n", n),
            Value::Dice { n, sides, bonus } => one(w, "dice", &(n, (sides, bonus))),
            Value::Range(a, b) => one(w, "range", &(a, b)),
            Value::Percent(p) => one(w, "pct", p),
            Value::Bool(b) => one(w, "bool", b),
            Value::Text(t) => one(w, "t", t),
            Value::Name(n) => one(w, "name", n),
            Value::Dir(d) => one(w, "dir", d),
            Value::Side(s) => one(w, "side", s),
            Value::Pos(p) => p.put(w),
            Value::Area(a) => a.put(w),
            Value::Entity(e) => one(w, "ent", e),
            Value::Card(c) => c.put(w),
            Value::Collection(v) => one(w, "coll", v),
            Value::List(v) => one(w, "list", v),
            Value::Dict(v) => one(w, "dict", v),
            Value::Deck(n) => one(w, "deckname", n),
            Value::Table(n) => one(w, "table", n),
            Value::Command(c) => c.put(w),
            Value::Scene(c) => one(w, "scene", c),
            Value::Component(c) => c.put(w),
            Value::Sound(s) => s.put(w),
            Value::Time(ms) => one(w, "ms", ms),
            Value::Handle(h) => one(w, "handle", h),
            Value::Size(a, b) => one(w, "size", &(a, b)),
            Value::All => w.word("all"),
            Value::None => w.word("none"),
        }
    }
}

impl<T: Put + ?Sized> Put for &T {
    fn put(&self, w: &mut Out) {
        (**self).put(w)
    }
}

impl Get for Value {
    fn get(r: &mut In) -> RtResult<Self> {
        match r.peek() {
            Some(Tok::Word(w)) if w == "all" => {
                r.next();
                return Ok(Value::All);
            }
            Some(Tok::Word(w)) if w == "none" => {
                r.next();
                return Ok(Value::None);
            }
            _ => {}
        }
        let tag = r
            .peek_tag()
            .ok_or_else(|| r.err("a value".into()))?
            .to_string();
        // forms with their own `Get`
        match tag.as_str() {
            "pos" => return Ok(Value::Pos(<Pos as Get>::get(r)?)),
            "area" => return Ok(Value::Area(<Area as Get>::get(r)?)),
            "card" => return Ok(Value::Card(<Card as Get>::get(r)?)),
            "cmd" => return Ok(Value::Command(<CommandValue as Get>::get(r)?)),
            "comp" => return Ok(Value::Component(<ComponentValue as Get>::get(r)?)),
            "sound" => return Ok(Value::Sound(<Sound as Get>::get(r)?)),
            _ => {}
        }
        r.open(&tag)?;
        let v = match tag.as_str() {
            "n" => Value::Number(Get::get(r)?),
            "dice" => {
                let (n, (sides, bonus)) = Get::get(r)?;
                Value::Dice { n, sides, bonus }
            }
            "range" => {
                let (a, b) = Get::get(r)?;
                Value::Range(a, b)
            }
            "pct" => Value::Percent(Get::get(r)?),
            "bool" => Value::Bool(Get::get(r)?),
            "t" => Value::Text(Get::get(r)?),
            "name" => Value::Name(Get::get(r)?),
            "dir" => Value::Dir(Get::get(r)?),
            "side" => Value::Side(Get::get(r)?),
            "ent" => Value::Entity(Get::get(r)?),
            "coll" => Value::Collection(Get::get(r)?),
            "list" => Value::List(Get::get(r)?),
            "dict" => Value::Dict(Get::get(r)?),
            "deckname" => Value::Deck(Get::get(r)?),
            "table" => Value::Table(Get::get(r)?),
            "ms" => Value::Time(Get::get(r)?),
            "handle" => Value::Handle(Get::get(r)?),
            "scene" => Value::Scene(<CommandValue as Get>::get(r)?),
            "size" => {
                let (a, b) = Get::get(r)?;
                Value::Size(a, b)
            }
            t => return Err(r.err(format!("no value `{t}`"))),
        };
        r.close()?;
        Ok(v)
    }
}

impl Put for LoopState {
    fn put(&self, w: &mut Out) {
        match self {
            LoopState::Repeat { round, count } => {
                w.open("repeat");
                (round, count).put(w);
            }
            LoopState::For { items, index } => {
                w.open("for");
                (items, index).put(w);
            }
            LoopState::While => w.open("while"),
        }
        w.close();
    }
}

impl Get for LoopState {
    fn get(r: &mut In) -> RtResult<Self> {
        let tag = r.peek_tag().unwrap_or("").to_string();
        r.open(&tag)?;
        let s = match tag.as_str() {
            "repeat" => {
                let (round, count) = Get::get(r)?;
                LoopState::Repeat { round, count }
            }
            "for" => {
                let (items, index) = Get::get(r)?;
                LoopState::For { items, index }
            }
            "while" => LoopState::While,
            t => return Err(r.err(format!("no loop `{t}`"))),
        };
        r.close()?;
        Ok(s)
    }
}

const BODY_KINDS: [(BodyKind, &str); 8] = [
    (BodyKind::Function, "fn"),
    (BodyKind::Query, "query"),
    (BodyKind::Present, "present"),
    (BodyKind::Command, "command"),
    (BodyKind::Handler, "handler"),
    (BodyKind::Ai, "ai"),
    (BodyKind::Generator, "generator"),
    (BodyKind::Scene, "scene"),
];

impl Put for BodyKind {
    fn put(&self, w: &mut Out) {
        w.word(
            BODY_KINDS
                .iter()
                .find(|(k, _)| k == self)
                .map_or("handler", |(_, n)| n),
        );
    }
}

impl Get for BodyKind {
    fn get(r: &mut In) -> RtResult<Self> {
        let t = r.word()?;
        BODY_KINDS
            .iter()
            .find(|(_, n)| *n == t)
            .map(|(k, _)| *k)
            .ok_or_else(|| r.err(format!("no body kind `{t}`")))
    }
}

form!(
    Env, "env", locals, self_, holder, actor, it, kind, ns, map, rng, name, file
);
form!(Suspended, "waiting", command, hash, cost, point, loops, env);

/// A map's tiles, layer by layer, as runs of one tile: `(FLOOR 30)`.
struct Tiles<'a>(&'a [Vec<Option<String>>]);

impl Put for Tiles<'_> {
    fn put(&self, w: &mut Out) {
        w.open("");
        for layer in self.0 {
            w.open("");
            let mut i = 0;
            while i < layer.len() {
                let n = layer[i..].iter().take_while(|t| **t == layer[i]).count();
                (&layer[i], n).put(w);
                i += n;
            }
            w.close();
        }
        w.close();
    }
}

fn get_tiles(r: &mut In) -> RtResult<Vec<Vec<Option<String>>>> {
    r.open("")?;
    let mut layers = Vec::new();
    while !r.at_close() {
        r.open("")?;
        let mut layer = Vec::new();
        while !r.at_close() {
            let (t, n): (Option<String>, usize) = Get::get(r)?;
            layer.extend(std::iter::repeat_n(t, n));
        }
        r.close()?;
        layers.push(layer);
    }
    r.close()?;
    Ok(layers)
}

impl Put for MapState {
    fn put(&self, w: &mut Out) {
        w.open("map");
        (&self.name, (self.width, self.height)).put(w);
        self.layers.put(w);
        Tiles(&self.tiles).put(w);
        (&self.labels, &self.wires).put(w);
        (&self.vars, &self.remembered).put(w);
        (
            self.transient,
            (self.loaded_at, (self.frozen_total, self.frozen_at)),
        )
            .put(w);
        (self.tracking, &self.changed).put(w);
        w.close();
    }
}

impl Get for MapState {
    fn get(r: &mut In) -> RtResult<Self> {
        r.open("map")?;
        let (name, (width, height)) = Get::get(r)?;
        let layers = Get::get(r)?;
        let tiles = get_tiles(r)?;
        let (labels, wires) = Get::get(r)?;
        let (vars, remembered) = Get::get(r)?;
        let (transient, (loaded_at, (frozen_total, frozen_at))) = Get::get(r)?;
        // saves from before the changed cells do not know them, so their
        // maps are not merged with a new drawing (M 5.7)
        let (tracking, changed) = if r.at_close() {
            (false, vec![])
        } else {
            Get::get(r)?
        };
        r.close()?;
        Ok(MapState {
            name,
            width,
            height,
            layers,
            tiles,
            labels,
            wires,
            vars,
            remembered,
            transient,
            loaded_at,
            frozen_total,
            frozen_at,
            changed,
            tracking,
        })
    }
}

impl Put for State {
    fn put(&self, w: &mut Out) {
        w.open("state");
        self.game.put(w);
        w.line();
        w.fresh = true;
        w.word("(entities");
        for e in &self.entities {
            w.line();
            e.put(w);
        }
        w.close();
        w.line();
        w.word("(maps");
        for m in &self.maps {
            w.line();
            m.put(w);
        }
        w.close();
        w.line();
        (&self.wakes, self.next_seq).put(w);
        w.line();
        self.decks.put(w);
        w.line();
        (self.player, &self.seats).put(w);
        (self.party_each, (self.leash, self.seat_range)).put(w);
        (self.acting, self.last_player_map).put(w);
        w.line();
        (&self.profile, self.camera).put(w);
        (&self.scene_queue, self.scene_starts).put(w);
        w.line();
        (&self.backgrounds, self.next_handle).put(w);
        w.close();
    }
}

/// `(NAME` then members up to `)`.
fn get_list<T>(r: &mut In, tag: &str, one: fn(&mut In) -> RtResult<T>) -> RtResult<Vec<T>> {
    r.open(tag)?;
    let mut out = Vec::new();
    while !r.at_close() {
        out.push(one(r)?);
    }
    r.close()?;
    Ok(out)
}

impl Get for State {
    fn get(r: &mut In) -> RtResult<Self> {
        r.open("state")?;
        let game = <Game as Get>::get(r)?;
        let entities = get_list(r, "entities", <Instance as Get>::get)?;
        let maps = get_list(r, "maps", <MapState as Get>::get)?;
        let (wakes, next_seq) = Get::get(r)?;
        let decks = Get::get(r)?;
        let (player, seats) = Get::get(r)?;
        let (party_each, (leash, seat_range)) = Get::get(r)?;
        let (acting, last_player_map) = Get::get(r)?;
        let (profile, camera) = Get::get(r)?;
        let (scene_queue, scene_starts) = Get::get(r)?;
        // saves from before background scenes have none
        let (backgrounds, next_handle) = if r.at_close() {
            (vec![], 0)
        } else {
            Get::get(r)?
        };
        r.close()?;
        Ok(State {
            entities,
            game,
            maps,
            wakes,
            next_seq,
            decks,
            player,
            seats,
            party_each,
            acting,
            leash,
            seat_range,
            last_player_map,
            profile,
            camera,
            scene_queue,
            scene_starts,
            backgrounds,
            next_handle,
        })
    }
}

impl Put for Snapshot {
    fn put(&self, w: &mut Out) {
        w.open("snapshot");
        w.line();
        w.fresh = true;
        self.s.put(w);
        w.line();
        self.rng.put(w);
        (
            &self.sounds,
            (self.sounds_made, (&self.heard_mark, &self.heard)),
        )
            .put(w);
        w.line();
        (&self.last_view, self.switch).put(w);
        w.line();
        (&self.waiting, &self.woke).put(w);
        w.close();
    }
}

impl Get for Snapshot {
    fn get(r: &mut In) -> RtResult<Self> {
        r.open("snapshot")?;
        let s = <State as Get>::get(r)?;
        let rng = <Rng as Get>::get(r)?;
        let (sounds, (sounds_made, (heard_mark, heard))) = Get::get(r)?;
        let (last_view, switch) = Get::get(r)?;
        let (waiting, woke) = Get::get(r)?;
        r.close()?;
        Ok(Snapshot {
            s,
            rng,
            sounds,
            sounds_made,
            heard_mark,
            heard,
            last_view,
            switch,
            waiting,
            woke,
        })
    }
}

impl Save {
    pub fn write(&self) -> String {
        let mut w = Out::default();
        w.word("rlml-save 1");
        w.line();
        w.word("game");
        w.text(&self.game);
        w.line();
        w.word("stamp");
        self.stamp.put(&mut w);
        w.word("session");
        self.session.put(&mut w);
        w.word("slots");
        self.slots.put(&mut w);
        w.line();
        w.word("now");
        self.now.put(&mut w);
        w.line();
        w.word("checkpoint");
        self.checkpoint.put(&mut w);
        w.line();
        w.word("log");
        self.log.put(&mut w);
        w.line();
        w.text
    }

    pub fn read(text: &str) -> RtResult<Save> {
        let mut r = In::new(text)?;
        for w in ["rlml-save", "1"] {
            if r.word().ok().as_deref() != Some(w) {
                return Err("not a save: the first line is `rlml-save 1`".into());
            }
        }
        r.keyword("game")?;
        let game = r.text()?;
        r.keyword("stamp")?;
        let stamp = Get::get(&mut r)?;
        r.keyword("session")?;
        let session = Get::get(&mut r)?;
        r.keyword("slots")?;
        let slots = Get::get(&mut r)?;
        r.keyword("now")?;
        let now = <Snapshot as Get>::get(&mut r)?;
        r.keyword("checkpoint")?;
        let checkpoint = Get::get(&mut r)?;
        r.keyword("log")?;
        let log = Get::get(&mut r)?;
        if r.peek().is_some() {
            return Err(r.err("text after the save".into()));
        }
        Ok(Save {
            game,
            stamp,
            session,
            slots,
            now,
            checkpoint,
            log,
        })
    }
}

fn game_name(m: &Model) -> String {
    m.named_game().map(|g| g.name.clone()).unwrap_or_default()
}

impl<'m> Runner<'m> {
    /// The game as a save, between actions (M 5.2).
    pub fn save(&self) -> Save {
        Save {
            game: game_name(self.w.m),
            stamp: 0,
            session: 0,
            slots: 0,
            now: self.snapshot(),
            checkpoint: self.checkpoint.as_deref().cloned(),
            log: self.w.log.clone(),
        }
    }

    /// A runner that goes on from `save` (M 5.2): `begin` does not run, and
    /// the input log starts from the save (M 4.4).
    pub fn load(m: &'m Model, save: &Save) -> RtResult<Runner<'m>> {
        let name = game_name(m);
        if save.game != name {
            return Err(format!(
                "the save is of the game `{}`, not `{name}`",
                save.game
            ));
        }
        // maps merge with drawings changed since the save (M 5.7)
        let mut save = save.clone();
        let warnings = save.merge_maps(m);
        let save = &save;
        let mut r = Runner::new(World::new(m, save.now.s.game.seed));
        r.restore(&save.now);
        // the session's own view of the profile comes back (M 5.6)
        r.w.s.profile = save.now.s.profile.clone();
        r.w.profile_start = save.now.s.profile.clone();
        r.checkpoint = save.checkpoint.clone().map(Box::new);
        r.w.log = save.log.clone();
        r.inputs = InputLog {
            seed: r.w.s.game.seed,
            seats: r.w.s.seats.len() as u8,
            ..InputLog::default()
        };
        r.scene_saved = true;
        r.w.warnings.extend(warnings);
        let misplaced = r.misplaced();
        r.w.warnings.extend(misplaced);
        Ok(r)
    }
}

/// `v` in the save's text form, on one line.
pub fn write_value<T: Put>(v: &T) -> String {
    let mut w = Out::default();
    v.put(&mut w);
    w.text.replace('\n', " ")
}

/// A value in the save's text form.
pub fn read_value<T: Get>(text: &str) -> RtResult<T> {
    let mut r = In::new(text)?;
    let v = T::get(&mut r)?;
    if r.peek().is_some() {
        return Err(r.err("text after the value".into()));
    }
    Ok(v)
}

// ---- reading ----

#[derive(Debug, Clone, PartialEq)]
pub enum Tok {
    Open(String),
    Close,
    Word(String),
    Text(String),
}

/// Tokens being read.
pub struct In {
    toks: Vec<(Tok, usize)>,
    at: usize,
}

impl In {
    fn new(text: &str) -> RtResult<In> {
        let mut toks = Vec::new();
        let mut chars = text.char_indices().peekable();
        let line_of = |i: usize| text[..i].matches('\n').count() + 1;
        while let Some((i, c)) = chars.next() {
            match c {
                c if c.is_whitespace() => {}
                '(' => {
                    let mut tag = String::new();
                    while let Some(&(_, c)) = chars.peek() {
                        if c.is_whitespace() || c == '(' || c == ')' || c == '"' {
                            break;
                        }
                        tag.push(c);
                        chars.next();
                    }
                    toks.push((Tok::Open(tag), line_of(i)));
                }
                ')' => toks.push((Tok::Close, line_of(i))),
                '"' => {
                    let mut s = String::new();
                    loop {
                        match chars.next() {
                            Some((_, '"')) => break,
                            Some((_, '\\')) => match chars.next() {
                                Some((_, 'n')) => s.push('\n'),
                                Some((_, c)) => s.push(c),
                                None => {
                                    return Err(format!(
                                        "save line {}: a text is not closed",
                                        line_of(i)
                                    ));
                                }
                            },
                            Some((_, c)) => s.push(c),
                            None => {
                                return Err(format!(
                                    "save line {}: a text is not closed",
                                    line_of(i)
                                ));
                            }
                        }
                    }
                    toks.push((Tok::Text(s), line_of(i)));
                }
                c => {
                    let mut w = String::from(c);
                    while let Some(&(_, c)) = chars.peek() {
                        if c.is_whitespace() || c == '(' || c == ')' || c == '"' {
                            break;
                        }
                        w.push(c);
                        chars.next();
                    }
                    toks.push((Tok::Word(w), line_of(i)));
                }
            }
        }
        Ok(In { toks, at: 0 })
    }

    fn err(&self, what: String) -> String {
        let line = self
            .toks
            .get(self.at)
            .or(self.toks.last())
            .map_or(0, |t| t.1);
        format!("save line {line}: {what}")
    }

    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.at).map(|t| &t.0)
    }

    fn peek_tag(&self) -> Option<&str> {
        match self.peek() {
            Some(Tok::Open(t)) => Some(t),
            _ => None,
        }
    }

    fn next(&mut self) -> Option<Tok> {
        let t = self.toks.get(self.at).map(|t| t.0.clone());
        self.at += 1;
        t
    }

    fn at_close(&self) -> bool {
        matches!(self.peek(), Some(Tok::Close) | None)
    }

    fn at_open(&self) -> bool {
        matches!(self.peek(), Some(Tok::Open(_)))
    }

    fn open(&mut self, tag: &str) -> RtResult<()> {
        match self.peek() {
            Some(Tok::Open(t)) if t == tag => {
                self.next();
                Ok(())
            }
            other => Err(self.err(format!("`({tag}` expected, not {other:?}"))),
        }
    }

    fn close(&mut self) -> RtResult<()> {
        match self.peek() {
            Some(Tok::Close) => {
                self.next();
                Ok(())
            }
            other => Err(self.err(format!("`)` expected, not {other:?}"))),
        }
    }

    fn word(&mut self) -> RtResult<String> {
        match self.peek().cloned() {
            Some(Tok::Word(w)) => {
                self.next();
                Ok(w)
            }
            other => Err(self.err(format!("a word expected, not {other:?}"))),
        }
    }

    fn keyword(&mut self, k: &str) -> RtResult<()> {
        match self.word() {
            Ok(w) if w == k => Ok(()),
            _ => Err(self.err(format!("`{k}` expected"))),
        }
    }

    fn text(&mut self) -> RtResult<String> {
        match self.peek().cloned() {
            Some(Tok::Text(s)) => {
                self.next();
                Ok(s)
            }
            other => Err(self.err(format!("a text expected, not {other:?}"))),
        }
    }
}
