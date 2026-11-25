//! A command that waits for a scene (RLML-kohtaukset.md step 3): `wait
//! scene`, `wait confirm`, `open` and fields end the action, the engine
//! opens what the command waits for at the action's end, and the command
//! goes on as a new action when it ends (M 6.1.6). The `on scene` save
//! keeps the waiting command, and a loaded game shows its wait from the
//! start (M 5.4).

use std::path::Path;

use rlml_core::load::MemFiles;
use rlml_core::model::Model;
use rlml_core::rt::input::{InputLog, Replay};
use rlml_core::rt::run::{Runner, Status};
use rlml_core::rt::save::Save;
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
            out.insert(p.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/"), std::fs::read_to_string(&p).unwrap());
        }
    }
}

const GAME: &str = r#"game T
- start ROOM
- TALES

include lib/std.rlml
include text/*

settings
- style ascii
- language fi

autosave
- slots 3
- on scene

component TALES
- told number
- word text

map ROOM

```
#S..#
```

legend:
- # WALL
- . FLOOR
- S FLOOR as START

```roguescript
scene STORY(hero: entity):
    say(LINE.ONE, speaker: hero)
    game.TALES.told += 1000

command ASK() cost 30:
    game.TALES.told += 1
    wait scene STORY(actor)
    game.TALES.told += 10
    wait confirm
    game.TALES.told += 100
    answer = wait text(8)
    game.TALES.word = answer
    return true

command PAUSED() cost 0:
    start(STORY(actor))
    open PAUSE()
    game.TALES.told += 5
    return true
```

menu PAUSE
- RESUME
"#;

fn model() -> Model {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut f = MemFiles::default();
    read_tree(&repo, &repo.join("lib"), &mut f.engine);
    f.game.insert("game.rlml".into(), GAME.into());
    f.game.insert("text/fi.rlml".into(), "language fi\nLINE.ONE Hei.\n".into());
    let (m, r) = rlml_core::check_game(&f, "game.rlml");
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

fn told(r: &Runner) -> i64 {
    r.w.s.game.components.iter().find(|c| c.name == "TALES").unwrap().get("told").unwrap().as_number().unwrap()
}

fn word(r: &Runner) -> Option<Value> {
    r.w.s.game.components.iter().find(|c| c.name == "TALES").unwrap().get("word").cloned()
}

fn command(r: &mut Runner, name: &str) {
    let n = r.awaiting().unwrap();
    r.input(n, &CommandValue { name: name.into(), args: vec![] }).unwrap();
}

#[test]
fn a_command_waits_for_a_scene_a_confirmation_and_a_field() {
    let m = model();
    let mut r = start(&m);
    command(&mut r, "ASK");
    assert_eq!(told(&r), 1, "the command ran up to `wait scene`");
    assert!(matches!(r.scene_pause(), Some(Pause::Say { key, .. }) if key == "LINE.ONE"), "the action's end opened STORY");
    assert_eq!(r.run(0).unwrap(), Status::Scene);
    r.scene_confirm().unwrap();
    assert!(!r.in_scene());
    assert_eq!(told(&r), 1001, "the scene ran; the command waits for its new action");
    assert_eq!(r.run(0).unwrap(), Status::Scene, "the command went on to `wait confirm`");
    assert_eq!(told(&r), 1011);
    assert!(matches!(r.scene_pause(), Some(Pause::Confirm)));
    r.scene_confirm().unwrap();
    assert_eq!(r.run(0).unwrap(), Status::Scene, "and to its field");
    assert_eq!(told(&r), 1111);
    assert!(matches!(r.scene_pause(), Some(Pause::Ask { .. })));
    assert!(r.scene_answer(Value::Text("Dorothy Gale".into())).unwrap());
    assert_eq!(r.run(0).unwrap(), Status::Input);
    assert_eq!(word(&r), Some(Value::Text("Dorothy ".into())), "the answer went into the command's name, cut to 8");
    assert_eq!(r.w.s.game.time, 30, "the command's cost, once, at its end; the waits took no time");
    assert!(r.waiting.is_empty());

    let text = r.input_log().write().unwrap();
    let got = Replay::run(&m, &InputLog::read(&text).unwrap()).unwrap_or_else(|e| panic!("{e}\n{text}"));
    assert!(got.w.s == r.w.s, "the replay waits as the game did\n{text}");
}

#[test]
fn queued_scenes_run_before_the_menu_a_command_waits_for() {
    let m = model();
    let mut r = start(&m);
    command(&mut r, "PAUSED");
    assert!(matches!(r.scene_pause(), Some(Pause::Say { .. })), "the queued scene first (M 6.1.6)");
    r.scene_confirm().unwrap();
    assert!(matches!(r.scene_pause(), Some(Pause::Open { name, .. }) if name == "PAUSE"), "then the menu");
    assert_eq!(told(&r), 1000);
    assert!(r.scene_closed().unwrap());
    assert_eq!(r.run(0).unwrap(), Status::Input);
    assert_eq!(told(&r), 1005, "the command went on after its menu");
}

#[test]
fn the_on_scene_save_keeps_the_waiting_command() {
    let m = model();
    let mut r = start(&m);
    command(&mut r, "ASK");
    assert_eq!(r.due.len(), 1, "`on scene`: a save before STORY");
    r.scene_confirm().unwrap();
    r.run(0).unwrap();
    assert_eq!(r.due.len(), 2, "and one before the confirmation");
    // the first save: the command waits at `wait scene`
    let save = Save::read(&r.due[0].write()).unwrap();
    let mut l = Runner::load(&m, &save).unwrap();
    assert_eq!(told(&l), 1);
    assert_eq!(l.run(0).unwrap(), Status::Scene, "the loaded game shows STORY from its start (M 5.4)");
    assert_eq!(l.due.len(), 0, "the loaded state is the save: no second save");
    assert!(matches!(l.scene_pause(), Some(Pause::Say { key, .. }) if key == "LINE.ONE"));
    l.scene_confirm().unwrap();
    assert_eq!(l.run(0).unwrap(), Status::Scene);
    assert_eq!(told(&l), 1011, "the command went on from its `wait scene`");
    // the second save: the command waits at `wait confirm`
    let save = Save::read(&r.due[1].write()).unwrap();
    let mut l = Runner::load(&m, &save).unwrap();
    assert_eq!(l.run(0).unwrap(), Status::Scene);
    assert!(matches!(l.scene_pause(), Some(Pause::Confirm)));
    assert_eq!(told(&l), 1011);
    l.scene_confirm().unwrap();
    l.run(0).unwrap();
    assert!(l.scene_answer(Value::None).unwrap());
    assert_eq!(l.run(0).unwrap(), Status::Input);
    assert_eq!(told(&l), 1111);
}

#[test]
fn restart_shows_a_held_wait_again() {
    let m = model();
    let mut r = start(&m);
    command(&mut r, "ASK");
    r.scene_restart().unwrap();
    assert!(matches!(r.scene_pause(), Some(Pause::Say { key, .. }) if key == "LINE.ONE"));
    r.scene_confirm().unwrap();
    assert_eq!(told(&r), 1001, "the draft went, the scene ran once");
    r.run(0).unwrap();
    r.scene_restart().unwrap();
    assert!(matches!(r.scene_pause(), Some(Pause::Confirm)), "a confirmation restarts as itself");
    assert_eq!(told(&r), 1011);
}
