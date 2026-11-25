//! Foreground coroutine scenes (RS 9, RLML-kohtaukset.md step 1): the
//! queue, `say`, `wait confirm`, `wait scene`, the draft, `skip`,
//! `restart`, the game's time standing still and the input log.

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
            out.insert(p.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/"), std::fs::read_to_string(&p).unwrap());
        }
    }
}

const GAME: &str = r#"game T
- start ROOM
- TALES

include lib/std.rlml

settings
- style ascii
- language fi

component TALES
- told number
- roll number

map ROOM

```
#S..#
```

legend:
- # WALL
- . FLOOR
- S FLOOR as START

```roguescript
on begin() as OPENING:
    start(GREETING(player))

scene GREETING(hero: entity):
    say(LINE.ONE, speaker: hero)
    game.TALES.told += 1
    game.TALES.roll = 1d1000
    wait confirm
    wait scene ASIDE()
    say(LINE.TWO)

scene ASIDE():
    log(LINE.NOTE)
    say(LINE.ASIDE)

command TELL() cost 0:
    start(GREETING(actor))
    return true
```

messages
- LINE.NOTE
"#;

fn model() -> Model {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut f = MemFiles::default();
    read_tree(&repo, &repo.join("lib"), &mut f.engine);
    f.game.insert("game.rlml".into(), GAME.into());
    f.game.insert("text/fi.rlml".into(), "language fi\nLINE.ONE Hei.\nLINE.TWO Hyvästi.\nLINE.ASIDE Sivuun.\nLINE.NOTE Muistiin.\n".into());
    f.game.get_mut("game.rlml").unwrap().push_str("\ninclude text/*\n");
    let (m, r) = rlml_core::check_game(&f, "game.rlml");
    assert!(!r.has_errors(), "{:#?}", r.errors().map(|d| d.to_string()).collect::<Vec<_>>());
    m
}

fn start(m: &Model) -> Runner<'_> {
    let mut w = World::new_game(m, 7).unwrap();
    w.begin().unwrap();
    Runner::new(w)
}

fn told(r: &Runner) -> i64 {
    r.w.s.game.components.iter().find(|c| c.name == "TALES").unwrap().get("told").unwrap().as_number().unwrap()
}

fn said(r: &Runner) -> Option<String> {
    match r.scene_pause() {
        Some(Pause::Say { key, .. }) => Some(key.clone()),
        Some(Pause::Confirm) => Some("confirm".into()),
        Some(_) => Some("other".into()),
        None => None,
    }
}

#[test]
fn a_scene_from_begin_waits_line_by_line() {
    let m = model();
    let mut r = start(&m);
    assert_eq!(r.run(0).unwrap(), Status::Scene, "begin's scene starts before the first turn");
    assert_eq!(said(&r).as_deref(), Some("LINE.ONE"));
    let Some(Pause::Say { speaker, .. }) = r.scene_pause() else { panic!() };
    assert_eq!(*speaker, r.w.s.player, "the speaker shows (RS 9.6)");
    assert_eq!(told(&r), 0, "the line waits before the rest runs");
    r.scene_confirm().unwrap();
    assert_eq!(said(&r).as_deref(), Some("confirm"), "`wait confirm`");
    assert_eq!(told(&r), 1, "the draft shows as the state (RS 9.8)");
    r.scene_confirm().unwrap();
    assert_eq!(said(&r).as_deref(), Some("LINE.ASIDE"), "`wait scene ASIDE()`");
    r.scene_confirm().unwrap();
    assert_eq!(said(&r).as_deref(), Some("LINE.TWO"), "back in GREETING after ASIDE");
    assert_eq!(r.w.s.game.time, 0, "the game's time stands in a scene");
    r.scene_confirm().unwrap();
    assert!(!r.in_scene());
    assert_eq!(r.run(0).unwrap(), Status::Input, "the game goes on");
    let keys: Vec<(&str, bool)> = r.w.log.iter().map(|l| (l.key.as_str(), l.speech)).collect();
    assert_eq!(keys, [("LINE.ONE", true), ("LINE.NOTE", false), ("LINE.ASIDE", true), ("LINE.TWO", true)], "lines go to SPEECH, `log` to its message");
}

#[test]
fn restart_drops_the_draft_and_skip_runs_the_rest_without_lines() {
    let m = model();
    let mut r = start(&m);
    r.run(0).unwrap();
    r.scene_confirm().unwrap();
    assert_eq!(told(&r), 1);
    let roll = r.w.s.game.components.iter().find(|c| c.name == "TALES").unwrap().get("roll").cloned();
    r.scene_restart().unwrap();
    assert_eq!(told(&r), 0, "`restart` drops the draft (RS 9.8)");
    assert_eq!(said(&r).as_deref(), Some("LINE.ONE"), "from its start");
    assert_eq!(r.w.log.len(), 1, "the dropped run's lines go too");
    r.scene_confirm().unwrap();
    let again = r.w.s.game.components.iter().find(|c| c.name == "TALES").unwrap().get("roll").cloned();
    assert_eq!(again, roll, "the presentation stream starts the same (M 4.1)");
    r.scene_skip().unwrap();
    assert!(!r.in_scene(), "skipped to its end");
    assert_eq!(told(&r), 1, "the state changes ran");
    assert!(!r.w.log.iter().any(|l| l.key == "LINE.TWO" || l.key == "LINE.NOTE"), "lines and log messages were left out");
}

#[test]
fn a_scene_does_not_move_the_game_stream_and_replays() {
    let m = model();
    let mut r = start(&m);
    let rng = r.w.rng.clone();
    r.run(0).unwrap();
    for _ in 0..4 {
        r.scene_confirm().unwrap();
    }
    assert_eq!(r.w.rng, rng, "1d1000 in a scene drew from its own stream");
    r.run(0).unwrap();
    // a command starts the scene again: its second start has another stream
    let n = r.awaiting().unwrap();
    r.input(n, &CommandValue { name: "TELL".into(), args: vec![] }).unwrap();
    assert_eq!(r.run(0).unwrap(), Status::Scene, "a scene from a command's action end");
    r.scene_confirm().unwrap();
    r.scene_skip().unwrap();
    r.run(0).unwrap();
    assert_eq!(told(&r), 2);
    assert_eq!(r.w.s.scene_starts, 2);
    let text = r.input_log().write().unwrap();
    assert!(text.contains(" scene.confirm\n") && text.contains(" skip\n"), "{text}");
    let got = Replay::run(&m, &InputLog::read(&text).unwrap()).unwrap_or_else(|e| panic!("{e}\n{text}"));
    assert!(got.w.s == r.w.s, "the replay answers the scenes as the game did\n{text}");
    assert_eq!(got.w.log, r.w.log);
    let _ = Value::None;
}

// ---- dialogues (RS 9.7, RLML-kohtaukset.md step 2) ----

const DIALOGUES: &str = r#"
component STATS
- strength number default 10
- agility number default 10

component MOOD
- hinted bool default false

component QUEST
- state number

```roguescript
scene KING.TALK(speaker: entity):
    say(KING.HELLO)
    choice:
        QUEST hide when game.QUEST.state != 0:
            game.QUEST.state = 1
            say(KING.THANKS)
        GATE disabled when game.QUEST.state == 0:
            say(KING.GATE)
        BYE:
            return

scene GNOME.RIDDLE(speaker: entity):
    choice:
        RIDDLE:
            speaker.MOOD.hinted = true
        BYE:
            return
    choice:
        KITTEN hide when not speaker.MOOD.hinted:
            say(GNOME.HINT)
        BYE:
            return

scene CREATE(hero: entity):
    hero_name = wait text(16)
    if hero_name != none:
        add(hero, name(text: hero_name))
    points = 2
    choice:
        STRENGTH adjust hero.STATS.strength 10..25 pool points
        AGILITY adjust hero.STATS.agility 10..25 pool points
        DONE disabled when points > 0:
            return

scene VICTORY_SCENE():
    choice_made = open PAUSE()
    say(KING.BYE)

command TALK() cost 0:
    start(KING.TALK(actor))
    return true

command RIDDLE() cost 0:
    start(GNOME.RIDDLE(actor))
    return true

command NEW_HERO() cost 0:
    start(CREATE(actor))
    return true

command PAUSED() cost 0:
    start(VICTORY_SCENE())
    return true
```

menu PAUSE
- RESUME
"#;

fn dialogue_model() -> Model {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut f = MemFiles::default();
    read_tree(&repo, &repo.join("lib"), &mut f.engine);
    let game = GAME.replace("on begin() as OPENING:\n    start(GREETING(player))\n", "").replace("- TALES\n", "- TALES\n- QUEST\n");
    f.game.insert("game.rlml".into(), format!("{game}\n{DIALOGUES}\nentity HERO extends STD.DEFAULT.PLAYER\n- STATS\n- MOOD\n\ninclude text/*\n").replace("game T\n- start ROOM\n", "game T\n- start ROOM\n- PLAYER HERO\n"));
    f.game.insert("text/fi.rlml".into(), "language fi\nKING.HELLO Hei.\nKING.THANKS Kiitos.\nKING.GATE Portti.\nKING.BYE Hei hei.\nGNOME.HINT Vihje.\nLINE.ONE Hei.\nLINE.TWO Hyvästi.\nLINE.ASIDE Sivuun.\nLINE.NOTE Muistiin.\n".into());
    let (m, r) = rlml_core::check_game(&f, "game.rlml");
    assert!(!r.has_errors(), "{:#?}", r.errors().map(|d| d.to_string()).collect::<Vec<_>>());
    m
}

fn begin_scene<'m>(m: &'m Model, command: &str) -> Runner<'m> {
    let mut r = start(m);
    assert_eq!(r.run(0).unwrap(), Status::Input);
    let n = r.awaiting().unwrap();
    r.input(n, &CommandValue { name: command.into(), args: vec![] }).unwrap();
    assert_eq!(r.run(0).unwrap(), Status::Scene);
    r
}

fn items(r: &Runner) -> Vec<(String, bool)> {
    match r.scene_pause() {
        Some(Pause::Choice { items, .. }) => items.iter().map(|i| (i.name.clone(), i.enabled)).collect(),
        other => panic!("no choice: {other:?}"),
    }
}

fn quest(r: &Runner) -> i64 {
    r.w.s.game.components.iter().find(|c| c.name == "QUEST").unwrap().get("state").unwrap().as_number().unwrap()
}

#[test]
fn a_dialogue_hides_dims_and_branches() {
    let m = dialogue_model();
    let mut r = begin_scene(&m, "TALK");
    r.scene_confirm().unwrap();
    assert_eq!(items(&r), [("QUEST".into(), true), ("GATE".into(), false), ("BYE".into(), true)], "GATE dims before the quest (RS 9.7)");
    assert!(!r.scene_choose("GATE").unwrap(), "a dimmed item is no choice");
    assert!(r.scene_choose("QUEST").unwrap());
    assert_eq!(quest(&r), 1);
    r.scene_confirm().unwrap();
    assert!(!r.in_scene(), "the scene ends after the choice's block");
    r.run(0).unwrap();
    // the next talk: QUEST is hidden, GATE can be chosen
    let n = r.awaiting().unwrap();
    r.input(n, &CommandValue { name: "TALK".into(), args: vec![] }).unwrap();
    r.run(0).unwrap();
    r.scene_confirm().unwrap();
    assert_eq!(items(&r), [("GATE".into(), true), ("BYE".into(), true)], "QUEST hidden once taken");
    assert!(r.scene_choose("GATE").unwrap());
    assert!(matches!(r.scene_pause(), Some(Pause::Say { key, .. }) if key == "KING.GATE"));
}

#[test]
fn an_earlier_choice_opens_a_later_item_and_a_skip_stops_at_a_choice() {
    let m = dialogue_model();
    let mut r = begin_scene(&m, "RIDDLE");
    assert_eq!(items(&r).len(), 2);
    r.scene_choose("RIDDLE").unwrap();
    assert_eq!(items(&r), [("KITTEN".into(), true), ("BYE".into(), true)], "hinted opened KITTEN (RS 9.7)");
    r.scene_choose("BYE").unwrap();
    assert!(!r.in_scene());
    r.run(0).unwrap();
    // a skip runs up to a choice and stops there (RS 9.8)
    let n = r.awaiting().unwrap();
    r.input(n, &CommandValue { name: "TALK".into(), args: vec![] }).unwrap();
    r.run(0).unwrap();
    r.scene_skip().unwrap();
    assert!(matches!(r.scene_pause(), Some(Pause::Choice { .. })), "skipped to the choice");
}

#[test]
fn a_field_and_adjust_items_make_a_hero() {
    let m = dialogue_model();
    let mut r = begin_scene(&m, "NEW_HERO");
    assert!(matches!(r.scene_pause(), Some(Pause::Ask { kind: rlml_core::rt::world::AskKind::Text { max: 16 }, .. })));
    assert!(r.scene_answer(Value::Text("DorothyFromKansas".into())).unwrap());
    let hero = r.w.s.player.unwrap();
    assert_eq!(r.w.name_text("fi", &Value::Entity(hero)), "DorothyFromKansa", "cut to 16 characters");
    assert_eq!(items(&r)[2], ("DONE".into(), false), "two points left");
    assert!(r.scene_adjust("STRENGTH", 1).unwrap());
    assert!(r.scene_adjust("STRENGTH", 1).unwrap());
    assert!(!r.scene_adjust("AGILITY", 1).unwrap(), "the pool is empty");
    assert!(r.scene_adjust("STRENGTH", -1).unwrap(), "a cut gives a point back");
    assert!(r.scene_adjust("AGILITY", 1).unwrap());
    let Some(Pause::Choice { items, .. }) = r.scene_pause() else { panic!() };
    assert_eq!((items[0].adjust, items[1].adjust), (Some((11, 10, 25)), Some((11, 10, 25))));
    assert!(items[2].enabled, "DONE once the pool is spent: the items are seen again after an adjust");
    assert!(r.scene_choose("DONE").unwrap());
    assert!(!r.in_scene());
    let stats = r.w.s.get(hero).get("STATS").unwrap().clone();
    assert_eq!(stats.get("strength").unwrap().as_number().unwrap() + stats.get("agility").unwrap().as_number().unwrap(), 22, "two points spent");
    let text = r.input_log().write().unwrap();
    assert!(text.contains("scene.answer \"DorothyFromKansa\"") && text.contains("scene.adjust STRENGTH +1") && text.contains("scene.choose DONE"), "{text}");
    r.run(0).unwrap();
    let got = Replay::run(&m, &InputLog::read(&text).unwrap()).unwrap_or_else(|e| panic!("{e}\n{text}"));
    assert!(got.w.s == r.w.s, "the replay\n{text}");
}


#[test]
fn open_waits_for_its_menu() {
    let m = dialogue_model();
    let mut r = begin_scene(&m, "PAUSED");
    assert!(matches!(r.scene_pause(), Some(Pause::Open { name, into: Some(_) }) if name == "PAUSE"));
    assert!(r.scene_closed().unwrap());
    assert!(matches!(r.scene_pause(), Some(Pause::Say { key, .. }) if key == "KING.BYE"), "the scene goes on after the menu");
}
