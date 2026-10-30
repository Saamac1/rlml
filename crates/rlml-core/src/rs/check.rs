//! Load-time checks that read RogueScript bodies (RS 2.4, RS 4, RS 5.1,
//! RS 12, M 2.5, M 9), with type inference (RS 2.6, RS 3.3, RS 5.3).
//!
//! * Upper-case names resolve like data names (D 4.4) and are checked by
//!   kind after lookup: a call is a function, query, command, ai,
//!   generator, scene, entity (a card) or component (a value), `emit`
//!   takes an event and `has` a component.
//! * A lower-case word is a local name, a context name, a direction or,
//!   when called, an engine function (M 9.3).
//! * `wait` and `open` belong to commands and coroutine scenes; a `query`
//!   or `present fn` does not change the game's state (M 9.2).
//! * Every expression has a type. A type that cannot be inferred is
//!   `Unknown` and never causes an error (see [`super::types`]). A local
//!   name takes its type from its first assignment.

use std::collections::{BTreeMap, BTreeSet};

use super::ast::*;
use super::sigs::{self, FnKind, Ret, Sig};
use super::types::{self, Member, Ty};
use crate::error::{Diagnostic, Report};
use crate::fields::{DIRS, SIDES, component_fields};
use crate::model::{Def, Kind, Model, Source as DefSource};
use crate::names;
use crate::rogue::Decl;

/// An engine function's kind (M 9.2); see [`super::sigs`].
pub fn engine_function(name: &str) -> Option<FnKind> {
    sigs::kind_of(name)
}

/// Context names (RS 2.4).
const CONTEXT: &[&str] = &["self", "holder", "actor", "it", "player", "game", "map", "camera", "profile"];

/// Engine component names usable as values in calls (`ai(behavior: …)`).
const ENGINE_COMPONENTS: &[&str] = &[
    "render", "movement", "ai", "sight", "light", "facing", "timer", "interior", "shape", "label", "wire", "blocks", "hearing", "sound",
    "name", "revealed",
];

/// Vocabulary values: `blocks(it, move, …)`, `via: line`, `delete_save(all)`.
const VALUES: &[&str] = &["move", "sight", "path", "line", "checkpoint", "auto", "all"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ctx {
    Function,
    Query,
    Present,
    Command,
    Handler { before: bool },
    Ai,
    Generator,
    Scene,
    Timeline,
}

impl Ctx {
    fn waits(self) -> bool {
        matches!(self, Ctx::Command | Ctx::Scene | Ctx::Timeline)
    }

    fn pure(self) -> bool {
        matches!(self, Ctx::Query | Ctx::Present)
    }
}

struct Body<'a> {
    m: &'a Model,
    decl: &'a Decl,
    ctx: Ctx,
    ret: bool,
    /// Parameters and locals with their types.
    locals: BTreeMap<String, Ty>,
    /// Locals assigned in the body (not parameters): a query may change them.
    own: BTreeSet<String>,
    /// The member type of each enclosing `where`; the last is `it`.
    it: Vec<Ty>,
    texts: &'a BTreeSet<String>,
    errors: Vec<Diagnostic>,
    tiles: &'a mut BTreeSet<String>,
}

/// Checks every RogueScript body; returns the tiles the logic names.
pub fn check(m: &Model, r: &mut Report) -> BTreeSet<String> {
    let texts = text_keys(m);
    let mut tiles = BTreeSet::new();
    for decl in &m.code {
        check_decl(m, decl, &texts, &mut tiles, r);
    }
    tiles
}

/// The inferred types of a declaration's parameters and locals, for tests
/// and tools.
pub fn local_types(m: &Model, decl: &Decl) -> BTreeMap<String, Ty> {
    let texts = text_keys(m);
    let mut tiles = BTreeSet::new();
    let mut r = Report::default();
    check_decl(m, decl, &texts, &mut tiles, &mut r)
}

fn text_keys(m: &Model) -> BTreeSet<String> {
    m.languages.iter().flat_map(|f| f.texts.iter().map(move |t| names::qualify(f.namespace.as_deref(), t.key()))).collect()
}

/// Checks one declaration; returns the locals of its (last) body.
fn check_decl(m: &Model, decl: &Decl, texts: &BTreeSet<String>, tiles: &mut BTreeSet<String>, r: &mut Report) -> BTreeMap<String, Ty> {
    let item = decl.item.clone();
    let mut last = BTreeMap::new();
    let mut run = |ctx: Ctx, ret: bool, f: &mut dyn FnMut(&mut Body)| {
        let mut b = Body { m, decl, ctx, ret, locals: BTreeMap::new(), own: BTreeSet::new(), it: vec![], texts, errors: vec![], tiles: &mut *tiles };
        f(&mut b);
        for e in b.errors {
            r.push(e);
        }
        last = b.locals;
    };
    match &item.kind {
        ItemKind::Component { fields, handlers } => {
            run(Ctx::Function, false, &mut |b| b.params(fields));
            for h in handlers {
                // a component's handlers see its fields by name (RS 3.1)
                run(Ctx::Handler { before: h.phase == Phase::Before }, false, &mut |b| {
                    b.params(fields);
                    b.handler(h, true);
                });
            }
        }
        ItemKind::Event { params } => run(Ctx::Function, false, &mut |b| b.params(params)),
        ItemKind::Handler(h) => run(Ctx::Handler { before: h.phase == Phase::Before }, false, &mut |b| b.handler(h, false)),
        ItemKind::Function { present, query, params, ret, body } => {
            let ctx = if *query {
                Ctx::Query
            } else if *present {
                Ctx::Present
            } else {
                Ctx::Function
            };
            run(ctx, ret.is_some(), &mut |b| {
                b.params(params);
                b.body(body);
            });
        }
        ItemKind::Command { params, body, .. } => run(Ctx::Command, true, &mut |b| {
            b.params(params);
            let counts = params.iter().filter(|p| matches!(p.source, Some(Source::Count))).collect::<Vec<_>>();
            if counts.len() > 1 {
                b.error(counts[1].pos, "a command has at most one `count` parameter (RS 12.1)");
            }
            for p in counts.iter().filter(|p| p.ty.base != "number") {
                b.error(p.pos, format!("`count` is only for a `number` parameter, not `{}`", p.ty));
            }
            b.body(body);
        }),
        ItemKind::CommandAlias(call) => run(Ctx::Command, true, &mut |b| {
            if let ExprKind::Call(f, args) = &call.kind
                && let ExprKind::Name { name, root } = &f.kind
            {
                b.name(f.pos, name, *root, &[Kind::Command]);
                for a in args {
                    b.expr(&a.value);
                }
            }
        }),
        ItemKind::Ai { body } => run(Ctx::Ai, true, &mut |b| b.body(body)),
        ItemKind::Generator { params, body, .. } => run(Ctx::Generator, false, &mut |b| {
            b.params(params);
            b.body(body);
        }),
        ItemKind::Map { generate, .. } => run(Ctx::Function, false, &mut |b| {
            if let ExprKind::Call(f, args) = &generate.kind
                && let ExprKind::Name { name, root } = &f.kind
            {
                b.name(f.pos, name, *root, &[Kind::Generator]);
                for a in args {
                    b.expr(&a.value);
                }
            } else {
                b.error(generate.pos, "`generate` takes a generator call");
            }
        }),
        ItemKind::Scene { params, body } => run(Ctx::Scene, false, &mut |b| {
            b.params(params);
            b.body(body);
        }),
        ItemKind::Timeline { params, timeline } => run(Ctx::Timeline, false, &mut |b| {
            b.params(params);
            for t in &timeline.tracks {
                b.rows(&t.rows);
            }
            b.rows(&timeline.rows);
        }),
    }
    last
}

impl Body<'_> {
    fn error(&mut self, pos: Pos, msg: impl Into<String>) {
        self.errors.push(Diagnostic::error(&self.decl.file, pos.line, format!("column {}: {}", pos.col, msg.into())));
    }

    fn ns(&self) -> Option<&str> {
        self.decl.namespace.as_deref()
    }

    fn is_component(&self, name: &str) -> bool {
        self.m.resolve(self.ns(), name).is_ok_and(|d| d.kind == Kind::Component)
    }

    /// A type as written; a bad type is reported once and is `Unknown`.
    fn written_type(&mut self, pos: Pos, text: &str) -> Ty {
        match types::parse(text, &|n| self.is_component(n)) {
            Ok(t) => t,
            Err(e) => {
                self.error(pos, e);
                Ty::Unknown
            }
        }
    }

    /// Declares parameters (or a component's fields) as typed locals.
    fn params(&mut self, params: &[Param]) {
        for p in params {
            let ty = self.written_type(p.pos, &p.ty.to_string());
            self.locals.insert(p.name.clone(), ty.clone());
            if let Some(d) = &p.default {
                self.expr(d);
            }
            match &p.source {
                Some(Source::From(e)) | Some(Source::Ask(Some(e))) => {
                    self.expr(e);
                }
                Some(Source::Aim { range, at }) => {
                    self.expr(range);
                    if let Some(at) = at {
                        self.expr(at);
                    }
                }
                _ => {}
            }
        }
    }

    /// The parameter types of an event: an engine event's from M 9.6, a
    /// game event's from its declaration. A component's handler leaves out
    /// the subject.
    fn event_params(&self, event: &str, in_component: bool) -> Vec<Ty> {
        let mut tys: Vec<Ty> = if names::is_vocabulary(event) {
            sigs::engine_event(event).map(|ps| ps.into_iter().map(|(_, t)| t).collect()).unwrap_or_default()
        } else {
            match self.m.resolve(self.ns(), event).map(|d| &d.source) {
                Ok(DefSource::Decl(d)) => match &d.item.kind {
                    ItemKind::Event { params } => {
                        params.iter().map(|p| types::parse(&p.ty.to_string(), &|n| self.is_component(n)).unwrap_or(Ty::Unknown)).collect()
                    }
                    _ => vec![],
                },
                _ => vec![],
            }
        };
        if in_component && !tys.is_empty() {
            tys.remove(0);
        }
        tys
    }

    fn handler(&mut self, h: &Handler, in_component: bool) {
        let engine = names::is_vocabulary(&h.event);
        if engine {
            if !crate::check::ENGINE_EVENTS.contains(&h.event.as_str()) {
                self.error(h.pos, format!("unknown engine event `{}`", h.event));
            } else if h.phase == Phase::Before {
                self.error(h.pos, format!("engine event `{}` has no `before` phase (RS 4.1)", h.event));
            }
        } else if in_component
            && let Err(e) = self.m.resolve_kind(self.ns(), &h.event, &[Kind::Event])
        {
            // top-level handlers' events are checked with the declarations (check.rs)
            self.error(h.pos, e);
        }
        let tys = self.event_params(&h.event, in_component);
        for (i, n) in h.names.iter().enumerate() {
            self.locals.insert(n.clone(), tys.get(i).cloned().unwrap_or(Ty::Unknown));
        }
        if let Some(w) = &h.when {
            self.expr(w);
        }
        self.body(&h.body);
    }

    fn body(&mut self, body: &[Stmt]) {
        let mut assigned = BTreeSet::new();
        collect_assigned(body, &mut assigned);
        for n in &assigned {
            self.locals.entry(n.clone()).or_insert(Ty::Unknown);
        }
        self.own.extend(assigned);
        for s in body {
            self.stmt(s);
        }
    }

    /// Gives a local its type at an assignment (RS 2.6): the first known
    /// type sticks, and an empty list takes its member type from its use.
    fn bind(&mut self, pos: Pos, name: &str, ty: Ty) {
        let old = self.locals.get(name).cloned().unwrap_or(Ty::Unknown);
        let new = match (&old, &ty) {
            (Ty::Unknown, _) | (Ty::None, _) => ty,
            (Ty::List(m), Ty::List(n)) if !m.is_known() => Ty::List(n.clone()),
            (_, Ty::Unknown) | (_, Ty::None) => old,
            (o, n) if types::fits(n, o) => old.clone(),
            // a number local may hold dice or a percentage's product
            (o, n) => {
                self.error(pos, format!("`{name}` has the type `{o}`; a `{n}` cannot be assigned to it (RS 2.6)"));
                old.clone()
            }
        };
        self.locals.insert(name.to_string(), new);
    }

    fn stmt(&mut self, s: &Stmt) {
        match &s.kind {
            StmtKind::Assign { target, op, value } => {
                if self.ctx.pure() {
                    let root = root_word(target);
                    if !root.is_some_and(|r| self.own.contains(r)) {
                        let what = if self.ctx == Ctx::Query { "a `query`" } else { "a `present fn`" };
                        self.error(s.pos, format!("{what} does not change the game's state; `{target}` is not a local name of its own (M 9.2)"));
                    }
                }
                let vt = self.expr(value);
                match (&target.kind, op) {
                    (ExprKind::Word(w), AssignOp::Set) => self.bind(s.pos, w, vt),
                    (ExprKind::Word(w), AssignOp::Add) => {
                        // `l += v` adds to a list (RS 2.6)
                        if let Some(Ty::List(m)) = self.locals.get(w).cloned() {
                            if !m.is_known() {
                                self.locals.insert(w.clone(), Ty::list(vt));
                            } else if !types::fits(&vt, &m) && !types::fits(&vt, &Ty::list((*m).clone())) {
                                self.error(s.pos, format!("`{w}` has the type `list {m}`; a `{vt}` cannot be added to it (RS 2.6)"));
                            }
                        }
                    }
                    (ExprKind::Word(_), AssignOp::Sub) => {}
                    _ => {
                        self.expr(target);
                    }
                }
            }
            StmtKind::WaitInput { name, kind, arg } => {
                self.waits(s.pos, "`wait`");
                self.expr(arg);
                self.bind(s.pos, name, if kind == "number" { Ty::Number } else { Ty::Text });
            }
            StmtKind::OpenValue { call, .. } => {
                self.waits(s.pos, "`open`");
                self.open(call);
            }
            StmtKind::If { arms, otherwise } => {
                for (c, b) in arms {
                    self.expr(c);
                    self.stmts(b);
                }
                if let Some(b) = otherwise {
                    self.stmts(b);
                }
            }
            StmtKind::For { var, iter, body } => {
                let it = self.expr(iter);
                self.bind(s.pos, var, it.member());
                self.stmts(body);
            }
            StmtKind::Repeat { count, body } => {
                self.expr(count);
                self.stmts(body);
            }
            StmtKind::While { cond, body } => {
                self.expr(cond);
                self.stmts(body);
            }
            StmtKind::Break => {}
            StmtKind::Match { subject, cases } => {
                self.expr(subject);
                for (c, b) in cases {
                    if let Some(c) = c {
                        self.expr(c);
                    }
                    self.stmts(b);
                }
            }
            StmtKind::Choice(options) => {
                if !self.ctx.waits() {
                    self.error(s.pos, "`choice` belongs to a scene (RS 9.7)");
                }
                for o in options {
                    for a in &o.args {
                        self.expr(&a.value);
                    }
                    for e in o.hide.iter().chain(o.disabled.iter()) {
                        self.expr(e);
                    }
                    match &o.body {
                        OptionBody::Block(b) => self.stmts(b),
                        OptionBody::Adjust { target, range, pool } => {
                            self.expr(target);
                            self.expr(range);
                            if let Some(p) = pool {
                                self.expr(p);
                            }
                        }
                    }
                }
            }
            StmtKind::Emit(call) => {
                if self.ctx.pure() {
                    self.error(s.pos, "`emit` changes the game's state and is not allowed in a `query` or `present fn` (M 9.2)");
                }
                match &call.kind {
                    ExprKind::Call(f, args) => {
                        if let ExprKind::Name { name, root } = &f.kind {
                            self.name(f.pos, name, *root, &[Kind::Event]);
                        } else {
                            self.error(call.pos, "`emit` takes an event call such as `emit HIT(target, attacker, 1)`");
                        }
                        for a in args {
                            self.expr(&a.value);
                        }
                    }
                    _ => {
                        self.expr(call);
                    }
                }
            }
            StmtKind::Wait(arg) => {
                self.waits(s.pos, "`wait`");
                self.wait_arg(arg);
            }
            StmtKind::Open(call) => {
                self.waits(s.pos, "`open`");
                self.open(call);
            }
            StmtKind::Return(value) => match (value, self.ret) {
                (Some(_), false) => {
                    let msg = match self.ctx {
                        Ctx::Function | Ctx::Present => "a `fn` without a return type returns no value: add `-> type` or write `return` (RS 5.1)",
                        _ => "`return` takes no value here",
                    };
                    self.error(s.pos, msg);
                    self.expr(value.as_ref().unwrap());
                }
                (None, true) if matches!(self.ctx, Ctx::Query | Ctx::Ai) => self.error(s.pos, "a `query` or `ai` returns a value"),
                (Some(v), _) => {
                    self.expr(v);
                }
                _ => {}
            },
            StmtKind::Cancel => {
                if !matches!(self.ctx, Ctx::Handler { before: true }) {
                    self.error(s.pos, "`cancel` belongs to a `before` handler (RS 4)");
                }
            }
            StmtKind::Call(e) => {
                self.expr(e);
            }
        }
    }

    fn stmts(&mut self, b: &[Stmt]) {
        for s in b {
            self.stmt(s);
        }
    }

    fn waits(&mut self, pos: Pos, what: &str) {
        if !self.ctx.waits() {
            self.error(pos, format!("{what} is allowed only in a command or a coroutine scene (RS 5.1, M 6.1.6)"));
        }
    }

    fn wait_arg(&mut self, arg: &WaitArg) {
        match arg {
            WaitArg::Time(e) | WaitArg::Input { arg: e, .. } => {
                self.expr(e);
            }
            WaitArg::Scene(call) => {
                if let ExprKind::Call(f, args) = &call.kind {
                    if let ExprKind::Name { name, root } = &f.kind {
                        self.name(f.pos, name, *root, &[Kind::Scene]);
                    }
                    for a in args {
                        self.expr(&a.value);
                    }
                }
            }
            WaitArg::Confirm => {}
        }
    }

    fn open(&mut self, call: &Expr) {
        let (target, args) = match &call.kind {
            ExprKind::Call(f, args) => (f.as_ref(), args.as_slice()),
            _ => (call, &[][..]),
        };
        if let ExprKind::Name { name, root } = &target.kind {
            self.name(target.pos, name, *root, &[Kind::Menu, Kind::Panel]);
        } else {
            self.error(call.pos, "`open` takes a menu or a panel");
        }
        for a in args {
            self.expr(&a.value);
        }
    }

    fn rows(&mut self, rows: &[Row]) {
        for r in rows {
            match r {
                Row::Action { when, action, .. } => {
                    if let Some(w) = when {
                        self.expr(w);
                    }
                    match action {
                        Action::Call(c) => {
                            self.expr(c);
                        }
                        Action::Wait(w) => self.wait_arg(w),
                    }
                }
                Row::When { cond, rows, otherwise, .. } => {
                    self.expr(cond);
                    self.rows(rows);
                    self.rows(otherwise);
                }
                Row::Mark { .. } => {}
            }
        }
    }

    /// Resolves an upper-case name and checks its kind; `kinds` empty takes
    /// any kind, also text keys.
    fn name(&mut self, pos: Pos, name: &str, root: bool, kinds: &[Kind]) -> Option<&Def> {
        let written = if root { format!(".{name}") } else { name.to_string() };
        let m = self.m;
        match m.resolve(self.ns(), &written) {
            Ok(def) => {
                if def.kind == Kind::Tile {
                    self.tiles.insert(def.full.clone());
                }
                if !kinds.is_empty() && !kinds.contains(&def.kind) {
                    let want: Vec<String> = kinds.iter().map(|k| k.to_string()).collect();
                    let msg = format!("`{}` is a {} ({}), not a {}", def.full, def.kind, def.place(), want.join(" or "));
                    self.error(pos, msg);
                }
                Some(def)
            }
            Err(e) => {
                if kinds.is_empty() && names::candidates(self.ns(), &written).iter().any(|c| self.texts.contains(c)) {
                    return None;
                }
                self.error(pos, e);
                None
            }
        }
    }

    /// The type of a bare upper-case name (RS 2.6): a constant has its
    /// value's type, a deck or table is a source, and other definitions are
    /// names.
    fn name_ty(&self, def: Option<&Def>) -> Ty {
        let Some(def) = def else { return Ty::Name };
        match def.kind {
            Kind::Deck => Ty::Deck,
            Kind::Table => Ty::Table,
            Kind::Constant => match &def.source {
                DefSource::Row(row) => literal_ty(row.rest(1)),
                _ => Ty::Unknown,
            },
            _ => Ty::Name,
        }
    }

    fn expr(&mut self, e: &Expr) -> Ty {
        use ExprKind::*;
        match &e.kind {
            Int(_) => Ty::Number,
            Dice(..) => Ty::Dice,
            Time(..) => Ty::Time,
            Percent(_) => Ty::Percent,
            Size(..) => Ty::Size,
            Str(_) => Ty::Text,
            Bool(_) => Ty::Bool,
            None => Ty::None,
            All => Ty::All,
            EmptyDict => Ty::Dict(Box::new(Ty::Unknown), Box::new(Ty::Unknown)),
            Name { name, root } => {
                let def = self.name(e.pos, name, *root, &[]).cloned();
                self.name_ty(def.as_ref())
            }
            Word(w) => self.word(e.pos, w),
            Wild { name, root, .. } => {
                let prefix = format!("{}{name}.", if *root { "." } else { "" });
                let hits = names::candidates(self.ns(), name).iter().any(|c| {
                    let p = format!("{c}.");
                    self.m.defs.keys().any(|k| k.starts_with(&p)) || self.texts.iter().any(|k| k.starts_with(&p))
                });
                if !hits {
                    self.error(e.pos, format!("wildcard `{prefix}*` matches no names"));
                }
                Ty::Collection(Member::Name)
            }
            List(items) => {
                let mut member = Ty::Unknown;
                for i in items {
                    let t = self.expr(i);
                    if !member.is_known() {
                        member = t;
                    } else if t.is_known() && !types::fits(&t, &member) && !types::fits(&member, &t) {
                        self.error(i.pos, format!("a list holds one type: `{i}` is a `{t}` among `{member}` members (RS 2.6)"));
                    }
                }
                Ty::list(member)
            }
            Field(..) => self.path(e),
            TileIndex(base, _) => {
                self.expr(base);
                Ty::Name
            }
            Call(f, args) => self.call(e, f, args),
            Subscript(a, i) => {
                let at = self.expr(a);
                self.expr(i);
                match at {
                    Ty::List(m) => *m,
                    Ty::Dict(_, v) => *v,
                    _ => Ty::Unknown,
                }
            }
            Binary(op, a, b) => {
                let (ta, tb) = (self.expr(a), self.expr(b));
                binary_ty(*op, &ta, &tb)
            }
            In(a, b) | Is(a, b) => {
                self.expr(a);
                self.expr(b);
                Ty::Bool
            }
            Neg(a) => match self.expr(a) {
                t @ (Ty::Number | Ty::Dice | Ty::Percent | Ty::Dir | Ty::Side) => t,
                _ => Ty::Unknown,
            },
            Not(a) => {
                self.expr(a);
                Ty::Bool
            }
            Range(a, b) => {
                let index = matches!(a.kind, TileIndex(..));
                self.expr(a);
                if let Some(b) = b {
                    self.expr(b);
                }
                // a tile index range is a set of tile names (RS 2.6)
                if index { Ty::Collection(Member::Name) } else { Ty::Range }
            }
            Has { subject, component, .. } => {
                self.expr(subject);
                if names::is_vocabulary(component) {
                    if !ENGINE_COMPONENTS.contains(&component.as_str()) {
                        self.error(e.pos, format!("`has {component}`: no engine component `{component}`"));
                    }
                } else {
                    self.name(e.pos, component, false, &[Kind::Component]);
                }
                Ty::Bool
            }
            Where(a, b) => {
                let ta = self.expr(a);
                self.it.push(ta.member());
                self.expr(b);
                self.it.pop();
                ta
            }
        }
    }

    fn word(&mut self, pos: Pos, w: &str) -> Ty {
        if w == "it" {
            return match self.it.last() {
                Some(t) => t.clone(),
                Option::None => {
                    self.error(pos, "`it` is the member inside `where` (RS 5.3)");
                    Ty::Unknown
                }
            };
        }
        if let Some(t) = self.locals.get(w) {
            return t.clone();
        }
        match w {
            "self" | "holder" | "actor" | "player" => return Ty::Entity,
            "camera" => return Ty::Camera,
            _ => {}
        }
        if CONTEXT.contains(&w) || VALUES.contains(&w) {
            return Ty::Unknown;
        }
        if DIRS.contains(&w) {
            return Ty::Dir;
        }
        if SIDES.contains(&w) {
            return Ty::Side;
        }
        if engine_function(w).is_some() {
            self.error(pos, format!("`{w}` is an engine function: call it with `{w}(…)`"));
        } else {
            self.error(pos, format!("unknown name `{w}`: not a parameter, a local name or a context name"));
        }
        Ty::Unknown
    }

    /// A field path such as `target.HP.now`, `game.KING_QUEST`, `p.x` or
    /// `w.range` (RS 3.3): upper-case parts match the longest component
    /// name, the rest are fields.
    fn path(&mut self, e: &Expr) -> Ty {
        let mut parts = Vec::new();
        let mut root = e;
        while let ExprKind::Field(base, part) = &root.kind {
            parts.push(part.as_str());
            root = base;
        }
        parts.reverse();
        // a tile variant: `GATE.open`, `STAIR.down`
        if let ExprKind::Name { name, root: r } = &root.kind {
            let def = self.name(root.pos, name, *r, &[]).cloned();
            if def.as_ref().is_some_and(|d| d.kind == Kind::Tile) && parts.iter().all(|p| names::is_vocabulary(p)) {
                return Ty::Name;
            }
            return Ty::Unknown;
        }
        let record = match &root.kind {
            ExprKind::Word(w) if ["game", "profile", "map"].contains(&w.as_str()) && !self.locals.contains_key(w) => Some(w.as_str()),
            _ => Option::None,
        };
        let mut cur = match record {
            Some(_) => Ty::Unknown,
            Option::None => self.expr(root),
        };
        let mut i = 0;
        if let Some(rec) = record {
            match parts.first() {
                Some(p) if rec == "game" && names::is_vocabulary(p) => {
                    cur = sigs::game_field(p).unwrap_or(Ty::Unknown);
                    i = 1;
                }
                // map variables are set by generators (D 8.4)
                _ if rec == "map" => return Ty::Unknown,
                Some(_) => cur = Ty::Entity,
                Option::None => return Ty::Unknown,
            }
        }
        while i < parts.len() {
            let p = parts[i];
            let next = match &cur {
                Ty::Entity if names::is_part(p) => {
                    // the longest run of upper-case parts that is a component
                    let upper = parts[i..].iter().take_while(|q| names::is_part(q)).count();
                    let found = (1..=upper).rev().find_map(|n| {
                        let name = parts[i..i + n].join(".");
                        self.m.resolve(self.ns(), &name).ok().filter(|d| d.kind == Kind::Component).map(|d| (n, d.full.clone()))
                    });
                    match found {
                        Some((n, full)) => {
                            i += n - 1;
                            Ty::Component(Some(full))
                        }
                        Option::None => {
                            self.error(e.pos, format!("`{}`: no component `{p}` (RS 3.3)", e));
                            return Ty::Unknown;
                        }
                    }
                }
                Ty::Entity if ENGINE_COMPONENTS.contains(&p) && sigs::field(&Ty::Entity, p).is_none() => Ty::Component(Some(p.to_string())),
                Ty::Component(Some(c)) => self.component_field(c, p),
                t => sigs::field(t, p).unwrap_or(Ty::Unknown),
            };
            cur = next;
            i += 1;
        }
        cur
    }

    /// A field's type in a component value (D 5.1, D 5.3).
    fn component_field(&self, component: &str, field: &str) -> Ty {
        if names::is_vocabulary(component) {
            return sigs::engine_component_field(component, field).unwrap_or(Ty::Unknown);
        }
        let Some(def) = self.m.defs.get(component) else { return Ty::Unknown };
        let ns = def.full.rsplit_once('.').map(|(n, _)| n.to_string());
        component_fields(def)
            .into_iter()
            .find(|f| f.name == field)
            .map(|f| types::parse(&f.ty, &|n| self.m.resolve(ns.as_deref(), n).is_ok_and(|d| d.kind == Kind::Component)).unwrap_or(Ty::Unknown))
            .unwrap_or(Ty::Unknown)
    }

    fn call(&mut self, e: &Expr, f: &Expr, args: &[Arg]) -> Ty {
        match &f.kind {
            ExprKind::Name { name, root } => {
                let callable = [Kind::Function, Kind::Query, Kind::Command, Kind::Ai, Kind::Generator, Kind::Scene, Kind::Entity, Kind::Component];
                let def = self.name(f.pos, name, *root, &callable).cloned();
                if self.ctx.pure()
                    && let Some(def) = &def
                    && let DefSource::Decl(d) = &def.source
                    && let ItemKind::Function { present, query: false, .. } = &d.item.kind
                    && !(*present && self.ctx == Ctx::Present)
                {
                    self.error(f.pos, format!("`{}` is a `fn` that may change the game's state; a `query` or `present fn` calls only queries (M 9.2)", def.full));
                }
                for a in args {
                    self.expr(&a.value);
                }
                def.map(|d| self.call_ty(&d)).unwrap_or(Ty::Unknown)
            }
            ExprKind::Word(w) => {
                let w = w.clone();
                if ENGINE_COMPONENTS.contains(&w.as_str()) && engine_function(&w).is_none() {
                    // `ai(behavior: FOLLOWER)` is a component value (M 3.4)
                    for a in args {
                        self.expr(&a.value);
                    }
                    return Ty::Component(Some(w));
                }
                let mut tys = Vec::new();
                for a in args {
                    let t = if self.is_label_arg(&w, a, args) { Ty::Unknown } else { self.expr(&a.value) };
                    tys.push((a.name.clone(), t));
                }
                self.engine_call(f.pos, &w, args.first().map(|a| &a.value), &tys)
            }
            ExprKind::Field(recv, w) if names::is_vocabulary(w) => {
                // a chain call `x.f(a)` is `f(x, a)` (RS 2.7)
                let mut tys = vec![(Option::None, self.expr(recv))];
                for a in args {
                    let t = self.expr(&a.value);
                    tys.push((a.name.clone(), t));
                }
                self.engine_call(f.pos, w, Some(recv), &tys)
            }
            _ => {
                self.error(e.pos, format!("`{f}` cannot be called"));
                self.expr(f);
                Ty::Unknown
            }
        }
    }

    /// The type of `NAME(…)` by the name's kind (RS 2.8).
    fn call_ty(&self, def: &Def) -> Ty {
        match def.kind {
            Kind::Command | Kind::Ai => Ty::Command,
            Kind::Scene => Ty::Scene,
            Kind::Component => Ty::Component(Some(def.full.clone())),
            Kind::Entity => Ty::Card,
            Kind::Function | Kind::Query => match &def.source {
                DefSource::Decl(d) => match &d.item.kind {
                    ItemKind::Function { ret: Some(ReturnType::Type(t)), .. } => {
                        let ns = d.namespace.clone();
                        types::parse(&t.to_string(), &|n| self.m.resolve(ns.as_deref(), n).is_ok_and(|d| d.kind == Kind::Component)).unwrap_or(Ty::Unknown)
                    }
                    ItemKind::Function { ret: Some(ReturnType::NameOrText), .. } => Ty::Union(vec![Ty::Name, Ty::Text]),
                    ItemKind::Function { ret: Option::None, .. } => Ty::None,
                    _ => Ty::Unknown,
                },
                _ => Ty::Unknown,
            },
            _ => Ty::Unknown,
        }
    }

    /// Label arguments are names of the map, not general names (D 8.5), and
    /// movement modes (`mode: MISSILE`, `blocks(p, WALK)`) are not defined
    /// anywhere (D 7.5).
    fn is_label_arg(&self, f: &str, a: &Arg, args: &[Arg]) -> bool {
        let named = |n: &str| a.name.as_deref() == Some(n);
        let first = std::ptr::eq(a, &args[0]);
        let second = args.len() > 1 && std::ptr::eq(a, &args[1]);
        let is_name = matches!(a.value.kind, ExprKind::Name { .. });
        let label = (f == "label" && first) || (f == "tag" && second) || (["travel", "spawn"].contains(&f) && named("at")) || (f == "tile" && named("label"));
        let mode = (f == "ray" && named("mode")) || (f == "blocks" && (second || named("what")));
        is_name && (label || mode)
    }

    /// Checks an engine call's kind against the body and gives its type
    /// from the version the arguments select (M 9.2).
    fn engine_call(&mut self, pos: Pos, w: &str, receiver: Option<&Expr>, args: &[(Option<String>, Ty)]) -> Ty {
        let Some(kind) = engine_function(w) else {
            if self.locals.contains_key(w) {
                self.error(pos, format!("`{w}` is a local name and cannot be called"));
            } else {
                self.error(pos, format!("unknown engine function `{w}` (M 9.3)"));
            }
            return Ty::Unknown;
        };
        if w == "external" && self.ctx != Ctx::Generator {
            self.error(pos, "`external` is allowed only in a generator (M 9.3)");
        }
        if self.ctx.pure() {
            let local = receiver.is_some_and(|r| self.local_value(r));
            let bad = match kind {
                FnKind::Changes => true,
                FnKind::ChangesReceiver => !local && !(w == "draw" && receiver.is_some_and(|r| self.is_table(r))),
                FnKind::Present => self.ctx == Ctx::Query && w != "reveal",
                FnKind::Query => false,
            };
            if bad {
                let what = if self.ctx == Ctx::Query { "a `query`" } else { "a `present fn`" };
                let why = match kind {
                    FnKind::Present => "is presentation",
                    FnKind::ChangesReceiver => "changes its receiver, which is not a local copy or list",
                    _ => "changes the game's state",
                };
                self.error(pos, format!("`{w}` {why}; it is not allowed in {what} (M 9.2)"));
            }
        }
        match select(w, args) {
            Some(sig) => ret_ty(sig, w, args),
            Option::None => Ty::Unknown,
        }
    }

    /// A local value of the body's own: a local name, a list literal or a
    /// copy (`LOOT.RARE.copy()`).
    fn local_value(&self, e: &Expr) -> bool {
        match &e.kind {
            ExprKind::Word(w) => self.own.contains(w),
            ExprKind::List(_) | ExprKind::EmptyDict => true,
            ExprKind::Call(f, _) => matches!(&f.kind, ExprKind::Field(_, w) if w == "copy") || matches!(&f.kind, ExprKind::Word(w) if w == "copy"),
            _ => false,
        }
    }

    fn is_table(&self, e: &Expr) -> bool {
        matches!(&e.kind, ExprKind::Name { name, .. } if self.m.resolve(self.ns(), name).is_ok_and(|d| d.kind == Kind::Table))
    }
}

/// The one version of an engine function that the arguments fit, if there
/// is exactly one (M 9.2).
pub fn select(name: &str, args: &[(Option<String>, Ty)]) -> Option<&'static Sig> {
    let fitting: Vec<&Sig> = sigs::versions(name).into_iter().filter(|s| fits_version(s, args)).collect();
    match fitting.as_slice() {
        [one] => Some(one),
        _ => Option::None,
    }
}

/// Do the arguments fit a version: positional ones in order, named ones by
/// name, every required parameter given and every type fitting?
pub fn fits_version(sig: &Sig, args: &[(Option<String>, Ty)]) -> bool {
    let mut given = vec![false; sig.params.len()];
    let mut next = 0;
    for (name, ty) in args {
        let i = match name {
            Option::None => {
                if next >= sig.params.len() {
                    return false;
                }
                next += 1;
                next - 1
            }
            Some(n) => match sig.params.iter().position(|p| !p.positional && &p.name == n) {
                Some(i) => i,
                // further named arguments: text parameters of `say` (M 9.3)
                Option::None if sig.rest => continue,
                Option::None => return false,
            },
        };
        if given[i] || !types::fits(ty, &sig.params[i].ty) {
            return false;
        }
        given[i] = true;
    }
    sig.params.iter().zip(&given).all(|(p, g)| *g || p.optional)
}

fn ret_ty(sig: &Sig, name: &str, args: &[(Option<String>, Ty)]) -> Ty {
    let first = args.first().map(|(_, t)| t.clone()).unwrap_or(Ty::Unknown);
    match &sig.ret {
        Ret::Ty(t) => t.clone(),
        Ret::Member => first.member(),
        // `shuffled` and `pick` of an area give positions in drawn order (M 9.3)
        Ret::Same if first == Ty::Area && ["shuffled", "pick"].contains(&name) => Ty::list(Ty::Pos),
        Ret::Same => first,
    }
}

/// Operator types (RS 5.3, RS 6.2).
fn binary_ty(op: BinOp, a: &Ty, b: &Ty) -> Ty {
    use BinOp::*;
    let area = |t: &Ty| matches!(t, Ty::Area);
    match op {
        Eq | Ne | Lt | Le | Gt | Ge => Ty::Bool,
        // `and`, `or` and `-` are intersection, union and difference of areas
        And | Or if area(a) || area(b) => Ty::Area,
        And | Or => Ty::Bool,
        Sub if area(a) => Ty::Area,
        Add if *a == Ty::Pos && matches!(b, Ty::Dir | Ty::Unknown) => Ty::Pos,
        Add if *a == Ty::Text && *b == Ty::Text => Ty::Text,
        Add | Sub if *a == Ty::Percent && *b == Ty::Percent => Ty::Percent,
        Add | Sub | Mul | Div | Rem if [a, b].iter().all(|t| matches!(t, Ty::Number | Ty::Dice | Ty::Range | Ty::Percent)) => Ty::Number,
        _ => Ty::Unknown,
    }
}

/// The type of a data value as written (`50`, `1d6`, `25%`, `"x"`).
fn literal_ty(v: &str) -> Ty {
    if crate::fields::is_int(v) {
        Ty::Number
    } else if crate::fields::is_dice(v) {
        Ty::Dice
    } else if crate::fields::is_range(v) {
        Ty::Range
    } else if v.strip_suffix('%').is_some_and(crate::fields::is_int) {
        Ty::Percent
    } else if v == "true" || v == "false" {
        Ty::Bool
    } else if v.starts_with('"') {
        Ty::Text
    } else {
        Ty::Unknown
    }
}

/// The local name at the root of an assignment target (`rooms`, `l[i]`).
fn root_word(e: &Expr) -> Option<&str> {
    match &e.kind {
        ExprKind::Word(w) => Some(w),
        ExprKind::Subscript(a, _) | ExprKind::Field(a, _) => root_word(a),
        _ => Option::None,
    }
}

fn collect_assigned(body: &[Stmt], out: &mut BTreeSet<String>) {
    for s in body {
        match &s.kind {
            StmtKind::Assign { target: Expr { kind: ExprKind::Word(w), .. }, .. } => {
                out.insert(w.clone());
            }
            StmtKind::WaitInput { name, .. } | StmtKind::OpenValue { name, .. } => {
                out.insert(name.clone());
            }
            StmtKind::For { var, body, .. } => {
                out.insert(var.clone());
                collect_assigned(body, out);
            }
            StmtKind::If { arms, otherwise } => {
                for (_, b) in arms {
                    collect_assigned(b, out);
                }
                if let Some(b) = otherwise {
                    collect_assigned(b, out);
                }
            }
            StmtKind::Repeat { body, .. } | StmtKind::While { body, .. } => collect_assigned(body, out),
            StmtKind::Match { cases, .. } => {
                for (_, b) in cases {
                    collect_assigned(b, out);
                }
            }
            StmtKind::Choice(options) => {
                for o in options {
                    if let OptionBody::Block(b) = &o.body {
                        collect_assigned(b, out);
                    }
                }
            }
            _ => {}
        }
    }
}
