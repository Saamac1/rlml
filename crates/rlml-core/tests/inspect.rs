//! The inspector, reading (RLML-kehitystyokalut.md step 3): the state by
//! path, a waiting command's names, and reading that changes nothing.

use std::path::Path;

use rlml_core::load::MemFiles;
use rlml_core::model::Model;
use rlml_core::rt::inspect::Node;
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

const GAME: &str = r#"game T
- start HALL
- TALLY

include lib/std.rlml

settings
- style ascii
- language fi

component TALLY
- count number
- seen list name

map HALL

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
command SLOW() cost 0:
    steps = 3
    game.TALLY.count += steps
    wait 60
    game.TALLY.count += 1
    return true
```
"#;

fn model() -> Model {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut f = MemFiles::default();
    read_tree(&repo, &repo.join("lib"), &mut f.engine);
    f.game.insert("game.rlml".into(), GAME.into());
    let (m, r) = rlml_core::check_game(&f, "game.rlml");
    assert!(
        !r.has_errors(),
        "{:#?}",
        r.errors().map(|d| d.to_string()).collect::<Vec<_>>()
    );
    m
}

fn leaf(n: Node) -> String {
    match n {
        Node::Leaf(v) => v,
        b => panic!("not a value: {b:?}"),
    }
}

#[test]
fn paths_read_the_state() {
    let m = model();
    let mut w = World::new_game(&m, 7).unwrap();
    w.begin().unwrap();
    let mut r = Runner::new(w);
    assert_eq!(r.run(0).unwrap(), Status::Input);
    let p = r.w.s.player.unwrap();
    let roots = r.inspect("").unwrap().lines("");
    assert!(
        roots.iter().any(|l| l.starts_with("game: turn 0")),
        "{roots:#?}"
    );
    assert!(
        roots.iter().any(|l| l.starts_with("map: HALL")),
        "{roots:#?}"
    );
    assert_eq!(leaf(r.inspect("game.turn").unwrap()), "0");
    assert_eq!(leaf(r.inspect("game.TALLY.count").unwrap()), "0");
    assert_eq!(
        leaf(r.inspect("game.TALLY.seen").unwrap()),
        "[]",
        "a list in full"
    );
    // a component by its full name or its unique last part
    let hp = leaf(r.inspect(&format!("entity#{p}.STD.HP.now")).unwrap());
    assert_eq!(leaf(r.inspect(&format!("entity#{p}.HP.now")).unwrap()), hp);
    let me = r.inspect(&format!("entity#{p}")).unwrap().lines("");
    assert!(
        me.iter()
            .any(|l| l.starts_with("defined: <engine>/lib/std/core.rlml:")),
        "where it is defined:\n{me:#?}"
    );
    assert!(r.inspect("map.HALL.labels.START").is_ok());
    for (bad, says) in [
        ("entity#99", "no entity #99"),
        ("game.NOPE", "no `NOPE` in game"),
        ("nothing", "no `nothing`"),
        ("game.turn.x", "goes past a value"),
    ] {
        let e = r.inspect(bad).unwrap_err();
        assert!(e.contains(says), "{bad}: {e}");
    }
}

#[test]
fn a_waiting_command_shows_its_names_and_reading_changes_nothing() {
    let m = model();
    let mut w = World::new_game(&m, 7).unwrap();
    w.begin().unwrap();
    let mut r = Runner::new(w);
    r.run(0).unwrap();
    let n = r.awaiting().unwrap();
    r.input(
        n,
        &CommandValue {
            name: "SLOW".into(),
            args: vec![],
        },
    )
    .unwrap();
    let p = r.w.s.player.unwrap();
    let before = r.w.s.clone();
    let wait = r.inspect(&format!("wait#{p}")).unwrap().lines("");
    assert!(wait.contains(&"command: SLOW".to_string()), "{wait:#?}");
    assert_eq!(
        leaf(r.inspect(&format!("wait#{p}.locals.steps")).unwrap()),
        "3"
    );
    assert!(
        r.inspect("wait")
            .unwrap()
            .lines("")
            .iter()
            .any(|l| l.contains("SLOW at wait point 0"))
    );
    for path in [
        "", "game", "maps", "map.HALL", "entities", "seats", "wait", "scenes",
    ] {
        r.inspect(path).unwrap();
    }
    assert!(r.w.s == before, "reading leaves the state as it was");
    r.run(0).unwrap();
    assert_eq!(
        leaf(r.inspect("game.TALLY.count").unwrap()),
        "4",
        "the command went on"
    );
}
