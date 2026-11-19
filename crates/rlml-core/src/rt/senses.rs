//! Sight, light, hearing and paths (M 6.8, M 7.2, M 9.3).
//!
//! * Line of sight is Albert Ford's symmetric shadowcasting (2021) in four
//!   quadrants, with exact fractions: a floor cell is in sight when its
//!   centre is, a blocking cell when any part of it is (M 7.2).
//! * A visible cell is in sight and either lit and within `radius` or
//!   within `dark`; a cell is lit by a `lit` tile in it or next to it, or by
//!   a `light` source.
//! * A sound spreads in eight directions, one level less a step, and stops
//!   at `sound` blocking. A listener hears its wake interval's sounds; the
//!   loudest masks those `masking` levels below it (M 6.8).
//! * `path` is A* with the tie-breaking of M 9.3.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use super::value::{Dir, EntityId, MapId, Pos, Sound, Value};
use super::world::{RtResult, World};

/// A fraction with a positive denominator, for exact slopes.
#[derive(Debug, Clone, Copy)]
struct Frac {
    n: i64,
    d: i64,
}

impl Frac {
    fn new(n: i64, d: i64) -> Frac {
        if d < 0 { Frac { n: -n, d: -d } } else { Frac { n, d } }
    }

    /// `depth × self` rounded half up: floor(x + 1/2).
    fn round_ties_up(self, depth: i64) -> i64 {
        (2 * depth * self.n + self.d).div_euclid(2 * self.d)
    }

    /// `depth × self` rounded half down: ceil(x − 1/2).
    fn round_ties_down(self, depth: i64) -> i64 {
        -((-2 * depth * self.n + self.d).div_euclid(2 * self.d))
    }

    /// `col ≥ depth × self`.
    fn le(self, depth: i64, col: i64) -> bool {
        depth * self.n <= col * self.d
    }

    /// `col ≤ depth × self`.
    fn ge(self, depth: i64, col: i64) -> bool {
        col * self.d <= depth * self.n
    }
}

/// The cells in line of sight from `origin` up to `radius`, by symmetric
/// shadowcasting. `blocks` tells whether a cell blocks sight.
pub fn line_of_sight(origin: (i32, i32), radius: i64, blocks: &dyn Fn(i32, i32) -> bool) -> BTreeSet<(i32, i32)> {
    let mut seen = BTreeSet::new();
    seen.insert(origin);
    for quadrant in 0..4 {
        let transform = |depth: i64, col: i64| -> (i32, i32) {
            let (ox, oy) = (origin.0 as i64, origin.1 as i64);
            let (x, y) = match quadrant {
                0 => (ox + col, oy - depth),
                1 => (ox + depth, oy + col),
                2 => (ox + col, oy + depth),
                _ => (ox - depth, oy + col),
            };
            (x as i32, y as i32)
        };
        let mut rows = vec![(1i64, Frac::new(-1, 1), Frac::new(1, 1))];
        while let Some((depth, mut start, end)) = rows.pop() {
            if depth > radius {
                continue;
            }
            let min_col = start.round_ties_up(depth);
            let max_col = end.round_ties_down(depth);
            let mut prev: Option<bool> = None;
            for col in min_col..=max_col {
                let (x, y) = transform(depth, col);
                let wall = blocks(x, y);
                let symmetric = start.le(depth, col) && end.ge(depth, col);
                if wall || symmetric {
                    seen.insert((x, y));
                }
                if prev == Some(true) && !wall {
                    start = Frac::new(2 * col - 1, 2 * depth);
                }
                if prev == Some(false) && wall {
                    rows.push((depth + 1, start, Frac::new(2 * col - 1, 2 * depth)));
                }
                prev = Some(wall);
            }
            if prev == Some(false) {
                rows.push((depth + 1, start, end));
            }
        }
    }
    seen
}

impl World<'_> {
    /// A tile flag (`lit`, `forget`) down the ladder: the nearest row that
    /// names it, `false` when written `lit false` (D 7.5).
    pub fn tile_flag(&self, tile: &str, flag: &str) -> bool {
        let key = (tile.to_string(), flag.to_string());
        if let Some(&v) = self.flags.borrow().get(&key) {
            return v;
        }
        let v = self.find_tile_flag(tile, flag);
        self.flags.borrow_mut().insert(key, v);
        v
    }

    fn find_tile_flag(&self, tile: &str, flag: &str) -> bool {
        let mut ladder = vec![tile.to_string()];
        let mut t = tile.to_string();
        while let Some((head, _)) = t.rsplit_once('.') {
            ladder.push(head.to_string());
            t = head.to_string();
        }
        ladder.push("default".into());
        for key in ladder {
            let mut found = None;
            for (_, item) in self.m.section_rows("tiles") {
                let (_, row) = item.marker();
                let row_key = if row.key() == "default" { "default".to_string() } else { self.full_name(row.key(), row.namespace.as_deref()) };
                if row_key != key {
                    continue;
                }
                let words: Vec<&str> = row.tokens.iter().skip(1).map(|t| t.text.as_str()).collect();
                if let Some(i) = words.iter().position(|w| *w == flag) {
                    found = Some(words.get(i + 1) != Some(&"false"));
                }
            }
            if let Some(v) = found {
                return v;
            }
        }
        false
    }

    fn blocks_sight(&self, map: MapId, x: i32, y: i32) -> bool {
        self.blocks(Pos { map, x, y }, "sight", None, true)
    }

    /// Is a cell lit: a `lit` tile in it or next to it, or a `light`
    /// source's reach (M 7.2)?
    pub fn lit(&self, p: Pos) -> bool {
        self.lit_in(p, &self.light_reach(p.map))
    }

    /// `lit` with the map's light reach computed beforehand, so a view
    /// casts each light once and not once per cell.
    fn lit_in(&self, p: Pos, reach: &BTreeSet<(i32, i32)>) -> bool {
        let m = &self.s.maps[p.map];
        for dx in -1..=1 {
            for dy in -1..=1 {
                let (x, y) = (p.x + dx, p.y + dy);
                if (0..m.layers.len()).any(|l| m.tile(l, x, y).is_some_and(|t| self.tile_flag(t, "lit"))) {
                    return true;
                }
            }
        }
        reach.contains(&(p.x, p.y))
    }

    /// The cells the `light` sources on a map reach: within the radius and
    /// in the source's line of sight.
    fn light_reach(&self, map: MapId) -> BTreeSet<(i32, i32)> {
        let mut out = BTreeSet::new();
        for e in self.s.entities_on(map) {
            let Some(light) = self.s.get(e).get("light") else { continue };
            let r = light.get("radius").and_then(Value::as_number).unwrap_or(0);
            let Some(at) = self.s.at(e) else { continue };
            for (x, y) in line_of_sight((at.x, at.y), r, &|x, y| self.blocks_sight(map, x, y)) {
                if at.distance(Pos { map, x, y }) <= r {
                    out.insert((x, y));
                }
            }
        }
        out
    }

    /// The cells an entity sees (M 7.2); `None` when it has no `sight`.
    pub fn field_of_view(&self, e: EntityId) -> Option<BTreeSet<(i32, i32)>> {
        let sight = self.s.get(e).get("sight")?;
        let at = self.s.at(e)?;
        let num = |f: &str, d: i64| sight.get(f).and_then(Value::as_number).unwrap_or(d);
        let (radius, dark) = (num("radius", 0), num("dark", 1));
        let in_sight = line_of_sight((at.x, at.y), radius.max(dark), &|x, y| self.blocks_sight(at.map, x, y));
        let m = &self.s.maps[at.map];
        let reach = self.light_reach(at.map);
        Some(
            in_sight
                .into_iter()
                .filter(|&(x, y)| m.inside(x, y))
                .filter(|&(x, y)| {
                    let p = Pos { map: at.map, x, y };
                    let d = at.distance(p);
                    d == 0 || d <= dark || (d <= radius && self.lit_in(p, &reach))
                })
                .collect(),
        )
    }

    /// The shared view on `map` (M 7.2): the union of the living members'
    /// views there; a member without `sight` sees the whole map. `None`
    /// when no member is on the map.
    pub fn shared_view(&self, map: MapId) -> Option<BTreeSet<(i32, i32)>> {
        let on_map: Vec<EntityId> = self.s.players().into_iter().filter(|&e| self.s.at(e).is_some_and(|p| p.map == map)).collect();
        if on_map.is_empty() {
            return None;
        }
        let m = &self.s.maps[map];
        let mut view = BTreeSet::new();
        for e in on_map {
            match self.field_of_view(e) {
                Some(v) => view.extend(v),
                None => return Some((0..m.height).flat_map(|y| (0..m.width).map(move |x| (x, y))).collect()),
            }
        }
        Some(view)
    }

    /// The shared view on the active map, kept as the last shared view for
    /// when no member is left (M 7.2).
    pub fn current_view(&mut self) -> Option<(MapId, BTreeSet<(i32, i32)>)> {
        if let Some(map) = self.s.active_map()
            && let Some(v) = self.shared_view(map)
        {
            self.last_view = Some((map, v));
        }
        self.last_view.clone()
    }

    /// `in_view(x)`: some cell of `x` is in the shared view, or in the last
    /// shared view without members (M 7.2).
    pub fn in_view(&mut self, cells: &[Pos]) -> bool {
        match self.current_view() {
            Some((map, view)) => cells.iter().any(|p| p.map == map && view.contains(&(p.x, p.y))),
            None => false,
        }
    }

    /// `sees(a, b)`: some cell of `b` is in `a`'s view. A member without
    /// `sight` sees its whole map; a dead member sees the last shared view
    /// (M 7.2).
    pub fn sees(&mut self, a: EntityId, cells: &[Pos]) -> bool {
        let member = self.s.is_member(a) || self.s.player == Some(a);
        if member && !self.s.get(a).alive {
            return match &self.last_view {
                Some((map, view)) => cells.iter().any(|p| p.map == *map && view.contains(&(p.x, p.y))),
                None => false,
            };
        }
        let Some(at) = self.s.at(a) else { return false };
        if member {
            // keep the last shared view for when the group is gone
            self.current_view();
        }
        match self.field_of_view(a) {
            Some(view) => cells.iter().any(|p| p.map == at.map && view.contains(&(p.x, p.y))),
            None => member && cells.iter().any(|p| p.map == at.map),
        }
    }

    // ---- hearing (M 6.8) ----

    /// A sound kind's level from the `sounds` section (D 12.3).
    fn sound_level(&self, kind: &str) -> i64 {
        for (_, item) in self.m.section_rows("sounds") {
            let (_, row) = item.marker();
            if self.full_name(row.key(), row.namespace.as_deref()) == kind
                && let Some(i) = row.tokens.iter().position(|t| t.is("level"))
            {
                return row.word(i + 1).and_then(|w| w.parse().ok()).unwrap_or(0);
            }
        }
        0
    }

    /// Steps from a sound's cell to each cell it reaches, stopped by `sound`
    /// blocking (M 6.8).
    fn spread(&self, from: Pos, max: i64) -> BTreeMap<(i32, i32), i64> {
        let mut dist = BTreeMap::new();
        dist.insert((from.x, from.y), 0);
        let mut queue = VecDeque::from([(from.x, from.y)]);
        while let Some((x, y)) = queue.pop_front() {
            let d = dist[&(x, y)];
            if d >= max {
                continue;
            }
            for dir in Dir::ALL {
                let (dx, dy) = dir.delta();
                let n = (x + dx, y + dy);
                if dist.contains_key(&n) || self.blocks(Pos { map: from.map, x: n.0, y: n.1 }, "sound", None, true) {
                    continue;
                }
                dist.insert(n, d + 1);
                queue.push_back(n);
            }
        }
        dist
    }

    /// The sounds `e` hears since its last wake, loudest first, with the
    /// masked ones left out (M 6.8). Moves `e`'s mark past them.
    pub fn listen(&mut self, e: EntityId) -> Vec<Sound> {
        let from = self.heard_mark.get(&e).copied().unwrap_or(0);
        self.heard_mark.insert(e, self.sounds_made);
        let Some(acuity) = self.s.get(e).get("hearing").map(|h| h.get("acuity").and_then(Value::as_number).unwrap_or(0)) else {
            self.heard.insert(e, vec![]);
            return vec![];
        };
        let Some(at) = self.s.at(e) else { return vec![] };
        let masking = self.m.named_game().and_then(|g| g.items.iter().find(|i| i.key() == "masking")).and_then(|i| i.word(1)).and_then(|w| w.parse().ok()).unwrap_or(3);
        let first = self.sounds_made - self.sounds.len() as u64;
        let mut out = Vec::new();
        for (k, n) in self.sounds.clone().into_iter().enumerate() {
            if first + (k as u64) < from || n.at.map != at.map {
                continue;
            }
            let level = self.sound_level(&n.kind);
            let reach = self.spread(n.at, level.max(0));
            let Some(d) = reach.get(&(at.x, at.y)) else { continue };
            let heard = level - d + acuity;
            if heard > 0 {
                out.push(Sound { kind: n.kind.clone(), at: n.at, source: n.source, level: heard });
            }
        }
        out.sort_by_key(|s| -s.level);
        if let Some(loudest) = out.first().map(|s| s.level) {
            out.retain(|s| s.level > loudest - masking);
        }
        self.heard.insert(e, out.clone());
        // a sound nobody can still hear can go
        let oldest = self.heard_mark.values().copied().min().unwrap_or(self.sounds_made);
        while self.sounds_made - (self.sounds.len() as u64) < oldest && !self.sounds.is_empty() {
            self.sounds.remove(0);
        }
        out
    }

    // ---- paths (M 9.3) ----

    /// Is `who` an actor that a route may pass at a cost of four steps?
    fn is_actor(&self, e: EntityId) -> bool {
        self.s.player == Some(e) || self.s.is_actor(e)
    }

    /// `path(a, b, for)`: an A* route from `a` to the nearest cell of `b`,
    /// without the start cell, in order (M 9.3).
    pub fn route(&self, from: Pos, to: &[Pos], who: Option<EntityId>) -> Vec<Pos> {
        self.route_within(from, to, who, None)
    }

    /// `route` through the cells of `known` only, when given: a travel's
    /// route passes only cells the players see or remember (M 8.1).
    pub fn route_within(&self, from: Pos, to: &[Pos], who: Option<EntityId>, known: Option<&BTreeSet<(i32, i32)>>) -> Vec<Pos> {
        let goals: BTreeSet<(i32, i32)> = to.iter().filter(|p| p.map == from.map).map(|p| (p.x, p.y)).collect();
        if goals.is_empty() || goals.contains(&(from.x, from.y)) {
            return vec![];
        }
        let map = &self.s.maps[from.map];
        let h = |(x, y): (i32, i32)| goals.iter().map(|g| ((g.0 - x).abs().max((g.1 - y).abs())) as i64).min().unwrap_or(0);
        let mv = who.and_then(|w| self.s.get(w).get("movement"));
        let dirs = mv.and_then(|m| m.get("dirs")).and_then(Value::as_number).unwrap_or(8);
        let corners = matches!(mv.and_then(|m| m.get("corners")), Some(Value::Bool(true)));
        // the cost of entering a cell, or `None` if it is closed: a blocking
        // actor is passed at four steps on a route `for` someone (M 9.3)
        let enter = |x: i32, y: i32| -> Option<i64> {
            let p = Pos { map: from.map, x, y };
            if !map.inside(x, y) || known.is_some_and(|k| !k.contains(&(x, y))) || self.blocks_tiles(p, "move", who) {
                return None;
            }
            let Some(w) = who else { return Some(1) };
            let blockers: Vec<EntityId> = self.s.entities_at(p).into_iter().filter(|&e| e != w && self.blocks_entities(p, "move", Some(w)) && self.blocks_entity(e)).collect();
            if blockers.is_empty() {
                Some(1)
            } else if blockers.iter().all(|&e| self.is_actor(e)) {
                Some(4)
            } else {
                None
            }
        };
        let start = (from.x, from.y);
        let mut g: BTreeMap<(i32, i32), i64> = BTreeMap::from([(start, 0)]);
        let mut parent: BTreeMap<(i32, i32), (i32, i32)> = BTreeMap::new();
        let mut open: BTreeSet<(i64, i64, i32, i32)> = BTreeSet::from([(h(start), h(start), start.1, start.0)]);
        let mut closed = BTreeSet::new();
        while let Some(&(f, hh, y, x)) = open.iter().next() {
            open.remove(&(f, hh, y, x));
            let cur = (x, y);
            if !closed.insert(cur) {
                continue;
            }
            if goals.contains(&cur) {
                let mut route = vec![cur];
                let mut c = cur;
                while let Some(&p) = parent.get(&c) {
                    if p == start {
                        break;
                    }
                    route.push(p);
                    c = p;
                }
                route.reverse();
                return route.into_iter().map(|(x, y)| Pos { map: from.map, x, y }).collect();
            }
            for d in Dir::ALL {
                let (dx, dy) = d.delta();
                let diagonal = dx != 0 && dy != 0;
                if diagonal && who.is_some() && dirs == 4 {
                    continue;
                }
                let n = (x + dx, y + dy);
                if closed.contains(&n) {
                    continue;
                }
                if diagonal && who.is_some() && !corners {
                    let side = |x: i32, y: i32| self.blocks(Pos { map: from.map, x, y }, "move", who, true);
                    if side(x + dx, y) || side(x, y + dy) {
                        continue;
                    }
                }
                let Some(cost) = enter(n.0, n.1) else { continue };
                let ng = g[&cur] + cost;
                if g.get(&n).is_none_or(|old| ng < *old) {
                    g.insert(n, ng);
                    parent.insert(n, cur);
                    open.insert((ng + h(n), h(n), n.1, n.0));
                }
            }
        }
        vec![]
    }

    fn blocks_entity(&self, e: EntityId) -> bool {
        matches!(self.s.get(e).get("blocks").and_then(|c| c.get("values")), Some(Value::List(vs)) if vs.iter().any(|v| *v == Value::Name("move".into())))
    }

    /// `ray(a, b, for, range, mode)`: the flight from `a` towards `b`
    /// without the start, on to `range` steps, ending before the first cell
    /// that stops it (M 9.3).
    pub fn ray(&self, a: Pos, b: Pos, who: Option<EntityId>, range: i64, mode: Option<&str>) -> Vec<Pos> {
        if a == b {
            return vec![];
        }
        let far = if range > 0 {
            let k = range.max(1) as i32 * 2 + 1;
            Pos { x: a.x + (b.x - a.x) * k, y: a.y + (b.y - a.y) * k, ..a }
        } else {
            b
        };
        let mut out = Vec::new();
        for p in super::eval::line(a, far).into_iter().skip(1) {
            if range > 0 && out.len() as i64 >= range {
                break;
            }
            if range == 0 && out.last() == Some(&b) {
                break;
            }
            // tiles that stop movement, and tiles or entities that stop the mode
            let stops = self.blocks_tiles(p, "move", None) || mode.is_some_and(|m| self.blocks(p, m, who, true));
            if stops {
                break;
            }
            out.push(p);
        }
        out
    }

    /// `hear` events before an actor's turn: each new sound it hears,
    /// loudest first, not its own (M 6.8).
    pub fn hear_events(&mut self, e: EntityId) -> RtResult<()> {
        let sounds = self.listen(e);
        let mut env = super::eval::Env::new(super::eval::BodyKind::Handler, "hear", None, "<engine>");
        env.map = self.s.at(e).map(|p| p.map);
        for s in sounds.into_iter().filter(|s| s.source != Some(e)) {
            self.emit("hear", vec![Value::Entity(e), Value::Sound(s)], &env)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid<'a>(rows: &'a [&'a str]) -> impl Fn(i32, i32) -> bool + 'a {
        move |x, y| rows.get(y as usize).and_then(|r| r.as_bytes().get(x as usize)).is_none_or(|c| *c == b'#')
    }

    fn view(rows: &[&str], o: (i32, i32), r: i64) -> Vec<String> {
        let seen = line_of_sight(o, r, &grid(rows));
        rows.iter().enumerate().map(|(y, row)| row.chars().enumerate().map(|(x, c)| if seen.contains(&(x as i32, y as i32)) { c } else { ' ' }).collect()).collect()
    }

    #[test]
    fn an_open_room_is_all_in_sight() {
        let rows = ["#####", "#...#", "#...#", "#...#", "#####"];
        assert_eq!(view(&rows, (2, 2), 5), rows);
    }

    #[test]
    fn a_pillar_casts_a_shadow() {
        let rows = ["#######", "#.....#", "#..#..#", "#.....#", "#.....#", "#######"];
        let v = view(&rows, (3, 4), 9);
        assert_eq!(v[2], "#..#..#", "the pillar itself is seen");
        assert_eq!(&v[1][3..4], " ", "the cell behind it is in its shadow");
        assert_eq!(&v[1][1..2], ".");
    }

    #[test]
    fn sight_is_symmetric_between_floors() {
        let rows = ["#########", "#.......#", "#..#.#..#", "#...#...#", "#.......#", "#########"];
        let floors: Vec<(i32, i32)> = (0..6).flat_map(|y| (0..9).map(move |x| (x, y))).filter(|&(x, y)| !grid(&rows)(x, y)).collect();
        for &a in &floors {
            let sa = line_of_sight(a, 20, &grid(&rows));
            for &b in &floors {
                let sb = line_of_sight(b, 20, &grid(&rows));
                assert_eq!(sa.contains(&b), sb.contains(&a), "{a:?} and {b:?}");
            }
        }
    }

    #[test]
    fn radius_limits_depth() {
        let rows = ["#########", "#.......#", "#########"];
        let v = view(&rows, (1, 1), 2);
        assert_eq!(v[1], "#...     ");
    }

    #[test]
    fn exact_rounding() {
        assert_eq!(Frac::new(-1, 1).round_ties_up(3), -3);
        assert_eq!(Frac::new(1, 2).round_ties_up(1), 1, "0.5 rounds up");
        assert_eq!(Frac::new(1, 2).round_ties_down(1), 0, "0.5 rounds down");
        assert_eq!(Frac::new(3, 4).round_ties_down(2), 1, "1.5 rounds down");
    }
}
