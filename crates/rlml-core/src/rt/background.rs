//! Background scenes (RS 9.9): timelines beside the turns.
//!
//! A `background` scene runs on the game's clock in rounds: its rows run
//! at round boundaries after the round's events (M 6.1.4 phase 3), and its
//! tweens' steps are wakes between the actors' (phase 4), each a cell at
//! its microstep. Every moment of a scene is an action (M 6.1.7). The
//! scene is state: its handle, call, `scope`, `bind` and how far its
//! timeline has run go into a save, and its rows follow again from the
//! call (RS 11.1).
//!
//! A `background realtime` scene runs on the front end's clock and only
//! presents: its clock is not state, so a replay without a front end is
//! the same.

use std::collections::BTreeMap;

use super::eval::{BodyKind, Env};
use super::run::Runner;
use super::timeline::{Plan, TlFlow, TlState};

/// `background realtime` scenes' clocks, by handle; `None` once ended.
pub(crate) type Realtime = BTreeMap<u64, Option<TlState>>;
use super::value::CommandValue;
use super::world::RtResult;
use crate::rs::ast::ItemKind;

/// A background scene's rows and names, laid out once from its call.
#[derive(Debug, Clone)]
pub(crate) struct BgPlan {
    call: CommandValue,
    plan: Plan,
    env: Env,
}

/// The rows laid out, by handle; not state.
pub(crate) type BgPlans = BTreeMap<u64, BgPlan>;

impl<'m> Runner<'m> {
    /// The rows and names of background scene `i`, from the cache or laid
    /// out again from its call, as after loading a save.
    fn bg_plan(&mut self, i: usize) -> RtResult<(Plan, Env)> {
        let b = &self.w.s.backgrounds[i];
        if let Some(c) = self.bg_plans.get(&b.handle)
            && c.call == b.call
        {
            return Ok((c.plan.clone(), c.env.clone()));
        }
        let (handle, call, map, opener) = (b.handle, b.call.clone(), b.map, b.opener);
        let (item, ns, file) = self
            .w
            .decl_of(&call.name)
            .ok_or_else(|| format!("no scene `{}`", call.name))?;
        let ItemKind::Timeline { params, timeline } = &item.kind else {
            return Err(format!("`{}` is not a timeline", call.name));
        };
        let mut env = Env::new(BodyKind::Scene, &call.name, ns.clone(), &file);
        env.map = map;
        env.actor = opener;
        self.w
            .bind_params(params, call.args.clone(), &mut env, ns.as_deref(), item.pos)?;
        // a game-time scene counts rounds of 120 microsteps (M 6.1)
        let unit = if timeline.realtime { 1 } else { 120 };
        let plan = self.w.timeline_new(timeline, &mut env, unit)?.plan;
        self.bg_plans.insert(
            handle,
            BgPlan {
                call,
                plan: plan.clone(),
                env: env.clone(),
            },
        );
        Ok((plan, env))
    }

    /// Background scenes end when an entity they are bound to is gone
    /// (`removed`) or the last member leaves their `scope: map` (RS 9.9).
    pub(crate) fn prune_backgrounds(&mut self) {
        let s = &self.w.s;
        let here: Vec<_> = s
            .players()
            .into_iter()
            .filter_map(|p| s.at(p))
            .map(|p| p.map)
            .collect();
        let ended: Vec<u64> = s
            .backgrounds
            .iter()
            .filter(|b| {
                let unbound = b
                    .bind
                    .iter()
                    .any(|&e| e >= s.entities.len() || !s.get(e).alive);
                let left = b.scope_map && !b.map.is_some_and(|m| here.contains(&m));
                unbound || left
            })
            .map(|b| b.handle)
            .collect();
        if !ended.is_empty() {
            self.w.s.backgrounds.retain(|b| !ended.contains(&b.handle));
        }
        let live: Vec<u64> = self.w.s.backgrounds.iter().map(|b| b.handle).collect();
        self.bg_plans.retain(|h, _| live.contains(h));
        self.realtime.retain(|h, _| live.contains(h));
    }

    /// Phase 3 of a round boundary: the game-time background scenes' rows
    /// of this round, in starting order (M 6.1.4).
    pub(crate) fn background_round(&mut self) -> RtResult<()> {
        let now = self.w.s.game.time;
        let handles: Vec<u64> = self
            .w
            .s
            .backgrounds
            .iter()
            .filter(|b| !b.realtime)
            .map(|b| b.handle)
            .collect();
        for h in handles {
            self.run_background(h, now)?;
        }
        Ok(())
    }

    /// The next game-time tween step of a background scene: its moment and
    /// the scene's handle (M 6.1.4 phase 4).
    pub(crate) fn next_background_step(&self) -> Option<(i64, u64)> {
        self.w
            .s
            .backgrounds
            .iter()
            .filter(|b| !b.realtime)
            .filter_map(|b| Some((b.base? + b.st.next_step()?, b.handle)))
            .min()
    }

    /// Background scene `h` runs up to the game's moment `now`; the moment
    /// is an action.
    pub(crate) fn run_background(&mut self, h: u64, now: i64) -> RtResult<()> {
        let Some(i) = self.w.s.backgrounds.iter().position(|b| b.handle == h) else {
            return Ok(());
        };
        let (plan, mut env) = self.bg_plan(i)?;
        let b = &mut self.w.s.backgrounds[i];
        // its clock starts at the first boundary after the start
        if b.base.is_none() {
            b.base = Some(now);
            b.st = plan.fresh();
        }
        let base = b.base.unwrap_or(now);
        let mut st = std::mem::take(&mut b.st);
        let flow = loop {
            match self
                .w
                .timeline_run(&plan, &mut st, &mut env, Some(now - base))
            {
                // a background scene takes no input: until it shows its
                // window, a `say` is its line in the log
                Ok(TlFlow::Pause) => {
                    self.w.pause = None;
                }
                other => break other,
            }
        };
        if let Some(b) = self.w.s.backgrounds.iter_mut().find(|b| b.handle == h) {
            b.st = st;
        }
        match flow? {
            TlFlow::Done => self.w.s.backgrounds.retain(|b| b.handle != h),
            TlFlow::Sub(call) => self.w.warnings.push(format!(
                "a background scene does not wait for `{}`: it is left out",
                call.name
            )),
            _ => {}
        }
        self.end_action()
    }

    /// A `background realtime` scene runs: the front end ticks often.
    pub fn realtime_runs(&self) -> bool {
        self.w
            .s
            .backgrounds
            .iter()
            .any(|b| b.realtime && self.realtime.get(&b.handle) != Some(&None))
    }

    /// The front end's tick for `background realtime` scenes: they present
    /// and do not change the state, so their clock is the front end's
    /// (RS 9.9).
    pub fn realtime_tick(&mut self, ms: i64) -> RtResult<()> {
        let handles: Vec<(usize, u64)> = self
            .w
            .s
            .backgrounds
            .iter()
            .enumerate()
            .filter(|(_, b)| b.realtime)
            .map(|(i, b)| (i, b.handle))
            .collect();
        for (i, h) in handles {
            // a scene that has ended stays in the state until `stop`, but
            // shows no more
            let st = match self.realtime.get(&h) {
                Some(None) => continue,
                Some(Some(st)) => Some(st.clone()),
                None => None,
            };
            let (plan, mut env) = self.bg_plan(i)?;
            let mut st = st.unwrap_or_else(|| plan.fresh());
            let until = st.clock + ms.max(0);
            let flow = self.w.timeline_run(&plan, &mut st, &mut env, Some(until))?;
            self.realtime
                .insert(h, (flow != TlFlow::Done).then_some(st));
            self.w.pause = None;
        }
        Ok(())
    }
}
