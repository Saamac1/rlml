//! The model and the state together: instances from data (M 3.2), cards
//! and draws (D 6), loading maps (D 8) and the new game (M 5.1).
//!
//! `created` events and `begin` need the interpreter; until it runs them,
//! instances whose `created` is due wait in [`World::created`] in the
//! order M 3.2 gives (members before their holder).

use super::rng::Rng;
use super::state::{DeckState, MapState, Place, State};
use super::value::{Area, Card, ComponentValue, Dir, EntityId, MapId, Pos, Side, Value};
use crate::fields::{self, Values, component_fields, is_dice, is_int, is_range};
use crate::model::{Def, Kind, Model, glyph_char};
use crate::names;
use crate::rs::types::{self, Ty};
use crate::syntax::{Modifier, RawBlock, RawItem};
use crate::token::{Token, tokenize};

/// The holder of `game`'s collection fields (D 5.5): `game` is not an
/// instance.
pub const GAME: EntityId = usize::MAX;

/// The default cost of a command (M 6.1.3).
pub const DEFAULT_COST: i64 = 120;

pub type RtResult<T> = Result<T, String>;

/// A map block's drawing, legend rows by character and `at:` rows.
type MapParts = (Vec<String>, Vec<(char, RawItem)>, Vec<RawItem>);

#[derive(Clone)]
pub struct World<'m> {
    pub m: &'m Model,
    pub s: State,
    /// The game stream (M 4.1).
    pub rng: Rng,
    /// Instances whose `created` event is due, in order (M 3.2 step 7).
    pub created: Vec<EntityId>,
    /// Runtime warnings, such as a full collection (D 5.4).
    pub warnings: Vec<String>,
    /// Log messages written so far (D 11.4).
    pub log: Vec<LogEntry>,
    /// Sounds made since the last wake (M 6.8).
    pub sounds: Vec<Noise>,
    /// Travels waiting for the action's end (M 6.1.7).
    pub travels: Vec<Travel>,
    /// `checkpoint`, `rollback`, `delete_save` and `autosave` calls waiting
    /// for the action's end (M 6.1.7 steps 2–5).
    pub saves: SaveCalls,
    /// Nested calls and events now (M 6.2, `game.call_depth`).
    pub depth: u32,
    /// The active map's change at the microstep's end: from and to.
    pub switch: Option<(MapId, MapId)>,
    /// Sounds ever made; `sounds` holds the newest of them.
    pub sounds_made: u64,
    /// Where each listener's wake interval starts in the sound count.
    pub heard_mark: std::collections::BTreeMap<EntityId, u64>,
    /// What each listener heard at its last wake, loudest first (M 6.8).
    pub heard: std::collections::BTreeMap<EntityId, Vec<super::value::Sound>>,
    /// The player's last view, for a dead player's `sees` (M 7.2).
    pub last_view: Option<(MapId, std::collections::BTreeSet<(i32, i32)>)>,
    /// The map area's written cells, read once (M 7.1).
    map_area: crate::model::MapArea,
    /// Tile looks by tile name, resolved once: the style rows do not change
    /// during a session, and drawing asks for every cell of every frame.
    pub(crate) looks: std::cell::RefCell<std::collections::HashMap<String, super::view::Look>>,
    /// Tile flags (`lit`, `forget`) by tile name and flag, resolved once
    /// for the same reason (D 7.5).
    pub(crate) flags: std::cell::RefCell<std::collections::HashMap<(String, String), bool>>,
    /// The shared view held while the presentation evaluates queries,
    /// which cannot change the state: `in_view` reads it instead of
    /// casting the view again for each call (M 7.7).
    pub(crate) held_view: Option<(MapId, std::collections::BTreeSet<(i32, i32)>)>,
}

/// The save calls of an action, done at its end (M 6.1.7).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SaveCalls {
    /// `checkpoint()` or `rollback()`: the last of them in the action
    /// decides (M 5.3).
    pub point: Option<Point>,
    /// `delete_save(checkpoint)`, or the checkpoint part of `all`.
    pub delete_checkpoint: bool,
    /// `delete_save(auto)`, or that part of `all`.
    pub delete_auto: bool,
    /// `autosave()`.
    pub autosave: bool,
    /// A member arrived on a map: the `on arrive` point (M 5.4).
    pub arrived: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Point {
    Checkpoint,
    Rollback,
}

/// A `log` call: the message key and its text arguments.
#[derive(Debug, Clone, PartialEq)]
pub struct LogEntry {
    pub key: String,
    pub args: Vec<(String, Value)>,
}

/// A `noise` call (M 6.8).
#[derive(Debug, Clone, PartialEq)]
pub struct Noise {
    pub at: Pos,
    pub kind: String,
    pub source: Option<EntityId>,
    pub time: i64,
}

/// A `travel` call, done at the action's end (M 6.1.7, M 6.7).
#[derive(Debug, Clone, PartialEq)]
pub struct Travel {
    pub who: EntityId,
    pub map: String,
    pub at: String,
    pub align: bool,
}

/// `viive = max(1, ceil(hinta × 10 / nopeus))`; no wake at speed 0 or less
/// (M 6.1.3).
pub fn delay(cost: i64, speed: i64) -> Option<i64> {
    (speed > 0).then(|| ((cost * 10 + speed - 1) / speed).max(1))
}

fn toks(text: &str) -> Vec<Token> {
    tokenize(text).unwrap_or_default()
}

impl<'m> World<'m> {
    pub fn new(m: &'m Model, seed: u64) -> World<'m> {
        let mut s = State::default();
        s.game.seed = seed;
        World { m, s, rng: Rng::from_seed(seed), created: vec![], warnings: vec![], log: vec![], sounds: vec![], travels: vec![], saves: SaveCalls::default(), depth: 0, switch: None, sounds_made: 0, heard_mark: Default::default(), heard: Default::default(), last_view: None, map_area: m.map_area(), looks: Default::default(), flags: Default::default(), held_view: None }
    }

    fn component_ns(&self, full: &str) -> Option<String> {
        self.m.defs.get(full).and_then(|d| d.full.rsplit_once('.').map(|(n, _)| n.to_string()))
    }

    /// A field's type, from the component's definition or the engine's.
    fn field_ty(&self, component: &str, field: &str) -> Ty {
        if names::is_vocabulary(component) {
            return crate::rs::sigs::engine_component_field(component, field).unwrap_or(Ty::Unknown);
        }
        let Some(def) = self.m.defs.get(component) else { return Ty::Unknown };
        let ns = self.component_ns(component);
        component_fields(def)
            .into_iter()
            .find(|f| f.name == field)
            .map(|f| types::parse(&f.ty, &|n| self.m.resolve(ns.as_deref(), n).is_ok_and(|d| d.kind == Kind::Component)).unwrap_or(Ty::Unknown))
            .unwrap_or(Ty::Unknown)
    }

    fn field_defs(&self, component: &str) -> Vec<fields::Field> {
        if names::is_vocabulary(component) {
            return fields::engine_component(component).unwrap_or_default();
        }
        self.m.defs.get(component).map(component_fields).unwrap_or_default()
    }

    // ---- data values (D 5.1) ----

    /// A data value of type `ty` as written; dice and ranges in number
    /// fields stay unrolled until the instance is created (M 3.2 step 4).
    /// Is the player's own save stopped: a member is on a `nosave` map
    /// (D 8.1, M 5.2)? Automatic saves and checkpoints go on.
    pub fn save_blocked(&self) -> bool {
        let members = self.s.players().into_iter().chain(self.s.player);
        members.filter_map(|e| self.s.at(e)).any(|p| {
            let name = &self.s.maps[p.map].name;
            self.m.defs.get(name).and_then(|d| d.block()).is_some_and(|b| b.has_arg("nosave"))
        })
    }

    pub fn data_value(&self, ty: &Ty, text: &str, ns: Option<&str>) -> RtResult<Value> {
        let t = text.trim();
        if t == "none" {
            return Ok(Value::None);
        }
        let words = toks(t);
        let one = words.first().map(|w| w.text.as_str()).unwrap_or("");
        Ok(match ty {
            Ty::Number | Ty::Dice | Ty::Percent | Ty::Range => number_value(one).ok_or_else(|| format!("`{t}` is not a number"))?,
            Ty::Bool => Value::Bool(one == "true"),
            Ty::Text => Value::Text(words.first().map(|w| w.text.clone()).unwrap_or_default()),
            Ty::Dir => Value::Dir(Dir::parse(one).ok_or_else(|| format!("`{one}` is not a direction"))?),
            Ty::Side => Value::Side(Side::parse(one).ok_or_else(|| format!("`{one}` is not a relative direction"))?),
            Ty::Label => Value::Name(one.to_string()),
            Ty::Name => Value::Name(self.full_name(one, ns)),
            Ty::Deck => Value::Deck(self.full_name(one, ns)),
            Ty::Table => Value::Table(self.full_name(one, ns)),
            Ty::Card => Value::Card(self.card(&words, ns)?),
            Ty::Area => Value::Area(Area::default()),
            Ty::List(m) => Value::List(words.iter().map(|w| self.data_value(m, &w.text, ns)).collect::<RtResult<_>>()?),
            Ty::Dict(k, v) => {
                let mut pairs = Vec::new();
                for pair in words.chunks(2) {
                    if let [a, b] = pair {
                        pairs.push((self.data_value(k, &a.text, ns)?, self.data_value(v, &b.text, ns)?));
                    }
                }
                Value::Dict(pairs)
            }
            _ => Value::None,
        })
    }

    /// The full name of a written name: a definition, or a tile with its
    /// variant and index (`STAIR.down` → `STAIR.down` of `STAIR`'s full
    /// name).
    pub fn full_name(&self, written: &str, ns: Option<&str>) -> String {
        self.m.full_name(written, ns)
    }

    /// The zero of a type: 0, `false`, `0,0`, `none`, empty (D 5.1).
    fn zero(ty: &Ty) -> Value {
        match ty {
            Ty::Number | Ty::Dice | Ty::Percent => Value::Number(0),
            Ty::Bool => Value::Bool(false),
            Ty::Text => Value::Text(String::new()),
            Ty::List(_) => Value::List(vec![]),
            Ty::Dict(..) => Value::Dict(vec![]),
            Ty::Area => Value::Area(Area::default()),
            Ty::Collection(_) => Value::Collection(vec![]),
            _ => Value::None,
        }
    }

    // ---- cards and draws (D 6) ----

    /// A data card: `TARGET [count] [COMPONENT values …]` (D 6.4).
    pub fn card(&self, words: &[Token], ns: Option<&str>) -> RtResult<Card> {
        let Some(first) = words.first() else { return Ok(Card { entity: None, count: None, overrides: vec![] }) };
        if first.text == "none" {
            return Ok(Card { entity: None, count: None, overrides: vec![] });
        }
        let target = self.full_name(&first.text, ns);
        let mut rest = &words[1..];
        let mut count = None;
        if let Some(n) = rest.first()
            && (n.text == "all" || number_value(&n.text).is_some())
        {
            count = Some(Box::new(if n.text == "all" { Value::All } else { number_value(&n.text).unwrap() }));
            rest = &rest[1..];
        }
        if rest.first().is_some_and(|t| t.text == "copy") {
            rest = &rest[1..];
        }
        let v = Values { m: self.m, ns };
        let is_comp = |w: &str| {
            if names::is_vocabulary(w) {
                fields::engine_component(w).is_some()
            } else {
                self.m.resolve(ns, w).is_ok_and(|d| d.kind == Kind::Component)
            }
        };
        let starts: Vec<usize> = (0..rest.len()).filter(|&i| is_comp(&rest[i].text)).collect();
        let mut overrides = Vec::new();
        for (k, &s) in starts.iter().enumerate() {
            let e = starts.get(k + 1).copied().unwrap_or(rest.len());
            let text = crate::fields::join(&rest[s..e]);
            let item = RawItem::new("", 0, ns.map(String::from), &text).map_err(|d| d.message)?;
            let row = v.row(&item)?;
            let cv = self.given_fields(&row.component, &row.values[..row.given], ns, &item)?;
            overrides.push((row.component.clone(), Value::Component(cv)));
        }
        Ok(Card { entity: Some(target), count, overrides })
    }

    /// A component value with only the fields given on a row.
    fn given_fields(&self, component: &str, given: &[(String, String)], ns: Option<&str>, item: &RawItem) -> RtResult<ComponentValue> {
        let mut cv = ComponentValue { name: component.to_string(), fields: vec![] };
        if component == "blocks" {
            let values = item.tokens[1..].iter().map(|t| Value::Name(t.text.clone())).collect();
            cv.fields.push(("values".into(), Value::List(values)));
            return Ok(cv);
        }
        for (f, text) in given {
            let ty = self.field_ty(component, f);
            // a collection's initial content is a draw, kept as written
            let v = if matches!(ty, Ty::Collection(_)) { Value::Text(text.clone()) } else { self.data_value(&ty, text, ns)? };
            cv.fields.push((f.clone(), v));
        }
        Ok(cv)
    }

    /// Draws `n` cards from a shared deck: the draw pile from the top, a
    /// `shuffled` deck reshuffling its discards when the pile runs out
    /// (D 6.2).
    pub fn draw_deck(&mut self, deck: &str, n: Option<usize>) -> RtResult<Vec<Card>> {
        let shuffled = self.m.defs.get(deck).and_then(Def::block).is_some_and(|b| b.has_arg("shuffled"));
        let mut out = Vec::new();
        let all = n.is_none();
        let want = n.unwrap_or(usize::MAX);
        while out.len() < want {
            let state = self.s.deck_mut(deck).ok_or_else(|| format!("no deck `{deck}`"))?;
            if state.draw.is_empty() {
                if all || !shuffled || state.discard.is_empty() {
                    break;
                }
                let mut pile = std::mem::take(&mut state.discard);
                self.rng.shuffle(&mut pile);
                self.s.deck_mut(deck).unwrap().draw = pile;
                continue;
            }
            let state = self.s.deck_mut(deck).unwrap();
            let card = state.draw.remove(0);
            state.discard.push(card.clone());
            out.push(card);
        }
        Ok(out)
    }

    /// Rolls a table and gives the row's cards (D 6.5).
    pub fn draw_table(&mut self, table: &str) -> RtResult<Vec<Card>> {
        let def = self.m.defs.get(table).ok_or_else(|| format!("no table `{table}`"))?;
        let b = def.block().ok_or_else(|| format!("`{table}` is not a data table"))?.clone();
        let ns = b.namespace.clone();
        let dice = b.args.first().map(|t| t.text.clone()).unwrap_or_default();
        let roll = match number_value(&dice) {
            Some(Value::Dice { n, sides, bonus }) => self.rng.dice(n, sides, bonus),
            Some(Value::Number(k)) => k,
            _ => return Err(format!("table `{table}` has no dice")),
        };
        for item in &b.items {
            let hit = match number_value(item.key()) {
                Some(Value::Number(k)) => k == roll,
                Some(Value::Range(a, b)) => roll >= a && b.is_none_or(|b| roll <= b),
                _ => false,
            };
            if hit {
                return Ok(vec![self.card(&item.tokens[1..], ns.as_deref())?]);
            }
        }
        Ok(vec![])
    }

    /// Spawns a card: its count of separate instances, a nested deck or
    /// table drawn (D 6.2, M 3.2). `place` gives each instance its place.
    pub fn spawn_card(&mut self, card: &Card, place: &dyn Fn(&State) -> Place) -> RtResult<Vec<EntityId>> {
        let Some(target) = &card.entity else { return Ok(vec![]) };
        let count = match card.count.as_deref() {
            None => 1,
            Some(Value::All) => usize::MAX,
            Some(v) => self.roll(v).max(0) as usize,
        };
        match self.m.kind(target) {
            Some(Kind::Deck) => {
                let cards = self.draw_deck(target, (count != usize::MAX).then_some(count))?;
                let mut out = Vec::new();
                for c in cards {
                    out.extend(self.spawn_card(&c, place)?);
                }
                Ok(out)
            }
            Some(Kind::Table) => {
                let mut out = Vec::new();
                for _ in 0..count.min(1000) {
                    for c in self.draw_table(target)? {
                        out.extend(self.spawn_card(&c, place)?);
                    }
                }
                Ok(out)
            }
            Some(Kind::Entity) => {
                let mut out = Vec::new();
                for _ in 0..count.min(1000) {
                    let p = place(&self.s);
                    out.push(self.spawn(target, &card.overrides, p)?);
                }
                Ok(out)
            }
            _ => Err(format!("`{target}` is not an entity, deck or table")),
        }
    }

    /// Rolls dice and ranges; numbers stay (M 3.2 step 4).
    pub fn roll(&mut self, v: &Value) -> i64 {
        match v {
            Value::Number(n) => *n,
            Value::Dice { n, sides, bonus } => self.rng.dice(*n, *sides, *bonus),
            Value::Range(a, Some(b)) => self.rng.range(*a, *b).unwrap_or(*a),
            Value::Percent(p) => *p,
            _ => 0,
        }
    }

    // ---- instances (M 3.2) ----

    /// The component rows of an entity through its `extends` chain: the
    /// child's row replaces the parent's for the same component, and
    /// `drop` removes it (D 5.2).
    fn entity_rows(&self, full: &str) -> RtResult<Vec<(String, RawItem)>> {
        let def = self.m.defs.get(full).ok_or_else(|| format!("no entity `{full}`"))?;
        let b = def.block().ok_or_else(|| format!("`{full}` is not a data entity"))?;
        let mut rows = match b.arg_after("extends") {
            Some(parent) => {
                let p = self.m.resolve_kind(b.namespace.as_deref(), parent, &[Kind::Entity])?;
                if p.full == full {
                    return Err(format!("`{full}` extends itself"));
                }
                self.entity_rows(&p.full.clone())?
            }
            None => vec![],
        };
        let v = Values { m: self.m, ns: b.namespace.as_deref() };
        for item in &b.items {
            let (marker, row) = item.marker();
            let comp = if names::is_vocabulary(row.key()) {
                row.key().to_string()
            } else {
                v.row(&row).map(|r| r.component).unwrap_or_else(|_| row.key().to_string())
            };
            match marker {
                Modifier::Drop => rows.retain(|(c, _)| c != &comp),
                _ => match rows.iter_mut().find(|(c, _)| c == &comp) {
                    Some(slot) => slot.1 = row,
                    None => rows.push((comp, row)),
                },
            }
        }
        Ok(rows)
    }

    /// Creates an instance of entity `full` with card overrides at `place`
    /// (M 3.2 steps 1–8; step 7, `created`, is queued).
    pub fn spawn(&mut self, full: &str, overrides: &[(String, Value)], place: Place) -> RtResult<EntityId> {
        // 1. components through the inheritance chain
        let rows = self.entity_rows(full)?;
        let mut comps = Vec::new();
        for (comp, item) in &rows {
            let ns = item.namespace.as_deref();
            let row = Values { m: self.m, ns }.row(item).map_err(|e| format!("{}:{}: {e}", item.file, item.line))?;
            comps.push((comp.clone(), self.given_fields(comp, &row.values[..row.given], ns, item)?, item.namespace.clone()));
        }
        // 2. overrides
        for (comp, value) in overrides {
            let given = match value {
                Value::Component(cv) => cv.clone(),
                // a one-field component's value given directly (RS 2.8)
                v => {
                    let field = self.field_defs(comp).first().map(|f| f.name.clone()).unwrap_or_default();
                    ComponentValue { name: comp.clone(), fields: vec![(field, v.clone())] }
                }
            };
            match comps.iter_mut().find(|(c, _, _)| c == comp) {
                Some((_, cv, _)) => {
                    for (f, v) in given.fields {
                        cv.set(&f, v);
                    }
                }
                None => comps.push((comp.clone(), given, self.component_ns(comp))),
            }
        }
        // 3.–4. defaults, zeros and rolls
        let mut done = Vec::new();
        let mut collections = Vec::new();
        for (comp, mut cv, ns) in comps {
            for f in self.field_defs(&comp) {
                let ty = self.field_ty(&comp, &f.name);
                if cv.get(&f.name).is_none() {
                    let v = match &f.default {
                        // `default now`: another field of the same component (D 5.1)
                        Some(d) if cv.get(d).is_some() => cv.get(d).cloned().unwrap(),
                        Some(d) if matches!(ty, Ty::Collection(_)) => Value::Text(d.clone()),
                        Some(d) => self.data_value(&ty, d, ns.as_deref())?,
                        None => Self::zero(&ty),
                    };
                    cv.set(&f.name, v);
                }
                if ty == Ty::Number
                    && let Some(v) = cv.get(&f.name).cloned()
                    && matches!(v, Value::Dice { .. } | Value::Range(..))
                {
                    let n = self.roll(&v);
                    cv.set(&f.name, Value::Number(n));
                }
                if matches!(ty, Ty::Collection(_))
                    && let Some(Value::Text(draw)) = cv.get(&f.name).cloned()
                {
                    collections.push((comp.clone(), f.name.clone(), draw, ns.clone()));
                    cv.set(&f.name, Value::Collection(vec![]));
                }
            }
            done.push(cv);
        }
        let e = self.s.create(full);
        self.s.get_mut(e).components = done;
        // 5. collections: members are created first, so their `created` comes first
        for (comp, field, draw, ns) in collections {
            self.fill(e, &comp, &field, &draw, ns.as_deref())?;
        }
        // 6. the place
        self.s.get_mut(e).place = place.clone();
        // 7. `created`
        self.created.push(e);
        // a data member `- seat N` joins its seat if that seat is joined (D 5.3)
        if let Some(v) = self.s.get(e).get("seat").and_then(|c| c.get("n")).cloned() {
            // a seat that is not joined leaves no stray component
            self.s.get_mut(e).remove("seat");
            self.s.set_seat(e, crate::rt::state::State::seat_number(&v));
        }
        // 8. the wake queue for an actor on a map
        if let Place::Map(p) = place
            && self.s.is_actor(e)
        {
            self.wake_new(e, p.map);
        }
        Ok(e)
    }

    /// An actor that comes onto a map wakes after the default cost's delay
    /// (M 6.1.2).
    pub fn wake_new(&mut self, e: EntityId, map: MapId) {
        if let Some(d) = delay(DEFAULT_COST, self.speed(e)) {
            let now = self.s.maps[map].clock(self.s.game.time);
            self.s.schedule(e, now + d);
        }
    }

    /// `movement.speed`, 10 without it (M 6.1.3).
    pub fn speed(&self, e: EntityId) -> i64 {
        self.s.get(e).get("movement").and_then(|m| m.get("speed")).and_then(Value::as_number).unwrap_or(10)
    }

    /// Fills a collection field with its initial content: a draw from a
    /// deck or table, or an entity card (D 5.4).
    fn fill(&mut self, holder: EntityId, comp: &str, field: &str, draw: &str, ns: Option<&str>) -> RtResult<()> {
        let words = toks(draw);
        if words.is_empty() {
            return Ok(());
        }
        let card = self.card(&words, ns)?;
        let (c, f) = (comp.to_string(), field.to_string());
        let size = self.field_defs(comp).into_iter().find(|fd| fd.name == field).and_then(|fd| fd.ty.split_whitespace().nth(1).and_then(|n| n.parse::<usize>().ok()));
        let members = self.spawn_card(&card, &|s: &State| Place::In { holder, component: c.clone(), field: f.clone(), slot: s.free_slot(holder, &c, &f) })?;
        if let Some(n) = size {
            for extra in members.iter().skip(n) {
                self.s.get_mut(*extra).alive = false;
                self.s.get_mut(*extra).place = Place::Nowhere;
                self.created.retain(|x| x != extra);
                self.warnings.push(format!("collection `{comp}.{field}` is full ({n} slots); a member was left out (D 5.4)"));
            }
        }
        Ok(())
    }

    // ---- maps (D 8) ----

    /// Loads a drawn map once; a loaded map is kept (D 8.1).
    pub fn load_map(&mut self, full: &str) -> RtResult<MapId> {
        if let Some(id) = self.s.map_id(full) {
            return Ok(id);
        }
        let def = self.m.defs.get(full).ok_or_else(|| format!("no map `{full}`"))?;
        let b = def.block().ok_or_else(|| format!("map `{full}` is generated; generators are not supported yet"))?.clone();
        let (drawing, mut legend, at_rows) = self.map_parts(&b)?;
        if let Some(base) = b.arg_after("extends") {
            let base = self.m.resolve_kind(b.namespace.as_deref(), base, &[Kind::Map])?;
            let (_, base_legend, _) = self.map_parts(base.block().unwrap())?;
            let own: Vec<char> = legend.iter().map(|(c, _)| *c).collect();
            legend = base_legend.into_iter().filter(|(c, _)| !own.contains(c)).chain(legend).collect();
        }
        let size = b.arg_after("size").and_then(|s| s.split_once('x')).and_then(|(w, h)| Some((w.parse::<i32>().ok()?, h.parse::<i32>().ok()?)));
        let (w, h) = size.unwrap_or((drawing.iter().map(|r| r.chars().count()).max().unwrap_or(0) as i32, drawing.len() as i32));
        let layers = self.layers();
        let default_layer = self.default_layer();
        let mut map = MapState::new(full, w, h, layers.iter().map(|(n, _, _)| n.clone()).collect(), self.s.game.time);
        map.transient = b.has_arg("transient");
        // layer fills (D 7.2)
        for (i, (_, _, fill)) in layers.iter().enumerate() {
            if let Some(t) = fill {
                for y in 0..h {
                    for x in 0..w {
                        map.set_tile(i, x, y, Some(t.clone()));
                    }
                }
            }
        }
        for item in b.sub("vars") {
            let ty = literal_ty(item.rest(1));
            map.vars.push((item.key().to_string(), self.data_value(&ty, item.rest(1), item.namespace.as_deref())?));
        }
        self.s.maps.push(map);
        let id = self.s.maps.len() - 1;
        // cells in reading order: tiles, labels, wires and `with` entities (M 4.2)
        let mut spawns: Vec<(Pos, RawItem)> = Vec::new();
        for (y, row) in drawing.iter().enumerate() {
            for (x, c) in row.chars().enumerate() {
                if let Some((_, rest)) = legend.iter().find(|(lc, _)| *lc == c) {
                    let p = Pos { map: id, x: x as i32, y: y as i32 };
                    self.place_row(p, rest, default_layer, &mut spawns)?;
                }
            }
        }
        for item in &at_rows {
            let (x, y) = item.key().split_once(',').and_then(|(x, y)| Some((x.parse::<i32>().ok()?, y.parse::<i32>().ok()?))).ok_or("bad `at:` row")?;
            self.place_row(Pos { map: id, x, y }, &item.shift(1), default_layer, &mut spawns)?;
        }
        spawns.sort_by_key(|(p, _)| (p.y, p.x));
        for (p, with) in spawns {
            let card = self.card(&with.tokens, with.namespace.as_deref())?;
            self.spawn_card(&card, &|_| Place::Map(p))?;
        }
        Ok(id)
    }

    /// Drawing, legend by character and `at:` rows of a map block.
    fn map_parts(&self, b: &RawBlock) -> RtResult<MapParts> {
        let mut legend = Vec::new();
        for item in b.sub("legend") {
            let (c, rest) = glyph_char(item)?;
            legend.retain(|(x, _): &(char, RawItem)| *x != c);
            legend.push((c, rest));
        }
        if !legend.iter().any(|(c, _)| *c == ' ') {
            legend.push((' ', RawItem::new(&b.file, b.line, b.namespace.clone(), "none").map_err(|d| d.message)?));
        }
        Ok((b.drawing.clone(), legend, b.sub("at").cloned().collect()))
    }

    /// Terrain layers in order: name, `default` and `fill` (D 7.2); one
    /// unnamed layer without a `terrain` section.
    fn layers(&self) -> Vec<(String, bool, Option<String>)> {
        let rows: Vec<&RawItem> = self.m.section_rows("terrain").map(|(_, i)| i).collect();
        if rows.is_empty() {
            return vec![(String::new(), true, None)];
        }
        rows.iter()
            .map(|i| {
                let ns = i.namespace.as_deref();
                let fill = i.tokens.iter().position(|t| t.is("fill")).and_then(|k| i.word(k + 1)).map(|t| self.full_name(t, ns));
                (names::qualify(ns, i.key()), i.tokens.iter().any(|t| t.is("default")), fill)
            })
            .collect()
    }

    fn default_layer(&self) -> usize {
        self.layers().iter().position(|(_, d, _)| *d).unwrap_or(0)
    }

    /// Applies a legend or `at:` row to a cell: tiles by layer, label, wire,
    /// and `with` entities to spawn after the tiles (D 8.2, D 8.9).
    fn place_row(&mut self, p: Pos, rest: &RawItem, default_layer: usize, spawns: &mut Vec<(Pos, RawItem)>) -> RtResult<()> {
        let ns = rest.namespace.clone();
        let words: Vec<&str> = rest.tokens.iter().map(|t| t.text.as_str()).collect();
        let kw = |w: &str| ["with", "as", "wire"].contains(&w);
        let tile_end = words.iter().position(|w| kw(w)).unwrap_or(words.len());
        let tiles = &words[..tile_end];
        let layer_names: Vec<String> = self.s.maps[p.map].layers.clone();
        let mut i = 0;
        if tiles.len() % 2 == 1 {
            let t = (tiles[0] != "none").then(|| self.full_name(tiles[0], ns.as_deref()));
            self.s.maps[p.map].set_tile(default_layer, p.x, p.y, t);
            i = 1;
        }
        while i + 1 < tiles.len() {
            let layer = self.full_name(tiles[i], ns.as_deref());
            if let Some(l) = layer_names.iter().position(|n| *n == layer) {
                let t = (tiles[i + 1] != "none").then(|| self.full_name(tiles[i + 1], ns.as_deref()));
                self.s.maps[p.map].set_tile(l, p.x, p.y, t);
            }
            i += 2;
        }
        let mut k = tile_end;
        while k < words.len() {
            let end = (k + 1..words.len()).find(|&j| kw(words[j])).unwrap_or(words.len());
            match words[k] {
                "with" => spawns.push((p, rest.shift(k + 1).clone_until(end - k - 1))),
                "as" => {
                    let label = words[k + 1..end].last().copied().unwrap_or("");
                    self.s.maps[p.map].add_label(label, p.x, p.y);
                }
                "wire" => {
                    let wire = words[k + 1..end].last().copied().unwrap_or("").to_string();
                    self.s.maps[p.map].wires.push(((p.x, p.y), wire));
                }
                _ => {}
            }
            k = end;
        }
        Ok(())
    }

    // ---- the new game (M 5.1) ----

    /// Computes a new game up to `begin` (M 5.1 steps 1–7). `seed` is used
    /// when the `game` block has none.
    pub fn new_game(m: &'m Model, seed: u64) -> RtResult<World<'m>> {
        Self::new_game_seats(m, seed, None)
    }

    /// `new_game` with `seats` joined seats from the join screen (M 8.2);
    /// `None` joins the game's lower bound. The count is clamped to the
    /// game's `seats a..b`.
    pub fn new_game_seats(m: &'m Model, seed: u64, seats: Option<u8>) -> RtResult<World<'m>> {
        let g = m.named_game().ok_or("no named `game` block")?.clone();
        let field = |name: &str| g.items.iter().find(|i| i.key() == name).map(|i| i.rest(1).to_string());
        // 1. the seed
        let seed = field("seed").and_then(|s| s.parse::<i64>().ok()).map(|s| s as u64).unwrap_or(seed);
        let mut w = World::new(m, seed);
        let (lo, hi) = field("seats").as_deref().and_then(seat_range).unwrap_or((1, 1));
        // the checker keeps 1 <= lo <= hi <= 8 (D 3.3); a model that skipped
        // it gets its bounds in order, starting from seat 1 at least
        let (lo, hi) = (lo.min(hi).max(1), lo.max(hi).max(1));
        for n in 1..=seats.unwrap_or(lo).clamp(lo, hi) {
            w.s.join_seat(n);
        }
        w.s.party_each = field("party").as_deref() == Some("each");
        w.s.leash = field("leash").map_or(hi > 1, |l| l == "on");
        w.s.seat_range = (lo, hi);
        w.s.game.call_depth = field("call_depth").and_then(|s| s.parse().ok()).unwrap_or(256);
        w.s.game.loop_steps = field("loop_steps").and_then(|s| s.parse().ok()).unwrap_or(1_000_000);
        // 2. the profile is not supported yet
        // 3. decks: written order, `shuffled` ones shuffled by full name
        let decks: Vec<(String, RawBlock)> = m.defs.values().filter(|d| d.kind == Kind::Deck).filter_map(|d| d.block().map(|b| (d.full.clone(), b.clone()))).collect();
        for (name, b) in decks {
            let cards = b.items.iter().map(|i| w.card(&i.tokens, b.namespace.as_deref())).collect::<RtResult<Vec<_>>>()?;
            let mut state = DeckState { draw: cards, discard: vec![] };
            if b.has_arg("shuffled") {
                w.rng.shuffle(&mut state.draw);
            }
            w.s.decks.push((name, state));
        }
        // 4. the `game` block's components and collections
        let games: Vec<RawBlock> = m.games.clone();
        for gb in &games {
            for item in gb.items.iter().filter(|i| !names::is_vocabulary(i.key())) {
                let ns = item.namespace.as_deref();
                let row = Values { m, ns }.row(item)?;
                let mut cv = w.given_fields(&row.component, &row.values[..row.given], ns, item)?;
                for f in w.field_defs(&row.component) {
                    if cv.get(&f.name).is_none() {
                        let ty = w.field_ty(&row.component, &f.name);
                        let v = match &f.default {
                            Some(d) => w.data_value(&ty, d, ns)?,
                            None => Self::zero(&ty),
                        };
                        cv.set(&f.name, v);
                    }
                }
                let collections: Vec<(String, String)> =
                    cv.fields.iter().filter_map(|(f, v)| if let Value::Text(t) = v && matches!(w.field_ty(&row.component, f), Ty::Collection(_)) { Some((f.clone(), t.clone())) } else { None }).collect();
                for (f, _) in &collections {
                    cv.set(f, Value::Collection(vec![]));
                }
                match w.s.game.components.iter_mut().find(|c| c.name == cv.name) {
                    Some(old) => *old = cv,
                    None => w.s.game.components.push(cv),
                }
                for (f, draw) in collections {
                    w.fill(GAME, &row.component, &f, &draw, ns)?;
                }
            }
        }
        // 5. the start map
        let ns = g.namespace.as_deref();
        let start = field("start").ok_or("the game has no `start`")?;
        let start_def = m.resolve(ns, &start)?;
        let start_map = match start_def.kind {
            Kind::Map => start_def.full.clone(),
            Kind::Deck | Kind::Table => {
                let card = w.card(&toks(&start), ns)?;
                let cards = match start_def.kind {
                    Kind::Deck => w.draw_deck(&start_def.full, Some(1))?,
                    _ => w.draw_table(&card.entity.clone().unwrap_or_default())?,
                };
                cards.first().and_then(|c| c.entity.clone()).ok_or("the start draw gave no map")?
            }
            k => return Err(format!("`start` is a {k}")),
        };
        w.s.game.start = Some(start_map.clone());
        // 6. load it
        let map = w.load_map(&start_map)?;
        // 7. the start search
        let at = field("at").unwrap_or_else(|| "START".into());
        w.s.game.at = w.search(map, &at, ns)?;
        if w.s.game.at.is_empty() {
            return Err(format!("the start search `at {at}` finds no cell on map `{start_map}` (D 3.3)"));
        }
        Ok(w)
    }

    /// A data search on a map: a position, a label or a tile (D 8.8).
    pub fn search(&self, map: MapId, target: &str, ns: Option<&str>) -> RtResult<Area> {
        let words = toks(target);
        let first = words.first().map(|t| t.text.as_str()).unwrap_or("");
        if words.len() > 1 {
            return Err(format!("`at {target}`: `near` and `from` searches are not supported yet"));
        }
        let m = &self.s.maps[map];
        if let Some((x, y)) = first.split_once(',').and_then(|(x, y)| Some((x.parse().ok()?, y.parse().ok()?))) {
            return Ok(Area::one(Pos { map, x, y }));
        }
        let label = m.label(first);
        if !label.is_empty() {
            return Ok(Area::new(label.iter().map(|(x, y)| Pos { map, x: *x, y: *y }).collect()));
        }
        let tile = self.full_name(first, ns);
        let mut cells = Vec::new();
        for y in 0..m.height {
            for x in 0..m.width {
                if (0..m.layers.len()).any(|l| m.tile(l, x, y).is_some_and(|t| t == tile || t.starts_with(&format!("{tile}.")))) {
                    cells.push(Pos { map, x, y });
                }
            }
        }
        Ok(Area::new(cells))
    }
}

impl World<'_> {
    /// The layout's map area, read once (D 11.1); the front end draws the
    /// map at its written size and border.
    pub fn map_area(&self) -> crate::model::MapArea {
        self.map_area
    }

    /// `view_width()` and `view_height()` (M 7.1): the map area's cells
    /// inside its border as the layout's legend writes them (`size.width
    /// N`, `size.height N`, `size WxH`). They come from data, not from the
    /// screen, so a game runs the same at any window size (M 4). An axis
    /// without a written cell count is the active map's own size.
    pub fn view_dims(&self) -> (i32, i32) {
        let map = self.s.active_map().map(|m| (self.s.maps[m].width, self.s.maps[m].height)).unwrap_or((0, 0));
        let (w, h) = self.map_area.cells();
        (w.unwrap_or(map.0), h.unwrap_or(map.1))
    }
}

/// `seats a..b` (D 3.3), or a single number for `a..a`.
pub fn seat_range(s: &str) -> Option<(u8, u8)> {
    let (a, b) = s.split_once("..").unwrap_or((s, s));
    Some((a.trim().parse().ok()?, b.trim().parse().ok()?))
}

/// A number, dice or range value as written (`3`, `1d6+2`, `-1..3`, `50%`).
pub fn number_value(s: &str) -> Option<Value> {
    if is_int(s) {
        return s.parse().ok().map(Value::Number);
    }
    if let Some(p) = s.strip_suffix('%').filter(|p| is_int(p)) {
        return p.parse().ok().map(Value::Percent);
    }
    if is_range(s) {
        let (a, b) = s.split_once("..")?;
        return Some(Value::Range(a.parse().ok()?, if b.is_empty() { None } else { Some(b.parse().ok()?) }));
    }
    if is_dice(s) {
        let (body, bonus) = match s.find(['+', '-']) {
            Some(i) if i > 0 => (&s[..i], s[i..].parse::<i64>().ok()?),
            _ => (s, 0),
        };
        let (n, sides) = body.split_once('d')?;
        return Some(Value::Dice { n: if n.is_empty() { 1 } else { n.parse().ok()? }, sides: sides.parse().ok()?, bonus });
    }
    None
}

fn literal_ty(v: &str) -> Ty {
    if number_value(v).is_some() {
        Ty::Number
    } else if v == "true" || v == "false" {
        Ty::Bool
    } else if v.starts_with('"') {
        Ty::Text
    } else {
        Ty::Name
    }
}

trait CloneUntil {
    fn clone_until(&self, n: usize) -> RawItem;
}

impl CloneUntil for RawItem {
    /// The item's first `n` words.
    fn clone_until(&self, n: usize) -> RawItem {
        let n = n.min(self.tokens.len());
        let text = self.span(0, n).to_string();
        RawItem::new(&self.file, self.line, self.namespace.clone(), &text).unwrap_or_else(|_| self.clone())
    }
}
