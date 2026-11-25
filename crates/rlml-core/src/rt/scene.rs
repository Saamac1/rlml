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
use super::value::{CommandValue, EntityId};
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
    /// The command that waits for it (M 6.1.6), woken when it ends.
    command: Option<EntityId>,
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
    fn scene_level(
        &mut self,
        call: &CommandValue,
        map: Option<super::value::MapId>,
        rng: Rng,
        opener: Option<super::value::EntityId>,
    ) -> RtResult<Level> {
        let (item, ns, file) = self
            .w
            .decl_of(&call.name)
            .ok_or_else(|| format!("no scene `{}`", call.name))?;
        let ItemKind::Scene { params, body } = &item.kind else {
            return Err(format!("`{}` is not a coroutine scene", call.name));
        };
        let mut env = Env::new(BodyKind::Scene, &call.name, ns.clone(), &file);
        env.map = map;
        env.actor = opener;
        env.rng = Some(rng);
        self.w
            .bind_params(params, call.args.clone(), &mut env, ns.as_deref(), item.pos)?;
        let body: Vec<Stmt> = body.clone();
        Ok(Level {
            frames: vec![Frame::Block { stmts: body, i: 0 }],
            env,
        })
    }

    /// The scene's map (RS 9): the opener seat's controlled member's, or
    /// without one the map it was started on.
    fn scene_map(&self, q: &QueuedScene) -> Option<super::value::MapId> {
        let member = q
            .opener
            .and_then(|o| self.w.s.seat_of(o))
            .and_then(|n| self.w.s.active(n))
            .or(self.w.s.player);
        member.and_then(|p| self.w.s.at(p)).map(|p| p.map).or(q.map)
    }

    /// The queue's next scenes start, one after another, until one waits for
    /// the player or the queue is empty, and then what a held command waits
    /// for (M 6.1.6, M 6.1.7 step 6).
    pub(crate) fn start_scenes(&mut self) -> RtResult<()> {
        loop {
            if self.scene.is_some() {
                break;
            }
            let held = if self.w.s.scene_queue.is_empty() {
                self.held()
            } else {
                None
            };
            if self.w.s.scene_queue.is_empty() && held.is_none() {
                break;
            }
            // `on scene`: the state before the scene, its start in the queue
            // or in the waiting command (M 5.4)
            if self.rules.on_scene && !self.auto_off && !self.scene_saved {
                self.due.push(self.save());
            }
            self.scene_saved = false;
            let n = self.w.s.scene_starts;
            self.w.s.scene_starts += 1;
            match held {
                Some(e) => {
                    let name = self
                        .waiting
                        .get(&e)
                        .map(|s| s.command.clone())
                        .unwrap_or_default();
                    let draft = Box::new(self.snapshot());
                    let log_len = self.w.log.len();
                    let rng = self.scene_rng(&name, n);
                    self.open_held(e, rng, draft, log_len)?;
                }
                None => {
                    let q = self.w.s.scene_queue.remove(0);
                    let rng = self.scene_rng(&q.call.name, n);
                    let draft = Box::new(self.snapshot());
                    let map = self.scene_map(&q);
                    let level = self.scene_level(&q.call, map, rng.clone(), q.opener)?;
                    let seat = q.opener.and_then(|o| self.w.s.seat_of(o));
                    self.scene = Some(Box::new(ActiveScene {
                        started: q,
                        rng,
                        draft,
                        log_len: self.w.log.len(),
                        stack: vec![level],
                        seat,
                        command: None,
                    }));
                    self.run_scene()?;
                }
            }
        }
        Ok(())
    }

    /// The presentation stream of a scene's start `n`: the seed, the name
    /// and the number (M 4.1).
    fn scene_rng(&self, name: &str, n: u64) -> Rng {
        Rng::from_seed(mix(mix_name(self.w.s.game.seed, name), n))
    }

    /// The command that waits, without a wake, for a scene, a confirmation,
    /// a menu or a field. One action holds at most one, and its end opens
    /// it; a loaded save opens it from its start (M 5.4).
    fn held(&self) -> Option<EntityId> {
        self.waiting
            .iter()
            .find(|(e, st)| self.w.s.wake_of(**e).is_none() && self.w.is_held(st))
            .map(|(e, _)| *e)
    }

    /// Opens what command `e` waits for: the scene of its `wait scene`, or
    /// its pause as a level of the command's own names, which a field's
    /// answer goes into.
    fn open_held(
        &mut self,
        e: EntityId,
        rng: Rng,
        draft: Box<Snapshot>,
        log_len: usize,
    ) -> RtResult<()> {
        let state = self.waiting.get(&e).cloned().ok_or("no waiting command")?;
        let mut env = state.env.clone();
        let flow = self.w.held_wait(&state, &mut env)?;
        let here = self.w.s.at(e).map(|p| p.map);
        let seat = self.w.s.seat_of(e);
        match flow {
            Some(Flow::Sub(call)) => {
                let started = QueuedScene {
                    call: call.clone(),
                    opener: Some(e),
                    map: here,
                };
                let map = self.scene_map(&started);
                let level = self.scene_level(&call, map, rng.clone(), Some(e))?;
                self.scene = Some(Box::new(ActiveScene {
                    started,
                    rng,
                    draft,
                    log_len,
                    stack: vec![level],
                    seat,
                    command: Some(e),
                }));
                self.run_scene()
            }
            Some(Flow::Pause) => {
                let started = QueuedScene {
                    call: CommandValue {
                        name: state.command.clone(),
                        args: vec![],
                    },
                    opener: Some(e),
                    map: here,
                };
                self.scene = Some(Box::new(ActiveScene {
                    started,
                    rng,
                    draft,
                    log_len,
                    stack: vec![Level {
                        frames: vec![],
                        env,
                    }],
                    seat,
                    command: Some(e),
                }));
                Ok(())
            }
            // nothing to wait for: the command goes on
            _ => {
                self.wake_held(e);
                Ok(())
            }
        }
    }

    /// A held command goes on as a new action now (M 6.1.6).
    fn wake_held(&mut self, e: EntityId) {
        if let Some(p) = self.w.s.at(e).filter(|_| self.w.s.get(e).alive) {
            let now = self.clock(p.map);
            self.w.s.schedule(e, now);
        } else {
            self.waiting.remove(&e);
        }
    }

    /// Runs the scene on until it waits for the player or ends. Its end is
    /// an action's end: its travels, saves and the next queued scene.
    fn run_scene(&mut self) -> RtResult<()> {
        loop {
            let Some(scene) = self.scene.as_mut() else {
                return Ok(());
            };
            let Some(top) = scene.stack.last_mut() else {
                let command = scene.command;
                self.scene = None;
                self.w.scene_skipping = false;
                if let Some(e) = command {
                    self.wake_held(e);
                }
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
                    let (map, rng, opener) = (
                        top.env.map,
                        top.env.rng.clone().unwrap_or_else(|| scene.rng.clone()),
                        top.env.actor,
                    );
                    let level = self.scene_level(&call, map, rng, opener)?;
                    self.scene
                        .as_mut()
                        .expect("the scene runs")
                        .stack
                        .push(level);
                }
                // a scene's `return` or end: back to the scene that waited
                _ => {
                    let scene = self.scene.as_mut().expect("the scene runs");
                    let done = scene.stack.pop().expect("its level ran");
                    // a held command's own level: its names, a field's
                    // answer in them, go back to it
                    if let (Some(e), BodyKind::Command, true) =
                        (scene.command, done.env.kind, scene.stack.is_empty())
                    {
                        let env = done.env.clone();
                        if let Some(w) = self.waiting.get_mut(&e) {
                            w.env = env;
                        }
                        continue;
                    }
                    let scene = self.scene.as_mut().expect("the scene runs");
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
        let Some(scene) = self.scene.take() else {
            return Ok(());
        };
        let seat = scene.seat.or(Some(1));
        self.record(seat, Act::SceneRestart);
        self.restore(&scene.draft);
        self.w.log.truncate(scene.log_len);
        self.w.pause = None;
        self.w.scene_skipping = false;
        if let Some(e) = scene.command {
            return self.open_held(e, scene.rng, scene.draft, scene.log_len);
        }
        let map = self.scene_map(&scene.started);
        let level = self.scene_level(
            &scene.started.call,
            map,
            scene.rng.clone(),
            scene.started.opener,
        )?;
        self.scene = Some(Box::new(ActiveScene {
            stack: vec![level],
            ..*scene
        }));
        self.run_scene()
    }

    /// The top level's frames and env, taken out while the world runs
    /// with them.
    fn with_top<T>(
        &mut self,
        f: impl FnOnce(&mut super::world::World<'m>, &mut Vec<Frame>, &mut Env) -> RtResult<T>,
    ) -> RtResult<T> {
        let scene = self.scene.as_mut().ok_or("no scene")?;
        let top = scene.stack.last_mut().ok_or("no scene level")?;
        let (mut frames, mut env) = (std::mem::take(&mut top.frames), top.env.clone());
        let out = f(&mut self.w, &mut frames, &mut env);
        let top = self
            .scene
            .as_mut()
            .and_then(|s| s.stack.last_mut())
            .ok_or("no scene level")?;
        top.frames = frames;
        top.env = env;
        out
    }

    /// A choice's item chosen (RS 9.7): its block runs, and the scene goes
    /// on after the choice. A hidden, dimmed or `adjust` item is no choice.
    pub fn scene_choose(&mut self, name: &str) -> RtResult<bool> {
        let Some(Pause::Choice { items, options, .. }) = self.w.pause.clone() else {
            return Ok(false);
        };
        let ok = items
            .iter()
            .any(|i| i.name == name && i.enabled && i.adjust.is_none());
        let Some(option) = options.iter().find(|o| o.name == name).filter(|_| ok) else {
            return Ok(false);
        };
        let seat = self.scene_seat();
        self.record(seat, Act::SceneChoose(name.to_string()));
        self.w.pause = None;
        if let crate::rs::ast::OptionBody::Block(body) = &option.body {
            let body = body.clone();
            self.with_top(|_, frames, _| {
                frames.push(Frame::Block { stmts: body, i: 0 });
                Ok(())
            })?;
        }
        self.run_scene()?;
        Ok(true)
    }

    /// An `adjust` item moved by one (`step` +1 or -1) (RS 9.7). The choice
    /// stays, its items seen again: a moved value may enable another item.
    pub fn scene_adjust(&mut self, name: &str, step: i64) -> RtResult<bool> {
        let Some(Pause::Choice { scene, options, .. }) = self.w.pause.clone() else {
            return Ok(false);
        };
        let Some(option) = options.iter().find(|o| o.name == name).cloned() else {
            return Ok(false);
        };
        let moved = self.with_top(|w, _, env| w.adjust(&option, step, env))?;
        if moved {
            let seat = self.scene_seat();
            self.record(seat, Act::SceneAdjust(name.to_string(), step.signum()));
            let items = self.with_top(|w, _, env| w.choice_items(&options, env))?;
            self.w.pause = Some(Pause::Choice {
                scene,
                items,
                options,
            });
        }
        Ok(moved)
    }

    /// A field's answer (RS 9.7): a number in its range, a text cut to its
    /// length, or `none` when the player cancels.
    pub fn scene_answer(&mut self, v: super::value::Value) -> RtResult<bool> {
        use super::value::Value;
        use super::world::AskKind;
        let Some(Pause::Ask { into, kind }) = self.w.pause.clone() else {
            return Ok(false);
        };
        let v = match (kind, v) {
            (_, Value::None) => Value::None,
            (AskKind::Number { lo, hi }, Value::Number(n)) if (lo..=hi).contains(&n) => {
                Value::Number(n)
            }
            (AskKind::Text { max }, Value::Text(t)) => Value::Text(t.chars().take(max).collect()),
            _ => return Ok(false),
        };
        let seat = self.scene_seat();
        match super::input::Arg::of(&v, &self.w.s) {
            Ok(a) => self.record(seat, Act::SceneAnswer(a)),
            Err(e) => {
                self.inputs.broken.get_or_insert(e);
            }
        }
        self.w.pause = None;
        self.with_top(|_, _, env| {
            env.locals.insert(into, v);
            Ok(())
        })?;
        self.run_scene()?;
        Ok(true)
    }

    /// The menu or panel `open` waited for closed (RS 9.7): `x = open …`
    /// gets `none` until list menus give their choice.
    pub fn scene_closed(&mut self) -> RtResult<bool> {
        let Some(Pause::Open { into, .. }) = self.w.pause.clone() else {
            return Ok(false);
        };
        let seat = self.scene_seat();
        self.record(seat, Act::SceneClose);
        self.w.pause = None;
        if let Some(name) = into {
            self.with_top(|_, _, env| {
                env.locals.insert(name, super::value::Value::None);
                Ok(())
            })?;
        }
        self.run_scene()?;
        Ok(true)
    }
}
