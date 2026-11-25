//! The look cursor of a seat (M 8.1): `look` opens it for one travel,
//! `cursor` opens it to stay. A staying cursor is the seat's own and moves
//! on other seats' turns too, where `confirm` and `follow` give an order
//! for the seat's next turn. Cursors are presentation, not game state.

use rlml_core::rt::value::{Dir, EntityId, Pos};
use rlml_core::rt::world::World;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cursor {
    pub pos: Pos,
    /// `cursor`: stays over turns and travels; `look`: closes when a
    /// travel starts.
    pub stays: bool,
}

/// `cursor.north` … `cursor.northwest` as a direction.
pub fn dir_of(action: &str) -> Option<Dir> {
    let name = action.strip_prefix("cursor.")?;
    Dir::ALL.into_iter().find(|d| d.name() == name)
}

/// The cursor moved one cell, kept on its map.
pub fn moved(w: &World, p: Pos, d: Dir) -> Pos {
    let m = &w.s.maps[p.map];
    let n = p.step(d);
    Pos {
        x: n.x.clamp(0, m.width - 1),
        y: n.y.clamp(0, m.height - 1),
        ..n
    }
}

/// The entities a cursor's `next` and `prev` visit: drawn ones in the
/// shared view, nearest to `from` first, then in reading order (M 8.1).
pub fn visible(w: &World, from: Pos) -> Vec<Pos> {
    let Some(view) = w.shared_view(from.map) else {
        return vec![];
    };
    let mut cells: Vec<Pos> =
        w.s.entities_on(from.map)
            .into_iter()
            .filter(|&e| w.s.get(e).alive && w.s.get(e).get("render").is_some())
            .filter_map(|e| w.s.at(e))
            .filter(|p| view.contains(&(p.x, p.y)) && *p != from)
            .collect();
    cells.sort_by_key(|p| (from.distance(*p), p.y, p.x));
    cells.dedup();
    cells
}

/// The actor under a cursor that `follow` follows: the top one drawn.
pub fn actor_at(w: &World, p: Pos) -> Option<EntityId> {
    w.s.entities_at(p)
        .into_iter()
        .rev()
        .find(|&e| w.s.get(e).alive && w.s.is_actor(e))
}
