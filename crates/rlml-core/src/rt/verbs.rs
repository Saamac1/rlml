//! The target rule of verbs (M 8.1): which of a command's parameters a
//! click fills, and with what.
//!
//! The clickable parameters are those whose source is `aim`, `from`, or
//! `ask` of type `dir` or `pos`. One is free when it is not filled and the
//! parameters its source names are. A click on a cell, or on a docked list
//! menu's row, fills the first free parameter it fits, in parameter order,
//! so the targets of "unlock the door with the key" go in either order.

use std::collections::BTreeSet;

use super::eval::{BodyKind, Env};
use super::value::{Dir, EntityId, Pos, Value};
use super::world::{RtResult, World};
use crate::rs::ast::{Expr, ExprKind, ItemKind, Param, Source};

/// What a click points at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    /// A map cell: its seen entities, or the cell itself.
    Cell(Pos),
    /// A row of a docked list menu: its member.
    Entity(EntityId),
}

/// A clickable parameter and the parameters its source names.
#[derive(Debug, Clone, PartialEq)]
pub struct VerbParam {
    pub name: String,
    pub deps: BTreeSet<String>,
}

/// The lower-case words of an expression: the names it may read.
fn words(e: &Expr, out: &mut BTreeSet<String>) {
    match &e.kind {
        ExprKind::Word(w) => {
            out.insert(w.clone());
        }
        ExprKind::List(xs) => xs.iter().for_each(|x| words(x, out)),
        ExprKind::Field(b, _) | ExprKind::TileIndex(b, _) | ExprKind::Neg(b) | ExprKind::Not(b) => words(b, out),
        ExprKind::Has { subject, .. } => words(subject, out),
        ExprKind::Call(f, args) => {
            words(f, out);
            args.iter().for_each(|a| words(&a.value, out));
        }
        ExprKind::Subscript(a, b) | ExprKind::Binary(_, a, b) | ExprKind::Is(a, b) | ExprKind::In(a, b) | ExprKind::Where(a, b) => {
            words(a, out);
            words(b, out);
        }
        ExprKind::Range(a, b) => {
            words(a, out);
            if let Some(b) = b {
                words(b, out);
            }
        }
        _ => {}
    }
}

/// The parameters a parameter's source names (M 8.1).
pub fn deps_of(p: &Param, params: &[Param]) -> BTreeSet<String> {
    let mut ws = BTreeSet::new();
    match &p.source {
        Some(Source::From(e)) => words(e, &mut ws),
        Some(Source::Aim { range, at }) => {
            words(range, &mut ws);
            if let Some(a) = at {
                words(a, &mut ws);
            }
        }
        Some(Source::Ask(Some(e))) => words(e, &mut ws),
        _ => {}
    }
    params.iter().filter(|q| q.name != p.name && ws.contains(&q.name)).map(|q| q.name.clone()).collect()
}

/// Is a parameter clickable: `aim`, `from`, or `ask` of `dir` or `pos`?
pub fn clickable(p: &Param) -> bool {
    match &p.source {
        Some(Source::From(_)) | Some(Source::Aim { .. }) => true,
        Some(Source::Ask(_)) => matches!(p.ty.base.as_str(), "dir" | "pos"),
        _ => false,
    }
}

/// A ring among the sources of a command's parameters, as the names in
/// it, for the load check (M 2.5).
pub fn source_ring(params: &[Param]) -> Option<Vec<String>> {
    let deps: Vec<(String, BTreeSet<String>)> = params.iter().map(|p| (p.name.clone(), deps_of(p, params))).collect();
    // a parameter is settled when all it names are; what never settles is a ring
    let mut settled: BTreeSet<String> = BTreeSet::new();
    loop {
        let before = settled.len();
        for (n, d) in &deps {
            if d.is_subset(&settled) {
                settled.insert(n.clone());
            }
        }
        if settled.len() == before {
            break;
        }
    }
    let ring: Vec<String> = deps.into_iter().map(|(n, _)| n).filter(|n| !settled.contains(n)).collect();
    (!ring.is_empty()).then_some(ring)
}

/// The nearest of the eight directions from `a` towards `b` (M 8.1).
pub fn toward(a: Pos, b: Pos) -> Option<Dir> {
    let (dx, dy) = ((b.x - a.x) as f64, (b.y - a.y) as f64);
    if dx == 0.0 && dy == 0.0 {
        return None;
    }
    // y grows down: north is -y; sectors of 45° from east
    let angle = (-dy).atan2(dx).to_degrees();
    let i = (((angle + 360.0 + 22.5) % 360.0) / 45.0) as usize % 8;
    Some([Dir::East, Dir::Northeast, Dir::North, Dir::Northwest, Dir::West, Dir::Southwest, Dir::South, Dir::Southeast][i])
}

/// What a player's command needs before it runs (M 8, RS 7.1): the next
/// parameter to ask, in parameter order.
#[derive(Debug, Clone, PartialEq)]
pub enum Ask {
    /// Every parameter is given: the command can run.
    Ready,
    /// The command fails before asking and takes no time: an empty `from`,
    /// or an `aim` range that is `none`.
    Fails,
    /// Filled without asking: `count` is 1 without the repeat count.
    Given(String, Value),
    /// `from`: a list menu of the members.
    From { name: String, members: Vec<EntityId> },
    /// `aim N [at SET]`: a cell in range; the candidates nearest first.
    Aim { name: String, range: i64, candidates: Vec<Pos> },
    /// `ask` of `dir` or `pos`: a direction, the cell next to the actor
    /// that way for `pos`.
    Direction { name: String, pos: bool },
    /// `ask` of a number or a text: an input field.
    Field { name: String },
}

impl World<'_> {
    /// The next parameter of `actor`'s command `full` to ask, after the
    /// `given` arguments (named, or by position). A command alias or
    /// another kind asks nothing (M 8).
    pub fn next_ask(&mut self, actor: EntityId, full: &str, given: &[(Option<String>, Value)]) -> RtResult<Ask> {
        let Some(params) = self.command_params(full) else { return Ok(Ask::Ready) };
        let Some(at) = self.s.at(actor) else { return Ok(Ask::Ready) };
        let (_, ns, file) = self.decl_of(full).ok_or("no command")?;
        // the given values by parameter name
        let mut named: Vec<(String, Value)> = Vec::new();
        let mut next = 0;
        for (n, v) in given {
            let name = match n {
                Some(n) => n.clone(),
                None => {
                    next += 1;
                    match params.get(next - 1) {
                        Some(p) => p.name.clone(),
                        None => continue,
                    }
                }
            };
            named.push((name, v.clone()));
        }
        for p in &params {
            if named.iter().any(|(n, _)| *n == p.name) {
                continue;
            }
            let Some(source) = &p.source else { continue };
            let mut env = Env::new(BodyKind::Query, full, ns.clone(), &file);
            env.actor = Some(actor);
            env.self_ = Some(actor);
            env.map = Some(at.map);
            for (n, v) in &named {
                env.locals.insert(n.clone(), v.clone());
            }
            return Ok(match source {
                Source::Count => Ask::Given(p.name.clone(), Value::Number(1)),
                Source::From(e) => {
                    let members: Vec<EntityId> = match self.eval(e, &mut env)? {
                        Value::Collection(v) | Value::List(v) => v.iter().filter_map(Value::as_entity).collect(),
                        _ => vec![],
                    };
                    if members.is_empty() { Ask::Fails } else { Ask::From { name: p.name.clone(), members } }
                }
                Source::Aim { range, at: within } => {
                    let range = match self.eval(range, &mut env)? {
                        Value::Number(n) => n,
                        _ => return Ok(Ask::Fails),
                    };
                    let mut candidates: Vec<Pos> = match within {
                        None => vec![],
                        Some(w) => match self.eval(w, &mut env)? {
                            Value::Collection(v) | Value::List(v) => v
                                .iter()
                                .filter_map(|x| match x {
                                    Value::Entity(e) => self.s.at(*e),
                                    Value::Pos(q) => Some(*q),
                                    _ => None,
                                })
                                .collect(),
                            v => self.area(&v, &env, p.pos).map(|a| a.cells().to_vec()).unwrap_or_default(),
                        },
                    };
                    candidates.retain(|c| at.distance(*c) <= range);
                    candidates.sort_by_key(|c| (at.distance(*c), c.y, c.x));
                    candidates.dedup();
                    Ask::Aim { name: p.name.clone(), range, candidates }
                }
                Source::Ask(_) if matches!(p.ty.base.as_str(), "dir" | "pos") => Ask::Direction { name: p.name.clone(), pos: p.ty.base == "pos" },
                Source::Ask(_) => Ask::Field { name: p.name.clone() },
            });
        }
        Ok(Ask::Ready)
    }

    /// A command's parameters, by full name.
    fn command_params(&self, full: &str) -> Option<Vec<Param>> {
        let (item, _, _) = self.decl_of(full)?;
        match &item.kind {
            ItemKind::Command { params, .. } => Some(params.clone()),
            _ => None,
        }
    }

    /// A command's clickable parameters with what their sources name.
    pub fn verb_params(&self, full: &str) -> Vec<VerbParam> {
        let Some(params) = self.command_params(full) else { return vec![] };
        params.iter().filter(|p| clickable(p)).map(|p| VerbParam { name: p.name.clone(), deps: deps_of(p, &params) }).collect()
    }

    /// Is the command hidden from the verb coin (`noverb`, RS 7.1)?
    pub fn is_noverb(&self, full: &str) -> bool {
        self.decl_of(full).is_some_and(|(item, _, _)| matches!(item.kind, ItemKind::Command { noverb: true, .. }))
    }

    /// The verb coin (M 8.1): the candidates, in order, that `t` fits for
    /// one of their free parameters, each with that fill. A command
    /// without clickable parameters fits nothing.
    pub fn coin(&mut self, actor: EntityId, candidates: &[String], t: Target) -> RtResult<Vec<(String, (String, Value))>> {
        let mut out = Vec::new();
        for c in candidates {
            if out.iter().any(|(o, _): &(String, _)| o == c) {
                continue;
            }
            if let Some(fill) = self.fill_verb(actor, c, &[], t)? {
                out.push((c.clone(), fill));
            }
        }
        Ok(out)
    }

    /// Has the command a clickable parameter not filled yet?
    pub fn verb_open(&self, full: &str, filled: &[(String, Value)]) -> bool {
        self.verb_params(full).iter().any(|p| !filled.iter().any(|(n, _)| *n == p.name))
    }

    /// The parameter a click on `t` fills for `actor`'s command, and its
    /// value: the first free one it fits (M 8.1). `None`: it fits none.
    /// The sources are queries on the filled values, so nothing changes.
    pub fn fill_verb(&mut self, actor: EntityId, full: &str, filled: &[(String, Value)], t: Target) -> RtResult<Option<(String, Value)>> {
        let Some(params) = self.command_params(full) else { return Ok(None) };
        let Some(at) = self.s.at(actor) else { return Ok(None) };
        let (_, ns, file) = self.decl_of(full).ok_or("no command")?;
        let is_filled = |n: &str| filled.iter().any(|(f, _)| f == n);
        for p in params.iter().filter(|p| clickable(p) && !is_filled(&p.name)) {
            if !deps_of(p, &params).iter().all(|d| is_filled(d)) {
                continue;
            }
            let mut env = Env::new(BodyKind::Query, full, ns.clone(), &file);
            env.actor = Some(actor);
            env.self_ = Some(actor);
            env.map = Some(at.map);
            for (n, v) in filled {
                env.locals.insert(n.clone(), v.clone());
            }
            if let Some(v) = self.fits(p, &mut env, at, t)? {
                return Ok(Some((p.name.clone(), v)));
            }
        }
        Ok(None)
    }

    /// The value `t` gives parameter `p`, or `None` when it does not fit.
    fn fits(&mut self, p: &Param, env: &mut Env, at: Pos, t: Target) -> RtResult<Option<Value>> {
        // the cell, and the entities a click there points at: seen ones, top first
        let (cell, entities) = match t {
            Target::Cell(c) => {
                let view = self.player_view(c.map);
                let seen = view.contains(&(c.x, c.y));
                let es: Vec<EntityId> = self.s.entities_at(c).into_iter().rev().filter(|&e| seen || self.s.get(e).get("revealed").is_some()).collect();
                (Some(c), es)
            }
            Target::Entity(e) => (None, vec![e]),
        };
        Ok(match &p.source {
            Some(Source::From(e)) => {
                let set = self.eval(e, env)?;
                let members: Vec<Value> = match set {
                    Value::Collection(v) | Value::List(v) => v,
                    _ => vec![],
                };
                entities.into_iter().find(|e| members.contains(&Value::Entity(*e))).map(Value::Entity)
            }
            Some(Source::Aim { range, at: within }) => {
                let Some(c) = cell else { return Ok(None) };
                let range = match self.eval(range, env)? {
                    Value::Number(n) => n,
                    _ => return Ok(None),
                };
                if at.distance(c) > range {
                    return Ok(None);
                }
                if let Some(w) = within {
                    let ok = match self.eval(w, env)? {
                        Value::Collection(v) | Value::List(v) => entities.iter().any(|e| v.contains(&Value::Entity(*e))) || v.contains(&Value::Pos(c)),
                        v => self.area(&v, env, p.pos).is_ok_and(|a| a.contains(c)),
                    };
                    if !ok {
                        return Ok(None);
                    }
                }
                Some(Value::Pos(c))
            }
            Some(Source::Ask(_)) => {
                let Some(c) = cell else { return Ok(None) };
                let Some(d) = toward(at, c) else { return Ok(None) };
                Some(if p.ty.base == "pos" { Value::Pos(at.step(d)) } else { Value::Dir(d) })
            }
            _ => None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_nearest_of_eight_directions() {
        let a = Pos { map: 0, x: 5, y: 5 };
        let at = |x, y| toward(a, Pos { map: 0, x, y });
        assert_eq!(at(9, 5), Some(Dir::East));
        assert_eq!(at(5, 1), Some(Dir::North));
        assert_eq!(at(8, 2), Some(Dir::Northeast));
        assert_eq!(at(9, 4), Some(Dir::East), "mostly east");
        assert_eq!(at(1, 9), Some(Dir::Southwest));
        assert_eq!(at(5, 5), None);
    }
}
