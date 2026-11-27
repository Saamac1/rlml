//! A game session without a screen: keys from the `keys` contexts
//! (D 11.3), turns from the scheduler and a text frame after each of the
//! player's turns. Same seed and keys, same transcript.

use super::input::{InputLog, Replay};
use super::run::{Runner, Status};
use super::value::{CommandValue, Value};
use super::world::{RtResult, World};
use crate::model::{Kind, Model};
use crate::names;

/// What a key does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// A command by full name.
    Command(String),
    /// `open MENU`; menus are not supported yet.
    Open(String),
    /// An engine action such as `pass` (M 9.1).
    Engine(String),
}

pub struct Session<'m> {
    pub r: Runner<'m>,
    pub lang: String,
    keys: Vec<(String, Action)>,
    log_seen: usize,
}

/// The game's keys: its base context and `ALWAYS` (D 11.3), a later row
/// winning.
pub fn keymap(m: &Model) -> Vec<(String, Action)> {
    let game = m.named_game();
    let ns = game.and_then(|g| g.namespace.as_deref());
    let base = game
        .and_then(|g| g.items.iter().find(|i| i.key() == "keys"))
        .and_then(|i| i.word(1))
        .map(String::from);
    let mut out = Vec::new();
    for ctx in ["ALWAYS".to_string()].into_iter().chain(base) {
        let Some(full) = names::candidates(ns, &ctx)
            .into_iter()
            .find(|c| m.contexts.contains_key(c))
        else {
            continue;
        };
        for b in &m.contexts[&full] {
            if b.kind != "keys" {
                continue;
            }
            for item in &b.items {
                let (_, row) = item.marker();
                let (action, n) = match row.key() {
                    "open" => (Action::Open(row.word(1).unwrap_or("").to_string()), 2),
                    w if names::is_vocabulary(w) => (Action::Engine(w.to_string()), 1),
                    w => match m.resolve_kind(row.namespace.as_deref(), w, &[Kind::Command]) {
                        Ok(d) => (Action::Command(d.full.clone()), 1),
                        Err(_) => continue,
                    },
                };
                for k in row.tokens.iter().skip(n) {
                    out.retain(|(key, _): &(String, Action)| key != &k.text);
                    out.push((k.text.clone(), action.clone()));
                }
            }
        }
    }
    out
}

impl<'m> Session<'m> {
    /// A new game with `seed`, run to the player's first turn.
    pub fn start(m: &'m Model, seed: u64) -> RtResult<Session<'m>> {
        let mut w = World::new_game(m, seed)?;
        w.begin()?;
        let lang = m
            .settings()
            .get("language")
            .map(|i| i.rest(1).to_string())
            .unwrap_or_default();
        let mut s = Session {
            r: Runner::new(w),
            lang,
            keys: keymap(m),
            log_seen: 0,
        };
        s.r.run(0)?;
        Ok(s)
    }

    pub fn action(&self, key: &str) -> Option<&Action> {
        self.keys.iter().find(|(k, _)| k == key).map(|(_, a)| a)
    }

    /// The frame: turn, time, the player's HP, the active map and the new
    /// log lines.
    pub fn frame(&mut self) -> String {
        frame(&mut self.r.w, &self.lang, &mut self.log_seen)
    }
}

/// A frame of `w` (see [`Session::frame`]); `log_seen` counts the log
/// lines already shown.
pub fn frame(w: &mut World, lang: &str, log_seen: &mut usize) -> String {
    let mut out = format!("turn {}  time {}", w.s.game.turn(), w.s.game.time);
    if let Some(p) = w.s.player {
        let hp =
            w.m.resolve(None, "HP")
                .ok()
                .map(|d| d.full.clone())
                .and_then(|hp| w.s.get(p).get(&hp).cloned());
        if let Some(hp) = hp {
            let now = hp.get("now").and_then(Value::as_number).unwrap_or(0);
            let max = hp.get("max").and_then(Value::as_number).unwrap_or(0);
            out.push_str(&format!("  HP {now}/{max}"));
        }
        if !w.s.get(p).alive {
            out.push_str("  (dead)");
        }
    }
    out.push('\n');
    if let Some(map) = w.s.active_map() {
        out.push_str(&format!("[{}]\n", w.s.maps[map].name));
        for row in w.render(map) {
            out.push_str(&row);
            out.push('\n');
        }
    }
    let lines: Vec<String> = w.log[*log_seen..]
        .iter()
        .map(|l| w.text(lang, &l.key, &l.args))
        .collect();
    *log_seen = w.log.len();
    for l in lines {
        out.push_str(&format!("  · {l}\n"));
    }
    out
}

impl<'m> Session<'m> {
    /// Presses a key on the player's turn and runs to the next one. Returns
    /// a note for the transcript.
    pub fn press(&mut self, key: &str) -> RtResult<String> {
        // a scene takes the engine's SCENE keys (M 8): `enter` or `space`
        // confirms, `tab` skips, `r` starts it again and `backspace` rewinds
        if self.r.in_scene() {
            use super::world::Pause;
            // a choice takes its visible items by number, from 1
            if let Some(Pause::Choice { items, .. }) = self.r.scene_pause().cloned()
                && let Some(i) = key.parse::<usize>().ok().filter(|i| *i >= 1)
                && let Some(item) = items.get(i - 1)
            {
                let ok = self.r.scene_choose(&item.name)?;
                self.r.run(0)?;
                return Ok(if ok {
                    format!("scene.choose {}", item.name)
                } else {
                    format!("({} cannot be chosen)", item.name)
                });
            }
            // `enter` cancels a field and closes what `open` waits for
            if matches!(self.r.scene_pause(), Some(Pause::Ask { .. })) && key == "enter" {
                self.r.scene_answer(Value::None)?;
                self.r.run(0)?;
                return Ok("scene.answer none".into());
            }
            if matches!(self.r.scene_pause(), Some(Pause::Open { .. })) && key == "enter" {
                self.r.scene_closed()?;
                self.r.run(0)?;
                return Ok("scene.close".into());
            }
            let note = match key {
                "enter" | "space" => {
                    self.r.scene_confirm()?;
                    "scene.confirm"
                }
                "tab" => {
                    self.r.scene_skip()?;
                    "skip"
                }
                "r" => {
                    self.r.scene_restart()?;
                    "restart"
                }
                "backspace" => {
                    self.r.scene_rewind()?;
                    "rewind"
                }
                _ => return Ok(format!("(a scene waits: `{key}` does nothing)")),
            };
            self.r.run(0)?;
            return Ok(note.into());
        }
        let player_alive = self.r.w.s.player.is_some_and(|p| self.r.w.s.get(p).alive);
        let note = match self.action(key).cloned() {
            None => return Ok(format!("(key `{key}` is not bound)")),
            Some(Action::Open(menu)) => {
                return Ok(format!("(menu {menu}: menus are not supported yet)"));
            }
            Some(Action::Engine(a)) if a == "pass" => {
                // without a player's turn, `pass` runs the next round (M 6.1.2)
                if !player_alive {
                    self.r.run(1)?;
                }
                "pass".to_string()
            }
            Some(Action::Engine(a)) => return Ok(format!("({a}: not supported yet)")),
            Some(Action::Command(c)) => {
                if !player_alive {
                    // without a player the world goes on, a round a key (M 6.1.2)
                    self.r.run(1)?;
                    return Ok("(no player: a round goes by)".into());
                }
                let ok = self.r.player(&CommandValue {
                    name: c.clone(),
                    args: vec![],
                })?;
                if ok { c } else { format!("{c} failed") }
            }
        };
        match self.r.run(0)? {
            Status::Input | Status::Scene => {}
            // a dead player: the world goes on at the `round` pace for a round
            Status::NoPlayer { .. } => {
                self.r.run(1)?;
            }
        }
        Ok(note)
    }
}

/// Plays keys from a new game and gives the whole transcript.
pub fn transcript(m: &Model, seed: u64, keys: &[&str]) -> RtResult<String> {
    Ok(transcript_and_log(m, seed, keys)?.0)
}

/// `transcript` and the session's input log (M 4.4).
pub fn transcript_and_log(m: &Model, seed: u64, keys: &[&str]) -> RtResult<(String, InputLog)> {
    transcript_with(m, seed, keys, &[])
}

/// `transcript_and_log` with breakpoints (M 5.7): each stop from the first
/// key on is a line before the key's frame, and the game goes on.
pub fn transcript_with(
    m: &Model,
    seed: u64,
    keys: &[&str],
    breaks: &[String],
) -> RtResult<(String, InputLog)> {
    let mut s = Session::start(m, seed)?;
    let mut stops = None;
    if !breaks.is_empty() {
        let (hook, lines) = super::debug::printer();
        let mut d = super::debug::Debugger::new(hook);
        for b in breaks {
            d.add(m, b)?;
        }
        s.r.w.debug = Some(d.shared());
        stops = Some(lines);
    }
    let mut out = format!("seed {seed}\n");
    out.push_str(&s.frame());
    for k in keys {
        let note = s.press(k)?;
        out.push_str(&format!("\n> {k}  {note}\n"));
        if let Some(lines) = &stops
            && let Ok(mut l) = lines.lock()
        {
            for line in l.drain(..) {
                out.push_str(&line);
                out.push('\n');
            }
        }
        out.push_str(&s.frame());
    }
    Ok((out, s.r.input_log()))
}

/// Replays an input log and gives a transcript: the first frame, then each
/// row and the frame after it (M 4.4).
pub fn replay_transcript(m: &Model, log: &InputLog) -> RtResult<String> {
    let lang = m
        .settings()
        .get("language")
        .map(|i| i.rest(1).to_string())
        .unwrap_or_default();
    let mut rp = Replay::start(m, log)?;
    let mut seen = 0;
    let mut out = format!("seed {}\n", log.seed);
    out.push_str(&frame(&mut rp.r.w, &lang, &mut seen));
    loop {
        let before = rp.r.w.s.game.time;
        let Some(row) = rp.step()? else { break };
        // the end shows only when the world went on to it
        if row.act == super::input::Act::End && rp.r.w.s.game.time == before {
            continue;
        }
        out.push_str(&format!("\n> {row}\n"));
        out.push_str(&frame(&mut rp.r.w, &lang, &mut seen));
    }
    Ok(out)
}
