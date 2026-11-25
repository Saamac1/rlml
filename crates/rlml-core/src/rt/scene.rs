//! Foreground coroutine scenes (RS 9, M 6.1.7 step 6): the queue starts
//! at an action's end, a scene runs as frames like a waiting command, and
//! it stops where it waits for the player: `say` and `wait confirm`. While
//! a scene runs the game's time stands, and the scheduler gives
//! `Status::Scene` instead of running.
//!
//! The scene's draft (RS 9.8) is a snapshot taken when it starts: its
//! changes go into the state at once and show as the real state, and
//! `restart` loads the snapshot and runs the scene from its start. A scene
//! draws from a presentation stream of its own (M 4.1), so restarts and
//! skips go the same way and the game's stream does not move.

use super::checkpoint::Snapshot;
use super::eval::{BodyKind, Env, Flow, Frame};
use super::input::Act;
use super::rng::{Rng, mix, mix_name};
use super::run::Runner;
use super::state::{QueuedScene, SeatId};
use super::value::CommandValue;
use super::world::{Pause, RtResult};
use crate::rs::ast::{ItemKind, Stmt};

/// A scene or a `wait scene` scene inside it.
#[derive(Debug, Clone)]
struct Level {
    frames: Vec<Frame>,
    env: Env,
}

/// The foreground scene running now.
#[derive(Debug, Clone)]
pub struct ActiveScene {
    /// The scene as it was started, for `restart`.
    started: QueuedScene,
    /// Its presentation stream's start, for `restart`.
    rng: Rng,
    /// The state when it started: the draft's base (RS 9.8).
    draft: Box<Snapshot>,
    /// The log's length when it started; a restart drops later messages.
    log_len: usize,
    /// The scenes inside it by `wait scene`, the outermost first.
    stack: Vec<Level>,
    /// The seat that answers it (M 8.2).
    pub seat: Option<SeatId>,
}

impl<'m> Runner<'m> {
    /// A foreground scene runs.
    pub fn in_scene(&self) -> bool {
        self.scene.is_some()
    }

    /// What the scene waits for: a line to confirm or a confirmation.
    pub fn scene_pause(&self) -> Option<&Pause> {
        self.scene.as_ref().and(self.w.pause.as_ref())
    }

    /// The level that runs `call`: its body as frames, its parameters bound,
    /// the scene's map and its presentation stream.
    fn scene_level(&mut self, call: &CommandValue, map: Option<super::value::MapId>, rng: Rng, opener: Option<super::value::EntityId>) -> RtResult<Level> {
        let (item, ns, file) = self.w.decl_of(&call.name).ok_or_else(|| format!("no scene `{}`", call.name))?;
        let ItemKind::Scene { params, body } = &item.kind else { return Err(format!("`{}` is not a coroutine scene", call.name)) };
        let mut env = Env::new(BodyKind::Scene, &call.name, ns.clone(), &file);
        env.map = map;
        env.actor = opener;
        env.rng = Some(rng);
        self.w.bind_params(params, call.args.clone(), &mut env, ns.as_deref(), item.pos)?;
        let body: Vec<Stmt> = body.clone();
        Ok(Level { frames: vec![Frame::Block { stmts: body, i: 0 }], env })
    }

    /// The scene's map (RS 9): the opener seat's controlled member's, or
    /// without one the map it was started on.
    fn scene_map(&self, q: &QueuedScene) -> Option<super::value::MapId> {
        let member = q.opener.and_then(|o| self.w.s.seat_of(o)).and_then(|n| self.w.s.active(n)).or(self.w.s.player);
        member.and_then(|p| self.w.s.at(p)).map(|p| p.map).or(q.map)
    }

    /// The queue's next scenes start, one after another, until one waits for
    /// the player or the queue is empty (M 6.1.7 step 6).
    pub(crate) fn start_scenes(&mut self) -> RtResult<()> {
        while self.scene.is_none() && !self.w.s.scene_queue.is_empty() {
            let q = self.w.s.scene_queue.remove(0);
            let n = self.w.s.scene_starts;
            self.w.s.scene_starts += 1;
            // the presentation stream: the seed, the scene's name and its
            // start's number (M 4.1)
            let rng = Rng::from_seed(mix(mix_name(self.w.s.game.seed, &q.call.name), n));
            let draft = Box::new(self.snapshot());
            let map = self.scene_map(&q);
            let level = self.scene_level(&q.call, map, rng.clone(), q.opener)?;
            let seat = q.opener.and_then(|o| self.w.s.seat_of(o));
            self.scene = Some(Box::new(ActiveScene { started: q, rng, draft, log_len: self.w.log.len(), stack: vec![level], seat }));
            self.run_scene()?;
        }
        Ok(())
    }

    /// Runs the scene on until it waits for the player or ends. Its end is
    /// an action's end: its travels, saves and the next queued scene.
    fn run_scene(&mut self) -> RtResult<()> {
        loop {
            let Some(scene) = self.scene.as_mut() else { return Ok(()) };
            let Some(top) = scene.stack.last_mut() else {
                self.scene = None;
                self.w.scene_skipping = false;
                return self.end_action();
            };
            let (mut frames, mut env) = (std::mem::take(&mut top.frames), top.env.clone());
            let flow = self.w.run_frames(&mut frames, &mut env);
            let scene = self.scene.as_mut().expect("the scene runs");
            let top = scene.stack.last_mut().expect("its level runs");
            top.frames = frames;
            top.env = env;
            match flow? {
                Flow::Pause => return Ok(()),
                Flow::Sub(call) => {
                    let (map, rng, opener) = (top.env.map, top.env.rng.clone().unwrap_or_else(|| scene.rng.clone()), top.env.actor);
                    let level = self.scene_level(&call, map, rng, opener)?;
                    self.scene.as_mut().expect("the scene runs").stack.push(level);
                }
                // a scene's `return` or end: back to the scene that waited
                _ => {
                    let scene = self.scene.as_mut().expect("the scene runs");
                    let done = scene.stack.pop().expect("its level ran");
                    // a sub-scene's presentation stream goes on in its caller
                    if let (Some(caller), Some(rng)) = (scene.stack.last_mut(), done.env.rng) {
                        caller.env.rng = Some(rng);
                    }
                }
            }
        }
    }

    fn scene_seat(&self) -> Option<SeatId> {
        self.scene.as_ref().and_then(|s| s.seat).or(Some(1))
    }

    /// `confirm` in a scene: the line or the confirmation it waits for is
    /// confirmed, and it goes on (RS 9.5).
    pub fn scene_confirm(&mut self) -> RtResult<()> {
        if self.scene_pause().is_none() {
            return Ok(());
        }
        let seat = self.scene_seat();
        self.record(seat, Act::SceneConfirm);
        self.w.pause = None;
        self.run_scene()
    }

    /// `skip`: the rest runs by the skipping rules, lines left out (RS 9.8).
    pub fn scene_skip(&mut self) -> RtResult<()> {
        if !self.in_scene() {
            return Ok(());
        }
        let seat = self.scene_seat();
        self.record(seat, Act::SceneSkip);
        self.w.scene_skipping = true;
        self.w.pause = None;
        self.run_scene()
    }

    /// `restart`: the draft goes, and the scene runs from its start
    /// (RS 9.8).
    pub fn scene_restart(&mut self) -> RtResult<()> {
        let Some(scene) = self.scene.take() else { return Ok(()) };
        let seat = scene.seat.or(Some(1));
        self.record(seat, Act::SceneRestart);
        self.restore(&scene.draft);
        self.w.log.truncate(scene.log_len);
        self.w.pause = None;
        self.w.scene_skipping = false;
        let map = self.scene_map(&scene.started);
        let level = self.scene_level(&scene.started.call, map, scene.rng.clone(), scene.started.opener)?;
        self.scene = Some(Box::new(ActiveScene { stack: vec![level], ..*scene }));
        self.run_scene()
    }
}
