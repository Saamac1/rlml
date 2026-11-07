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
    assert_eq!(w.start_command(p, &cmd("STD.MOVE.EAST")).unwrap(), Outcome::Done { cost: 120 }, "the attack's cost");
    assert_eq!(at(&w, p), (x, y), "a bump does not move");
    let hp = field(&w, rat, "STD.HP", "now").as_number().unwrap();
    assert!(hp < 3);
    assert_eq!(log_texts(&w), [format!("rotta ottaa {} vahinkoa.", 3 - hp)], "the player sees the rat next to it");
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
    w.apply_switch();
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

// ---- the scheduler (R4) ----

use rlml_core::rt::run::{Runner, Status};

const SCHED: &str = r#"game T
- start ROOM
- COUNTER

include lib/std.rlml

settings
- style ascii
- language fi

component COUNTER
- turns number
- player_wakes number
- slow_done number

entity BOT
- ai behavior IDLER
- TICKS

entity FAST extends BOT
- movement speed 20

entity FAILING extends BOT
- ai behavior FAILER

map ROOM

```
#@bfx#
```

legend:
- # WALL
- @ FLOOR as START
- b FLOOR with BOT
- f FLOOR with FAST
- x FLOOR with FAILING

```roguescript
component TICKS(n: number = 0):
    on wake():
        self.TICKS.n += 1

ai IDLER:
    return STD.WAIT()

command NOPE() cost 60:
    return false

ai FAILER:
    return NOPE()

command SLOWLY() cost 60:
    wait 120
    game.COUNTER.slow_done += 1

on turn() as COUNT_TURNS:
    game.COUNTER.turns += 1

on wake(who) as COUNT_PLAYER when who == player:
    game.COUNTER.player_wakes += 1
```
"#;

fn counter(w: &World, f: &str) -> i64 {
    w.s.game.components.iter().find(|c| c.name == "COUNTER").unwrap().get(f).unwrap().as_number().unwrap()
}

fn ticks(w: &World, kind: &str) -> i64 {
    let e = kinds(w, kind)[0];
    field(w, e, "TICKS", "n").as_number().unwrap()
}

#[test]
fn rounds_wakes_and_speed() {
    let f = files(&[("game.rlml", SCHED)]);
    let m = model(&f);
    let mut r = Runner::new(started(&m));
    assert_eq!(r.run(0).unwrap(), Status::Input);
    assert_eq!(r.w.s.game.time, 0, "the player acts first, at 0 (M 5.1)");
    assert_eq!(counter(&r.w, "player_wakes"), 1);
    assert!(r.player(&cmd("STD.WAIT")).unwrap());
    assert_eq!(r.run(0).unwrap(), Status::Input);
    assert_eq!(r.w.s.game.time, 120);
    assert_eq!(counter(&r.w, "turns"), 1, "the round at 120, none at 0 (M 6.1.4)");
    // at 120 the player's wake (scheduled at 0) comes before the fast
    // one's (scheduled at 60): the sequence number decides (M 6.1.2)
    assert_eq!(ticks(&r.w, "FAST"), 1);
    assert_eq!(ticks(&r.w, "BOT"), 1);
    assert!(r.player(&cmd("STD.WAIT")).unwrap());
    assert_eq!(r.run(0).unwrap(), Status::Input);
    assert_eq!(r.w.s.game.time, 240);
    assert_eq!(ticks(&r.w, "FAST"), 3, "speed 20 wakes at 60, 120 and 180 (M 6.1.3)");
    assert_eq!(ticks(&r.w, "BOT"), 2);
    // an ai's failure loses the turn: the default delay, not its cost 60 (M 6.1.5)
    let failing = kinds(&r.w, "FAILING")[0];
    assert_eq!(r.w.s.wake_of(failing).unwrap().at, 360);
    assert_eq!(counter(&r.w, "player_wakes"), 3);
    assert_eq!(counter(&r.w, "turns"), 2);
}

#[test]
fn the_players_failure_and_free_actions_take_no_time() {
    let f = files(&[("game.rlml", SCHED)]);
    let m = model(&f);
    let mut r = Runner::new(started(&m));
    r.run(0).unwrap();
    assert!(!r.player(&cmd("NOPE")).unwrap(), "the command failed");
    assert_eq!(r.run(0).unwrap(), Status::Input);
    assert_eq!(r.w.s.game.time, 0, "no time went by");
    assert!(r.player(&cmd("STD.SNEAK")).unwrap(), "`cost 0`");
    assert_eq!(r.run(0).unwrap(), Status::Input);
    assert_eq!(r.w.s.game.time, 0);
    assert_eq!(counter(&r.w, "player_wakes"), 1, "no new `wake` for the same turn");
}

#[test]
fn a_waiting_command_goes_on_by_itself() {
    let f = files(&[("game.rlml", SCHED)]);
    let m = model(&f);
    let mut r = Runner::new(started(&m));
    r.run(0).unwrap();
    assert!(r.player(&cmd("SLOWLY")).unwrap());
    assert_eq!(counter(&r.w, "slow_done"), 0);
    assert_eq!(r.run(0).unwrap(), Status::Input, "no input while the command waits (M 6.1.6)");
    assert_eq!(counter(&r.w, "slow_done"), 1);
    assert_eq!(r.w.s.game.time, 180, "wait 120, then the cost 60");
    assert_eq!(counter(&r.w, "player_wakes"), 3, "the continuation is a wake too (M 6.1.6)");
}

#[test]
fn the_world_goes_on_without_a_player() {
    let f = files(&[("game.rlml", SCHED)]);
    let m = model(&f);
    let mut r = Runner::new(started(&m));
    r.run(0).unwrap();
    let p = r.w.s.player.unwrap();
    let env = rlml_core::rt::eval::Env::new(rlml_core::rt::eval::BodyKind::Handler, "test", None, "test");
    r.w.destroy(p, &env).unwrap();
    assert_eq!(r.run(3).unwrap(), Status::NoPlayer { rounds: 3 });
    assert_eq!(counter(&r.w, "turns"), 3);
    assert!(ticks(&r.w, "BOT") >= 3, "the others act (M 6.1.2)");
}

// ---- the senses (R5) ----

/// Runs the player's commands, one per turn, and gives the runner.
fn play<'m>(m: &'m Model, setup: impl FnOnce(&mut World), cmds: &[&str]) -> Runner<'m> {
    let mut w = World::new_game(m, 7).unwrap();
    w.begin().unwrap();
    setup(&mut w);
    let mut r = Runner::new(w);
    for c in cmds {
        if r.run(0).unwrap() != Status::Input {
            break;
        }
        r.player(&cmd(c)).unwrap();
    }
    r
}

#[test]
fn in_the_dark_the_player_sees_only_next_to_it() {
    let mut f = mini();
    let g = f.game.get_mut("game.rlml").unwrap();
    *g = g.replace("- default blocks none lit", "- default blocks none");
    let m = model(&f);
    let mut w = started(&m);
    let p = player(&w);
    let view = w.field_of_view(p).unwrap();
    let (x, y) = at(&w, p);
    assert_eq!(view.len(), 9, "no `lit` tiles: only `dark` 1 (M 7.2)");
    assert!(view.iter().all(|(vx, vy)| (vx - x).abs() <= 1 && (vy - y).abs() <= 1));
    let far = Pos { map: 0, x: x + 2, y };
    assert!(!w.sees(p, &[far]));
}

#[test]
fn the_minigame_caves_are_lit() {
    let f = mini();
    let m = model(&f);
    let mut w = started(&m);
    let p = player(&w);
    let at = w.s.at(p).unwrap();
    assert!(w.field_of_view(p).unwrap().len() > 9, "`default … lit`: the player's radius 8");
    assert!(w.sees(p, &[Pos { y: at.y + 3, ..at }]));
}

#[test]
fn a_lit_map_shows_the_room() {
    let lit = format!("{DATA}\ntiles\n- default lit\n- WALL blocks move sight\n");
    let f = files(&[("game.rlml", &lit)]);
    let m = model(&f);
    let mut w = started(&m);
    let p = player(&w);
    let goblin = kinds(&w, "GOBLIN")[0];
    let gp = w.s.at(goblin).unwrap();
    assert!(w.sees(p, &[gp]), "`default lit`: radius 8 in light");
}

#[test]
fn a_rat_hears_steps_and_hunts() {
    let f = mini();
    let m = model(&f);
    let mut r = play(
        &m,
        |w| {
            // one rat three floor cells south of the player, the others gone
            let p = w.s.player.unwrap();
            let at = w.s.at(p).unwrap();
            let rats = kinds(w, "RAT");
            w.s.get_mut(rats[0]).place = Place::Map(Pos { y: at.y + 3, ..at });
            for &other in &rats[1..] {
                w.s.get_mut(other).alive = false;
            }
        },
        // a step south: the rat, two cells away, hears it at level 1 (M 6.8)
        &["STD.MOVE.SOUTH", "STD.WAIT", "STD.WAIT", "STD.WAIT", "STD.WAIT"],
    );
    let p = r.w.s.player.unwrap();
    let rat = kinds(&r.w, "RAT")[0];
    assert_eq!(r.w.s.at(rat).unwrap().distance(r.w.s.at(p).unwrap()), 1, "the rat came next to the player");
    assert!(field(&r.w, p, "STD.HP", "now").as_number().unwrap() < 10, "and bit: {:?}", log_texts(&r.w));
    assert!(r.w.heard.contains_key(&rat), "the rat listened at its wake (M 6.8)");
    let _ = r.run(0);
}

#[test]
fn the_rat_hears_the_step_and_its_own_sounds_mask() {
    let f = mini();
    let m = model(&f);
    let mut w = started(&m);
    let p = player(&w);
    let rat = kinds(&w, "RAT")[0];
    let at = w.s.at(p).unwrap();
    w.s.get_mut(rat).place = Place::Map(Pos { x: at.x + 2, ..at });
    w.listen(rat);
    w.start_command(p, &cmd("STD.MOVE.EAST")).unwrap();
    let heard = w.listen(rat);
    assert_eq!(heard.len(), 1);
    assert_eq!((heard[0].kind.as_str(), heard[0].level, heard[0].source), ("STD.SOUND.STEP", 2, Some(p)), "level 3, one step away");
    assert!(w.listen(rat).is_empty(), "a sound is heard in one wake interval");
}

#[test]
fn routes_go_around_walls() {
    let f = mini();
    let m = model(&f);
    let w = started(&m);
    let p = player(&w);
    let from = w.s.at(p).unwrap();
    let caves = &f.game["maps/caves.rlml"];
    let (dx, dy) = cells_of(caves, "CAVE_A", '>')[0];
    let route = w.route(from, &[Pos { map: 0, x: dx, y: dy }], Some(p));
    assert_eq!(route.last(), Some(&Pos { map: 0, x: dx, y: dy }));
    assert!(route.windows(2).all(|s| s[0].distance(s[1]) == 1), "one step at a time");
    assert!(route.iter().all(|c| !w.blocks_tiles(*c, "move", Some(p))), "never through a wall");
    assert_eq!(w.route(from, &[Pos { map: 0, x: dx, y: dy }], Some(p)), route, "the same route every time");
}

#[test]
fn a_whole_run_is_reproducible() {
    let f = mini();
    let m = model(&f);
    let keys = ["STD.MOVE.EAST", "STD.MOVE.EAST", "STD.WAIT", "STD.MOVE.SOUTH", "STD.WAIT", "STD.MOVE.EAST", "STD.WAIT", "STD.WAIT"];
    let a = play(&m, |_| {}, &keys);
    let b = play(&m, |_| {}, &keys);
    assert_eq!(a.w.s, b.w.s);
    assert_eq!(a.w.log, b.w.log);
    assert!(a.w.s.game.time > 0);
}

// ---- golden transcripts (R6) ----

/// Plays keys from seed 7 and compares the transcript with
/// `tests/golden/NAME.txt`; `RLML_UPDATE_GOLDEN=1` writes it instead.
fn golden(name: &str, keys: &str) {
    golden_of(mini(), name, keys)
}

/// The group example (esimerkit/ryhma) with the standard library.
fn ryhma() -> MemFiles {
    let mut f = files(&[]);
    let game = repo().join("esimerkit/ryhma");
    read_tree(&game, &game, &mut f.game);
    f
}

fn golden_of(f: MemFiles, name: &str, keys: &str) {
    let m = model(&f);
    let keys: Vec<&str> = keys.split_whitespace().collect();
    let got = rlml_core::rt::play::transcript(&m, 7, &keys).unwrap();
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden").join(format!("{name}.txt"));
    if std::env::var_os("RLML_UPDATE_GOLDEN").is_some() {
        std::fs::write(&path, &got).unwrap();
        return;
    }
    let want = std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("no {}; run with RLML_UPDATE_GOLDEN=1", path.display()));
    if got != want {
        let line = got.lines().zip(want.lines()).position(|(a, b)| a != b).unwrap_or(0);
        panic!("{name}: the transcript differs from line {}:\n got:  {:?}\n want: {:?}", line + 1, got.lines().nth(line), want.lines().nth(line));
    }
}

#[test]
fn golden_walk_into_a_wall() {
    golden("walk_into_a_wall", "l l l");
}

#[test]
fn golden_a_rat_kills_the_waiting_player() {
    golden("rat_kills_player", "s s s s s s s s s s s s");
}

#[test]
fn golden_the_player_kills_a_rat() {
    golden("player_kills_rat", "s s s j h h");
}

#[test]
fn golden_stairs_down_and_back() {
    golden("stairs_down_and_back", "s s s j h j j j l l l l l l l k k l l > s <");
}

// ---- seats (M 6.1.10) ----

const SEATS: &str = r#"game T
- start ROOM
- seats 1..2

include lib/std.rlml

settings
- style ascii
- language fi

entity HERO
- blocks move
- sight radius 2
- WAKES

entity BUDDY extends HERO
- seat 2

map ROOM

```
#A.C........B#
```

legend:
- # WALL
- A FLOOR as START
- B FLOOR as P2
- C FLOOR as P3
- . FLOOR

```roguescript
override on begin() as STD.SPAWN_PLAYER:
    for n in seats():
        if n == 1:
            control(spawn(HERO, at: label(START)), seat: 1)
        else:
            control(spawn(HERO, at: label(P2)), seat: n)

component WAKES(n: number = 0):
    on wake():
        self.WAKES.n += 1

command NOPE() cost 60:
    return false

command TO_SEAT_2() cost 0:
    add(actor, seat(n: 2))

command ADOPT() cost 0:
    add(spawn(HERO, at: label(P3)), seat(n: 1))

command TO_FRIEND() cost 0:
    for e in members(seat_of(actor)) where it != actor:
        control(e)
```
"#;

fn seats_game<'m>(m: &'m Model, seats: u8, setup: impl FnOnce(&mut World)) -> Runner<'m> {
    let mut w = World::new_game_seats(m, 7, Some(seats)).unwrap();
    w.begin().unwrap();
    setup(&mut w);
    Runner::new(w)
}

/// A second member for seat 1 at `P3`, without `ai`; seat 1's first
/// member stays the controlled one.
fn second_member(w: &mut World) -> usize {
    let first = w.s.active(1).unwrap();
    let at = w.s.maps[0].label("P3")[0];
    let c = w.spawn("HERO", &[], Place::Map(Pos { map: 0, x: at.0, y: at.1 })).unwrap();
    w.control(c, Some(1));
    w.control(first, None);
    let now = w.s.game.time;
    w.s.schedule(c, now);
    c
}

#[test]
fn two_seats_take_turns() {
    let f = files(&[("game.rlml", SEATS)]);
    let m = model(&f);
    let mut r = seats_game(&m, 2, |_| {});
    let (a, b) = (r.w.s.active(1).unwrap(), r.w.s.active(2).unwrap());
    assert_eq!(r.w.s.players(), [a, b], "`players` by seat (M 6.1.10)");
    assert_eq!((r.w.s.seat_of(a), r.w.s.seat_of(b)), (Some(1), Some(2)));
    assert!(r.w.s.get(b).has("seat"), "the engine component `seat` (D 5.3)");
    assert_eq!(r.run(0).unwrap(), Status::Input);
    assert_eq!((r.awaiting(), r.w.s.player, r.w.s.game.time), (Some(1), Some(a), 0));
    assert!(r.input(2, &cmd("STD.WAIT")).is_err(), "seat 2 has no turn now");
    assert!(r.input(1, &cmd("STD.WAIT")).unwrap());
    assert_eq!(r.run(0).unwrap(), Status::Input);
    assert_eq!((r.awaiting(), r.w.s.player, r.w.s.game.time), (Some(2), Some(b), 0), "both members act at 0 (M 5.1)");
    assert!(r.input(2, &cmd("STD.WAIT")).unwrap());
    assert_eq!(r.run(0).unwrap(), Status::Input);
    assert_eq!((r.awaiting(), r.w.s.game.time), (Some(1), 120));
}

#[test]
fn one_seat_is_the_old_game() {
    let f = files(&[("game.rlml", SEATS)]);
    let m = model(&f);
    let mut r = seats_game(&m, 1, |_| {});
    assert_eq!(r.w.s.seats.len(), 1, "the lower bound joins");
    r.run(0).unwrap();
    assert!(r.player(&cmd("STD.WAIT")).unwrap());
    assert_eq!(r.run(0).unwrap(), Status::Input);
    assert_eq!((r.awaiting(), r.w.s.game.time), (Some(1), 120));
}

#[test]
fn the_other_member_rests_with_party_switch() {
    let f = files(&[("game.rlml", SEATS)]);
    let m = model(&f);
    let mut c = 0;
    let mut r = seats_game(&m, 1, |w| c = second_member(w));
    let a = r.w.s.active(1).unwrap();
    assert_eq!(r.w.s.seat_members(1), [a, c]);
    assert_eq!(r.run(0).unwrap(), Status::Input);
    assert_eq!(r.w.s.player, Some(a));
    assert!(r.player(&cmd("STD.WAIT")).unwrap());
    // `c` has no `ai`: it rests for the default cost and stays in the queue
    assert_eq!(r.run(0).unwrap(), Status::Input);
    assert_eq!((r.w.s.player, r.w.s.game.time), (Some(a), 120));
    assert_eq!(r.w.s.wake_of(c).unwrap().at, 120, "a member without `ai` rests (M 6.1.10)");
}

#[test]
fn switching_gives_no_free_turns() {
    let f = files(&[("game.rlml", SEATS)]);
    let m = model(&f);
    let mut c = 0;
    let mut r = seats_game(&m, 1, |w| c = second_member(w));
    let a = r.w.s.active(1).unwrap();
    r.run(0).unwrap();
    assert!(r.player(&cmd("TO_FRIEND")).unwrap(), "a free switch");
    assert_eq!(r.w.s.active(1), Some(c));
    // `c` acts on its own wake at 0; `a` goes on as another member, resting
    assert_eq!(r.run(0).unwrap(), Status::Input);
    assert_eq!((r.w.s.player, r.w.s.game.time), (Some(c), 0));
    assert!(r.player(&cmd("STD.WAIT")).unwrap());
    assert_eq!(r.run(0).unwrap(), Status::Input);
    assert_eq!((r.w.s.player, r.w.s.game.time), (Some(c), 120), "one turn each in a round");
    // `a` kept its place in each moment: it rested first at 0 and at 120,
    // one turn a round, so the switch cost it its turn and no more
    assert_eq!(r.w.s.wake_of(a).unwrap().at, 240, "the switch cost `a` its turn, no more and no less");
}

#[test]
fn every_member_takes_input_with_party_each() {
    let f = files(&[("game.rlml", &SEATS.replace("- seats 1..2", "- seats 1..2\n- party each"))]);
    let m = model(&f);
    let mut c = 0;
    let mut r = seats_game(&m, 1, |w| c = second_member(w));
    let a = r.w.s.active(1).unwrap();
    r.run(0).unwrap();
    assert!(r.player(&cmd("STD.WAIT")).unwrap());
    assert_eq!(r.run(0).unwrap(), Status::Input);
    assert_eq!((r.w.s.player, r.w.s.active(1), r.w.s.game.time), (Some(c), Some(c), 0));
    assert!(r.player(&cmd("STD.WAIT")).unwrap());
    assert_eq!(r.run(0).unwrap(), Status::Input);
    assert_eq!((r.w.s.player, r.w.s.game.time), (Some(a), 120));
}

#[test]
fn control_moves_when_the_controlled_member_has_no_wake() {
    let f = files(&[("game.rlml", SEATS)]);
    let m = model(&f);
    let mut c = 0;
    let mut r = seats_game(&m, 1, |w| c = second_member(w));
    let a = r.w.s.active(1).unwrap();
    r.w.s.unschedule(a);
    assert_eq!(r.run(0).unwrap(), Status::Input, "a paralysed member does not stall the game");
    assert_eq!((r.w.s.player, r.w.s.active(1)), (Some(c), Some(c)));
}

#[test]
fn the_world_goes_on_when_every_seat_is_empty() {
    let f = files(&[("game.rlml", SEATS)]);
    let m = model(&f);
    let mut r = seats_game(&m, 2, |_| {});
    r.run(0).unwrap();
    let env = rlml_core::rt::eval::Env::new(rlml_core::rt::eval::BodyKind::Handler, "test", None, "test");
    let a = r.w.s.active(1).unwrap();
    r.w.destroy(a, &env).unwrap();
    assert_eq!(r.run(0).unwrap(), Status::Input, "seat 2 still plays");
    assert_eq!(r.awaiting(), Some(2));
    let b = r.w.s.active(2).unwrap();
    r.w.destroy(b, &env).unwrap();
    assert_eq!(r.run(2).unwrap(), Status::NoPlayer { rounds: 2 });
    assert_eq!(r.w.s.seat_of(a), Some(1), "a dead member keeps its seat for `seat_of`");
    assert!(r.w.s.players().is_empty());
}

#[test]
fn the_shared_view_is_the_union() {
    let f = files(&[("game.rlml", SEATS)]);
    let m = model(&f);
    let mut r = seats_game(&m, 2, |_| {});
    let (a, b) = (r.w.s.active(1).unwrap(), r.w.s.active(2).unwrap());
    let va = r.w.field_of_view(a).unwrap();
    let vb = r.w.field_of_view(b).unwrap();
    let shared = r.w.player_view(0);
    assert!(va.is_subset(&shared) && vb.is_subset(&shared), "every member's view (M 7.2)");
    assert_eq!(shared.len(), va.union(&vb).count());
    let (bx, by) = at(&r.w, b);
    assert!(!va.contains(&(bx, by)));
    assert!(r.w.in_view(&[Pos { map: 0, x: bx, y: by }]), "`in_view` sees what any member sees");
    assert!(!r.w.sees(a, &[Pos { map: 0, x: bx, y: by }]), "`sees` stays one member's");
    let env = rlml_core::rt::eval::Env::new(rlml_core::rt::eval::BodyKind::Handler, "test", None, "test");
    r.w.destroy(a, &env).unwrap();
    r.w.destroy(b, &env).unwrap();
    assert!(r.w.in_view(&[Pos { map: 0, x: bx, y: by }]), "the last shared view stays");
}

#[test]
fn control_needs_a_joined_seat() {
    let f = files(&[("game.rlml", &SEATS.replace("seat: n)", "seat: 2)").replace("control(spawn(HERO, at: label(START)), seat: 1)", "control(spawn(HERO, at: label(START)), seat: 2)"))]);
    let m = model(&f);
    let mut w = World::new_game_seats(&m, 7, Some(1)).unwrap();
    let e = w.begin().unwrap_err();
    assert!(e.contains("the seat is not joined"), "{e}");
}

#[test]
fn std_spawns_a_member_per_seat() {
    let mut f = mini();
    let g = f.game.get_mut("game.rlml").unwrap();
    *g = g.replace("- PLAYER HERO", "- PLAYER HERO\n- seats 1..2");
    let m = model(&f);
    let mut r = seats_game(&m, 2, |_| {});
    let (a, b) = (r.w.s.active(1).unwrap(), r.w.s.active(2).unwrap());
    assert_eq!(r.w.s.players(), [a, b], "`SPAWN_PLAYER` makes one per seat (STD 8)");
    assert_ne!(at(&r.w, a), at(&r.w, b), "on different cells around `game.at`");
    assert_eq!(r.w.s.get(b).kind, "HERO");
    assert!(r.w.s.leash, "`leash` defaults to on with more than one seat (D 3.3)");
    for _ in 0..6 {
        assert_eq!(r.run(0).unwrap(), Status::Input);
        r.player(&cmd("STD.WAIT")).unwrap();
    }
    assert_eq!(r.run(0).unwrap(), Status::Input);
    assert_eq!((r.awaiting(), r.w.s.game.time), (Some(1), 360), "three rounds, both seats taking turns");
}

fn wakes(w: &World, e: usize) -> i64 {
    field(w, e, "WAKES", "n").as_number().unwrap()
}

#[test]
fn a_kept_turn_sends_no_second_wake() {
    let f = files(&[("game.rlml", SEATS)]);
    let m = model(&f);
    let mut r = seats_game(&m, 2, |_| {});
    let (a, b) = (r.w.s.active(1).unwrap(), r.w.s.active(2).unwrap());
    r.run(0).unwrap();
    assert!(!r.input(1, &cmd("NOPE")).unwrap(), "a failure keeps the turn");
    // the kept turn keeps its place: seat 1 is asked again before seat 2
    assert_eq!(r.run(0).unwrap(), Status::Input);
    assert_eq!((r.awaiting(), r.w.s.game.time), (Some(1), 0), "asked again first (M 6.1.5)");
    assert!(r.input(1, &cmd("STD.WAIT")).unwrap());
    assert_eq!(r.run(0).unwrap(), Status::Input);
    assert_eq!(r.awaiting(), Some(2));
    assert_eq!((wakes(&r.w, a), wakes(&r.w, b)), (1, 1), "one `wake` per turn (M 6.1.5)");
}

#[test]
fn the_leash_refuses_only_a_widening_step() {
    let layout = "\nlayout L\n\n```\nm\n```\n\nlegend:\n- m map size 6x3\n";
    let f = files(&[("game.rlml", &format!("{}{layout}", SEATS.replace("- seats 1..2", "- seats 1..2\n- leash on")))]);
    let m = model(&f);
    let mut r = seats_game(&m, 2, |_| {});
    assert_eq!(r.w.view_dims(), (6, 3), "the map area's size from the layout (M 7.1)");
    let (a, b) = (r.w.s.active(1).unwrap(), r.w.s.active(2).unwrap());
    // spread wider than the limit 4: stepping toward the group is allowed
    let a_x = at(&r.w, a).0;
    r.run(0).unwrap();
    assert!(r.input(1, &cmd("STD.MOVE.EAST")).unwrap());
    assert_eq!(at(&r.w, a).0, a_x + 1, "a spread group can close up");
    // close together: a step that widens the box past the limit is refused
    r.w.s.get_mut(b).place = Place::Map(Pos { map: 0, x: 2, y: 0 });
    r.w.s.get_mut(a).place = Place::Map(Pos { map: 0, x: 5, y: 0 });
    r.run(0).unwrap();
    assert!(r.input(2, &cmd("STD.WAIT")).unwrap());
    r.run(0).unwrap();
    assert_eq!(r.awaiting(), Some(1));
    assert!(!r.input(1, &cmd("STD.MOVE.EAST")).unwrap(), "`LEASHED` (STD 5)");
    assert_eq!(at(&r.w, a).0, 5);
}

#[test]
fn adding_seat_makes_a_member() {
    let f = files(&[("game.rlml", SEATS)]);
    let m = model(&f);
    let mut r = seats_game(&m, 1, |_| {});
    r.run(0).unwrap();
    assert!(r.player(&cmd("ADOPT")).unwrap());
    let c = kinds(&r.w, "HERO").into_iter().find(|&e| Some(e) != r.w.s.active(1)).unwrap();
    assert_eq!(r.w.s.seat_of(c), Some(1), "`add(e, seat(n: 1))` goes through the seat table");
    assert_eq!(r.w.s.players().len(), 2);
}

#[test]
fn a_data_member_of_a_seat_not_joined_is_no_member() {
    let f = files(&[("game.rlml", SEATS)]);
    let m = model(&f);
    for (seats, member) in [(1, None), (2, Some(2))] {
        let mut w = World::new_game_seats(&m, 7, Some(seats)).unwrap();
        let e = w.spawn("BUDDY", &[], Place::Map(Pos { map: 0, x: 3, y: 0 })).unwrap();
        assert_eq!(w.s.seat_of(e), member);
        assert_eq!(w.s.get(e).has("seat"), member.is_some(), "`has seat` agrees with `seat_of`");
    }
}

#[test]
fn reversed_seats_without_the_checker_still_start() {
    let f = files(&[("game.rlml", &SEATS.replace("- seats 1..2", "- seats 3..1"))]);
    let (m, r) = rlml_core::check_game(&f, "game.rlml");
    assert!(r.has_errors(), "the checker refuses `seats 3..1`");
    let w = World::new_game_seats(&m, 7, None).unwrap();
    assert_eq!(w.s.seat_range, (1, 3), "the bounds in order");
    assert_eq!(w.s.seats.len(), 1, "the lower bound joins");
}

#[test]
fn a_new_wake_entry_gets_its_wake() {
    let f = files(&[("game.rlml", SEATS)]);
    let m = model(&f);
    let mut r = seats_game(&m, 2, |_| {});
    let a = r.w.s.active(1).unwrap();
    r.run(0).unwrap();
    assert!(!r.input(1, &cmd("NOPE")).unwrap(), "a kept turn");
    // the kept turn is taken away and a new one scheduled, as a move into a
    // collection and back would do
    r.w.s.unschedule(a);
    r.w.s.schedule(a, 0);
    assert_eq!(r.run(0).unwrap(), Status::Input);
    assert!(r.input(2, &cmd("STD.WAIT")).unwrap());
    assert_eq!(r.run(0).unwrap(), Status::Input);
    assert_eq!(r.awaiting(), Some(1));
    assert_eq!(wakes(&r.w, a), 2, "the new entry is a new turn with its own `wake`");
}

#[test]
fn a_seat_field_is_read_only() {
    let f = files(&[("game.rlml", &SEATS.replace("command ADOPT() cost 0:", "command BAD() cost 0:\n    actor.seat.n = 2\n\ncommand ADOPT() cost 0:"))]);
    let (_, r) = rlml_core::check_game(&f, "game.rlml");
    assert!(r.errors().any(|d| d.to_string().contains("`actor.seat.n` is read-only")), "{:#?}", r.errors().map(|d| d.to_string()).collect::<Vec<_>>());
}

#[test]
fn moving_the_controlled_member_to_another_seat() {
    let f = files(&[("game.rlml", SEATS)]);
    let m = model(&f);
    let mut r = seats_game(&m, 2, |_| {});
    let (a, b) = (r.w.s.active(1).unwrap(), r.w.s.active(2).unwrap());
    r.run(0).unwrap();
    assert!(r.input(1, &cmd("TO_SEAT_2")).unwrap());
    assert_eq!((r.w.s.seat_of(a), r.w.s.active(1), r.w.s.active(2)), (Some(2), None, Some(b)));
    assert_eq!(r.w.s.players(), [b, a], "seat 2 in joining order");
    // the game goes on: seat 2's members take their turns
    assert_eq!(r.run(0).unwrap(), Status::Input);
    assert_eq!(r.awaiting(), Some(2));
}

#[test]
fn without_a_written_size_the_view_is_the_map() {
    let f = mini();
    let m = model(&f);
    let w = started(&m);
    let map = w.s.active_map().unwrap();
    assert_eq!(w.view_dims(), (w.s.maps[map].width, w.s.maps[map].height), "the minigame's map area has no cell size (M 7.1)");
}

#[test]
fn a_border_takes_cells_from_the_view() {
    let layout = "\nlayout L\n\n```\nm\n```\n\nlegend:\n- m map size 10x6\n\npanels\n- map border single\n";
    let f = files(&[("game.rlml", &format!("{SEATS}{layout}"))]);
    let m = model(&f);
    let w = started(&m);
    assert_eq!(w.view_dims(), (8, 4), "inside the border, as on the screen (M 7.1)");
}

#[test]
fn control_stays_on_the_seat_a_member_leaves() {
    let f = files(&[("game.rlml", SEATS)]);
    let m = model(&f);
    let mut c = 0;
    let mut r = seats_game(&m, 2, |w| c = second_member(w));
    let (a, b) = (r.w.s.active(1).unwrap(), r.w.s.active(2).unwrap());
    r.run(0).unwrap();
    assert!(r.input(1, &cmd("TO_SEAT_2")).unwrap());
    assert_eq!((r.w.s.active(1), r.w.s.active(2)), (Some(c), Some(b)), "seat 1's next member takes control");
    assert_eq!(r.w.s.seat_of(a), Some(2));
    // the scheduler sets `player`: whoever waits for input is controlled
    assert_eq!(r.run(0).unwrap(), Status::Input);
    let n = r.awaiting().unwrap();
    assert_eq!(r.w.s.player, r.w.s.active(n), "`player` is the awaited seat's controlled member");
}

#[test]
fn an_unjoined_seat_changes_nothing() {
    let f = files(&[("game.rlml", SEATS)]);
    let m = model(&f);
    let mut r = seats_game(&m, 1, |_| {});
    let a = r.w.s.active(1).unwrap();
    r.w.s.set_seat(a, Some(2));
    assert_eq!((r.w.s.seat_of(a), r.w.s.active(1)), (Some(1), Some(a)), "seat 2 is not joined");
    assert!(r.w.s.get(a).has("seat"));
}

#[test]
fn leaving_a_seat_keeps_the_player() {
    let f = files(&[("game.rlml", SEATS)]);
    let m = model(&f);
    let mut r = seats_game(&m, 1, |_| {});
    let a = r.w.s.active(1).unwrap();
    r.w.s.set_seat(a, None);
    assert_eq!(r.w.s.player, Some(a), "`remove(player, seat)` leaves `player` to the caller (M 6.1.10)");
    assert_eq!(r.w.s.seat_of(a), None);
}

#[test]
fn a_dead_controlled_member_does_not_hold_a_seat() {
    let f = files(&[("game.rlml", SEATS)]);
    let m = model(&f);
    let mut r = seats_game(&m, 2, |_| {});
    let (a, b) = (r.w.s.active(1).unwrap(), r.w.s.active(2).unwrap());
    let env = rlml_core::rt::eval::Env::new(rlml_core::rt::eval::BodyKind::Handler, "test", None, "test");
    r.w.destroy(b, &env).unwrap();
    r.w.s.set_seat(a, Some(2));
    assert_eq!(r.w.s.active(2), Some(a), "seat 2 had no living controlled member");
}

#[test]
fn the_view_and_the_screen_read_the_border_alike() {
    for (size, border, want) in [("10x6", "single", (8, 4)), ("10x6", "thick", (10, 6)), ("10x6", "none", (10, 6)), ("2x1", "single", (2, 1)), ("2x2", "single", (0, 0))] {
        let layout = format!("\nlayout L\n\n```\nm\n```\n\nlegend:\n- m map size {size}\n\npanels\n- map border {border}\n");
        let f = files(&[("game.rlml", &format!("{SEATS}{layout}"))]);
        let (m, _) = rlml_core::check_game(&f, "game.rlml");
        assert_eq!(World::new(&m, 7).view_dims(), want, "size {size}, border {border}: an unknown style or a tiny area draws no border");
    }
}

#[test]
fn a_controlled_member_leaving_hands_control_on() {
    let f = files(&[("game.rlml", SEATS)]);
    let m = model(&f);
    // leaving: seat 1's next member takes control and `player`
    let mut c = 0;
    let mut r = seats_game(&m, 1, |w| c = second_member(w));
    let a = r.w.s.active(1).unwrap();
    r.w.s.player = Some(a);
    r.w.s.set_seat(a, None);
    assert_eq!(r.w.s.active(1), Some(c), "one rule for leaving and moving (M 6.1.10)");
    assert_eq!(r.w.s.player, Some(a), "`set_seat` leaves `player` to the scheduler");
    // moving to a seat with a controlled member: not controlled there
    let mut r = seats_game(&m, 2, |_| {});
    let (a, b) = (r.w.s.active(1).unwrap(), r.w.s.active(2).unwrap());
    r.w.s.set_seat(a, Some(2));
    assert_eq!((r.w.s.active(1), r.w.s.active(2)), (None, Some(b)), "nobody is left on seat 1");
}

#[test]
fn removing_the_players_seat_ends_its_turns() {
    let f = files(&[("game.rlml", &SEATS.replace("command ADOPT() cost 0:", "command UNSEAT() cost 0:\n    remove(actor, seat)\n\ncommand ADOPT() cost 0:"))]);
    let m = model(&f);
    let mut r = seats_game(&m, 1, |_| {});
    let a = r.w.s.active(1).unwrap();
    r.run(0).unwrap();
    assert!(r.player(&cmd("UNSEAT")).unwrap());
    assert_eq!(r.w.s.wake_of(a), None, "no `seat` and no `ai`: no longer an actor (M 6.1.10)");
    assert_eq!(r.run(1).unwrap(), Status::NoPlayer { rounds: 1 });
}

#[test]
fn only_the_awaited_seat_gives_input() {
    let f = files(&[("game.rlml", SEATS)]);
    let m = model(&f);
    let mut r = seats_game(&m, 2, |_| {});
    let (a, b) = (r.w.s.active(1).unwrap(), r.w.s.active(2).unwrap());
    r.run(0).unwrap();
    assert!(r.input(1, &cmd("TO_SEAT_2")).unwrap(), "a free command moves `a` to seat 2");
    assert!(r.input(2, &cmd("STD.WAIT")).is_err(), "no input before `run` chooses the turn");
    assert_eq!(r.run(0).unwrap(), Status::Input);
    assert_eq!((r.awaiting(), r.w.s.player), (Some(2), Some(b)), "seat 2's controlled member, not `a`");
    assert!(r.w.s.seat_of(a) == Some(2) && r.w.s.active(2) == Some(b));
}

#[test]
fn a_swapped_out_body_gets_no_wake() {
    let f = files(&[("game.rlml", SEATS)]);
    let m = model(&f);
    let mut r = seats_game(&m, 1, |_| {});
    let a = r.w.s.active(1).unwrap();
    let at = r.w.s.maps[0].label("P3")[0];
    let golem = r.w.spawn("HERO", &[], Place::Map(Pos { map: 0, x: at.0, y: at.1 })).unwrap();
    r.w.control(golem, None);
    assert_eq!((r.w.s.seat_of(a), r.w.s.active(1), r.w.s.player), (None, Some(golem), Some(golem)), "a body swap (RS 10.1)");
    r.w.s.schedule(golem, 0);
    assert_eq!(r.run(0).unwrap(), Status::Input);
    assert!(r.input(1, &cmd("STD.WAIT")).unwrap());
    r.run(0).unwrap();
    assert_eq!(wakes(&r.w, a), 0, "the old body is no actor: no `wake` (M 6.1.10)");
}


#[test]
fn the_group_example() {
    let f = ryhma();
    let m = model(&f);
    let mut r = Runner::new(started(&m));
    let robin = kinds(&r.w, "ROBIN")[0];
    let heroes = kinds(&r.w, "HERO");
    assert_eq!(r.w.s.seats.len(), 2, "`seats 2..2`");
    assert_eq!(r.w.s.seat_members(1), [robin, heroes[0]], "Robin is a data member of seat 1 (D 5.3)");
    assert_eq!(r.w.s.seat_members(2), [heroes[1]]);
    assert_eq!(r.w.view_dims(), (20, 8), "`- m map size 20x8`");
    assert!(r.w.s.leash);
    // seat 1 switches to Robin: free, and seat 2 plays next
    assert_eq!(r.run(0).unwrap(), Status::Input);
    assert_eq!(r.awaiting(), Some(1));
    assert!(r.player(&cmd("STD.SWITCH")).unwrap());
    assert_eq!(r.w.s.active(1), Some(robin));
    assert_eq!(r.run(0).unwrap(), Status::Input);
    assert_eq!(r.awaiting(), Some(2));
}

#[test]
fn golden_the_group_example() {
    golden_of(ryhma(), "ryhma", "l d tab l . q . q");
}

#[test]
fn friends_swap_places_when_they_bump() {
    let m = model(&ryhma());
    let mut r = Runner::new(started(&m));
    let robin = kinds(&r.w, "ROBIN")[0];
    r.run(0).unwrap();
    let p = r.w.s.player.unwrap();
    let (px, py) = at(&r.w, p);
    // Robin stands east of the player
    r.w.s.get_mut(robin).place = Place::Map(Pos { map: 0, x: px + 1, y: py });
    assert!(r.player(&cmd("STD.MOVE.EAST")).unwrap(), "the bump swaps places (STD 5)");
    assert_eq!((at(&r.w, p), at(&r.w, robin)), ((px + 1, py), (px, py)), "the mover's own cell does not block the friend");
}

#[test]
fn seats_have_their_own_colors() {
    let m = model(&ryhma());
    let mut w = started(&m);
    let heroes = kinds(&w, "HERO");
    let looks: Vec<_> = heroes.iter().map(|&e| {
        let (x, y) = at(&w, e);
        w.render_cells(0)[y as usize][x as usize].fg
    }).collect();
    assert_eq!(looks, [Some([0xf0, 0xd0, 0x40]), Some([0x60, 0xa0, 0xff])], "`GLYPH.HERO.SEAT1` and `.SEAT2` (M 7.2)");
    let robin = kinds(&w, "ROBIN")[0];
    let (x, y) = at(&w, robin);
    assert_eq!(w.render_cells(0)[y as usize][x as usize].fg, Some([0x60, 0xc0, 0x60]), "no seat row: the tile's own color");
}

#[test]
fn in_the_group_example_a_wall_takes_the_turn() {
    let m = model(&ryhma());
    let mut r = Runner::new(started(&m));
    r.run(0).unwrap();
    let p = r.w.s.player.unwrap();
    let (_, y) = at(&r.w, p);
    // the cave's west wall is at x 0: stand next to it and walk into it
    r.w.s.get_mut(p).place = Place::Map(Pos { map: 0, x: 1, y });
    assert!(r.player(&cmd("WALK.WEST")).unwrap(), "the bump succeeds");
    assert_eq!(at(&r.w, p), (1, y), "no step");
    assert_eq!(r.w.s.wake_of(p).unwrap().at, 120, "it cost the step's 120");
    assert!(log_texts(&r.w).iter().any(|t| t.contains("kolauttaa seinään")), "{:?}", log_texts(&r.w));
}

#[test]
fn seats_have_their_own_names() {
    let m = model(&ryhma());
    let w = started(&m);
    let heroes = kinds(&w, "HERO");
    let robin = kinds(&w, "ROBIN")[0];
    let name = |e: usize| w.text("fi", "STD.MESSAGE.COMBAT.DIED", &[("VICTIM".into(), Value::Entity(e))]);
    assert_eq!(name(heroes[0]), "Aino kuolee.", "`HERO.SEAT1.NAME` (D 10)");
    assert_eq!(name(heroes[1]), "Eero kuolee.");
    assert_eq!(name(robin), "Robin kuolee.", "a child's own name wins over its parent's seat name");
    let rat = kinds(&w, "RAT")[0];
    assert_eq!(name(rat), "rotta kuolee.", "no seat: the kind's name");
}

#[test]
fn players_going_the_same_way_do_not_dance() {
    let m = model(&ryhma());
    let mut r = Runner::new(started(&m));
    let robin = kinds(&r.w, "ROBIN")[0];
    r.w.s.get_mut(robin).place = Place::Map(Pos { map: 0, x: 1, y: 6 });
    let (a, b) = (r.w.s.active(1).unwrap(), r.w.s.active(2).unwrap());
    // seat 1's hero right behind seat 2's on the open row 4
    r.w.s.get_mut(a).place = Place::Map(Pos { map: 0, x: 3, y: 4 });
    r.w.s.get_mut(b).place = Place::Map(Pos { map: 0, x: 4, y: 4 });
    assert!(r.w.s.controlled(a) && r.w.s.controlled(b) && !r.w.s.controlled(robin), "`controlled` (M 9.3)");
    r.run(0).unwrap();
    assert_eq!(r.awaiting(), Some(1));
    assert!(!r.input(1, &cmd("STD.MOVE.EAST")).unwrap(), "no swap with a member a person plays (STD 5)");
    assert_eq!((at(&r.w, a), at(&r.w, b)), ((3, 4), (4, 4)), "nobody moved");
    r.run(0).unwrap();
    assert_eq!((r.awaiting(), r.w.s.game.time), (Some(1), 0), "seat 1 again: no time went by");
    assert!(r.input(1, &cmd("STD.WAIT")).unwrap());
    r.run(0).unwrap();
    assert!(r.input(2, &cmd("STD.MOVE.EAST")).unwrap());
    r.run(0).unwrap();
    assert!(r.input(1, &cmd("STD.MOVE.EAST")).unwrap(), "the way is free now");
    assert_eq!((at(&r.w, a), at(&r.w, b)), ((4, 4), (5, 4)), "both went east, one behind the other");
}
