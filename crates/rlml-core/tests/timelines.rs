//! Foreground timelines (RS 9.1–9.5, RLML-kohtaukset.md step 4): rows on
//! a presentation clock, marks, `when … else`, tracks and loops, tweens a
//! cell at a time, `say` stopping the clock, and a skip that ends in the
//! same state and log as watching to the end (RS 9.8).

use std::path::Path;

use rlml_core::load::MemFiles;
use rlml_core::model::Model;
use rlml_core::rt::input::{InputLog, Replay};
use rlml_core::rt::run::{Runner, Status};
use rlml_core::rt::value::{CommandValue, Value};
use rlml_core::rt::world::{Pause, World};

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
- QUEST

include lib/std.rlml
include text/*

settings
- style ascii
- language fi

tiles
- WALL blocks move sight sound
- GATE.shut blocks move sight
- GATE.open
- default blocks none lit

component QUEST
- state number

entity GUARDSMAN
- blocks move

map HALL

```
##########
#S.......#
#.ggg....#
#........#
#G...d..t#
##########
```

legend:
- # WALL
- . FLOOR
- S FLOOR as START
- g GATE.shut as GATE_A
- G FLOOR with GUARDSMAN as POST
- d FLOOR as DOOR_SPOT
- t FLOOR as TORCH

```roguescript
scene GATE_OPENS(lever: entity) timeline:
    0000ms        log(LINE.LEVER)
    0500ms 1000ms shake(camera, strength: 2)
    1000ms 2000ms tile(label(GATE_A), GATE.open)
    2000ms        mark OPEN
    OPEN          when game.QUEST.state == 1:
        say(GATE.FOR_THE_KING)
    else:
        log(LINE.NOTE)
    OPEN+1s       say(GATE.OPENED)
    +200ms 0500ms pan(camera, to: player)

scene AUDIENCE() timeline:
    track TORCHES loop 2s:
        0000ms 1000ms fade(light(label(TORCH)), to: 80%)
        1000ms 1000ms fade(light(label(TORCH)), to: 100%)
    track ACTION:
        0000ms        say(KING.HELLO)
        1000ms 2000ms move(entities(in: label(POST)).first, to: label(DOOR_SPOT))
        3000ms        log(LINE.NOTE)

command OPEN() cost 0:
    start(GATE_OPENS(actor))
    return true

command KINGLY() cost 0:
    game.QUEST.state = 1
    start(GATE_OPENS(actor))
    return true

command HOLD_AUDIENCE() cost 0:
    start(AUDIENCE())
    return true
```

messages
- LINE.LEVER
- LINE.NOTE
"#;

fn model() -> Model {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut f = MemFiles::default();
    read_tree(&repo, &repo.join("lib"), &mut f.engine);
    f.game.insert("game.rlml".into(), GAME.into());
    f.game.insert(
        "text/fi.rlml".into(),
        "language fi\nLINE.LEVER Vipu narahtaa.\nLINE.NOTE Muistiin.\nGATE.FOR_THE_KING Kuninkaalle.\nGATE.OPENED Portti on auki.\nKING.HELLO Tervetuloa.\n".into(),
    );
    let (m, r) = rlml_core::check_game(&f, "game.rlml");
    assert!(
        !r.has_errors(),
        "{:#?}",
        r.errors().map(|d| d.to_string()).collect::<Vec<_>>()
    );
    m
}

fn start(m: &Model, clock: bool) -> Runner<'_> {
    let mut w = World::new_game(m, 7).unwrap();
    w.begin().unwrap();
    let mut r = Runner::new(w);
    r.scene_clock = clock;
    assert_eq!(r.run(0).unwrap(), Status::Input);
    r
}

fn command(r: &mut Runner, name: &str) {
    let n = r.awaiting().unwrap();
    r.input(
        n,
        &CommandValue {
            name: name.into(),
            args: vec![],
        },
    )
    .unwrap();
}

fn open_gates(r: &Runner) -> usize {
    let map = &r.w.s.maps[0];
    (2..5)
        .filter(|&x| map.tile(0, x, 2).is_some_and(|t| t.ends_with("GATE.open")))
        .count()
}

fn said(r: &Runner) -> Option<String> {
    match r.scene_pause() {
        Some(Pause::Say { key, .. }) => Some(key.clone()),
        Some(_) => Some("other".into()),
        None => None,
    }
}

fn effects(r: &mut Runner) -> Vec<(String, i64)> {
    r.w.effects.drain(..).map(|e| (e.call, e.span)).collect()
}

fn keys(r: &Runner) -> Vec<String> {
    r.w.log.iter().map(|l| l.key.clone()).collect()
}

fn guard_x(r: &Runner) -> i32 {
    let e = (0..r.w.s.entities.len())
        .find(|&e| r.w.s.get(e).kind.ends_with("GUARDSMAN"))
        .unwrap();
    r.w.s.at(e).unwrap().x
}

#[test]
fn the_clock_runs_rows_tweens_and_marks() {
    let m = model();
    let mut r = start(&m, true);
    command(&mut r, "OPEN");
    assert!(
        r.in_scene() && r.scene_clock_runs(),
        "the timeline runs on the front end's clock"
    );
    assert_eq!(
        keys(&r),
        ["STD.LINE.LEVER".to_string()].map(|k| k.replace("STD.", "")),
        "0000ms ran at once"
    );
    r.scene_tick(499).unwrap();
    assert!(effects(&mut r).is_empty());
    r.scene_tick(1).unwrap();
    assert_eq!(
        effects(&mut r),
        [("shake".to_string(), 1000)],
        "0500ms: the shake with its duration"
    );
    r.scene_tick(800).unwrap();
    assert_eq!(
        open_gates(&r),
        0,
        "1300ms: the tween's first cell turns at 1334ms"
    );
    r.scene_tick(34).unwrap();
    assert_eq!(open_gates(&r), 1, "a cell at a time (RS 9.1)");
    r.scene_tick(666).unwrap();
    assert_eq!(open_gates(&r), 2);
    assert_eq!(
        keys(&r).last().map(String::as_str),
        Some("LINE.NOTE"),
        "OPEN: the `else` side, the quest is not taken"
    );
    r.scene_tick(1000).unwrap();
    assert_eq!(open_gates(&r), 3);
    assert_eq!(said(&r).as_deref(), Some("GATE.OPENED"), "OPEN+1s");
    assert_eq!(r.w.s.game.time, 0, "the game's time stands");
    r.scene_tick(5000).unwrap();
    assert!(effects(&mut r).is_empty(), "`say` stops the clock (RS 9.5)");
    r.scene_confirm().unwrap();
    r.scene_tick(199).unwrap();
    assert!(effects(&mut r).is_empty());
    r.scene_tick(1).unwrap();
    assert_eq!(
        effects(&mut r),
        [("pan".to_string(), 500)],
        "+200ms from the row before"
    );
    assert!(r.in_scene(), "the pan's duration is the track's end");
    r.scene_tick(500).unwrap();
    assert!(!r.in_scene(), "every track is done");
    assert_eq!(r.run(0).unwrap(), Status::Input);
}

#[test]
fn without_a_clock_the_timeline_runs_to_its_pauses() {
    let m = model();
    let mut r = start(&m, false);
    command(&mut r, "KINGLY");
    assert_eq!(r.run(0).unwrap(), Status::Scene);
    assert_eq!(
        said(&r).as_deref(),
        Some("GATE.FOR_THE_KING"),
        "the `when` side, at OPEN"
    );
    assert_eq!(open_gates(&r), 2, "the tween went as far as the clock");
    r.scene_confirm().unwrap();
    assert_eq!(r.run(0).unwrap(), Status::Scene);
    assert_eq!(said(&r).as_deref(), Some("GATE.OPENED"));
    assert_eq!(open_gates(&r), 3);
    r.scene_confirm().unwrap();
    assert_eq!(r.run(0).unwrap(), Status::Input);
    assert_eq!(keys(&r), ["LINE.LEVER", "GATE.FOR_THE_KING", "GATE.OPENED"]);
}

#[test]
fn tracks_run_side_by_side_and_a_loop_ends_with_the_scene() {
    let m = model();
    let mut r = start(&m, true);
    command(&mut r, "HOLD_AUDIENCE");
    assert_eq!(said(&r).as_deref(), Some("KING.HELLO"));
    assert_eq!(
        effects(&mut r),
        [("fade".to_string(), 1000)],
        "the loop's first row at 0ms, before the line"
    );
    r.scene_confirm().unwrap();
    assert_eq!(guard_x(&r), 1);
    r.scene_tick(1250).unwrap();
    assert_eq!(
        guard_x(&r),
        2,
        "the guard walks a cell at a time: 4 cells over 2s"
    );
    assert_eq!(
        effects(&mut r),
        [("fade".to_string(), 1000)],
        "1000ms: the loop's second row"
    );
    r.scene_tick(1500).unwrap();
    assert_eq!(guard_x(&r), 5, "at the door by 2750ms");
    assert_eq!(
        effects(&mut r),
        [("fade".to_string(), 1000)],
        "2000ms: the loop again"
    );
    r.scene_tick(250).unwrap();
    assert!(
        !r.in_scene(),
        "ACTION is done at 3000ms, and the loop ends with the scene (RS 9.3)"
    );
    assert_eq!(keys(&r).last().map(String::as_str), Some("LINE.NOTE"));
}

#[test]
fn a_skip_ends_where_watching_does() {
    let m = model();
    // watched to the end
    let mut a = start(&m, true);
    command(&mut a, "HOLD_AUDIENCE");
    a.scene_confirm().unwrap();
    a.scene_tick(10_000).unwrap();
    assert_eq!(a.run(0).unwrap(), Status::Input);
    // skipped in the middle of the guard's walk
    let mut b = start(&m, true);
    command(&mut b, "HOLD_AUDIENCE");
    b.scene_confirm().unwrap();
    b.scene_tick(1300).unwrap();
    effects(&mut b);
    b.scene_skip().unwrap();
    assert!(!b.in_scene());
    assert!(
        effects(&mut b).is_empty(),
        "a skip leaves the presentation out"
    );
    assert_eq!(b.run(0).unwrap(), Status::Input);
    assert!(a.w.s == b.w.s, "the same state (RS 9.8)");
    assert_eq!(a.w.log, b.w.log, "and the same log");
    // skipped before the first line: the line goes to the log without a window
    let mut c = start(&m, true);
    command(&mut c, "OPEN");
    c.scene_skip().unwrap();
    assert!(!c.in_scene());
    assert_eq!(open_gates(&c), 3);
    assert_eq!(
        keys(&c),
        ["LINE.LEVER", "LINE.NOTE", "GATE.OPENED"],
        "skipped lines go to the log"
    );
}

#[test]
fn a_timeline_replays_without_the_clock() {
    let m = model();
    let mut r = start(&m, true);
    command(&mut r, "KINGLY");
    r.scene_tick(2100).unwrap();
    r.scene_confirm().unwrap();
    r.scene_tick(700).unwrap();
    r.scene_skip().unwrap();
    assert_eq!(r.run(0).unwrap(), Status::Input);
    command(&mut r, "HOLD_AUDIENCE");
    r.scene_confirm().unwrap();
    r.scene_tick(3000).unwrap();
    assert_eq!(r.run(0).unwrap(), Status::Input);
    let text = r.input_log().write().unwrap();
    let got =
        Replay::run(&m, &InputLog::read(&text).unwrap()).unwrap_or_else(|e| panic!("{e}\n{text}"));
    if got.w.s != r.w.s {
        let (a, b) = (r.save().write(), got.save().write());
        let diff: Vec<_> = a
            .lines()
            .zip(b.lines())
            .filter(|(x, y)| x != y)
            .take(6)
            .collect();
        panic!("the replay runs the rows in their order\n{text}\n{diff:#?}");
    }
    assert_eq!(got.w.log, r.w.log);
    let _ = Value::None;
}

#[test]
fn rewind_goes_back_to_the_line_before() {
    let m = model();
    let mut r = start(&m, true);
    command(&mut r, "KINGLY");
    r.scene_tick(2000).unwrap();
    assert_eq!(said(&r).as_deref(), Some("GATE.FOR_THE_KING"));
    r.scene_confirm().unwrap();
    r.scene_tick(1000).unwrap();
    assert_eq!(said(&r).as_deref(), Some("GATE.OPENED"));
    let log_len = r.w.log.len();
    r.scene_rewind().unwrap();
    assert_eq!(
        said(&r).as_deref(),
        Some("GATE.FOR_THE_KING"),
        "back to the line before (RS 9.8)"
    );
    assert_eq!(open_gates(&r), 2, "the state as it was then");
    assert_eq!(
        r.w.log.len(),
        log_len - 1,
        "the later line went from the log"
    );
    assert!(r.w.effects.is_empty(), "the quiet run is not shown");
    r.scene_rewind().unwrap();
    assert_eq!(
        said(&r).as_deref(),
        Some("GATE.FOR_THE_KING"),
        "the first line stays the first"
    );
    r.scene_confirm().unwrap();
    r.scene_tick(1000).unwrap();
    r.scene_confirm().unwrap();
    r.scene_tick(1000).unwrap();
    assert_eq!(r.run(0).unwrap(), Status::Input);
    let text = r.input_log().write().unwrap();
    assert!(text.contains(" rewind\n"), "{text}");
    let got =
        Replay::run(&m, &InputLog::read(&text).unwrap()).unwrap_or_else(|e| panic!("{e}\n{text}"));
    assert!(
        got.w.s == r.w.s,
        "the replay rewinds as the game did\n{text}"
    );
    assert_eq!(got.w.log, r.w.log);
}

#[test]
fn the_load_checks_durations_and_loops() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut f = MemFiles::default();
    read_tree(&repo, &repo.join("lib"), &mut f.engine);
    let bad = r#"
```roguescript
fn SET_QUEST():
    game.QUEST.state = 2

scene BAD() timeline:
    track PULSE loop 1s:
        0000ms        mark BEAT
        0500ms 0600ms shake(camera, strength: 1)
        1000ms        log(LINE.NOTE)
        0200ms        destroy(player)
    0000ms 0500ms SET_QUEST()
    0000ms 0500ms log(LINE.NOTE)
```
"#;
    f.game.insert("game.rlml".into(), format!("{GAME}\n{bad}"));
    f.game.insert("text/fi.rlml".into(), "language fi\nLINE.LEVER x\nLINE.NOTE x\nGATE.FOR_THE_KING x\nGATE.OPENED x\nKING.HELLO x\n".into());
    let (_, r) = rlml_core::check_game(&f, "game.rlml");
    let all: Vec<String> = r.errors().map(|d| d.to_string()).collect();
    let warns: Vec<String> = r.warnings().map(|d| d.to_string()).collect();
    for want in [
        "a loop track has no marks",
        "goes over its loop's length",
        "`SET_QUEST()` has nothing to show",
        "`log(LINE.NOTE)` has nothing to show",
    ] {
        assert!(all.iter().any(|e| e.contains(want)), "{want}: {all:#?}");
    }
    assert_eq!(
        all.iter()
            .filter(|e| e.contains("goes over its loop's length"))
            .count(),
        2,
        "0500+0600 and 1000: {all:#?}"
    );
    assert!(
        warns
            .iter()
            .any(|w| w.contains("`destroy` changes the state in a loop track")),
        "{warns:#?}"
    );
}
