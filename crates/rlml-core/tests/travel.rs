//! Travel by click, following and orders (M 8.1, M 8.2).

use std::path::Path;

use rlml_core::load::MemFiles;
use rlml_core::model::Model;
use rlml_core::rt::run::{Runner, Status};
use rlml_core::rt::state::Place;
use rlml_core::rt::travel::{Goal, OrderGoal, Stop};
use rlml_core::rt::value::{CommandValue, Pos};
use rlml_core::rt::world::World;

fn read_tree(root: &Path, dir: &Path, out: &mut std::collections::BTreeMap<String, String>) {
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        // hidden files, such as Finder's `.DS_Store`, are not game data
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

fn files(game: &str) -> MemFiles {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut f = MemFiles::default();
    read_tree(&repo, &repo.join("lib"), &mut f.engine);
    f.game.insert("game.rlml".into(), game.into());
    f
}

fn model(f: &MemFiles) -> Model {
    let (m, r) = rlml_core::check_game(f, "game.rlml");
    assert!(!r.has_errors(), "{:#?}", r.errors().map(|d| d.to_string()).collect::<Vec<_>>());
    m
}

/// Two seats in a corridor; a still rat at the far end, out of sight.
const CORRIDOR: &str = r#"game T
- start HALL
- seats 1..2
- walk STD.MOVE
- rest STD.WAIT
- leader STD.LEADER_OF
FIELDS

include lib/std.rlml

settings
- style ascii
- language fi

tiles
- WALL blocks move sight sound
- default blocks none lit

entity HERO
- blocks move
- sight radius 3

entity RAT
- blocks move
- ai behavior STILL

map HALL

```
####################
#A.................#
#B...............R.#
####################
```

legend:
- # WALL
- A FLOOR as START
- B FLOOR as P2
- R FLOOR as DEN
- . FLOOR

```roguescript
override on begin() as STD.SPAWN_PLAYER:
    control(spawn(HERO, at: label(START)), seat: 1)
    control(spawn(HERO, at: label(P2)), seat: 2)
    spawn(RAT, at: label(DEN))

ai STILL:
    return STD.WAIT()

query NEVER_PAST_6(who: entity, next: pos) -> bool:
    return next.x > 6
```
"#;

fn game(fields: &str) -> MemFiles {
    files(&CORRIDOR.replace("FIELDS", fields))
}

fn start(m: &Model) -> Runner<'_> {
    let mut w = World::new_game_seats(m, 7, Some(2)).unwrap();
    w.begin().unwrap();
    let mut r = Runner::new(w);
    assert_eq!(r.run(0).unwrap(), Status::Input);
    r
}

/// The corridor as if explored: every cell remembered (M 7.2), so routes
/// may pass all of it.
fn explored(r: &mut Runner) {
    let (w, h) = (r.w.s.maps[0].width, r.w.s.maps[0].height);
    r.w.s.maps[0].remembered = (0..h).flat_map(|y| (0..w).map(move |x| (x, y))).collect();
}

fn wait() -> CommandValue {
    CommandValue { name: "STD.WAIT".into(), args: vec![] }
}

fn cell(x: i32, y: i32) -> Pos {
    Pos { map: 0, x, y }
}

fn at(r: &Runner, n: u8) -> (i32, i32) {
    let p = r.w.s.at(r.w.s.active(n).unwrap()).unwrap();
    (p.x, p.y)
}

/// Runs turns: a travelling seat steps, the others wait. Gives the stops
/// in order, at most `turns` turns.
fn drive(r: &mut Runner, turns: usize) -> Vec<(u8, Stop)> {
    let mut stops = vec![];
    for _ in 0..turns {
        assert_eq!(r.run(0).unwrap(), Status::Input);
        let n = r.awaiting().unwrap();
        if r.travelling(n) {
            if let Some(s) = r.step_travel().unwrap() {
                stops.push((n, s));
            }
        } else {
            r.input(n, &wait()).unwrap();
        }
    }
    stops
}

#[test]
fn travel_walks_until_an_actor_comes_into_view() {
    let m = model(&game(""));
    let mut r = start(&m);
    explored(&mut r);
    assert!(r.travel_to(1, cell(18, 1)).unwrap());
    let stops = drive(&mut r, 40);
    assert_eq!(stops.first().map(|s| s.1), Some(Stop::NewActor), "{stops:?}");
    let (x, _) = at(&r, 1);
    assert!((10..17).contains(&x), "seat 1 walked east until the rat came into view: x {x}");
    assert!(!r.travelling(1));
}

#[test]
fn travel_arrives() {
    let m = model(&game(""));
    let mut r = start(&m);
    explored(&mut r);
    assert!(r.travel_to(1, cell(5, 1)).unwrap());
    let stops = drive(&mut r, 20);
    assert_eq!(stops, [(1, Stop::Arrived)]);
    assert_eq!(at(&r, 1), (5, 1));
}

#[test]
fn travel_needs_walk_and_a_route_through_known_cells() {
    let src = CORRIDOR.replace("- walk STD.MOVE\n", "").replace("FIELDS", "");
    let m = model(&files(&src));
    let mut r = start(&m);
    assert!(!r.travel_to(1, cell(3, 1)).unwrap(), "no `walk`: no travel (M 8.1)");
    let m = model(&game(""));
    let mut r = start(&m);
    assert!(!r.travel_to(1, cell(1, 1)).unwrap(), "the own cell");
    assert!(!r.travel_to(1, cell(17, 1)).unwrap(), "never seen: no route");
    assert!(r.travel_to(2, cell(3, 2)).is_err(), "seat 2 has no turn");
}

#[test]
fn walk_stop_stops_before_the_step() {
    let m = model(&game("- walk_stop NEVER_PAST_6"));
    let mut r = start(&m);
    explored(&mut r);
    assert!(r.travel_to(1, cell(9, 1)).unwrap());
    let stops = drive(&mut r, 30);
    assert_eq!(stops, [(1, Stop::Game)]);
    assert_eq!(at(&r, 1), (6, 1));
}

#[test]
fn a_follower_keeps_next_to_its_leader() {
    let m = model(&game(""));
    let mut r = start(&m);
    // seat 1 waits its first turn; seat 2 starts following
    r.input(1, &wait()).unwrap();
    assert_eq!(r.run(0).unwrap(), Status::Input);
    assert_eq!(r.awaiting(), Some(2));
    assert!(r.follow(2, None).unwrap(), "the leader from `leader`");
    // seat 1 walks east five times; seat 2's turns step after it
    for _ in 0..5 {
        assert_eq!(r.run(0).unwrap(), Status::Input);
        match r.awaiting().unwrap() {
            1 => {
                r.input(1, &CommandValue { name: "STD.MOVE.EAST".into(), args: vec![] }).unwrap();
            }
            _ => assert_eq!(r.step_travel().unwrap(), None),
        }
    }
    let (a, b) = (at(&r, 1), at(&r, 2));
    assert!((a.0 - b.0).abs() <= 1 && (a.1 - b.1).abs() <= 1, "next to the leader: {a:?} {b:?}");
    assert!(a.0 >= 3, "the leader went east: {a:?}");
    assert!(r.travelling(2));
    r.stop_travel(2);
    assert!(!r.travelling(2), "a press stops it");
}

#[test]
fn the_leader_is_a_member_a_person_drives() {
    // seat 1's companion stands as near to seat 2 as seat 1's hero does
    let src = CORRIDOR
        .replace("FIELDS", "- party switch")
        .replace("#B...............R.#", "#BP..............R.#")
        .replace("- B FLOOR as P2", "- B FLOOR as P2\n- P FLOOR with PAL")
        .replace("entity RAT", "entity PAL extends HERO\n- seat 1\n- ai behavior STILL\n\nentity RAT");
    let m = model(&files(&src));
    let mut r = start(&m);
    r.input(1, &wait()).unwrap();
    assert_eq!(r.run(0).unwrap(), Status::Input);
    assert_eq!(r.awaiting(), Some(2));
    assert!(r.follow(2, None).unwrap());
    let hero = r.w.s.active(1).unwrap();
    assert_eq!(r.travels[&2].goal, Goal::Follow(hero), "seat 1's hero, not its companion (STD 6.1)");
}

#[test]
fn following_in_a_ring_is_refused() {
    let m = model(&game(""));
    let mut r = start(&m);
    let (a, b) = (r.w.s.active(1).unwrap(), r.w.s.active(2).unwrap());
    assert!(r.follow(1, Some(b)).unwrap());
    // seat 1's first step: it is next to b already, so it rests
    assert_eq!(r.step_travel().unwrap(), None);
    assert_eq!(r.run(0).unwrap(), Status::Input);
    assert_eq!(r.awaiting(), Some(2));
    assert!(!r.follow(2, Some(a)).unwrap(), "a ring: nobody would lead (M 8.1)");
    assert!(!r.follow(2, Some(b)).unwrap(), "not oneself");
}

#[test]
fn a_stale_order_ends_before_its_second_step() {
    let m = model(&game(""));
    let mut r = start(&m);
    explored(&mut r);
    // seat 2 orders a travel on seat 1's turn
    r.give_order(2, OrderGoal::Cell(cell(5, 2)));
    assert!(r.travelling(2));
    // then the rat comes next to seat 2, before seat 2's turn
    let rat = r.w.s.entities_on(0).into_iter().find(|&e| !r.w.s.is_member(e) && r.w.s.is_actor(e)).unwrap();
    r.w.s.get_mut(rat).place = Place::Map(cell(3, 1));
    r.input(1, &wait()).unwrap();
    assert_eq!(r.run(0).unwrap(), Status::Input);
    assert_eq!(r.awaiting(), Some(2));
    assert_eq!(r.step_travel().unwrap(), None, "the first step always goes (M 8.1)");
    assert_eq!(at(&r, 2), (2, 2));
    let stops = drive(&mut r, 2);
    assert_eq!(stops, [(2, Stop::Stale)], "cancelled before the second step");
    assert_eq!(at(&r, 2), (2, 2));
}

#[test]
fn an_order_without_a_new_actor_runs_to_the_end() {
    let m = model(&game(""));
    let mut r = start(&m);
    explored(&mut r);
    r.give_order(2, OrderGoal::Cell(cell(4, 2)));
    let stops = drive(&mut r, 20);
    assert_eq!(stops, [(2, Stop::Arrived)]);
    assert_eq!(at(&r, 2), (4, 2));
}

#[test]
fn follow_ai_lets_the_ai_choose_the_steps() {
    let m = model(&game("- follow_ai STD.FOLLOWER"));
    let mut r = start(&m);
    r.input(1, &wait()).unwrap();
    assert_eq!(r.run(0).unwrap(), Status::Input);
    assert!(r.follow(2, None).unwrap());
    for _ in 0..8 {
        assert_eq!(r.run(0).unwrap(), Status::Input);
        match r.awaiting().unwrap() {
            1 => {
                r.input(1, &CommandValue { name: "STD.MOVE.EAST".into(), args: vec![] }).unwrap();
            }
            _ => assert_eq!(r.step_travel().unwrap(), None),
        }
    }
    let (a, b) = (at(&r, 1), at(&r, 2));
    assert!((a.0 - b.0).abs() <= 2, "FOLLOWER keeps up, a step behind after the leader's move: {a:?} {b:?}");
    assert!(b.0 > 2, "the follower went east: {b:?}");
}
