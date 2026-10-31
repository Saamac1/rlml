//! Keys and contexts (D 11.3, M 8).
//!
//! Key names are read with Termplate's `parse_binding`, so `k`, `up`,
//! `esc`, `shift+q`, `plus` and `num8` mean the same in both. Contexts
//! stack at run time: `ALWAYS` is heard above all of them, then the top of
//! the stack; a `modal` context (the default) stops there, a
//! `passthrough` one lets keys it does not bind go down. The engine's
//! contexts have default bindings, and the game's row for the same action
//! in the same context replaces the default.

use std::collections::BTreeMap;

use rlml_core::model::{Kind, Model};
use rlml_core::names;
use rlml_core::rs::ast::{ItemKind, Source};
use rlml_core::rt::value::{CommandValue, Value};
use termplate_core::keys::{Binding, KeyCode, parse_binding};

/// What a key does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// A command by full name.
    Command(String),
    /// An engine action (`pass`, `confirm`, `cursor.north`, …, M 9.1).
    Engine(String),
    /// `open NAME [stack]` (D 11.2).
    Open { menu: String, stack: bool },
    /// `toggle PANEL` (D 11.3).
    Toggle(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Modal,
    Passthrough,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Context {
    pub mode: Mode,
    pub rows: Vec<(Binding, Action)>,
}

/// The engine's contexts and their default bindings (D 11.3).
fn defaults() -> Vec<(&'static str, Vec<(&'static str, &'static str)>)> {
    let cursor = [
        ("cursor.north", "up k num8"),
        ("cursor.south", "down j num2"),
        ("cursor.west", "left h num4"),
        ("cursor.east", "right l num6"),
        ("cursor.northwest", "y num7"),
        ("cursor.northeast", "u num9"),
        ("cursor.southwest", "b num1"),
        ("cursor.southeast", "n num3"),
        ("next", "tab"),
        ("prev", "shift+tab"),
        ("confirm", "enter space"),
        ("back", "esc"),
    ];
    let menu = vec![("cursor.north", "up k"), ("cursor.south", "down j"), ("confirm", "enter space"), ("back", "esc backspace")];
    vec![
        ("MENU", menu.clone()),
        ("DIALOGUE", menu),
        ("SCENE", vec![("confirm", "enter space"), ("skip", "tab"), ("restart", "r"), ("rewind", "backspace")]),
        ("AIM", cursor.to_vec()),
        ("LOOK", cursor.to_vec()),
        ("DIRECTION", cursor.to_vec()),
        ("ALWAYS", vec![]),
    ]
}

#[derive(Debug, Clone, Default)]
pub struct Keys {
    pub contexts: BTreeMap<String, Context>,
    /// The game's base context (`game.keys`, D 3.3).
    pub base: Option<String>,
    /// Key names that could not be read.
    pub warnings: Vec<String>,
}

impl Keys {
    pub fn from_model(m: &Model) -> Keys {
        let mut keys = Keys::default();
        for (name, rows) in defaults() {
            let mut ctx = Context { mode: Mode::Modal, rows: vec![] };
            for (action, names) in rows {
                for n in names.split_whitespace() {
                    if let Ok(b) = parse_binding(n) {
                        ctx.rows.push((b, Action::Engine(action.to_string())));
                    }
                }
            }
            keys.contexts.insert(name.to_string(), ctx);
        }
        for (full, blocks) in &m.contexts {
            let name = full.clone();
            for b in blocks.iter().filter(|b| b.kind == "keys") {
                let ctx = keys.contexts.entry(name.clone()).or_insert(Context { mode: Mode::Modal, rows: vec![] });
                if b.has_arg("passthrough") {
                    ctx.mode = Mode::Passthrough;
                }
                for item in &b.items {
                    let (_, row) = item.marker();
                    let (action, n) = match row.key() {
                        "open" => (Action::Open { menu: row.word(1).and_then(|w| m.resolve(row.namespace.as_deref(), w).ok()).map(|d| d.full.clone()).unwrap_or_default(), stack: row.word(2) == Some("stack") }, if row.word(2) == Some("stack") { 3 } else { 2 }),
                        "toggle" => (Action::Toggle(row.word(1).unwrap_or("").to_string()), 2),
                        w if names::is_vocabulary(w) => (Action::Engine(w.to_string()), 1),
                        w => match m.resolve_kind(row.namespace.as_deref(), w, &[Kind::Command]) {
                            Ok(d) => (Action::Command(d.full.clone()), 1),
                            Err(_) => continue,
                        },
                    };
                    // the game's row for an action replaces the default's keys
                    let ctx = keys.contexts.get_mut(&name).unwrap();
                    ctx.rows.retain(|(_, a)| *a != action);
                    for k in row.tokens.iter().skip(n) {
                        match parse_binding(&k.text) {
                            Ok(b) => {
                                ctx.rows.retain(|(x, _)| *x != b);
                                ctx.rows.push((b, action.clone()));
                            }
                            Err(e) => keys.warnings.push(format!("{}:{}: {e}", item.file, item.line)),
                        }
                    }
                }
            }
        }
        let game = m.named_game();
        let ns = game.and_then(|g| g.namespace.as_deref());
        keys.base = game.and_then(|g| g.items.iter().find(|i| i.key() == "keys")).and_then(|i| i.word(1)).and_then(|n| names::candidates(ns, n).into_iter().find(|c| keys.contexts.contains_key(c)));
        keys
    }

    /// The action of a key in a context stack (bottom first): `ALWAYS`
    /// first, then the top down while contexts pass keys through (D 11.3).
    pub fn lookup(&self, stack: &[String], b: &Binding) -> Option<&Action> {
        let find = |name: &str| self.contexts.get(name).and_then(|c| c.rows.iter().find(|(x, _)| x == b).map(|(_, a)| a));
        if let Some(a) = find("ALWAYS") {
            return Some(a);
        }
        for name in stack.iter().rev() {
            if let Some(a) = find(name) {
                return Some(a);
            }
            if self.contexts.get(name).is_none_or(|c| c.mode == Mode::Modal) {
                return None;
            }
        }
        None
    }
}

/// What a key press asks the game to do.
#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    Command(CommandValue),
    Engine(String),
    Open { menu: String, stack: bool },
    Toggle(String),
    /// The repeat count is being typed (D 11.3).
    Counting(String),
    /// The key is not bound here.
    Unbound(Binding),
}

/// Keys and the repeat count being typed.
#[derive(Debug, Clone)]
pub struct Input {
    pub keys: Keys,
    count: Option<String>,
}

/// The parameter a command's repeat count goes to (`count`, RS 7.1).
fn count_param(m: &Model, command: &str) -> Option<String> {
    let rlml_core::model::Source::Decl(d) = &m.defs.get(command)?.source else { return None };
    let ItemKind::Command { params, .. } = &d.item.kind else { return None };
    params.iter().find(|p| matches!(p.source, Some(Source::Count))).map(|p| p.name.clone())
}

impl Input {
    pub fn new(m: &Model) -> Input {
        Input { keys: Keys::from_model(m), count: None }
    }

    /// The context stack of the game's play: its base context.
    pub fn base_stack(&self) -> Vec<String> {
        self.keys.base.iter().cloned().collect()
    }

    pub fn press(&mut self, m: &Model, stack: &[String], b: Binding) -> Effect {
        // `repeat`: digits write the count, the first other key ends it and is
        // the command's key (D 11.3)
        if let Some(count) = &mut self.count {
            if let Binding::Key(KeyCode::Char(c), _) = b
                && c.is_ascii_digit()
            {
                count.push(c);
                return Effect::Counting(count.clone());
            }
            let n = self.count.take().unwrap();
            return match self.keys.lookup(stack, &b).cloned() {
                Some(Action::Engine(a)) if a == "back" => Effect::Counting(String::new()),
                Some(Action::Command(c)) => {
                    let mut cmd = CommandValue { name: c.clone(), args: vec![] };
                    if let (Some(p), Ok(k)) = (count_param(m, &c), n.parse::<i64>()) {
                        cmd.args.push((Some(p), Value::Number(k)));
                    }
                    Effect::Command(cmd)
                }
                Some(a) => self.effect(a),
                None => Effect::Unbound(b),
            };
        }
        match self.keys.lookup(stack, &b).cloned() {
            Some(Action::Engine(a)) if a == "repeat" => {
                self.count = Some(String::new());
                Effect::Counting(String::new())
            }
            Some(a) => self.effect(a),
            None => Effect::Unbound(b),
        }
    }

    fn effect(&self, a: Action) -> Effect {
        match a {
            Action::Command(c) => Effect::Command(CommandValue { name: c, args: vec![] }),
            Action::Engine(e) => Effect::Engine(e),
            Action::Open { menu, stack } => Effect::Open { menu, stack },
            Action::Toggle(p) => Effect::Toggle(p),
        }
    }
}
