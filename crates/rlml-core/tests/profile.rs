//! The profile (D 3.5, M 5.6, RS 10.4): a session's view of it, the file
//! over sessions, kept lists, rollback and saves, and replays that do not
//! depend on this computer's profile.

use std::path::Path;

use rlml_core::load::MemFiles;
use rlml_core::model::Model;
use rlml_core::rt::input::{InputLog, Replay};
use rlml_core::rt::profile::{BACKUP, FILE, Profile};
use rlml_core::rt::run::{Runner, Status};
use rlml_core::rt::store::{MemStore, Store};
use rlml_core::rt::value::{CommandValue, ComponentValue, Value};
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

profile T
- HALL
- UNLOCKS

include lib/std.rlml

settings
- style ascii
- language fi

component HALL
- scores list SCORE 3 sort score desc

component SCORE
- name text
- score number

component UNLOCKS
- wins number

map ROOM

```
#@..#
```

legend:
- # WALL
- @ FLOOR as START
- . FLOOR

```roguescript
command WIN() cost 120:
    profile.UNLOCKS.wins += 1
    profile.HALL.scores += SCORE(name: "Iolo", score: game.turn)
    return true

command BACK() cost 0:
    rollback()
    return true

command VETERAN() cost 0:
    if profile.UNLOCKS.wins >= 2:
        return true
    return false
```
"#;

fn model(game: &str) -> Model {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut f = MemFiles::default();
    read_tree(&repo, &repo.join("lib"), &mut f.engine);
    f.game.insert("game.rlml".into(), game.into());
    let (m, r) = rlml_core::check_game(&f, "game.rlml");
    assert!(!r.has_errors(), "{:#?}", r.errors().map(|d| d.to_string()).collect::<Vec<_>>());
    m
}

/// A new game whose profile is `profile`'s file.
fn start<'m>(m: &'m Model, profile: &mut Profile<MemStore>) -> Runner<'m> {
    let (file, warning) = profile.load();
    assert!(warning.is_none(), "{warning:?}");
    let mut w = World::new_game_with(m, 7, None, Some(&file)).unwrap();
    w.begin().unwrap();
    let mut r = Runner::new(w);
    assert_eq!(r.run(0).unwrap(), Status::Input);
    r
}

/// The player's command, the world to the next turn and the profile's
/// writes to its file, as a front end does after each action.
fn input(r: &mut Runner, profile: &mut Profile<MemStore>, name: &str) -> bool {
    let ok = r.player(&CommandValue { name: name.into(), args: vec![] }).unwrap();
    r.run(0).unwrap();
    profile.write(&r.take_profile_writes()).unwrap();
    ok
}

fn view<'a>(r: &'a Runner, comp: &str) -> &'a ComponentValue {
    r.w.s.profile.iter().find(|c| c.name == comp).unwrap()
}

fn wins(r: &Runner) -> i64 {
    view(r, "UNLOCKS").get("wins").unwrap().as_number().unwrap()
}

fn scores(r: &Runner) -> Vec<i64> {
    let Some(Value::List(l)) = view(r, "HALL").get("scores") else { panic!("a list") };
    l.iter().map(|v| if let Value::Component(c) = v { c.get("score").unwrap().as_number().unwrap() } else { panic!() }).collect()
}

#[test]
fn the_profile_lives_over_sessions() {
    let m = model(GAME);
    let mut p = Profile::new(MemStore::default(), &m);
    let mut r = start(&m, &mut p);
    assert_eq!(wins(&r), 0, "the `profile` block's start values (D 3.5)");
    input(&mut r, &mut p, "WIN");
    assert_eq!(wins(&r), 1);
    assert!(p.store.read(FILE).is_some(), "written at the action's end (M 5.6)");
    let r2 = start(&m, &mut p);
    assert_eq!(wins(&r2), 1, "the next session's snapshot has it");
    assert_eq!(scores(&r2), [0]);
}

#[test]
fn a_kept_list_is_sorted_and_cut() {
    let m = model(GAME);
    let mut p = Profile::new(MemStore::default(), &m);
    let mut r = start(&m, &mut p);
    for _ in 0..4 {
        input(&mut r, &mut p, "WIN");
    }
    assert_eq!(scores(&r), [3, 2, 1], "`list SCORE 3 sort score desc`: largest first, the last dropped (D 5.1)");
}

#[test]
fn a_session_sees_its_snapshot_and_its_own_writes() {
    let m = model(GAME);
    let mut p = Profile::new(MemStore::default(), &m);
    let mut a = start(&m, &mut p);
    let mut b = start(&m, &mut p);
    input(&mut a, &mut p, "WIN");
    assert_eq!(wins(&b), 0, "b does not see a's later write (M 5.6)");
    input(&mut b, &mut p, "WIN");
    assert_eq!(wins(&b), 1);
    assert_eq!(wins(&start(&m, &mut p)), 1, "b's field replaced a's: each field as the session that wrote it last saw it");
}

#[test]
fn rollback_does_not_undo_the_profile() {
    let m = model(GAME);
    let mut p = Profile::new(MemStore::default(), &m);
    let mut r = start(&m, &mut p);
    input(&mut r, &mut p, "WIN");
    input(&mut r, &mut p, "BACK");
    assert_eq!(r.w.s.game.time, 0, "back to checkpoint 0");
    assert_eq!(wins(&r), 1, "the profile's write stays (M 5.3)");
}

#[test]
fn an_edited_file_is_kept_aside() {
    let m = model(GAME);
    let mut p = Profile::new(MemStore::default(), &m);
    let mut r = start(&m, &mut p);
    input(&mut r, &mut p, "WIN");
    let text = p.store.read(FILE).unwrap();
    p.store.write(FILE, &text.replace("(n 1)", "(n 99)")).unwrap();
    let (file, warning) = p.load();
    assert!(file.is_empty());
    assert!(warning.is_some_and(|w| w.contains("checksum")));
    assert!(p.store.read(FILE).is_none());
    assert!(p.store.read(BACKUP).is_some_and(|t| t.contains("(n 99)")), "the edited file is the backup");
}

#[test]
fn the_data_may_add_and_drop_components() {
    let m = model(GAME);
    let mut p = Profile::new(MemStore::default(), &m);
    let mut r = start(&m, &mut p);
    input(&mut r, &mut p, "WIN");
    // a later version of the game drops HALL and adds a field to UNLOCKS
    let later = GAME
        .replace("- HALL\n- UNLOCKS", "- UNLOCKS")
        .replace("- wins number\n", "- wins number\n- losses number default 5\n")
        .replace("    profile.HALL.scores += SCORE(name: \"Iolo\", score: game.turn)\n", "");
    let m2 = model(&later);
    let mut p2 = Profile::new(std::mem::take(&mut p.store), &m2);
    let mut r2 = start(&m2, &mut p2);
    assert!(r2.w.s.profile.iter().all(|c| c.name != "HALL"), "a dropped component is not seen");
    assert_eq!(view(&r2, "UNLOCKS").get("losses"), Some(&Value::Number(5)), "an added field gets its start value");
    assert_eq!(wins(&r2), 1);
    input(&mut r2, &mut p2, "WIN");
    assert!(p2.store.read(FILE).unwrap().contains("HALL"), "the dropped component stays in the file (D 3.5)");
}

#[test]
fn a_replay_has_its_own_snapshot() {
    let m = model(GAME);
    let mut p = Profile::new(MemStore::default(), &m);
    let mut r = start(&m, &mut p);
    input(&mut r, &mut p, "WIN");
    input(&mut r, &mut p, "WIN");
    // a session that depends on the profile: VETERAN succeeds with two wins
    let mut r = start(&m, &mut p);
    assert!(input(&mut r, &mut p, "VETERAN"));
    let text = r.input_log().write().unwrap();
    assert!(text.contains("\nprofile "), "the snapshot is in the log's header (M 4.4)\n{text}");
    // replayed where the profile is empty, it goes the same way
    let log = InputLog::read(&text).unwrap();
    let got = Replay::run(&m, &log).unwrap();
    assert!(got.w.s == r.w.s, "the replay does not read this computer's profile\n{text}");
}

#[test]
fn a_save_keeps_the_sessions_view() {
    use rlml_core::rt::save::Save;
    let m = model(GAME);
    let mut p = Profile::new(MemStore::default(), &m);
    let mut r = start(&m, &mut p);
    input(&mut r, &mut p, "WIN");
    let save = Save::read(&r.save().write()).unwrap();
    // later sessions win more; loading the save gives back its own view
    let mut other = start(&m, &mut p);
    input(&mut other, &mut p, "WIN");
    let loaded = Runner::load(&m, &save).unwrap();
    assert_eq!(wins(&loaded), 1, "the session's view (M 5.6)");
    assert!(p.store.read(FILE).unwrap().contains("(n 2)"), "the file keeps the later win");
}

#[test]
fn profile_blocks_are_checked() {
    let errors = |game: &str| {
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut f = MemFiles::default();
        read_tree(&repo, &repo.join("lib"), &mut f.engine);
        f.game.insert("game.rlml".into(), game.into());
        let (_, r) = rlml_core::check_game(&f, "game.rlml");
        r.errors().map(|d| d.to_string()).collect::<Vec<_>>()
    };
    let e = errors(&GAME.replace("profile T\n", "profile OTHER\n"));
    assert!(e.iter().any(|e| e.contains("named after the game")), "{e:#?}");
    let e = errors(&GAME.replace("profile T\n- HALL\n", "profile T\n- HALL\n\nprofile T\n- HALL\n"));
    assert!(e.iter().any(|e| e.contains("a second named `profile` block")), "{e:#?}");
    let e = errors(&GAME.replace("profile T\n- HALL\n", "profile T\n- HALL\n- NO_SUCH\n"));
    assert!(e.iter().any(|e| e.contains("NO_SUCH")), "{e:#?}");
}
