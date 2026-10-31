//! The map as text, by the console layers (D 9.5) and the ASCII style
//! (D 9.2): `map` draws the terrain, `entities [where …]` the entities
//! with a `render` tile, `fog` hides what the player does not see. A layer
//! with `remember` keeps what was last seen (M 7.2).

use std::collections::BTreeSet;

use super::eval::{BodyKind, Env};
use super::value::{MapId, Pos, Value};
use super::world::World;

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

    /// A tile's ASCII glyph down the ladder of `tiles.glyphs ascii` (D 9.2).
    pub fn glyph(&self, tile: &str) -> char {
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
                    if row_key == key
                        && let Ok((c, _)) = crate::model::glyph_char(&row.shift(1))
                    {
                        found = Some(c);
                    }
                }
            }
            if let Some(c) = found {
                return c;
            }
        }
        '?'
    }

    /// The player's view on a map now, or the whole map without `sight`.
    pub fn player_view(&mut self, map: MapId) -> BTreeSet<(i32, i32)> {
        let m = &self.s.maps[map];
        let all = || (0..m.height).flat_map(|y| (0..m.width).map(move |x| (x, y))).collect();
        match self.s.player {
            Some(p) if self.s.get(p).alive && self.s.at(p).is_some_and(|a| a.map == map) => {
                let v = self.field_of_view(p).unwrap_or_else(all);
                self.last_view = Some((map, v.clone()));
                v
            }
            _ => match &self.last_view {
                Some((lm, v)) if *lm == map => v.clone(),
                _ => all(),
            },
        }
    }

    /// Draws a map as rows of characters.
    pub fn render(&mut self, map: MapId) -> Vec<String> {
        let view = self.player_view(map);
        let (w, h) = (self.s.maps[map].width, self.s.maps[map].height);
        // what the player now sees is remembered (M 7.2)
        for &cell in &view {
            if !self.s.maps[map].remembered.contains(&cell) {
                self.s.maps[map].remembered.push(cell);
            }
        }
        let remembered: BTreeSet<(i32, i32)> = self.s.maps[map].remembered.iter().copied().collect();
        let mut grid = vec![vec![' '; w as usize]; h as usize];
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
                            let top = (0..m.layers.len()).rev().find_map(|l| m.tile(l, x, y).map(String::from));
                            if let Some(t) = top {
                                grid[y as usize][x as usize] = self.glyph(&t);
                                drawn[y as usize][x as usize] = true;
                            }
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
                        grid[p.y as usize][p.x as usize] = self.glyph(&tile);
                        drawn[p.y as usize][p.x as usize] = true;
                    }
                }
                "fog" => {
                    for y in 0..h {
                        for x in 0..w {
                            if !view.contains(&(x, y)) && !(drawn[y as usize][x as usize] && remembered.contains(&(x, y))) {
                                grid[y as usize][x as usize] = ' ';
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        grid.into_iter().map(|r| r.into_iter().collect::<String>().trim_end().to_string()).collect()
    }

    /// A cell as `x,y` for messages.
    pub fn cell(p: Pos) -> String {
        format!("{},{}", p.x, p.y)
    }
}
