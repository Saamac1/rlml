//! Load-time checks that read RogueScript bodies and need no types
//! (RS 2.4, RS 4, RS 5.1, RS 12, M 2.5, M 9).
//!
//! * Upper-case names resolve like data names (D 4.4) and are checked by
//!   kind after lookup: a call is a function, query, command, ai,
//!   generator, scene, entity (a card) or component (a value), `emit`
//!   takes an event and `has` a component.
//! * A lower-case word is a local name, a context name, a direction or,
//!   when called, an engine function (M 9.3).
//! * `wait` and `open` belong to commands and coroutine scenes; a `query`
//!   or `present fn` does not change the game's state (M 9.2).

use std::collections::BTreeSet;

use super::ast::*;
use super::sigs::FnKind;
use crate::error::{Diagnostic, Report};
use crate::fields::{DIRS, SIDES};
use crate::model::{Kind, Model};
use crate::names;
use crate::rogue::Decl;

/// An engine function's kind (M 9.2); see [`super::sigs`].
pub fn engine_function(name: &str) -> Option<FnKind> {
    super::sigs::kind_of(name)
}

/// Context names (RS 2.4).
const CONTEXT: &[&str] = &["self", "holder", "actor", "it", "player", "game", "map", "camera", "profile"];

/// Engine component names usable as values in calls (`ai(behavior: …)`).
const ENGINE_COMPONENTS: &[&str] = &[
    "render", "movement", "ai", "sight", "light", "facing", "timer", "interior", "shape", "label", "wire", "blocks", "hearing", "sound",
    "name", "revealed",
];

/// Vocabulary values: `blocks(it, move, …)`, `mode: MISSILE`.
const VALUES: &[&str] = &["move", "sight", "path", "line", "checkpoint", "auto", "all"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ctx {
    Function,
    Query,
    Present,
    Command,
    Handler { before: bool, engine: bool },
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
    locals: BTreeSet<String>,
    /// Locals assigned in the body (not parameters): a query may change them.
    own: BTreeSet<String>,
    where_depth: usize,
    texts: &'a BTreeSet<String>,
    errors: Vec<Diagnostic>,
    tiles: &'a mut BTreeSet<String>,
}

/// Checks every RogueScript body; returns the tiles the logic names.
pub fn check(m: &Model, r: &mut Report) -> BTreeSet<String> {
    let texts: BTreeSet<String> = m.languages.iter().flat_map(|f| f.texts.iter().map(move |t| names::qualify(f.namespace.as_deref(), t.key()))).collect();
    let mut tiles = BTreeSet::new();
    for decl in &m.code {
        let item = decl.item.clone();
        let mut run = |ctx: Ctx, params: Vec<String>, ret: bool, f: &mut dyn FnMut(&mut Body)| {
            let mut b = Body {
                m,
                decl,
                ctx,
                ret,
                locals: params.into_iter().collect(),
                own: BTreeSet::new(),
                where_depth: 0,
                texts: &texts,
                errors: vec![],
                tiles: &mut tiles,
            };
            f(&mut b);
            for e in b.errors {
                r.push(e);
            }
        };
        let names = |ps: &[Param]| ps.iter().map(|p| p.name.clone()).collect::<Vec<_>>();
        match &item.kind {
            ItemKind::Component { fields, handlers } => {
                run(Ctx::Function, vec![], false, &mut |b| b.params(fields));
                for h in handlers {
                    // a component's handlers see its fields by name (RS 3.1)
                    let mut locals = names(fields);
                    locals.extend(h.names.iter().cloned());
                    run(Ctx::Handler { before: h.phase == Phase::Before, engine: names::is_vocabulary(&h.event) }, locals, false, &mut |b| b.handler(h, true));
                }
            }
            ItemKind::Event { params } => run(Ctx::Function, vec![], false, &mut |b| b.params(params)),
            ItemKind::Handler(h) => {
                run(Ctx::Handler { before: h.phase == Phase::Before, engine: names::is_vocabulary(&h.event) }, h.names.clone(), false, &mut |b| b.handler(h, false))
            }
            ItemKind::Function { present, query, params, ret, body } => {
                let ctx = if *query {
                    Ctx::Query
                } else if *present {
                    Ctx::Present
                } else {
                    Ctx::Function
                };
                run(ctx, names(params), ret.is_some(), &mut |b| {
                    b.params(params);
                    b.body(body);
                });
            }
            ItemKind::Command { params, body, .. } => run(Ctx::Command, names(params), true, &mut |b| {
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
            ItemKind::CommandAlias(call) => run(Ctx::Command, vec![], true, &mut |b| {
                if let ExprKind::Call(f, args) = &call.kind
                    && let ExprKind::Name { name, root } = &f.kind
                {
                    b.name(f.pos, name, *root, &[Kind::Command]);
                    for a in args {
                        b.expr(&a.value);
                    }
                }
            }),
            ItemKind::Ai { body } => run(Ctx::Ai, vec![], true, &mut |b| b.body(body)),
            ItemKind::Generator { params, body, .. } => run(Ctx::Generator, names(params), false, &mut |b| {
                b.params(params);
                b.body(body);
            }),
            ItemKind::Map { generate, .. } => run(Ctx::Function, vec![], false, &mut |b| {
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
            ItemKind::Scene { params, body } => run(Ctx::Scene, names(params), false, &mut |b| {
                b.params(params);
                b.body(body);
            }),
            ItemKind::Timeline { params, timeline } => run(Ctx::Timeline, names(params), false, &mut |b| {
                b.params(params);
                for t in &timeline.tracks {
                    b.rows(&t.rows);
                }
                b.rows(&timeline.rows);
            }),
        }
    }
    tiles
}

impl Body<'_> {
    fn error(&mut self, pos: Pos, msg: impl Into<String>) {
        self.errors.push(Diagnostic::error(&self.decl.file, pos.line, format!("column {}: {}", pos.col, msg.into())));
    }

    fn ns(&self) -> Option<&str> {
        self.decl.namespace.as_deref()
    }

    fn params(&mut self, params: &[Param]) {
        for p in params {
            if let Some(d) = &p.default {
                self.expr(d);
            }
            match &p.source {
                Some(Source::From(e)) | Some(Source::Ask(Some(e))) => self.expr(e),
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
        if let Some(w) = &h.when {
            self.expr(w);
        }
        self.body(&h.body);
    }

    fn body(&mut self, body: &[Stmt]) {
        let mut assigned = BTreeSet::new();
        collect_assigned(body, &mut assigned);
        self.own.extend(assigned.iter().cloned());
        self.locals.extend(assigned);
        for s in body {
            self.stmt(s);
        }
    }

    fn stmt(&mut self, s: &Stmt) {
        match &s.kind {
            StmtKind::Assign { target, value, .. } => {
                if self.ctx.pure() {
                    let root = root_word(target);
                    if !root.is_some_and(|r| self.own.contains(r)) {
                        let what = if self.ctx == Ctx::Query { "a `query`" } else { "a `present fn`" };
                        self.error(s.pos, format!("{what} does not change the game's state; `{target}` is not a local name of its own (M 9.2)"));
                    }
                }
                match &target.kind {
                    ExprKind::Word(_) => {}
                    _ => self.expr(target),
                }
                self.expr(value);
            }
            StmtKind::WaitInput { arg, .. } => {
                self.waits(s.pos, "`wait`");
                self.expr(arg);
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
            StmtKind::For { iter, body, .. } => {
                self.expr(iter);
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
                    _ => self.expr(call),
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
                (Some(v), _) => self.expr(v),
                _ => {}
            },
            StmtKind::Cancel => {
                if !matches!(self.ctx, Ctx::Handler { before: true, .. }) {
                    self.error(s.pos, "`cancel` belongs to a `before` handler (RS 4)");
                }
            }
            StmtKind::Call(e) => self.expr(e),
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
            WaitArg::Time(e) | WaitArg::Input { arg: e, .. } => self.expr(e),
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
                        Action::Call(c) => self.expr(c),
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
    fn name(&mut self, pos: Pos, name: &str, root: bool, kinds: &[Kind]) -> Option<Kind> {
        let written = if root { format!(".{name}") } else { name.to_string() };
        match self.m.resolve(self.ns(), &written) {
            Ok(def) => {
                if def.kind == Kind::Tile {
                    self.tiles.insert(def.full.clone());
                }
                if !kinds.is_empty() && !kinds.contains(&def.kind) {
                    let want: Vec<String> = kinds.iter().map(|k| k.to_string()).collect();
                    let msg = format!("`{}` is a {} ({}), not a {}", def.full, def.kind, def.place(), want.join(" or "));
                    self.error(pos, msg);
                }
                Some(def.kind)
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

    fn expr(&mut self, e: &Expr) {
        use ExprKind::*;
        match &e.kind {
            Int(_) | Dice(..) | Time(..) | Percent(_) | Size(..) | Str(_) | Bool(_) | None | All | EmptyDict => {}
            Name { name, root } => {
                self.name(e.pos, name, *root, &[]);
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
            }
            List(items) => {
                for i in items {
                    self.expr(i);
                }
            }
            Field(base, _) | TileIndex(base, _) => self.expr(base),
            Call(f, args) => self.call(e, f, args),
            Subscript(a, b) | Binary(_, a, b) | In(a, b) | Is(a, b) => {
                self.expr(a);
                self.expr(b);
            }
            Neg(a) | Not(a) => self.expr(a),
            Range(a, b) => {
                self.expr(a);
                if let Some(b) = b {
                    self.expr(b);
                }
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
            }
            Where(a, b) => {
                self.expr(a);
                self.where_depth += 1;
                self.expr(b);
                self.where_depth -= 1;
            }
        }
    }

    fn word(&mut self, pos: Pos, w: &str) {
        if w == "it" {
            if self.where_depth == 0 {
                self.error(pos, "`it` is the member inside `where` (RS 5.3)");
            }
            return;
        }
        if self.locals.contains(w) || CONTEXT.contains(&w) || DIRS.contains(&w) || SIDES.contains(&w) || VALUES.contains(&w) {
            return;
        }
        if engine_function(w).is_some() {
            self.error(pos, format!("`{w}` is an engine function: call it with `{w}(…)`"));
            return;
        }
        self.error(pos, format!("unknown name `{w}`: not a parameter, a local name or a context name"));
    }

    fn call(&mut self, e: &Expr, f: &Expr, args: &[Arg]) {
        match &f.kind {
            ExprKind::Name { name, root } => {
                let callable = [Kind::Function, Kind::Query, Kind::Command, Kind::Ai, Kind::Generator, Kind::Scene, Kind::Entity, Kind::Component];
                let kind = self.name(f.pos, name, *root, &callable);
                if self.ctx.pure()
                    && kind == Some(Kind::Function)
                    && let Ok(def) = self.m.resolve(self.ns(), name)
                    && let crate::model::Source::Decl(d) = &def.source
                    && let ItemKind::Function { present, query: false, .. } = &d.item.kind
                    && !(*present && self.ctx == Ctx::Present)
                {
                    self.error(f.pos, format!("`{}` is a `fn` that may change the game's state; a `query` or `present fn` calls only queries (M 9.2)", def.full));
                }
                // named arguments of an entity or component call name components or fields
                for a in args {
                    self.expr(&a.value);
                }
            }
            ExprKind::Word(w) => {
                self.engine_call(f.pos, w, args.first().map(|a| &a.value), args);
                for a in args {
                    if self.is_label_arg(w, a, args) {
                        continue;
                    }
                    self.expr(&a.value);
                }
            }
            ExprKind::Field(recv, w) if names::is_vocabulary(w) => {
                // a chain call `x.f(a)` is `f(x, a)` (RS 2.7)
                self.engine_call(f.pos, w, Some(recv), args);
                self.expr(recv);
                for a in args {
                    self.expr(&a.value);
                }
            }
            _ => {
                self.error(e.pos, format!("`{f}` cannot be called"));
                self.expr(f);
            }
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

    fn engine_call(&mut self, pos: Pos, w: &str, receiver: Option<&Expr>, _args: &[Arg]) {
        if ENGINE_COMPONENTS.contains(&w) && engine_function(w).is_none() {
            // `ai(behavior: FOLLOWER)` is a component value (M 3.4)
            return;
        }
        let Some(kind) = engine_function(w) else {
            if self.locals.contains(w) {
                self.error(pos, format!("`{w}` is a local name and cannot be called"));
            } else {
                self.error(pos, format!("unknown engine function `{w}` (M 9.3)"));
            }
            return;
        };
        if w == "external" && self.ctx != Ctx::Generator {
            self.error(pos, "`external` is allowed only in a generator (M 9.3)");
        }
        if !self.ctx.pure() {
            return;
        }
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

/// The local name at the root of an assignment target (`rooms`, `l[i]`).
fn root_word(e: &Expr) -> Option<&str> {
    match &e.kind {
        ExprKind::Word(w) => Some(w),
        ExprKind::Subscript(a, _) | ExprKind::Field(a, _) => root_word(a),
        _ => None,
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
