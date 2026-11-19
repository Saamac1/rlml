//! Named map areas (D 8.11) on the minigame's first cave: load-time checks,
//! nesting, and areas as the targets and anchors of a place search (D 8.8).

use std::path::Path;

use rlml_core::check_game;
use rlml_core::error::Report;
use rlml_core::load::MemFiles;
use rlml_core::rt::value::Pos;
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

fn mini() -> MemFiles {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut files = MemFiles::default();
    let game = repo.join("esimerkit/mini");
    read_tree(&game, &game, &mut files.game);
    read_tree(&repo, &repo.join("lib"), &mut files.engine);
    files
}

const CAVES: &str = "maps/caves.rlml";

/// The minigame with `areas:` rows after CAVE_A's legend.
fn with_areas(rows: &str) -> MemFiles {
    let mut f = mini();
    let src = f.game.get_mut(CAVES).unwrap();
    let at = src.find("map CAVE_B").unwrap();
    src.insert_str(at, &format!("areas:\n{rows}\n"));
    f
}

/// A cave, its west room and the room's corner around `SPAWN` (2,2).
const NESTED: &str = "- 0,0 size 20x8 as LUOLA\n- 1,1 size 4x6 as VASEN\n- 1,1 size 2x2 as NURKKA\n";

fn messages(r: &Report) -> Vec<String> {
    r.0.iter().map(|d| d.to_string()).collect()
}

fn expect(r: &Report, part: &str) {
    let all = messages(r);
    assert!(all.iter().any(|m| m.contains(part)), "expected a message with `{part}`, got {all:#?}");
}

fn edit(f: &mut MemFiles, path: &str, old: &str, new: &str) {
    let src = f.game.get_mut(path).unwrap();
    assert!(src.contains(old), "`{old}` not in {path}");
    *src = src.replacen(old, new, 1);
}

/// The cells of `at SEARCH` on the start map.
fn search(f: &mut MemFiles, at: &str) -> Vec<(i32, i32)> {
    edit(f, "game.rlml", "- at SPAWN", &format!("- at {at}"));
    let (m, r) = check_game(f, "game.rlml");
    assert!(r.0.is_empty(), "{:#?}", messages(&r));
    let w = World::new_game(&m, 7).unwrap();
    w.s.game.at.cells().iter().map(|p| (p.x, p.y)).collect()
}

#[test]
fn nested_areas_load_and_give_a_path() {
    let f = with_areas(NESTED);
    let (m, r) = check_game(&f, "game.rlml");
    assert!(r.0.is_empty(), "{:#?}", messages(&r));
    let w = World::new_game(&m, 7).unwrap();
    let a = &w.s.maps[0].areas;
    let path: Vec<&str> = a.path(2, 2).iter().map(|a| a.name.as_str()).collect();
    assert_eq!(path, ["LUOLA", "VASEN", "NURKKA"]);
    assert_eq!(a.deepest(10, 4).unwrap().name, "LUOLA");
    assert_eq!(a.get("NURKKA").unwrap().level, 2);
}

#[test]
fn an_area_is_a_search_target() {
    let cells = search(&mut with_areas(NESTED), "VASEN");
    assert_eq!(cells.len(), 24);
    assert_eq!((cells[0], cells[23]), ((1, 1), (4, 6)), "reading order");
}

#[test]
fn near_takes_a_label_or_an_area_as_its_anchor() {
    // every cell around SPAWN is floor
    assert_eq!(search(&mut with_areas(NESTED), "FLOOR near SPAWN").len(), 9, "the default distance is 1");
    assert_eq!(search(&mut with_areas(NESTED), "SPAWN near 2").len(), 25, "without an anchor the target is the anchor");
    // a ring of distance exactly 1 around the corner; the walls are not floor
    assert_eq!(search(&mut with_areas(NESTED), "FLOOR near NURKKA 1..1"), [(3, 1), (3, 2), (1, 3), (2, 3), (3, 3)]);
    // the cave's walls next to the west room: west 8, north 5, south 5, east 3
    assert_eq!(search(&mut with_areas(NESTED), "WALL near VASEN").len(), 21);
}

#[test]
fn from_moves_from_the_anchor() {
    assert_eq!(search(&mut with_areas(NESTED), "1,0 from SPAWN"), [(3, 2)]);
    assert_eq!(search(&mut with_areas(NESTED), "2 north from SPAWN"), [(2, 0)]);
    assert_eq!(search(&mut with_areas(NESTED), "1 southeast from NURKKA"), [(2, 2), (3, 2), (2, 3), (3, 3)]);
    assert!(search(&mut with_areas(NESTED), "5,0 from LUOLA").len() < 160, "cells off the map drop out");
}

#[test]
fn row_errors() {
    let (_, r) = check_game(&with_areas("- 18,0 size 4x2 as ULKONA\n"), "game.rlml");
    expect(&r, "area `ULKONA`: 18,0 size 4x2 is not inside the 20x8 map");
    let (_, r) = check_game(&with_areas("- 1,1 4x2 as X\n"), "game.rlml");
    expect(&r, "an `areas:` row is `- X,Y size WxH as NAME`");
    let (_, r) = check_game(&with_areas("- 1,1 size 0x2 as X\n"), "game.rlml");
    expect(&r, "at least 1x1");
}

#[test]
fn name_errors() {
    let (_, r) = check_game(&with_areas("- 1,1 size 2x2 as SPAWN\n"), "game.rlml");
    expect(&r, "`SPAWN` is both a label and an area on map `CAVE_A`");
    let (_, r) = check_game(&with_areas("- 1,1 size 2x2 as RAT\n"), "game.rlml");
    expect(&r, "area `RAT` of map `CAVE_A` has the same name as the entity `RAT`");
    let mut f = with_areas("- 1,1 size 2x2 as HALL\n");
    f.game.get_mut(CAVES).unwrap().push_str("\nentity CAVE_A.HALL\n- STD.HP now 1\n");
    let (_, r) = check_game(&f, "game.rlml");
    expect(&r, "`CAVE_A.HALL` is the entity at maps/caves.rlml");
}

#[test]
fn a_label_with_the_map_name_must_not_be_a_general_name() {
    let mut f = mini();
    f.game.get_mut(CAVES).unwrap().push_str("\nentity CAVE_A.SPAWN\n- STD.HP now 1\n");
    let (_, r) = check_game(&f, "game.rlml");
    expect(&r, "label `SPAWN` of map `CAVE_A`: `CAVE_A.SPAWN` is the entity");
}

#[test]
fn same_cells_and_partial_overlaps_warn() {
    let (_, r) = check_game(&with_areas("- 1,1 size 4x6 as VASEN\n- 1,1 size 4x6 as HUONE\n"), "game.rlml");
    assert!(!r.has_errors());
    expect(&r, "warning: areas `VASEN` and `HUONE` of map `CAVE_A` have the same cells");
    let (_, r) = check_game(&with_areas("- 1,1 size 4x6 as VASEN\n- 3,1 size 4x2 as KAISTA\n"), "game.rlml");
    expect(&r, "warning: areas `VASEN` and `KAISTA` of map `CAVE_A` share cells, but neither is inside the other");
}

#[test]
fn search_checks_know_areas() {
    let mut f = with_areas(NESTED);
    edit(&mut f, "game.rlml", "- at SPAWN", "- at FLOOR near NOPE 2");
    let (_, r) = check_game(&f, "game.rlml");
    expect(&r, "`near`: map `CAVE_A` has no label or area `NOPE`");
}

#[test]
fn an_extending_map_gets_its_base_areas() {
    let mut f = with_areas("- 1,1 size 4x6 as VASEN\n");
    f.game.get_mut(CAVES).unwrap().push_str("\nmap CAVE_C extends CAVE_A\n\nareas:\n- 1,1 size 2x2 as NURKKA\n- 30,1 size 2x2 as POIS\n");
    let (m, r) = check_game(&f, "game.rlml");
    expect(&r, "area `POIS`: 30,1 size 2x2 is not inside the 20x8 map");
    assert_eq!(r.errors().count(), 1, "{:#?}", messages(&r));
    // without the broken row the map loads with both areas
    edit(&mut f, CAVES, "- 30,1 size 2x2 as POIS\n", "");
    edit(&mut f, "game.rlml", "- start CAVE_A", "- start CAVE_C");
    let (m2, r) = check_game(&f, "game.rlml");
    assert!(r.0.is_empty(), "{:#?}", messages(&r));
    drop(m);
    let w = World::new_game(&m2, 7).unwrap();
    let path: Vec<&str> = w.s.maps[0].areas.path(1, 1).iter().map(|a| a.name.as_str()).collect();
    assert_eq!(path, ["VASEN", "NURKKA"], "the base's rows come first");
    assert_eq!(w.s.game.at.cells(), [Pos { map: 0, x: 2, y: 2 }], "the base's labels");
    let c = &w.s.maps[0];
    assert_eq!((c.name.as_str(), c.width, c.height), ("CAVE_C", 20, 8), "the base's drawing");
    assert_eq!((c.tile(0, 0, 0), c.tile(0, 2, 2)), (Some("WALL"), Some("FLOOR")));
}

#[test]
fn an_extending_map_inherits_vars_and_at_rows_through_a_chain() {
    let mut f = mini();
    f.game.get_mut(CAVES).unwrap().push_str(concat!(
        "\nmap CAVE_C extends CAVE_A\n\nvars:\n- DEPTH 1\n- DARK false\n\nat:\n- 3,3 FLOOR as MARK\n",
        "\nmap CAVE_D extends CAVE_C\n\nlegend:\n- . WALL\n\nvars:\n- DEPTH 2\n\nat:\n- 4,4 FLOOR as MARK2\n",
    ));
    edit(&mut f, "game.rlml", "- start CAVE_A", "- start CAVE_D");
    edit(&mut f, "game.rlml", "- at SPAWN", "- at MARK2");
    let (m, r) = check_game(&f, "game.rlml");
    assert!(r.0.is_empty(), "{:#?}", messages(&r));
    let w = World::new_game(&m, 7).unwrap();
    let d = &w.s.maps[0];
    assert_eq!(d.label("MARK"), [(3, 3)], "an `at:` row of the base's base");
    assert_eq!(d.label("SPAWN"), [(2, 2)]);
    assert_eq!(d.tile(0, 1, 1), Some("WALL"), "the own legend replaces `.`");
    assert_eq!(d.tile(0, 3, 3), Some("FLOOR"), "`at:` rows come after the drawing");
    let vars: Vec<String> = d.vars.iter().map(|(n, v)| format!("{n}={v}")).collect();
    assert_eq!(vars, ["DARK=false", "DEPTH=2"]);
}

#[test]
fn a_prefab_cannot_have_areas() {
    let mut f = mini();
    f.game.get_mut(CAVES).unwrap().push_str("\nprefab NOOK\n\n```\n..\n```\n\nlegend:\n- . FLOOR\n\nareas:\n- 0,0 size 1x1 as SOPPI\n");
    let (_, r) = check_game(&f, "game.rlml");
    expect(&r, "a prefab cannot have `areas:` yet");
}
