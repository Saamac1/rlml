//! Atlases (D 8.12): sheets, what they inherit, missing sheets and the
//! load-time checks. The atlas is the map generator's test atlas: 6×3
//! sheets in a 3×2 grid, filled with water.

use std::path::Path;

use rlml_core::check_game;
use rlml_core::error::Report;
use rlml_core::load::MemFiles;
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

const WORLD: &str = "atlas MAAILMA sheet 6x3 grid 3x2 fill WATER

legend:
- ~ WATER
- . FLOOR

vars:
- OUTDOOR true
- DEPTH 0

areas:
- 4,1 size 4x2 as NORSUSAARI

## A01

map MAAILMA.A01

```
~~~~~~
~~..~~
~~~~~.
```

at:
- 2,1 as START

areas:
- 2,1 size 2x1 as LAHDE

## B01

map MAAILMA.B01

```
~~~~~~
......
```

legend:
- x FLOOR with RAT

vars:
- DEPTH 1
";

/// The minigame starting on sheet A01 of the atlas.
fn game(world: &str) -> MemFiles {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut files = MemFiles::default();
    let game = repo.join("esimerkit/mini");
    read_tree(&game, &game, &mut files.game);
    read_tree(&repo, &repo.join("lib"), &mut files.engine);
    files.game.insert("maps/world.rlml".into(), world.into());
    let src = files.game.get_mut("game.rlml").unwrap();
    *src = src.replacen("- start CAVE_A", "- start MAAILMA.A01", 1).replacen("- at SPAWN", "- at START", 1);
    src.push_str("\ntiles\n- WATER blocks move\n");
    files
}

fn messages(r: &Report) -> Vec<String> {
    r.0.iter().map(|d| d.to_string()).collect()
}

fn errors(r: &Report) -> Vec<String> {
    r.errors().map(|d| d.to_string()).collect()
}

fn expect(r: &Report, part: &str) {
    let all = messages(r);
    assert!(all.iter().any(|m| m.contains(part)), "expected a message with `{part}`, got {all:#?}");
}

fn clean(f: &MemFiles) -> rlml_core::model::Model {
    let (m, r) = check_game(f, "game.rlml");
    assert!(errors(&r).is_empty(), "{:#?}", messages(&r));
    m
}

#[test]
fn a_sheet_inherits_the_atlas() {
    let f = game(WORLD);
    let m = clean(&f);
    let mut w = World::new_game(&m, 7).unwrap();
    let a = &w.s.maps[0];
    assert_eq!((a.name.as_str(), a.width, a.height), ("MAAILMA.A01", 6, 3));
    assert_eq!((a.tile(0, 0, 0), a.tile(0, 2, 1), a.tile(0, 5, 2)), (Some("WATER"), Some("FLOOR"), Some("FLOOR")), "the atlas's legend");
    assert_eq!(a.label("START"), [(2, 1)]);
    let vars: Vec<String> = a.vars.iter().map(|(n, v)| format!("{n}={v}")).collect();
    assert_eq!(vars, ["OUTDOOR=true", "DEPTH=0"]);
    // the island's part on A01, in the sheet's coordinates, and the sheet's own area
    let island = a.areas.get("NORSUSAARI").unwrap();
    assert_eq!(island.cells, [(4, 1), (5, 1), (4, 2), (5, 2)]);
    assert_eq!(a.areas.deepest(2, 1).unwrap().name, "LAHDE");

    let b = w.load_map("MAAILMA.B01").unwrap();
    let b = &w.s.maps[b];
    assert_eq!(b.areas.get("NORSUSAARI").unwrap().cells, [(0, 1), (1, 1), (0, 2), (1, 2)]);
    assert!(b.areas.get("LAHDE").is_none(), "a sheet's own area stays on it");
    assert_eq!(b.tile(0, 0, 2), None, "a short drawing is filled with empty cells like other maps");
    let vars: Vec<String> = b.vars.iter().map(|(n, v)| format!("{n}={v}")).collect();
    assert_eq!(vars, ["OUTDOOR=true", "DEPTH=1"], "the sheet's own row replaces the atlas's by name");
}

#[test]
fn a_missing_sheet_is_the_fill() {
    let m = clean(&game(WORLD));
    let mut w = World::new_game(&m, 7).unwrap();
    let c = w.load_map("MAAILMA.C02").unwrap();
    let c = &w.s.maps[c];
    assert_eq!((c.width, c.height), (6, 3));
    assert!((0..3).all(|y| (0..6).all(|x| c.tile(0, x, y) == Some("WATER"))));
    assert_eq!(c.vars.len(), 2, "the atlas's vars");
    assert!(c.areas.is_empty(), "the island is not on C02");
    assert!(w.load_map("MAAILMA.D01").unwrap_err().contains("outside the 3x2 grid"));
    // without `fill` a missing sheet is empty
    let m = clean(&game(&WORLD.replace(" fill WATER", "")));
    let mut w = World::new_game(&m, 7).unwrap();
    let c = w.load_map("MAAILMA.A02").unwrap();
    assert_eq!(w.s.maps[c].tile(0, 0, 0), None);
}

#[test]
fn a_game_can_start_on_a_missing_sheet() {
    let mut f = game(WORLD);
    let src = f.game.get_mut("game.rlml").unwrap();
    *src = src.replacen("- start MAAILMA.A01", "- start MAAILMA.C02", 1).replacen("- at START", "- at 3,1", 1);
    let m = clean(&f);
    let w = World::new_game(&m, 7).unwrap();
    assert_eq!(w.s.maps[0].name, "MAAILMA.C02");
    assert_eq!(w.s.game.at.len(), 1);
}

#[test]
fn an_atlas_is_not_a_map() {
    let mut f = game(WORLD);
    let src = f.game.get_mut("game.rlml").unwrap();
    *src = src.replacen("- start MAAILMA.A01", "- start MAAILMA", 1);
    let (_, r) = check_game(&f, "game.rlml");
    expect(&r, "an atlas is not a map; start on one of its sheets, such as `MAAILMA.A01`");
}

#[test]
fn header_errors() {
    let (_, r) = check_game(&game(&WORLD.replace("sheet 6x3 grid 3x2", "sheet 6x3")), "game.rlml");
    expect(&r, "an atlas is `atlas NAME sheet WxH grid CxR [fill TILE]`");
    let (_, r) = check_game(&game(&WORLD.replace("grid 3x2", "grid 0x2")), "game.rlml");
    expect(&r, "`grid 0x2` is not a size");
    let (_, r) = check_game(&game(&WORLD.replace("fill WATER", "fill BOG.1..2")), "game.rlml");
    expect(&r, "`fill BOG.1..2`: a missing sheet is one tile, not a range");
    let (_, r) = check_game(&game(&WORLD.replace("vars:\n- OUTDOOR", "at:\n- 1,1 as X\n\nvars:\n- OUTDOOR")), "game.rlml");
    expect(&r, "an atlas has no `at:` rows");
}

#[test]
fn sheet_errors() {
    let (_, r) = check_game(&game(&format!("{WORLD}\nmap MAAILMA.D01\n\n```\n~\n```\n")), "game.rlml");
    expect(&r, "sheet `MAAILMA.D01` is outside the 3x2 grid of atlas `MAAILMA`");
    let (_, r) = check_game(&game(&format!("{WORLD}\nmap MAAILMA.A1\n\n```\n~\n```\n")), "game.rlml");
    expect(&r, "sheet `MAAILMA.A1` is written `MAAILMA.A01`");
    let (_, r) = check_game(&game(&format!("{WORLD}\nmap MAAILMA.B01\n\n```\n~\n```\n")), "game.rlml");
    expect(&r, "sheet `MAAILMA.B01` is written twice");
    let (_, r) = check_game(&game(&WORLD.replace("map MAAILMA.B01", "map MAAILMA.B01 size 6x4")), "game.rlml");
    expect(&r, "sheet `MAAILMA.B01` is `size 6x4`, but the sheets of atlas `MAAILMA` are 6x3");
    let (_, r) = check_game(&game(&WORLD.replace("map MAAILMA.B01", "map MAAILMA.B01 wrap")), "game.rlml");
    expect(&r, "a sheet cannot `wrap`");
    let (_, r) = check_game(&game(&WORLD.replace("~~~~~~\n......\n", "~~~~~~~\n......\n~~~~~~\n~~~~~~\n")), "game.rlml");
    expect(&r, "drawing row is 7 wide; the size is 6x3");
    expect(&r, "drawing has 4 rows; the size is 6x3");
    let (_, r) = check_game(&game(&format!("{WORLD}\nmap MAAILMA.C01 extends CAVE_A\n")), "game.rlml");
    expect(&r, "a sheet takes its atlas's legend, vars and areas; it cannot extend a map");
}

#[test]
fn area_errors() {
    let (_, r) = check_game(&game(&WORLD.replace("- 4,1 size 4x2 as NORSUSAARI", "- 16,1 size 4x2 as NORSUSAARI")), "game.rlml");
    expect(&r, "area `NORSUSAARI`: 16,1 size 4x2 is not inside the 18x6 map");
    let (_, r) = check_game(&game(&WORLD.replace("- 2,1 size 2x1 as LAHDE", "- 2,1 size 2x1 as NORSUSAARI")), "game.rlml");
    expect(&r, "area `NORSUSAARI` of sheet `MAAILMA.A01` has the name of an area of atlas `MAAILMA`");
    let (_, r) = check_game(&game(&WORLD.replace("- 2,1 as START", "- 2,1 as NORSUSAARI")), "game.rlml");
    expect(&r, "`NORSUSAARI` is both a label of sheet `MAAILMA.A01` and an area of its atlas");
    // an atlas area named like a sheet: `MAAILMA.B01.NAME` would read two ways
    let (_, r) = check_game(&game(&WORLD.replace("as NORSUSAARI", "as B01")), "game.rlml");
    expect(&r, "`MAAILMA.B01` is the map at");
}

#[test]
fn a_radius_larger_than_a_sheet_warns() {
    let mut f = game(WORLD);
    f.game.get_mut("maps/world.rlml").unwrap().push_str("\nentity OWL\n- sight radius 7\n");
    let (_, r) = check_game(&f, "game.rlml");
    expect(&r, "warning: `sight` radius 7 of `OWL` is larger than the 3-cell sheets of atlas `MAAILMA`");
}

// ---- edge crossing (M 6.10) ----

use rlml_core::rt::state::Place;
use rlml_core::rt::value::{CommandValue, Pos};

fn started(m: &rlml_core::model::Model) -> World<'_> {
    let mut w = World::new_game(m, 7).unwrap();
    w.begin().unwrap();
    w
}

fn step(w: &mut World, dir: &str) {
    let p = w.s.player.unwrap();
    w.start_command(p, &CommandValue { name: format!("STD.MOVE.{dir}"), args: vec![] }).unwrap();
    w.end_action().unwrap();
    w.apply_switch();
}

/// The player's sheet and cell.
fn where_(w: &World) -> (String, i32, i32) {
    let p = w.s.at(w.s.player.unwrap()).unwrap();
    (w.s.maps[p.map].name.clone(), p.x, p.y)
}

fn put(w: &mut World, sheet: &str, x: i32, y: i32) {
    let map = w.s.map_id(sheet).unwrap();
    let p = w.s.player.unwrap();
    w.s.get_mut(p).place = Place::Map(Pos { map, x, y });
}

fn loaded(w: &World) -> Vec<String> {
    let mut v: Vec<String> = w.s.maps.iter().map(|m| m.name.clone()).collect();
    v.sort();
    v
}

#[test]
fn the_nearby_region_is_active() {
    let m = clean(&game(WORLD));
    let w = started(&m);
    assert_eq!(loaded(&w), ["MAAILMA.A01", "MAAILMA.A02", "MAAILMA.B01", "MAAILMA.B02"]);
    assert!(w.s.maps.iter().all(|m| m.frozen_at.is_none()), "the sheet and its neighbours run together");
    let mut active: Vec<String> = w.s.active_maps().iter().map(|&m| w.s.maps[m].name.clone()).collect();
    active.sort();
    assert_eq!(active, loaded(&w));
}

/// The atlas with floor on row 2 of every sheet in the first row of sheets.
fn corridor() -> String {
    WORLD.replace("~~~~~.\n```", "~~~~..\n```").replace("~~~~~~\n......\n```", "~~~~~~\n......\n......\n```") + "\nmap MAAILMA.C01\n\n```\n~~~~~~\n~~~~~~\n......\n```\n"
}

#[test]
fn walking_east_crosses_to_the_next_sheet() {
    let m = clean(&game(&corridor()));
    let mut w = started(&m);
    put(&mut w, "MAAILMA.A01", 5, 2);
    step(&mut w, "EAST");
    assert_eq!(where_(&w), ("MAAILMA.B01".into(), 0, 2));
    let b = w.s.map_id("MAAILMA.B01").unwrap();
    assert_eq!(w.s.active_map(), Some(b));
    assert_eq!(loaded(&w).len(), 6, "B01's neighbours C01 and C02 load too");
    assert!(w.s.maps.iter().all(|m| m.frozen_at.is_none()), "A01 and A02 stay in B01's region");
    // on C01 the A sheets fall out of the region and freeze
    put(&mut w, "MAAILMA.B01", 5, 2);
    step(&mut w, "EAST");
    assert_eq!(where_(&w), ("MAAILMA.C01".into(), 0, 2));
    let frozen: Vec<&str> = w.s.maps.iter().filter(|m| m.frozen_at.is_some()).map(|m| m.name.as_str()).collect();
    assert_eq!(frozen, ["MAAILMA.A01", "MAAILMA.A02"]);
    // and back west: they thaw
    step(&mut w, "WEST");
    assert_eq!(where_(&w), ("MAAILMA.B01".into(), 5, 2));
    assert!(w.s.maps.iter().all(|m| m.frozen_at.is_none()));
    // a water cell on the neighbour blocks like water on the sheet
    put(&mut w, "MAAILMA.B01", 0, 1);
    step(&mut w, "WEST");
    assert_eq!(where_(&w), ("MAAILMA.B01".into(), 0, 1));
}

#[test]
fn an_actor_on_a_neighbour_sheet_has_turns() {
    use rlml_core::rt::run::{Runner, Status};
    let world = WORLD.replace("~~~~~~\n......\n```", "~~~~~~\n....x.\n```");
    let m = clean(&game(&world));
    let w = started(&m);
    let rat = (0..w.s.entities.len()).find(|&e| w.s.get(e).kind == "RAT").unwrap();
    let rat_map = w.s.at(rat).unwrap().map;
    assert_eq!(w.s.maps[rat_map].name, "MAAILMA.B01");
    let first = w.s.wake_of(rat).unwrap().at;
    let mut r = Runner::new(w);
    for _ in 0..4 {
        assert_eq!(r.run(0).unwrap(), Status::Input);
        r.player(&CommandValue { name: "STD.WAIT".into(), args: vec![] }).unwrap();
    }
    assert!(r.w.s.wake_of(rat).unwrap().at > first, "the rat's turns go on while the player is on A01");
}

#[test]
fn blocked_crossings() {
    let m = clean(&game(WORLD));
    let mut w = started(&m);
    // the grid's outer edge is a plain map edge
    put(&mut w, "MAAILMA.A01", 0, 1);
    step(&mut w, "WEST");
    assert_eq!(where_(&w), ("MAAILMA.A01".into(), 0, 1));
    // a diagonal into the missing sheet B02, which is water
    put(&mut w, "MAAILMA.A01", 5, 2);
    step(&mut w, "SOUTHEAST");
    assert_eq!(where_(&w), ("MAAILMA.A01".into(), 5, 2), "the fill blocks the move");
    // without a fill a missing sheet stops the crossing
    let m = clean(&game(&WORLD.replace(" fill WATER", "")));
    let mut w = started(&m);
    put(&mut w, "MAAILMA.A01", 5, 2);
    step(&mut w, "SOUTH");
    assert_eq!(where_(&w), ("MAAILMA.A01".into(), 5, 2));
}

#[test]
fn a_diagonal_crosses_a_corner() {
    let world = WORLD.replace("## B01", "## B02\n\nmap MAAILMA.B02\n\n```\n......\n```\n\n## B01");
    let m = clean(&game(&world));
    let mut w = started(&m);
    put(&mut w, "MAAILMA.A01", 5, 2);
    step(&mut w, "SOUTHEAST");
    assert_eq!(where_(&w), ("MAAILMA.A01".into(), 5, 2), "the water of the missing A02 is a wall's corner");
    let world = world.replace("## B02", "## A02\n\nmap MAAILMA.A02\n\n```\n......\n```\n\n## B02");
    let m = clean(&game(&world));
    let mut w = started(&m);
    put(&mut w, "MAAILMA.A01", 5, 2);
    step(&mut w, "SOUTHEAST");
    assert_eq!(where_(&w), ("MAAILMA.B02".into(), 0, 0));
}

// ---- searches across a sheet edge (M 6.10) ----

fn cell(w: &World, sheet: &str, x: i32, y: i32) -> Pos {
    Pos { map: w.s.map_id(sheet).unwrap(), x, y }
}

fn named(w: &World, cells: &[Pos]) -> Vec<(String, i32, i32)> {
    cells.iter().map(|p| (w.s.maps[p.map].name.clone(), p.x, p.y)).collect()
}

#[test]
fn distances_reach_across_one_edge() {
    let m = clean(&game(&corridor()));
    let mut w = started(&m);
    w.load_nearby(w.s.map_id("MAAILMA.B01").unwrap()).unwrap();
    let (a, b, c) = (cell(&w, "MAAILMA.A01", 5, 2), cell(&w, "MAAILMA.B01", 0, 2), cell(&w, "MAAILMA.C01", 0, 2));
    assert_eq!(w.dist(a, b), 1);
    assert_eq!(w.dist(b, a), 1);
    assert_eq!(w.dist(cell(&w, "MAAILMA.A01", 0, 0), cell(&w, "MAAILMA.B02", 5, 2)), 11, "diagonal neighbours");
    assert_eq!(w.dist(a, c), i64::MAX, "two edges away is out of reach");
    assert_eq!(w.region_cells(a.map).len(), 4 * 18, "A01 and its three neighbours");
    assert_eq!(w.region_cells(b.map).len(), 6 * 18);
}

#[test]
fn a_line_a_route_and_a_flight_cross_the_edge() {
    let m = clean(&game(&corridor()));
    let w = started(&m);
    let (from, to) = (cell(&w, "MAAILMA.A01", 4, 2), cell(&w, "MAAILMA.B01", 2, 2));
    let along = [("MAAILMA.A01".to_string(), 4, 2), ("MAAILMA.A01".into(), 5, 2), ("MAAILMA.B01".into(), 0, 2), ("MAAILMA.B01".into(), 1, 2), ("MAAILMA.B01".into(), 2, 2)];
    assert_eq!(named(&w, &w.line_across(from, to)), along);
    let route = named(&w, &w.route(from, &[to], None));
    assert_eq!((route.len(), &route[0], route.last().unwrap()), (4, &along[1], &along[4]), "{route:?}");
    assert_eq!(named(&w, &w.ray(from, to, None, 0, None)), along[1..]);
    // a flight goes on past the target and stops at the water of the missing C01
    let ray = w.ray(from, to, None, 10, None);
    assert_eq!(named(&w, &ray).last().unwrap(), &("MAAILMA.B01".to_string(), 5, 2));
    // a route around the water through the missing sheet below is closed
    assert!(w.route(from, &[cell(&w, "MAAILMA.B01", 1, 0)], None).is_empty(), "B01's top row is water");
}
