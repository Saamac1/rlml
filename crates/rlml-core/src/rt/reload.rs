//! Reloading in development mode (M 5.7, RLML-kehitystyokalut.md step 4).
//!
//! The model is borrowed everywhere, so a reload does not change it in
//! place: the state goes into the new model as a save does, and maps are
//! merged with their new drawings by the cells the game has changed. The
//! same merge runs when a save is loaded under a changed drawing.

use super::run::Runner;
use super::save::Save;
use super::state::State;
use super::world::{RtResult, World};
use crate::model::Model;

/// Merges the state's maps with their drawings in `m` (M 5.7): a cell
/// the game has not changed takes the new drawing's tile, a changed one
/// keeps the game's, and a drawing that changed under a game's change is
/// a warning. Labels and wires come from the new drawing. Maps without
/// the record of changed cells (saves from before it) stay as they are.
pub(crate) fn merge_maps(m: &Model, s: &mut State) -> Vec<String> {
    let mut warnings = Vec::new();
    for id in 0..s.maps.len() {
        if !s.maps[id].tracking {
            continue;
        }
        let name = s.maps[id].name.clone();
        // the drawing as a new game would place it
        let mut fresh = World::new(m, s.game.seed);
        let drawn = match fresh.load_map(&name) {
            Ok(i) => fresh.s.maps[i].clone(),
            Err(e) => {
                warnings.push(format!("map `{name}` is kept as it was: {e}"));
                continue;
            }
        };
        let map = &mut s.maps[id];
        if (drawn.width, drawn.height) != (map.width, map.height) || drawn.layers != map.layers {
            warnings.push(format!(
                "map `{name}` is kept as it was: its size or layers changed ({}×{} → {}×{})",
                map.width, map.height, drawn.width, drawn.height
            ));
            continue;
        }
        // the drawing's own cells change without counting as the game's
        map.tracking = false;
        for l in 0..map.layers.len() {
            for y in 0..map.height {
                for x in 0..map.width {
                    let new = drawn.tile(l, x, y).map(str::to_string);
                    match map.changed.iter().position(|(k, _)| *k == ((x, y), l)) {
                        None => {
                            if map.tile(l, x, y) != new.as_deref() {
                                map.set_tile(l, x, y, new);
                            }
                        }
                        Some(j) if map.changed[j].1 != new => {
                            warnings.push(format!(
                                "map `{name}` {x},{y}: the drawing changed a cell the game has changed; the game's {} stays",
                                map.tile(l, x, y).unwrap_or("none")
                            ));
                            // the new drawing is the cell's value from here on
                            if map.tile(l, x, y) == new.as_deref() {
                                map.changed.remove(j);
                            } else {
                                map.changed[j].1 = new;
                            }
                        }
                        Some(_) => {}
                    }
                }
            }
        }
        map.tracking = true;
        for (label, _) in &map.labels {
            if !drawn.labels.iter().any(|(l, _)| l == label) {
                warnings.push(format!("map `{name}`: the label `{label}` is gone"));
            }
        }
        map.labels = drawn.labels;
        map.wires = drawn.wires;
    }
    warnings
}

impl<'m> Runner<'m> {
    /// Actors standing on a cell whose tiles block their own move, as a
    /// merged map may leave them (M 5.7).
    pub(crate) fn misplaced(&self) -> Vec<String> {
        let s = &self.w.s;
        (0..s.entities.len())
            .filter(|&e| s.get(e).alive && (s.is_actor(e) || s.player == Some(e)))
            .filter_map(|e| s.at(e).map(|p| (e, p)))
            .filter(|&(e, p)| s.maps[p.map].tracking && self.w.blocks_tiles(p, "move", Some(e)))
            .map(|(e, p)| {
                format!(
                    "#{e} {} stands at {} {},{}, where the map blocks its move",
                    s.get(e).kind,
                    s.maps[p.map].name,
                    p.x,
                    p.y
                )
            })
            .collect()
    }

    /// Reloads the game from `m` between actions (M 5.7): the state goes
    /// over, the maps merge with their drawings, and the input log starts
    /// a segment from here (M 4.4). Running background scenes keep their
    /// timelines. Returns the merge's warnings; an error leaves the game
    /// as it was.
    pub fn reload(&mut self, m: &'m Model) -> RtResult<Vec<String>> {
        if self.in_scene() || self.w.save_blocked() {
            return Err("a reload waits for the scene, dialogue or menu to end".into());
        }
        let save = self.save();
        let mut r = Runner::load(m, &save)?;
        let warnings = std::mem::take(&mut r.w.warnings);
        r.scene_clock = self.scene_clock;
        r.auto_off = self.auto_off;
        r.delete_auto = self.delete_auto;
        r.due = std::mem::take(&mut self.due);
        r.bg_plans = std::mem::take(&mut self.bg_plans);
        r.realtime = std::mem::take(&mut self.realtime);
        r.inputs.from = Some((m.source_hash, r.save().write()));
        *self = r;
        Ok(warnings)
    }
}

impl Save {
    /// The state's and the checkpoint's maps merged with `m`'s drawings
    /// (M 5.7).
    pub(crate) fn merge_maps(&mut self, m: &Model) -> Vec<String> {
        let mut warnings = merge_maps(m, &mut self.now.s);
        if let Some(c) = self.checkpoint.as_mut() {
            // the checkpoint's warnings repeat the state's
            let _ = merge_maps(m, &mut c.s);
        }
        warnings.dedup();
        warnings
    }
}
