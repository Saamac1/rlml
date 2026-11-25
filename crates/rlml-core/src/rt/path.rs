//! A waiting command's place as a structural path (M 6.1.6, RS 11.1): the
//! number of its wait point and the state of each loop around it, outer
//! to inner. The interpreter runs frames; a wait turns them into a path,
//! and going on turns the path back into frames over the body as it is.
//!
//! Wait points (`wait N`, `wait scene`, `wait confirm`, `open`, and the
//! `wait number(…)` and `open MENU(…)` assignments) and loops (`repeat`,
//! `for`, `while`) are numbered in source order from 0. A wait point is in
//! exactly one place in the body, so its number tells the statement and
//! every `if` arm and loop around it. The body's hash leaves out where
//! things are written, so comments and spacing do not change it.

use super::eval::Frame;
use super::value::Value;
use super::world::RtResult;
use crate::rs::ast::{Pos, Stmt, StmtKind};

/// The state of a loop around the wait point.
#[derive(Debug, Clone, PartialEq)]
pub enum LoopState {
    /// `repeat N`: the round from 0, and N as counted at the loop's start.
    Repeat { round: i64, count: i64 },
    /// `for`: the set fixed at the loop's start and the index of the
    /// member now.
    For { items: Vec<Value>, index: usize },
    /// `while`: nothing; the condition is read at the next round.
    While,
}

/// A loop on the path: its number in the body and its state.
#[derive(Debug, Clone, PartialEq)]
pub struct PathLoop {
    pub loop_no: usize,
    pub state: LoopState,
}

fn is_wait(s: &Stmt) -> bool {
    matches!(
        s.kind,
        StmtKind::Wait(_)
            | StmtKind::WaitInput { .. }
            | StmtKind::Open(_)
            | StmtKind::OpenValue { .. }
    )
}

fn is_loop(s: &Stmt) -> bool {
    matches!(
        s.kind,
        StmtKind::For { .. } | StmtKind::Repeat { .. } | StmtKind::While { .. }
    )
}

/// The bodies inside a statement, in source order: `if` arms and then
/// `else`, `match` cases, a loop's body.
fn children(s: &Stmt) -> Vec<&[Stmt]> {
    match &s.kind {
        StmtKind::If { arms, otherwise } => arms
            .iter()
            .map(|(_, b)| b.as_slice())
            .chain(otherwise.as_deref())
            .collect(),
        StmtKind::Match { cases, .. } => cases.iter().map(|(_, b)| b.as_slice()).collect(),
        StmtKind::For { body, .. }
        | StmtKind::Repeat { body, .. }
        | StmtKind::While { body, .. } => vec![body.as_slice()],
        _ => vec![],
    }
}

/// The positions of the statements `pick` chooses, in source order.
fn numbered(body: &[Stmt], pick: fn(&Stmt) -> bool, out: &mut Vec<Pos>) {
    for s in body {
        if pick(s) {
            out.push(s.pos);
        }
        for c in children(s) {
            numbered(c, pick, out);
        }
    }
}

fn number_of(body: &[Stmt], pick: fn(&Stmt) -> bool, at: Pos) -> Option<usize> {
    let mut all = Vec::new();
    numbered(body, pick, &mut all);
    all.iter().position(|p| *p == at)
}

/// The body's hash: FNV-1a over its parse tree without positions.
pub fn body_hash(body: &[Stmt]) -> u64 {
    let tree = format!("{body:?}");
    // `pos: Pos { line: 3, col: 5 }` goes, so moving code does not count
    let mut text = String::with_capacity(tree.len());
    let mut rest = tree.as_str();
    while let Some(i) = rest.find("pos: Pos {") {
        text.push_str(&rest[..i]);
        rest = &rest[i..];
        rest = rest.find('}').map_or("", |j| &rest[j + 1..]);
    }
    text.push_str(rest);
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in text.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

/// The statement a block is at: the one it ran last.
fn at(f: &Frame) -> Option<&Stmt> {
    match f {
        Frame::Block { stmts, i } if *i > 0 => stmts.get(*i - 1),
        _ => None,
    }
}

/// The path of frames stopped at a wait point.
pub fn path_of(body: &[Stmt], frames: &[Frame]) -> RtResult<(usize, Vec<PathLoop>)> {
    let bad = || "a waiting command's frames do not match its body".to_string();
    let wait = frames
        .last()
        .and_then(at)
        .filter(|s| is_wait(s))
        .ok_or_else(bad)?;
    let point = number_of(body, is_wait, wait.pos).ok_or_else(bad)?;
    let mut loops = Vec::new();
    for (j, f) in frames.iter().enumerate() {
        let state = match f {
            Frame::Block { .. } => continue,
            Frame::Repeat { left, count, .. } => LoopState::Repeat {
                round: count - left - 1,
                count: *count,
            },
            Frame::For { items, k, .. } => LoopState::For {
                items: items.clone(),
                index: k.saturating_sub(1),
            },
            Frame::While { .. } => LoopState::While,
        };
        // the loop statement is where the block before it is
        let stmt = j
            .checked_sub(1)
            .and_then(|p| at(&frames[p]))
            .filter(|s| is_loop(s))
            .ok_or_else(bad)?;
        let loop_no = number_of(body, is_loop, stmt.pos).ok_or_else(bad)?;
        loops.push(PathLoop { loop_no, state });
    }
    Ok((point, loops))
}

/// The route to the statement at `target`: each level's statement index
/// and the body taken inside it.
fn route(body: &[Stmt], target: Pos) -> Option<Vec<(usize, Option<usize>)>> {
    for (i, s) in body.iter().enumerate() {
        if s.pos == target {
            return Some(vec![(i, None)]);
        }
        for (k, c) in children(s).into_iter().enumerate() {
            if let Some(mut r) = route(c, target) {
                r.insert(0, (i, Some(k)));
                return Some(r);
            }
        }
    }
    None
}

/// The statement of wait point `point`: what a waiting command waits for
/// is read from it (M 5.4).
pub fn wait_stmt(body: &[Stmt], point: usize) -> Option<Stmt> {
    let mut waits = Vec::new();
    numbered(body, is_wait, &mut waits);
    let target = *waits.get(point)?;
    let mut cur = body;
    for (i, child) in route(body, target)? {
        let Some(k) = child else {
            return cur.get(i).cloned();
        };
        cur = *children(&cur[i]).get(k)?;
    }
    None
}

/// Frames that go on after wait point `point`, the loops in their saved
/// states. The body must be the one the path was taken in: the caller
/// checks its hash.
pub fn frames_at(body: &[Stmt], point: usize, loops: &[PathLoop]) -> RtResult<Vec<Frame>> {
    let bad = || format!("a waiting command's path (wait point {point}) does not fit its body");
    let mut waits = Vec::new();
    numbered(body, is_wait, &mut waits);
    let target = *waits.get(point).ok_or_else(bad)?;
    let route = route(body, target).ok_or_else(bad)?;
    let mut saved = loops.iter();
    let mut frames = Vec::new();
    let mut cur: Vec<Stmt> = body.to_vec();
    for (i, child) in route {
        frames.push(Frame::Block {
            stmts: cur.clone(),
            i: i + 1,
        });
        let Some(k) = child else { break };
        let s = cur[i].clone();
        let next: Vec<Stmt> = children(&s).get(k).ok_or_else(bad)?.to_vec();
        match &s.kind {
            StmtKind::For { var, body, .. } => match saved.next().map(|l| &l.state) {
                Some(LoopState::For { items, index }) => frames.push(Frame::For {
                    var: var.clone(),
                    items: items.clone(),
                    k: index + 1,
                    body: body.clone(),
                }),
                _ => return Err(bad()),
            },
            StmtKind::Repeat { body, .. } => match saved.next().map(|l| &l.state) {
                Some(LoopState::Repeat { round, count }) => frames.push(Frame::Repeat {
                    left: count - round - 1,
                    count: *count,
                    body: body.clone(),
                }),
                _ => return Err(bad()),
            },
            StmtKind::While { cond, body } => match saved.next().map(|l| &l.state) {
                Some(LoopState::While) => frames.push(Frame::While {
                    cond: cond.clone(),
                    body: body.clone(),
                }),
                _ => return Err(bad()),
            },
            _ => {}
        }
        cur = next;
    }
    if saved.next().is_some() {
        return Err(bad());
    }
    Ok(frames)
}
