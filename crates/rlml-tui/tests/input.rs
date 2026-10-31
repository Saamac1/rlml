//! Keys and contexts (D 11.3, M 8).

use std::path::Path;

use rlml_core::load::MemFiles;
use rlml_core::model::Model;
use rlml_core::rt::value::{CommandValue, Value};
use rlml_tui::input::{Action, Effect, Input, Keys};
use termplate_core::keys::{Binding, KeyCode, Mods, parse_binding};

fn read_tree(root: &Path, dir: &Path, out: &mut std::collections::BTreeMap<String, String>) {
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            read_tree(root, &p, out);
        } else {
            out.insert(p.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/"), std::fs::read_to_string(&p).unwrap());
        }
    }
}

fn repo() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn files(game: &[(&str, &str)]) -> MemFiles {
    let mut f = MemFiles::default();
    read_tree(&repo(), &repo().join("lib"), &mut f.engine);
    for (p, s) in game {
        f.game.insert(p.to_string(), s.to_string());
    }
    f
}

fn mini() -> MemFiles {
    let mut f = files(&[]);
    let game = repo().join("esimerkit/mini");
    read_tree(&game, &game, &mut f.game);
    f
}

fn model(f: &MemFiles) -> Model {
    let (m, r) = rlml_core::check_game(f, "game.rlml");
    assert!(!r.has_errors(), "{:?}", r.errors().map(|d| d.to_string()).collect::<Vec<_>>());
    m
}

fn key(s: &str) -> Binding {
    parse_binding(s).unwrap()
}

fn stack(names: &[&str]) -> Vec<String> {
    names.iter().map(|s| s.to_string()).collect()
}

/// Every key name in the specification's `keys` blocks reads (D 11.3).
#[test]
fn spec_key_names_read() {
    let mut bad = Vec::new();
    let mut n = 0;
    for doc in ["dataformaatit", "moottori", "vakiokirjasto", "roguescript"] {
        for e in std::fs::read_dir(repo().join(doc)).unwrap().flatten() {
            let src = std::fs::read_to_string(e.path()).unwrap();
            let mut in_keys = false;
            for line in src.lines() {
                if line.starts_with("keys ") {
                    in_keys = true;
                    continue;
                }
                if !line.starts_with("- ") {
                    in_keys = false;
                    continue;
                }
                if !in_keys {
                    continue;
                }
                let mut words: Vec<&str> = line[2..].split_whitespace().collect();
                // a row marker comes before the action (D 4.8)
                if matches!(words.first(), Some(&"new" | &"override" | &"drop")) {
                    words.remove(0);
                }
                let skip = match words.first() {
                    Some(&"open") | Some(&"toggle") => 2 + usize::from(words.get(2) == Some(&"stack")),
                    _ => 1,
                };
                for k in &words[skip.min(words.len())..] {
                    n += 1;
                    if let Err(err) = parse_binding(k) {
                        bad.push(format!("{}: {k}: {err}", e.path().display()));
                    }
                }
            }
        }
    }
    assert!(n > 30, "{n} key names");
    assert!(bad.is_empty(), "{bad:#?}");
}

#[test]
fn the_minigames_keys() {
    let f = mini();
    let m = model(&f);
    let keys = Keys::from_model(&m);
    assert_eq!(keys.base.as_deref(), Some("GAME"), "`keys GAME` in the game block");
    let game = stack(&["GAME"]);
    let cmd = |s: &str| Some(Action::Command(s.into()));
    assert_eq!(keys.lookup(&game, &key("k")).cloned(), cmd("STD.MOVE.NORTH"));
    assert_eq!(keys.lookup(&game, &key("up")).cloned(), cmd("STD.MOVE.NORTH"));
    assert_eq!(keys.lookup(&game, &key(">")).cloned(), cmd("STD.TAKE_EXIT"));
    assert_eq!(keys.lookup(&game, &key("<")).cloned(), cmd("STD.TAKE_EXIT"));
    assert_eq!(keys.lookup(&game, &key("z")).cloned(), Some(Action::Engine("pass".into())));
    assert_eq!(keys.lookup(&game, &key("esc")).cloned(), Some(Action::Open { menu: "PAUSE".into(), stack: false }), "ALWAYS");
    assert_eq!(keys.lookup(&game, &key("q")), None);
    assert!(keys.warnings.is_empty(), "{:?}", keys.warnings);
}

#[test]
fn contexts_stack_and_engine_defaults() {
    let f = mini();
    let m = model(&f);
    let keys = Keys::from_model(&m);
    let menu = stack(&["GAME", "MENU"]);
    // MENU is modal: its `k` is the cursor, not the game's move (D 11.3)
    assert_eq!(keys.lookup(&menu, &key("k")).cloned(), Some(Action::Engine("cursor.north".into())));
    assert_eq!(keys.lookup(&menu, &key("enter")).cloned(), Some(Action::Engine("confirm".into())));
    assert_eq!(keys.lookup(&menu, &key("l")), None, "a modal context stops keys it does not bind");
    // ALWAYS is heard above everything: `esc` opens (or closes) the pause menu
    assert_eq!(keys.lookup(&menu, &key("esc")).cloned(), Some(Action::Open { menu: "PAUSE".into(), stack: false }));
}

const PASS: &str = r#"game T
- start ROOM
- keys GAME

include lib/std.rlml

settings
- style ascii
- language fi

keys GAME
- SEARCH s
- repeat #
- MOVE.NORTH k num8
- quit shift+q

keys POPUP passthrough
- close backspace

keys MENU
- confirm o

map ROOM

```
#.#
```

legend:
- # WALL
- . FLOOR as START

```roguescript
command SEARCH(n: number count) cost 120:
    return
```
"#;

#[test]
fn passthrough_and_game_rows_over_defaults() {
    let f = files(&[("game.rlml", PASS)]);
    let m = model(&f);
    let keys = Keys::from_model(&m);
    let popup = stack(&["GAME", "POPUP"]);
    assert_eq!(keys.lookup(&popup, &key("backspace")).cloned(), Some(Action::Engine("close".into())));
    assert_eq!(keys.lookup(&popup, &key("k")).cloned(), Some(Action::Command("STD.MOVE.NORTH".into())), "passthrough lets the game's keys down");
    let menu = stack(&["GAME", "MENU"]);
    assert_eq!(keys.lookup(&menu, &key("o")).cloned(), Some(Action::Engine("confirm".into())));
    assert_eq!(keys.lookup(&menu, &key("enter")), None, "the game's row for `confirm` replaces the default keys");
    assert_eq!(keys.lookup(&stack(&["GAME"]), &Binding::key(KeyCode::Char('Q'), Mods::default())).cloned(), Some(Action::Engine("quit".into())), "shift+q is the key Q");
}

#[test]
fn the_repeat_count_goes_to_the_count_parameter() {
    let f = files(&[("game.rlml", PASS)]);
    let m = model(&f);
    let mut input = Input::new(&m);
    let st = input.base_stack();
    assert_eq!(input.press(&m, &st, key("#")), Effect::Counting(String::new()));
    assert_eq!(input.press(&m, &st, key("2")), Effect::Counting("2".into()));
    assert_eq!(input.press(&m, &st, key("0")), Effect::Counting("20".into()));
    assert_eq!(input.press(&m, &st, key("s")), Effect::Command(CommandValue { name: "SEARCH".into(), args: vec![(Some("n".into()), Value::Number(20))] }), "`#20s` (D 11.3)");
    assert_eq!(input.press(&m, &st, key("s")), Effect::Command(CommandValue { name: "SEARCH".into(), args: vec![] }), "the count is used once");
}

#[test]
fn numpad_keys_are_digits_in_a_terminal() {
    let f = files(&[("game.rlml", PASS)]);
    let m = model(&f);
    let mut input = Input::new(&m);
    let st = input.base_stack();
    assert_eq!(key("num8"), key("8"), "`numN` is the key `N` in a terminal (D 11.3)");
    let north = Effect::Command(CommandValue { name: "STD.MOVE.NORTH".into(), args: vec![] });
    assert_eq!(input.press(&m, &st, key("8")), north, "`num8` is bound, so `8` moves");
    // while the count is typed, digits write it whatever they are bound to
    input.press(&m, &st, key("#"));
    assert_eq!(input.press(&m, &st, key("8")), Effect::Counting("8".into()));
    assert_eq!(input.press(&m, &st, key("s")), Effect::Command(CommandValue { name: "SEARCH".into(), args: vec![(Some("n".into()), Value::Number(8))] }));
}
