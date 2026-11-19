//! The map as text, by the console layers (D 9.5) and the ASCII style
//! (D 9.2): `map` draws the terrain, `entities [where …]` the entities
//! with a `render` tile, `fog` hides what the player does not see. A layer
//! with `remember` keeps what was last seen (M 7.2).

use std::collections::BTreeSet;

use super::eval::{BodyKind, Env};
use super::value::{EntityId, MapId, Pos, Value};
use super::world::World;
use crate::names;

/// A color as red, green and blue.
pub type Rgb = [u8; 3];

/// A drawn cell: its character and colors (D 9.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Look {
    pub ch: char,
    pub fg: Option<Rgb>,
    pub bg: Option<Rgb>,
}

struct Layer {
    source: String,
    filter: Option<crate::rs::ast::Expr>,
    remember: bool,
}

impl World<'_> {
    fn layers_section(&self) -> Vec<Layer> {
        let mut out = Vec::new();
        for (_, item) in self.m.section_rows("layers") {
            let source = item.word(1).unwrap_or("").to_string();
            let words: Vec<&str> = item.tokens.iter().map(|t| t.text.as_str()).collect();
            let remember = words.contains(&"remember");
            let filter = words.iter().position(|w| *w == "where").and_then(|i| {
                let end = words.iter().position(|w| *w == "remember" || *w == "smooth").filter(|e| *e > i).unwrap_or(words.len());
                crate::rs::parse::parse_expr(item.span(i + 1, end)).ok()
            });
            out.push(Layer { source, filter, remember });
        }
        if out.is_empty() {
            out = vec![
                Layer { source: "map".into(), filter: None, remember: true },
                Layer { source: "entities".into(), filter: None, remember: false },
                Layer { source: "fog".into(), filter: None, remember: false },
            ];
        }
        out
    }

    /// A color by name (`colors`, D 9.1) or six lower-case hex digits.
    pub fn color(&self, word: &str, ns: Option<&str>) -> Option<Rgb> {
        let hex = |h: &str| {
            (h.len() == 6).then(|| [0, 2, 4].map(|i| u8::from_str_radix(&h[i..i + 2], 16).ok()))?.into_iter().collect::<Option<Vec<u8>>>().map(|v| [v[0], v[1], v[2]])
        };
        if let Some(c) = hex(word) {
            return Some(c);
        }
        let full = self.m.resolve(ns, word).ok()?.full.clone();
        self.m.section_rows("colors").find_map(|(_, item)| {
            let (_, row) = item.marker();
            (names::qualify(row.namespace.as_deref(), row.key()) == full).then(|| row.word(1).and_then(hex)).flatten()
        })
    }

    /// A tile's ASCII look down the ladder of `tiles.glyphs` (D 9.2): its
    /// character and colors.
    pub fn look(&self, tile: &str) -> Look {
        let style = self.m.settings().get("style").map(|i| i.rest(1).to_string()).unwrap_or_else(|| "ascii".into());
        let mut ladder = vec![tile.to_string()];
        let mut t = tile.to_string();
        while let Some((head, _)) = t.rsplit_once('.') {
            ladder.push(head.to_string());
            t = head.to_string();
        }
        ladder.push("default".into());
        for key in ladder {
            let mut found = None;
            for b in self.m.sections.iter().filter(|b| b.kind == "tiles.glyphs" && b.args.first().is_some_and(|a| a.text == style)) {
                for item in &b.items {
                    let (_, row) = item.marker();
                    let row_key = if row.key() == "default" { "default".to_string() } else { self.full_name(row.key(), row.namespace.as_deref()) };
                    if row_key != key {
                        continue;
                    }
                    let Ok((ch, rest)) = crate::model::glyph_char(&row.shift(1)) else { continue };
                    let ns = row.namespace.as_deref();
                    let after = |w: &str| rest.tokens.iter().position(|t| t.is(w)).and_then(|i| rest.word(i + 1)).and_then(|c| self.color(c, ns));
                    found = Some(Look { ch, fg: after("fg"), bg: after("bg") });
                }
            }
            if let Some(l) = found {
                return l;
            }
        }
        Look { ch: '?', fg: None, bg: None }
    }

    /// A tile's ASCII glyph (D 9.2).
    pub fn glyph(&self, tile: &str) -> char {
        self.look(tile).ch
    }

    /// The shared view on a map now (M 7.2), the last shared view without
    /// members, or the whole map.
    pub fn player_view(&mut self, map: MapId) -> BTreeSet<(i32, i32)> {
        if let Some(v) = self.shared_view(map) {
            self.last_view = Some((map, v.clone()));
            return v;
        }
        let m = &self.s.maps[map];
        match &self.last_view {
            Some((lm, v)) if *lm == map => v.clone(),
            _ => (0..m.height).flat_map(|y| (0..m.width).map(move |x| (x, y))).collect(),
        }
    }

    /// Draws a map as rows of characters.
    pub fn render(&mut self, map: MapId) -> Vec<String> {
        self.render_cells(map).into_iter().map(|r| r.into_iter().map(|c| c.ch).collect::<String>().trim_end().to_string()).collect()
    }

    /// Draws a map as cells with colors. A look without a background takes
    /// the next non-empty layer's below it (D 9.4).
    pub fn render_cells(&mut self, map: MapId) -> Vec<Vec<Look>> {
        let view = self.player_view(map);
        let (w, h) = (self.s.maps[map].width, self.s.maps[map].height);
        // what the player now sees is remembered (M 7.2), on a neighbour
        // sheet by that sheet (M 6.10)
        for &(x, y) in &view {
            let p = self.unframe(map, x, y);
            if self.on_map(p) && !self.s.maps[p.map].remembered.contains(&(p.x, p.y)) {
                self.s.maps[p.map].remembered.push((p.x, p.y));
            }
        }
        let remembered: BTreeSet<(i32, i32)> = self.s.maps[map].remembered.iter().copied().collect();
        let blank = Look { ch: ' ', fg: None, bg: None };
        let mut grid = vec![vec![blank; w as usize]; h as usize];
        let mut drawn = vec![vec![false; w as usize]; h as usize];
        for layer in self.layers_section() {
            match layer.source.as_str() {
                "map" => {
                    for y in 0..h {
                        for x in 0..w {
                            if !(view.contains(&(x, y)) || layer.remember && remembered.contains(&(x, y))) {
                                continue;
                            }
                            let m = &self.s.maps[map];
                            let tiles: Vec<String> = (0..m.layers.len()).rev().filter_map(|l| m.tile(l, x, y).map(String::from)).collect();
                            let Some(top) = tiles.first() else { continue };
                            let mut look = self.look(top);
                            if look.bg.is_none() {
                                look.bg = tiles[1..].iter().find_map(|t| self.look(t).bg);
                            }
                            grid[y as usize][x as usize] = look;
                            drawn[y as usize][x as usize] = true;
                        }
                    }
                }
                "entities" => {
                    for e in self.s.entities_on(map) {
                        let Some(Value::Name(tile)) = self.s.get(e).get("render").and_then(|c| c.get("tile")).cloned() else { continue };
                        let Some(p) = self.s.at(e) else { continue };
                        if !view.contains(&(p.x, p.y)) && self.s.get(e).get("revealed").is_none() {
                            continue;
                        }
                        if let Some(f) = &layer.filter {
                            let mut env = Env::new(BodyKind::Present, "layers", None, "<layers>");
                            env.map = Some(map);
                            env.it.push(Value::Entity(e));
                            if !matches!(self.eval(f, &mut env), Ok(Value::Bool(true))) {
                                continue;
                            }
                        }
                        let under = grid[p.y as usize][p.x as usize].bg;
                        // a seat's member looks for `TILE.SEATn` first, so a
                        // style can tell the seats apart (D 9.2, M 7.2)
                        let mut look = match self.s.seat_of(e) {
                            Some(n) => self.look(&format!("{tile}.SEAT{n}")),
                            None => self.look(&tile),
                        };
                        look.bg = look.bg.or(under);
                        grid[p.y as usize][p.x as usize] = look;
                        drawn[p.y as usize][p.x as usize] = true;
                    }
                }
                "fog" => {
                    for y in 0..h {
                        for x in 0..w {
                            if !view.contains(&(x, y)) && !(drawn[y as usize][x as usize] && remembered.contains(&(x, y))) {
                                grid[y as usize][x as usize] = blank;
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        grid
    }

    /// The hover text of a cell (M 8.1): the top drawn entity's
    /// description, or the top tile's; only for a cell in the shared view,
    /// and for a remembered one its tile. `None` for a cell nobody sees or
    /// remembers, so the hover gives nothing away.
    pub fn hover_text(&mut self, lang: &str, p: Pos) -> Option<String> {
        let view = self.player_view(p.map);
        let seen = view.contains(&(p.x, p.y));
        let drawn = |e: EntityId| self.s.get(e).get("render").is_some();
        let entity = self.s.entities_at(p).into_iter().rev().find(|&e| drawn(e) && (seen || self.s.get(e).get("revealed").is_some()));
        if let Some(e) = entity {
            return Some(self.description(lang, &Value::Entity(e)));
        }
        if !seen && !self.s.maps[p.map].remembered.contains(&(p.x, p.y)) {
            return None;
        }
        let m = &self.s.maps[p.map];
        let tile = (0..m.layers.len()).rev().find_map(|l| m.tile(l, p.x, p.y).map(String::from))?;
        Some(self.description(lang, &Value::Name(tile)))
    }

    /// A cell as `x,y` for messages.
    pub fn cell(p: Pos) -> String {
        format!("{},{}", p.x, p.y)
    }
}
