//! Reloading in development mode (M 5.7, RLML-kehitystyokalut.md step 4):
//! the state goes over to the new files, maps merge with their drawings
//! by the cells the game changed, a changed waiting command is dropped,
//! the input log starts a segment, and a save loads under a changed
//! drawing the same way.

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
- TALLY

include lib/std.rlml
include text/*

settings
- style ascii
- language fi

tiles
- WALL blocks move sight sound
- DOOR.shut blocks move sight
- DOOR.open
- default blocks none lit

component TALLY
- count number

map HALL

```
#######
#S...+#
#.....#
#######
```

legend:
- # WALL
- . FLOOR
- S FLOOR as START
- + DOOR.shut as GATE

```roguescript
command OPEN() cost 0:
    tile(label(GATE), DOOR.open)
    return true

command BUMP() cost 0:
    game.TALLY.count += 1
    return true

command SLOW() cost 0:
    wait 60
    game.TALLY.count += 100
    return true

command RESTORE() cost 0:
    for p in changed():
        tile(p, tile_original(p))
    return true

command NOTE() cost 0:
    log(LINE.NOTE, N: changed().count)
    return true
```

messages
- LINE.NOTE N
"#;

fn files(game: &str) -> MemFiles {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut f = MemFiles::default();
    read_tree(&repo, &repo.join("lib"), &mut f.engine);
    f.game.insert("game.rlml".into(), game.into());
    f.game.insert(
        "text/fi.rlml".into(),
        "language fi\nLINE.NOTE Muutettu: {N}.\n".into(),
    );
    f
}

fn model(game: &str) -> Model {
    let (m, r) = rlml_core::check_game(&files(game), "game.rlml");
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

/// A reload, and the run to the next turn as the front end does it.
fn reload<'m>(r: &mut Runner<'m>, m: &'m Model) -> Vec<String> {
    let warnings = r.reload(m).unwrap();
    r.run(0).unwrap();
    warnings
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

fn tile(r: &Runner, x: i32, y: i32) -> String {
    r.w.s.maps[0].tile(0, x, y).unwrap_or("none").to_string()
}

fn count(r: &Runner) -> i64 {
    r.w.s
        .game
        .components
        .iter()
        .find(|c| c.name == "TALLY")
        .unwrap()
        .get("count")
        .unwrap()
        .as_number()
        .unwrap()
}

/// The game with its map's lower row drawn as `row` and BUMP adding `by`.
fn changed_game(row: &str, by: i64) -> String {
    GAME.replace("#.....#\n#######", &format!("{row}\n#######"))
        .replace(
            "game.TALLY.count += 1\n",
            &format!("game.TALLY.count += {by}\n"),
        )
}

#[test]
fn the_game_tracks_the_cells_it_changes() {
    let m = model(GAME);
    let mut r = start(&m);
    assert!(
        r.w.s.maps[0].changed.is_empty(),
        "loading the map is no change"
    );
    press(&mut r, "OPEN");
    assert_eq!(tile(&r, 5, 1), "DOOR.open");
    assert_eq!(
        r.w.s.maps[0].changed,
        [(((5, 1), 0), Some("DOOR.shut".into()))]
    );
    press(&mut r, "NOTE");
    assert_eq!(r.w.log.last().unwrap().args[0].1.to_string(), "1");
    press(&mut r, "RESTORE");
    assert_eq!(
        tile(&r, 5, 1),
        "DOOR.shut",
        "tile_original gave the drawn door back"
    );
    assert!(
        r.w.s.maps[0].changed.is_empty(),
        "a cell back at its value is unchanged"
    );
    // the record goes into a save and comes back
    press(&mut r, "OPEN");
    let save = Save::read(&r.save().write()).unwrap();
    let l = Runner::load(&m, &save).unwrap();
    assert_eq!(l.w.s.maps[0].changed, r.w.s.maps[0].changed);
}

#[test]
fn a_reload_keeps_the_state_and_merges_the_map() {
    let m = model(GAME);
    let new = model(&changed_game("#..#..#", 10));
    let conflict = model(&GAME.replace("#S...+#", "#S....#"));
    let mut r = start(&m);
    press(&mut r, "OPEN");
    press(&mut r, "BUMP");
    assert_eq!(count(&r), 1);
    let warnings = reload(&mut r, &new);
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(
        tile(&r, 3, 2),
        "WALL",
        "a cell the game left takes the new drawing"
    );
    assert_eq!(tile(&r, 5, 1), "DOOR.open", "the opened door stays");
    assert_eq!(count(&r), 1, "the state went over");
    press(&mut r, "BUMP");
    assert_eq!(count(&r), 11, "the new command runs from the next action");
    // a drawing that changed under the game's change: the game's stays
    let warnings = reload(&mut r, &conflict);
    assert!(
        warnings
            .iter()
            .any(|w| w.contains("5,1") && w.contains("DOOR.open")),
        "{warnings:?}"
    );
    assert_eq!(tile(&r, 5, 1), "DOOR.open");
    assert_eq!(
        tile(&r, 3, 2),
        "FLOOR",
        "and the wall went with the drawing"
    );
}

#[test]
fn a_changed_waiting_command_is_dropped() {
    let m = model(GAME);
    let new = model(&GAME.replace("game.TALLY.count += 100", "game.TALLY.count += 200"));
    let same = model(&changed_game("#..#..#", 1));
    // its body changed: dropped with a warning
    let mut r = start(&m);
    let n = r.awaiting().unwrap();
    r.input(
        n,
        &CommandValue {
            name: "SLOW".into(),
            args: vec![],
        },
    )
    .unwrap();
    assert_eq!(r.waiting.len(), 1);
    reload(&mut r, &new);
    assert_eq!(count(&r), 0, "the changed command did not go on");
    assert!(
        r.w.warnings.iter().any(|w| w.contains("SLOW")),
        "{:?}",
        r.w.warnings
    );
    // its body is the same: it goes on
    let mut r = start(&m);
    let n = r.awaiting().unwrap();
    r.input(
        n,
        &CommandValue {
            name: "SLOW".into(),
            args: vec![],
        },
    )
    .unwrap();
    reload(&mut r, &same);
    assert_eq!(count(&r), 100);
}

#[test]
fn the_log_starts_a_segment_at_the_reload() {
    let m = model(GAME);
    let new = model(&changed_game("#..#..#", 10));
    let mut r = start(&m);
    press(&mut r, "OPEN");
    reload(&mut r, &new);
    press(&mut r, "BUMP");
    press(&mut r, "STD.WAIT");
    let text = r.input_log().write().unwrap();
    assert!(text.contains("\nfrom "), "{text}");
    let log = InputLog::read(&text).unwrap();
    let got = Replay::run(&new, &log).unwrap_or_else(|e| panic!("{e}\n{text}"));
    assert!(
        got.w.s == r.w.s,
        "the segment replays with the new files\n{text}"
    );
    let e = Replay::run(&m, &log).err().expect("other files");
    assert!(e.contains("other files"), "{e}");
}

#[test]
fn a_save_loads_under_a_changed_drawing() {
    let m = model(GAME);
    let new = model(&changed_game("#..#..#", 1));
    let mut r = start(&m);
    press(&mut r, "OPEN");
    let save = Save::read(&r.save().write()).unwrap();
    let l = Runner::load(&new, &save).unwrap();
    assert_eq!(l.w.s.maps[0].tile(0, 3, 2), Some("WALL"));
    assert_eq!(l.w.s.maps[0].tile(0, 5, 1), Some("DOOR.open"));
    // a wall drawn where the player stands is told
    let walled = model(&GAME.replace("#S...+#", "#W...+#").replace(
        "- S FLOOR as START",
        "- S FLOOR as START\n- W WALL as START",
    ));
    let l = Runner::load(&walled, &save).unwrap();
    assert!(
        l.w.warnings
            .iter()
            .any(|w| w.contains("where the map blocks its move")),
        "{:?}",
        l.w.warnings
    );
}
