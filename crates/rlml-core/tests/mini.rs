//! The STD minigame (STD 12) as an acceptance test: it loads cleanly with
//! the standard library, and broken copies give the load errors of M 2.

use std::path::Path;

use rlml_core::check_game;
use rlml_core::error::Report;
use rlml_core::load::MemFiles;

fn read_tree(root: &Path, dir: &Path, out: &mut std::collections::BTreeMap<String, String>) {
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            read_tree(root, &p, out);
        } else {
            let rel = p.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
            out.insert(rel, std::fs::read_to_string(&p).unwrap());
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

fn run(files: &MemFiles) -> Report {
    check_game(files, "game.rlml").1
}

/// Replaces `old` with `new` in one file.
fn edit(files: &mut MemFiles, path: &str, old: &str, new: &str) {
    let src = files.game.get_mut(path).unwrap();
    assert!(src.contains(old), "`{old}` not in {path}");
    *src = src.replacen(old, new, 1);
}

fn errors(r: &Report) -> Vec<String> {
    r.errors().map(|d| d.to_string()).collect()
}

fn expect_error(r: &Report, part: &str) {
    let e = errors(r);
    assert!(e.iter().any(|m| m.contains(part)), "expected an error with `{part}`, got {e:#?}");
}

#[test]
fn minigame_loads_cleanly() {
    let r = run(&mini());
    let all: Vec<String> = r.0.iter().map(|d| d.to_string()).collect();
    assert!(all.is_empty(), "{all:#?}");
}

#[test]
fn unknown_map_character() {
    let mut f = mini();
    edit(&mut f, "maps/caves.rlml", "#.@..#", "#.@X.#");
    expect_error(&run(&f), "maps/caves.rlml:12: error: unknown character `X` at row 3, column 4");
}

#[test]
fn missing_language_setting() {
    let mut f = mini();
    edit(&mut f, "game.rlml", "- language fi\n", "");
    expect_error(&run(&f), "missing setting `language`");
}

#[test]
fn exit_label_is_checked_in_the_target_map() {
    let mut f = mini();
    edit(&mut f, "game.rlml", "- EXIT to CAVE_B", "- EXIT to CAVE_B at BOTTOM");
    expect_error(&run(&f), "`STD.EXIT.at`: map `CAVE_B` has no label `BOTTOM`");
}

#[test]
fn start_search() {
    let mut f = mini();
    edit(&mut f, "game.rlml", "- at SPAWN", "- at SPAWNN");
    expect_error(&run(&f), "map `CAVE_A` has no label `SPAWNN`");
    let mut f = mini();
    edit(&mut f, "game.rlml", "- start CAVE_A", "- start CAVE_C");
    expect_error(&run(&f), "`start`: `CAVE_C` is not defined");
}

#[test]
fn keys_and_menus() {
    let mut f = mini();
    edit(&mut f, "ui/main.rlml", "- WAIT s", "- WIAT s");
    expect_error(&run(&f), "`WIAT` is not defined");
    let mut f = mini();
    edit(&mut f, "ui/main.rlml", "- pass z", "- pass s");
    expect_error(&run(&f), "key `s` is bound twice in context `GAME`");
    let mut f = mini();
    edit(&mut f, "ui/main.rlml", "- RESUME", "- RESUMEE");
    expect_error(&run(&f), "item `RESUMEE` has no action");
}

#[test]
fn component_rows() {
    let mut f = mini();
    edit(&mut f, "game.rlml", "- HP now 3", "- HP 3");
    expect_error(&run(&f), "`STD.HP` has 2 fields, so the row must name them: now, max");
    let mut f = mini();
    edit(&mut f, "game.rlml", "- MELEE 1d2", "- MELEE strong");
    expect_error(&run(&f), "`STD.MELEE.damage`: `strong` is not dice");
}

#[test]
fn texts_and_logs() {
    let mut f = mini();
    edit(&mut f, "ui/main.rlml", "- STD.MESSAGE.SENSE", "- STD.MESSAGE.SENSES");
    expect_error(&run(&f), "`STD.MESSAGE.SENSES` is not the start of any message key");
    let mut f = mini();
    f.engine.get_mut("lib/std/text/fi.rlml").unwrap().push_str("STD.MESSAGE.COMBAT.MISS {TARGETT} väistää.\n");
    expect_error(&run(&f), "message `STD.MESSAGE.COMBAT.MISS` has no parameter `TARGETT`");
}

#[test]
fn labels_must_not_be_general_names() {
    let mut f = mini();
    edit(&mut f, "maps/caves.rlml", "- @ FLOOR as SPAWN", "- @ FLOOR as RAT");
    edit(&mut f, "game.rlml", "- at SPAWN", "- at RAT");
    expect_error(&run(&f), "label `RAT` of map `CAVE_A` has the same name as the entity `RAT`");
}

#[test]
fn syntax_error_in_the_standard_library() {
    let mut f = mini();
    let ai = f.engine.get_mut("lib/std/ai.rlml").unwrap();
    assert!(ai.contains("        return APPROACH(foe)\n"));
    *ai = ai.replacen("        return APPROACH(foe)\n", "        return APPROACH(foe\n", 1);
    expect_error(&run(&f), "<engine>/lib/std/ai.rlml:48: error: column 28: expected `)`, found the end of the line");
    expect_error(&run(&f), "`HUNTER` is not defined");
}

/// Replaces `old` with `new` in a standard library file.
fn edit_std(files: &mut MemFiles, path: &str, old: &str, new: &str) {
    let src = files.engine.get_mut(path).unwrap();
    assert!(src.contains(old), "`{old}` not in {path}");
    *src = src.replacen(old, new, 1);
}

#[test]
fn names_in_logic() {
    let mut f = mini();
    edit_std(&mut f, "lib/std/ai.rlml", "    return WANDER()\n\ncomponent", "    return WANDERR()\n\ncomponent");
    expect_error(&run(&f), "`WANDERR` is not defined (tried STD.WANDERR, WANDERR)");
    let mut f = mini();
    edit_std(&mut f, "lib/std/ai.rlml", "return APPROACH(foe)", "return APPROACH(fo)");
    expect_error(&run(&f), "unknown name `fo`");
    let mut f = mini();
    edit_std(&mut f, "lib/std/exits.rlml", "it has EXIT", "it has EXITS");
    expect_error(&run(&f), "`EXITS` is not defined");
    let mut f = mini();
    edit_std(&mut f, "lib/std/combat.rlml", "emit HIT(target, attacker, attacker.MELEE.damage)", "emit STRIKE(target, attacker)");
    expect_error(&run(&f), "`STD.STRIKE` is a function");
    let mut f = mini();
    edit_std(&mut f, "lib/std/move.rlml", "noise(to, STEP_SOUND(who), source: who)", "nois(to, STEP_SOUND(who), source: who)");
    expect_error(&run(&f), "unknown engine function `nois`");
    let mut f = mini();
    edit_std(&mut f, "lib/std/exits.rlml", "exit = (entities(in: actor.at) where it has EXIT).first", "exit = it");
    expect_error(&run(&f), "`it` is the member inside `where`");
}

#[test]
fn kinds_of_bodies() {
    let mut f = mini();
    edit_std(&mut f, "lib/std/core.rlml", "    if game has PLAYER:", "    wait 120\n    if game has PLAYER:");
    expect_error(&run(&f), "`wait` is allowed only in a command or a coroutine scene");
    let mut f = mini();
    edit_std(&mut f, "lib/std/move.rlml", "    return to in adjacent(who)", "    move(who, to: to)\n    return to in adjacent(who)");
    expect_error(&run(&f), "`move` changes the game's state; it is not allowed in a `query`");
    let mut f = mini();
    edit_std(&mut f, "lib/std/move.rlml", "    noise(to, STEP_SOUND(who), source: who)", "    noise(to, STEP_SOUND(who), source: who)\n    return 1");
    expect_error(&run(&f), "a `fn` without a return type returns no value");
    let mut f = mini();
    edit_std(&mut f, "lib/std/relations.rlml", "    return ATTITUDE(a, b) < 0 or ATTITUDE(b, a) < 0", "    BEFRIEND(a, b, 1)\n    return ATTITUDE(a, b) < 0 or ATTITUDE(b, a) < 0");
    expect_error(&run(&f), "`STD.BEFRIEND` is a `fn` that may change the game's state");
    let mut f = mini();
    edit_std(&mut f, "lib/std/core.rlml", "    if game has PLAYER:", "    cancel\n    if game has PLAYER:");
    expect_error(&run(&f), "`cancel` belongs to a `before` handler");
    let mut f = mini();
    edit_std(&mut f, "lib/std/core.rlml", "on begin() as SPAWN_PLAYER:", "before begin() as SPAWN_PLAYER:");
    expect_error(&run(&f), "engine event `begin` has no `before` phase");
}

#[test]
fn style() {
    let mut f = mini();
    edit(&mut f, "style/ascii.rlml", "- GLYPH.RAT r fg FUR", "- GLYPH.RAT r fg FURR");
    expect_error(&run(&f), "`FURR` is not a color name or a hex color");
}

#[test]
fn data_field_types_read() {
    use rlml_core::model::Kind;
    let (m, _) = check_game(&mini(), "game.rlml");
    let mut n = 0;
    for def in m.defs.values().filter(|d| d.kind == Kind::Component) {
        for f in rlml_core::fields::component_fields(def) {
            n += 1;
            let t = rlml_core::rs::types::parse(&f.ty, &|c| m.resolve(None, c).is_ok_and(|d| d.kind == Kind::Component));
            assert!(t.is_ok(), "{}.{}: {:?}", def.full, f.name, t);
        }
    }
    assert!(n > 20, "{n} fields");
}

/// Inference knows every parameter and local of the standard library.
#[test]
fn std_local_types_are_inferred() {
    let (m, _) = check_game(&mini(), "game.rlml");
    let mut unknown = Vec::new();
    let mut by_name = std::collections::BTreeMap::new();
    for d in &m.code {
        for (local, ty) in rlml_core::rs::check::local_types(&m, d) {
            if !ty.is_known() {
                unknown.push(format!("{}.{local}", d.name));
            }
            by_name.insert(format!("{}.{local}", d.name), ty.to_string());
        }
    }
    assert!(unknown.is_empty(), "{unknown:?}");
    assert!(by_name.len() > 200, "{} locals", by_name.len());
    for (local, ty) in [
        ("HUNTER.foe", "entity"),
        ("HUNTER.noises", "collection sound"),
        ("ROOMS.rooms", "list area"),
        ("ROOMS.want", "number"),
        ("SHOOT.w", "STD.RANGED"),
        ("SHOOT.land", "pos"),
        ("CONNECT.goal", "pos"),
        ("FEED.eaters", "collection"),
        ("ADJACENCY_ARRIVE.map", "name"),
        ("HEAR_MESSAGE.s", "sound"),
        ("BASE.s", "number"),
    ] {
        assert_eq!(by_name.get(local).map(String::as_str), Some(ty), "{local}");
    }
}

#[test]
fn local_types_stick() {
    let mut f = mini();
    edit_std(&mut f, "lib/std/ai.rlml", "        near_foe = foe.at in adjacent(self)\n", "        near_foe = foe.at in adjacent(self)\n        foe = 1\n");
    expect_error(&run(&f), "`foe` has the type `entity`; a `number` cannot be assigned to it");
    let mut f = mini();
    edit_std(&mut f, "lib/std/gen.rlml", "            rooms += room\n", "            rooms += room\n            rooms += 1\n");
    expect_error(&run(&f), "`rooms` has the type `list area`; a `number` cannot be added to it");
    let mut f = mini();
    edit_std(&mut f, "lib/std/gen.rlml", "    rooms = []\n", "    rooms = [1, \"a\"]\n");
    expect_error(&run(&f), "a list holds one type");
}
