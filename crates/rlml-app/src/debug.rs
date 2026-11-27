//! The stop of a breakpoint or a step as the front end shows it (M 5.7,
//! RLML-kehitystyokalut.md step 9): where the game is, the source around
//! the line, the call chain and the body's names. Keys continue or step;
//! `:` reads a path of the state, which cannot be written as the action is
//! not over. The front end runs this view in its own loop inside the
//! debugger's hook.

use rlml_core::rt::debug::{Step, Stop};
use rlml_core::rt::world::World;
use termplate_core::frame::{Frame, Role};
use termplate_core::keys::{Binding, KeyCode};

/// A file's text by the name the loader gives it.
pub type Source = Box<dyn Fn(&str) -> Option<String> + Send>;

/// How breakpoints come in at the start: `--break FILE:LINE` and the
/// source the view shows.
pub struct DebugSetup {
    pub breaks: Vec<String>,
    /// A file's text by the name the loader gives it.
    pub source: Source,
}

/// The stop being shown.
pub struct DebugView {
    pub stop: Stop,
    /// The source around the line: its number and text.
    source: Vec<(usize, String)>,
    /// The `:` line being written.
    pub line: Option<String>,
    /// What `:` read last.
    read: Vec<String>,
}

impl DebugView {
    /// The view of `stop`, with `text` the stop's file.
    pub fn new(stop: Stop, text: Option<String>) -> DebugView {
        let source = text
            .map(|t| {
                t.lines()
                    .enumerate()
                    .map(|(i, l)| (i + 1, l.to_string()))
                    .filter(|(n, _)| n + 4 >= stop.line && *n <= stop.line + 4)
                    .collect()
            })
            .unwrap_or_default();
        DebugView {
            stop,
            source,
            line: None,
            read: vec![],
        }
    }

    /// A key: the step it gives, or `None` while the stop goes on.
    /// `c` or `f8` continues, `s` or `f11` steps into, `n` or `f10` over,
    /// `o` out, `esc` continues; `:` reads a path.
    pub fn key(&mut self, b: &Binding, w: &World) -> Option<Step> {
        let Binding::Key(code, mods) = b else {
            return None;
        };
        if let Some(line) = &mut self.line {
            match code {
                KeyCode::Char(c) if !mods.ctrl && !mods.alt => line.push(*c),
                KeyCode::Backspace => {
                    line.pop();
                }
                KeyCode::Enter => {
                    let path = self.line.take().unwrap_or_default();
                    self.read = self.read_path(path.trim(), w);
                }
                KeyCode::Esc => self.line = None,
                _ => {}
            }
            return None;
        }
        match code {
            KeyCode::Char('c') | KeyCode::F(8) | KeyCode::Esc => Some(Step::Continue),
            KeyCode::Char('s') | KeyCode::F(11) => Some(Step::Into),
            KeyCode::Char('n') | KeyCode::F(10) => Some(Step::Over),
            KeyCode::Char('o') => Some(Step::Out),
            KeyCode::Char(':') => {
                self.line = Some(String::new());
                None
            }
            _ => None,
        }
    }

    /// The lines a path gives: `locals.NAME` from the stop, the rest from
    /// the state as the inspector reads it.
    fn read_path(&self, path: &str, w: &World) -> Vec<String> {
        if let Some(name) = path.strip_prefix("locals.") {
            return match self.stop.locals.iter().find(|(n, _)| n == name) {
                Some((n, v)) => vec![format!("{n}: {v}")],
                None => vec![format!("no local `{name}` here")],
            };
        }
        let r = rlml_core::rt::run::Runner::new(w.clone());
        match r.inspect(path) {
            Ok(node) => node.lines(path),
            Err(e) => vec![e],
        }
    }

    pub fn draw(&self, w: usize, h: usize) -> Frame {
        let mut f = Frame::new(w, h);
        if h == 0 {
            return f;
        }
        let s = &self.stop;
        let what = if s.breakpoint.is_some() {
            "breakpoint"
        } else {
            "step"
        };
        f.text(
            0,
            0,
            w,
            &format!(
                "{what} {}:{} in {} (depth {})",
                s.file, s.line, s.name, s.depth
            ),
            Role::Title,
        );
        let mut lines: Vec<(String, Role)> = Vec::new();
        for (n, l) in &self.source {
            let mark = if *n == s.line { "→" } else { " " };
            let role = if *n == s.line { Role::Title } else { Role::Dim };
            lines.push((format!("{mark}{n:>5} │ {l}"), role));
        }
        lines.push((String::new(), Role::Normal));
        lines.push((format!("calls: {}", s.calls.join(" › ")), Role::Normal));
        lines.push(("locals:".into(), Role::Normal));
        for (n, v) in &s.locals {
            lines.push((format!("  {n}: {v}"), Role::Normal));
        }
        if !self.read.is_empty() {
            lines.push((String::new(), Role::Normal));
            lines.extend(self.read.iter().map(|l| (l.clone(), Role::Normal)));
        }
        for (i, (l, role)) in lines.iter().take(h.saturating_sub(2)).enumerate() {
            f.text(0, 1 + i, w, l, *role);
        }
        if h > 1 {
            let bottom = match &self.line {
                Some(l) => format!(":{l}█"),
                None => "c continue  s into  n over  o out  : read".into(),
            };
            f.text(0, h - 1, w, &bottom, Role::Dim);
        }
        f
    }
}
