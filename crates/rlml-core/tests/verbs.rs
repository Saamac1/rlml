//! The target rule of verbs (M 8.1).

use std::path::Path;

use rlml_core::load::MemFiles;
use rlml_core::model::Model;
use rlml_core::rt::value::{Dir, Pos, Value};
use rlml_core::rt::verbs::Target;
use rlml_core::rt::world::World;

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

fn model(game: &str) -> Model {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut f = MemFiles::default();
    read_tree(&repo, &repo.join("lib"), &mut f.engine);
    f.game.insert("game.rlml".into(), game.into());
    let (m, r) = rlml_core::check_game(&f, "game.rlml");
    assert!(!r.has_errors(), "{:#?}", r.errors().map(|d| d.to_string()).collect::<Vec<_>>());
    m
}

const ROOM: &str = r#"game T
- start ROOM

include lib/std.rlml

settings
- style ascii
- language fi

tiles
- WALL blocks move sight sound
- default blocks none lit

entity HERO
- sight radius 8

entity KEYCARD
- KEY

entity GATE
- DOOR

map ROOM

```
##########
#A.k....d#
#........#
##########
```

legend:
- # WALL
- A FLOOR as START
- . FLOOR
- k FLOOR with KEYCARD
- d FLOOR with GATE

```roguescript
override on begin() as STD.SPAWN_PLAYER:
    control(spawn(HERO, at: label(START)))

component KEY(reach: number = 2)

component DOOR

command UNLOCK(key: entity from entities(in: near(actor.at, 9)) where it has KEY, door: entity from entities(in: near(actor.at, 9)) where it has DOOR) cost 120:
    return

command TOSS(item: entity from entities(in: near(actor.at, 9)) where it has KEY, to: pos aim item.KEY.reach) cost 120:
    return

command PUSH(dir: dir ask) cost 120:
    return

command SPOT(to: pos aim 3) cost 120:
    return
```
"#;

fn world(m: &Model) -> World<'_> {
    let mut w = World::new_game(m, 7).unwrap();
    w.begin().unwrap();
    w
}

fn kind_at(w: &World, kind: &str) -> (usize, Pos) {
    let e = w.s.entities_on(0).into_iter().find(|&e| w.s.get(e).kind == kind).unwrap();
    (e, w.s.at(e).unwrap())
}

#[test]
fn the_targets_go_in_any_order() {
    let m = model(ROOM);
    let mut w = world(&m);
    let p = w.s.player.unwrap();
    let (key, kp) = kind_at(&w, "KEYCARD");
    let (gate, gp) = kind_at(&w, "GATE");
    // the door first: it fills `door`, the second parameter
    let first = w.fill_verb(p, "UNLOCK", &[], Target::Cell(gp)).unwrap();
    assert_eq!(first, Some(("door".into(), Value::Entity(gate))));
    let second = w.fill_verb(p, "UNLOCK", &[first.unwrap()], Target::Cell(kp)).unwrap();
    assert_eq!(second, Some(("key".into(), Value::Entity(key))));
    // the key first
    let first = w.fill_verb(p, "UNLOCK", &[], Target::Cell(kp)).unwrap();
    assert_eq!(first, Some(("key".into(), Value::Entity(key))));
    assert!(w.verb_open("UNLOCK", &[first.clone().unwrap()]), "`door` is still open");
    assert_eq!(w.fill_verb(p, "UNLOCK", &[first.unwrap()], Target::Cell(kp)).unwrap(), None, "the key fits no other parameter");
    // an empty cell fits neither
    assert_eq!(w.fill_verb(p, "UNLOCK", &[], Target::Cell(Pos { map: 0, x: 5, y: 2 })).unwrap(), None);
    // a docked list's row
    assert_eq!(w.fill_verb(p, "UNLOCK", &[], Target::Entity(key)).unwrap(), Some(("key".into(), Value::Entity(key))));
}

#[test]
fn a_parameter_waits_for_the_ones_its_source_names() {
    let m = model(ROOM);
    let mut w = world(&m);
    let p = w.s.player.unwrap();
    let (key, kp) = kind_at(&w, "KEYCARD");
    let near = Pos { map: 0, x: 2, y: 2 };
    // `to` names `item`: a cell fills nothing until the item is chosen
    assert_eq!(w.fill_verb(p, "TOSS", &[], Target::Cell(near)).unwrap(), None);
    let item = w.fill_verb(p, "TOSS", &[], Target::Cell(kp)).unwrap().unwrap();
    assert_eq!(item, ("item".into(), Value::Entity(key)));
    let filled = [item];
    assert_eq!(w.fill_verb(p, "TOSS", &filled, Target::Cell(near)).unwrap(), Some(("to".into(), Value::Pos(near))));
    // the reach is the item's: 2
    assert_eq!(w.fill_verb(p, "TOSS", &filled, Target::Cell(Pos { map: 0, x: 5, y: 1 })).unwrap(), None, "out of reach");
    assert!(!w.verb_open("TOSS", &[filled[0].clone(), ("to".into(), Value::Pos(near))]));
}

#[test]
fn aim_ranges_and_asked_directions() {
    let m = model(ROOM);
    let mut w = world(&m);
    let p = w.s.player.unwrap();
    let at = w.s.at(p).unwrap();
    let cell = |x, y| Pos { map: 0, x, y };
    assert_eq!(w.fill_verb(p, "SPOT", &[], Target::Cell(cell(4, 2))).unwrap(), Some(("to".into(), Value::Pos(cell(4, 2)))));
    assert_eq!(w.fill_verb(p, "SPOT", &[], Target::Cell(cell(6, 2))).unwrap(), None, "beyond `aim 3`");
    assert_eq!(w.fill_verb(p, "PUSH", &[], Target::Cell(cell(8, 1))).unwrap(), Some(("dir".into(), Value::Dir(Dir::East))));
    assert_eq!(w.fill_verb(p, "PUSH", &[], Target::Cell(at)).unwrap(), None, "the own cell gives no direction");
}

#[test]
fn a_ring_among_the_sources_is_a_load_error() {
    let src = ROOM.replace("command SPOT(to: pos aim 3)", "command RING(a: pos aim b.x, b: pos aim a.x) cost 120:\n    return\n\ncommand SPOT(to: pos aim 3)");
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut f = MemFiles::default();
    read_tree(&repo, &repo.join("lib"), &mut f.engine);
    f.game.insert("game.rlml".into(), src);
    let (_, r) = rlml_core::check_game(&f, "game.rlml");
    let errors: Vec<String> = r.errors().map(|d| d.to_string()).collect();
    assert!(errors.iter().any(|e| e.contains("`a`, `b` name each other in a ring")), "{errors:#?}");
}

#[test]
fn the_coin_offers_the_verbs_a_cell_fits() {
    let src = ROOM.replace("command SPOT(to: pos aim 3) cost 120:", "command SPOT(to: pos aim 3) cost 120 noverb:");
    let m = model(&src);
    let mut w = world(&m);
    let p = w.s.player.unwrap();
    let (gate, gp) = kind_at(&w, "GATE");
    let (_, kp) = kind_at(&w, "KEYCARD");
    let all: Vec<String> = ["SPOT", "UNLOCK", "TOSS", "PUSH", "STD.WAIT"].iter().map(|s| s.to_string()).collect();
    let names = |c: &[(String, (String, Value))]| c.iter().map(|(n, _)| n.clone()).collect::<Vec<_>>();
    // the door: UNLOCK by its second parameter, PUSH towards it; not TOSS
    let coin = w.coin(p, &all, Target::Cell(gp)).unwrap();
    assert_eq!(names(&coin), ["UNLOCK", "PUSH"], "SPOT is out of range there, WAIT takes no target");
    assert_eq!(coin[0].1, ("door".to_string(), Value::Entity(gate)));
    // the key: UNLOCK and TOSS by the key, PUSH, and SPOT in range
    assert_eq!(names(&w.coin(p, &all, Target::Cell(kp)).unwrap()), ["SPOT", "UNLOCK", "TOSS", "PUSH"]);
    assert!(w.is_noverb("SPOT") && !w.is_noverb("UNLOCK"), "`noverb` (RS 7.1)");
}

#[test]
fn the_parameters_are_asked_in_order() {
    use rlml_core::rt::verbs::Ask;
    let src = ROOM.replace(
        "command SPOT(to: pos aim 3)",
        "command DIG(times: number count) cost 0:\n    return\n\ncommand NOTHING(x: entity from entities(in: near(actor.at, 9)) where it has HP) cost 0:\n    return\n\ncommand SPOT(to: pos aim 3)",
    );
    let m = model(&src);
    let mut w = world(&m);
    let p = w.s.player.unwrap();
    let (key, _) = kind_at(&w, "KEYCARD");
    let (gate, _) = kind_at(&w, "GATE");
    assert_eq!(w.next_ask(p, "UNLOCK", &[]).unwrap(), Ask::From { name: "key".into(), members: vec![key] });
    let given = [(Some("key".to_string()), Value::Entity(key))];
    assert_eq!(w.next_ask(p, "UNLOCK", &given).unwrap(), Ask::From { name: "door".into(), members: vec![gate] });
    let given = [(None, Value::Entity(key)), (None, Value::Entity(gate))];
    assert_eq!(w.next_ask(p, "UNLOCK", &given).unwrap(), Ask::Ready, "by position too");
    let given = [(Some("item".to_string()), Value::Entity(key))];
    assert_eq!(w.next_ask(p, "TOSS", &given).unwrap(), Ask::Aim { name: "to".into(), range: 2, candidates: vec![] }, "the range from the item");
    assert_eq!(w.next_ask(p, "PUSH", &[]).unwrap(), Ask::Direction { name: "dir".into(), pos: false });
    assert_eq!(w.next_ask(p, "DIG", &[]).unwrap(), Ask::Given("times".into(), Value::Number(1)), "`count` is 1 without the repeat count");
    assert_eq!(w.next_ask(p, "NOTHING", &[]).unwrap(), Ask::Fails, "an empty `from` fails before asking");
    assert_eq!(w.next_ask(p, "STD.WAIT", &[]).unwrap(), Ask::Ready);
}
