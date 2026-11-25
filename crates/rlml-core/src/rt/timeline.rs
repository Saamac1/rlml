//! Foreground timelines (RS 9.1–9.5): rows on a presentation clock.
//!
//! The rows' times are fixed when the scene starts: `+` times, marks and
//! inherited times become moments in milliseconds, and the rows are sorted
//! by time, rows of the same moment in writing order. A `when … else`
//! block is a gate: its condition is read when the clock reaches it, and
//! its rows run only on the side it chose. Loop tracks run beside the
//! rest and repeat at their length until the scene ends or
//! `stop(track:)`.
//!
//! The clock moves when the front end ticks it, or at once to the next
//! pause without one (`Runner::scene_clock`); either way the rows run in
//! the same order, so the clock is not input (M 4.4). A state-changing
//! call with a duration is a tween: `move` walks its route a cell at a
//! time and `tile` or `carve` on a set change a cell at a time, each step
//! an ordinary change with its events at its moment (RS 9.1).

use super::eval::Env;
use super::value::{CommandValue, EntityId, Pos, Value};
use super::world::{Pause, RtResult, World};
use crate::rs::ast::{Action, Expr, ExprKind, Row, Time, Timeline, WaitArg};

/// What a row does when the clock reaches it.
#[derive(Debug, Clone)]
enum What {
    Call(Expr),
    Wait(WaitArg),
    /// A `when … else` block's condition, read into its gate.
    Gate(usize, Expr),
    Mark,
}

#[derive(Debug, Clone)]
struct Entry {
    at: i64,
    /// The track in writing order, the rows outside tracks first: rows of
    /// the same moment run in this order.
    track: usize,
    span: Option<i64>,
    always: bool,
    /// A row's own `when`.
    cond: Option<Expr>,
    /// The blocks around the row: gate and the side it is on.
    gates: Vec<(usize, bool)>,
    what: What,
}

#[derive(Debug, Clone)]
struct LoopTrack {
    name: String,
    len: i64,
    entries: Vec<Entry>,
    /// The moment the current round started.
    round: i64,
    next: usize,
    stopped: bool,
}

/// The cells a tween changes, one step at a time.
#[derive(Debug, Clone, PartialEq)]
pub enum Steps {
    Move {
        e: EntityId,
        route: Vec<Pos>,
    },
    Tiles {
        cells: Vec<Pos>,
        tile: Option<String>,
        layer: usize,
    },
}

impl Steps {
    fn len(&self) -> usize {
        match self {
            Steps::Move { route, .. } => route.len(),
            Steps::Tiles { cells, .. } => cells.len(),
        }
    }
}

#[derive(Debug, Clone)]
struct Tween {
    start: i64,
    span: i64,
    done: usize,
    steps: Steps,
}

impl Tween {
    /// The moment of the next step: a step's cell is reached when the
    /// presented position, moving evenly over the span, rounds to it.
    fn next_at(&self) -> i64 {
        let n = self.steps.len() as i64;
        let i = self.done as i64 + 1;
        // ceil((2i - 1) * span / 2n), in integers on every platform
        self.start + ((2 * i - 1) * self.span + 2 * n - 1) / (2 * n)
    }
}

/// A timeline that runs: its rows, its loops, its clock and its tweens.
#[derive(Debug, Clone)]
pub struct TimelineRun {
    entries: Vec<Entry>,
    next: usize,
    loops: Vec<LoopTrack>,
    gates: Vec<Option<bool>>,
    tweens: Vec<Tween>,
    /// The clock in milliseconds from the scene's start.
    pub clock: i64,
    /// When the last row without a loop ends, its duration included.
    pub end: i64,
}

/// How far a timeline got.
#[derive(Debug, Clone, PartialEq)]
pub enum TlFlow {
    /// A `say` or `wait confirm` stopped the clock.
    Pause,
    /// `wait scene`: the sub-scene runs, and the clock waits.
    Sub(CommandValue),
    /// The clock is where it was asked to go; rows are left.
    Running,
    /// Every track without a loop is done (RS 9.3).
    Done,
}

/// Rows of a block with their times.
struct Flat<'a> {
    marks: &'a mut Vec<(String, i64)>,
    gates: usize,
    seq: usize,
    out: Vec<(usize, Entry)>,
    /// Unknown marks are left out: the first pass only collects marks.
    lenient: bool,
    track: usize,
}

impl World<'_> {
    fn time_ms(&mut self, e: &Expr, env: &mut Env) -> RtResult<i64> {
        match self.eval(e, env)? {
            Value::Time(ms) => Ok(ms as i64),
            Value::Number(n) => Ok(n),
            v => Err(self.rt_err(env, e.pos, format!("a time, not `{v}`"))),
        }
    }

    /// A row's start: its own time, the previous row's for none, or from a
    /// mark. `None` when a mark is not known yet.
    fn row_time(
        &mut self,
        time: &Option<Time>,
        prev: i64,
        plus: i64,
        f: &Flat,
        env: &mut Env,
    ) -> RtResult<Option<i64>> {
        Ok(Some(match time {
            None => prev,
            Some(Time::At(e)) => self.time_ms(e, env)?,
            Some(Time::After(e)) => plus + self.time_ms(e, env)?,
            Some(Time::Mark { name, offset }) => {
                let Some(t) = f.marks.iter().find(|(n, _)| n == name).map(|(_, t)| *t) else {
                    if f.lenient {
                        return Ok(None);
                    }
                    return Err(format!(
                        "{}: runtime error: no mark `{name}` in the timeline",
                        env.file
                    ));
                };
                match offset {
                    Some((plus, e)) => {
                        let d = self.time_ms(e, env)?;
                        if *plus { t + d } else { t - d }
                    }
                    None => t,
                }
            }
        }))
    }

    /// The rows of a block with their moments. `block` is the block's
    /// start inside a `when`: its `+` times count from it (RS 9.4).
    fn flatten(
        &mut self,
        rows: &[Row],
        start: i64,
        block: Option<i64>,
        gates: &[(usize, bool)],
        f: &mut Flat,
        env: &mut Env,
    ) -> RtResult<()> {
        let mut prev = start;
        for row in rows {
            let plus = block.unwrap_or(prev);
            match row {
                Row::Mark { time, name, .. } => {
                    let Some(t) = self.row_time(&Some(time.clone()), prev, plus, f, env)? else {
                        continue;
                    };
                    if !f.marks.iter().any(|(n, _)| n == name) {
                        f.marks.push((name.clone(), t));
                    }
                    prev = t;
                    f.seq += 1;
                    f.out.push((
                        f.seq,
                        Entry {
                            at: t,
                            track: f.track,
                            span: None,
                            always: false,
                            cond: None,
                            gates: gates.to_vec(),
                            what: What::Mark,
                        },
                    ));
                }
                Row::Action {
                    time,
                    span,
                    always,
                    when,
                    action,
                    ..
                } => {
                    let Some(t) = self.row_time(time, prev, plus, f, env)? else {
                        continue;
                    };
                    let span = match span {
                        Some(e) => Some(self.time_ms(e, env)?),
                        None => None,
                    };
                    let what = match action {
                        Action::Call(e) => What::Call(e.clone()),
                        Action::Wait(w) => What::Wait(w.clone()),
                    };
                    prev = t;
                    f.seq += 1;
                    f.out.push((
                        f.seq,
                        Entry {
                            at: t,
                            track: f.track,
                            span,
                            always: *always,
                            cond: when.clone(),
                            gates: gates.to_vec(),
                            what,
                        },
                    ));
                }
                Row::When {
                    time,
                    cond,
                    rows,
                    otherwise,
                    ..
                } => {
                    let Some(t) = self.row_time(time, prev, plus, f, env)? else {
                        continue;
                    };
                    let id = f.gates;
                    f.gates += 1;
                    f.seq += 1;
                    f.out.push((
                        f.seq,
                        Entry {
                            at: t,
                            track: f.track,
                            span: None,
                            always: false,
                            cond: None,
                            gates: gates.to_vec(),
                            what: What::Gate(id, cond.clone()),
                        },
                    ));
                    for (side, rows) in [(true, rows), (false, otherwise)] {
                        let mut inner = gates.to_vec();
                        inner.push((id, side));
                        self.flatten(rows, t, Some(t), &inner, f, env)?;
                    }
                    prev = t;
                }
            }
        }
        Ok(())
    }

    /// A timeline's rows on its clock, as the scene starts: times are read
    /// once, in the scene's names (RS 9.1).
    pub(crate) fn timeline_new(&mut self, tl: &Timeline, env: &mut Env) -> RtResult<TimelineRun> {
        let tracks: Vec<(usize, &Vec<Row>)> = std::iter::once((0, &tl.rows))
            .chain(
                tl.tracks
                    .iter()
                    .enumerate()
                    .filter(|(_, t)| t.looped.is_none())
                    .map(|(i, t)| (i + 1, &t.rows)),
            )
            .collect();
        let mut marks = Vec::new();
        // the first pass collects the marks, so a row may use a mark set
        // further down; the second gives every row its moment
        for _ in 0..2 {
            let mut gates = 0;
            let mut seq = 0;
            for (track, rows) in tracks.clone() {
                let mut f = Flat {
                    marks: &mut marks,
                    gates,
                    seq,
                    out: vec![],
                    lenient: true,
                    track,
                };
                self.flatten(rows, 0, None, &[], &mut f, env)?;
                (gates, seq) = (f.gates, f.seq);
            }
        }
        let mut gates = 0;
        let mut seq = 0;
        let mut all = Vec::new();
        for (track, rows) in tracks {
            let mut f = Flat {
                marks: &mut marks,
                gates,
                seq,
                out: vec![],
                lenient: false,
                track,
            };
            self.flatten(rows, 0, None, &[], &mut f, env)?;
            (gates, seq) = (f.gates, f.seq);
            all.extend(f.out);
        }
        all.sort_by_key(|(seq, e)| (e.at, e.track, *seq));
        let entries: Vec<Entry> = all.into_iter().map(|(_, e)| e).collect();
        let end = entries
            .iter()
            .map(|e| e.at + e.span.unwrap_or(0))
            .max()
            .unwrap_or(0);
        let mut loops = Vec::new();
        for (i, t) in tl.tracks.iter().enumerate() {
            let Some(len) = &t.looped else { continue };
            let len = self.time_ms(len, env)?.max(1);
            let mut f = Flat {
                marks: &mut marks,
                gates,
                seq,
                out: vec![],
                lenient: false,
                track: i + 1,
            };
            self.flatten(&t.rows, 0, None, &[], &mut f, env)?;
            (gates, seq) = (f.gates, f.seq);
            let mut rows = f.out;
            rows.sort_by_key(|(seq, e)| (e.at, *seq));
            loops.push(LoopTrack {
                name: t.name.clone(),
                len,
                entries: rows.into_iter().map(|(_, e)| e).collect(),
                round: 0,
                next: 0,
                stopped: false,
            });
        }
        Ok(TimelineRun {
            entries,
            next: 0,
            loops,
            gates: vec![None; gates],
            tweens: vec![],
            clock: 0,
            end,
        })
    }

    /// Is a call with a duration a tween that changes the state, a step at
    /// a time (RS 9.1)?
    fn tweened(&self, e: &Expr, env: &Env) -> bool {
        let ExprKind::Call(f, _) = &e.kind else {
            return false;
        };
        let _ = env;
        matches!(&f.kind, ExprKind::Word(name) if matches!(name.as_str(), "move" | "tile" | "carve"))
    }

    /// A row the clock reached. `true` when it stopped the clock or asked
    /// for a sub-scene, given in `sub`.
    fn run_entry(
        &mut self,
        run: &mut TimelineRun,
        e: &Entry,
        env: &mut Env,
        sub: &mut Option<CommandValue>,
    ) -> RtResult<bool> {
        if !e.gates.iter().all(|(g, side)| run.gates[*g] == Some(*side)) {
            return Ok(false);
        }
        if let Some(c) = &e.cond
            && !self.truth(c, env)?
        {
            return Ok(false);
        }
        match &e.what {
            What::Mark => {}
            What::Gate(id, c) => run.gates[*id] = Some(self.truth(c, env)?),
            What::Wait(WaitArg::Confirm) => {
                if !self.scene_skipping {
                    self.pause = Some(Pause::Confirm);
                    return Ok(true);
                }
            }
            What::Wait(WaitArg::Scene(x)) => match self.eval(x, env)? {
                Value::Scene(call) => {
                    *sub = Some(call);
                    return Ok(true);
                }
                v => {
                    return Err(self.rt_err(
                        env,
                        x.pos,
                        format!("`wait scene` takes a scene, not `{v}`"),
                    ));
                }
            },
            What::Wait(_) => {
                return Err(format!(
                    "{}: runtime error: a timeline row waits only for a confirmation or a scene",
                    env.file
                ));
            }
            What::Call(x) => {
                let span = e.span.filter(|s| *s > 0);
                if span.is_some() && self.tweened(x, env) {
                    self.tweening = span;
                }
                self.row_always = e.always;
                self.row_span = span.unwrap_or(0);
                let r = self.eval(x, env);
                self.tweening = None;
                self.row_always = false;
                self.row_span = 0;
                r?;
                if let (Some(steps), Some(span)) = (self.tween_out.take(), span)
                    && steps.len() > 0
                {
                    run.tweens.push(Tween {
                        start: e.at,
                        span,
                        done: 0,
                        steps,
                    });
                }
                for name in std::mem::take(&mut self.stop_tracks) {
                    for l in run.loops.iter_mut().filter(|l| l.name == name) {
                        l.stopped = true;
                    }
                }
                if self.pause.is_some() {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    /// A tween's next step: a `move` into its next cell, which stops it
    /// when the cell is blocked then, or a tile changed (RS 9.1).
    fn tween_step(&mut self, tw: &mut Tween, env: &mut Env) -> RtResult<bool> {
        match &tw.steps {
            Steps::Move { e, route } => {
                let (e, c) = (*e, route[tw.done]);
                let here = self.s.at(e);
                if !self.s.get(e).alive
                    || here.is_none_or(|p| p.map != c.map)
                    || self.blocks(c, "move", Some(e), true)
                {
                    return Ok(false);
                }
                self.engine_move(
                    &[(None, Value::Entity(e)), (Some("to".into()), Value::Pos(c))],
                    env,
                    crate::rs::ast::Pos::default(),
                )?;
            }
            Steps::Tiles { cells, tile, layer } => {
                let c = cells[tw.done];
                self.s.maps[c.map].set_tile(*layer, c.x, c.y, tile.clone());
            }
        }
        tw.done += 1;
        Ok(tw.done < tw.steps.len())
    }

    /// Runs the timeline up to `until` on its clock, or until a row stops
    /// it. Without `until` only the rows of the clock's moment run. Loops
    /// do not run when the scene is skipped (RS 9.8).
    pub(crate) fn timeline_run(
        &mut self,
        run: &mut TimelineRun,
        env: &mut Env,
        until: Option<i64>,
    ) -> RtResult<TlFlow> {
        let limit = until.unwrap_or(run.clock).max(run.clock);
        loop {
            // the next moment: a tween's step, a row, a loop's row
            let tween = run
                .tweens
                .iter()
                .enumerate()
                .map(|(i, t)| (t.next_at(), i))
                .min();
            let row = run.entries.get(run.next).map(|e| (e.at, e.track));
            let skipping = self.scene_skipping;
            let looped = run
                .loops
                .iter()
                .enumerate()
                .filter(|(_, l)| !l.stopped && !skipping && !l.entries.is_empty())
                .map(|(i, l)| {
                    let (at, track) = match l.entries.get(l.next) {
                        Some(e) => (l.round + e.at, e.track),
                        None => (l.round + l.len + l.entries[0].at, l.entries[0].track),
                    };
                    ((at, track), i)
                })
                .min();
            let t = [tween.map(|t| t.0), row.map(|r| r.0), looped.map(|l| l.0.0)]
                .into_iter()
                .flatten()
                .min();
            match t {
                Some(t) if t <= limit => {
                    run.clock = run.clock.max(t);
                    if let Some((at, i)) = tween
                        && at == t
                    {
                        let mut tw = run.tweens.remove(i);
                        if self.tween_step(&mut tw, env)? {
                            run.tweens.insert(i, tw);
                        }
                        continue;
                    }
                    // rows of the same moment by track, a loop's among them
                    let row_first = match (row, looped) {
                        (Some(r), Some((l, _))) => r.0 == t && (l.0 > t || r.1 <= l.1),
                        (Some(r), None) => r.0 == t,
                        _ => false,
                    };
                    if row_first {
                        let e = run.entries[run.next].clone();
                        run.next += 1;
                        let mut sub = None;
                        if self.run_entry(run, &e, env, &mut sub)? {
                            return Ok(match sub {
                                Some(call) => TlFlow::Sub(call),
                                None => TlFlow::Pause,
                            });
                        }
                        continue;
                    }
                    if let Some((_, i)) = looped {
                        let l = &mut run.loops[i];
                        if l.next >= l.entries.len() {
                            l.round += l.len;
                            l.next = 0;
                        }
                        let e = l.entries[l.next].clone();
                        l.next += 1;
                        let mut sub = None;
                        if self.run_entry(run, &e, env, &mut sub)? {
                            return Ok(match sub {
                                Some(call) => TlFlow::Sub(call),
                                None => TlFlow::Pause,
                            });
                        }
                    }
                }
                _ => {
                    run.clock = limit;
                    let rest = run.next < run.entries.len()
                        || !run.tweens.is_empty()
                        || run.clock < run.end;
                    let loops = !skipping
                        && run
                            .loops
                            .iter()
                            .any(|l| !l.stopped && !l.entries.is_empty());
                    let only_loops = run.entries.is_empty();
                    return Ok(if rest || (only_loops && loops) {
                        TlFlow::Running
                    } else {
                        TlFlow::Done
                    });
                }
            }
        }
    }
}
