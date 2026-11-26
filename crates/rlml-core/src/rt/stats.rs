//! Interpreter counters for `rlml bench` (RLML-kehitystyokalut.md step 1):
//! statements run, name lookups, local reads, body copies, events and
//! handlers. They exist with the `stats` feature only; without it every
//! `bump` is empty and costs nothing.
//!
//! Spans time a few parts of the engine that the counters cannot weigh:
//! gathering an event's handlers, fields of view, routes, hearing, ai
//! choices and commands.
//! A span inside another is counted in both.
//!
//! The counters are global and count every runner in the process, so a
//! bench resets them before it plays and reads them after.

/// What is counted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Count {
    /// A statement the frames ran.
    Stmt,
    /// A body or a block copied into a frame (`run_to_end`, `if`, loops).
    BodyCopy,
    /// A local name read from `Env.locals`.
    LocalRead,
    /// An upper-case name resolved in the model (`World::resolve`).
    NameResolve,
    /// A declaration looked up by its full name (`World::decl_of`).
    DeclLookup,
    /// A user function or query called.
    Call,
    /// An event sent to a phase's handlers (`emit_to`).
    Emit,
    /// A declaration looked at while gathering an event's handlers.
    HandlerScan,
    /// A handler run.
    HandlerRun,
}

pub const ALL: [Count; 9] = [
    Count::Stmt,
    Count::BodyCopy,
    Count::LocalRead,
    Count::NameResolve,
    Count::DeclLookup,
    Count::Call,
    Count::Emit,
    Count::HandlerScan,
    Count::HandlerRun,
];

impl Count {
    pub fn name(self) -> &'static str {
        match self {
            Count::Stmt => "statements",
            Count::BodyCopy => "body copies",
            Count::LocalRead => "local reads",
            Count::NameResolve => "name resolves",
            Count::DeclLookup => "decl lookups",
            Count::Call => "function calls",
            Count::Emit => "emits",
            Count::HandlerScan => "handler scans",
            Count::HandlerRun => "handler runs",
        }
    }
}

/// A timed part of the engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Span {
    /// `World::handlers`: an event's handlers gathered from the model.
    Handlers,
    /// `World::field_of_view`.
    Sight,
    /// `World::route_within`: A* routes.
    Route,
    /// `World::listen`: the sounds a listener hears.
    Hearing,
    /// `World::run_ai`: an actor's ai choosing its command.
    Ai,
    /// `World::start_command`: a command's body, with its events.
    Command,
}

pub const SPANS: [Span; 6] = [
    Span::Handlers,
    Span::Sight,
    Span::Route,
    Span::Hearing,
    Span::Ai,
    Span::Command,
];

impl Span {
    pub fn name(self) -> &'static str {
        match self {
            Span::Handlers => "handler lookup",
            Span::Sight => "field of view",
            Span::Route => "routes",
            Span::Hearing => "hearing",
            Span::Ai => "ai choices",
            Span::Command => "commands",
        }
    }
}

/// Adds its time to its span when it goes; nothing without `stats`.
pub struct Timer {
    #[cfg(feature = "stats")]
    started: (Span, std::time::Instant),
}

#[cfg(feature = "stats")]
impl Drop for Timer {
    fn drop(&mut self) {
        let (s, t) = self.started;
        on::add_time(s as usize, t.elapsed().as_nanos() as u64);
    }
}

/// Times the rest of the scope as `s`.
#[inline(always)]
pub fn span(s: Span) -> Timer {
    #[cfg(feature = "stats")]
    return Timer {
        started: (s, std::time::Instant::now()),
    };
    #[cfg(not(feature = "stats"))]
    {
        let _ = s;
        Timer {}
    }
}

#[cfg(feature = "stats")]
mod on {
    use std::sync::atomic::{AtomicU64, Ordering};

    pub static COUNTS: [AtomicU64; 9] = [const { AtomicU64::new(0) }; 9];
    pub static NANOS: [AtomicU64; 6] = [const { AtomicU64::new(0) }; 6];

    pub fn add_time(i: usize, ns: u64) {
        NANOS[i].fetch_add(ns, Ordering::Relaxed);
    }

    pub fn time(i: usize) -> u64 {
        NANOS[i].load(Ordering::Relaxed)
    }

    pub fn bump(i: usize) {
        COUNTS[i].fetch_add(1, Ordering::Relaxed);
    }

    pub fn read(i: usize) -> u64 {
        COUNTS[i].load(Ordering::Relaxed)
    }

    pub fn reset() {
        for c in COUNTS.iter().chain(&NANOS) {
            c.store(0, Ordering::Relaxed);
        }
    }
}

/// Counts one `c`; nothing without the `stats` feature.
#[inline(always)]
pub fn bump(c: Count) {
    #[cfg(feature = "stats")]
    on::bump(c as usize);
    #[cfg(not(feature = "stats"))]
    let _ = c;
}

/// Whether the counters count: the `stats` feature.
pub fn enabled() -> bool {
    cfg!(feature = "stats")
}

/// Every counter to zero.
pub fn reset() {
    #[cfg(feature = "stats")]
    on::reset();
}

/// The counters now, by name; `None` without the `stats` feature.
pub fn read() -> Option<Vec<(Count, u64)>> {
    #[cfg(feature = "stats")]
    return Some(ALL.iter().map(|&c| (c, on::read(c as usize))).collect());
    #[cfg(not(feature = "stats"))]
    None
}

/// The spans' times now, in nanoseconds; `None` without `stats`.
pub fn times() -> Option<Vec<(Span, u64)>> {
    #[cfg(feature = "stats")]
    return Some(SPANS.iter().map(|&s| (s, on::time(s as usize))).collect());
    #[cfg(not(feature = "stats"))]
    None
}
