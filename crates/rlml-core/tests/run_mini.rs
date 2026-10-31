//! Running the game: the new game of the STD minigame (M 5.1, M 3.2,
//! D 8) and instances from data.

use std::path::Path;

use rlml_core::load::MemFiles;
use rlml_core::model::Model;
use rlml_core::rt::state::Place;
use rlml_core::rt::value::{Pos, Value};
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
    assert!(!r.has_errors(), "{:#?}", r.errors().map(|d| d.to_string()).collect::<Vec<_>>());
    m
}

/// Cells of a character in a map file's first drawing after `map NAME`.
fn cells_of(src: &str, map: &str, c: char) -> Vec<(i32, i32)> {
    let start = src.find(&format!("map {map}")).unwrap();
    let body = &src[start..];
    let fence = body.find("```\n").unwrap() + 4;
    let end = body[fence..].find("```").unwrap() + fence;
    let mut out = Vec::new();
    for (y, row) in body[fence..end].lines().enumerate() {
        for (x, ch) in row.chars().enumerate() {
            if ch == c {
                out.push((x as i32, y as i32));
            }
        }
    }
    out
}

fn field<'a>(w: &'a World, e: usize, comp: &str, f: &str) -> &'a Value {
    w.s.get(e).get(comp).unwrap_or_else(|| panic!("no {comp}")).get(f).unwrap_or_else(|| panic!("no {comp}.{f}"))
}

#[test]
fn new_minigame() {
    let f = mini();
    let m = model(&f);
    let w = World::new_game(&m, 7).unwrap();
    let caves = &f.game["maps/caves.rlml"];
    assert_eq!(w.s.maps.iter().map(|m| m.name.as_str()).collect::<Vec<_>>(), ["CAVE_A"], "only the start map is loaded");
    let a = &w.s.maps[0];
    let (sx, sy) = cells_of(caves, "CAVE_A", '@')[0];
    assert_eq!(w.s.game.at.cells(), [Pos { map: 0, x: sx, y: sy }], "`at SPAWN`");
    assert_eq!(a.tile(0, 0, 0), Some("WALL"));
    assert_eq!(a.tile(0, sx, sy), Some("FLOOR"));
    let (dx, dy) = cells_of(caves, "CAVE_A", '>')[0];
    assert_eq!(a.tile(0, dx, dy), Some("STAIR.down"));
    assert_eq!(a.label("START"), [(dx, dy)]);

    let rats: Vec<usize> = (0..w.s.entities.len()).filter(|&e| w.s.get(e).kind == "RAT").collect();
    let rat_cells: Vec<(i32, i32)> = rats.iter().map(|&e| match w.s.get(e).place {
        Place::Map(p) => (p.x, p.y),
        _ => panic!("a rat off the map"),
    }).collect();
    assert_eq!(rat_cells, cells_of(caves, "CAVE_A", 'r'), "rats in reading order");
    let r = rats[0];
    assert_eq!(field(&w, r, "STD.HP", "now"), &Value::Number(3));
    assert_eq!(field(&w, r, "STD.HP", "max"), &Value::Number(3), "`max default now`");
    assert_eq!(field(&w, r, "STD.MELEE", "damage"), &Value::Dice { n: 1, sides: 2, bonus: 0 }, "a dice field stays dice");
    assert_eq!(field(&w, r, "STD.ALIGNMENT", "value"), &Value::Number(-100));
    assert_eq!(field(&w, r, "sight", "radius"), &Value::Number(6));
    assert_eq!(field(&w, r, "sight", "dark"), &Value::Number(1), "engine default");
    assert_eq!(field(&w, r, "hearing", "acuity"), &Value::Number(0));
    assert_eq!(field(&w, r, "ai", "behavior"), &Value::Name("STD.HUNTER".into()));
    assert_eq!(field(&w, r, "render", "tile"), &Value::Name("GLYPH.RAT".into()));
    assert_eq!(field(&w, r, "blocks", "values"), &Value::List(vec![Value::Name("move".into())]));

    let stairs = (0..w.s.entities.len()).find(|&e| w.s.get(e).kind == "STAIRS_DOWN").unwrap();
    assert_eq!(w.s.get(stairs).place, Place::Map(Pos { map: 0, x: dx, y: dy }));
    assert_eq!(field(&w, stairs, "STD.EXIT", "to"), &Value::Name("CAVE_B".into()));
    assert_eq!(field(&w, stairs, "STD.EXIT", "at"), &Value::Name("START".into()));

    // actors wake after the default cost (M 5.1, M 6.1.2), in creation order
    let wakes: Vec<(usize, i64)> = rats.iter().map(|&e| (e, w.s.wake_of(e).unwrap().at)).collect();
    assert!(wakes.iter().all(|(_, at)| *at == 120), "{wakes:?}");
    let seqs: Vec<u64> = rats.iter().map(|&e| w.s.wake_of(e).unwrap().seq).collect();
    assert!(seqs.windows(2).all(|p| p[0] < p[1]));
    assert!(w.s.wake_of(stairs).is_none(), "no `ai`, no wake");
    assert_eq!(w.created.len(), w.s.entities.len(), "every instance waits for `created`");
    assert_eq!(w.s.player, None, "`begin` creates the player");
}

#[test]
fn new_game_is_reproducible() {
    let m = model(&mini());
    let a = World::new_game(&m, 7).unwrap();
    let b = World::new_game(&m, 7).unwrap();
    assert_eq!(a.s, b.s);
    assert_eq!(a.rng, b.rng);
}

const DATA: &str = r#"game T
- start ROOM
- LOOTS POOL all

include lib/std.rlml

settings
- style ascii
- language fi

component LOOTS
- items collection 3

component ENCHANT
- level number

deck POOL
- COIN 2
- GEM ENCHANT 1d4

deck BAG shuffled
- COIN
- GEM
- RING

table DROP 1d6
- 1..3 COIN
- 4..6 none

entity COIN
- STD.ITEM

entity GEM extends COIN
- ENCHANT 0

entity RING extends GEM
- drop STD.ITEM

entity GOBLIN
- STD.HP now 4
- STD.PACK items BAG 2

map ROOM

```
#.g.#
```

legend:
- # WALL
- . FLOOR as START
- g FLOOR with GOBLIN STD.HP now 2
"#;

#[test]
fn instances_from_data() {
    let f = files(&[("game.rlml", DATA)]);
    let m = model(&f);
    let w = World::new_game(&m, 11).unwrap();
    // inheritance and `drop` (D 5.2)
    let ring = w.spawn_probe("RING");
    assert!(ring.iter().any(|c| c == "ENCHANT") && !ring.iter().any(|c| c == "STD.ITEM"), "{ring:?}");
    // `game`'s collection drawn whole from a deck: COIN ×2 and GEM, cut to 3 slots
    let loot = w.s.members(rlml_core::rt::world::GAME, "LOOTS", "items");
    let kinds: Vec<&str> = loot.iter().map(|&e| w.s.get(e).kind.as_str()).collect();
    assert_eq!(kinds, ["COIN", "COIN", "GEM"]);
    let gem = loot[2];
    assert!(matches!(field(&w, gem, "ENCHANT", "level"), Value::Number(1..=4)), "a dice override is rolled per instance");
    // a legend override applies before defaults: `max default now` follows it (M 3.2)
    let goblin = (0..w.s.entities.len()).find(|&e| w.s.get(e).kind == "GOBLIN").unwrap();
    assert_eq!(field(&w, goblin, "STD.HP", "now"), &Value::Number(2));
    assert_eq!(field(&w, goblin, "STD.HP", "max"), &Value::Number(2));
    // the goblin's pack: two cards from a shuffled shared deck, members created first
    let pack = w.s.members(goblin, "STD.PACK", "items");
    assert_eq!(pack.len(), 2);
    assert!(w.created.iter().position(|e| *e == pack[0]) < w.created.iter().position(|e| *e == goblin));
    assert_eq!(w.s.deck("BAG").unwrap().draw.len(), 1, "the shared deck lost two cards");
    assert_eq!(w.s.maps[0].label("START").len(), 2, "a label group in reading order");
}

trait Probe {
    fn spawn_probe(&self, kind: &str) -> Vec<String>;
}

impl Probe for World<'_> {
    /// The component names an instance of `kind` gets, from a copy.
    fn spawn_probe(&self, kind: &str) -> Vec<String> {
        let mut w = self.clone();
        let e = w.spawn(kind, &[], Place::Nowhere).unwrap();
        w.s.get(e).components.iter().map(|c| c.name.clone()).collect()
    }
}

// ---- the interpreter (R3) ----

use rlml_core::rt::eval::Outcome;
use rlml_core::rt::value::CommandValue;

fn started(m: &Model) -> World<'_> {
    let mut w = World::new_game(m, 7).unwrap();
    w.begin().unwrap();
    w
}

fn player(w: &World) -> usize {
    w.s.player.expect("`begin` made a player")
}

fn cmd(name: &str) -> CommandValue {
    CommandValue { name: name.into(), args: vec![] }
}

fn at(w: &World, e: usize) -> (i32, i32) {
    let p = w.s.at(e).unwrap();
    (p.x, p.y)
}

fn put(w: &mut World, e: usize, (x, y): (i32, i32)) {
    let map = w.s.at(e).unwrap().map;
    w.s.get_mut(e).place = Place::Map(Pos { map, x, y });
}

fn kinds(w: &World, kind: &str) -> Vec<usize> {
    (0..w.s.entities.len()).filter(|&e| w.s.get(e).kind == kind && w.s.get(e).alive).collect()
}

fn log_texts(w: &World) -> Vec<String> {
    w.log.iter().map(|l| w.text("fi", &l.key, &l.args)).collect()
}

#[test]
fn begin_spawns_the_player() {
    let f = mini();
    let m = model(&f);
    let w = started(&m);
    let p = player(&w);
    assert_eq!(w.s.get(p).kind, "HERO", "`PLAYER HERO` in the game block");
    assert_eq!(at(&w, p), cells_of(&f.game["maps/caves.rlml"], "CAVE_A", '@')[0]);
    assert_eq!(w.s.wake_of(p).unwrap().at, 0, "`control` wakes the player now (M 5.1)");
    assert_eq!(field(&w, p, "STD.HP", "now"), &Value::Number(10), "inherited from STD.DEFAULT.PLAYER");
    assert!(w.created.is_empty());
}

#[test]
fn walking_and_walls() {
    let f = mini();
    let m = model(&f);
    let mut w = started(&m);
    let p = player(&w);
    let (x, y) = at(&w, p);
    assert_eq!(w.start_command(p, &cmd("STD.MOVE.EAST")).unwrap(), Outcome::Done { cost: 120 });
    assert_eq!(at(&w, p), (x + 1, y));
    assert_eq!(w.sounds.last().map(|s| s.kind.as_str()), Some("STD.SOUND.STEP"), "a step makes a sound (STD 5)");
    w.start_command(p, &cmd("STD.MOVE.WEST")).unwrap();
    w.start_command(p, &cmd("STD.MOVE.WEST")).unwrap();
    assert_eq!(at(&w, p), (x - 1, y));
    assert_eq!(w.start_command(p, &cmd("STD.MOVE.WEST")).unwrap(), Outcome::Failed { waited: false }, "a wall: the command fails and takes no time");
    assert_eq!(at(&w, p), (x - 1, y));
}

#[test]
fn sneaking_waits() {
    let f = mini();
    let m = model(&f);
    let mut w = started(&m);
    let p = player(&w);
    assert_eq!(w.start_command(p, &cmd("STD.SNEAK")).unwrap(), Outcome::Done { cost: 0 });
    let Outcome::Waiting { delay, state } = w.start_command(p, &cmd("STD.MOVE.EAST")).unwrap() else { panic!("`wait 120` while sneaking") };
    assert_eq!(delay, 120);
    let before = at(&w, p);
    assert_eq!(w.continue_command(p, *state, true).unwrap(), Outcome::Done { cost: 120 });
    assert_eq!(at(&w, p), (before.0 + 1, before.1), "the step happens after the wait");
    assert_eq!(w.sounds.last().map(|s| s.kind.as_str()), Some("STD.SOUND.SNEAK"));
}

#[test]
fn a_rat_bites_until_the_player_dies() {
    let f = mini();
    let m = model(&f);
    let mut w = started(&m);
    let p = player(&w);
    let rat = kinds(&w, "RAT")[0];
    let (x, y) = at(&w, p);
    put(&mut w, rat, (x + 1, y));
    let attack = CommandValue { name: "STD.ATTACK".into(), args: vec![(None, Value::Entity(p))] };
    assert_eq!(w.start_command(rat, &attack).unwrap(), Outcome::Done { cost: 120 });
    let hp = |w: &World| field(w, p, "STD.HP", "now").as_number().unwrap();
    assert!((8..=9).contains(&hp(&w)), "1d2 damage: {}", hp(&w));
    let first = &log_texts(&w)[0];
    assert!(first == "Otat 1 vahinkoa." || first == "Otat 2 vahinkoa.", "{first}");
    for _ in 0..20 {
        if !w.s.get(p).alive {
            break;
        }
        w.start_command(rat, &attack).unwrap();
    }
    assert!(!w.s.get(p).alive, "DEATH destroys the player too (STD 8)");
    // DIED is sent inside HIT's `on` phase, so its messages come before the
    // hit's own `after` message, which sees the dead handle (STD 8)
    let log = log_texts(&w);
    let n = log.len();
    assert_eq!(log[n - 2], "Kuolet.");
    assert!(log[n - 1].starts_with("Otat "), "{log:?}");
    assert_eq!(w.s.player, Some(p), "`player` stays the dead handle (M 6.1.2)");
}

#[test]
fn bumping_a_rat_attacks_it() {
    let f = mini();
    let m = model(&f);
    let mut w = started(&m);
    let p = player(&w);
    let rat = kinds(&w, "RAT")[0];
    let (x, y) = at(&w, p);
    put(&mut w, rat, (x + 1, y));
    // the rat's attitude to the player is 50 − |−100 − 0| = −50: enemies (STD 7)
    let r = w.start_command(p, &cmd("STD.MOVE.EAST"));
    // the hit message asks whether the player sees the rat, which the senses give
    let err = r.unwrap_err();
    assert!(err.contains("`sees` is not implemented yet"), "{err}");
    assert_eq!(at(&w, p), (x, y), "a bump does not move");
    assert!(field(&w, rat, "STD.HP", "now").as_number().unwrap() < 3, "the hit was applied before the message");
}

#[test]
fn stairs_travel_at_the_action_end() {
    let f = mini();
    let m = model(&f);
    let mut w = started(&m);
    let p = player(&w);
    let caves = &f.game["maps/caves.rlml"];
    let down = cells_of(caves, "CAVE_A", '>')[0];
    put(&mut w, p, down);
    assert_eq!(w.start_command(p, &cmd("STD.TAKE_EXIT")).unwrap(), Outcome::Done { cost: 120 });
    assert_eq!(at(&w, p), down, "the move waits for the action's end (M 6.7)");
    assert_eq!(w.s.maps.len(), 2, "the target map loads at the call");
    w.end_action().unwrap();
    let to = w.s.at(p).unwrap();
    assert_eq!(w.s.maps[to.map].name, "CAVE_B");
    assert_eq!((to.x, to.y), cells_of(caves, "CAVE_B", '<')[0], "arrives at `START` on the up stairs");
    assert!(w.s.maps[0].frozen_at.is_some(), "the map left behind is frozen (M 6.1.8)");
    assert_eq!(w.s.active_map(), Some(to.map));
}

#[test]
fn texts_fill_placeholders() {
    let f = mini();
    let m = model(&f);
    let w = started(&m);
    let rat = kinds(&w, "RAT")[0];
    assert_eq!(w.text("fi", "STD.MESSAGE.COMBAT.HIT", &[("TARGET".into(), Value::Entity(rat)), ("AMOUNT".into(), Value::Number(2))]), "rotta ottaa 2 vahinkoa.");
    assert_eq!(w.text("fi", "NO.SUCH.KEY", &[]), "NO.SUCH.KEY", "a missing text shows as its key (M 7.3)");
    let hero = player(&w);
    assert_eq!(w.text("fi", "STD.MESSAGE.COMBAT.DIED", &[("VICTIM".into(), Value::Entity(hero))]), "sinä kuolee.", "HERO.NAME");
}
