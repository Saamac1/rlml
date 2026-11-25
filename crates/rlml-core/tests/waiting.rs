//! A waiting command's place as a structural path (M 6.1.6, RS 11.1):
//! the wait point's number, the loops' states and the body's hash.

use std::path::Path;

use rlml_core::load::MemFiles;
use rlml_core::model::Model;
use rlml_core::rt::path::{LoopState, PathLoop};
use rlml_core::rt::run::{Runner, Status};
use rlml_core::rt::value::{CommandValue, Value};
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
            out.insert(p.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/"), std::fs::read_to_string(&p).unwrap());
        }
    }
}

const GAME: &str = r#"game T
- start ROOM
- COUNTER

include lib/std.rlml

settings
- style ascii
- language fi

component COUNTER
- dug number
- searched number
- spots number

map ROOM

```
#@..#
```

legend:
- # WALL
- @ FLOOR as START
- . FLOOR

```roguescript
command DIG() cost 60:
    repeat 5:
        wait 120
        game.COUNTER.dug += 1

command SEARCH() cost 0:
    for spot in [1, 2, 3]:
        game.COUNTER.spots += 1
        repeat 3:
            if spot == 3:
                wait 40
            game.COUNTER.searched += 1
```
"#;

fn files(game: &str) -> MemFiles {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut f = MemFiles::default();
    read_tree(&repo, &repo.join("lib"), &mut f.engine);
    f.game.insert("game.rlml".into(), game.into());
    f
}

fn model(game: &str) -> Model {
    let (m, r) = rlml_core::check_game(&files(game), "game.rlml");
    assert!(!r.has_errors(), "{:#?}", r.errors().map(|d| d.to_string()).collect::<Vec<_>>());
    m
}

fn start(m: &Model) -> Runner<'_> {
    let mut w = World::new_game(m, 7).unwrap();
    w.begin().unwrap();
    let mut r = Runner::new(w);
    assert_eq!(r.run(0).unwrap(), Status::Input);
    r
}

fn counter(r: &Runner, f: &str) -> i64 {
    r.w.s.game.components.iter().find(|c| c.name == "COUNTER").unwrap().get(f).unwrap().as_number().unwrap()
}

/// The player's command on its turn.
fn command(r: &mut Runner, name: &str) {
    assert!(r.player(&CommandValue { name: name.into(), args: vec![] }).unwrap());
}

#[test]
fn dig_waits_at_point_0_in_round_2() {
    let m = model(GAME);
    let mut r = start(&m);
    command(&mut r, "DIG");
    let p = r.w.s.player.unwrap();
    // the first wait is in round 0; two more wakes take it to round 2
    let path = |r: &Runner| {
        let s = &r.waiting[&p];
        (s.point, s.loops.clone())
    };
    assert_eq!(path(&r), (0, vec![PathLoop { loop_no: 0, state: LoopState::Repeat { round: 0, count: 5 } }]));
    assert_eq!(r.w.s.wake_of(p).unwrap().at, 120);
    // the scheduler runs to the player's next input: all five rounds
    assert_eq!(r.run(0).unwrap(), Status::Input);
    assert_eq!(counter(&r, "dug"), 5);
    assert_eq!(r.w.s.game.time, 5 * 120 + 60, "DIG takes 660 microsteps (M 6.1.6)");
}

#[test]
fn the_path_holds_the_round_it_waits_in() {
    // RS 11.1: DIG waiting in its third round is wait point 0, repeat round 2
    let m = model(GAME);
    let mut w = World::new_game(&m, 7).unwrap();
    w.begin().unwrap();
    let p = w.s.player.unwrap();
    let mut out = w.start_command(p, &CommandValue { name: "DIG".into(), args: vec![] }).unwrap();
    for round in 0..3 {
        let rlml_core::rt::eval::Outcome::Waiting { state, .. } = out else { panic!("DIG waits") };
        assert_eq!(state.point, 0);
        assert_eq!(state.loops, vec![PathLoop { loop_no: 0, state: LoopState::Repeat { round, count: 5 } }]);
        out = w.continue_command(p, *state, true).unwrap();
    }
}

#[test]
fn nested_loops_keep_their_set_index_and_round() {
    let m = model(GAME);
    let mut w = World::new_game(&m, 7).unwrap();
    w.begin().unwrap();
    let p = w.s.player.unwrap();
    let out = w.start_command(p, &CommandValue { name: "SEARCH".into(), args: vec![] }).unwrap();
    let rlml_core::rt::eval::Outcome::Waiting { state, .. } = out else { panic!("SEARCH waits at the third spot") };
    let set = vec![Value::Number(1), Value::Number(2), Value::Number(3)];
    assert_eq!(state.point, 0);
    assert_eq!(
        state.loops,
        vec![PathLoop { loop_no: 0, state: LoopState::For { items: set.clone(), index: 2 } }, PathLoop { loop_no: 1, state: LoopState::Repeat { round: 0, count: 3 } }]
    );
    assert_eq!(state.env.locals.get("spot"), Some(&Value::Number(3)), "the loop variable is a local name");
    let out = w.continue_command(p, *state, true).unwrap();
    let rlml_core::rt::eval::Outcome::Waiting { state, .. } = out else { panic!("the second round waits too") };
    assert_eq!(state.loops[1].state, LoopState::Repeat { round: 1, count: 3 });
    let out = w.continue_command(p, *state, true).unwrap();
    let rlml_core::rt::eval::Outcome::Waiting { state, .. } = out else { panic!("and the third") };
    assert!(matches!(w.continue_command(p, *state, true).unwrap(), rlml_core::rt::eval::Outcome::Done { .. }));
    let searched = w.s.game.components.iter().find(|c| c.name == "COUNTER").unwrap().get("searched").unwrap().as_number();
    assert_eq!(searched, Some(9), "three spots, three rounds each");
}

#[test]
fn comments_and_spacing_keep_the_hash() {
    let a = model(GAME);
    let b = model(&GAME.replace("    repeat 5:\n", "    // five rounds\n\n    repeat 5:\n"));
    let mut r = start(&a);
    command(&mut r, "DIG");
    let p = r.w.s.player.unwrap();
    r.w.m = &b;
    assert!(r.w.waiting_fits(&r.waiting[&p]), "the same body, written otherwise");
    assert_eq!(r.run(0).unwrap(), Status::Input);
    assert_eq!(counter(&r, "dug"), 5);
    assert!(r.w.warnings.is_empty(), "{:?}", r.w.warnings);
}

#[test]
fn a_changed_body_drops_the_wait() {
    let a = model(GAME);
    let b = model(&GAME.replace("        game.COUNTER.dug += 1\n", "        game.COUNTER.dug += 2\n"));
    let mut r = start(&a);
    command(&mut r, "DIG");
    let p = r.w.s.player.unwrap();
    assert!(r.waiting.contains_key(&p));
    // as after loading a save made before the game changed (M 6.1.6)
    r.w.m = &b;
    assert_eq!(r.run(0).unwrap(), Status::Input, "the wake is the player's ordinary turn");
    assert!(!r.waiting.contains_key(&p));
    assert_eq!(counter(&r, "dug"), 0, "nothing more is dug");
    assert_eq!(r.w.s.game.time, 120, "no cost is paid: the turn comes at the wake");
    assert!(r.w.warnings.iter().any(|w| w.contains("DIG")), "{:?}", r.w.warnings);
}

#[test]
fn a_saved_wait_goes_on_after_loading() {
    use rlml_core::rt::save::Save;
    let m = model(GAME);
    let mut r = start(&m);
    command(&mut r, "SEARCH");
    let p = r.w.s.player.unwrap();
    assert!(r.waiting.contains_key(&p), "SEARCH waits at the third spot");
    let text = r.save().write();
    let save = Save::read(&text).unwrap();
    assert_eq!(save.now.s, r.w.s);
    let mut loaded = Runner::load(&m, &save).unwrap();
    assert_eq!(loaded.waiting, r.waiting, "the path, the set, the round and the locals (M 5.2)");
    assert_eq!(r.run(0).unwrap(), Status::Input);
    assert_eq!(loaded.run(0).unwrap(), Status::Input);
    assert_eq!(counter(&loaded, "searched"), 9);
    assert!(loaded.w.s == r.w.s, "the loaded game went as the one that was not saved");
}
