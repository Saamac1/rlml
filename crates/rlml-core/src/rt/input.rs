//! The input log (M 4.4): the session's input as the engine ran it, and
//! its replay. A start (seed and seats) and the log give the same game.
//!
//! The runner writes a row for each input it runs: a seat's command with
//! its filled arguments, the start of a travel or a following, a travel's
//! step and a stop. Rounds without a turn are no rows: a row's moment
//! tells how far the world went before it, and the replay runs rounds up
//! to it. A row whose moment the replay does not reach is an error, since
//! the data or the engine has changed since the log was written.
//!
//! The text form is the engine's own, one row a line:
//!
//! ```text
//! rlml-input 1
//! seed 7
//! seats 2
//! 0 1 STD.MOVE dir=north
//! 120 2 travel_to ROOM:14,6
//! 240 2 step
//! 360 2 stop
//! 360 end
//! ```

use std::fmt;

use super::run::{Runner, Status};
use super::state::{SeatId, State};
use super::value::{CommandValue, ComponentValue, Dir, EntityId, Pos, Side, Value};
use super::world::{RtResult, World};
use crate::model::Model;

/// A command argument as the log keeps it: cells by their map's name, so
/// a log does not depend on the order maps were loaded in.
#[derive(Debug, Clone, PartialEq)]
pub enum Arg {
    Number(i64),
    Bool(bool),
    Text(String),
    Name(String),
    Dir(Dir),
    Side(Side),
    Pos { map: String, x: i32, y: i32 },
    Entity(EntityId),
    None,
}

/// What a row does.
#[derive(Debug, Clone, PartialEq)]
pub enum Act {
    /// The seat's command with its filled arguments.
    Command {
        name: String,
        args: Vec<(Option<String>, Arg)>,
    },
    /// `travel_to`: a travel to a cell starts (M 8.1).
    TravelTo { map: String, x: i32, y: i32 },
    /// `follow`: a following of the entity starts (M 8.1).
    Follow(EntityId),
    /// A step of the seat's travel or following (`follow.step`).
    Step,
    /// A press that stopped a travel, or an order gone stale (M 8.1).
    Stop,
    /// `join`: the row's seat joined (M 8.2).
    Join,
    /// `leave_seat`: the row's seat left.
    Leave,
    /// A scene's line or confirmation confirmed (`scene.confirm`).
    SceneConfirm,
    /// A scene skipped (`skip`, RS 9.8).
    SceneSkip,
    /// A scene from its start again (`restart`).
    SceneRestart,
    /// `rewind` (RS 9.8).
    SceneRewind,
    /// A choice's item (`scene.choose NAME`).
    SceneChoose(String),
    /// An `adjust` item moved (`scene.adjust NAME +1`).
    SceneAdjust(String, i64),
    /// A field's answer (`scene.answer VALUE`).
    SceneAnswer(Arg),
    /// The menu `open` waited for closed (`scene.close`).
    SceneClose,
    /// The end of the log: the world went on to this moment.
    End,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    /// `game.time` when the engine ran the row.
    pub time: i64,
    /// The seat whose input it was; none on `end`.
    pub seat: Option<SeatId>,
    pub act: Act,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct InputLog {
    pub seed: u64,
    /// Seats joined at the start (M 5.1).
    pub seats: u8,
    /// The profile's snapshot at the start (M 5.1 step 2), so a replay
    /// does not depend on this computer's profile.
    pub profile: Vec<ComponentValue>,
    pub rows: Vec<Row>,
    /// The active mods in load order (M 4.4, D 3.6).
    pub mods: Vec<String>,
    /// A segment from a reload (M 4.4): the game's files' hash and the
    /// save the rows start from.
    pub from: Option<(u64, String)>,
    /// An input the log cannot hold, such as an argument of a kind it has
    /// no text form for. Such a log is not written.
    pub broken: Option<String>,
}

impl Arg {
    /// `v` as the log keeps it, with map names from `s`.
    pub fn of(v: &Value, s: &State) -> Result<Arg, String> {
        Ok(match v {
            Value::Number(n) => Arg::Number(*n),
            Value::Bool(b) => Arg::Bool(*b),
            Value::Text(t) => Arg::Text(t.clone()),
            Value::Name(n) => Arg::Name(n.clone()),
            Value::Dir(d) => Arg::Dir(*d),
            Value::Side(d) => Arg::Side(*d),
            Value::Pos(p) => Arg::Pos {
                map: s.maps[p.map].name.clone(),
                x: p.x,
                y: p.y,
            },
            Value::Entity(e) => Arg::Entity(*e),
            Value::None => Arg::None,
            other => return Err(format!("the input log cannot hold the argument `{other}`")),
        })
    }

    /// The value on the state's maps.
    pub fn value(&self, s: &State) -> RtResult<Value> {
        Ok(match self {
            Arg::Number(n) => Value::Number(*n),
            Arg::Bool(b) => Value::Bool(*b),
            Arg::Text(t) => Value::Text(t.clone()),
            Arg::Name(n) => Value::Name(n.clone()),
            Arg::Dir(d) => Value::Dir(*d),
            Arg::Side(d) => Value::Side(*d),
            Arg::Pos { map, x, y } => Value::Pos(pos(s, map, *x, *y)?),
            Arg::Entity(e) => {
                if *e >= s.entities.len() {
                    return Err(format!(
                        "the input log names entity #{e}, which the game has not made"
                    ));
                }
                Value::Entity(*e)
            }
            Arg::None => Value::None,
        })
    }
}

fn pos(s: &State, map: &str, x: i32, y: i32) -> RtResult<Pos> {
    let map = s
        .map_id(map)
        .ok_or_else(|| format!("the input log names map {map}, which is not loaded"))?;
    Ok(Pos { map, x, y })
}

/// A name is written bare when it cannot be read as anything else.
fn bare_name(n: &str) -> bool {
    n.chars().any(|c| c.is_ascii_uppercase() || c == '.') && !n.contains([' ', '"', '=', ':', '#'])
}

impl fmt::Display for Arg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Arg::Number(n) => write!(f, "{n}"),
            Arg::Bool(b) => write!(f, "{b}"),
            Arg::Text(t) => {
                f.write_str("\"")?;
                for c in t.chars() {
                    match c {
                        '"' => f.write_str("\\\"")?,
                        '\\' => f.write_str("\\\\")?,
                        '\n' => f.write_str("\\n")?,
                        c => write!(f, "{c}")?,
                    }
                }
                f.write_str("\"")
            }
            Arg::Name(n) if bare_name(n) => f.write_str(n),
            Arg::Name(n) => write!(f, "name:{n}"),
            Arg::Dir(Dir::None) => f.write_str("dir:none"),
            Arg::Dir(d) => f.write_str(d.name()),
            Arg::Side(s) => write!(f, "side:{}", s.name()),
            Arg::Pos { map, x, y } => write!(f, "{map}:{x},{y}"),
            Arg::Entity(e) => write!(f, "#{e}"),
            Arg::None => f.write_str("none"),
        }
    }
}

impl fmt::Display for Row {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.time)?;
        if let Some(n) = self.seat {
            write!(f, " {n}")?;
        }
        match &self.act {
            Act::Command { name, args } => {
                write!(f, " {name}")?;
                for (k, v) in args {
                    match k {
                        Some(k) => write!(f, " {k}={v}")?,
                        None => write!(f, " {v}")?,
                    }
                }
                Ok(())
            }
            Act::TravelTo { map, x, y } => write!(f, " travel_to {map}:{x},{y}"),
            Act::Follow(e) => write!(f, " follow #{e}"),
            Act::Step => f.write_str(" step"),
            Act::Stop => f.write_str(" stop"),
            Act::Join => f.write_str(" join"),
            Act::Leave => f.write_str(" leave_seat"),
            Act::SceneConfirm => f.write_str(" scene.confirm"),
            Act::SceneSkip => f.write_str(" skip"),
            Act::SceneRestart => f.write_str(" restart"),
            Act::SceneRewind => f.write_str(" rewind"),
            Act::SceneChoose(n) => write!(f, " scene.choose {n}"),
            Act::SceneAdjust(n, s) => write!(f, " scene.adjust {n} {s:+}"),
            Act::SceneAnswer(a) => write!(f, " scene.answer {a}"),
            Act::SceneClose => f.write_str(" scene.close"),
            Act::End => f.write_str(" end"),
        }
    }
}

/// Splits a row into words; a quoted text is one word with its quotes.
fn words(line: &str) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    let mut escaped = false;
    for c in line.chars() {
        if quoted {
            cur.push(c);
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                quoted = false;
            }
        } else if c.is_whitespace() {
            if !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
            }
        } else {
            if c == '"' {
                quoted = true;
            }
            cur.push(c);
        }
    }
    if quoted {
        return Err("a text is not closed".into());
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    Ok(out)
}

fn read_text(w: &str) -> Result<String, String> {
    let inner = &w[1..w.len() - 1];
    let mut out = String::new();
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some(c @ ('"' | '\\')) => out.push(c),
                _ => return Err(format!("a bad escape in {w}")),
            }
        } else {
            out.push(c);
        }
    }
    Ok(out)
}

fn read_cell(w: &str) -> Option<(String, i32, i32)> {
    let (map, xy) = w.rsplit_once(':')?;
    let (x, y) = xy.split_once(',')?;
    Some((map.to_string(), x.parse().ok()?, y.parse().ok()?))
}

fn read_arg(w: &str) -> Result<Arg, String> {
    if w.len() >= 2 && w.starts_with('"') && w.ends_with('"') {
        return read_text(w).map(Arg::Text);
    }
    if let Some(e) = w.strip_prefix('#') {
        return e
            .parse()
            .map(Arg::Entity)
            .map_err(|_| format!("a bad entity `{w}`"));
    }
    if let Ok(n) = w.parse() {
        return Ok(Arg::Number(n));
    }
    Ok(match w {
        "true" => Arg::Bool(true),
        "false" => Arg::Bool(false),
        "none" => Arg::None,
        _ => {
            if let Some(d) = Dir::parse(w.strip_prefix("dir:").unwrap_or(w)) {
                Arg::Dir(d)
            } else if let Some(s) = w.strip_prefix("side:") {
                Arg::Side(Side::parse(s).ok_or_else(|| format!("a bad side `{w}`"))?)
            } else if let Some(n) = w.strip_prefix("name:") {
                Arg::Name(n.to_string())
            } else if let Some((map, x, y)) = read_cell(w) {
                Arg::Pos { map, x, y }
            } else if bare_name(w) {
                Arg::Name(w.to_string())
            } else {
                return Err(format!("cannot read the argument `{w}`"));
            }
        }
    })
}

fn read_row(line: &str) -> Result<Row, String> {
    let ws = words(line)?;
    let time: i64 = ws
        .first()
        .and_then(|w| w.parse().ok())
        .ok_or("a row starts with its moment")?;
    if ws.get(1).map(String::as_str) == Some("end") {
        return Ok(Row {
            time,
            seat: None,
            act: Act::End,
        });
    }
    let seat: SeatId = ws
        .get(1)
        .and_then(|w| w.parse().ok())
        .ok_or("a seat number after the moment")?;
    let what = ws.get(2).ok_or("an input after the seat")?;
    let rest = &ws[3..];
    let act = match what.as_str() {
        "travel_to" => {
            let (map, x, y) = rest
                .first()
                .and_then(|w| read_cell(w))
                .ok_or("`travel_to` takes a cell MAP:x,y")?;
            Act::TravelTo { map, x, y }
        }
        "follow" => match rest.first().map(|w| read_arg(w)) {
            Some(Ok(Arg::Entity(e))) => Act::Follow(e),
            _ => return Err("`follow` takes an entity #N".into()),
        },
        "step" => Act::Step,
        "stop" => Act::Stop,
        "join" => Act::Join,
        "leave_seat" => Act::Leave,
        "scene.confirm" => Act::SceneConfirm,
        "skip" => Act::SceneSkip,
        "restart" => Act::SceneRestart,
        "rewind" => Act::SceneRewind,
        "scene.close" => Act::SceneClose,
        "scene.choose" => {
            Act::SceneChoose(rest.first().ok_or("`scene.choose` takes an item")?.clone())
        }
        "scene.adjust" => Act::SceneAdjust(
            rest.first().ok_or("`scene.adjust` takes an item")?.clone(),
            rest.get(1)
                .and_then(|s| s.parse().ok())
                .ok_or("`scene.adjust` takes +1 or -1")?,
        ),
        "scene.answer" => Act::SceneAnswer(read_arg(
            rest.first().ok_or("`scene.answer` takes a value")?,
        )?),
        name => {
            let mut args = Vec::new();
            for w in rest {
                let named = !w.starts_with('"')
                    && w.split_once('=').is_some_and(|(k, _)| {
                        !k.is_empty() && k.chars().all(|c| c.is_alphanumeric() || c == '_')
                    });
                if named {
                    let (k, v) = w.split_once('=').unwrap();
                    args.push((Some(k.to_string()), read_arg(v)?));
                } else {
                    args.push((None, read_arg(w)?));
                }
            }
            Act::Command {
                name: name.to_string(),
                args,
            }
        }
    };
    Ok(Row {
        time,
        seat: Some(seat),
        act,
    })
}

impl InputLog {
    /// The text form, or why the log cannot be written.
    pub fn write(&self) -> Result<String, String> {
        if let Some(e) = &self.broken {
            return Err(e.clone());
        }
        let mut out = format!("rlml-input 1\nseed {}\nseats {}\n", self.seed, self.seats);
        if !self.profile.is_empty() {
            out.push_str(&format!(
                "profile {}\n",
                super::save::write_value(&self.profile)
            ));
        }
        if !self.mods.is_empty() {
            out.push_str(&format!("mods {}\n", self.mods.join(" ")));
        }
        if let Some((hash, save)) = &self.from {
            out.push_str(&format!("from {hash:016x}\n"));
            for l in save.lines() {
                out.push_str(&format!("| {l}\n"));
            }
        }
        for r in &self.rows {
            out.push_str(&r.to_string());
            out.push('\n');
        }
        Ok(out)
    }

    pub fn read(text: &str) -> Result<InputLog, String> {
        let mut lines = text
            .lines()
            .enumerate()
            .filter(|(_, l)| !l.trim().is_empty());
        let at = |i: usize, e: String| format!("input log line {}: {e}", i + 1);
        match lines.next() {
            Some((_, l)) if l.trim() == "rlml-input 1" => {}
            _ => return Err("not an input log: the first line is `rlml-input 1`".into()),
        }
        let mut log = InputLog::default();
        let mut header = |key: &str| -> Result<String, String> {
            match lines.next() {
                Some((i, l)) => l
                    .trim()
                    .strip_prefix(key)
                    .map(|v| v.trim().to_string())
                    .ok_or_else(|| at(i, format!("`{key} …` expected"))),
                None => Err(format!("the input log has no `{key}` line")),
            }
        };
        log.seed = header("seed")?
            .parse()
            .map_err(|_| "a bad seed".to_string())?;
        log.seats = header("seats")?
            .parse()
            .map_err(|_| "a bad seat count".to_string())?;
        let mut lines = lines.peekable();
        if let Some((i, l)) = lines.peek().copied()
            && let Some(p) = l.trim().strip_prefix("profile ")
        {
            log.profile = super::save::read_value(p).map_err(|e| at(i, e))?;
            lines.next();
        }
        if let Some((_, l)) = lines.peek().copied()
            && let Some(m) = l.trim().strip_prefix("mods ")
        {
            log.mods = m.split_whitespace().map(str::to_string).collect();
            lines.next();
        }
        // a segment from a reload: its hash and its save, a line each
        // behind `| `
        if let Some((i, l)) = lines.peek().copied()
            && let Some(h) = l.trim().strip_prefix("from ")
        {
            let hash = u64::from_str_radix(h.trim(), 16).map_err(|_| at(i, "a bad hash".into()))?;
            lines.next();
            let mut save = String::new();
            while let Some((_, l)) = lines.peek().copied()
                && let Some(line) = l.strip_prefix("| ").or(l.strip_prefix("|"))
            {
                save.push_str(line);
                save.push('\n');
                lines.next();
            }
            log.from = Some((hash, save));
        }
        for (i, l) in lines {
            log.rows.push(read_row(l.trim()).map_err(|e| at(i, e))?);
        }
        Ok(log)
    }
}

impl<'m> Runner<'m> {
    /// Writes a row at the moment now.
    pub(crate) fn record(&mut self, seat: Option<SeatId>, act: Act) {
        let time = self.w.s.game.time;
        self.inputs.rows.push(Row { time, seat, act });
    }

    pub(crate) fn record_command(&mut self, n: SeatId, cmd: &CommandValue) {
        let mut args = Vec::new();
        for (k, v) in &cmd.args {
            match Arg::of(v, &self.w.s) {
                Ok(a) => args.push((k.clone(), a)),
                Err(e) => {
                    self.inputs.broken.get_or_insert(e);
                    return;
                }
            }
        }
        self.record(
            Some(n),
            Act::Command {
                name: cmd.name.clone(),
                args,
            },
        );
    }

    pub(crate) fn record_cell(&mut self, n: SeatId, p: Pos) {
        let map = self.w.s.maps[p.map].name.clone();
        self.record(
            Some(n),
            Act::TravelTo {
                map,
                x: p.x,
                y: p.y,
            },
        );
    }

    /// The log so far, with its `end` row at the moment now.
    pub fn input_log(&self) -> InputLog {
        let mut log = self.inputs.clone();
        log.rows.push(Row {
            time: self.w.s.game.time,
            seat: None,
            act: Act::End,
        });
        log
    }
}

/// A replay of a log, row by row (M 4.4).
pub struct Replay<'m> {
    pub r: Runner<'m>,
    status: Status,
    rows: Vec<Row>,
    next: usize,
}

impl<'m> Replay<'m> {
    /// The log's start: a new game with its seed and seats, run to the
    /// first turn.
    pub fn start(m: &'m Model, log: &InputLog) -> RtResult<Replay<'m>> {
        // the log's mods are the game's rules too (M 4.4)
        let mods: Vec<String> = m.mods.iter().map(|(n, _)| n.clone()).collect();
        if mods != log.mods {
            return Err(format!(
                "the input log was played with the mods [{}], and these are [{}]",
                log.mods.join(" "),
                mods.join(" ")
            ));
        }
        // a segment from a reload starts from its save, with the files it
        // was played with (M 4.4)
        if let Some((hash, save)) = &log.from {
            if *hash != m.source_hash {
                return Err(format!(
                    "the input log's segment was played with other files (hash {hash:016x}, these are {:016x})",
                    m.source_hash
                ));
            }
            let save = super::save::Save::read(save)?;
            let mut r = Runner::load(m, &save)?;
            r.inputs.from = log.from.clone();
            let status = r.run(0)?;
            return Ok(Replay {
                r,
                status,
                rows: log.rows.clone(),
                next: 0,
            });
        }
        let mut w = World::new_game_with(m, log.seed, Some(log.seats), Some(&log.profile))?;
        w.begin()?;
        let mut r = Runner::new(w);
        let status = r.run(0)?;
        Ok(Replay {
            r,
            status,
            rows: log.rows.clone(),
            next: 0,
        })
    }

    /// A replay from a game under way, as from a save (M 4.4): `log`'s
    /// rows after the ones `r` has written. Travels and orders are not
    /// state, so they are dropped.
    pub fn from_runner(mut r: Runner<'m>, log: &InputLog) -> RtResult<Replay<'m>> {
        let done = r.inputs.rows.len();
        r.travels.clear();
        r.orders.clear();
        let status = r.run(0)?;
        let rows = log.rows.get(done..).unwrap_or_default().to_vec();
        Ok(Replay {
            r,
            status,
            rows,
            next: 0,
        })
    }

    /// Runs every row left.
    pub fn finish(mut self) -> RtResult<Runner<'m>> {
        while self.step()?.is_some() {}
        Ok(self.r)
    }

    /// Runs the next row and gives it; `None` after the last one.
    pub fn step(&mut self) -> RtResult<Option<Row>> {
        let Some(row) = self.rows.get(self.next).cloned() else {
            return Ok(None);
        };
        self.next += 1;
        let line = self.next;
        let err = |e: String| format!("input log row {line} ({row}): {e}");
        // rounds without a turn up to the row's moment
        while self.r.w.s.game.time < row.time && self.status != Status::Input {
            let before = self.r.w.s.game.time;
            self.status = self.r.run(1)?;
            if self.r.w.s.game.time == before {
                break;
            }
        }
        let now = self.r.w.s.game.time;
        if now != row.time {
            return Err(err(format!(
                "the game is at time {now}, not {}: the data or the engine has changed since the log was written",
                row.time
            )));
        }
        let seat = row.seat.unwrap_or(1);
        match &row.act {
            Act::Command { name, args } => {
                let args = args
                    .iter()
                    .map(|(k, a)| Ok((k.clone(), a.value(&self.r.w.s)?)))
                    .collect::<RtResult<Vec<_>>>()
                    .map_err(err)?;
                self.r
                    .input(
                        seat,
                        &CommandValue {
                            name: name.clone(),
                            args,
                        },
                    )
                    .map_err(err)?;
                self.status = self.r.run(0)?;
            }
            Act::TravelTo { map, x, y } => {
                let p = pos(&self.r.w.s, map, *x, *y).map_err(err)?;
                if !self.r.travel_to(seat, p).map_err(err)? {
                    return Err(err("the travel does not start".into()));
                }
            }
            Act::Follow(e) => {
                if !self.r.follow(seat, Some(*e)).map_err(err)? {
                    return Err(err("the following does not start".into()));
                }
            }
            Act::Step => {
                if !self.r.travels.contains_key(&seat) {
                    return Err(err("the seat does not travel".into()));
                }
                self.r.step_travel().map_err(err)?;
                self.status = self.r.run(0)?;
            }
            Act::Stop => self.r.stop_travel(seat),
            Act::Join => {
                if self.r.join().map_err(err)? != Some(seat) {
                    return Err(err("the seat does not join as it did".into()));
                }
                self.status = self.r.run(0)?;
            }
            Act::SceneConfirm
            | Act::SceneSkip
            | Act::SceneRestart
            | Act::SceneRewind
            | Act::SceneChoose(_)
            | Act::SceneAdjust(..)
            | Act::SceneAnswer(_)
            | Act::SceneClose => {
                if !self.r.in_scene() {
                    return Err(err("no scene waits".into()));
                }
                let ok = match &row.act {
                    Act::SceneConfirm => self.r.scene_confirm().map(|_| true),
                    Act::SceneSkip => self.r.scene_skip().map(|_| true),
                    Act::SceneRestart => self.r.scene_restart().map(|_| true),
                    Act::SceneRewind => self.r.scene_rewind().map(|_| true),
                    Act::SceneChoose(n) => self.r.scene_choose(n),
                    Act::SceneAdjust(n, s) => self.r.scene_adjust(n, *s),
                    Act::SceneAnswer(a) => {
                        let v = a.value(&self.r.w.s).map_err(err)?;
                        self.r.scene_answer(v)
                    }
                    _ => self.r.scene_closed(),
                }
                .map_err(err)?;
                if !ok {
                    return Err(err("the scene does not take the answer".into()));
                }
                self.status = self.r.run(0)?;
            }
            Act::Leave => {
                if !self.r.leave_seat(seat).map_err(err)? {
                    return Err(err("the seat does not leave".into()));
                }
                self.status = self.r.run(0)?;
            }
            Act::End => {}
        }
        Ok(Some(row))
    }

    /// Runs every row from the log's start.
    pub fn run(m: &'m Model, log: &InputLog) -> RtResult<Runner<'m>> {
        Replay::start(m, log)?.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_read_back() {
        let text = "rlml-input 1\nseed 7\nseats 2\n0 1 STD.MOVE dir=north\n120 2 STD.SAY \"a \\\"b\\\"\" to=#3 at=ROOM:4,5 side:left 2 name:low\n240 2 travel_to ROOM:14,6\n360 2 follow #4\n360 2 step\n480 2 stop\n480 end\n";
        let log = InputLog::read(text).unwrap();
        assert_eq!(log.seed, 7);
        assert_eq!(log.seats, 2);
        assert_eq!(log.rows.len(), 7);
        assert_eq!(
            log.rows[1].act,
            Act::Command {
                name: "STD.SAY".into(),
                args: vec![
                    (None, Arg::Text("a \"b\"".into())),
                    (Some("to".into()), Arg::Entity(3)),
                    (
                        Some("at".into()),
                        Arg::Pos {
                            map: "ROOM".into(),
                            x: 4,
                            y: 5
                        }
                    ),
                    (None, Arg::Side(Side::parse("left").unwrap())),
                    (None, Arg::Number(2)),
                    (None, Arg::Name("low".into())),
                ]
            }
        );
        assert_eq!(log.write().unwrap(), text);
    }

    #[test]
    fn a_bad_log_says_where() {
        let e =
            InputLog::read("rlml-input 1\nseed 7\nseats 1\n0 1 STD.MOVE dir=\"open\n").unwrap_err();
        assert!(e.contains("line 4"), "{e}");
        assert!(InputLog::read("seed 7\n").is_err());
    }
}
