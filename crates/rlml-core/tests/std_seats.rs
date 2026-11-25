//! The standard library with one and three seats (STD 3, 5, 6, 7, 8 and
//! moninpeli vaihe 5): every rule runs the same tests with each count.

use std::path::Path;

use rlml_core::load::MemFiles;
use rlml_core::model::Model;
use rlml_core::rt::eval::{BodyKind, Env};
use rlml_core::rt::run::{Runner, Status};
use rlml_core::rt::state::Place;
use rlml_core::rt::value::{CommandValue, Pos, Value};
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

/// A hall with stairs down to a cellar whose arrival cell is a single
/// cell, so a group arriving there spreads out (GROUP_ARRIVAL).
const GAME: &str = r#"game T
- start HALL
- seats 1..3

include lib/std.rlml

settings
- style ascii
- language fi

entity DOWN
- EXIT to CELLAR

map HALL

```
##########
#........#
#.S......#
#........#
#......>.#
##########
```

legend:
- # WALL
- . FLOOR
- S FLOOR as START
- > FLOOR with DOWN

map CELLAR

```
#######
#.....#
#..S..#
#.....#
#######
```

legend:
- # WALL
- . FLOOR
- S FLOOR as START
"#;

fn model() -> Model {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut f = MemFiles::default();
    read_tree(&repo, &repo.join("lib"), &mut f.engine);
    f.game.insert("game.rlml".into(), GAME.into());
    let (m, r) = rlml_core::check_game(&f, "game.rlml");
    assert!(!r.has_errors(), "{:#?}", r.errors().map(|d| d.to_string()).collect::<Vec<_>>());
    m
}

fn start(m: &Model, seats: u8) -> Runner<'_> {
    let mut w = World::new_game_seats(m, 7, Some(seats)).unwrap();
    w.begin().unwrap();
    let mut r = Runner::new(w);
    assert_eq!(r.run(0).unwrap(), Status::Input);
    r
}

fn members(r: &Runner) -> Vec<usize> {
    r.w.s.players()
}

fn cell(r: &Runner, e: usize) -> (String, i32, i32) {
    let p = r.w.s.at(e).unwrap();
    (r.w.s.maps[p.map].name.clone(), p.x, p.y)
}

fn put(r: &mut Runner, e: usize, (x, y): (i32, i32)) {
    let map = r.w.s.at(e).unwrap().map;
    r.w.s.get_mut(e).place = Place::Map(Pos { map, x, y });
}

/// The awaited seat's command, then the world to the next turn.
fn input(r: &mut Runner, name: &str) -> bool {
    let n = r.awaiting().unwrap();
    let ok = r.input(n, &CommandValue { name: name.into(), args: vec![] }).unwrap();
    r.run(0).unwrap();
    ok
}

fn each_count(test: impl Fn(&Model, u8)) {
    let m = model();
    for n in [1, 3] {
        test(&m, n);
    }
}

#[test]
fn spawn_player_gives_every_seat_a_member_on_its_own_cell() {
    each_count(|m, n| {
        let r = start(m, n);
        let all = members(&r);
        assert_eq!(all.len(), n as usize, "{n} seats");
        for (i, &e) in all.iter().enumerate() {
            assert_eq!(r.w.s.seat_of(e), Some(i as u8 + 1));
            assert_eq!(r.w.s.active(i as u8 + 1), Some(e));
        }
        let mut cells: Vec<_> = all.iter().map(|&e| cell(&r, e)).collect();
        cells.dedup();
        assert_eq!(cells.len(), n as usize, "{n} seats: no two members share a cell");
        assert_eq!(cell(&r, all[0]), ("HALL".into(), 2, 2), "seat 1 at START");
    });
}

#[test]
fn the_group_takes_the_stairs_together_and_spreads_out() {
    each_count(|m, n| {
        let mut r = start(m, n);
        // the whole group next to the stairs (7, 4)
        let spots = [(7, 4), (6, 4), (6, 3)];
        for (i, &e) in members(&r).iter().enumerate() {
            put(&mut r, e, spots[i]);
        }
        let first = r.awaiting().unwrap();
        let mover = r.w.s.active(first).unwrap();
        // the awaited member must stand on the stairs
        let others: Vec<usize> = members(&r).into_iter().filter(|&e| e != mover).collect();
        let on = members(&r).into_iter().find(|&e| cell(&r, e).1 == 7).unwrap();
        if on != mover {
            let p = r.w.s.at(mover).unwrap();
            put(&mut r, on, (p.x, p.y));
            put(&mut r, mover, (7, 4));
        }
        assert!(input(&mut r, "STD.TAKE_EXIT"), "{n} seats: everyone is at the stairs");
        let cells: Vec<_> = members(&r).iter().map(|&e| cell(&r, e)).collect();
        assert!(cells.iter().all(|c| c.0 == "CELLAR"), "{n} seats: the group went down together: {cells:?}");
        let mut distinct = cells.clone();
        distinct.sort();
        distinct.dedup();
        assert_eq!(distinct.len(), n as usize, "GROUP_ARRIVAL spread them: {cells:?}");
        assert!(others.iter().all(|&e| r.w.s.seat_of(e).is_some()), "still members");
    });
}

#[test]
fn hits_are_told_to_the_one_and_to_the_group() {
    each_count(|m, n| {
        let mut r = start(m, n);
        let target = members(&r)[0];
        let env = Env::new(BodyKind::Handler, "test", None, "test");
        let mut env = env;
        env.map = r.w.s.at(target).map(|p| p.map);
        let before = r.w.log.len();
        r.w.call_function("STD.HURT", vec![(None, Value::Entity(target)), (None, Value::Number(1))], &mut env).unwrap();
        let keys: Vec<&str> = r.w.log[before..].iter().map(|l| l.key.as_str()).collect();
        // alone the player is "you"; in a group the message names who was hit
        let want = if n == 1 { "STD.MESSAGE.COMBAT.HURT" } else { "STD.MESSAGE.COMBAT.HIT" };
        assert_eq!(keys, [want], "{n} seats (STD 8, SOLO)");
    });
}

#[test]
fn a_seat_that_leaves_follows_a_member_a_person_drives() {
    let m = model();
    let mut r = start(&m, 3);
    let third = r.w.s.active(3).unwrap();
    assert!(r.leave_seat(3).unwrap());
    r.run(0).unwrap();
    assert!(r.w.s.get(third).has("STD.FOLLOW") && r.w.s.get(third).has("ai"), "PART_PLAYER (STD 7.2)");
    let mut env = Env::new(BodyKind::Handler, "test", None, "test");
    env.map = r.w.s.at(third).map(|p| p.map);
    let leader = r.w.call_function("STD.LEADER_OF", vec![(None, Value::Entity(third))], &mut env).unwrap();
    let leader = leader.as_entity().expect("a leader");
    assert!(r.w.s.seat_of(leader).is_some_and(|s| s <= 2), "LEADER_OF is a member of a seat still playing (STD 6.1)");
    // the seats left take their turns, and the follower walks with them
    for _ in 0..6 {
        input(&mut r, "STD.WAIT");
    }
    assert_eq!(r.w.s.seats.len(), 2);
}

#[test]
fn three_seats_take_their_turns_in_order() {
    let m = model();
    let mut r = start(&m, 3);
    let mut order = vec![];
    for _ in 0..6 {
        order.push(r.awaiting().unwrap());
        input(&mut r, "STD.WAIT");
    }
    assert_eq!(order, [1, 2, 3, 1, 2, 3], "every seat once a round, by number (M 6.1.10)");
}

#[test]
fn the_group_gathers_before_it_takes_the_stairs() {
    // with one member away the stairs wait for the group, without a turn
    // spent (STD 5; the dialogue EXIT.LEAVE_OTHERS waits for scenes)
    let m = model();
    let mut r = start(&m, 3);
    let mover = r.w.s.active(r.awaiting().unwrap()).unwrap();
    put(&mut r, mover, (7, 4));
    for e in members(&r) {
        if e != mover {
            put(&mut r, e, (1, 1));
        }
    }
    let (n, time) = (r.awaiting().unwrap(), r.w.s.game.time);
    assert!(!r.input(n, &CommandValue { name: "STD.TAKE_EXIT".into(), args: vec![] }).unwrap());
    assert_eq!(r.w.s.game.time, time, "no time spent");
    assert_eq!(r.w.log.last().map(|l| l.key.as_str()), Some("STD.MESSAGE.EXIT.OTHERS_AWAY"));
    assert_eq!(r.w.text("fi", "STD.MESSAGE.EXIT.OTHERS_AWAY", &[]), "Muut jäävät jälkeen. Kokoontukaa ensin uloskäynnille.");
    assert!(members(&r).iter().all(|&e| cell(&r, e).0 == "HALL"), "nobody went");
}
