//! Checkpoints in memory (M 5.3, M 5.5, M 6.1.7): `checkpoint()` at an
//! action's end, `rollback()` back to it, `delete_save(checkpoint)`, and
//! their order in one action.

use std::path::Path;

use rlml_core::load::MemFiles;
use rlml_core::model::Model;
use rlml_core::rt::input::{InputLog, Replay};
use rlml_core::rt::run::{Runner, Status};
use rlml_core::rt::value::CommandValue;
use rlml_core::rt::world::World;

fn read_tree(root: &Path, dir: &Path, out: &mut std::collections::BTreeMap<String, String>) {
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if e.file_name().to_string_lossy().starts_with('.') {
            continue;
        }
        if p.is_dir() {
            read_tree(root, &p, out);
        } else {
            out.insert(
                p.strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/"),
                std::fs::read_to_string(&p).unwrap(),
            );
        }
    }
}

fn repo() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// An example game with `LOGIC` as one more file of it.
fn example(name: &str) -> MemFiles {
    let mut f = MemFiles::default();
    read_tree(&repo(), &repo().join("lib"), &mut f.engine);
    let game = repo().join("esimerkit").join(name);
    read_tree(&game, &game, &mut f.game);
    f.game
        .get_mut("game.rlml")
        .unwrap()
        .push_str("\ninclude logic.rlml\n");
    f.game.insert("logic.rlml".into(), LOGIC.into());
    f
}

/// The altar is a command here; death rolls back (RS 10.2).
const LOGIC: &str = r#"
```roguescript
command PRAY() cost 0:
    checkpoint()
    return true

command BOTH() cost 0:
    checkpoint()
    rollback()
    return true

command BOTH.BACK() cost 0:
    rollback()
    checkpoint()
    return true

command FORGET() cost 0:
    delete_save(checkpoint)
    return true

command BACK() cost 0:
    rollback()
    return true

on DIED(victim, killer) when victim == player:
    rollback()
```
"#;

fn model(f: &MemFiles) -> Model {
    let (m, r) = rlml_core::check_game(f, "game.rlml");
    assert!(
        !r.has_errors(),
        "{:#?}",
        r.errors().map(|d| d.to_string()).collect::<Vec<_>>()
    );
    m
}

fn start(m: &Model) -> Runner<'_> {
    let mut w = World::new_game(m, 7).unwrap();
    w.begin().unwrap();
    let mut r = Runner::new(w);
    assert_eq!(r.run(0).unwrap(), Status::Input);
    r
}

fn cmd(name: &str) -> CommandValue {
    CommandValue {
        name: name.into(),
        args: vec![],
    }
}

/// The awaited seat's command, then the world to the next turn.
fn input(r: &mut Runner, name: &str) {
    let n = r.awaiting().unwrap();
    r.input(n, &cmd(name)).unwrap();
    r.run(0).unwrap();
}

fn hp(r: &Runner) -> i64 {
    let p = r.w.s.player.unwrap();
    let hp = r.w.s.get(p).get("STD.HP").unwrap();
    hp.get("now").and_then(|v| v.as_number()).unwrap()
}

#[test]
fn death_rolls_back_to_the_altar() {
    let m = model(&example("mini"));
    let mut r = start(&m);
    // the rat bites the waiting player to death (golden rat_kills_player);
    // a prayer after two turns sets the checkpoint
    input(&mut r, "STD.WAIT");
    input(&mut r, "STD.WAIT");
    input(&mut r, "PRAY");
    let altar = r.snapshot();
    let full = hp(&r);
    let mut bitten = false;
    for _ in 0..40 {
        input(&mut r, "STD.WAIT");
        bitten |= hp(&r) < full;
        if r.w.s.game.time == altar.s.game.time {
            break;
        }
    }
    assert!(bitten, "the rat bit");
    assert_eq!(
        r.w.s.game.time, altar.s.game.time,
        "death went back to the prayer's moment"
    );
    assert!(r.w.s == altar.s, "the state is the checkpoint's (M 5.3)");
    assert_eq!(r.w.rng, altar.rng, "and so is the stream");
    assert_eq!(hp(&r), full);
    assert!(
        r.w.log.iter().any(|l| l.key.ends_with("YOU_DIED")),
        "the log is presentation and keeps the death"
    );
}

#[test]
fn checkpoint_0_is_the_start() {
    let m = model(&example("mini"));
    let mut r = start(&m);
    let zero = r.checkpoint.clone().unwrap();
    input(&mut r, "STD.WAIT");
    input(&mut r, "STD.WAIT");
    input(&mut r, "BACK");
    assert_eq!(r.w.s.game.time, 0);
    assert!(
        r.w.s == zero.s,
        "checkpoint 0 is the state after `begin` (M 5.1)"
    );
}

#[test]
fn the_last_call_of_an_action_decides() {
    let m = model(&example("mini"));
    let mut r = start(&m);
    input(&mut r, "STD.WAIT");
    input(&mut r, "BOTH");
    assert_eq!(
        r.w.s.game.time, 0,
        "checkpoint then rollback: back to checkpoint 0"
    );
    input(&mut r, "STD.WAIT");
    input(&mut r, "BOTH.BACK");
    let t = r.w.s.game.time;
    assert_eq!(t, 120, "rollback then checkpoint: a new checkpoint here");
    input(&mut r, "STD.WAIT");
    input(&mut r, "BACK");
    assert_eq!(r.w.s.game.time, t);
}

#[test]
fn without_a_checkpoint_rollback_warns() {
    let m = model(&example("mini"));
    let mut r = start(&m);
    input(&mut r, "STD.WAIT");
    input(&mut r, "FORGET");
    assert!(r.checkpoint.is_none());
    let t = r.w.s.game.time;
    input(&mut r, "BACK");
    assert_eq!(r.w.s.game.time, t, "nothing happens");
    assert!(
        r.w.warnings.iter().any(|w| w.contains("rollback")),
        "{:?}",
        r.w.warnings
    );
}

#[test]
fn rollback_brings_back_the_seats() {
    let m = model(&example("ryhma"));
    let mut r = start(&m);
    assert_eq!(r.awaiting(), Some(1));
    let hero = r.w.s.active(1).unwrap();
    input(&mut r, "PRAY");
    assert_eq!(r.awaiting(), Some(1));
    input(&mut r, "STD.SWITCH");
    let robin = r.w.s.active(1).unwrap();
    assert_ne!(robin, hero, "seat 1 controls Robin");
    // Robin's turn is later, so seat 2 may go first; either seat's
    // rollback loads the same checkpoint
    input(&mut r, "BACK");
    assert_eq!(
        r.w.s.active(1),
        Some(hero),
        "the checkpoint's controlled member (M 5.3)"
    );
}

#[test]
fn a_rollback_replays() {
    let m = model(&example("mini"));
    let mut r = start(&m);
    input(&mut r, "PRAY");
    for _ in 0..12 {
        input(&mut r, "STD.WAIT");
    }
    let text = r.input_log().write().unwrap();
    let log = InputLog::read(&text).unwrap();
    let got = Replay::run(&m, &log).unwrap_or_else(|e| panic!("{e}\n{text}"));
    assert!(
        got.w.s == r.w.s,
        "moments go back in the log, and the replay follows\n{text}"
    );
    assert_eq!(got.w.rng, r.w.rng);
}

#[test]
fn the_checkpoint_goes_with_the_save() {
    use rlml_core::rt::save::Save;
    let m = model(&example("mini"));
    let mut r = start(&m);
    input(&mut r, "STD.WAIT");
    input(&mut r, "PRAY");
    let altar = r.snapshot();
    input(&mut r, "STD.WAIT");
    // saved and loaded after the prayer: the rollback still finds the altar
    let text = r.save().write();
    let mut r = Runner::load(&m, &Save::read(&text).unwrap()).unwrap();
    assert_eq!(r.run(0).unwrap(), Status::Input);
    for _ in 0..40 {
        input(&mut r, "STD.WAIT");
        if r.w.s.game.time == altar.s.game.time {
            break;
        }
    }
    assert!(
        r.w.s == altar.s,
        "death in the loaded game went back to the prayer (M 5.3)"
    );
}
