//! Load-time checks (M 2.5): everything that can be known without the
//! game's state. Each check reports where the problem was written.

use std::collections::{BTreeMap, BTreeSet};

use unicode_width::UnicodeWidthChar;

use crate::error::Report;
use crate::fields::{self, Values, component_fields, is_int, is_pos};
use crate::model::{Def, Kind, Model, glyph_char};
use crate::names::{self, Index};
use crate::rogue::DeclKind;
use crate::syntax::{RawBlock, RawItem};
use crate::token::Token;

/// Engine actions a key, menu item or `{^…}` may name (M 9.1).
pub const ENGINE_ACTIONS: &[&str] = &[
    "save", "quit", "pass", "repeat", "help", "confirm", "back", "close", "resume", "skip", "restart", "rewind", "new_game", "load_game",
    "exit", "next", "prev", "toggle", "travel_to", "look", "join", "leave_seat",
];

/// Engine events (M 9.1, M 9.6).
pub const ENGINE_EVENTS: &[&str] = &["begin", "turn", "wake", "created", "removed", "enter", "leave", "arrive", "depart", "hear", "join", "part"];

const GAME_FIELDS: &[&str] =
    &["start", "at", "seed", "call_depth", "loop_steps", "keys", "title", "layout", "naming", "walk", "walk_stop", "masking", "seats", "party", "leash"];

const SETTINGS: &[&str] = &["language", "style", "round", "idle", "external", "external_timeout", "mouse", "targeting"];

/// A drawn map or prefab after its legend has been read.
#[derive(Debug, Default)]
struct Drawn {
    width: usize,
    height: usize,
    labels: BTreeSet<String>,
}

pub fn check(m: &Model, main: &str, r: &mut Report) {
    let logic_tiles = crate::rs::check::check(m, r);
    let mut c = Checker { m, r, drawn: BTreeMap::new(), used_tiles: logic_tiles };
    c.components();
    c.maps();
    c.entities();
    c.labels();
    c.game(main);
    c.settings(main);
    c.tiles_section();
    c.style();
    c.layouts();
    c.logs();
    c.texts();
    c.keys();
    c.menus();
    c.code();
    c.std_shadowing();
    c.unused_tiles();
}

struct Checker<'a> {
    m: &'a Model,
    r: &'a mut Report,
    drawn: BTreeMap<String, Drawn>,
    used_tiles: BTreeSet<String>,
}

fn defs_of(m: &Model, kind: Kind) -> impl Iterator<Item = &Def> {
    m.defs.values().filter(move |d| d.kind == kind)
}

impl Checker<'_> {
    fn values<'b>(&'b self, ns: Option<&'b str>) -> Values<'b> {
        Values { m: self.m, ns }
    }

    fn components(&mut self) {
        for def in defs_of(self.m, Kind::Component) {
            let Some(b) = def.block() else { continue };
            if names::is_vocabulary(&b.name) || fields::engine_component(&def.full).is_some() {
                self.r.push(b.err(format!("`{}` is an engine component; give your own component an upper-case name", b.name)));
            }
            let fields = component_fields(def);
            for (item, f) in b.items.iter().zip(&fields) {
                if !names::is_vocabulary(&f.name) && !names::is_name(&f.name) {
                    self.r.push(item.err(format!("bad field name `{}`", f.name)));
                }
                if let Err(e) = fields::check_type(&f.ty) {
                    self.r.push(item.err(e));
                    continue;
                }
                // the same type reading as RogueScript's (D 5.1 = RS 2.6)
                let ns = b.namespace.as_deref();
                if let Err(e) = crate::rs::types::parse(&f.ty, &|n| self.m.resolve(ns, n).is_ok_and(|d| d.kind == Kind::Component)) {
                    self.r.push(item.err(e));
                    continue;
                }
                if let Some(d) = &f.default {
                    // a default may name another field of the same component
                    if fields.iter().any(|o| &o.name == d) {
                        continue;
                    }
                    let toks = crate::token::tokenize(d).unwrap_or_default();
                    let v = self.values(b.namespace.as_deref());
                    let res = if f.base_type() == "collection" { v.draw(&toks, &[Kind::Deck, Kind::Table, Kind::Entity]) } else { v.value(&f.ty, &toks) };
                    if let Err(e) = res {
                        self.r.push(item.err(format!("default of `{}`: {e}", f.name)));
                    }
                }
            }
        }
    }

    fn entities(&mut self) {
        for def in defs_of(self.m, Kind::Entity) {
            let Some(b) = def.block() else { continue };
            let ns = b.namespace.as_deref();
            if let Some(parent) = b.arg_after("extends") {
                match self.m.resolve_kind(ns, parent, &[Kind::Entity]) {
                    Ok(_) => {
                        if let Some(cycle) = self.extends_cycle(def, Kind::Entity) {
                            self.r.push(b.err(format!("`extends` cycle: {cycle}")));
                        }
                    }
                    Err(e) => self.r.push(b.err(e)),
                }
            } else if !b.args.is_empty() {
                self.r.push(b.err(format!("unexpected `{}` in the entity header", b.args_text)));
            }
            for item in &b.items {
                let (marker, row) = item.marker();
                if marker == crate::syntax::Modifier::Drop {
                    continue;
                }
                self.component_row(&row, ns);
            }
        }
    }

    /// Checks a component row and, for a component with a `label` field and
    /// a map field, the label in that map (D 5.1).
    fn component_row(&mut self, item: &RawItem, ns: Option<&str>) {
        let row = match self.values(ns).row(item) {
            Ok(row) => row,
            Err(e) => {
                self.r.push(item.err(e));
                return;
            }
        };
        if row.component == "render"
            && let Some(t) = row.get("tile")
            && let Ok(full) = self.m.resolve_tile(ns, t)
        {
            self.used_tiles.insert(full);
        }
        let Some(def) = self.m.defs.get(&row.component) else { return };
        let fields = component_fields(def);
        let maps: Vec<&str> = fields
            .iter()
            .filter(|f| f.base_type() == "name")
            .filter_map(|f| row.get(&f.name))
            .filter(|v| self.m.resolve(ns, v).is_ok_and(|d| d.kind == Kind::Map))
            .collect();
        let [map] = maps.as_slice() else { return };
        let map_full = self.m.resolve(ns, map).unwrap().full.clone();
        let Some(drawn) = self.drawn.get(&map_full) else { return };
        for f in fields.iter().filter(|f| f.base_type() == "label") {
            if let Some(label) = row.get(&f.name)
                && !drawn.labels.contains(label)
            {
                self.r.push(item.err(format!("`{}.{}`: map `{map_full}` has no label `{label}`", row.component, f.name)));
            }
        }
    }

    fn extends_cycle(&self, def: &Def, kind: Kind) -> Option<String> {
        let mut seen = vec![def.full.clone()];
        let mut cur = def;
        while let Some(b) = cur.block() {
            let parent = b.arg_after("extends")?;
            let next = self.m.resolve_kind(b.namespace.as_deref(), parent, &[kind]).ok()?;
            seen.push(next.full.clone());
            if next.full == def.full {
                return Some(seen.join(" → "));
            }
            if seen[..seen.len() - 1].contains(&next.full) {
                return None;
            }
            cur = next;
        }
        None
    }

    fn maps(&mut self) {
        let defs: Vec<&Def> = self.m.defs.values().filter(|d| matches!(d.kind, Kind::Map | Kind::Prefab)).collect();
        for def in &defs {
            let Some(b) = def.block() else { continue };
            if b.has_arg("extends") {
                continue;
            }
            let drawn = self.drawing(b);
            self.drawn.insert(def.full.clone(), drawn);
        }
        // extending maps get their base's drawing and labels (D 8.10)
        for def in defs {
            let Some(b) = def.block() else { continue };
            let Some(base) = b.arg_after("extends") else { continue };
            if !b.drawing.is_empty() {
                self.r.push(b.err("an extending map takes its base's drawing; it cannot have its own (D 8.10)"));
            }
            let base_def = match self.m.resolve_kind(b.namespace.as_deref(), base, &[Kind::Map]) {
                Ok(d) => d,
                Err(e) => {
                    self.r.push(b.err(e));
                    continue;
                }
            };
            if let Some(cycle) = self.extends_cycle(def, Kind::Map) {
                self.r.push(b.err(format!("`extends` cycle: {cycle}")));
                continue;
            }
            let mut labels = self.drawn.get(&base_def.full).map(|d| d.labels.clone()).unwrap_or_default();
            let (w, h) = self.drawn.get(&base_def.full).map(|d| (d.width, d.height)).unwrap_or((0, 0));
            for item in b.sub("legend") {
                if let Some(l) = self.legend_row(item, b) {
                    labels.extend(l);
                }
            }
            self.drawn.insert(def.full.clone(), Drawn { width: w, height: h, labels });
        }
    }

    /// Reads a drawn map or prefab: size, characters and legend (D 8.1–8.9).
    fn drawing(&mut self, b: &RawBlock) -> Drawn {
        let mut d = Drawn::default();
        if b.drawing.is_empty() {
            self.r.push(b.err(format!("{} `{}` has no drawing", b.kind, b.name)));
            return d;
        }
        let rows: Vec<Vec<char>> = b.drawing.iter().map(|l| l.chars().collect()).collect();
        let size = b.arg_after("size").map(|s| s.split_once('x').and_then(|(w, h)| Some((w.parse::<usize>().ok()?, h.parse::<usize>().ok()?))));
        match size {
            Some(Some((w, h))) => {
                (d.width, d.height) = (w, h);
                for (i, row) in rows.iter().enumerate() {
                    if row.len() > w {
                        self.r.error(&b.file, b.drawing_line + i, format!("drawing row is {} wide; the size is {w}x{h}", row.len()));
                    }
                }
                if rows.len() > h {
                    self.r.error(&b.file, b.drawing_line + h, format!("drawing has {} rows; the size is {w}x{h}", rows.len()));
                }
            }
            Some(None) => self.r.push(b.err("size is written `WxH`, such as `size 24x4`")),
            None => {
                d.width = rows[0].len();
                d.height = rows.len();
                for (i, row) in rows.iter().enumerate() {
                    if row.len() != d.width {
                        self.r.error(&b.file, b.drawing_line + i, format!("drawing rows differ in length ({} and {}); give the map a `size`", d.width, row.len()));
                        break;
                    }
                }
            }
        }
        let mut legend: BTreeMap<char, BTreeSet<String>> = BTreeMap::new();
        legend.insert(' ', BTreeSet::new());
        for item in b.sub("legend") {
            match glyph_char(item) {
                Ok((c, _)) => {
                    let labels = self.legend_row(item, b).unwrap_or_default();
                    legend.insert(c, labels);
                }
                Err(e) => self.r.push(item.err(e)),
            }
        }
        for (y, row) in rows.iter().enumerate() {
            for (x, c) in row.iter().enumerate() {
                let line = b.drawing_line + y;
                if is_joining(*c) {
                    self.r.error(&b.file, line, format!("combining character, joiner or variation selector in the drawing at column {} (D 8.1)", x + 1));
                    continue;
                }
                if c.width().unwrap_or(1) > 1 {
                    self.r.warning(&b.file, line, format!("wide character `{c}` at column {} breaks the drawing's alignment", x + 1));
                }
                match legend.get(c) {
                    Some(labels) => d.labels.extend(labels.iter().cloned()),
                    None => self.r.error(&b.file, line, format!("unknown character `{c}` at row {}, column {}: it is not in the legend", y + 1, x + 1)),
                }
            }
        }
        let mut seen_at = BTreeSet::new();
        for item in b.sub("at") {
            let pos = item.key();
            if !is_pos(pos) {
                self.r.push(item.err(format!("an `at:` row starts with a position `X,Y`, not `{pos}`")));
                continue;
            }
            let (x, y) = pos.split_once(',').map(|(x, y)| (x.parse::<usize>().unwrap(), y.parse::<usize>().unwrap())).unwrap();
            if x >= d.width || y >= d.height {
                self.r.push(item.err(format!("position {pos} is outside the {}x{} map", d.width, d.height)));
            }
            if !seen_at.insert(pos.to_string()) {
                self.r.push(item.err(format!("second `at:` row for {pos}")));
            }
            if let Some(labels) = self.place_row(&item.shift(1), b) {
                d.labels.extend(labels);
            }
        }
        for item in b.sub("vars") {
            if !names::is_name(item.key()) || item.tokens.len() < 2 {
                self.r.push(item.err("a map variable is `- NAME value`"));
            }
        }
        d
    }

    /// Checks a legend row and returns the labels it gives.
    fn legend_row(&mut self, item: &RawItem, b: &RawBlock) -> Option<BTreeSet<String>> {
        let (_, rest) = glyph_char(item).ok()?;
        self.place_row(&rest, b)
    }

    /// `[TILE] [LAYER TILE …] [with …] [as [local] LABEL] [wire LABEL]`
    /// (D 8.2, D 8.9).
    fn place_row(&mut self, rest: &RawItem, b: &RawBlock) -> Option<BTreeSet<String>> {
        let ns = rest.namespace.as_deref();
        let words: Vec<&Token> = rest.tokens.iter().collect();
        let kw = |w: &Token| !w.quoted && ["with", "as", "wire"].contains(&w.text.as_str());
        let tile_end = words.iter().position(|w| kw(w)).unwrap_or(words.len());
        let mut ok = true;
        // the tile part: `TILE`, `TILE LAYER TILE …` or `LAYER TILE …`
        let tiles = &words[..tile_end];
        let pairs_from = tiles.len() % 2;
        for (i, w) in tiles.iter().enumerate() {
            let is_layer = i >= pairs_from && (i - pairs_from).is_multiple_of(2);
            if is_layer {
                if let Err(e) = self.m.resolve_kind(ns, &w.text, &[Kind::Layer]) {
                    self.r.push(rest.err(format!("`{}` is not a terrain layer: {e}", w.text)));
                    ok = false;
                }
            } else if w.text != "none" {
                match names::tile_name(&w.text) {
                    Ok(t) if matches!(t.index, Some(Index::Range(..))) => {
                        self.r.push(rest.err(format!("`{}`: a legend produces one tile, not a range (D 7.4)", w.text)));
                        ok = false;
                    }
                    Ok(_) => match self.m.resolve_tile(ns, &w.text) {
                        Ok(full) => {
                            self.used_tiles.insert(full);
                        }
                        Err(e) => {
                            self.r.push(rest.err(e));
                            ok = false;
                        }
                    },
                    Err(e) => {
                        self.r.push(rest.err(e));
                        ok = false;
                    }
                }
            }
        }
        let mut labels = BTreeSet::new();
        let mut i = tile_end;
        while i < words.len() {
            let end = (i + 1..words.len()).find(|&j| kw(words[j])).unwrap_or(words.len());
            let args: Vec<Token> = words[i + 1..end].iter().map(|t| (*t).clone()).collect();
            match words[i].text.as_str() {
                "with" => {
                    if let Err(e) = self.values(ns).draw(&args, &[Kind::Entity, Kind::Deck, Kind::Table]) {
                        self.r.push(rest.err(format!("`with`: {e}")));
                        ok = false;
                    }
                    // an entity label given as an override (`with LIFT label GATE_A`)
                    if let Some(p) = args.iter().position(|t| t.text == "label") {
                        let l = args.get(p + 1).filter(|t| t.text == "local").map_or(p + 1, |_| p + 2);
                        if let Some(t) = args.get(l) {
                            labels.insert(t.text.clone());
                        }
                    }
                }
                "as" | "wire" => {
                    let name = match args.as_slice() {
                        [l] => l,
                        [local, l] if local.text == "local" => l,
                        _ => {
                            self.r.push(rest.err(format!("`{}` takes one label, optionally `local`", words[i].text)));
                            ok = false;
                            i = end;
                            continue;
                        }
                    };
                    if !names::is_name(&name.text) {
                        self.r.push(rest.err(format!("`{}` is not a label NAME", name.text)));
                        ok = false;
                    } else if words[i].text == "as" {
                        labels.insert(name.text.clone());
                    }
                }
                _ => unreachable!(),
            }
            i = end;
        }
        let _ = b;
        ok.then_some(labels)
    }

    /// A label must not be a general name that lookup would find (D 2.1).
    fn labels(&mut self) {
        let mut errors = Vec::new();
        for (map, drawn) in &self.drawn {
            let def = &self.m.defs[map];
            let ns = def.block().and_then(|b| b.namespace.as_deref());
            for l in &drawn.labels {
                if let Ok(other) = self.m.resolve(ns, l) {
                    errors.push(crate::error::Diagnostic::error(
                        &def.file,
                        def.line,
                        format!("label `{l}` of map `{map}` has the same name as the {} `{}` ({})", other.kind, other.full, other.place()),
                    ));
                }
            }
        }
        for e in errors {
            self.r.push(e);
        }
    }

    fn game(&mut self, main: &str) {
        let named: Vec<&RawBlock> = self.m.games.iter().filter(|g| !g.name.is_empty()).collect();
        if named.len() > 1 {
            self.r.push(named[1].err(format!("second named `game` block (the first is at {}:{})", named[0].file, named[0].line)));
        }
        for g in &self.m.games {
            for item in &g.items {
                if names::is_vocabulary(item.key()) {
                    if g.name.is_empty() {
                        self.r.push(item.err(format!("`{}`: only the main file's named `game` block has engine fields", item.key())));
                    } else if !GAME_FIELDS.contains(&item.key()) {
                        self.r.push(item.err(format!("unknown `game` field `{}`", item.key())));
                    }
                } else {
                    self.component_row(item, g.namespace.as_deref());
                }
            }
        }
        let Some(g) = named.first().copied() else {
            self.r.error(main, 0, "no named `game` block: the main file names the game with `game NAME` (D 3.3)");
            return;
        };
        let ns = g.namespace.as_deref();
        let mut fields: BTreeMap<String, RawItem> = BTreeMap::new();
        // the short form `game NAME start TOWN at START` (D 3.3)
        let header = RawItem::new(&g.file, g.line, g.namespace.clone(), &g.args_text).ok();
        if let Some(h) = header.filter(|h| !h.tokens.is_empty()) {
            let at = h.tokens.iter().position(|t| t.is("at")).unwrap_or(h.tokens.len());
            if h.key() == "start" {
                fields.insert("start".into(), RawItem::new(&g.file, g.line, g.namespace.clone(), h.span(0, at)).unwrap());
            } else {
                self.r.push(g.err(format!("unexpected `{}` in the game header", h.key())));
            }
            if at < h.tokens.len() {
                fields.insert("at".into(), h.shift(at));
            }
        }
        for item in g.items.iter().filter(|i| names::is_vocabulary(i.key())) {
            fields.insert(item.key().to_string(), item.clone());
        }
        let one = |item: &RawItem| -> Option<String> {
            match item.tokens.len() {
                2 => Some(item.tokens[1].text.clone()),
                _ => None,
            }
        };
        let mut start_map = None;
        match fields.get("start") {
            None => self.r.push(g.err("no `start`: the game cannot be started as a new game (D 3.3)")),
            Some(item) => match one(item) {
                None => self.r.push(item.err("`start` takes a map, a deck or a table")),
                Some(s) => match self.m.resolve_kind(ns, &s, &[Kind::Map, Kind::Deck, Kind::Table]) {
                    Ok(d) if d.kind == Kind::Map => start_map = Some(d.full.clone()),
                    Ok(_) => {}
                    Err(e) => self.r.push(item.err(format!("`start`: {e}"))),
                },
            },
        }
        let at = fields.get("at").cloned();
        match (&at, &start_map) {
            (Some(item), Some(map)) => self.search(&item.shift(1), map),
            (None, Some(map))
                if self.drawn.get(map).is_some_and(|d| !d.labels.contains("START")) => {
                    self.r.push(g.err(format!("no `at` and the start map `{map}` has no label `START` (D 3.3)")));
                }
            _ => {}
        }
        let refs: &[(&str, &[Kind])] = &[
            ("title", &[Kind::Menu]),
            ("layout", &[Kind::Layout]),
            ("naming", &[Kind::Query]),
            ("walk", &[Kind::Command]),
            ("walk_stop", &[Kind::Query]),
        ];
        for (field, kinds) in refs {
            if let Some(item) = fields.get(*field) {
                match one(item) {
                    Some(n) => {
                        if let Err(e) = self.m.resolve_kind(ns, &n, kinds) {
                            self.r.push(item.err(format!("`{field}`: {e}")));
                        }
                    }
                    None => self.r.push(item.err(format!("`{field}` takes one NAME"))),
                }
            }
        }
        if let Some(item) = fields.get("keys") {
            match one(item) {
                Some(n) if self.context(ns, &n).is_some() => {}
                Some(n) => self.r.push(item.err(format!("`keys`: no key context `{n}`"))),
                None => self.r.push(item.err("`keys` takes one context NAME")),
            }
        }
        if !fields.contains_key("layout") {
            let layouts: Vec<&str> = defs_of(self.m, Kind::Layout).map(|d| d.full.as_str()).collect();
            if layouts.len() > 1 {
                self.r.push(g.err(format!("several layouts ({}) and no `layout` field to choose one (D 11.1)", layouts.join(", "))));
            }
        }
        // seats, party and leash (D 3.3)
        let mut upper = 1;
        if let Some(item) = fields.get("seats") {
            match one(item).as_deref().and_then(crate::rt::world::seat_range) {
                Some((a, b)) if (1..=8).contains(&a) && (1..=8).contains(&b) && a <= b => upper = b,
                _ => self.r.push(item.err("`seats` is a range `a..b` from 1 to 8 with `a` at most `b` (D 3.3)")),
            }
        }
        for (field, values) in [("party", ["switch", "each"]), ("leash", ["on", "off"])] {
            if let Some(item) = fields.get(field)
                && !one(item).is_some_and(|v| values.contains(&v.as_str()))
            {
                self.r.push(item.err(format!("`{field}` is `{}` or `{}` (D 3.3)", values[0], values[1])));
            }
        }
        if upper == 1
            && let Some(item) = fields.get("leash").filter(|i| one(i).as_deref() == Some("on"))
        {
            self.r.push(item.warn("`leash on` in a one-seat game does nothing (D 3.3)"));
        }
        for def in defs_of(self.m, Kind::Entity) {
            let Some(b) = def.block() else { continue };
            for item in b.items.iter().filter(|i| i.key() == "seat") {
                if !one(item).and_then(|v| v.parse::<u8>().ok()).is_some_and(|n| (1..=upper).contains(&n)) {
                    self.r.push(item.err(format!("`seat` is a seat number from 1 to the game's `seats` upper bound {upper} (D 5.3)")));
                }
            }
        }
        for (field, lo, hi) in [("call_depth", 32, 4096), ("loop_steps", 10_000, i64::MAX), ("seed", i64::MIN, i64::MAX), ("masking", i64::MIN, i64::MAX)] {
            if let Some(item) = fields.get(field) {
                match one(item).and_then(|v| v.parse::<i64>().ok()) {
                    Some(n) if (lo..=hi).contains(&n) => {}
                    _ if field == "call_depth" => self.r.push(item.err("`call_depth` is a number from 32 to 4096")),
                    _ if field == "loop_steps" => self.r.push(item.err("`loop_steps` is a number of at least 10000")),
                    _ => self.r.push(item.err(format!("`{field}` takes a number"))),
                }
            }
        }
    }

    /// A key context by reference.
    fn context(&self, ns: Option<&str>, name: &str) -> Option<String> {
        names::candidates(ns, name).into_iter().find(|c| self.m.contexts.contains_key(c))
    }

    /// A data search `KOHDE [near [ANKKURI] [ETÄISYYS]]` or
    /// `SIIRTYMÄ from ANKKURI` on map `map` (D 8.8).
    fn search(&mut self, item: &RawItem, map: &str) {
        let ns = item.namespace.as_deref();
        let words: Vec<&str> = item.tokens.iter().map(|t| t.text.as_str()).collect();
        let drawn = self.drawn.get(map);
        let label_or_set = |c: &Self, w: &str| -> Result<(), String> {
            if drawn.is_some_and(|d| d.labels.contains(w)) {
                return Ok(());
            }
            match c.m.resolve(ns, w) {
                Ok(d) if matches!(d.kind, Kind::Tile | Kind::Tileset) => Ok(()),
                Ok(d) => Err(format!("`{w}` is a {} ({}), not a label, tile or tileset", d.kind, d.place())),
                Err(_) if names::tile_name(w).is_ok_and(|t| c.m.kind(&names::qualify(ns, &t.base)) == Some(Kind::Tile)) => Ok(()),
                Err(_) if drawn.is_some() => Err(format!("map `{map}` has no label `{w}`, and no tile or tileset has that name")),
                Err(e) => Err(e),
            }
        };
        let res = match words.as_slice() {
            [] => Err("`at` needs a target".to_string()),
            [w, ..] if is_pos(w) => match drawn {
                Some(d) => {
                    let (x, y) = w.split_once(',').map(|(x, y)| (x.parse::<usize>().unwrap(), y.parse::<usize>().unwrap())).unwrap();
                    if x < d.width && y < d.height { Ok(()) } else { Err(format!("{w} is outside the {}x{} map `{map}`", d.width, d.height)) }
                }
                None => Ok(()),
            },
            [_, rest @ ..] if rest.contains(&"from") => {
                let anchor = rest.iter().position(|w| *w == "from").and_then(|i| rest.get(i + 1));
                match anchor {
                    Some(a) => label_or_set(self, a),
                    None => Err("`from` needs an anchor label".into()),
                }
            }
            [target, rest @ ..] => label_or_set(self, target).and_then(|_| match rest {
                [] => Ok(()),
                ["near", tail @ ..] => {
                    let anchor = tail.iter().find(|w| !is_int(w) && !fields::is_range(w));
                    match anchor {
                        Some(a) if drawn.is_some_and(|d| !d.labels.contains(*a)) => Err(format!("`near`: map `{map}` has no label `{a}`")),
                        _ => Ok(()),
                    }
                }
                _ => Err(format!("unexpected `{}` after the target", rest.join(" "))),
            }),
        };
        if let Err(e) = res {
            self.r.push(item.err(format!("`at`: {e}")));
        }
    }

    fn settings(&mut self, main: &str) {
        let settings = self.m.settings();
        for (key, item) in &settings {
            let v = item.rest(1);
            let bad = match key.as_str() {
                k if !SETTINGS.contains(&k) => Some(format!("unknown setting `{k}`")),
                "language" | "style" if !names::is_vocabulary(v) => Some(format!("`{key}` takes a lower-case name")),
                "round" | "external_timeout" if !is_time(v) => Some(format!("`{key}` takes a time such as `500ms` or `1s`")),
                "external" if !["ask", "all", "none"].contains(&v) => Some("`external` is ask, all or none".into()),
                "mouse" if !["on", "off"].contains(&v) => Some("`mouse` is on or off".into()),
                "targeting" if !["cursor", "list", "auto"].contains(&v) => Some("`targeting` is cursor, list or auto".into()),
                "idle" if !item.word(1).is_some_and(is_time) => Some("`idle` starts with a time: `idle 4s [ai] [COMMAND]`".into()),
                _ => None,
            };
            if let Some(msg) = bad {
                self.r.push(item.err(msg));
            }
        }
        if self.m.named_game().is_none() {
            return;
        }
        for key in ["style", "language"] {
            if !settings.contains_key(key) {
                self.r.error(main, 0, format!("missing setting `{key}`: the engine does not guess it (D 3.2)"));
            }
        }
        if let Some(lang) = settings.get("language").map(|i| i.rest(1))
            && !self.m.languages.iter().any(|f| f.language == lang) {
                let item = &settings["language"];
                self.r.push(item.warn(format!("no language file has `language {lang}`: every text shows as its key")));
            }
        if let Some(style) = settings.get("style").map(|i| i.rest(1)) {
            let has = self.m.section_rows("tiles.glyphs").next().is_none() || self.m.sections.iter().any(|b| b.kind == "tiles.glyphs" && b.args.first().is_some_and(|a| a.text == style));
            if !has {
                self.r.push(settings["style"].warn(format!("no `tiles.glyphs {style}` section: every tile uses the default look")));
            }
        }
    }

    fn tiles_section(&mut self) {
        for (_, item) in self.m.section_rows("tiles") {
            let (_, row) = item.marker();
            let mut i = 1;
            while i < row.tokens.len() {
                let prop = row.tokens[i].text.as_str();
                let end = (i + 1..row.tokens.len()).find(|&j| ["blocks", "only", "lit", "forget"].contains(&row.tokens[j].text.as_str())).unwrap_or(row.tokens.len());
                let args = &row.tokens[i + 1..end];
                let res = match prop {
                    "blocks" => self.values(row.namespace.as_deref()).blocks(args),
                    "only" if !args.is_empty() && args.iter().all(|a| names::is_name(&a.text)) => Ok(()),
                    "only" => Err("`only` lists movement modes in upper case".to_string()),
                    "lit" | "forget" if args.is_empty() || (args.len() == 1 && ["true", "false"].contains(&args[0].text.as_str())) => Ok(()),
                    "lit" | "forget" => Err(format!("`{prop}` is a flag: `{prop}` or `{prop} false`")),
                    p => Err(format!("unknown tile property `{p}` (blocks, only, lit, forget; the look is in `tiles.glyphs`)")),
                };
                if let Err(e) = res {
                    self.r.push(item.err(e));
                }
                i = end;
            }
        }
    }

    fn style(&mut self) {
        for (_, item) in self.m.section_rows("colors") {
            let (_, row) = item.marker();
            if !row.word(1).is_some_and(is_hex) || row.tokens.len() != 2 {
                self.r.push(item.err("a color is `- NAME hex`, with six lower-case hex digits and no `#`"));
            }
        }
        for b in self.m.sections.iter().filter(|b| b.kind == "tiles.glyphs") {
            let presentation = b.args.first().map(|t| t.text.as_str()).unwrap_or("");
            if presentation.is_empty() {
                self.r.push(b.err("`tiles.glyphs` names its presentation: `tiles.glyphs ascii`"));
                continue;
            }
            let ns = b.namespace.as_deref();
            for item in &b.items {
                let (_, row) = item.marker();
                let tile = row.key();
                if tile != "default" {
                    match names::tile_name(tile) {
                        Ok(t) => {
                            if let Ok(full) = self.m.resolve_tile(ns, tile) {
                                self.used_tiles.insert(full);
                            }
                            if row.word(1) == Some("series") && !matches!(t.index, Some(Index::Range(_, Some(_)))) {
                                self.r.push(item.err("a series row needs a closed index range such as `GLYPH.NKI.1..36` (D 9.2)"));
                            }
                        }
                        Err(e) => {
                            self.r.push(item.err(e));
                            continue;
                        }
                    }
                }
                if presentation != "ascii" {
                    continue;
                }
                let rest = row.shift(1);
                let colors_from = if rest.key() == "series" {
                    if !rest.tokens.get(1).is_some_and(|t| t.quoted) {
                        self.r.push(item.err("`series` takes the characters in quotes: `series \"&*+?\"`"));
                    }
                    2
                } else {
                    match glyph_char(&rest) {
                        Ok((c, _)) if is_joining(c) => {
                            self.r.push(item.err("combining character, joiner or variation selector as a glyph (D 9.2)"));
                        }
                        Ok((c, _)) if c.width().unwrap_or(1) > 1 => self.r.push(item.warn(format!("wide character `{c}` as a glyph"))),
                        Ok(_) => {}
                        Err(e) => self.r.push(item.err(e)),
                    }
                    if rest.text.starts_with("' '") { 2 } else { 1 }
                };
                let words: Vec<&str> = rest.tokens.iter().skip(colors_from).map(|t| t.text.as_str()).collect();
                let mut i = 0;
                while i < words.len() {
                    if !["fg", "bg"].contains(&words[i]) {
                        self.r.push(item.err(format!("unexpected `{}`: expected `fg COLOR` or `bg COLOR`", words[i])));
                        break;
                    }
                    let mut j = i + 1;
                    while j < words.len() && !["fg", "bg"].contains(&words[j]) {
                        if !is_hex(words[j]) && self.m.resolve_kind(ns, words[j], &[Kind::Color]).is_err() {
                            self.r.push(item.err(format!("`{}` is not a color name or a hex color", words[j])));
                        }
                        j += 1;
                    }
                    if j == i + 1 {
                        self.r.push(item.err(format!("`{}` needs a color", words[i])));
                    }
                    i = j;
                }
            }
        }
    }

    fn layouts(&mut self) {
        for def in defs_of(self.m, Kind::Layout) {
            let Some(b) = def.block() else { continue };
            let rows: Vec<Vec<char>> = b.drawing.iter().map(|l| l.chars().collect()).collect();
            if rows.is_empty() {
                self.r.push(b.err("a layout needs a drawing of its areas"));
                continue;
            }
            let mut boxes: BTreeMap<char, (usize, usize, usize, usize, usize)> = BTreeMap::new();
            for (y, row) in rows.iter().enumerate() {
                for (x, c) in row.iter().enumerate() {
                    let e = boxes.entry(*c).or_insert((x, y, x, y, 0));
                    *e = (e.0.min(x), e.1.min(y), e.2.max(x), e.3.max(y), e.4 + 1);
                }
            }
            for (c, (x0, y0, x1, y1, n)) in &boxes {
                if (x1 - x0 + 1) * (y1 - y0 + 1) != *n {
                    self.r.push(b.err(format!("area `{c}` is not a rectangle (D 11.1)")));
                }
            }
            let mut legend = BTreeSet::new();
            for item in b.sub("legend") {
                let Ok((c, rest)) = glyph_char(item) else {
                    self.r.push(item.err("a layout legend row starts with the area's character"));
                    continue;
                };
                legend.insert(c);
                if !boxes.contains_key(&c) {
                    self.r.push(item.warn(format!("area `{c}` is not in the drawing")));
                }
                let source = rest.key();
                let name = source.split('(').next().unwrap_or(source);
                if !["map", "hover"].contains(&name)
                    && let Err(e) = self.m.resolve_kind(item.namespace.as_deref(), name, &[Kind::Log, Kind::Panel])
                {
                    self.r.push(item.err(format!("area `{c}`: {e}")));
                }
            }
            for c in boxes.keys().filter(|c| !legend.contains(c)) {
                self.r.push(b.err(format!("area `{c}` of the drawing is not in the legend")));
            }
        }
    }

    fn logs(&mut self) {
        let messages: Vec<&str> = defs_of(self.m, Kind::Message).map(|d| d.full.as_str()).collect();
        let selects = |row: &str| messages.iter().any(|m| *m == row || m.starts_with(&format!("{row}.")));
        let mut rows: Vec<(String, RawItem)> = Vec::new();
        for def in defs_of(self.m, Kind::Log) {
            if let Some(b) = def.block() {
                rows.extend(b.items.iter().map(|i| (i.key().to_string(), i.clone())));
            }
        }
        for (_, item) in self.m.section_rows("logs") {
            rows.push((item.key().to_string(), item.clone()));
        }
        for (row, item) in rows {
            if row == "SPEECH" {
                continue;
            }
            let found = names::candidates(item.namespace.as_deref(), &row).iter().any(|c| selects(c));
            if !found {
                self.r.push(item.err(format!("`{row}` is not the start of any message key (D 11.4)")));
            }
        }
    }

    fn texts(&mut self) {
        let languages: BTreeSet<&str> = self.m.languages.iter().map(|f| f.language.as_str()).collect();
        for f in &self.m.languages {
            for t in &f.texts {
                if !names::is_name(t.key()) {
                    self.r.push(t.err(format!("text key `{}` is not a NAME", t.key())));
                }
            }
        }
        for lang in languages {
            let texts = self.m.texts(lang);
            for def in defs_of(self.m, Kind::Message) {
                let Some(text) = texts.get(&def.full) else { continue };
                let crate::model::Source::Row(row) = &def.source else { continue };
                let params: Vec<&str> = row.tokens.iter().skip(1).map(|t| t.text.as_str()).collect();
                for p in placeholders(text.rest(1)) {
                    let name = p.split([':', '.']).next().unwrap_or(&p);
                    if !params.contains(&name) {
                        self.r.push(text.err(format!("`{{{p}}}`: message `{}` has no parameter `{name}` (parameters: {})", def.full, params.join(", "))));
                    }
                }
            }
            // `{@KEY}` references and their cycles (D 10)
            for (key, text) in &texts {
                for r in refs(text.rest(1)) {
                    if !texts.contains_key(&r) {
                        self.r.push(text.err(format!("`{{@{r}}}`: no text `{r}` in language `{lang}`")));
                    }
                }
                let mut path = vec![key.clone()];
                if let Some(cycle) = ref_cycle(&texts, key, &mut path) {
                    self.r.push(text.err(format!("text references form a cycle: {cycle}")));
                }
            }
        }
    }

    fn keys(&mut self) {
        for (name, blocks) in &self.m.contexts {
            let mut seen: BTreeMap<String, usize> = BTreeMap::new();
            // keys by their terminal key: `num8` is `8` there (D 11.3)
            let mut terminal: BTreeMap<String, (String, String, usize)> = BTreeMap::new();
            for b in blocks {
                for item in &b.items {
                    let (_, row) = item.marker();
                    let n = match self.action(&row) {
                        Ok(n) => n,
                        Err(e) => {
                            self.r.push(item.err(e));
                            continue;
                        }
                    };
                    if n >= row.tokens.len() {
                        self.r.push(item.err("a key row names the action and then its keys"));
                    }
                    let action = row.span(0, n.min(row.tokens.len())).to_string();
                    for k in &row.tokens[n.min(row.tokens.len())..] {
                        if let Some(line) = seen.insert(k.text.clone(), item.line) {
                            self.r.push(item.err(format!("key `{}` is bound twice in context `{name}` (also line {line})", k.text)));
                            continue;
                        }
                        let key = terminal_key(&k.text);
                        match terminal.get(&key) {
                            Some((other, other_action, line)) if *other != k.text && *other_action != action => {
                                self.r.push(item.warn(format!(
                                    "`{}` and `{other}` (line {line}) are the same key in a terminal, bound to `{action}` and `{other_action}` in context `{name}` (D 11.3)",
                                    k.text
                                )));
                            }
                            Some(_) => {}
                            None => {
                                terminal.insert(key, (k.text.clone(), action.clone(), item.line));
                            }
                        }
                    }
                }
            }
        }
    }

    /// Checks the action at the start of a key or menu row; returns how many
    /// words it took.
    fn action(&self, row: &RawItem) -> Result<usize, String> {
        let ns = row.namespace.as_deref();
        let w = row.key();
        if w == "open" {
            let target = row.word(1).ok_or("`open` needs a menu or panel")?;
            let name = target.split('(').next().unwrap_or(target);
            self.m.resolve_kind(ns, name, &[Kind::Menu, Kind::Panel])?;
            return Ok(if row.word(2) == Some("stack") { 3 } else { 2 });
        }
        if names::is_vocabulary(w) {
            let dir = w.strip_prefix("cursor.").is_some_and(|d| fields::DIRS.contains(&d));
            return if ENGINE_ACTIONS.contains(&w) || dir { Ok(1) } else { Err(format!("unknown engine action `{w}`")) };
        }
        self.m.resolve_kind(ns, w, &[Kind::Command])?;
        Ok(1)
    }

    fn menus(&mut self) {
        for def in defs_of(self.m, Kind::Menu) {
            let Some(b) = def.block() else { continue };
            if b.has_arg("list") {
                continue;
            }
            for item in &b.items {
                let (_, row) = item.marker();
                let key = row.key();
                if ["TITLE", "TEXT"].contains(&key) {
                    self.r.push(item.err(format!("`{key}` is the menu's own text key and cannot be an item (D 11.2)")));
                    continue;
                }
                if !names::is_name(key) {
                    self.r.push(item.err(format!("a menu item starts with its text key NAME, not `{key}`")));
                    continue;
                }
                let rest = row.shift(1);
                let has_action = !rest.tokens.is_empty() && !["hide", "disabled"].contains(&rest.key());
                if rest.key() == "adjust" {
                    continue;
                }
                if has_action {
                    if let Err(e) = self.action(&rest) {
                        self.r.push(item.err(e));
                    }
                } else {
                    // `- RESUME` runs the command RESUME, or the engine action `resume` (D 11.2)
                    let command = self.m.resolve_kind(b.namespace.as_deref(), key, &[Kind::Command]).is_ok();
                    if !command && !ENGINE_ACTIONS.contains(&key.to_lowercase().as_str()) {
                        self.r.push(item.err(format!("item `{key}` has no action: there is no command `{key}` and no engine action `{}`", key.to_lowercase())));
                    }
                }
            }
        }
    }

    fn code(&mut self) {
        for d in &self.m.code {
            if d.kind != DeclKind::Handler {
                continue;
            }
            let event = d.event.as_deref().unwrap_or("");
            if names::is_vocabulary(event) {
                if !ENGINE_EVENTS.contains(&event) {
                    self.r.error(&d.file, d.line, format!("unknown engine event `{event}`"));
                }
                continue;
            }
            match self.m.resolve_kind(d.namespace.as_deref(), event, &[Kind::Event]) {
                Ok(ev) => {
                    let crate::model::Source::Decl(e) = &ev.source else { continue };
                    let required = e.params.iter().filter(|p| p.default.is_none()).count();
                    if d.params.len() > e.params.len() || d.params.len() < required {
                        self.r.error(
                            &d.file,
                            d.line,
                            format!("handler of `{}` names {} parameters; the event has {} ({} without a default)", ev.full, d.params.len(), e.params.len(), required),
                        );
                    }
                }
                Err(e) => self.r.error(&d.file, d.line, e),
            }
        }
    }

    /// A root name that is also a standard library name splits the name in
    /// two (D 4.4).
    fn std_shadowing(&mut self) {
        for def in self.m.defs.values() {
            if def.full.starts_with("STD.") || def.kind == Kind::Tile {
                continue;
            }
            if let Some(std) = self.m.defs.get(&format!("STD.{}", def.full)) {
                self.r.warning(&def.file, def.line, format!("`{}` shadows the standard library's `{}` ({}); override or patch `STD.{}` instead", def.full, std.full, std.place(), def.full));
            }
        }
    }

    /// A `tiles` row for a tile that nothing uses is probably a typo (D 7.5).
    fn unused_tiles(&mut self) {
        for (_, item) in self.m.section_rows("tiles") {
            let (_, row) = item.marker();
            if row.key() == "default" {
                continue;
            }
            let Ok(full) = self.m.resolve_tile(row.namespace.as_deref(), row.key()) else { continue };
            if !self.used_tiles.contains(&full) {
                self.r.push(item.warn(format!("tile `{}` is not used in a legend, a style or logic: a typo?", row.key())));
            }
        }
    }
}

/// A key as a terminal sees it: `numN` is the digit `N` (D 11.3).
fn terminal_key(k: &str) -> String {
    match k.strip_prefix("num") {
        Some(d) if d.len() == 1 && d.chars().all(|c| c.is_ascii_digit()) => d.to_string(),
        _ => k.to_string(),
    }
}

fn is_hex(s: &str) -> bool {
    s.len() == 6 && s.chars().all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
}

fn is_time(s: &str) -> bool {
    let n = s.strip_suffix("ms").or_else(|| s.strip_suffix('s'));
    n.is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
}

/// Combining marks, joiners and variation selectors (D 8.1).
fn is_joining(c: char) -> bool {
    matches!(c as u32, 0x0300..=0x036F | 0x1AB0..=0x1AFF | 0x1DC0..=0x1DFF | 0x20D0..=0x20FF | 0xFE20..=0xFE2F | 0x200D | 0xFE00..=0xFE0F | 0xE0100..=0xE01EF)
}

/// `{NAME}`, `{NAME:SUFFIX}` and `{NAME.DESC}` placeholders; `{@KEY}` and
/// `{^ACTION}` are not parameters.
pub(crate) fn placeholders(text: &str) -> Vec<String> {
    braces(text).into_iter().filter(|p| !p.starts_with('@') && !p.starts_with('^')).collect()
}

fn refs(text: &str) -> Vec<String> {
    braces(text).into_iter().filter_map(|p| p.strip_prefix('@').map(String::from)).collect()
}

fn braces(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(open) = rest.find('{') {
        let Some(close) = rest[open..].find('}') else { break };
        out.push(rest[open + 1..open + close].to_string());
        rest = &rest[open + close + 1..];
    }
    out
}

fn ref_cycle(texts: &BTreeMap<String, &RawItem>, key: &str, path: &mut Vec<String>) -> Option<String> {
    let text = texts.get(key)?;
    for r in refs(text.rest(1)) {
        if r == path[0] {
            path.push(r);
            return Some(path.join(" → "));
        }
        if path.contains(&r) {
            continue;
        }
        path.push(r.clone());
        if let Some(c) = ref_cycle(texts, &r, path) {
            return Some(c);
        }
        path.pop();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn helpers() {
        assert!(is_hex("5a5a6e") && !is_hex("5A5A6E") && !is_hex("#5a5a6e"));
        assert!(is_time("250ms") && is_time("1s") && is_time("0500ms") && !is_time("ms") && !is_time("5"));
        assert_eq!(placeholders("{TARGET} ottaa {AMOUNT} {SOUND:DESC} {ITEM.DESC} {@GOLD.NAME} {^confirm}"), ["TARGET", "AMOUNT", "SOUND:DESC", "ITEM.DESC"]);
        assert_eq!(refs("Löysit {@GOLD.NAME}!"), ["GOLD.NAME"]);
        assert!(is_joining('\u{0308}') && is_joining('\u{200D}') && !is_joining('a'));
    }
}
