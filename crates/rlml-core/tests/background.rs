//! Background scenes (RS 9.9, RLML-kohtaukset.md step 5): a patrol on the
//! game's clock in rounds, its tween's steps between the turns, `scope:
//! map`, `bind`, `stop(handle)`, and a save and load in the middle of a
//! walk that goes on as without (RS 11.1).

use std::path::Path;

use rlml_core::load::MemFiles;
use rlml_core::model::Model;
use rlml_core::rt::input::{InputLog, Replay};
use rlml_core::rt::run::{Runner, Status};
use rlml_core::rt::save::Save;
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

const GAME: &str = r#"game T
- start HALL
- WATCH

include lib/std.rlml
include text/*

settings
- style ascii
- language fi

tiles
- WALL blocks move sight sound
- default blocks none lit

component WATCH
- patrol handle

entity GUARDSMAN
- blocks move

map HALL

```
#########
#S......#
#a.....b#
#G......#
#########
```

legend:
- # WALL
- . FLOOR
- S FLOOR as START
- a FLOOR as POST_A
- b FLOOR as POST_B
- G FLOOR with GUARDSMAN as POST

map CELLAR

```
#####
#S..#
#####
```

legend:
- # WALL
- . FLOOR
- S FLOOR as START

```roguescript
after begin() as START_PATROL:
    g = entities(in: label(POST)).first
    game.WATCH.patrol = start(PATROL(g), scope: map, bind: g)

scene PATROL(guard: entity) timeline background:
    track ROUTE loop 12:
        00 02 move(guard, to: label(POST_A))
        05 02 move(guard, to: label(POST_B))
        10    log(GUARD.ALL_CLEAR, speaker: guard)

command DESCEND() cost 0:
    travel(actor, CELLAR)
    return true

command HALT() cost 0:
    stop(game.WATCH.patrol)
    return true

command STRIKE_GUARD() cost 0:
    destroy(entities(in: label(POST)).first)
    return true
```

messages
- GUARD.ALL_CLEAR
"#;

fn model() -> Model {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut f = MemFiles::default();
    read_tree(&repo, &repo.join("lib"), &mut f.engine);
    f.game.insert("game.rlml".into(), GAME.into());
    f.game.insert(
        "text/fi.rlml".into(),
        "language fi\nGUARD.ALL_CLEAR Kaikki hyvin.\n".into(),
    );
    let (m, r) = rlml_core::check_game(&f, "game.rlml");
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

fn press(r: &mut Runner, name: &str) {
    let n = r.awaiting().unwrap();
    r.input(
        n,
        &CommandValue {
            name: name.into(),
            args: vec![],
        },
    )
    .unwrap();
    r.run(0).unwrap();
}

fn wait(r: &mut Runner, rounds: usize) {
    for _ in 0..rounds {
        press(r, "STD.WAIT");
    }
}

fn guard(r: &Runner) -> Option<(i32, i32)> {
    let e = (0..r.w.s.entities.len())
        .find(|&e| r.w.s.get(e).alive && r.w.s.get(e).kind.ends_with("GUARDSMAN"))?;
    r.w.s.at(e).map(|p| (p.x, p.y))
}

fn cleared(r: &Runner) -> usize {
    r.w.log
        .iter()
        .filter(|l| l.key == "GUARD.ALL_CLEAR")
        .count()
}

#[test]
fn a_patrol_walks_on_the_rounds() {
    let m = model();
    let mut r = start(&m);
    let watch =
        r.w.s
            .game
            .components
            .iter()
            .find(|c| c.name == "WATCH")
            .unwrap()
            .clone();
    assert_eq!(
        r.w.s.backgrounds.len(),
        1,
        "begin started it: {watch:?} {:?}",
        r.w.warnings
    );
    wait(&mut r, 1);
    assert_eq!(
        guard(&r),
        Some((1, 3)),
        "time 120: its clock starts at the first boundary"
    );
    wait(&mut r, 1);
    assert_eq!(
        guard(&r),
        Some((1, 2)),
        "time 240: one cell over two rounds, at its middle"
    );
    wait(&mut r, 5);
    assert_eq!(
        guard(&r).map(|g| g.0),
        Some(4),
        "time 840: six cells over two rounds from 720, three done"
    );
    assert_eq!(r.w.s.game.time, 840);
    wait(&mut r, 1);
    assert_eq!(guard(&r), Some((7, 2)), "at POST_B");
    assert_eq!(cleared(&r), 0);
    wait(&mut r, 3);
    assert_eq!(cleared(&r), 1, "round 10 of the loop: time 1320");
    wait(&mut r, 2);
    assert_eq!(
        guard(&r),
        Some((7, 2)),
        "1560: the loop's second round starts; its first step is at 1580"
    );
    wait(&mut r, 1);
    assert_eq!(guard(&r).map(|g| g.0), Some(4), "1680: back towards POST_A");
    let text = r.input_log().write().unwrap();
    let got =
        Replay::run(&m, &InputLog::read(&text).unwrap()).unwrap_or_else(|e| panic!("{e}\n{text}"));
    assert!(got.w.s == r.w.s, "the replay walks the same\n{text}");
}

#[test]
fn a_save_in_the_middle_of_a_walk_goes_on_as_without() {
    let m = model();
    let mut r = start(&m);
    wait(&mut r, 7);
    assert_eq!(guard(&r).map(|g| g.0), Some(4), "half way to POST_B");
    let text = r.save().write();
    assert!(text.contains("(bg "), "the scene is in the save:\n{text}");
    let mut l = Runner::load(&m, &Save::read(&text).unwrap()).unwrap();
    assert_eq!(l.run(0).unwrap(), Status::Input);
    wait(&mut r, 8);
    wait(&mut l, 8);
    assert_eq!(guard(&l), guard(&r));
    assert!(
        l.w.s == r.w.s,
        "the loaded game walks on the same: the rest of the route, the loop's phase"
    );
    assert_eq!(cleared(&l), 1);
}

#[test]
fn stop_bind_and_scope_end_it() {
    let m = model();
    // stop(handle)
    let mut r = start(&m);
    wait(&mut r, 1);
    press(&mut r, "HALT");
    assert!(r.w.s.backgrounds.is_empty(), "stop(handle) (RS 9.9)");
    wait(&mut r, 3);
    assert_eq!(guard(&r), Some((1, 3)), "the patrol stood still");
    // bind: the guard destroyed
    let mut r = start(&m);
    wait(&mut r, 1);
    press(&mut r, "STRIKE_GUARD");
    assert!(
        r.w.s.backgrounds.is_empty(),
        "`bind` ends it with the guard (RS 9.9)"
    );
    // scope: map: the player leaves
    let mut r = start(&m);
    wait(&mut r, 1);
    press(&mut r, "DESCEND");
    assert!(
        r.w.s.backgrounds.is_empty(),
        "`scope: map` ends it when the player leaves (RS 9.9)"
    );
}

#[test]
fn a_realtime_scene_presents_on_the_front_ends_clock() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut f = MemFiles::default();
    read_tree(&repo, &repo.join("lib"), &mut f.engine);
    let extra = r#"
```roguescript
scene CANDLES() timeline background realtime:
    track FLICKER loop 600ms:
        000ms 300ms fade(light(label(POST_A)), to: 70%)
        300ms 300ms fade(light(label(POST_A)), to: 100%)

command LIGHT() cost 0:
    start(CANDLES())
    return true
```
"#;
    f.game
        .insert("game.rlml".into(), format!("{GAME}\n{extra}"));
    f.game.insert(
        "text/fi.rlml".into(),
        "language fi\nGUARD.ALL_CLEAR Kaikki hyvin.\n".into(),
    );
    let (m, rep) = rlml_core::check_game(&f, "game.rlml");
    assert!(
        !rep.has_errors(),
        "{:#?}",
        rep.errors().map(|d| d.to_string()).collect::<Vec<_>>()
    );
    let mut r = start(&m);
    press(&mut r, "LIGHT");
    let before = r.w.s.clone();
    r.w.effects.clear();
    r.realtime_tick(0).unwrap();
    r.realtime_tick(650).unwrap();
    let fades: Vec<i64> =
        r.w.effects
            .iter()
            .filter(|e| e.call == "fade")
            .map(|e| e.span)
            .collect();
    assert_eq!(
        fades,
        [300, 300, 300],
        "0ms, 300ms and the loop again at 600ms"
    );
    assert!(r.w.s == before, "its clock is not state (RS 9.9)");
    // the load's rules for background clocks
    let bad = r#"
```roguescript
scene NOISY() timeline background realtime:
    000ms log(GUARD.ALL_CLEAR)
    100ms destroy(player)
    5     fade(light(label(POST_A)), to: 70%)

scene SLOW() timeline background:
    500ms log(GUARD.ALL_CLEAR)
```
"#;
    f.game.insert("game.rlml".into(), format!("{GAME}\n{bad}"));
    let (_, rep) = rlml_core::check_game(&f, "game.rlml");
    let errors: Vec<String> = rep.errors().map(|d| d.to_string()).collect();
    for want in [
        "`log(GUARD.ALL_CLEAR)` changes the state or the log",
        "`destroy(player)` changes the state",
        "`5`: a realtime background scene counts presentation time",
        "`500ms`: a background scene counts rounds",
    ] {
        assert!(
            errors.iter().any(|e| e.contains(want)),
            "{want}: {errors:#?}"
        );
    }
}
