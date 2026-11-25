//! The interpreter (RS 4, RS 5, RS 7, M 6, M 9).
//!
//! Every body runs on a frame machine: a block, a loop or a branch is a
//! frame, so a command can stop at `wait` and go on from the same place
//! later (M 6.1.6: a waiting command is always one frame, its own body).
//! Functions, queries, handlers and ais run to their end.
//!
//! Events go to handlers by phase (`before`, `on`, `after`), then by
//! priority, largest first, then by load order; a component's handlers see
//! their subject as `self` (RS 4, M 6.2).

use std::collections::BTreeMap;
use std::sync::Arc;

use super::rng::Rng;
use super::state::{Place, SeatId};
use super::value::{Area, Card, CommandValue, ComponentValue, Dir, EntityId, MapId, Pos as Cell, Side, Value};
use super::world::{DEFAULT_COST, LogEntry, Noise, RtResult, Travel, World, delay, number_value};
use crate::model::{Def, Kind, Source as DefSource};
use crate::names;
use crate::rs::ast::*;
use crate::rs::types::{self, Ty};

/// What kind of body runs: it decides the random stream and what is
/// allowed (M 4.1, M 9.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodyKind {
    Function,
    Query,
    Present,
    Command,
    Handler,
    Ai,
    Generator,
    Scene,
}

/// A body's context: locals, context names, its map and stream (M 6.2).
#[derive(Debug, Clone, PartialEq)]
pub struct Env {
    pub locals: BTreeMap<String, Value>,
    pub self_: Option<EntityId>,
    pub holder: Option<EntityId>,
    pub actor: Option<EntityId>,
    pub it: Vec<Value>,
    pub kind: BodyKind,
    pub ns: Option<String>,
    pub map: Option<MapId>,
    /// A query's copy of its caller's stream (M 4.1).
    pub rng: Option<Rng>,
    /// The running declaration's full name.
    pub name: String,
    pub file: String,
}

impl Env {
    pub fn new(kind: BodyKind, name: &str, ns: Option<String>, file: &str) -> Env {
        Env { locals: BTreeMap::new(), self_: None, holder: None, actor: None, it: vec![], kind, ns, map: None, rng: None, name: name.to_string(), file: file.to_string() }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    Block { stmts: Vec<Stmt>, i: usize },
    For { var: String, items: Vec<Value>, k: usize, body: Vec<Stmt> },
    While { cond: Expr, body: Vec<Stmt> },
    Repeat { left: i64, body: Vec<Stmt> },
}

impl Frame {
    fn is_loop(&self) -> bool {
        !matches!(self, Frame::Block { .. })
    }
}

/// How a body stopped.
#[derive(Debug, Clone, PartialEq)]
pub enum Flow {
    /// The end or `return`, with its value.
    Done(Option<Value>),
    /// `wait N` in microsteps, before speed (M 6.1.6).
    Wait(i64),
    Cancel,
}

/// A command waiting at `wait`: its frames and context (M 6.1.6).
#[derive(Debug, Clone, PartialEq)]
pub struct Suspended {
    pub command: String,
    pub cost: i64,
    pub frames: Vec<Frame>,
    pub env: Env,
}

/// How a command ended (M 6.1.5).
#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    /// The body returned `false`. `waited` tells if it waited before.
    Failed { waited: bool },
    /// The body ended; the cost is the last continuation's (M 6.1.5).
    Done { cost: i64 },
    /// The body waits `delay` microsteps (already scaled by speed).
    Waiting { delay: i64, state: Box<Suspended> },
}

const CONTEXT: &[&str] = &["self", "holder", "actor", "it", "player", "players", "game", "map", "camera", "profile"];

impl World<'_> {
    fn rt_err(&self, env: &Env, pos: Pos, msg: impl Into<String>) -> String {
        format!("{}:{}: runtime error: column {}: {}", env.file, pos.line, pos.col, msg.into())
    }

    pub(crate) fn decl_of(&self, full: &str) -> Option<(Arc<Item>, Option<String>, String)> {
        match &self.m.defs.get(full)?.source {
            DefSource::Decl(d) => Some((d.item.clone(), d.namespace.clone(), d.file.clone())),
            _ => None,
        }
    }

    fn resolve(&self, env: &Env, name: &str) -> Option<&Def> {
        self.m.resolve(env.ns.as_deref(), name).ok()
    }

    fn rng_for<'a>(&'a mut self, env: &'a mut Env) -> &'a mut Rng {
        match env.rng.as_mut() {
            Some(r) => r,
            None => &mut self.rng,
        }
    }

    // ---- bodies ----

    /// Runs frames until the body ends, returns or waits.
    pub fn run_frames(&mut self, frames: &mut Vec<Frame>, env: &mut Env) -> RtResult<Flow> {
        let mut steps: u64 = 0;
        loop {
            let Some(top) = frames.last_mut() else { return Ok(Flow::Done(None)) };
            match top {
                Frame::Block { stmts, i } => {
                    if *i >= stmts.len() {
                        frames.pop();
                        continue;
                    }
                    let stmt = stmts[*i].clone();
                    *i += 1;
                    if let Some(flow) = self.exec(&stmt, frames, env)? {
                        return Ok(flow);
                    }
                }
                Frame::For { var, items, k, body } => {
                    if *k >= items.len() {
                        frames.pop();
                        continue;
                    }
                    let v = items[*k].clone();
                    *k += 1;
                    let (var, body) = (var.clone(), body.clone());
                    env.locals.insert(var, v);
                    frames.push(Frame::Block { stmts: body, i: 0 });
                }
                Frame::While { cond, body } => {
                    let (cond, body) = (cond.clone(), body.clone());
                    steps += 1;
                    if steps > self.s.game.loop_steps {
                        return Err(self.rt_err(env, cond.pos, format!("the loop went over `game.loop_steps` ({})", self.s.game.loop_steps)));
                    }
                    if self.truth(&cond, env)? {
                        frames.push(Frame::Block { stmts: body, i: 0 });
                    } else {
                        frames.pop();
                    }
                }
                Frame::Repeat { left, body } => {
                    if *left <= 0 {
                        frames.pop();
                        continue;
                    }
                    *left -= 1;
                    let body = body.clone();
                    frames.push(Frame::Block { stmts: body, i: 0 });
                }
            }
        }
    }

    /// Runs a body that cannot wait to its end.
    fn run_to_end(&mut self, body: &[Stmt], env: &mut Env) -> RtResult<Flow> {
        let mut frames = vec![Frame::Block { stmts: body.to_vec(), i: 0 }];
        match self.run_frames(&mut frames, env)? {
            Flow::Wait(_) => Err(format!("{}: runtime error: `wait` outside a command", env.file)),
            f => Ok(f),
        }
    }

    fn exec(&mut self, s: &Stmt, frames: &mut Vec<Frame>, env: &mut Env) -> RtResult<Option<Flow>> {
        match &s.kind {
            StmtKind::Assign { target, op, value } => {
                let v = self.eval(value, env)?;
                let v = match op {
                    AssignOp::Set => v,
                    AssignOp::Add => {
                        let old = self.eval(target, env)?;
                        match old {
                            Value::List(mut l) => {
                                l.push(v);
                                Value::List(l)
                            }
                            old => self.arith(BinOp::Add, old, v, env, s.pos)?,
                        }
                    }
                    AssignOp::Sub => {
                        let old = self.eval(target, env)?;
                        self.arith(BinOp::Sub, old, v, env, s.pos)?
                    }
                };
                self.assign(target, v, env)?;
            }
            StmtKind::If { arms, otherwise } => {
                for (c, b) in arms {
                    if self.truth(c, env)? {
                        frames.push(Frame::Block { stmts: b.clone(), i: 0 });
                        return Ok(None);
                    }
                }
                if let Some(b) = otherwise {
                    frames.push(Frame::Block { stmts: b.clone(), i: 0 });
                }
            }
            StmtKind::For { var, iter, body } => {
                let v = self.eval(iter, env)?;
                let items = v.members().ok_or_else(|| self.rt_err(env, iter.pos, format!("`for` goes over a set, not a {}", v.type_name())))?;
                frames.push(Frame::For { var: var.clone(), items, k: 0, body: body.clone() });
            }
            StmtKind::Repeat { count, body } => {
                let n = self.eval(count, env)?;
                let n = self.number(n, env, count.pos)?;
                frames.push(Frame::Repeat { left: n, body: body.clone() });
            }
            StmtKind::While { cond, body } => frames.push(Frame::While { cond: cond.clone(), body: body.clone() }),
            StmtKind::Break => {
                while let Some(f) = frames.pop() {
                    if f.is_loop() {
                        break;
                    }
                }
            }
            StmtKind::Match { subject, cases } => {
                let v = self.eval(subject, env)?;
                for (case, body) in cases {
                    let hit = match case {
                        None => true,
                        Some(c) => {
                            let cv = self.eval(c, env)?;
                            match (&cv, &v) {
                                (Value::Range(a, b), Value::Number(n)) => n >= a && b.is_none_or(|b| *n <= b),
                                _ => cv == v,
                            }
                        }
                    };
                    if hit {
                        frames.push(Frame::Block { stmts: body.clone(), i: 0 });
                        break;
                    }
                }
            }
            StmtKind::Emit(call) => {
                let ExprKind::Call(f, args) = &call.kind else { return Err(self.rt_err(env, s.pos, "`emit` takes an event call")) };
                let ExprKind::Name { name, .. } = &f.kind else { return Err(self.rt_err(env, s.pos, "`emit` takes an event")) };
                let event = self.resolve(env, name).map(|d| d.full.clone()).ok_or_else(|| self.rt_err(env, s.pos, format!("no event `{name}`")))?;
                let mut vals = Vec::new();
                for a in args {
                    vals.push((a.name.clone(), self.eval(&a.value, env)?));
                }
                let bound = self.bind_event_args(&event, vals, env, s.pos)?;
                self.emit(&event, bound, env)?;
            }
            StmtKind::Wait(WaitArg::Time(e)) => {
                let n = self.eval(e, env)?;
                let n = self.number(n, env, e.pos)?;
                return Ok(Some(Flow::Wait(n)));
            }
            StmtKind::Wait(_) | StmtKind::WaitInput { .. } | StmtKind::Open(_) | StmtKind::OpenValue { .. } | StmtKind::Choice(_) => {
                return Err(self.rt_err(env, s.pos, "scenes, dialogs, menus and input waits are not supported yet"));
            }
            StmtKind::Return(v) => {
                let v = match v {
                    Some(e) => Some(self.eval(e, env)?),
                    None => None,
                };
                return Ok(Some(Flow::Done(v)));
            }
            StmtKind::Cancel => return Ok(Some(Flow::Cancel)),
            StmtKind::Call(e) => {
                self.eval(e, env)?;
            }
        }
        Ok(None)
    }

    fn truth(&mut self, e: &Expr, env: &mut Env) -> RtResult<bool> {
        let v = self.eval(e, env)?;
        v.as_bool().ok_or_else(|| self.rt_err(env, e.pos, format!("a condition is a bool, not a {} (`{e}`)", v.type_name())))
    }

    /// A number: dice and ranges are rolled where a number is read (D 5.2).
    fn number(&mut self, v: Value, env: &mut Env, pos: Pos) -> RtResult<i64> {
        match v {
            Value::Number(n) => Ok(n),
            Value::Dice { n, sides, bonus } => Ok(self.rng_for(env).dice(n, sides, bonus)),
            Value::Range(a, Some(b)) => Ok(self.rng_for(env).range(a, b).unwrap_or(a)),
            v => Err(self.rt_err(env, pos, format!("a number, not a {} (`{v}`)", v.type_name()))),
        }
    }

    // ---- expressions ----

    pub fn eval(&mut self, e: &Expr, env: &mut Env) -> RtResult<Value> {
        use ExprKind::*;
        Ok(match &e.kind {
            Int(n) => Value::Number(*n),
            Dice(n, s) => Value::Dice { n: *n, sides: *s, bonus: 0 },
            Time(ms, _) => Value::Time(*ms),
            Percent(p) => Value::Percent(*p),
            Size(w, h) => Value::Size(*w, *h),
            Str(s) => Value::Text(s.clone()),
            Bool(b) => Value::Bool(*b),
            None => Value::None,
            All => Value::All,
            EmptyDict => Value::Dict(vec![]),
            Name { name, root } => {
                let written = if *root { format!(".{name}") } else { name.clone() };
                self.name_value(&written, env)
            }
            Word(w) => self.word(w, env, e.pos)?,
            Wild { name, deep, .. } => {
                let mut out: Vec<String> = Vec::new();
                for c in names::candidates(env.ns.as_deref(), name) {
                    let prefix = format!("{c}.");
                    let one = |k: &str| k.strip_prefix(&prefix).is_some_and(|r| *deep || !r.contains('.'));
                    out.extend(self.m.defs.keys().filter(|k| one(k)).cloned());
                    for f in &self.m.languages {
                        out.extend(f.texts.iter().map(|t| names::qualify(f.namespace.as_deref(), t.key())).filter(|k| one(k)));
                    }
                    if !out.is_empty() {
                        break;
                    }
                }
                out.sort();
                out.dedup();
                Value::Collection(out.into_iter().map(Value::Name).collect())
            }
            List(items) => {
                let mut out = Vec::new();
                for i in items {
                    out.push(self.eval(i, env)?);
                }
                Value::List(out)
            }
            Field(..) | TileIndex(..) => self.path(e, env)?,
            Call(f, args) => self.call(e, f, args, env)?,
            Subscript(a, i) => {
                let (av, iv) = (self.eval(a, env)?, self.eval(i, env)?);
                match av {
                    // a collection reads by place too, read-only (RS 2.6)
                    Value::List(l) | Value::Collection(l) => {
                        let k = self.number(iv, env, i.pos)?;
                        usize::try_from(k).ok().and_then(|k| l.get(k).cloned()).unwrap_or(Value::None)
                    }
                    Value::Dict(d) => d.into_iter().find(|(k, _)| *k == iv).map(|(_, v)| v).unwrap_or(Value::None),
                    v => return Err(self.rt_err(env, e.pos, format!("`[…]` reads a list or dictionary, not a {}", v.type_name()))),
                }
            }
            Neg(a) => match self.eval(a, env)? {
                Value::Number(n) => Value::Number(-n),
                Value::Percent(p) => Value::Percent(-p),
                Value::Dice { n, sides, bonus } => Value::Number(-self.rng_for(env).dice(n, sides, bonus)),
                Value::Dir(d) => Value::Dir(d.opposite()),
                Value::Side(s) => Value::Side(s.opposite()),
                v => return Err(self.rt_err(env, e.pos, format!("`-` takes a number or direction, not a {}", v.type_name()))),
            },
            Not(a) => Value::Bool(!self.truth(a, env)?),
            Binary(op, a, b) => self.binary(*op, a, b, env, e.pos)?,
            Range(a, b) => {
                let a = self.eval(a, env)?;
                let a = self.number(a, env, e.pos)?;
                let b = match b {
                    Some(b) => {
                        let v = self.eval(b, env)?;
                        Some(self.number(v, env, e.pos)?)
                    }
                    Option::None => Option::None,
                };
                Value::Range(a, b)
            }
            Has { subject, no, component } => {
                let full = if names::is_vocabulary(component) { component.clone() } else { self.resolve(env, component).map(|d| d.full.clone()).unwrap_or_else(|| component.clone()) };
                // `game has PERMADEATH` asks the game record (D 5.5)
                let is_game = matches!(&subject.kind, Word(w) if w == "game" && !env.locals.contains_key(w));
                let has = if is_game {
                    self.s.game.components.iter().any(|c| c.name == full)
                } else {
                    match self.eval(subject, env)? {
                        Value::Entity(x) => self.s.get(x).alive && self.s.get(x).has(&full),
                        Value::None => false,
                        v => return Err(self.rt_err(env, e.pos, format!("`has` asks an entity, not a {}", v.type_name()))),
                    }
                };
                Value::Bool(has != *no)
            }
            Is(a, b) => {
                let (av, bv) = (self.eval(a, env)?, self.eval(b, env)?);
                Value::Bool(self.is(&av, &bv))
            }
            In(a, b) => {
                let (av, bv) = (self.eval(a, env)?, self.eval(b, env)?);
                Value::Bool(self.contains(&bv, &av))
            }
            Where(a, cond) => {
                let av = self.eval(a, env)?;
                let items = av.members().ok_or_else(|| self.rt_err(env, e.pos, format!("`where` filters a set, not a {}", av.type_name())))?;
                let mut kept = Vec::new();
                for it in items {
                    env.it.push(it.clone());
                    let keep = self.truth(cond, env);
                    env.it.pop();
                    if keep? {
                        kept.push(it);
                    }
                }
                // the result is of the source's kind (RS 5.3)
                match av {
                    Value::Area(_) => Value::Area(Area::new(kept.into_iter().filter_map(|v| if let Value::Pos(p) = v { Some(p) } else { Option::None }).collect())),
                    Value::List(_) | Value::Range(..) | Value::Dict(_) => Value::List(kept),
                    _ => Value::Collection(kept),
                }
            }
        })
    }

    /// An upper-case name: a constant's value, a deck or table, or the full
    /// name; text keys and labels stay names.
    fn name_value(&self, written: &str, env: &Env) -> Value {
        match self.resolve(env, written) {
            Some(d) => match d.kind {
                Kind::Deck => Value::Deck(d.full.clone()),
                Kind::Table => Value::Table(d.full.clone()),
                Kind::Constant => match &d.source {
                    DefSource::Row(r) => number_value(r.rest(1)).unwrap_or_else(|| Value::Text(r.rest(1).trim_matches('"').to_string())),
                    _ => Value::None,
                },
                _ => Value::Name(d.full.clone()),
            },
            None => {
                for c in names::candidates(env.ns.as_deref(), written) {
                    if self.m.languages.iter().any(|f| f.texts.iter().any(|t| names::qualify(f.namespace.as_deref(), t.key()) == c)) {
                        return Value::Name(c);
                    }
                }
                Value::Name(written.trim_start_matches('.').to_string())
            }
        }
    }

    fn word(&mut self, w: &str, env: &mut Env, pos: Pos) -> RtResult<Value> {
        if w == "it" {
            return env.it.last().cloned().ok_or_else(|| self.rt_err(env, pos, "`it` outside `where`"));
        }
        if let Some(v) = env.locals.get(w) {
            return Ok(v.clone());
        }
        let ent = |e: Option<EntityId>| e.map(Value::Entity).unwrap_or(Value::None);
        Ok(match w {
            "self" => ent(env.self_),
            "holder" => ent(env.holder),
            "actor" => ent(env.actor),
            "player" => ent(self.s.player),
            "players" => Value::Collection(self.s.players().into_iter().map(Value::Entity).collect()),
            "map" => env.map.map(|m| Value::Name(self.s.maps[m].name.clone())).unwrap_or(Value::None),
            _ if CONTEXT.contains(&w) => Value::None,
            _ => {
                if let Some(d) = Dir::parse(w) {
                    Value::Dir(d)
                } else if let Some(s) = Side::parse(w) {
                    Value::Side(s)
                } else {
                    // vocabulary values: `move`, `sight`, `line`, …
                    Value::Name(w.to_string())
                }
            }
        })
    }

    fn is(&self, a: &Value, b: &Value) -> bool {
        let kind = |v: &Value| match v {
            Value::Entity(e) => Some(self.s.get(*e).kind.clone()),
            Value::Name(n) => Some(n.clone()),
            _ => None,
        };
        let (Some(ka), Some(kb)) = (kind(a), kind(b)) else { return false };
        // the kind or one it extends (RS 5.3)
        let mut cur = Some(ka);
        for _ in 0..64 {
            let Some(k) = cur else { return false };
            if k == kb {
                return true;
            }
            cur = self.m.defs.get(&k).and_then(|d| d.block()).and_then(|b| {
                let parent = b.arg_after("extends")?;
                self.m.resolve(b.namespace.as_deref(), parent).ok().map(|d| d.full.clone())
            });
        }
        false
    }

    fn contains(&self, set: &Value, x: &Value) -> bool {
        match (set, x) {
            (Value::Range(a, b), Value::Number(n)) => n >= a && b.is_none_or(|b| *n <= b),
            (Value::Area(a), Value::Pos(p)) => a.contains(*p),
            (Value::Area(a), Value::Entity(e)) => self.s.at(*e).is_some_and(|p| a.contains(p)),
            (Value::Collection(v) | Value::List(v), x) => v.contains(x),
            (Value::Dict(d), x) => d.iter().any(|(k, _)| k == x),
            // `pos in FLOOR`: a layer's tile is the name or its variant (RS 5.3)
            (Value::Name(n), Value::Pos(p)) => self.cell_has(*p, n),
            _ => false,
        }
    }

    fn cell_has(&self, p: Cell, name: &str) -> bool {
        let m = &self.s.maps[p.map];
        (0..m.layers.len()).any(|l| m.tile(l, p.x, p.y).is_some_and(|t| tile_matches(t, name)))
    }

    fn binary(&mut self, op: BinOp, a: &Expr, b: &Expr, env: &mut Env, pos: Pos) -> RtResult<Value> {
        use BinOp::*;
        let av = self.eval(a, env)?;
        if matches!(op, And | Or) && let Value::Bool(x) = av {
            // `and` and `or` short-circuit on truth values
            if (op == And && !x) || (op == Or && x) {
                return Ok(Value::Bool(x));
            }
            return Ok(Value::Bool(self.truth(b, env)?));
        }
        let bv = self.eval(b, env)?;
        Ok(match op {
            Eq => Value::Bool(self.equal(av, bv, env)),
            Ne => Value::Bool(!self.equal(av, bv, env)),
            Lt | Le | Gt | Ge => {
                let x = self.number(av, env, pos)?;
                let y = self.number(bv, env, pos)?;
                Value::Bool(match op {
                    Lt => x < y,
                    Le => x <= y,
                    Gt => x > y,
                    _ => x >= y,
                })
            }
            And | Or => {
                let (x, y) = (self.area(&av, env, pos)?, self.area(&bv, env, pos)?);
                Value::Area(if op == And { x.and(&y) } else { x.or(&y) })
            }
            op => self.arith(op, av, bv, env, pos)?,
        })
    }

    fn equal(&mut self, a: Value, b: Value, env: &mut Env) -> bool {
        match (&a, &b) {
            (Value::Dice { .. } | Value::Range(..), Value::Number(_)) | (Value::Number(_), Value::Dice { .. }) => {
                let p = Pos::default();
                self.number(a, env, p).ok() == self.number(b, env, p).ok()
            }
            _ => a == b,
        }
    }

    fn arith(&mut self, op: BinOp, a: Value, b: Value, env: &mut Env, pos: Pos) -> RtResult<Value> {
        use BinOp::*;
        Ok(match (op, &a, &b) {
            (Add, Value::Pos(p), Value::Dir(d)) => Value::Pos(p.step(*d)),
            (Add, Value::Text(x), Value::Text(y)) => Value::Text(format!("{x}{y}")),
            (Add | Sub, Value::Percent(x), Value::Percent(y)) => Value::Percent(if op == Add { x + y } else { x - y }),
            (Mul, _, Value::Percent(p)) => Value::Number((self.number(a.clone(), env, pos)? * p).div_euclid(100)),
            (Mul, Value::Percent(p), _) => Value::Number((self.number(b.clone(), env, pos)? * p).div_euclid(100)),
            (Sub, Value::Area(_) | Value::Pos(_), _) if !matches!(b, Value::Number(_)) => {
                let (x, y) = (self.area(&a, env, pos)?, self.area(&b, env, pos)?);
                Value::Area(x.minus(&y))
            }
            _ => {
                let x = self.number(a, env, pos)?;
                let y = self.number(b, env, pos)?;
                Value::Number(match op {
                    Add => x + y,
                    Sub => x - y,
                    Mul => x * y,
                    Div | Rem if y == 0 => return Err(self.rt_err(env, pos, "division by zero")),
                    Div => x.div_euclid(y),
                    Rem => x.rem_euclid(y),
                    _ => unreachable!(),
                })
            }
        })
    }

    /// An area from an area, a position, an entity on a map or a list of
    /// positions (RS 6.2).
    pub fn area(&self, v: &Value, env: &Env, pos: Pos) -> RtResult<Area> {
        Ok(match v {
            Value::Area(a) => a.clone(),
            Value::Pos(p) => Area::one(*p),
            Value::Entity(e) => self.s.at(*e).map(Area::one).unwrap_or_default(),
            Value::List(l) | Value::Collection(l) => Area::new(l.iter().filter_map(|x| match x {
                Value::Pos(p) => Some(*p),
                Value::Entity(e) => self.s.at(*e),
                _ => None,
            }).collect()),
            Value::None => Area::default(),
            v => return Err(self.rt_err(env, pos, format!("an area, not a {}", v.type_name()))),
        })
    }

    // ---- field paths (RS 3.3) ----

    fn path(&mut self, e: &Expr, env: &mut Env) -> RtResult<Value> {
        if let ExprKind::TileIndex(base, i) = &e.kind {
            return Ok(match self.eval(base, env)? {
                Value::Name(n) => Value::Name(format!("{n}.{i}")),
                v => v,
            });
        }
        let mut parts = Vec::new();
        let mut root = e;
        while let ExprKind::Field(base, part) = &root.kind {
            parts.push(part.clone());
            root = base;
        }
        parts.reverse();
        // a tile variant `GATE.open`
        if let ExprKind::Name { name, root: r } = &root.kind {
            let written = if *r { format!(".{name}") } else { name.clone() };
            return Ok(match self.name_value(&written, env) {
                Value::Name(n) => Value::Name(format!("{n}.{}", parts.join("."))),
                v => self.fields_of(v, &parts, env, e.pos)?,
            });
        }
        if let ExprKind::Word(w) = &root.kind
            && !env.locals.contains_key(w)
        {
            match w.as_str() {
                "game" => {
                    let first = &parts[0];
                    if names::is_vocabulary(first) {
                        let g = &self.s.game;
                        let v = match first.as_str() {
                            "start" => g.start.clone().map(Value::Name).unwrap_or(Value::None),
                            "at" => Value::Area(g.at.clone()),
                            "seed" => Value::Number(g.seed as i64),
                            "time" => Value::Number(g.time),
                            "turn" => Value::Number(g.turn()),
                            "call_depth" => Value::Number(g.call_depth as i64),
                            "loop_steps" => Value::Number(g.loop_steps as i64),
                            "party" => Value::Name(if self.s.party_each { "each" } else { "switch" }.into()),
                            "leash" => Value::Bool(self.s.leash),
                            "seats" => Value::Range(self.s.seat_range.0 as i64, Some(self.s.seat_range.1 as i64)),
                            _ => Value::None,
                        };
                        return self.fields_of(v, &parts[1..], env, e.pos);
                    }
                    let (comp, n) = self.component_at(&parts, env).ok_or_else(|| self.rt_err(env, e.pos, format!("`game` has no component `{first}`")))?;
                    let cv = self.s.game.components.iter().find(|c| c.name == comp).cloned();
                    let Some(cv) = cv else { return Ok(Value::None) };
                    return self.component_fields_of(cv, crate::rt::world::GAME, &parts[n..], env, e.pos);
                }
                "map" => {
                    let Some(m) = env.map else { return Ok(Value::None) };
                    let var = parts[0].clone();
                    let v = self.s.maps[m].vars.iter().find(|(k, _)| *k == var).map(|(_, v)| v.clone()).unwrap_or(Value::Number(0));
                    return self.fields_of(v, &parts[1..], env, e.pos);
                }
                _ => {}
            }
        }
        let v = self.eval(root, env)?;
        self.fields_of(v, &parts, env, e.pos)
    }

    /// The longest run of upper-case parts that names a component.
    fn component_at(&self, parts: &[String], env: &Env) -> Option<(String, usize)> {
        let upper = parts.iter().take_while(|p| names::is_part(p)).count();
        (1..=upper).rev().find_map(|n| {
            let name = parts[..n].join(".");
            self.resolve(env, &name).filter(|d| d.kind == Kind::Component).map(|d| (d.full.clone(), n))
        })
    }

    fn fields_of(&mut self, mut v: Value, parts: &[String], env: &mut Env, pos: Pos) -> RtResult<Value> {
        let mut i = 0;
        while i < parts.len() {
            let p = &parts[i];
            v = match v {
                Value::Entity(e) => {
                    let inst = self.s.get(e);
                    if !inst.alive {
                        // a dead handle's fields are `none` (RS 4)
                        return Ok(match p.as_str() {
                            "kind" => Value::Name(inst.kind.clone()),
                            _ => Value::None,
                        });
                    }
                    match p.as_str() {
                        "at" => self.s.at(e).map(Value::Pos).unwrap_or(Value::None),
                        "map" => self.s.at(e).map(|q| Value::Name(self.s.maps[q.map].name.clone())).unwrap_or(Value::None),
                        "kind" => Value::Name(inst.kind.clone()),
                        "slot" => match inst.place {
                            Place::In { slot, .. } => Value::Number(slot as i64),
                            _ => Value::None,
                        },
                        "stack" => Value::Collection(vec![Value::Entity(e)]),
                        _ => {
                            let (comp, n) = if names::is_vocabulary(p) {
                                (p.clone(), 1)
                            } else {
                                self.component_at(&parts[i..], env).ok_or_else(|| self.rt_err(env, pos, format!("no component `{p}`")))?
                            };
                            let Some(cv) = inst.get(&comp).cloned() else {
                                self.warnings.push(format!("{}:{}: `{}` has no `{comp}`; the field reads as none (RS 5.3)", env.file, pos.line, inst.kind));
                                return Ok(Value::None);
                            };
                            return self.component_fields_of(cv, e, &parts[i + n..], env, pos);
                        }
                    }
                }
                Value::Component(cv) => match cv.get(p) {
                    Some(f) => f.clone(),
                    None => Value::None,
                },
                Value::Pos(q) => match p.as_str() {
                    "x" => Value::Number(q.x as i64),
                    "y" => Value::Number(q.y as i64),
                    _ => Value::None,
                },
                Value::Sound(s) => match p.as_str() {
                    "kind" => Value::Name(s.kind),
                    "at" => Value::Pos(s.at),
                    "source" => s.source.map(Value::Entity).unwrap_or(Value::None),
                    "level" => Value::Number(s.level),
                    _ => Value::None,
                },
                Value::Deck(d) if p == "count" => Value::Number(self.s.deck(&d).map(|s| s.draw.len()).unwrap_or(0) as i64),
                ref set @ (Value::Collection(_) | Value::List(_) | Value::Area(_) | Value::Dict(_)) => {
                    let items = set.members().unwrap_or_default();
                    match p.as_str() {
                        "count" => Value::Number(items.len() as i64),
                        "first" => items.first().cloned().unwrap_or(Value::None),
                        "last" => items.last().cloned().unwrap_or(Value::None),
                        "keys" => Value::List(items),
                        _ => return Err(self.rt_err(env, pos, format!("a set has no field `{p}`"))),
                    }
                }
                Value::None => return Ok(Value::None),
                other => return Err(self.rt_err(env, pos, format!("a {} has no field `{p}`", other.type_name()))),
            };
            i += 1;
        }
        Ok(v)
    }

    /// The fields of a component value on an instance; a collection field
    /// reads as its members (D 5.4).
    fn component_fields_of(&mut self, cv: ComponentValue, holder: EntityId, rest: &[String], env: &mut Env, pos: Pos) -> RtResult<Value> {
        let Some(field) = rest.first() else { return Ok(Value::Component(cv)) };
        let v = match cv.get(field) {
            Some(Value::Collection(_)) => Value::Collection(self.s.members(holder, &cv.name, field).into_iter().map(Value::Entity).collect()),
            Some(v) => v.clone(),
            None => Value::None,
        };
        self.fields_of(v, &rest[1..], env, pos)
    }

    // ---- assignment ----

    fn assign(&mut self, target: &Expr, v: Value, env: &mut Env) -> RtResult<()> {
        match &target.kind {
            ExprKind::Word(w) => {
                env.locals.insert(w.clone(), v);
                Ok(())
            }
            ExprKind::Subscript(base, i) => {
                let iv = self.eval(i, env)?;
                let container = self.eval(base, env)?;
                let updated = match container {
                    Value::List(mut l) => {
                        let k = self.number(iv, env, i.pos)? as usize;
                        if k >= l.len() {
                            return Err(self.rt_err(env, target.pos, format!("list index {k} is outside the list of {}", l.len())));
                        }
                        l[k] = v;
                        Value::List(l)
                    }
                    Value::Dict(mut d) => {
                        match d.iter_mut().find(|(k, _)| *k == iv) {
                            Some((_, old)) => *old = v,
                            None => d.push((iv, v)),
                        }
                        Value::Dict(d)
                    }
                    other => return Err(self.rt_err(env, target.pos, format!("`[…] =` writes a list or dictionary, not a {}", other.type_name()))),
                };
                self.assign(base, updated, env)
            }
            ExprKind::Field(..) => {
                let mut parts = Vec::new();
                let mut root = target;
                while let ExprKind::Field(base, part) = &root.kind {
                    parts.push(part.clone());
                    root = base;
                }
                parts.reverse();
                let last = parts.pop().unwrap();
                // `game.COMP.field`
                if let ExprKind::Word(w) = &root.kind
                    && w == "game"
                    && !env.locals.contains_key(w)
                {
                    let (comp, _) = self.component_at(&parts, env).ok_or_else(|| self.rt_err(env, target.pos, "no such `game` component"))?;
                    let c = self.s.game.components.iter_mut().find(|c| c.name == comp).ok_or_else(|| format!("`game` has no `{comp}`; add it in the `game` block"))?;
                    c.set(&last, v);
                    return Ok(());
                }
                if let ExprKind::Word(w) = &root.kind
                    && w == "map"
                    && !env.locals.contains_key(w)
                    && parts.is_empty()
                {
                    let m = env.map.ok_or_else(|| self.rt_err(env, target.pos, "no map"))?;
                    let vars = &mut self.s.maps[m].vars;
                    match vars.iter_mut().find(|(k, _)| *k == last) {
                        Some((_, old)) => *old = v,
                        None => vars.push((last, v)),
                    }
                    return Ok(());
                }
                // the instance and component that hold the field
                let base_val = {
                    let mut b = self.eval(root, env)?;
                    let mut i = 0;
                    let mut comp = None;
                    while i < parts.len() {
                        match &b {
                            Value::Entity(_) => {
                                let (c, n) = if names::is_vocabulary(&parts[i]) { (parts[i].clone(), 1) } else { self.component_at(&parts[i..], env).ok_or_else(|| self.rt_err(env, target.pos, format!("no component `{}`", parts[i])))? };
                                comp = Some(c);
                                i += n;
                                if i < parts.len() {
                                    return Err(self.rt_err(env, target.pos, "nested field writes are not supported yet"));
                                }
                            }
                            _ => {
                                b = self.fields_of(b, &parts[i..i + 1], env, target.pos)?;
                                i += 1;
                            }
                        }
                    }
                    (b, comp)
                };
                match base_val {
                    (Value::Entity(e), Some(comp)) => {
                        let kind = self.s.get(e).kind.clone();
                        let c = self.s.get_mut(e).get_mut(&comp).ok_or_else(|| format!("{}:{}: runtime error: `{kind}` has no `{comp}`; add it with `add` (RS 5.3)", env.file, target.pos.line))?;
                        c.set(&last, v);
                        Ok(())
                    }
                    (Value::Entity(_), None) => Err(self.rt_err(env, target.pos, format!("`{last}` is kept by the engine"))),
                    (Value::Component(mut cv), _) => {
                        // a local component value: write it back
                        cv.set(&last, v);
                        let base_expr = match &target.kind {
                            ExprKind::Field(b, _) => b.as_ref().clone(),
                            _ => unreachable!(),
                        };
                        self.assign(&base_expr, Value::Component(cv), env)
                    }
                    (other, _) => Err(self.rt_err(env, target.pos, format!("cannot write a field of a {}", other.type_name()))),
                }
            }
            _ => Err(self.rt_err(env, target.pos, format!("cannot assign to `{target}`"))),
        }
    }

    // ---- calls ----

    fn call(&mut self, e: &Expr, f: &Expr, args: &[Arg], env: &mut Env) -> RtResult<Value> {
        match &f.kind {
            ExprKind::Name { name, root } => {
                let written = if *root { format!(".{name}") } else { name.clone() };
                let def = self.resolve(env, &written).cloned().ok_or_else(|| self.rt_err(env, f.pos, format!("`{name}` is not defined")))?;
                let mut vals = Vec::new();
                for a in args {
                    vals.push((a.name.clone(), self.eval(&a.value, env)?));
                }
                match def.kind {
                    Kind::Function | Kind::Query => self.call_function(&def.full, vals, env),
                    Kind::Command => Ok(Value::Command(CommandValue { name: def.full.clone(), args: vals })),
                    Kind::Ai => {
                        // another ai for the same `self` (RS 7.2)
                        let me = env.self_.ok_or_else(|| self.rt_err(env, f.pos, "an ai runs for `self`"))?;
                        self.run_ai(&def.full, me)
                    }
                    Kind::Entity => {
                        let mut card = Card { entity: Some(def.full.clone()), count: None, overrides: vec![] };
                        for (n, v) in vals {
                            match n {
                                None => card.count = Some(Box::new(v)),
                                Some(c) => {
                                    let full = if names::is_vocabulary(&c) { c } else { self.resolve(env, &c).map(|d| d.full.clone()).unwrap_or(c) };
                                    card.overrides.push((full, v));
                                }
                            }
                        }
                        Ok(Value::Card(card))
                    }
                    Kind::Component => Ok(Value::Component(self.component_value(&def.full, vals))),
                    k => Err(self.rt_err(env, f.pos, format!("a {k} cannot be called"))),
                }
            }
            ExprKind::Word(w) => {
                if super::super::fields::engine_component(w).is_some() && crate::rs::sigs::kind_of(w).is_none() {
                    let mut vals = Vec::new();
                    for a in args {
                        let v = match (&a.value.kind, a.name.as_deref()) {
                            // a movement mode is a name, not a definition (D 7.5)
                            (ExprKind::Name { name, .. }, Some("mode")) => Value::Name(name.clone()),
                            _ => self.eval(&a.value, env)?,
                        };
                        vals.push((a.name.clone(), v));
                    }
                    return Ok(Value::Component(self.component_value(w, vals)));
                }
                let mut vals = Vec::new();
                for a in args {
                    vals.push((a.name.clone(), self.engine_arg(w, a, env)?));
                }
                self.engine(w, vals, env, e.pos)
            }
            ExprKind::Field(recv, w) if names::is_vocabulary(w) => {
                let mut vals = vec![(None, self.eval(recv, env)?)];
                for a in args {
                    vals.push((a.name.clone(), self.engine_arg(w, a, env)?));
                }
                self.engine(w, vals, env, e.pos)
            }
            _ => Err(self.rt_err(env, e.pos, format!("`{f}` cannot be called"))),
        }
    }

    /// Engine arguments: a collection field given as `to:` or `into:` is a
    /// place, and labels and movement modes are names as written.
    fn engine_arg(&mut self, f: &str, a: &Arg, env: &mut Env) -> RtResult<Value> {
        if let ExprKind::Name { name, .. } = &a.value.kind {
            let label = (f == "label") || (f == "tag") || (["travel", "spawn"].contains(&f) && a.name.as_deref() == Some("at")) || (f == "tile" && a.name.as_deref() == Some("label"));
            let mode = (f == "ray" && a.name.as_deref() == Some("mode")) || f == "blocks";
            if (label || mode) && self.resolve(env, name).is_none() {
                return Ok(Value::Name(name.clone()));
            }
        }
        if ["move", "spawn"].contains(&f)
            && matches!(a.name.as_deref(), Some("to" | "into"))
            && let Some(place) = self.collection_place(&a.value, env)?
        {
            return Ok(place);
        }
        self.eval(&a.value, env)
    }

    /// `actor.PACK.items` as a place: a list with the holder, component and
    /// field, marked by the text `"@collection"`.
    fn collection_place(&mut self, e: &Expr, env: &mut Env) -> RtResult<Option<Value>> {
        let ExprKind::Field(base, field) = &e.kind else { return Ok(None) };
        let mut parts = vec![];
        let mut root = base.as_ref();
        while let ExprKind::Field(b, p) = &root.kind {
            parts.push(p.clone());
            root = b;
        }
        parts.reverse();
        let holder = self.eval(root, env)?;
        let Value::Entity(h) = holder else { return Ok(None) };
        let Some((comp, n)) = self.component_at(&parts, env) else { return Ok(None) };
        if n != parts.len() || !matches!(self.s.get(h).get(&comp).and_then(|c| c.get(field)), Some(Value::Collection(_))) {
            return Ok(None);
        }
        Ok(Some(Value::List(vec![Value::Text("@collection".into()), Value::Entity(h), Value::Name(comp), Value::Name(field.clone())])))
    }

    /// A component value from named or positional arguments; missing
    /// fields get their defaults when added (M 9.5).
    fn component_value(&self, comp: &str, vals: Vec<(Option<String>, Value)>) -> ComponentValue {
        let defs = if names::is_vocabulary(comp) { crate::fields::engine_component(comp).unwrap_or_default() } else { self.m.defs.get(comp).map(crate::fields::component_fields).unwrap_or_default() };
        let mut cv = ComponentValue { name: comp.to_string(), fields: vec![] };
        for (i, (n, v)) in vals.into_iter().enumerate() {
            let name = n.or_else(|| defs.get(i).map(|f| f.name.clone())).unwrap_or_default();
            cv.set(&name, v);
        }
        cv
    }

    /// Binds a function's or command's parameters (RS 2.5): positional,
    /// named, then defaults; a number parameter rolls dice (D 5.2).
    fn bind(&mut self, params: &[Param], args: Vec<(Option<String>, Value)>, env: &mut Env, ns: Option<&str>, pos: Pos) -> RtResult<()> {
        let mut given: Vec<Option<Value>> = vec![None; params.len()];
        let mut next = 0;
        for (n, v) in args {
            let i = match n {
                None => {
                    next += 1;
                    next - 1
                }
                Some(n) => params.iter().position(|p| p.name == n).ok_or_else(|| self.rt_err(env, pos, format!("no parameter `{n}`")))?,
            };
            if i < given.len() {
                given[i] = Some(v);
            }
        }
        for (p, g) in params.iter().zip(given) {
            let v = match (g, &p.default) {
                (Some(v), _) => v,
                (None, Some(d)) => self.eval(d, env)?,
                (None, None) if p.source.is_some() => return Err(self.rt_err(env, pos, format!("`{}` is asked from the player; give it as an argument (input is not supported yet)", p.name))),
                (None, None) => Value::None,
            };
            let ty = types::parse(&p.ty.to_string(), &|n| self.m.resolve(ns, n).is_ok_and(|d| d.kind == Kind::Component)).unwrap_or(Ty::Unknown);
            let v = if ty == Ty::Number && matches!(v, Value::Dice { .. } | Value::Range(..)) { Value::Number(self.number(v, env, pos)?) } else { v };
            env.locals.insert(p.name.clone(), v);
        }
        Ok(())
    }

    fn enter(&mut self, env: &Env) -> RtResult<()> {
        self.depth += 1;
        if self.depth > self.s.game.call_depth {
            self.depth = 0;
            return Err(format!("{}: runtime error: the calls went over `game.call_depth` ({})", env.file, self.s.game.call_depth));
        }
        Ok(())
    }

    fn leave(&mut self) {
        self.depth = self.depth.saturating_sub(1);
    }

    /// Calls a `fn` or `query` (RS 5.1). A query runs on a copy of its
    /// caller's stream (M 4.1).
    pub fn call_function(&mut self, full: &str, args: Vec<(Option<String>, Value)>, caller: &mut Env) -> RtResult<Value> {
        let (item, ns, file) = self.decl_of(full).ok_or_else(|| format!("no function `{full}`"))?;
        let ItemKind::Function { query, present, params, body, .. } = &item.kind else { return Err(format!("`{full}` is not a function")) };
        let kind = if *query {
            BodyKind::Query
        } else if *present {
            BodyKind::Present
        } else {
            BodyKind::Function
        };
        let mut env = Env::new(kind, full, ns.clone(), &file);
        env.self_ = caller.self_;
        env.holder = caller.holder;
        env.actor = caller.actor;
        env.map = caller.map;
        env.rng = match (kind, &caller.rng) {
            (BodyKind::Query, Some(r)) => Some(r.query_copy(&caller.name, full)),
            (BodyKind::Query, None) => Some(self.rng.query_copy(&caller.name, full)),
            _ => caller.rng.clone(),
        };
        self.enter(&env)?;
        self.bind(params, args, &mut env, ns.as_deref(), item.pos)?;
        let flow = self.run_to_end(body, &mut env);
        self.leave();
        // a plain fn shares its caller's stream
        if kind == BodyKind::Function && caller.rng.is_some() {
            caller.rng = env.rng.clone();
        }
        Ok(match flow? {
            Flow::Done(v) => v.unwrap_or(Value::None),
            _ => Value::None,
        })
    }

    /// Runs an ai for `me` and gives the command it chose, or `none`
    /// (RS 7.2).
    pub fn run_ai(&mut self, full: &str, me: EntityId) -> RtResult<Value> {
        self.run_ai_with(full, me, vec![])
    }

    /// `run_ai` with `it` bound: a follower's ai sees whom it follows
    /// (M 8.1).
    pub fn run_ai_with(&mut self, full: &str, me: EntityId, it: Vec<Value>) -> RtResult<Value> {
        let (item, ns, file) = self.decl_of(full).ok_or_else(|| format!("no ai `{full}`"))?;
        let ItemKind::Ai { body } = &item.kind else { return Err(format!("`{full}` is not an ai")) };
        let mut env = Env::new(BodyKind::Ai, full, ns, &file);
        env.self_ = Some(me);
        env.actor = Some(me);
        env.it = it;
        env.map = self.s.at(me).map(|p| p.map);
        self.enter(&env)?;
        let flow = self.run_to_end(body, &mut env);
        self.leave();
        Ok(match flow? {
            Flow::Done(v) => v.unwrap_or(Value::None),
            _ => Value::None,
        })
    }

    // ---- commands (RS 7.1, M 6.1.5) ----

    /// Starts a command for an actor. A continuation (`return ATTACK(x)`)
    /// runs in its place with its own cost and failure (M 6.1.5).
    pub fn start_command(&mut self, actor: EntityId, cmd: &CommandValue) -> RtResult<Outcome> {
        let (item, ns, file) = self.decl_of(&cmd.name).ok_or_else(|| format!("no command `{}`", cmd.name))?;
        let mut env = Env::new(BodyKind::Command, &cmd.name, ns.clone(), &file);
        env.actor = Some(actor);
        env.self_ = Some(actor);
        env.map = self.s.at(actor).map(|p| p.map);
        match &item.kind {
            ItemKind::CommandAlias(call) => {
                let v = self.eval(call, &mut env)?;
                match v {
                    Value::Command(next) => {
                        self.enter(&env)?;
                        let r = self.start_command(actor, &next);
                        self.leave();
                        r
                    }
                    _ => Err(format!("`{}` does not name a command", cmd.name)),
                }
            }
            ItemKind::Command { params, cost, body, .. } => {
                self.bind(params, cmd.args.clone(), &mut env, ns.as_deref(), item.pos)?;
                let state = Suspended { command: cmd.name.clone(), cost: cost.unwrap_or(DEFAULT_COST), frames: vec![Frame::Block { stmts: body.clone(), i: 0 }], env };
                self.continue_command(actor, state, false)
            }
            _ => Err(format!("`{}` is not a command", cmd.name)),
        }
    }

    /// Runs a command's frames on: from the start or after `wait`.
    pub fn continue_command(&mut self, actor: EntityId, mut state: Suspended, waited: bool) -> RtResult<Outcome> {
        let flow = self.run_frames(&mut state.frames, &mut state.env)?;
        match flow {
            Flow::Wait(n) => {
                let d = delay(n, self.speed(actor)).unwrap_or(n.max(1));
                Ok(Outcome::Waiting { delay: d, state: Box::new(state) })
            }
            Flow::Done(Some(Value::Bool(false))) => Ok(Outcome::Failed { waited }),
            Flow::Done(Some(Value::Command(next))) => {
                self.enter(&state.env)?;
                let r = self.start_command(actor, &next);
                self.leave();
                r
            }
            Flow::Done(_) => Ok(Outcome::Done { cost: state.cost }),
            Flow::Cancel => Err(format!("{}: runtime error: `cancel` in a command", state.env.file)),
        }
    }

    // ---- events (RS 4, M 6.2) ----

    fn bind_event_args(&mut self, event: &str, vals: Vec<(Option<String>, Value)>, env: &mut Env, pos: Pos) -> RtResult<Vec<Value>> {
        let (item, ns, _) = self.decl_of(event).ok_or_else(|| self.rt_err(env, pos, format!("no event `{event}`")))?;
        let ItemKind::Event { params } = &item.kind else { return Err(self.rt_err(env, pos, format!("`{event}` is not an event"))) };
        let mut tmp = Env::new(BodyKind::Function, event, ns.clone(), &env.file);
        tmp.rng = env.rng.clone();
        self.bind(params, vals, &mut tmp, ns.as_deref(), pos)?;
        Ok(params.iter().map(|p| tmp.locals.remove(&p.name).unwrap_or(Value::None)).collect())
    }

    /// Sends an event: game events by full name, engine events by their
    /// lower-case name. Handlers run before `emit` returns (M 6.2).
    pub fn emit(&mut self, event: &str, args: Vec<Value>, caller: &Env) -> RtResult<()> {
        self.emit_to(event, args, caller, Scope::All)
    }

    /// Sends an event to some of its handlers: `turn` goes to each
    /// instance's components and once to the general handlers (M 6.1.4).
    pub fn emit_to(&mut self, event: &str, mut args: Vec<Value>, caller: &Env, scope: Scope) -> RtResult<()> {
        let engine = names::is_vocabulary(event);
        let subject = args.first().and_then(Value::as_entity);
        if let Some(s) = subject
            && !self.s.get(s).alive
        {
            // a dead subject gets no event (RS 4)
            return Ok(());
        }
        self.enter(caller)?;
        let phases: &[Phase] = if engine { &[Phase::On, Phase::After] } else { &[Phase::Before, Phase::On, Phase::After] };
        for &phase in phases {
            for h in self.handlers(event, phase, subject).into_iter().filter(|h| scope.takes(h.component.is_some())) {
                // a component removed before its turn is skipped (RS 4)
                if let Some(c) = &h.component
                    && !subject.is_some_and(|s| self.s.get(s).has(c))
                {
                    continue;
                }
                let handler = h.handler.clone();
                let mut env = Env::new(BodyKind::Handler, &h.name, h.ns.clone(), &h.file);
                let offset = if h.component.is_some() { 1 } else { 0 };
                if h.component.is_some() {
                    env.self_ = subject;
                }
                env.map = match (event, args.get(1)) {
                    ("arrive" | "depart", Some(Value::Name(m))) => self.s.map_id(m),
                    _ => subject.and_then(|s| self.s.at(s)).map(|p| p.map).or(caller.map),
                };
                if event == "begin" {
                    env.map = self.s.game.start.as_deref().and_then(|s| self.s.map_id(s));
                }
                for (i, n) in handler.names.iter().enumerate() {
                    env.locals.insert(n.clone(), args.get(i + offset).cloned().unwrap_or(Value::None));
                }
                if let Some(w) = &handler.when
                    && !self.truth(w, &mut env)?
                {
                    continue;
                }
                let flow = self.run_to_end(&handler.body, &mut env)?;
                if phase == Phase::Before {
                    // `before` may change the parameters it names (RS 4)
                    for (i, n) in handler.names.iter().enumerate() {
                        if let Some(v) = env.locals.get(n)
                            && i + offset < args.len()
                        {
                            args[i + offset] = v.clone();
                        }
                    }
                    if flow == Flow::Cancel {
                        self.leave();
                        return Ok(());
                    }
                }
            }
        }
        self.leave();
        Ok(())
    }

    /// The handlers of an event's phase in order: priority, largest first,
    /// then load order (RS 4).
    fn handlers(&self, event: &str, phase: Phase, subject: Option<EntityId>) -> Vec<HandlerRef> {
        let mut out: Vec<(i64, usize, usize, HandlerRef)> = Vec::new();
        for (order, d) in self.m.code.iter().enumerate() {
            match &d.item.kind {
                ItemKind::Handler(h) if h.phase == phase && self.event_name(&h.event, d.namespace.as_deref()) == event => {
                    // a named handler runs only as the definition that won:
                    // `override on … as NAME` replaces it (RS 4, D 4.5)
                    if !d.name.is_empty()
                        && let Some(def) = self.m.defs.get(&names::qualify(d.namespace.as_deref(), &d.name))
                        && (def.file != d.file || def.line != d.line)
                    {
                        continue;
                    }
                    out.push((h.priority, order, 0, HandlerRef { handler: Arc::new(h.clone()), component: None, name: d.name.clone(), ns: d.namespace.clone(), file: d.file.clone() }));
                }
                ItemKind::Component { handlers, .. } => {
                    let Some(s) = subject else { continue };
                    let full = names::qualify(d.namespace.as_deref(), &d.name);
                    if !self.s.get(s).has(&full) {
                        continue;
                    }
                    for (k, h) in handlers.iter().enumerate() {
                        if h.phase == phase && self.event_name(&h.event, d.namespace.as_deref()) == event {
                            out.push((h.priority, order, k, HandlerRef { handler: Arc::new(h.clone()), component: Some(full.clone()), name: full.clone(), ns: d.namespace.clone(), file: d.file.clone() }));
                        }
                    }
                }
                _ => {}
            }
        }
        out.sort_by_key(|(p, o, k, _)| (-p, *o, *k));
        out.into_iter().map(|(_, _, _, h)| h).collect()
    }

    fn event_name(&self, written: &str, ns: Option<&str>) -> String {
        if names::is_vocabulary(written) {
            return written.to_string();
        }
        self.m.resolve(ns, written).map(|d| d.full.clone()).unwrap_or_else(|_| written.to_string())
    }

    /// Sends `created` for instances that wait for it, in order (M 3.2).
    pub fn flush_created(&mut self, caller: &Env) -> RtResult<()> {
        while !self.created.is_empty() {
            let e = self.created.remove(0);
            self.emit("created", vec![Value::Entity(e)], caller)?;
        }
        Ok(())
    }

    /// `control(e, seat:)` (M 6.1.10): a member becomes its seat's
    /// controlled member; another instance takes the player's place, or
    /// joins seat `n`, with the old controlled member staying a member.
    pub fn control(&mut self, e: EntityId, n: Option<SeatId>) {
        let n = match n {
            Some(n) => n,
            None => match self.s.seat_of(e) {
                Some(n) => n,
                None => {
                    // `e` replaces the player, who leaves the seat (body swap)
                    let n = self.s.player.and_then(|p| self.s.seat_of(p)).or_else(|| self.s.seats.first().map(|s| s.n)).unwrap_or(1);
                    if let Some(old) = self.s.player.filter(|&p| self.s.seat_of(p) == Some(n)) {
                        self.s.set_seat(old, None);
                    }
                    n
                }
            },
        };
        self.s.join_seat(n);
        self.s.set_seat(e, Some(n));
        self.s.set_active(n, e);
        self.s.player = Some(e);
    }

    /// `begin` after the new game is computed (M 5.1 step 8).
    pub fn begin(&mut self) -> RtResult<()> {
        let env = Env::new(BodyKind::Handler, "begin", None, "<engine>");
        self.flush_created(&env)?;
        self.emit("begin", vec![], &env)?;
        self.flush_created(&env)?;
        // the members made by `begin` act at 0 (M 5.1), also when they are
        // actors that got their spawn wake first (an open question, M 11)
        for p in self.s.players() {
            if let Some(at) = self.s.at(p) {
                let now = self.s.maps[at.map].clock(self.s.game.time);
                if self.s.wake_of(p).is_some_and(|w| w.at > now) {
                    self.s.schedule(p, now);
                }
            }
        }
        Ok(())
    }

    // ---- the engine's functions (M 9.3) ----

    fn arg<'v>(&self, args: &'v [(Option<String>, Value)], i: usize, name: &str) -> Option<&'v Value> {
        args.iter().find(|(n, _)| n.as_deref() == Some(name)).map(|(_, v)| v).or_else(|| args.iter().filter(|(n, _)| n.is_none()).nth(i).map(|(_, v)| v))
    }

    fn entity_arg(&self, v: Option<&Value>, f: &str, env: &Env, pos: Pos) -> RtResult<EntityId> {
        v.and_then(Value::as_entity).ok_or_else(|| self.rt_err(env, pos, format!("`{f}` takes an entity")))
    }

    /// One cell of an area, drawn from the caller's stream (M 4.3).
    fn one_cell(&mut self, a: &Area, env: &mut Env) -> Option<Cell> {
        let i = self.rng_for(env).choose(a.len())?;
        Some(a.cells()[i])
    }

    fn engine(&mut self, f: &str, args: Vec<(Option<String>, Value)>, env: &mut Env, pos: Pos) -> RtResult<Value> {
        let a0 = args.first().map(|(_, v)| v.clone()).unwrap_or(Value::None);
        Ok(match f {
            "entities" => {
                let area = match self.arg(&args, 0, "in") {
                    Some(v) => self.area(v, env, pos)?,
                    None => self.tiles(env),
                };
                let mut out = Vec::new();
                for e in 0..self.s.entities.len() {
                    if self.s.get(e).alive
                        && let Place::Map(p) = self.s.get(e).place
                        && area.contains(p)
                    {
                        out.push(Value::Entity(e));
                    }
                }
                Value::Collection(out)
            }
            "tiles" => Value::Area(self.tiles(env)),
            "pos" => {
                let x = self.number(a0, env, pos)?;
                let y = self.number(self.arg(&args, 1, "y").cloned().unwrap_or(Value::None), env, pos)?;
                Value::Pos(Cell { map: env.map.ok_or_else(|| self.rt_err(env, pos, "`pos` needs a map"))?, x: x as i32, y: y as i32 })
            }
            "label" => {
                let Value::Name(n) = a0 else { return Err(self.rt_err(env, pos, "`label` takes a label")) };
                let Some(m) = env.map else { return Ok(Value::Area(Area::default())) };
                let mut cells: Vec<Cell> = self.s.maps[m].label(&n).iter().map(|(x, y)| Cell { map: m, x: *x, y: *y }).collect();
                for e in self.s.entities_on(m) {
                    if self.s.get(e).get("label").and_then(|c| c.get("name")) == Some(&Value::Name(n.clone())) {
                        cells.extend(self.s.at(e));
                    }
                }
                Value::Area(Area::new(cells))
            }
            "at" => {
                let Value::Name(n) = a0 else { return Err(self.rt_err(env, pos, "`at` takes a tile or tileset")) };
                let Some(m) = env.map else { return Ok(Value::Area(Area::default())) };
                let all = self.tiles(env);
                Value::Area(Area::new(all.cells().iter().copied().filter(|p| self.cell_has(*p, &n)).collect::<Vec<_>>()).and(&Area::new(all.cells().iter().copied().filter(|p| p.map == m).collect())))
            }
            "near" => {
                let a = self.area(&a0, env, pos)?;
                let (lo, hi) = match self.arg(&args, 1, "d") {
                    Some(Value::Number(n)) => (0, Some(*n)),
                    Some(Value::Range(a, b)) => (*a, *b),
                    _ => (0, Some(1)),
                };
                let Some(first) = a.cells().first() else { return Ok(Value::Area(Area::default())) };
                let m = &self.s.maps[first.map];
                let mut cells = Vec::new();
                for y in 0..m.height {
                    for x in 0..m.width {
                        let p = Cell { map: first.map, x, y };
                        let d = a.cells().iter().map(|c| c.distance(p)).min().unwrap_or(i64::MAX);
                        if d >= lo && hi.is_none_or(|h| d <= h) {
                            cells.push(p);
                        }
                    }
                }
                Value::Area(Area::new(cells))
            }
            "distance" => {
                let (x, y) = (self.area(&a0, env, pos)?, self.area(self.arg(&args, 1, "b").unwrap_or(&Value::None), env, pos)?);
                let d = x.cells().iter().flat_map(|p| y.cells().iter().map(move |q| p.distance(*q))).min().unwrap_or(i64::MAX);
                Value::Number(d)
            }
            "nearest" => {
                let to = self.area(self.arg(&args, 1, "a").unwrap_or(&Value::None), env, pos)?;
                let items = a0.members().ok_or_else(|| self.rt_err(env, pos, "`nearest` takes a set"))?;
                let mut best: Option<(i64, Value)> = None;
                for it in items {
                    let ia = self.area(&it, env, pos)?;
                    let d = ia.cells().iter().flat_map(|p| to.cells().iter().map(move |q| p.distance(*q))).min().unwrap_or(i64::MAX);
                    if best.as_ref().is_none_or(|(bd, _)| d < *bd) {
                        best = Some((d, it));
                    }
                }
                best.map(|(_, v)| v).unwrap_or(Value::None)
            }
            "min" | "max" => {
                let x = self.number(a0, env, pos)?;
                let y = self.number(self.arg(&args, 1, "b").cloned().unwrap_or(Value::None), env, pos)?;
                Value::Number(if f == "min" { x.min(y) } else { x.max(y) })
            }
            "choose" => match a0 {
                Value::Range(a, Some(b)) => self.rng_for(env).range(a, b).map(Value::Number).unwrap_or(Value::None),
                v => {
                    let items = v.members().ok_or_else(|| self.rt_err(env, pos, "`choose` takes a set"))?;
                    match self.rng_for(env).choose(items.len()) {
                        Some(i) => items[i].clone(),
                        None => Value::None,
                    }
                }
            },
            "chance" => {
                let p = match a0 {
                    Value::Percent(p) => p,
                    v => self.number(v, env, pos)?,
                };
                Value::Bool(self.rng_for(env).chance(p))
            }
            "pick" | "shuffled" => {
                let items = a0.members().ok_or_else(|| self.rt_err(env, pos, format!("`{f}` takes a set")))?;
                let n = if f == "pick" { self.number(self.arg(&args, 1, "n").cloned().unwrap_or(Value::Number(1)), env, pos)? as usize } else { items.len() };
                let out = self.rng_for(env).pick(&items, n);
                match a0 {
                    Value::Collection(_) => Value::Collection(out),
                    _ => Value::List(out),
                }
            }
            "adjacent" => {
                let e = self.entity_arg(Some(&a0), f, env, pos)?;
                Value::Area(self.adjacent(e))
            }
            "blocks" => {
                let a = self.area(&a0, env, pos)?;
                let what = match self.arg(&args, 1, "what") {
                    Some(Value::Name(n)) => n.clone(),
                    _ => "move".into(),
                };
                let who = self.arg(&args, 2, "for").and_then(Value::as_entity);
                let tiles = !matches!(self.arg(&args, 3, "tiles"), Some(Value::Bool(false)));
                // `except` leaves one more entity out: a swap's own mover (M 9.3)
                let except = self.arg(&args, 4, "except").and_then(Value::as_entity);
                Value::Bool(a.cells().iter().any(|p| match except {
                    None => self.blocks(*p, &what, who, tiles),
                    Some(x) => (tiles && self.blocks_tiles(*p, &what, who)) || self.s.entities_at(*p).into_iter().any(|e| e != x && Some(e) != who && self.entity_blocks(e, &what)),
                }))
            }
            "tile_of" => {
                let Value::Pos(p) = a0 else { return Err(self.rt_err(env, pos, "`tile_of` takes a pos")) };
                self.s.maps[p.map].tile(0, p.x, p.y).map(|t| Value::Name(t.to_string())).unwrap_or(Value::None)
            }
            "line" => {
                let (x, y) = (self.area(&a0, env, pos)?, self.area(self.arg(&args, 1, "b").unwrap_or(&Value::None), env, pos)?);
                match (x.cells().first(), y.cells().first()) {
                    // a line keeps its order (M 9.3)
                    (Some(p), Some(q)) => Value::List(line(*p, *q).into_iter().map(Value::Pos).collect()),
                    _ => Value::List(vec![]),
                }
            }
            "move" => self.engine_move(&args, env, pos)?,
            "noise" => {
                let p = self.area(&a0, env, pos)?.cells().first().copied().ok_or_else(|| self.rt_err(env, pos, "`noise` takes a pos"))?;
                let kind = match self.arg(&args, 1, "kind") {
                    Some(Value::Name(n)) => n.clone(),
                    _ => return Err(self.rt_err(env, pos, "`noise` takes a sound kind")),
                };
                let source = self.arg(&args, 2, "source").and_then(Value::as_entity);
                self.sounds.push(Noise { at: p, kind, source, time: self.s.game.time });
                self.sounds_made += 1;
                Value::None
            }
            "spawn" => self.engine_spawn(&args, env, pos)?,
            "destroy" => {
                let e = self.entity_arg(Some(&a0), f, env, pos)?;
                self.destroy(e, env)?;
                Value::None
            }
            "add" => {
                let e = self.entity_arg(Some(&a0), f, env, pos)?;
                let Some(Value::Component(cv)) = self.arg(&args, 1, "k").cloned() else { return Err(self.rt_err(env, pos, "`add` takes a component value")) };
                let full = self.complete(cv, env.ns.as_deref())?;
                let name = full.name.clone();
                if name == "seat" {
                    // membership goes through the seat table (M 6.1.10)
                    let n = full.get("n").and_then(crate::rt::state::State::seat_number).filter(|&n| self.s.seat(n).is_some());
                    let n = n.ok_or_else(|| self.rt_err(env, pos, "`add`: the seat is not joined (M 6.1.10)"))?;
                    self.s.set_seat(e, Some(n));
                } else {
                    self.s.get_mut(e).set(full);
                }
                if (name == "ai" || name == "seat")
                    && let Some(p) = self.s.at(e)
                    && self.s.wake_of(e).is_none()
                    && matches!(self.s.get(e).place, Place::Map(_))
                {
                    self.wake_new(e, p.map);
                }
                Value::Entity(e)
            }
            "remove" => {
                let e = self.entity_arg(Some(&a0), f, env, pos)?;
                let comp = match self.arg(&args, 1, "k") {
                    Some(Value::Name(n)) => n.clone(),
                    _ => return Err(self.rt_err(env, pos, "`remove` takes a component name")),
                };
                if comp == "seat" {
                    self.s.set_seat(e, None);
                } else {
                    self.s.get_mut(e).remove(&comp);
                }
                // an instance that stops being an actor leaves the queue, the
                // player too: `remove(player, seat)` ends its turns (M 9.5,
                // M 6.1.10)
                if (comp == "ai" || comp == "seat") && !self.s.is_actor(e) {
                    self.s.unschedule(e);
                }
                Value::Entity(e)
            }
            "control" => {
                let e = self.entity_arg(Some(&a0), f, env, pos)?;
                let n = match self.arg(&args, 1, "seat") {
                    Some(v @ Value::Number(_)) => {
                        let n = crate::rt::state::State::seat_number(v).filter(|&n| self.s.seat(n).is_some());
                        Some(n.ok_or_else(|| self.rt_err(env, pos, "`control`: the seat is not joined (M 6.1.10)"))?)
                    }
                    Some(Value::None) | None => None,
                    Some(_) => return Err(self.rt_err(env, pos, "`control` takes a seat number")),
                };
                self.control(e, n);
                if let Some(p) = self.s.at(e) {
                    self.s.last_player_map = Some(p.map);
                    if self.s.wake_of(e).is_none() {
                        let now = self.s.maps[p.map].clock(self.s.game.time);
                        self.s.schedule(e, now);
                    }
                }
                Value::Entity(e)
            }
            "travel" => {
                let e = self.entity_arg(Some(&a0), f, env, pos)?;
                let map = match self.arg(&args, 1, "map") {
                    Some(Value::Name(n)) => n.clone(),
                    _ => return Err(self.rt_err(env, pos, "`travel` takes a map")),
                };
                let at = match self.arg(&args, 2, "at") {
                    Some(Value::Name(n)) => n.clone(),
                    _ => "START".into(),
                };
                let align = matches!(self.arg(&args, 3, "align"), Some(Value::Bool(true)));
                // the target map loads at the call, and its label is checked (M 6.7)
                let id = self.load_map(&map)?;
                self.flush_created(env)?;
                if self.s.maps[id].label(&at).is_empty() {
                    return Err(self.rt_err(env, pos, format!("map `{map}` has no label `{at}` (M 6.7)")));
                }
                self.travels.retain(|t| t.who != e);
                self.travels.push(Travel { who: e, map, at, align });
                Value::Entity(e)
            }
            "log" => {
                let Value::Name(key) = a0 else { return Err(self.rt_err(env, pos, "`log` takes a message key")) };
                let named = args.iter().filter_map(|(n, v)| n.clone().map(|n| (n, v.clone()))).collect();
                self.log.push(LogEntry { key, args: named });
                Value::None
            }
            // presentation without a screen does nothing (M 9.3)
            "fly" | "play" | "pan" | "shake" | "fade" | "reveal" => Value::None,
            "sees" => {
                let b = self.area(self.arg(&args, 1, "b").unwrap_or(&Value::None), env, pos)?;
                // a collection sees when one of its members does (M 9.3)
                let who: Vec<EntityId> = match &a0 {
                    Value::Collection(v) => v.iter().filter_map(Value::as_entity).collect(),
                    _ => vec![self.entity_arg(Some(&a0), f, env, pos)?],
                };
                Value::Bool(who.into_iter().any(|a| self.sees(a, b.cells())))
            }
            "in_view" => {
                let b = self.area(&a0, env, pos)?;
                Value::Bool(self.in_view(b.cells()))
            }
            "seats" => Value::List(self.s.seats.iter().map(|s| Value::Number(s.n as i64)).collect()),
            "seat_of" => {
                let e = a0.as_entity();
                e.and_then(|e| self.s.seat_of(e)).map(|n| Value::Number(n as i64)).unwrap_or(Value::None)
            }
            "members" | "active" => {
                let n = a0.as_number().and_then(|n| SeatId::try_from(n).ok());
                match (f, n) {
                    ("members", Some(n)) => Value::Collection(self.s.seat_members(n).into_iter().map(Value::Entity).collect()),
                    ("members", None) => Value::Collection(vec![]),
                    (_, n) => n.and_then(|n| self.s.active(n)).map(Value::Entity).unwrap_or(Value::None),
                }
            }
            "controlled" => Value::Bool(a0.as_entity().is_some_and(|e| self.s.controlled(e))),
            "view_width" | "view_height" => {
                let size = self.view_dims();
                Value::Number(if f == "view_width" { size.0 } else { size.1 } as i64)
            }
            "heard" => {
                let a = self.entity_arg(Some(&a0), f, env, pos)?;
                Value::Collection(self.heard.get(&a).cloned().unwrap_or_default().into_iter().map(Value::Sound).collect())
            }
            "hears" => {
                let a = self.entity_arg(Some(&a0), f, env, pos)?;
                let b = self.arg(&args, 1, "b").and_then(Value::as_entity);
                Value::Bool(self.heard.get(&a).is_some_and(|hs| hs.iter().any(|s| s.source.is_some() && s.source == b)))
            }
            "path" => {
                let from = self.area(&a0, env, pos)?;
                let to = self.area(self.arg(&args, 1, "b").unwrap_or(&Value::None), env, pos)?;
                let who = self.arg(&args, 2, "for").and_then(Value::as_entity);
                match from.cells().first() {
                    Some(p) => Value::List(self.route(*p, to.cells(), who).into_iter().map(Value::Pos).collect()),
                    None => Value::List(vec![]),
                }
            }
            "ray" => {
                let a = self.area(&a0, env, pos)?.cells().first().copied();
                let b = self.area(self.arg(&args, 1, "b").unwrap_or(&Value::None), env, pos)?.cells().first().copied();
                let who = self.arg(&args, 2, "for").and_then(Value::as_entity);
                let range = match self.arg(&args, 3, "range").cloned() {
                    Some(v) => self.number(v, env, pos)?,
                    None => 0,
                };
                let mode = match self.arg(&args, 4, "mode") {
                    Some(Value::Name(n)) => Some(n.clone()),
                    _ => None,
                };
                match (a, b) {
                    (Some(a), Some(b)) => Value::List(self.ray(a, b, who, range, mode.as_deref()).into_iter().map(Value::Pos).collect()),
                    _ => Value::List(vec![]),
                }
            }
            // done at the action's end (M 5.3, M 5.5, M 6.1.7)
            "checkpoint" => {
                self.saves.point = Some(super::world::Point::Checkpoint);
                Value::None
            }
            "rollback" => {
                self.saves.point = Some(super::world::Point::Rollback);
                Value::None
            }
            "autosave" => {
                self.saves.autosave = true;
                Value::None
            }
            "delete_save" => {
                match &a0 {
                    Value::Name(n) if n == "checkpoint" => self.saves.delete_checkpoint = true,
                    Value::Name(n) if n == "auto" => self.saves.delete_auto = true,
                    Value::All => {
                        self.saves.delete_checkpoint = true;
                        self.saves.delete_auto = true;
                    }
                    other => return Err(self.rt_err(env, pos, format!("`delete_save` takes `checkpoint`, `auto` or `all`, not `{other}` (M 5.5)"))),
                }
                Value::None
            }
            "text_of" => {
                // a key's text in the chosen language with its named
                // arguments as placeholders; a name, number or entity
                // shows as its placeholder would (RS 2.6, M 7.3)
                let lang = self.m.settings().get("language").map(|i| i.rest(1).to_string()).unwrap_or_default();
                let named: Vec<(String, Value)> = args.iter().skip(1).filter_map(|(n, v)| n.clone().map(|n| (n, v.clone()))).collect();
                Value::Text(match &a0 {
                    Value::Name(key) if self.m.has_text(&lang, key) => self.text(&lang, key, &named),
                    Value::None => String::new(),
                    other => self.name_text(&lang, other),
                })
            }
            other => return Err(self.rt_err(env, pos, format!("engine function `{other}` is not implemented yet"))),
        })
    }

    fn tiles(&self, env: &Env) -> Area {
        let Some(m) = env.map else { return Area::default() };
        let map = &self.s.maps[m];
        Area::new((0..map.height).flat_map(|y| (0..map.width).map(move |x| Cell { map: m, x, y })).collect())
    }

    /// Fills a component value's missing fields with defaults or zeros and
    /// rolls its number fields (M 9.5, M 3.2 steps 3–4).
    fn complete(&mut self, mut cv: ComponentValue, ns: Option<&str>) -> RtResult<ComponentValue> {
        let defs = if names::is_vocabulary(&cv.name) { crate::fields::engine_component(&cv.name).unwrap_or_default() } else { self.m.defs.get(&cv.name).map(crate::fields::component_fields).unwrap_or_default() };
        for f in defs {
            if cv.get(&f.name).is_none() {
                let ty = if names::is_vocabulary(&cv.name) {
                    crate::rs::sigs::engine_component_field(&cv.name, &f.name).unwrap_or(Ty::Unknown)
                } else {
                    types::parse(&f.ty, &|n| self.m.resolve(ns, n).is_ok_and(|d| d.kind == Kind::Component)).unwrap_or(Ty::Unknown)
                };
                let v = match &f.default {
                    Some(d) if cv.get(d).is_some() => cv.get(d).cloned().unwrap(),
                    Some(d) => self.data_value(&ty, d, ns)?,
                    None => match ty {
                        Ty::Number => Value::Number(0),
                        Ty::Bool => Value::Bool(false),
                        Ty::Dict(..) => Value::Dict(vec![]),
                        Ty::List(_) => Value::List(vec![]),
                        _ => Value::None,
                    },
                };
                cv.set(&f.name, v);
            }
        }
        Ok(cv)
    }

    /// The cells `e` can step to: its directions and the corner rule, not
    /// the target's own blocking (M 9.3).
    pub fn adjacent(&self, e: EntityId) -> Area {
        let Some(at) = self.s.at(e) else { return Area::default() };
        let mv = self.s.get(e).get("movement");
        let dirs = mv.and_then(|m| m.get("dirs")).and_then(Value::as_number).unwrap_or(8);
        let corners = matches!(mv.and_then(|m| m.get("corners")), Some(Value::Bool(true)));
        let map = &self.s.maps[at.map];
        let mut out = Vec::new();
        for d in Dir::ALL {
            let (dx, dy) = d.delta();
            let diagonal = dx != 0 && dy != 0;
            if diagonal && dirs == 4 {
                continue;
            }
            let p = at.step(d);
            if !map.inside(p.x, p.y) {
                continue;
            }
            if diagonal && !corners {
                let a = Cell { x: at.x + dx, ..at };
                let b = Cell { y: at.y + dy, ..at };
                if self.blocks(a, "move", Some(e), true) || self.blocks(b, "move", Some(e), true) {
                    continue;
                }
            }
            out.push(p);
        }
        Area::new(out)
    }

    /// Does a cell block `what` (`move`, `sight`, `sound` or a movement
    /// mode)? Tiles by their property ladder, entities by their `blocks`;
    /// `who` leaves itself out and brings its movement mode (M 9.3, D 7.5).
    pub fn blocks(&self, p: Cell, what: &str, who: Option<EntityId>, tiles: bool) -> bool {
        (tiles && self.blocks_tiles(p, what, who)) || self.blocks_entities(p, what, who)
    }

    /// The tiles' part of `blocks`; outside the map everything blocks.
    pub fn blocks_tiles(&self, p: Cell, what: &str, who: Option<EntityId>) -> bool {
        let m = &self.s.maps[p.map];
        if !m.inside(p.x, p.y) {
            return true;
        }
        let mode = who.and_then(|w| self.s.get(w).get("movement")).and_then(|c| c.get("mode")).and_then(|v| if let Value::Name(n) = v { Some(n.clone()) } else { None });
        for l in 0..m.layers.len() {
            let Some(t) = m.tile(l, p.x, p.y) else { continue };
            let blocks = self.tile_property(t, "blocks");
            if blocks.iter().any(|b| b == what) {
                return true;
            }
            if what == "move"
                && let Some(mode) = &mode
            {
                if blocks.iter().any(|b| b == mode) {
                    return true;
                }
                let only = self.tile_property(t, "only");
                if !only.is_empty() && !only.contains(mode) {
                    return true;
                }
            }
        }
        false
    }

    /// The entities' part of `blocks`: their own `blocks` component.
    pub fn blocks_entities(&self, p: Cell, what: &str, who: Option<EntityId>) -> bool {
        self.s.entities_at(p).into_iter().any(|e| Some(e) != who && self.entity_blocks(e, what))
    }

    /// Does an entity's own `blocks` component hold `what`?
    pub fn entity_blocks(&self, e: EntityId, what: &str) -> bool {
        matches!(self.s.get(e).get("blocks").and_then(|c| c.get("values")), Some(Value::List(vs)) if vs.iter().any(|v| *v == Value::Name(what.to_string())))
    }

    /// A tile property's values down the ladder: the exact name, index
    /// ranges, the base name, then `default` (D 7.1, D 7.5).
    pub fn tile_property(&self, tile: &str, prop: &str) -> Vec<String> {
        let mut ladder = vec![tile.to_string()];
        let mut t = tile.to_string();
        while let Some((head, _)) = t.rsplit_once('.') {
            ladder.push(head.to_string());
            t = head.to_string();
        }
        ladder.push("default".into());
        let rows = self.m.tile_rows();
        for key in ladder {
            let mut found: Option<Vec<String>> = None;
            for words in rows.get(&key).into_iter().flatten() {
                if let Some(i) = words.iter().position(|w| *w == prop) {
                    let end = (i + 1..words.len()).find(|&j| ["blocks", "only", "lit", "forget"].contains(&words[j].as_str())).unwrap_or(words.len());
                    found = Some(words[i + 1..end].iter().filter(|w| *w != "none").cloned().collect());
                }
            }
            if let Some(v) = found {
                return v;
            }
        }
        vec![]
    }

    fn engine_move(&mut self, args: &[(Option<String>, Value)], env: &mut Env, pos: Pos) -> RtResult<Value> {
        let e = self.entity_arg(args.first().map(|(_, v)| v), "move", env, pos)?;
        let to = self.arg(args, 1, "to").cloned().unwrap_or(Value::None);
        let prev = self.s.at(e);
        let was_on_map = matches!(self.s.get(e).place, Place::Map(_));
        // to a collection (D 5.4)
        if let Value::List(l) = &to
            && l.first() == Some(&Value::Text("@collection".into()))
        {
            let (Value::Entity(h), Value::Name(comp), Value::Name(field)) = (&l[1], &l[2], &l[3]) else { return Ok(Value::Bool(false)) };
            let slot = self.s.free_slot(*h, comp, field);
            self.s.get_mut(e).place = Place::In { holder: *h, component: comp.clone(), field: field.clone(), slot };
            self.s.unschedule(e);
            if let (true, Some(p)) = (was_on_map, prev) {
                let name = self.s.maps[p.map].name.clone();
                self.emit("leave", vec![Value::Entity(e), Value::Pos(p)], env)?;
                self.emit("depart", vec![Value::Entity(e), Value::Name(name), Value::Pos(p)], env)?;
            }
            return Ok(Value::Bool(true));
        }
        let area = self.area(&to, env, pos)?;
        let Some(p) = self.one_cell(&area, env) else { return Ok(Value::Bool(false)) };
        if was_on_map && prev.is_some_and(|q| q.map != p.map) {
            return Err(self.rt_err(env, pos, "`move` stays on one map; maps change with `travel` (M 6.2)"));
        }
        self.s.get_mut(e).place = Place::Map(p);
        match (was_on_map, prev) {
            (true, Some(q)) => {
                self.emit("leave", vec![Value::Entity(e), Value::Pos(q)], env)?;
                self.emit("enter", vec![Value::Entity(e), Value::Pos(p), Value::Pos(q)], env)?;
            }
            _ => {
                if self.s.is_actor(e) {
                    self.wake_new(e, p.map);
                }
                let name = self.s.maps[p.map].name.clone();
                self.emit("arrive", vec![Value::Entity(e), Value::Name(name), Value::Pos(p)], env)?;
                self.emit("enter", vec![Value::Entity(e), Value::Pos(p), Value::None], env)?;
            }
        }
        Ok(Value::Bool(true))
    }

    fn engine_spawn(&mut self, args: &[(Option<String>, Value)], env: &mut Env, pos: Pos) -> RtResult<Value> {
        let x = args.first().map(|(_, v)| v.clone()).unwrap_or(Value::None);
        let card = match &x {
            Value::Card(c) => c.clone(),
            Value::Name(n) | Value::Deck(n) | Value::Table(n) => Card { entity: Some(n.clone()), count: None, overrides: vec![] },
            Value::None => return Ok(Value::None),
            v => return Err(self.rt_err(env, pos, format!("`spawn` takes a name, card, deck or table, not a {}", v.type_name()))),
        };
        let place: Box<dyn Fn(&super::state::State) -> Place> = if let Some(Value::List(l)) = self.arg(args, 1, "into").cloned()
            && l.first() == Some(&Value::Text("@collection".into()))
        {
            let (Value::Entity(h), Value::Name(comp), Value::Name(field)) = (l[1].clone(), l[2].clone(), l[3].clone()) else { return Ok(Value::None) };
            Box::new(move |s: &super::state::State| Place::In { holder: h, component: comp.clone(), field: field.clone(), slot: s.free_slot(h, &comp, &field) })
        } else {
            let at = self.arg(args, 1, "at").cloned().unwrap_or(Value::None);
            let area = self.area(&at, env, pos)?;
            let Some(p) = self.one_cell(&area, env) else { return Ok(if matches!(x, Value::Name(_)) { Value::None } else { Value::Collection(vec![]) }) };
            Box::new(move |_| Place::Map(p))
        };
        let made = self.spawn_card(&card, &*place)?;
        self.flush_created(env)?;
        // a name spawns one entity; a card, deck or table a collection (M 9.4)
        Ok(match (&x, self.m.kind(card.entity.as_deref().unwrap_or(""))) {
            (Value::Name(_), Some(Kind::Entity)) => made.first().map(|e| Value::Entity(*e)).unwrap_or(Value::None),
            _ => Value::Collection(made.into_iter().map(Value::Entity).collect()),
        })
    }

    /// `destroy`: `removed` first, then the members, holder before members
    /// (M 9.5).
    pub fn destroy(&mut self, e: EntityId, env: &Env) -> RtResult<()> {
        if !self.s.get(e).alive {
            return Ok(());
        }
        self.emit("removed", vec![Value::Entity(e)], env)?;
        let members: Vec<EntityId> = (0..self.s.entities.len()).filter(|&m| matches!(&self.s.get(m).place, Place::In { holder, .. } if *holder == e) && self.s.get(m).alive).collect();
        self.s.get_mut(e).alive = false;
        self.s.unschedule(e);
        for m in members {
            self.destroy(m, env)?;
        }
        Ok(())
    }

    /// Does the action's end work: travels in call order (M 6.1.7, M 6.7).
    pub fn end_action(&mut self) -> RtResult<()> {
        let env = Env::new(BodyKind::Handler, "<action end>", None, "<engine>");
        let travels = std::mem::take(&mut self.travels);
        for t in travels {
            let Some(id) = self.s.map_id(&t.map) else { continue };
            let from = self.s.at(t.who);
            let cells: Vec<Cell> = self.s.maps[id].label(&t.at).iter().map(|(x, y)| Cell { map: id, x: *x, y: *y }).collect();
            let i = if t.align {
                // the same index as the start cell has in its own group
                let idx = from.and_then(|p| self.s.maps[p.map].labels.iter().find_map(|(_, cs)| cs.iter().position(|c| *c == (p.x, p.y))));
                match idx {
                    Some(k) => k.min(cells.len() - 1),
                    None => self.rng.below(cells.len() as u64) as usize,
                }
            } else {
                self.rng.below(cells.len() as u64) as usize
            };
            let to = cells[i];
            // the wake moves to the target map's clock with its wait kept (M 6.1.8)
            let wake = self.s.wake_of(t.who);
            let left = match (wake, from) {
                (Some(w), Some(p)) => Some(w.at - self.s.maps[p.map].clock(self.s.game.time)),
                _ => None,
            };
            self.s.get_mut(t.who).place = Place::Map(to);
            if let Some(left) = left {
                let at = self.s.maps[id].clock(self.s.game.time) + left;
                self.s.restore(super::state::Wake { at, ..wake.unwrap() });
            }
            if self.s.player == Some(t.who) {
                // the active map changes at the microstep's end (M 6.7)
                if let Some(p) = from
                    && p.map != id
                {
                    self.switch = Some((p.map, id));
                }
                self.s.last_player_map = Some(id);
            }
            if let Some(p) = from {
                let name = self.s.maps[p.map].name.clone();
                self.emit("leave", vec![Value::Entity(t.who), Value::Pos(p)], &env)?;
                self.emit("depart", vec![Value::Entity(t.who), Value::Name(name), Value::Pos(p)], &env)?;
            }
            let name = self.s.maps[id].name.clone();
            self.emit("arrive", vec![Value::Entity(t.who), Value::Name(name), Value::Pos(to)], &env)?;
        }
        Ok(())
    }

    /// Freezes the map the player left and thaws the one it came to, at
    /// the microstep's end (M 6.1.8, M 6.7).
    pub fn apply_switch(&mut self) {
        if let Some((from, to)) = self.switch.take() {
            let now = self.s.game.time;
            self.s.maps[from].freeze(now);
            self.s.maps[to].thaw(now);
        }
    }

    // ---- texts (D 10, M 7.3) ----

    /// A text key's text in a language, with placeholders filled; a
    /// missing text shows as its key (M 7.3).
    pub fn text(&self, lang: &str, key: &str, args: &[(String, Value)]) -> String {
        self.text_runs(lang, key, args).into_iter().map(|(t, _)| t).collect()
    }

    /// A text key's text as runs: plain text, and `{#TILE}` as the tile's
    /// glyph with its look, which a panel draws in the style's colors
    /// (D 10, M 7.3, M 7.7). A missing text is its key.
    pub fn text_runs(&self, lang: &str, key: &str, args: &[(String, Value)]) -> Vec<(String, Option<super::view::Look>)> {
        let Some(item) = self.m.text_item(lang, key) else { return vec![(key.to_string(), None)] };
        let mut out: Vec<(String, Option<super::view::Look>)> = Vec::new();
        let plain = |out: &mut Vec<(String, Option<super::view::Look>)>, t: &str| match out.last_mut() {
            Some((last, None)) => last.push_str(t),
            _ => out.push((t.to_string(), None)),
        };
        let mut rest = item.rest(1);
        while let Some(open) = rest.find('{') {
            plain(&mut out, &rest[..open]);
            let Some(close) = rest[open..].find('}') else {
                plain(&mut out, &rest[open..]);
                return out;
            };
            let inner = &rest[open + 1..open + close];
            if let Some(tile) = inner.strip_prefix('#') {
                let look = self.look(tile);
                out.push((look.ch.to_string(), Some(look)));
            } else if let Some(key) = inner.strip_prefix('@') {
                for (t, look) in self.text_runs(lang, key, &[]) {
                    match look {
                        None => plain(&mut out, &t),
                        Some(_) => out.push((t, look)),
                    }
                }
            } else {
                plain(&mut out, &self.placeholder(lang, inner, args));
            }
            rest = &rest[open + close + 1..];
        }
        plain(&mut out, rest);
        out
    }

    /// The name of an entity or a name value, as `{X}` shows it (D 10).
    pub fn name_text(&self, lang: &str, v: &Value) -> String {
        self.placeholder(lang, "X", &[("X".to_string(), v.clone())])
    }

    /// The description of an entity or a tile, as `{X:DESC}` shows it
    /// (D 10): `<KIND>.DESC` from the nearest kind that has it.
    pub fn description(&self, lang: &str, v: &Value) -> String {
        self.placeholder(lang, "X:DESC", &[("X".to_string(), v.clone())])
    }

    fn placeholder(&self, lang: &str, inner: &str, args: &[(String, Value)]) -> String {
        if let Some(key) = inner.strip_prefix('@') {
            return self.text(lang, key, &[]);
        }
        let (name, suffix) = match inner.split_once([':', '.']) {
            Some((n, s)) => (n, s),
            None => (inner, "NAME"),
        };
        let Some(v) = args.iter().find(|(n, _)| n == name).map(|(_, v)| v) else { return format!("{{{inner}}}") };
        match v {
            Value::Entity(e) => {
                let inst = self.s.get(*e);
                if suffix == "NAME"
                    && let Some(Value::Text(t)) = inst.get("name").and_then(|c| c.get("text"))
                {
                    return t.clone();
                }
                // a seat's member reads `KIND.SEATn.NAME` before `KIND.NAME`
                // at each level of its kind, so seats have their own names
                // and a child's own name still wins (D 10, M 7.3)
                let seat = self.s.seat_of(*e).map(|n| format!("SEAT{n}.{suffix}"));
                self.kind_text_opt(lang, &inst.kind, seat.as_deref(), suffix).unwrap_or_else(|| format!("{}.{suffix}", inst.kind))
            }
            Value::Name(n) => {
                if self.m.kind(n) == Some(Kind::Entity) || suffix != "NAME" {
                    // a tile's variant and index have no text keys of
                    // their own: the base name covers them (D 7.1), so
                    // `HULL.window` reads `HULL.DESC`
                    let mut base = n.as_str();
                    while self.kind_text_opt(lang, base, None, suffix).is_none()
                        && let Some((head, last)) = base.rsplit_once('.')
                        && last.chars().next().is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
                    {
                        base = head;
                    }
                    self.kind_text(lang, base, suffix)
                } else {
                    self.text(lang, n, &[])
                }
            }
            Value::Number(n) => n.to_string(),
            Value::Text(t) => t.clone(),
            other => other.to_string(),
        }
    }

    /// `<KIND>.NAME`, from the nearest parent that has the text (D 10).
    fn kind_text(&self, lang: &str, kind: &str, suffix: &str) -> String {
        self.kind_text_opt(lang, kind, None, suffix).unwrap_or_else(|| format!("{kind}.{suffix}"))
    }

    /// The text of the nearest kind that has `first` or `suffix`, `first`
    /// winning on the same level.
    fn kind_text_opt(&self, lang: &str, kind: &str, first: Option<&str>, suffix: &str) -> Option<String> {
        let mut cur = Some(kind.to_string());
        for _ in 0..64 {
            let k = cur?;
            for s in first.into_iter().chain([suffix]) {
                let key = format!("{k}.{s}");
                if self.m.has_text(lang, &key) {
                    return Some(self.text(lang, &key, &[]));
                }
            }
            cur = self.m.defs.get(&k).and_then(|d| d.block()).and_then(|b| {
                let parent = b.arg_after("extends")?;
                self.m.resolve(b.namespace.as_deref(), parent).ok().map(|d| d.full.clone())
            });
        }
        None
    }
}

/// Which handlers an event goes to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    All,
    /// Only the subject's components' handlers.
    Components,
    /// Only the general handlers.
    General,
}

impl Scope {
    fn takes(self, component: bool) -> bool {
        match self {
            Scope::All => true,
            Scope::Components => component,
            Scope::General => !component,
        }
    }
}

/// A handler in an event's list.
#[derive(Debug, Clone)]
struct HandlerRef {
    handler: Arc<Handler>,
    /// The component whose handler this is, for component handlers.
    component: Option<String>,
    name: String,
    ns: Option<String>,
    file: String,
}

/// A tile name matches a search name: the same or one of its variants and
/// indexes (`STAIR.down` is `STAIR`).
fn tile_matches(tile: &str, name: &str) -> bool {
    tile == name || tile.starts_with(&format!("{name}."))
}

/// Bresenham's line from `a` to `b`, both included; a tie goes to the
/// horizontal step (M 9.3).
pub fn line(a: Cell, b: Cell) -> Vec<Cell> {
    let (dx, dy) = ((b.x - a.x).abs(), -(b.y - a.y).abs());
    let (sx, sy) = (if a.x < b.x { 1 } else { -1 }, if a.y < b.y { 1 } else { -1 });
    let (mut x, mut y, mut err) = (a.x, a.y, dx + dy);
    let mut out = vec![];
    loop {
        out.push(Cell { x, y, ..a });
        if x == b.x && y == b.y {
            return out;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
}
