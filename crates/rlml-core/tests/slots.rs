//! Save slots and automatic saving (D 3.4, M 5.4, M 5.5): the stack, the
//! player's own slots, permadeath, leaving, `delete_save(auto)`, `nosave`
//! and the points where the engine saves on its own.

use std::path::Path;

use rlml_core::load::MemFiles;
use rlml_core::model::Model;
use rlml_core::rt::play::Session;
use rlml_core::rt::run::Runner;
use rlml_core::rt::store::{AutosaveRules, MemStore, Saved, Saves, Slot};

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

/// The minigame with `extra` as one more file of it.
fn mini(extra: &str) -> MemFiles {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut f = MemFiles::default();
    read_tree(&repo, &repo.join("lib"), &mut f.engine);
    let game = repo.join("esimerkit/mini");
    read_tree(&game, &game, &mut f.game);
    f.game.get_mut("game.rlml").unwrap().push_str("\ninclude extra.rlml\n");
    f.game.insert("extra.rlml".into(), extra.into());
    f
}

fn model(f: &MemFiles) -> Model {
    let (m, r) = rlml_core::check_game(f, "game.rlml");
    assert!(!r.has_errors(), "{:#?}", r.errors().map(|d| d.to_string()).collect::<Vec<_>>());
    m
}

fn errors(f: &MemFiles) -> Vec<String> {
    let (_, r) = rlml_core::check_game(f, "game.rlml");
    r.errors().map(|d| d.to_string()).collect()
}

fn session<'m>(m: &'m Model, keys: &str) -> Session<'m> {
    let mut s = Session::start(m, 7).unwrap();
    for k in keys.split_whitespace() {
        s.press(k).unwrap();
    }
    s
}

fn names(saves: &Saves<MemStore>) -> Vec<String> {
    saves.store.0.keys().cloned().collect()
}

#[test]
fn the_section_reads_and_is_checked() {
    let m = model(&mini("autosave\n- slots 3\n- manual 2\n- quit none\n- on input every 20\n- on checkpoint\n- on arrive\n"));
    let r = AutosaveRules::read(&m);
    assert_eq!(r, AutosaveRules { slots: 3, manual: 2, quit_save: false, on_input: Some(20), on_scene: false, on_checkpoint: true, on_arrive: true });
    let none = model(&mini(""));
    assert_eq!(AutosaveRules::read(&none), AutosaveRules { slots: 0, manual: 3, quit_save: true, on_input: None, on_scene: false, on_checkpoint: false, on_arrive: false }, "the defaults without the section");
    let bad = errors(&mini("autosave\n- slots many\n- quit maybe\n- on input every 0\n- on lunch\n- often\n"));
    for want in ["`slots` takes a count", "`quit` is save or none", "`on input` takes `every N`", "`on` is input, scene", "unknown `autosave` row `often`"] {
        assert!(bad.iter().any(|e| e.contains(want)), "{want}: {bad:#?}");
    }
}

#[test]
fn the_stack_drops_its_oldest_and_keeps_newer_ones_after_a_load() {
    let m = model(&mini("autosave\n- slots 2\n"));
    let mut saves = Saves::new(MemStore::default(), AutosaveRules::read(&m));
    let mut s = session(&m, "s");
    for _ in 0..3 {
        saves.autosave(s.r.save()).unwrap();
        s.press("s").unwrap();
    }
    assert_eq!(names(&saves), ["auto-1", "auto-2"], "two slots");
    let list = saves.list();
    assert!(list[0].save.stamp > list[1].save.stamp, "newest first");
    let newest = list[0].save.now.s.game.time;
    // the older one is loaded and played on: its next save takes the
    // oldest slot, which is itself, and the newer save stays (M 5.4)
    let older = list[1].slot;
    let save = saves.load(&m, older).unwrap();
    let mut r = Runner::load(&m, &save).unwrap();
    r.run(0).unwrap();
    saves.autosave(r.save()).unwrap();
    assert!(saves.list().iter().any(|l| l.save.now.s.game.time == newest), "the newer save is still in the stack");
}

#[test]
fn own_saves_take_their_own_slots() {
    let m = model(&mini("autosave\n- slots 2\n- manual 2\n"));
    let mut saves = Saves::new(MemStore::default(), AutosaveRules::read(&m));
    let mut s = session(&m, "s");
    assert_eq!(saves.manual(&s.r).unwrap(), Saved::In(Slot::Manual(1)));
    s.press("s").unwrap();
    assert_eq!(saves.manual(&s.r).unwrap(), Saved::In(Slot::Manual(2)));
    s.press("s").unwrap();
    assert_eq!(saves.manual(&s.r).unwrap(), Saved::In(Slot::Manual(1)), "the oldest own slot again");
    assert_eq!(saves.quit(&s.r).unwrap(), Some(Slot::Auto(1)), "`slots N`: leaving saves in the stack");
    let none = model(&mini("autosave\n- manual 0\n"));
    let mut saves = Saves::new(MemStore::default(), AutosaveRules::read(&none));
    assert!(saves.manual(&session(&none, "").r).is_err(), "`manual 0`: no own saves");
}

#[test]
fn permadeath_has_one_slot() {
    let m = model(&mini("autosave\n- slots 1\n"));
    let mut saves = Saves::new(MemStore::default(), AutosaveRules::read(&m));
    let s = session(&m, "s s");
    assert_eq!(saves.manual(&s.r).unwrap(), Saved::AndEnds, "the player's save ends the game (M 5.4)");
    saves.autosave(s.r.save()).unwrap();
    assert_eq!(saves.quit(&s.r).unwrap(), Some(Slot::Only));
    assert_eq!(names(&saves), ["game"]);
    saves.delete_auto();
    assert!(names(&saves).is_empty(), "`delete_save(auto)` removes the only slot");
    assert_eq!(saves.quit(&s.r).unwrap(), None, "and leaving saves nothing after it (M 5.5)");
}

#[test]
fn without_the_section_leaving_has_its_own_slot() {
    let m = model(&mini(""));
    let mut saves = Saves::new(MemStore::default(), AutosaveRules::read(&m));
    let s = session(&m, "s");
    saves.autosave(s.r.save()).unwrap();
    assert!(names(&saves).is_empty(), "`slots 0`: no automatic saves");
    assert_eq!(saves.quit(&s.r).unwrap(), Some(Slot::Quit));
    assert_eq!(names(&saves), ["quit"]);
}

#[test]
fn delete_auto_keeps_other_sessions_and_own_saves() {
    let m = model(&mini("autosave\n- slots 3\n"));
    let mut saves = Saves::new(MemStore::default(), AutosaveRules::read(&m));
    let s = session(&m, "s");
    saves.autosave(s.r.save()).unwrap();
    saves.new_session(&m);
    saves.autosave(s.r.save()).unwrap();
    saves.manual(&s.r).unwrap();
    saves.delete_auto();
    assert_eq!(names(&saves), ["auto-1", "manual-1"], "the earlier session's save and the own save stay");
    saves.autosave(s.r.save()).unwrap();
    assert_eq!(names(&saves).len(), 2, "no more automatic saves this session");
}

const POINTS: &str = r#"
autosave
- slots 5
- on input every 3
- on arrive

```roguescript
command REST() cost 120:
    autosave()
    autosave()
    return true
```
"#;

#[test]
fn the_runner_saves_at_its_points() {
    let m = model(&mini(POINTS));
    let mut s = Session::start(&m, 7).unwrap();
    // the start's input wait is the first; every third is saved
    assert!(s.r.due.is_empty());
    s.press("s").unwrap();
    assert!(s.r.due.is_empty());
    s.press("s").unwrap();
    assert_eq!(s.r.due.len(), 1, "`on input every 3`: the third wait");
    let at = s.r.due[0].now.s.game.time;
    assert_eq!(at, s.r.w.s.game.time, "taken at the wait");
    s.r.due.clear();
    s.r.player(&rlml_core::rt::value::CommandValue { name: "REST".into(), args: vec![] }).unwrap();
    assert_eq!(s.r.due.len(), 1, "two `autosave()` calls in one action make one save");
    s.r.run(0).unwrap();
    s.r.due.clear();
}

#[test]
fn arriving_on_a_map_saves_once() {
    let m = model(&mini("autosave\n- slots 5\n- on arrive\n"));
    // walking to the stairs, and the step onto them takes the player down
    // (golden stairs_down_and_back)
    let s = session(&m, "s s s j h j j j l l l l l l l k k l");
    assert!(s.r.due.is_empty(), "no point before the stairs");
    let mut s = s;
    s.press("l").unwrap();
    assert_eq!(s.r.due.len(), 1, "`on arrive`: one save");
    assert_eq!(s.r.due[0].now.s.maps.len(), 2, "taken after the player arrived on the second cave");
}

#[test]
fn a_nosave_map_stops_the_players_save() {
    let f = mini("");
    let mut f = f;
    let caves = f.game.get_mut("maps/caves.rlml").unwrap();
    *caves = caves.replace("map CAVE_A\n", "map CAVE_A nosave\n");
    let m = model(&f);
    let s = session(&m, "s");
    assert!(s.r.w.save_blocked(), "the player is on CAVE_A");
    let mut saves = Saves::new(MemStore::default(), AutosaveRules::read(&m));
    assert_eq!(saves.quit(&s.r).unwrap(), None, "leaving does not save there (M 5.2)");
}
