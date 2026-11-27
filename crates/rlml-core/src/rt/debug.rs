//! Breakpoints and stepping, the light kind (M 5.7, RLML-kehitystyokalut.md
//! step 9).
//!
//! The interpreter asks the world's debugger before every statement it
//! runs; without one the cost is a branch. A breakpoint is a source line,
//! checked against the model's statements when it is set. When one is hit,
//! or a step ends, the debugger's hook gets a [`Stop`]: where the game is,
//! its call chain and the body's names. The hook runs the front end's own
//! loop inside the interpreter and answers with a [`Step`]. The state can
//! be read then but not written, as the action is not over.

use std::sync::{Arc, Mutex, mpsc};

use super::eval::Env;
use super::world::World;
use crate::model::Model;
use crate::rs::ast::{ItemKind, Stmt};

/// Where the game stopped, owned, so it can go to another thread.
#[derive(Debug, Clone, PartialEq)]
pub struct Stop {
    pub file: String,
    pub line: usize,
    pub col: usize,
    /// The running declaration's full name.
    pub name: String,
    /// Nested calls and events now.
    pub depth: u32,
    /// The bodies the stop is in, outermost first, as far as the debugger
    /// has seen them start.
    pub calls: Vec<String>,
    /// The body's names and their values.
    pub locals: Vec<(String, String)>,
    /// The breakpoint hit, or `None` for the end of a step.
    pub breakpoint: Option<(String, usize)>,
}

impl Stop {
    /// The stop on one line: where, in what and the names.
    pub fn line(&self) -> String {
        let what = if self.breakpoint.is_some() {
            "break"
        } else {
            "step"
        };
        let names: Vec<String> = self
            .locals
            .iter()
            .map(|(n, v)| format!("{n} = {v}"))
            .collect();
        format!(
            "{what} {}:{} in {} (depth {}): {}",
            self.file,
            self.line,
            self.name,
            self.depth,
            if names.is_empty() {
                "no names".to_string()
            } else {
                names.join(", ")
            }
        )
    }
}

/// A hook that writes each stop as a line and goes on, for a run without
/// a screen (`rlml run --break`).
pub fn printer() -> (Hook, Arc<Mutex<Vec<String>>>) {
    let lines = Arc::new(Mutex::new(Vec::new()));
    let kept = lines.clone();
    let hook: Hook = Box::new(move |_, stop| {
        if let Ok(mut l) = kept.lock() {
            l.push(stop.line());
        }
        Step::Continue
    });
    (hook, lines)
}

/// What the game does after a stop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// To the next breakpoint.
    Continue,
    /// To the next statement, into calls.
    Into,
    /// To the next statement of this body or a caller's.
    Over,
    /// To the next statement of a caller.
    Out,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Run,
    Into,
    Over(u32),
    Out(u32),
}

/// The front end's answer to a stop: it may read the world, not change it.
pub type Hook = Box<dyn FnMut(&World, &Stop) -> Step + Send>;

/// Breakpoints, the step going on and the hook.
pub struct Debugger {
    pub breaks: Vec<(String, usize)>,
    mode: Mode,
    hook: Hook,
    /// The bodies started, by depth from 1, and the outermost body.
    calls: Vec<String>,
    top: String,
}

/// A debugger the world and the front end share.
pub type Shared = Arc<Mutex<Debugger>>;

impl Debugger {
    pub fn new(hook: Hook) -> Debugger {
        Debugger {
            breaks: vec![],
            mode: Mode::Run,
            hook,
            calls: vec![],
            top: String::new(),
        }
    }

    pub fn shared(self) -> Shared {
        Arc::new(Mutex::new(self))
    }

    /// Sets a breakpoint at `FILE:LINE` (M 5.7). The file may be named by
    /// its path's end; a line without a statement moves to the next one
    /// of its declaration. Gives the breakpoint as set.
    pub fn add(&mut self, m: &Model, spec: &str) -> Result<(String, usize), String> {
        let at = statement_at(m, spec)?;
        if !self.breaks.contains(&at) {
            self.breaks.push(at.clone());
        }
        Ok(at)
    }

    /// Takes the breakpoint at `FILE:LINE` away; `false` when there was none.
    pub fn remove(&mut self, m: &Model, spec: &str) -> bool {
        let Ok(at) = statement_at(m, spec) else {
            return false;
        };
        let n = self.breaks.len();
        self.breaks.retain(|b| *b != at);
        self.breaks.len() != n
    }
}

/// The breakpoint place `FILE:LINE` names: a file of the model's code and
/// the first statement line from LINE on within the declaration there.
pub fn statement_at(m: &Model, spec: &str) -> Result<(String, usize), String> {
    let (file, line) = spec
        .rsplit_once(':')
        .and_then(|(f, l)| Some((f.trim(), l.trim().parse::<usize>().ok()?)))
        .ok_or_else(|| format!("`{spec}`: a breakpoint is FILE:LINE"))?;
    let mut files: Vec<&str> = m.code.iter().map(|d| d.file.as_str()).collect();
    files.sort();
    files.dedup();
    let hits: Vec<&str> = files
        .iter()
        .copied()
        .filter(|f| *f == file || f.ends_with(&format!("/{file}")))
        .collect();
    let file = match hits.as_slice() {
        [one] => *one,
        [] => return Err(format!("`{file}` has no RogueScript")),
        many => {
            return Err(format!(
                "`{file}` could be {}: say more of the path",
                many.join(", ")
            ));
        }
    };
    // the declaration the line is in: the last one starting at or before it
    let decl = m
        .code
        .iter()
        .filter(|d| d.file == file && d.item.pos.line <= line)
        .max_by_key(|d| d.item.pos.line)
        .ok_or_else(|| format!("{file}:{line} is before the file's RogueScript"))?;
    let mut lines = Vec::new();
    for body in bodies(&decl.item.kind) {
        super::path::all_positions(body, &mut lines);
    }
    lines
        .into_iter()
        .map(|p| p.line)
        .filter(|&l| l >= line)
        .min()
        .map(|l| (file.to_string(), l))
        .ok_or_else(|| {
            format!(
                "{file}:{line}: no statement there or after it in `{}`",
                decl.name
            )
        })
}

/// The statement bodies of a declaration.
fn bodies(k: &ItemKind) -> Vec<&[Stmt]> {
    match k {
        ItemKind::Function { body, .. }
        | ItemKind::Command { body, .. }
        | ItemKind::Ai { body }
        | ItemKind::Generator { body, .. }
        | ItemKind::Scene { body, .. } => vec![body],
        ItemKind::Handler(h) => vec![&h.body],
        ItemKind::Component { handlers, .. } => {
            handlers.iter().map(|h| h.body.as_slice()).collect()
        }
        _ => vec![],
    }
}

/// A hook that sends each stop over a channel and waits for the answer:
/// the game runs in its own thread, and the other end, such as an MCP
/// server, answers `continue` or a step (RLML-kehitystyokalut.md step 9).
/// A closed channel lets the game run on.
pub fn channel() -> (Hook, mpsc::Receiver<Stop>, mpsc::Sender<Step>) {
    let (stop_tx, stop_rx) = mpsc::channel();
    let (step_tx, step_rx) = mpsc::channel::<Step>();
    let hook: Hook = Box::new(move |_, stop| {
        if stop_tx.send(stop.clone()).is_err() {
            return Step::Continue;
        }
        step_rx.recv().unwrap_or(Step::Continue)
    });
    (hook, stop_rx, step_tx)
}

impl World<'_> {
    /// A body starts at the depth now (M 6.2): the debugger keeps its
    /// name for the call chain.
    pub(crate) fn debug_enter(&mut self, name: &str) {
        if let Some(d) = &self.debug
            && let Ok(mut d) = d.lock()
        {
            let depth = self.depth as usize;
            d.calls.truncate(depth.saturating_sub(1));
            d.calls.push(name.to_string());
        }
    }

    /// Before statement `s` of the body `env` runs: a stop, when a
    /// breakpoint or the step going on says so.
    pub(crate) fn debug_stmt(&mut self, s: &Stmt, env: &Env) {
        let Some(shared) = self.debug.clone() else {
            return;
        };
        let Ok(mut d) = shared.lock() else {
            return;
        };
        let line = s.pos.line;
        let breakpoint = d
            .breaks
            .iter()
            .find(|(f, l)| *l == line && *f == env.file)
            .cloned();
        let depth = self.depth;
        // the outermost body does not start through a call
        if depth == 0 && d.top != env.name {
            d.top = env.name.clone();
        }
        let stepped = match d.mode {
            Mode::Run => false,
            Mode::Into => true,
            Mode::Over(at) => depth <= at,
            Mode::Out(at) => depth < at,
        };
        if breakpoint.is_none() && !stepped {
            return;
        }
        let mut calls = vec![d.top.clone()];
        calls.extend(d.calls.iter().take(depth as usize).cloned());
        calls.retain(|c| !c.is_empty());
        if calls.last() != Some(&env.name) {
            calls.push(env.name.clone());
        }
        let mut locals: Vec<(String, String)> = env
            .locals
            .iter()
            .map(|(n, v)| (n.clone(), super::inspect::show(v)))
            .collect();
        for (n, e) in [
            ("self", env.self_),
            ("actor", env.actor),
            ("holder", env.holder),
        ] {
            if let Some(e) = e {
                locals.push((n.into(), format!("entity#{e} {}", self.s.get(e).kind)));
            }
        }
        let stop = Stop {
            file: env.file.clone(),
            line,
            col: s.pos.col,
            name: env.name.clone(),
            depth,
            calls,
            locals,
            breakpoint,
        };
        let step = (d.hook)(self, &stop);
        d.mode = match step {
            Step::Continue => Mode::Run,
            Step::Into => Mode::Into,
            Step::Over => Mode::Over(depth),
            Step::Out => Mode::Out(depth),
        };
    }
}
