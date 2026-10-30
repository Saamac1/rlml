//! RogueScript parser (RS 12): recursive descent, one function per rule.
//!
//! An error stops the declaration it is in; parsing goes on from the next
//! top-level declaration, so one mistake does not hide the others.

use super::ast::*;
use super::lex::{Tok, Token, lex};
use crate::error::Diagnostic;
use crate::syntax::RawCode;

/// Reserved words (RS 2.4) that can never start an expression.
const KEYWORDS: &[&str] = &[
    "component", "event", "on", "before", "after", "as", "priority", "when", "hide", "disabled", "fn", "present", "query", "command",
    "cost", "aim", "generator", "transient", "generate", "size", "scene", "timeline", "background", "realtime", "track", "loop", "mark",
    "always", "if", "elif", "else", "for", "in", "where", "repeat", "while", "break", "match", "choice", "emit", "wait", "confirm",
    "open", "return", "cancel", "has", "no", "is", "and", "or", "not", "namespace", "override", "extends",
];

/// Parses a code block into declarations.
pub fn parse(code: &RawCode) -> (Vec<Item>, Vec<Diagnostic>) {
    let (toks, mut errors) = lex(code);
    if !errors.is_empty() {
        return (vec![], errors);
    }
    let mut p = Parser { toks, i: 0, file: code.file.clone() };
    let mut items = Vec::new();
    while !p.at(&Tok::Eof) {
        if p.eat(&Tok::Newline) {
            continue;
        }
        let start = p.i;
        match p.item() {
            Ok(item) => items.push(item),
            Err(e) => {
                errors.push(e);
                p.recover(start);
            }
        }
    }
    (items, errors)
}

/// Parses a documentation snippet: declarations, statements and timeline
/// rows may all stand at the top level.
pub fn parse_snippet(code: &RawCode) -> Vec<Diagnostic> {
    let (toks, mut errors) = lex(code);
    if !errors.is_empty() {
        return errors;
    }
    let mut p = Parser { toks, i: 0, file: code.file.clone() };
    while !p.at(&Tok::Eof) {
        if p.eat(&Tok::Newline) {
            continue;
        }
        let start = p.i;
        let declaration = matches!(&p.peek().tok, Tok::Word(w) if DECLARATIONS.contains(&w.as_str()));
        let row = matches!(p.peek().tok, Tok::Time(_) | Tok::Op("+")) || (matches!(p.peek().tok, Tok::Int(_)) && !matches!(p.peek_at(1), Tok::Op(_)));
        let res = if declaration {
            p.item().map(|_| ())
        } else if row {
            p.row().map(|_| ()).and_then(|_| {
                // `else:` rows of a `when` block that started the snippet
                if p.eat_word("else") {
                    p.expect_op(":")?;
                    p.end_line()?;
                    p.rows().map(|_| ())
                } else {
                    Ok(())
                }
            })
        } else if p.starts_operand() && !matches!(p.peek_at(1), Tok::Op("=" | "+=" | "-=")) && !matches!(&p.peek().tok, Tok::Word(w) if ["for", "while", "match", "if"].contains(&w.as_str())) && !matches!(p.peek().tok, Tok::Name(_)) {
            // a bare expression
            p.expr().and_then(|_| p.end_line())
        } else {
            p.stmt().map(|_| ())
        };
        if let Err(e) = res {
            errors.push(e);
            p.recover(start);
        }
    }
    errors
}

/// Words that start a top-level declaration.
const DECLARATIONS: &[&str] = &["override", "present", "component", "event", "on", "before", "after", "fn", "query", "command", "ai", "generator", "map", "scene"];

/// Parses one expression, for tests and tools.
pub fn parse_expr(src: &str) -> Result<Expr, Diagnostic> {
    let code = RawCode { file: "expr".into(), line: 1, namespace: None, lines: vec![src.to_string()] };
    let (toks, errors) = lex(&code);
    if let Some(e) = errors.into_iter().next() {
        return Err(e);
    }
    let mut p = Parser { toks, i: 0, file: code.file };
    let e = p.expr()?;
    p.expect(&Tok::Newline, "the end of the expression")?;
    Ok(e)
}

struct Parser {
    toks: Vec<Token>,
    i: usize,
    file: String,
}

type R<T> = Result<T, Diagnostic>;

fn op(s: &'static str) -> Tok {
    Tok::Op(s)
}

impl Parser {
    fn peek(&self) -> &Token {
        &self.toks[self.i]
    }

    fn peek_at(&self, n: usize) -> &Tok {
        &self.toks[(self.i + n).min(self.toks.len() - 1)].tok
    }

    fn pos(&self) -> Pos {
        let t = self.peek();
        Pos { line: t.line, col: t.col }
    }

    fn at(&self, t: &Tok) -> bool {
        &self.peek().tok == t
    }

    fn at_op(&self, s: &str) -> bool {
        matches!(&self.peek().tok, Tok::Op(o) if *o == s)
    }

    fn at_word(&self, w: &str) -> bool {
        matches!(&self.peek().tok, Tok::Word(x) if x == w)
    }

    fn bump(&mut self) -> Token {
        let t = self.toks[self.i].clone();
        if self.i < self.toks.len() - 1 {
            self.i += 1;
        }
        t
    }

    fn eat(&mut self, t: &Tok) -> bool {
        if self.at(t) {
            self.bump();
            true
        } else {
            false
        }
    }

    fn eat_op(&mut self, s: &'static str) -> bool {
        self.eat(&op(s))
    }

    fn eat_word(&mut self, w: &str) -> bool {
        if self.at_word(w) {
            self.bump();
            true
        } else {
            false
        }
    }

    fn err(&self, msg: impl Into<String>) -> Diagnostic {
        let t = self.peek();
        Diagnostic::error(&self.file, t.line, format!("column {}: {}", t.col, msg.into()))
    }

    fn found(&self) -> String {
        match &self.peek().tok {
            Tok::Newline => "the end of the line".into(),
            Tok::Indent => "an indented block".into(),
            Tok::Dedent | Tok::Eof => "the end of the block".into(),
            _ => format!("`{}`", self.peek().text),
        }
    }

    fn expect(&mut self, t: &Tok, what: &str) -> R<()> {
        if self.eat(t) { Ok(()) } else { Err(self.err(format!("expected {what}, found {}", self.found()))) }
    }

    fn expect_op(&mut self, s: &'static str) -> R<()> {
        self.expect(&op(s), &format!("`{s}`"))
    }

    fn expect_word(&mut self, w: &str) -> R<()> {
        self.expect(&Tok::Word(w.into()), &format!("`{w}`"))
    }

    /// Skips to the next top-level declaration after an error.
    fn recover(&mut self, start: usize) {
        if self.i == start {
            self.bump();
        }
        let mut depth: i64 = self.toks[..self.i].iter().fold(0, |d, t| match t.tok {
            Tok::Indent => d + 1,
            Tok::Dedent => d - 1,
            _ => d,
        });
        loop {
            match self.peek().tok {
                Tok::Eof => return,
                Tok::Indent => depth += 1,
                Tok::Dedent => {
                    depth -= 1;
                    if depth <= 0 {
                        self.bump();
                        return;
                    }
                }
                Tok::Newline if depth <= 0 && !matches!(self.peek_at(1), Tok::Indent) => {
                    self.bump();
                    return;
                }
                _ => {}
            }
            self.bump();
        }
    }

    // ---- names and types ----

    /// `NAME {. NAME}`.
    fn qname(&mut self) -> R<String> {
        let Tok::Name(first) = self.peek().tok.clone() else {
            return Err(self.err(format!("expected an upper-case NAME, found {}", self.found())));
        };
        self.bump();
        let mut name = first;
        while self.at_op(".") && matches!(self.peek_at(1), Tok::Name(_)) {
            self.bump();
            if let Tok::Name(n) = self.bump().tok {
                name.push('.');
                name.push_str(&n);
            }
        }
        Ok(name)
    }

    /// A lower-case word that is not a reserved keyword.
    fn local(&mut self, what: &str) -> R<String> {
        match self.peek().tok.clone() {
            Tok::Word(w) if !KEYWORDS.contains(&w.as_str()) => {
                self.bump();
                Ok(w)
            }
            Tok::Word(w) => Err(self.err(format!("`{w}` is a reserved word and cannot be {what} (RS 2.4)"))),
            _ => Err(self.err(format!("expected {what}, found {}", self.found()))),
        }
    }

    fn int(&mut self) -> R<i64> {
        let neg = self.eat_op("-");
        match self.peek().tok {
            Tok::Int(n) => {
                self.bump();
                Ok(if neg { -n } else { n })
            }
            _ => Err(self.err(format!("expected a number, found {}", self.found()))),
        }
    }

    /// A type (RS 12.1 `type` and `fieldtype`).
    fn ty(&mut self) -> R<Type> {
        let base = match self.peek().tok.clone() {
            Tok::Word(w) => {
                self.bump();
                w
            }
            Tok::Name(_) => self.qname()?,
            _ => return Err(self.err(format!("expected a type, found {}", self.found()))),
        };
        let mut t = Type { base: base.clone(), args: vec![], extra: vec![] };
        let word = |p: &mut Parser, w: &str, t: &mut Type| {
            if p.eat_word(w) {
                t.extra.push(w.to_string());
                true
            } else {
                false
            }
        };
        match base.as_str() {
            "deck" => {
                word(self, "copy", &mut t);
            }
            "collection" => {
                if !word(self, "card", &mut t) {
                    word(self, "name", &mut t);
                }
                if let Tok::Int(n) = self.peek().tok {
                    self.bump();
                    t.extra.push(n.to_string());
                }
                if word(self, "merge", &mut t) {
                    t.extra.push(self.qname()?);
                }
                word(self, "modifies", &mut t);
            }
            "list" => {
                t.args.push(self.ty()?);
                if let Tok::Int(n) = self.peek().tok {
                    self.bump();
                    t.extra.push(n.to_string());
                }
                if word(self, "sort", &mut t) {
                    if let Tok::Word(w) = self.peek().tok.clone()
                        && w != "desc"
                    {
                        self.bump();
                        t.extra.push(w);
                    }
                    word(self, "desc", &mut t);
                }
            }
            "dict" => {
                let key = self.ty()?;
                if !["entity", "name", "number"].contains(&key.base.as_str()) {
                    return Err(self.err(format!("a dictionary key is entity, name or number, not `{}`", key.base)));
                }
                t.args.push(key);
                t.args.push(self.ty()?);
            }
            _ => {}
        }
        Ok(t)
    }

    /// `name: type [= expr] [source]`, comma separated, up to `)`.
    fn params(&mut self) -> R<Vec<Param>> {
        self.expect_op("(")?;
        let mut out = Vec::new();
        while !self.at_op(")") {
            let pos = self.pos();
            let name = match self.peek().tok.clone() {
                // indexed fields are names (`FIRE: percent`)
                Tok::Name(n) => {
                    self.bump();
                    n
                }
                _ => self.local("a parameter name")?,
            };
            self.expect_op(":")?;
            let ty = self.ty()?;
            let default = if self.eat_op("=") { Some(self.expr()?) } else { None };
            let source = self.source()?;
            out.push(Param { name, ty, default, source, pos });
            if !self.eat_op(",") {
                break;
            }
        }
        self.expect_op(")")?;
        Ok(out)
    }

    /// `from expr`, `aim expr [at expr]`, `ask [expr]` or `count` (RS 7.1).
    fn source(&mut self) -> R<Option<Source>> {
        Ok(Some(if self.eat_word("from") {
            Source::From(self.expr()?)
        } else if self.eat_word("aim") {
            let range = self.expr()?;
            let at = if self.eat_word("at") { Some(self.expr()?) } else { None };
            Source::Aim { range, at }
        } else if self.eat_word("ask") {
            Source::Ask(if self.at_op(",") || self.at_op(")") { None } else { Some(self.expr()?) })
        } else if self.eat_word("count") {
            Source::Count
        } else {
            return Ok(None);
        }))
    }

    // ---- declarations ----

    fn item(&mut self) -> R<Item> {
        let pos = self.pos();
        let overrides = self.eat_word("override");
        let present = self.eat_word("present");
        let Tok::Word(kw) = self.peek().tok.clone() else {
            return Err(self.err(format!("expected a declaration, found {}", self.found())));
        };
        if kw == "namespace" {
            return Err(self.err("`namespace` is written at the start of the file, outside the code block; a code block holds only declarations (RS 2.1, RS 12.1)"));
        }
        if present && kw != "fn" {
            return Err(self.err("`present` is written before `fn`"));
        }
        self.bump();
        let name;
        let kind = match kw.as_str() {
            "component" => {
                name = self.qname()?;
                let fields = if self.at_op("(") { self.params()? } else { vec![] };
                let mut handlers = Vec::new();
                if self.eat_op(":") {
                    self.expect(&Tok::Newline, "the end of the line")?;
                    self.expect(&Tok::Indent, "the component's handlers")?;
                    while !self.eat(&Tok::Dedent) {
                        let Tok::Word(phase) = self.peek().tok.clone() else {
                            return Err(self.err(format!("expected a handler (`on`, `before` or `after`), found {}", self.found())));
                        };
                        self.bump();
                        handlers.push(self.handler(&phase)?);
                    }
                } else {
                    self.end_line()?;
                }
                ItemKind::Component { fields, handlers }
            }
            "event" => {
                name = self.qname()?;
                let params = self.params()?;
                self.end_line()?;
                ItemKind::Event { params }
            }
            "on" | "before" | "after" => {
                let h = self.handler(&kw)?;
                name = h.name.clone().unwrap_or_default();
                ItemKind::Handler(h)
            }
            "fn" | "query" => {
                name = self.qname()?;
                let params = self.params()?;
                let ret = if self.eat_op("->") { Some(self.return_type()?) } else { None };
                if kw == "query" && ret.is_none() {
                    return Err(self.err("a `query` needs a return type: `-> type` (RS 5.1)"));
                }
                let body = self.block()?;
                ItemKind::Function { present, query: kw == "query", params, ret, body }
            }
            "command" => {
                name = self.qname()?;
                if self.eat_op("=") {
                    let call = self.expr()?;
                    if !matches!(call.kind, ExprKind::Call(..)) {
                        return Err(self.err("`command NAME =` takes a call, such as `MOVE(north)`"));
                    }
                    self.end_line()?;
                    ItemKind::CommandAlias(call)
                } else {
                    let params = self.params()?;
                    let cost = if self.eat_word("cost") { Some(self.int()?) } else { None };
                    ItemKind::Command { params, cost, body: self.block()? }
                }
            }
            "ai" => {
                name = self.qname()?;
                ItemKind::Ai { body: self.block()? }
            }
            "generator" => {
                name = self.qname()?;
                let params = self.params()?;
                let size = if self.eat_word("size") {
                    match self.bump().tok {
                        Tok::Size(w, h) => Some((w, h)),
                        _ => return Err(self.err("`size` takes `WxH`, such as `size 80x40`")),
                    }
                } else {
                    None
                };
                ItemKind::Generator { params, size, body: self.block()? }
            }
            "map" => {
                name = self.qname()?;
                let (mut transient, mut wrap, mut nosave) = (false, false, false);
                loop {
                    if self.eat_word("transient") {
                        transient = true;
                    } else if self.eat_word("wrap") {
                        wrap = true;
                    } else if self.eat_word("nosave") {
                        nosave = true;
                    } else {
                        break;
                    }
                }
                self.expect_word("generate")?;
                let generate = self.expr()?;
                self.end_line()?;
                ItemKind::Map { transient, wrap, nosave, generate }
            }
            "scene" => {
                name = self.qname()?;
                let params = self.params()?;
                if self.eat_word("timeline") {
                    let background = self.eat_word("background");
                    let realtime = background && self.eat_word("realtime");
                    self.expect_op(":")?;
                    self.expect(&Tok::Newline, "the end of the line")?;
                    ItemKind::Timeline { params, timeline: self.timeline(background, realtime)? }
                } else {
                    ItemKind::Scene { params, body: self.block()? }
                }
            }
            w => return Err(Diagnostic::error(&self.file, pos.line, format!("column {}: expected a declaration, found `{w}`", pos.col))),
        };
        Ok(Item { name, overrides, kind, pos })
    }

    fn return_type(&mut self) -> R<ReturnType> {
        if self.at_word("name") && matches!(self.peek_at(1), Tok::Op("|")) {
            self.bump();
            self.bump();
            self.expect_word("text")?;
            return Ok(ReturnType::NameOrText);
        }
        Ok(ReturnType::Type(self.ty()?))
    }

    /// After the phase word: `EVENT(names) [as NAME] [priority N] [when expr]: block`.
    fn handler(&mut self, phase: &str) -> R<Handler> {
        let pos = self.pos();
        let phase = match phase {
            "on" => Phase::On,
            "before" => Phase::Before,
            "after" => Phase::After,
            w => return Err(self.err(format!("expected `on`, `before` or `after`, found `{w}`"))),
        };
        let event = match self.peek().tok.clone() {
            Tok::Word(w) => {
                self.bump();
                w
            }
            _ => self.qname()?,
        };
        self.expect_op("(")?;
        let mut names = Vec::new();
        while !self.at_op(")") {
            // a parameter that names the same thing as a context name is allowed (RS 2.4)
            match self.peek().tok.clone() {
                Tok::Word(w) => {
                    self.bump();
                    names.push(w);
                }
                _ => return Err(self.err(format!("expected a parameter name, found {}", self.found()))),
            }
            if !self.eat_op(",") {
                break;
            }
        }
        self.expect_op(")")?;
        let name = if self.eat_word("as") { Some(self.qname()?) } else { None };
        let priority = if self.eat_word("priority") { self.int()? } else { 0 };
        let when = if self.eat_word("when") { Some(self.expr()?) } else { None };
        let body = self.block()?;
        Ok(Handler { phase, event, names, name, priority, when, body, pos })
    }

    fn end_line(&mut self) -> R<()> {
        self.expect(&Tok::Newline, "the end of the line")
    }

    // ---- statements ----

    /// `: NL INDENT stmt {stmt} DEDENT`.
    fn block(&mut self) -> R<Vec<Stmt>> {
        self.expect_op(":")?;
        self.end_line()?;
        if !self.eat(&Tok::Indent) {
            return Err(self.err("expected an indented block (four more spaces)"));
        }
        let mut out = Vec::new();
        while !self.eat(&Tok::Dedent) {
            if self.at(&Tok::Eof) {
                break;
            }
            out.push(self.stmt()?);
        }
        Ok(out)
    }

    fn stmt(&mut self) -> R<Stmt> {
        let pos = self.pos();
        let kind = match self.peek().tok.clone() {
            Tok::Word(w) => match w.as_str() {
                "if" => {
                    self.bump();
                    let mut arms = vec![(self.expr()?, self.block()?)];
                    let mut otherwise = None;
                    loop {
                        if self.eat_word("elif") {
                            arms.push((self.expr()?, self.block()?));
                        } else if self.eat_word("else") {
                            otherwise = Some(self.block()?);
                            break;
                        } else {
                            break;
                        }
                    }
                    StmtKind::If { arms, otherwise }
                }
                "for" => {
                    self.bump();
                    let var = self.local("a loop variable")?;
                    self.expect_word("in")?;
                    let iter = self.expr()?;
                    StmtKind::For { var, iter, body: self.block()? }
                }
                "repeat" => {
                    self.bump();
                    let count = self.expr()?;
                    StmtKind::Repeat { count, body: self.block()? }
                }
                "while" => {
                    self.bump();
                    let cond = self.expr()?;
                    StmtKind::While { cond, body: self.block()? }
                }
                "break" => {
                    self.bump();
                    self.end_line()?;
                    StmtKind::Break
                }
                "cancel" => {
                    self.bump();
                    self.end_line()?;
                    StmtKind::Cancel
                }
                "return" => {
                    self.bump();
                    let value = if self.at(&Tok::Newline) { None } else { Some(self.expr()?) };
                    self.end_line()?;
                    StmtKind::Return(value)
                }
                "match" => {
                    self.bump();
                    let subject = self.expr()?;
                    self.expect_op(":")?;
                    self.end_line()?;
                    self.expect(&Tok::Indent, "the cases")?;
                    let mut cases = Vec::new();
                    while !self.eat(&Tok::Dedent) {
                        let case = if self.eat_word("else") { None } else { Some(self.expr()?) };
                        cases.push((case, self.block()?));
                    }
                    StmtKind::Match { subject, cases }
                }
                "choice" => {
                    self.bump();
                    self.expect_op(":")?;
                    self.end_line()?;
                    self.expect(&Tok::Indent, "the options")?;
                    let mut options = Vec::new();
                    while !self.eat(&Tok::Dedent) {
                        options.push(self.option()?);
                    }
                    StmtKind::Choice(options)
                }
                "emit" => {
                    self.bump();
                    let call = self.call("`emit`")?;
                    self.end_line()?;
                    StmtKind::Emit(call)
                }
                "wait" => {
                    self.bump();
                    let arg = self.wait_arg()?;
                    self.end_line()?;
                    StmtKind::Wait(arg)
                }
                "open" => {
                    self.bump();
                    let call = self.expr()?;
                    self.end_line()?;
                    StmtKind::Open(call)
                }
                _ => self.simple()?,
            },
            _ => self.simple()?,
        };
        Ok(Stmt { kind, pos })
    }

    /// An assignment or a call.
    fn simple(&mut self) -> R<StmtKind> {
        let target = self.expr()?;
        let op = if self.eat_op("=") {
            AssignOp::Set
        } else if self.eat_op("+=") {
            AssignOp::Add
        } else if self.eat_op("-=") {
            AssignOp::Sub
        } else {
            if !matches!(target.kind, ExprKind::Call(..)) {
                return Err(self.err(format!("a statement is a call or an assignment; `{target}` is neither")));
            }
            self.end_line()?;
            return Ok(StmtKind::Call(target));
        };
        if !matches!(target.kind, ExprKind::Word(_) | ExprKind::Field(..) | ExprKind::Subscript(..)) {
            return Err(Diagnostic::error(&self.file, target.pos.line, format!("column {}: cannot assign to `{target}`", target.pos.col)));
        }
        if let ExprKind::Word(w) = &target.kind
            && KEYWORDS.contains(&w.as_str())
        {
            return Err(self.err(format!("`{w}` is a reserved word and cannot be a local name (RS 2.4)")));
        }
        // `name = wait number(…)` and `name = open MENU(…)` (RS 12.2)
        if op == AssignOp::Set
            && let ExprKind::Word(name) = &target.kind
        {
            if self.eat_word("wait") {
                let arg = self.wait_arg()?;
                self.end_line()?;
                return match arg {
                    WaitArg::Input { kind, arg } => Ok(StmtKind::WaitInput { name: name.clone(), kind, arg }),
                    _ => Err(self.err("only `wait number(…)` and `wait text(…)` give a value")),
                };
            }
            if self.eat_word("open") {
                let call = self.expr()?;
                self.end_line()?;
                return Ok(StmtKind::OpenValue { name: name.clone(), call });
            }
        }
        let value = self.expr()?;
        self.end_line()?;
        Ok(StmtKind::Assign { target, op, value })
    }

    fn call(&mut self, what: &str) -> R<Expr> {
        let e = self.expr()?;
        if matches!(e.kind, ExprKind::Call(..)) {
            Ok(e)
        } else {
            Err(Diagnostic::error(&self.file, e.pos.line, format!("column {}: {what} takes a call, not `{e}`", e.pos.col)))
        }
    }

    /// `expr | confirm | scene CALL | (number|text)(expr)`.
    fn wait_arg(&mut self) -> R<WaitArg> {
        if self.eat_word("confirm") {
            return Ok(WaitArg::Confirm);
        }
        if self.eat_word("scene") {
            return Ok(WaitArg::Scene(self.call("`wait scene`")?));
        }
        for kind in ["number", "text"] {
            if self.at_word(kind) && matches!(self.peek_at(1), Tok::Op("(")) {
                self.bump();
                self.bump();
                let arg = self.expr()?;
                self.expect_op(")")?;
                return Ok(WaitArg::Input { kind: kind.into(), arg });
            }
        }
        Ok(WaitArg::Time(self.expr()?))
    }

    /// `NAME [(args)] [hide when e] [disabled when e] (: block | adjust …)`.
    fn option(&mut self) -> R<ChoiceOption> {
        let pos = self.pos();
        let name = self.qname()?;
        let args = if self.eat_op("(") { self.args()? } else { vec![] };
        let hide = if self.eat_word("hide") {
            self.expect_word("when")?;
            Some(self.expr()?)
        } else {
            None
        };
        let disabled = if self.eat_word("disabled") {
            self.expect_word("when")?;
            Some(self.expr()?)
        } else {
            None
        };
        let body = if self.eat_word("adjust") {
            let target = self.postfix()?;
            let range = self.range()?;
            let pool = if self.eat_word("pool") { Some(self.postfix()?) } else { None };
            self.end_line()?;
            OptionBody::Adjust { target, range, pool }
        } else {
            OptionBody::Block(self.block()?)
        };
        Ok(ChoiceOption { name, args, hide, disabled, body, pos })
    }

    // ---- timelines (RS 12.4) ----

    fn timeline(&mut self, background: bool, realtime: bool) -> R<Timeline> {
        let mut t = Timeline { background, realtime, tracks: vec![], rows: vec![] };
        self.expect(&Tok::Indent, "the timeline's rows")?;
        while !self.eat(&Tok::Dedent) {
            if self.at_word("track") {
                let pos = self.pos();
                self.bump();
                let name = self.qname()?;
                let looped = if self.eat_word("loop") { Some(self.span()?) } else { None };
                self.expect_op(":")?;
                self.end_line()?;
                t.tracks.push(Track { name, looped, rows: self.rows()?, pos });
            } else {
                t.rows.push(self.row()?);
            }
        }
        Ok(t)
    }

    /// `INDENT row {row} DEDENT`.
    fn rows(&mut self) -> R<Vec<Row>> {
        self.expect(&Tok::Indent, "indented rows")?;
        let mut rows = Vec::new();
        while !self.eat(&Tok::Dedent) {
            rows.push(self.row()?);
        }
        Ok(rows)
    }

    fn span(&mut self) -> R<Expr> {
        let pos = self.pos();
        let t = self.bump();
        let kind = match t.tok {
            Tok::Time(ms) => ExprKind::Time(ms, t.text),
            Tok::Int(n) => ExprKind::Int(n),
            _ => return Err(Diagnostic::error(&self.file, t.line, format!("column {}: expected a time such as `500ms` or a number of rounds", t.col))),
        };
        Ok(Expr { kind, pos })
    }

    fn at_span(&self) -> bool {
        matches!(self.peek().tok, Tok::Time(_) | Tok::Int(_))
    }

    /// `span | + span | MARK [(+|-) span]`, if the row starts with one. A
    /// name followed by `(` or `.` is a call, not a mark.
    fn time(&mut self) -> R<Option<Time>> {
        if self.at_span() {
            return Ok(Some(Time::At(self.span()?)));
        }
        if self.eat_op("+") {
            return Ok(Some(Time::After(self.span()?)));
        }
        if let Tok::Name(n) = self.peek().tok.clone()
            && !matches!(self.peek_at(1), Tok::Op("(" | "."))
        {
            self.bump();
            let offset = if self.at_op("+") || self.at_op("-") {
                let plus = self.bump().tok == op("+");
                Some((plus, self.span()?))
            } else {
                None
            };
            return Ok(Some(Time::Mark { name: n, offset }));
        }
        Ok(None)
    }

    fn row(&mut self) -> R<Row> {
        let pos = self.pos();
        let time = self.time()?;
        if self.eat_word("mark") {
            let Some(time) = time else { return Err(self.err("`mark` needs a time before it")) };
            let name = self.qname()?;
            self.end_line()?;
            return Ok(Row::Mark { time, name, pos });
        }
        let span = if time.is_some() && self.at_span() { Some(self.span()?) } else { None };
        let always = self.eat_word("always");
        let mut when = None;
        if self.eat_word("when") {
            let cond = self.expr()?;
            self.expect_op(":")?;
            if self.eat(&Tok::Newline) {
                // `when expr:` with indented rows and an optional `else:`
                let rows = self.rows()?;
                let otherwise = if self.eat_word("else") {
                    self.expect_op(":")?;
                    self.end_line()?;
                    self.rows()?
                } else {
                    vec![]
                };
                return Ok(Row::When { time, cond, rows, otherwise, pos });
            }
            when = Some(cond);
        }
        let action = if self.eat_word("wait") { Action::Wait(self.wait_arg()?) } else { Action::Call(self.call("a timeline row")?) };
        self.end_line()?;
        Ok(Row::Action { time, span, always, when, action, pos })
    }

    // ---- expressions (RS 12.3) ----

    pub fn expr(&mut self) -> R<Expr> {
        let mut e = self.or()?;
        while self.eat_word("where") {
            let cond = self.or()?;
            e = Expr { pos: e.pos, kind: ExprKind::Where(Box::new(e), Box::new(cond)) };
        }
        Ok(e)
    }

    fn or(&mut self) -> R<Expr> {
        let mut e = self.and()?;
        while self.eat_word("or") {
            let b = self.and()?;
            e = Expr { pos: e.pos, kind: ExprKind::Binary(BinOp::Or, Box::new(e), Box::new(b)) };
        }
        Ok(e)
    }

    fn and(&mut self) -> R<Expr> {
        let mut e = self.not()?;
        while self.eat_word("and") {
            let b = self.not()?;
            e = Expr { pos: e.pos, kind: ExprKind::Binary(BinOp::And, Box::new(e), Box::new(b)) };
        }
        Ok(e)
    }

    fn not(&mut self) -> R<Expr> {
        let pos = self.pos();
        if self.eat_word("not") {
            let e = self.test()?;
            return Ok(Expr { pos, kind: ExprKind::Not(Box::new(e)) });
        }
        self.test()
    }

    fn test(&mut self) -> R<Expr> {
        let a = self.range()?;
        let pos = a.pos;
        let cmp = [("==", BinOp::Eq), ("!=", BinOp::Ne), ("<=", BinOp::Le), (">=", BinOp::Ge), ("<", BinOp::Lt), (">", BinOp::Gt)];
        for (s, o) in cmp {
            if self.eat_op(s) {
                let b = self.range()?;
                return Ok(Expr { pos, kind: ExprKind::Binary(o, Box::new(a), Box::new(b)) });
            }
        }
        if self.eat_word("has") {
            let no = self.eat_word("no");
            let component = match self.peek().tok.clone() {
                Tok::Word(w) => {
                    self.bump();
                    w
                }
                Tok::Name(_) => self.qname()?,
                _ => return Err(self.err(format!("`has` takes a component, found {}", self.found()))),
            };
            return Ok(Expr { pos, kind: ExprKind::Has { subject: Box::new(a), no, component } });
        }
        if self.eat_word("is") {
            let b = self.postfix()?;
            return Ok(Expr { pos, kind: ExprKind::Is(Box::new(a), Box::new(b)) });
        }
        if self.eat_word("in") {
            let b = self.range()?;
            return Ok(Expr { pos, kind: ExprKind::In(Box::new(a), Box::new(b)) });
        }
        Ok(a)
    }

    /// Can the current token start an operand?
    fn starts_operand(&self) -> bool {
        match &self.peek().tok {
            Tok::Word(w) => !KEYWORDS.contains(&w.as_str()) || w == "not",
            Tok::Op(o) => ["(", "[", "{", "-", "."].contains(o),
            Tok::Newline | Tok::Indent | Tok::Dedent | Tok::Eof => false,
            _ => true,
        }
    }

    fn range(&mut self) -> R<Expr> {
        let a = self.sum()?;
        if self.eat_op("..") {
            let pos = a.pos;
            let b = if self.starts_operand() { Some(Box::new(self.sum()?)) } else { None };
            return Ok(Expr { pos, kind: ExprKind::Range(Box::new(a), b) });
        }
        Ok(a)
    }

    fn sum(&mut self) -> R<Expr> {
        let mut e = self.term()?;
        loop {
            let o = if self.eat_op("+") {
                BinOp::Add
            } else if self.eat_op("-") {
                BinOp::Sub
            } else {
                return Ok(e);
            };
            let b = self.term()?;
            e = Expr { pos: e.pos, kind: ExprKind::Binary(o, Box::new(e), Box::new(b)) };
        }
    }

    fn term(&mut self) -> R<Expr> {
        let mut e = self.unary()?;
        loop {
            let o = if self.eat_op("*") {
                BinOp::Mul
            } else if self.eat_op("/") {
                BinOp::Div
            } else if self.eat_op("%") {
                BinOp::Rem
            } else {
                return Ok(e);
            };
            let b = self.unary()?;
            e = Expr { pos: e.pos, kind: ExprKind::Binary(o, Box::new(e), Box::new(b)) };
        }
    }

    fn unary(&mut self) -> R<Expr> {
        let pos = self.pos();
        if self.eat_op("-") {
            let e = self.postfix()?;
            return Ok(Expr { pos, kind: ExprKind::Neg(Box::new(e)) });
        }
        self.postfix()
    }

    fn postfix(&mut self) -> R<Expr> {
        let mut e = self.atom()?;
        loop {
            if self.at_op(".") {
                match self.peek_at(1).clone() {
                    Tok::Word(w) => {
                        self.bump();
                        self.bump();
                        e = Expr { pos: e.pos, kind: ExprKind::Field(Box::new(e), w) };
                    }
                    Tok::Name(n) => {
                        self.bump();
                        self.bump();
                        e = Expr { pos: e.pos, kind: ExprKind::Field(Box::new(e), n) };
                    }
                    Tok::Int(i) => {
                        self.bump();
                        self.bump();
                        e = Expr { pos: e.pos, kind: ExprKind::TileIndex(Box::new(e), i) };
                    }
                    _ => return Ok(e),
                }
            } else if self.eat_op("(") {
                let args = self.args()?;
                e = Expr { pos: e.pos, kind: ExprKind::Call(Box::new(e), args) };
            } else if self.eat_op("[") {
                let i = self.expr()?;
                self.expect_op("]")?;
                e = Expr { pos: e.pos, kind: ExprKind::Subscript(Box::new(e), Box::new(i)) };
            } else if let Tok::Wild | Tok::WildDeep = self.peek().tok {
                // a wildcard after a field path: `map.LOOT.*` is not valid
                return Err(self.err("a wildcard follows an upper-case name directly, such as `ITEM.*`"));
            } else {
                return Ok(e);
            }
        }
    }

    /// After `(`: arguments up to `)`.
    fn args(&mut self) -> R<Vec<Arg>> {
        let mut out = Vec::new();
        while !self.at_op(")") {
            let name = match (self.peek().tok.clone(), self.peek_at(1)) {
                (Tok::Word(n) | Tok::Name(n), Tok::Op(":")) => {
                    self.bump();
                    self.bump();
                    Some(n)
                }
                _ => None,
            };
            let value = self.expr()?;
            if name.is_none() && out.iter().any(|a: &Arg| a.name.is_some()) {
                return Err(Diagnostic::error(&self.file, value.pos.line, format!("column {}: positional arguments come before named ones (RS 2.5)", value.pos.col)));
            }
            out.push(Arg { name, value });
            if !self.eat_op(",") {
                break;
            }
        }
        self.expect_op(")")?;
        Ok(out)
    }

    fn atom(&mut self) -> R<Expr> {
        let pos = self.pos();
        let t = self.peek().clone();
        let lit = |kind| Ok(Expr { kind, pos });
        match t.tok {
            Tok::Int(n) => {
                self.bump();
                lit(ExprKind::Int(n))
            }
            Tok::Dice(n, s) => {
                self.bump();
                lit(ExprKind::Dice(n, s))
            }
            Tok::Time(ms) => {
                self.bump();
                lit(ExprKind::Time(ms, t.text))
            }
            Tok::Percent(p) => {
                self.bump();
                lit(ExprKind::Percent(p))
            }
            Tok::Size(w, h) => {
                self.bump();
                lit(ExprKind::Size(w, h))
            }
            Tok::Str(s) => {
                self.bump();
                lit(ExprKind::Str(s))
            }
            Tok::Word(w) => {
                let kind = match w.as_str() {
                    "true" => ExprKind::Bool(true),
                    "false" => ExprKind::Bool(false),
                    "none" => ExprKind::None,
                    "all" => ExprKind::All,
                    k if KEYWORDS.contains(&k) => return Err(self.err(format!("expected a value, found the keyword `{k}`"))),
                    _ => ExprKind::Word(w),
                };
                self.bump();
                lit(kind)
            }
            Tok::Name(_) => self.name_atom(false, pos),
            Tok::Op(".") if matches!(self.peek_at(1), Tok::Name(_)) => {
                self.bump();
                self.name_atom(true, pos)
            }
            Tok::Op("(") => {
                self.bump();
                let e = self.expr()?;
                self.expect_op(")")?;
                Ok(e)
            }
            Tok::Op("[") => {
                self.bump();
                let mut items = Vec::new();
                while !self.at_op("]") {
                    items.push(self.expr()?);
                    if !self.eat_op(",") {
                        break;
                    }
                }
                self.expect_op("]")?;
                lit(ExprKind::List(items))
            }
            Tok::Op("{") => {
                self.bump();
                if !self.eat_op("}") {
                    return Err(self.err("only the empty dictionary `{}` can be written (RS 2.6)"));
                }
                lit(ExprKind::EmptyDict)
            }
            _ => Err(self.err(format!("expected a value, found {}", self.found()))),
        }
    }

    /// `NAME {. NAME}` with an optional wildcard right after it.
    fn name_atom(&mut self, root: bool, pos: Pos) -> R<Expr> {
        let name = self.qname()?;
        if self.eat(&Tok::Wild) {
            return Ok(Expr { pos, kind: ExprKind::Wild { name, root, deep: false } });
        }
        if self.eat(&Tok::WildDeep) {
            return Ok(Expr { pos, kind: ExprKind::Wild { name, root, deep: true } });
        }
        Ok(Expr { pos, kind: ExprKind::Name { name, root } })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn e(src: &str) -> String {
        parse_expr(src).unwrap_or_else(|d| panic!("{src}: {d}")).to_string()
    }

    fn items(src: &str) -> Vec<Item> {
        let code = RawCode { file: "t".into(), line: 1, namespace: None, lines: src.lines().map(String::from).collect() };
        let (items, errors) = parse(&code);
        assert!(errors.is_empty(), "{errors:#?}");
        items
    }

    fn errs(src: &str) -> Vec<String> {
        let code = RawCode { file: "t".into(), line: 1, namespace: None, lines: src.lines().map(String::from).collect() };
        parse(&code).1.into_iter().map(|d| d.to_string()).collect()
    }

    #[test]
    fn precedence() {
        assert_eq!(e("a or b and not c"), "(a or (b and (not c)))");
        assert_eq!(e("1 + 2 * 3 - 4"), "((1 + (2 * 3)) - 4)");
        assert_eq!(e("x < y + 1"), "(x < (y + 1))");
        assert_eq!(e("-1..3"), "(-1..3)");
        assert_eq!(e("0..n - 1"), "(0..(n - 1))");
        assert_eq!(e("d in 5.."), "(d in (5..))");
        assert_eq!(e("not target in adjacent(actor)"), "(not (target in adjacent(actor)))");
        assert_eq!(e("a where x where y"), "((a where x) where y)");
        assert_eq!(e("TOLERANCE - max(d, -d)"), "(TOLERANCE - max(d, -d))");
    }

    #[test]
    fn tests_and_postfix() {
        assert_eq!(e("e has no FACTION or game has no FACTIONS"), "((e has no FACTION) or (game has no FACTIONS))");
        assert_eq!(e("self has ai"), "(self has ai)");
        assert_eq!(e("it is e.FACTION.group"), "(it is e.FACTION.group)");
        assert_eq!(e("ga.STANCE.toward[b.FACTION.group]"), "ga.STANCE.toward[b.FACTION.group]");
        assert_eq!(e("(entities(in: to) where it != who).first"), "(entities(in: to) where (it != who)).first");
        assert_eq!(e("LOOT.RARE.copy().shuffle().draw(3)"), "LOOT.RARE.copy().shuffle().draw(3)");
        assert_eq!(e("ITEM.*.pick(18)"), "ITEM.*.pick(18)");
        assert_eq!(e("GLYPH.NKI.1..20"), "(GLYPH.NKI.1..20)");
        assert_eq!(e(".LOOT"), ".LOOT");
        assert_eq!(e("RING_MAIL(ENCHANT: 2)"), "RING_MAIL(ENCHANT: 2)");
        assert_eq!(e("[a, 1d20, 30ms, 50%, \"x\", {}]"), "[a, 1d20, 30ms, 50%, \"x\", {}]");
        assert_eq!(e("GATE.open"), "GATE.open");
    }

    #[test]
    fn expression_errors() {
        assert!(parse_expr("a +").unwrap_err().message.contains("expected a value"));
        assert!(parse_expr("f(x: 1, 2)").unwrap_err().message.contains("positional arguments come before"));
        assert!(parse_expr("{a}").unwrap_err().message.contains("empty dictionary"));
        assert!(parse_expr("if").unwrap_err().message.contains("keyword `if`"));
    }

    #[test]
    fn declarations() {
        let it = items(
            "event HIT(target: entity, attacker: entity, amount: number, cause: name = none)\n\
             component FOLLOW(target: entity = none, distance: number = 1)\n\
             command MOVE.NORTH = MOVE(north)\n\
             query CAN_STEP(who: entity, to: pos) -> bool:\n    return to in adjacent(who)\n\
             fn ROOMS(count: range, area: area = tiles()) -> list area:\n    rooms = []\n    return rooms\n\
             map DUNGEON transient generate CAVE(depth: 2)\n\
             generator CAVE(depth: number) size 80x40:\n    carve(tiles(), WALL)\n",
        );
        let names: Vec<&str> = it.iter().map(|i| i.name.as_str()).collect();
        assert_eq!(names, ["HIT", "FOLLOW", "MOVE.NORTH", "CAN_STEP", "ROOMS", "DUNGEON", "CAVE"]);
        let ItemKind::Event { params } = &it[0].kind else { panic!() };
        assert_eq!(params[3].default.as_ref().unwrap().to_string(), "none");
        let ItemKind::Function { ret: Some(r), .. } = &it[4].kind else { panic!() };
        assert_eq!(r.to_string(), "list area");
    }

    #[test]
    fn commands_with_sources() {
        let it = items(
            "command FEED(item: entity from actor.PACK.items where it has FOOD, target: pos aim 1 at entities(in: adjacent(actor))) cost 120:\n    destroy(item)\n",
        );
        let ItemKind::Command { params, cost, .. } = &it[0].kind else { panic!() };
        assert_eq!(*cost, Some(120));
        assert!(matches!(&params[0].source, Some(Source::From(e)) if e.to_string() == "(actor.PACK.items where (it has FOOD))"));
        assert!(matches!(&params[1].source, Some(Source::Aim { at: Some(_), .. })));
    }

    #[test]
    fn handlers_and_component_handlers() {
        let it = items(
            "after HIT(target, attacker, amount) as HIT_MESSAGE priority -10 when WITNESSED(target):\n    log(MESSAGE.COMBAT.HURT, AMOUNT: amount)\n\
             on arrive(who, map, p) as ADJACENCY_ARRIVE:\n    MEET(who, NEIGHBOURS(who, p))\n\
             component POISONED(turns: number = 5, damage: dice = 1d2):\n    on turn():\n        turns -= 1\n",
        );
        let ItemKind::Handler(h) = &it[0].kind else { panic!() };
        assert_eq!((h.event.as_str(), h.name.as_deref(), h.priority, h.names.len()), ("HIT", Some("HIT_MESSAGE"), -10, 3));
        let ItemKind::Handler(h) = &it[1].kind else { panic!() };
        assert_eq!(h.names, ["who", "map", "p"], "context names as event parameters");
        let ItemKind::Component { handlers, .. } = &it[2].kind else { panic!() };
        assert_eq!(handlers[0].event, "turn");
    }

    #[test]
    fn statements() {
        let it = items(
            "scene CREATE(hero: entity):\n    hero_name = wait text(16)\n    points = 30\n    choice:\n        STRENGTH adjust hero.STATS.strength 10..25 pool points\n        DONE disabled when points > 0:\n            return\n    match points:\n        0:\n            wait confirm\n        10..:\n            x = open SHOP.BUY(wares: w)\n        else:\n            cancel\n    for k in d:\n        if k == 1:\n            break\n        elif k < 0:\n            emit HIT(k, none, 1)\n        else:\n            l[k] += 2\n",
        );
        let ItemKind::Scene { body, .. } = &it[0].kind else { panic!() };
        assert!(matches!(body[0].kind, StmtKind::WaitInput { .. }));
        let StmtKind::Choice(opts) = &body[2].kind else { panic!() };
        assert!(matches!(&opts[0].body, OptionBody::Adjust { pool: Some(_), .. }));
        assert!(opts[1].disabled.is_some());
        let StmtKind::Match { cases, .. } = &body[3].kind else { panic!() };
        assert_eq!(cases.len(), 3);
        assert!(matches!(body[4].kind, StmtKind::For { .. }));
    }

    #[test]
    fn timelines() {
        let it = items(
            "scene GATE_OPENS(lever: entity) timeline:\n    0000ms        play(SFX.LEVER, at: lever)\n    0500ms 1000ms shake(camera, strength: 2)\n    2000ms        mark OPEN\n    OPEN          when game.KING_QUEST == 1: say(GATE.FOR_THE_KING)\n    OPEN+1s       say(GATE.OPENED)\n    +200ms 0500ms pan(camera, to: player)\n\
             scene PATROL(guard: entity) timeline background:\n    track ROUTE loop 12:\n        00 02 move(guard, to: label(POST_A))\n        10    log(GUARD.ALL_CLEAR, speaker: guard)\n",
        );
        let ItemKind::Timeline { timeline, .. } = &it[0].kind else { panic!() };
        assert_eq!(timeline.rows.len(), 6);
        assert!(matches!(&timeline.rows[2], Row::Mark { name, .. } if name == "OPEN"));
        assert!(matches!(&timeline.rows[3], Row::Action { when: Some(_), time: Some(Time::Mark { offset: None, .. }), .. }));
        assert!(matches!(&timeline.rows[4], Row::Action { time: Some(Time::Mark { offset: Some((true, _)), .. }), .. }));
        assert!(matches!(&timeline.rows[5], Row::Action { time: Some(Time::After(_)), span: Some(_), .. }));
        let ItemKind::Timeline { timeline, .. } = &it[1].kind else { panic!() };
        assert!(timeline.background && !timeline.realtime);
        assert_eq!(timeline.tracks[0].rows.len(), 2);
    }

    #[test]
    fn errors_and_recovery() {
        let e = errs("fn A():\n    x = \n    y = 1\nfn B(:\n    return\nquery C(x: number):\n    return x\nai D:\n    return WAIT()\n");
        assert_eq!(e.len(), 3, "{e:#?}");
        assert!(e[0].contains("t:2: error: column") && e[0].contains("expected a value"));
        assert!(e[1].contains("t:4"));
        assert!(e[2].contains("needs a return type"));
        assert!(errs("fn A():\n    x + 1\n")[0].contains("neither"));
        assert!(errs("fn A():\n    for if in x:\n        break\n")[0].contains("reserved word"));
        assert!(errs("fn A()\n    return\n")[0].contains("expected `:`"));
        assert!(errs("namespace STD\nfn A():\n    return\n")[0].contains("outside the code block"));
    }
}
