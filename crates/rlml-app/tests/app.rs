//! The game loop with a clock of its own (M 6.1.2, M 7.2, M 8).

use std::path::Path;
use std::time::Duration;

use rlml_core::load::MemFiles;
use rlml_core::model::Model;
use rlml_core::rt::eval::{BodyKind, Env};
use rlml_core::rt::run::Status;
use rlml_app::app::App;
use termplate_core::keys::{Binding, KeyCode, Mods, parse_binding};

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

fn mini_with(edit: impl FnOnce(&mut String)) -> MemFiles {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut f = MemFiles::default();
    read_tree(&repo, &repo.join("lib"), &mut f.engine);
    let game = repo.join("esimerkit/mini");
    read_tree(&game, &game, &mut f.game);
    edit(f.game.get_mut("game.rlml").unwrap());
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

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

fn player_x(app: &App) -> i32 {
    app.r.w.s.at(app.r.w.s.player.unwrap()).unwrap().x
}

#[test]
fn keys_move_the_player_and_walls_take_no_time() {
    let f = mini_with(|_| {});
    let m = model(&f);
    let mut app = App::new(&m, 7, (40, 12)).unwrap();
    assert!(app.players_turn());
    let x = player_x(&app);
    app.key(key("l"), ms(10)).unwrap();
    assert_eq!(player_x(&app), x + 1);
    assert_eq!(app.r.w.s.game.time, 120);
    app.key(key("right"), ms(20)).unwrap();
    app.key(key("l"), ms(30)).unwrap();
    assert_eq!(player_x(&app), x + 2, "a wall");
    assert_eq!(app.r.w.s.game.time, 240, "takes no time (M 6.1.5)");
    let frame = app.draw();
    assert_eq!(frame.rows()[2].chars().nth(10 + (x + 2) as usize), Some('@'));
}

#[test]
fn without_a_player_rounds_go_at_the_round_pace() {
    let f = mini_with(|_| {});
    let m = model(&f);
    let mut app = App::new(&m, 7, (40, 12)).unwrap();
    let p = app.r.w.s.player.unwrap();
    app.r.w.destroy(p, &Env::new(BodyKind::Handler, "test", None, "test")).unwrap();
    app.key(key("z"), ms(0)).unwrap();
    assert_eq!(app.status, Status::NoPlayer { rounds: 1 }, "`pass` runs the next round at once");
    let turn = app.r.w.s.game.turn();
    app.tick(ms(100)).unwrap();
    assert_eq!(app.r.w.s.game.turn(), turn, "`round 250ms`: not yet");
    app.tick(ms(250)).unwrap();
    assert_eq!(app.r.w.s.game.turn(), turn + 1);
    app.tick(ms(500)).unwrap();
    app.tick(ms(750)).unwrap();
    assert_eq!(app.r.w.s.game.turn(), turn + 3, "a round every 250 ms (M 6.1.2)");
}

#[test]
fn idle_gives_the_turn_away() {
    let f = mini_with(|g| *g = g.replace("- round 250ms", "- round 250ms\n- idle 1s STD.WAIT"));
    let m = model(&f);
    let mut app = App::new(&m, 7, (40, 12)).unwrap();
    app.tick(ms(900)).unwrap();
    assert_eq!(app.r.w.s.game.time, 0);
    app.tick(ms(1000)).unwrap();
    assert_eq!(app.r.w.s.game.time, 120, "`idle 1s STD.WAIT` waited for the player");
    app.key(key("s"), ms(1500)).unwrap();
    app.tick(ms(2400)).unwrap();
    assert_eq!(app.r.w.s.game.time, 240, "a key starts the time again");
}

#[test]
fn idle_with_the_players_ai() {
    let f = mini_with(|g| *g = g.replace("- round 250ms", "- round 250ms\n- idle 1s ai"));
    let m = model(&f);
    let mut app = App::new(&m, 7, (40, 12)).unwrap();
    app.tick(ms(1000)).unwrap();
    assert!(app.r.w.s.game.time > 0, "the player's HUNTER chose a command (M 6.1.2)");
}

#[test]
fn in_between_frames_with_presentation_frame() {
    let f = mini_with(|g| g.push_str("\npresentation\n- frame 15ms\n"));
    let m = model(&f);
    let mut app = App::new(&m, 7, (40, 12)).unwrap();
    assert_eq!(app.frame_time(), ms(15));
    app.frames.clear();
    for k in ["s", "s", "s"] {
        app.key(key(k), ms(0)).unwrap();
    }
    assert!(!app.frames.is_empty(), "the rat moved in sight between the player's turns");
    assert!(app.frames.windows(2).all(|w| w[0] != w[1]), "only changed moments are frames");
    let g = mini_with(|_| {});
    let m2 = model(&g);
    let mut quiet = App::new(&m2, 7, (40, 12)).unwrap();
    quiet.key(key("s"), ms(0)).unwrap();
    assert!(quiet.frames.is_empty(), "`frame` 0: only the state at the next input (D 9.3)");
}

#[test]
fn ctrl_c_quits_and_resizing_redraws() {
    let f = mini_with(|_| {});
    let m = model(&f);
    let mut app = App::new(&m, 7, (40, 12)).unwrap();
    app.resize(60, 20);
    assert_eq!((app.draw().width, app.draw().height), (60, 20));
    app.key(Binding::key(KeyCode::Char('c'), Mods { ctrl: true, ..Mods::default() }), ms(0)).unwrap();
    assert!(app.quit);
}

fn screen(app: &mut App) -> String {
    app.draw().rows().join("\n")
}

fn press(app: &mut App, keys: &str) {
    for k in keys.split_whitespace() {
        app.key(key(k), ms(10)).unwrap();
    }
}

#[test]
fn pause_opens_stops_time_and_closes_with_the_same_key() {
    let f = mini_with(|_| {});
    let m = model(&f);
    let mut app = App::new(&m, 7, (40, 12)).unwrap();
    press(&mut app, "esc");
    assert_eq!(app.contexts(), ["GAME", "MENU"]);
    let s = screen(&mut app);
    assert!(s.contains(" Tauko ") && s.contains("Jatka") && s.contains("Lopeta"), "{s}");
    // the game's keys do not reach under the menu, and time stands
    let (x, time) = (player_x(&app), app.r.w.s.game.time);
    press(&mut app, "l");
    app.tick(ms(5000)).unwrap();
    assert_eq!((player_x(&app), app.r.w.s.game.time), (x, time));
    press(&mut app, "esc");
    assert!(!app.menus.is_open());
    press(&mut app, "l");
    assert_eq!(player_x(&app), x + 1);
}

#[test]
fn resume_closes_and_quit_goes_to_the_engine_menu() {
    let f = mini_with(|_| {});
    let m = model(&f);
    let mut app = App::new(&m, 7, (40, 12)).unwrap();
    press(&mut app, "esc enter");
    assert!(!app.menus.is_open() && app.players_turn());
    // QUIT: the game ends, and without `title` the engine's menu opens
    press(&mut app, "esc j enter");
    assert!(!app.playing && !app.quit);
    let s = screen(&mut app);
    assert!(s.contains("New game") && s.contains("Exit"), "{s}");
    // the engine's menu is final: neither esc nor back closes it
    press(&mut app, "backspace");
    assert!(app.menus.is_open());
    // a new game takes a new seed
    press(&mut app, "enter");
    assert!(app.playing && app.players_turn() && !app.menus.is_open());
    assert_ne!(app.seed, 7);
    assert_eq!(app.games, 2);
    press(&mut app, "esc j enter j enter");
    assert!(app.quit);
}

#[test]
fn items_hide_disable_and_open_menus() {
    let f = mini_with(|g| {
        *g = g.replace("- PLAYER HERO", "- PLAYER HERO\n- title OPENING");
        g.push_str(
            "\nmenu OPENING final\n- NEW_GAME new_game\n- MORE open MORE\n- GONE quit hide when game.time >= 0\n- DIM quit disabled when game.time >= 0\n- EXIT\n\n\
             menu MORE size 20x*\n- AGAIN open MORE stack\n- HOME open OPENING\n- BACK back\n- CLOSE close\n",
        );
    });
    let m = model(&f);
    let mut app = App::new(&m, 7, (40, 12)).unwrap();
    // the game starts in its `title` menu
    assert!(!app.playing && app.games == 0);
    let layer = &app.menus.layers[0];
    let shown: Vec<&str> = layer.rows.iter().map(|r| layer.menu.items[r.item].key.as_str()).collect();
    assert_eq!(shown, ["NEW_GAME", "MORE", "DIM", "EXIT"]);
    assert!(!layer.rows[2].enabled);
    // the cursor skips the disabled item
    press(&mut app, "j j");
    assert_eq!(app.menus.layers[0].cursor, 3);
    press(&mut app, "k enter");
    assert_eq!(app.menus.layers.len(), 2);
    // `stack` adds a layer, `open` returns to the one already open
    press(&mut app, "enter enter");
    assert_eq!(app.menus.layers.len(), 4);
    press(&mut app, "j enter");
    assert_eq!(app.menus.layers.len(), 1);
    // `back` closes one, `close` all but the final menu
    press(&mut app, "j enter enter j j enter");
    assert_eq!(app.menus.layers.len(), 2);
    press(&mut app, "j j j enter");
    assert_eq!(app.menus.layers.len(), 1);
    // the first new game takes the seed given
    press(&mut app, "k enter");
    assert!(app.playing && app.seed == 7 && app.menus.layers.is_empty());
    // `quit` returns to the title menu
    press(&mut app, "esc j enter");
    assert!(!app.playing && app.menus.layers[0].menu.full == "OPENING");
}


fn ryhma() -> MemFiles {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut files = MemFiles::default();
    let game = repo.join("esimerkit/ryhma");
    read_tree(&game, &game, &mut files.game);
    read_tree(&repo, &repo.join("lib"), &mut files.engine);
    files
}

#[test]
fn the_turn_marker_flashes_and_blinks() {
    let f = ryhma();
    let m = model(&f);
    let mut app = App::new(&m, 7, (40, 14)).unwrap();
    assert!(app.players_turn());
    let p = app.r.w.s.player.unwrap();
    let reversed = |app: &mut App| {
        let frame = app.draw();
        frame.cells.iter().any(|c| c.ch == '@' && c.style.as_ref().is_some_and(|s| s.attrs.reverse))
    };
    app.tick(ms(10)).unwrap();
    assert!(reversed(&mut app), "the turn starts with a flash (M 7.1)");
    // a 600 ms flash, then 400 ms on and 400 ms off from 610 ms
    app.tick(ms(1200)).unwrap();
    assert!(!reversed(&mut app), "then it blinks off");
    app.tick(ms(1500)).unwrap();
    assert!(reversed(&mut app), "and on again");
    assert_eq!(app.r.w.s.player, Some(p));
}

#[test]
fn one_member_has_no_turn_marker() {
    let f = mini_with(|_| {});
    let m = model(&f);
    let mut app = App::new(&m, 7, (40, 12)).unwrap();
    app.tick(ms(10)).unwrap();
    app.draw();
    assert_eq!(app.view.marked, None);
}

#[test]
fn colors_fit_the_256_color_palette() {
    use rlml_app::draw::ansi256;
    assert_eq!(ansi256([0, 0, 0]), 16);
    assert_eq!(ansi256([255, 255, 255]), 231);
    assert_eq!(ansi256([0xf0, 0xd0, 0x40]), 221, "the example's seat 1 yellow");
    assert_eq!(ansi256([0x60, 0xa0, 0xff]), 75, "and seat 2 blue");
    assert_eq!(ansi256([0x4a, 0x4a, 0x5a]), 239, "a grey takes the grey ramp");
}

#[test]
fn the_camera_follows_the_group_and_falls_back_to_the_player() {
    use rlml_core::rt::state::Place;
    use rlml_core::rt::value::Pos;
    let f = ryhma();
    let m = model(&f);
    let mut app = App::new(&m, 7, (80, 24)).unwrap();
    let members = app.r.w.s.players();
    let player = app.r.w.s.player.unwrap();
    let place = |app: &mut App, xs: &[i32]| {
        for (&e, &x) in members.iter().zip(xs) {
            app.r.w.s.get_mut(e).place = Place::Map(Pos { map: 0, x, y: 4 });
        }
    };
    // the map shows 20 cells of the 40-wide cave, even in a wide window
    let xs: Vec<i32> = members.iter().map(|&e| if e == player { 20 } else { 10 }).collect();
    place(&mut app, &xs);
    app.draw();
    assert_eq!(app.view.camera[&0].0, 5, "the group's box 10..20 in the middle (M 7.1)");
    // spread wider than the area: the camera follows the player
    let xs: Vec<i32> = members.iter().map(|&e| if e == player { 30 } else { 2 }).collect();
    place(&mut app, &xs);
    app.draw();
    assert_eq!(app.view.camera[&0].0, 20, "the player at 30, clamped to the map's edge");
    // together again: back to the group
    let xs: Vec<i32> = members.iter().map(|&e| if e == player { 14 } else { 12 }).collect();
    place(&mut app, &xs);
    app.draw();
    assert_eq!(app.view.camera[&0].0, 3);
}

#[test]
fn the_map_shows_its_written_size_in_a_wide_window() {
    let f = ryhma();
    let m = model(&f);
    for size in [(34, 13), (80, 24)] {
        let mut app = App::new(&m, 7, size).unwrap();
        let frame = app.draw();
        let r = app.view.map_rect.expect("the map is drawn");
        assert_eq!((r.x, r.y, r.w, r.h), (0, 0, 20, 8), "the written 20x8 in {size:?}: what the leash measures (M 7.1)");
        assert_eq!(app.r.w.view_dims(), (20, 8));
        let rows = frame.rows();
        assert!(rows[0].chars().skip(20).collect::<String>().starts_with("Tutki"), "beside the map is the verbs menu: {:?}", rows[0]);
        assert!(rows[4..8].iter().all(|row| row.chars().skip(20).all(|c| c == ' ')), "and below its rows nothing");
        let (cx, cy) = app.view.camera[&0];
        assert!((0..=20).contains(&cx), "the camera has a place on the 40-wide cave: {cx}");
        // the map is drawn: the player shows where `cell_at` finds it
        let p = app.r.w.s.at(app.r.w.s.player.unwrap()).unwrap();
        let (sx, sy) = ((p.x - cx) as usize, (p.y - cy) as usize);
        assert_eq!(rows[sy].chars().nth(sx), Some('@'), "{rows:#?}");
        assert_eq!(rlml_app::draw::cell_at(&app.view, 0, sx, sy), Some(p));
        assert_eq!(rlml_app::draw::cell_at(&app.view, 0, 30, sy), None, "beside the map is no map cell");
    }
    // a window too small for the layout draws no map, and keeps no rect
    let mut app = App::new(&m, 7, (80, 24)).unwrap();
    app.draw();
    app.resize(10, 5);
    app.draw();
    assert_eq!(app.view.map_rect, None);
}

#[test]
fn each_seat_plays_from_its_own_device() {
    use rlml_app::input::{Device, PadPress};
    let f = ryhma();
    let m = model(&f);
    let mut app = App::new(&m, 7, (80, 24)).unwrap();
    assert_eq!(app.seat_devices.get(&1), Some(&Device::Keyboard(1)));
    assert_eq!(app.seat_devices.get(&2), Some(&Device::Keyboard(2)), "keyboard parts first (M 8.2)");
    assert_eq!(app.r.awaiting(), Some(1));
    // seat 2's key on seat 1's turn is dropped
    app.key(key("."), ms(10)).unwrap_or(());
    let t = app.r.w.s.game.time;
    assert_eq!(app.r.awaiting(), Some(2), "seat 1 waited with `.`");
    app.key(key("."), ms(20)).unwrap();
    assert_eq!((app.r.awaiting(), app.r.w.s.game.time), (Some(2), t), "`.` is part 1's: dropped on seat 2's turn");
    app.key(key("q"), ms(30)).unwrap();
    assert_eq!(app.r.awaiting(), Some(1), "`q` is part 2's: seat 2 waited");
    // a pad that connects plays seat 2 instead of keyboard part 2
    app.pad(0, PadPress::parse("south").unwrap(), ms(40)).unwrap();
    assert_eq!(app.r.awaiting(), Some(1), "seat 1's turn: the pad's press is dropped");
    assert_eq!(app.seat_devices.get(&2), Some(&Device::Pad(0)), "pads before the other keyboard parts (M 8.2)");
    app.pad_disconnected(0);
    assert_eq!(app.seat_devices.get(&2), Some(&Device::Keyboard(2)), "back to its keyboard part");
}

#[test]
fn a_pad_takes_the_first_seat_without_a_device() {
    use rlml_app::input::{Device, PadPress};
    let f = {
        let mut f = ryhma();
        let ui = f.game.get_mut("ui/main.rlml").unwrap();
        let i = ui.find("keys GAME keyboard 2").unwrap();
        let j = ui[i..].find("\npad GAME").unwrap() + i;
        ui.replace_range(i..j, "");
        let i = ui.find("keys LOOK keyboard 2").unwrap();
        let j = ui[i..].find("\npad LOOK").unwrap() + i;
        ui.replace_range(i..j, "");
        let i = ui.find("keys AIM keyboard 2").unwrap();
        ui.truncate(i);
        f
    };
    let m = model(&f);
    let mut app = App::new(&m, 7, (80, 24)).unwrap();
    assert_eq!(app.seat_devices.get(&2), None, "one keyboard part: seat 2 has no device yet");
    app.pad_connected(3);
    assert_eq!(app.seat_devices.get(&2), Some(&Device::Pad(3)));
    // seat 1 waits from the keyboard; the pad is seat 2's
    app.pad(3, PadPress::parse("south").unwrap(), ms(10)).unwrap();
    assert_eq!(app.r.awaiting(), Some(1), "the pad does not play seat 1");
    app.key(key("."), ms(20)).unwrap();
    assert_eq!(app.r.awaiting(), Some(2));
    app.key(key("."), ms(30)).unwrap();
    assert_eq!(app.r.awaiting(), Some(2), "the keyboard does not play the pad's seat");
    app.pad(3, PadPress::parse("south").unwrap(), ms(40)).unwrap();
    assert_eq!(app.r.awaiting(), Some(1), "`south` is WAIT on the pad");
    // a menu belongs to the device that opened it
    app.pad(3, PadPress::parse("start").unwrap(), ms(50)).unwrap();
    assert!(app.menus.is_open(), "`start` opens the pause menu from the pad (ALWAYS)");
    app.key(key("down"), ms(60)).unwrap();
    assert_eq!(app.menus.layers.last().unwrap().cursor, 0, "the keyboard does not drive the pad's menu");
    app.pad(3, PadPress::parse("dpad.down").unwrap(), ms(70)).unwrap();
    assert_eq!(app.menus.layers.last().unwrap().cursor, 1);
    app.pad_disconnected(3);
    assert_eq!(app.seat_devices.get(&2), None);
}

// ---- the mouse (M 8.1, M 8.2) ----

/// The screen cell of a map cell, from the last drawn frame.
fn screen_of(app: &App, x: i32, y: i32) -> (usize, usize) {
    let r = app.view.map_rect.unwrap();
    let map = app.r.w.s.active_map().unwrap();
    let (cx, cy) = app.view.camera[&map];
    (r.x + (x - cx) as usize, r.y + (y - cy) as usize)
}

fn player_at(app: &App) -> (i32, i32) {
    let p = app.r.w.s.at(app.r.w.s.player.unwrap()).unwrap();
    (p.x, p.y)
}

fn click() -> Binding {
    Binding::mouse(termplate_core::keys::Mouse::Click)
}

/// Ticks the clock in 100 ms steps until `until` or `n` ticks.
fn run_ticks(app: &mut App, from: u64, n: u64) -> u64 {
    let mut t = from;
    for _ in 0..n {
        t += 100;
        app.tick(ms(t)).unwrap();
    }
    t
}

#[test]
fn a_click_on_the_map_travels_there() {
    let f = mini_with(|g| g.push_str("\n"));
    let f = {
        let mut f = f;
        let g = f.game.get_mut("game.rlml").unwrap();
        *g = g.replace("- PLAYER HERO\n", "- PLAYER HERO\n- walk STD.MOVE\n");
        f
    };
    let m = model(&f);
    let mut app = App::new(&m, 7, (40, 12)).unwrap();
    app.draw();
    let (x, y) = player_at(&app);
    let (sx, sy) = screen_of(&app, x + 2, y);
    app.mouse_press(click(), sx, sy, ms(10)).unwrap();
    assert!(app.r.travelling(1) || player_at(&app) == (x + 2, y), "the travel started");
    run_ticks(&mut app, 10, 10);
    assert_eq!(player_at(&app), (x + 2, y), "two steps east");
    assert!(!app.r.travelling(1), "arrived");
    assert_eq!(app.r.w.s.game.time, 240, "each step is a turn of its own");
}

#[test]
fn a_key_stops_the_travel_and_does_nothing_else() {
    let mut f = mini_with(|_| {});
    let g = f.game.get_mut("game.rlml").unwrap();
    *g = g.replace("- PLAYER HERO\n", "- PLAYER HERO\n- walk STD.MOVE\n");
    let m = model(&f);
    let mut app = App::new(&m, 7, (40, 12)).unwrap();
    app.draw();
    let (x, y) = player_at(&app);
    let (sx, sy) = screen_of(&app, x, y + 3);
    app.mouse_press(click(), sx, sy, ms(10)).unwrap();
    assert!(app.r.travelling(1), "three steps to go");
    let (_, y1) = player_at(&app);
    let t = app.r.w.s.game.time;
    app.key(key("l"), ms(20)).unwrap();
    assert!(!app.r.travelling(1), "the key stopped it");
    assert_eq!((player_at(&app).1, app.r.w.s.game.time), (y1, t), "and did not move east");
}

#[test]
fn without_walk_a_click_does_nothing() {
    let f = mini_with(|_| {});
    let m = model(&f);
    let mut app = App::new(&m, 7, (40, 12)).unwrap();
    app.draw();
    let (x, y) = player_at(&app);
    let (sx, sy) = screen_of(&app, x + 1, y);
    app.mouse_press(click(), sx, sy, ms(10)).unwrap();
    assert!(!app.r.travelling(1));
    assert_eq!((player_at(&app), app.r.w.s.game.time), ((x, y), 0));
}

#[test]
fn the_hover_describes_what_is_seen() {
    let f = mini_with(|_| {});
    let m = model(&f);
    let mut app = App::new(&m, 7, (40, 12)).unwrap();
    app.draw();
    let view = app.r.w.player_view(0);
    let rats: Vec<_> = app.r.w.s.entities_on(0).into_iter().filter(|&e| app.r.w.s.get(e).kind.ends_with("RAT")).collect();
    let (seen, unseen): (Vec<_>, Vec<_>) = rats.into_iter().map(|e| app.r.w.s.at(e).unwrap()).partition(|p| view.contains(&(p.x, p.y)));
    let (sx, sy) = screen_of(&app, seen[0].x, seen[0].y);
    app.pointer_at(sx, sy);
    app.draw();
    assert_eq!(app.view.hover.as_deref(), Some("Iso harmaa rotta. Se on nälkäinen."), "the rat in the player's room");
    // a rat behind the wall, on a cell nobody has seen: nothing (M 8.1)
    let p = unseen[0];
    assert!(!app.r.w.s.maps[0].remembered.contains(&(p.x, p.y)));
    let (sx, sy) = screen_of(&app, p.x, p.y);
    app.pointer_at(sx, sy);
    app.draw();
    assert_eq!(app.view.hover, None, "an unseen rat gives nothing away");
    app.pointer = None;
    app.draw();
    assert_eq!(app.view.hover, None, "no pointer, no hover");
}

#[test]
fn the_mouse_follows_the_turn() {
    let mut f = ryhma();
    let g = f.game.get_mut("game.rlml").unwrap();
    *g = g.replace("- PLAYER HERO\n", "- PLAYER HERO\n- walk STD.MOVE\n");
    let m = model(&f);
    let mut app = App::new(&m, 7, (80, 24)).unwrap();
    // seat 1 waits; now it is seat 2's turn, and the mouse is seat 2's
    app.key(key("."), ms(10)).unwrap();
    assert_eq!(app.r.awaiting(), Some(2));
    app.draw();
    let b = app.r.w.s.active(2).unwrap();
    let a = app.r.w.s.active(1).unwrap();
    let pa = app.r.w.s.at(a).unwrap();
    let pb = app.r.w.s.at(b).unwrap();
    // one step to a free cell next to seat 2's hero
    let target = [(1, 0), (-1, 0), (0, 1), (0, -1)]
        .into_iter()
        .map(|(dx, dy)| rlml_core::rt::value::Pos { x: pb.x + dx, y: pb.y + dy, ..pb })
        .find(|&p| !app.r.w.blocks_tiles(p, "move", Some(b)) && app.r.w.s.entities_at(p).is_empty())
        .unwrap();
    let (sx, sy) = screen_of(&app, target.x, target.y);
    app.mouse_press(click(), sx, sy, ms(20)).unwrap();
    run_ticks(&mut app, 20, 5);
    assert_eq!(app.r.w.s.at(b), Some(target), "seat 2's hero went");
    assert_eq!(app.r.w.s.at(a), Some(pa), "seat 1's did not");
}

// ---- the staying cursor (M 8.1, M 8.2) ----

fn ryhma_walking() -> MemFiles {
    let mut f = ryhma();
    assert!(f.game["game.rlml"].contains("- walk WALK"), "the group example walks by click");
    f.game.get_mut("game.rlml").unwrap().push('\n');
    f
}

fn seat_at(app: &App, n: u8) -> (i32, i32) {
    let p = app.r.w.s.at(app.r.w.s.active(n).unwrap()).unwrap();
    (p.x, p.y)
}

fn cursor_of(app: &App, n: u8) -> Option<(i32, i32)> {
    app.cursors.get(&n).map(|c| (c.pos.x, c.pos.y))
}

/// Ticks the clock; seat 2 waits (`q`) whenever it has the turn.
fn seat_2_waits_while_ticking(app: &mut App, n: u64) {
    let mut t = 100;
    for _ in 0..n {
        t += 100;
        app.tick(ms(t)).unwrap();
        if app.r.awaiting() == Some(2) && app.players_turn() {
            app.key(key("q"), ms(t)).unwrap();
        }
    }
}

#[test]
fn the_cursor_opens_moves_and_travels_and_stays() {
    let m = model(&ryhma_walking());
    let mut app = App::new(&m, 7, (80, 24)).unwrap();
    let (x, y) = seat_at(&app, 1);
    press(&mut app, "c");
    assert_eq!(cursor_of(&app, 1), Some((x, y)), "it starts at the seat's hero");
    press(&mut app, "j j");
    assert_eq!(cursor_of(&app, 1), Some((x, y + 2)));
    assert_eq!(app.r.w.s.game.time, 0, "moving the cursor takes no time");
    app.draw();
    assert!(app.view.cursors.iter().any(|p| (p.x, p.y) == (x, y + 2)), "the cursor is marked on the map");
    press(&mut app, "enter");
    seat_2_waits_while_ticking(&mut app, 40);
    assert_eq!(seat_at(&app, 1), (x, y + 2), "travelled to the cursor");
    assert_eq!(cursor_of(&app, 1), Some((x, y + 2)), "the cursor stays (M 8.1)");
    press(&mut app, "c");
    assert_eq!(cursor_of(&app, 1), None, "`cursor` closes it");
}

#[test]
fn with_the_cursor_open_other_keys_do_nothing() {
    let m = model(&ryhma_walking());
    let mut app = App::new(&m, 7, (80, 24)).unwrap();
    press(&mut app, "c .");
    assert_eq!((app.r.awaiting(), app.r.w.s.game.time), (Some(1), 0), "`.` did not wait: the cursor's context is modal");
    press(&mut app, "backspace");
    assert_eq!(cursor_of(&app, 1), None, "`back` closes the cursor");
    press(&mut app, ".");
    assert_eq!(app.r.awaiting(), Some(2), "and the game's keys work again");
}

#[test]
fn an_order_on_another_seats_turn_runs_on_the_seats_next_one() {
    let m = model(&ryhma_walking());
    let mut app = App::new(&m, 7, (80, 24)).unwrap();
    let (x, y) = seat_at(&app, 1);
    press(&mut app, ".");
    assert_eq!(app.r.awaiting(), Some(2));
    // seat 1 plans on seat 2's turn
    press(&mut app, "c j j enter");
    assert_eq!(cursor_of(&app, 1), Some((x, y + 2)));
    assert!(app.r.orders.contains_key(&1), "an order, not a travel");
    assert_eq!(seat_at(&app, 1), (x, y), "nothing moved yet");
    app.draw();
    assert!(app.view.cursors.iter().filter(|p| (p.x, p.y) == (x, y + 2)).count() >= 2, "the cursor and the order's target are marked");
    // seat 2 waits; seat 1's order runs
    press(&mut app, "q");
    assert_eq!(app.r.awaiting(), Some(1));
    seat_2_waits_while_ticking(&mut app, 40);
    assert_eq!(seat_at(&app, 1), (x, y + 2), "the order took seat 1 there");
    assert!(!app.r.orders.contains_key(&1));
}

#[test]
fn back_cancels_the_order_first() {
    let m = model(&ryhma_walking());
    let mut app = App::new(&m, 7, (80, 24)).unwrap();
    press(&mut app, ". c j enter");
    assert!(app.r.orders.contains_key(&1));
    press(&mut app, "backspace");
    assert!(!app.r.orders.contains_key(&1), "the order went");
    assert!(cursor_of(&app, 1).is_some(), "the cursor stayed");
    press(&mut app, "backspace");
    assert_eq!(cursor_of(&app, 1), None);
}

#[test]
fn seat_2_moves_its_cursor_from_its_keyboard_part_on_seat_1s_turn() {
    let m = model(&ryhma_walking());
    let mut app = App::new(&m, 7, (80, 24)).unwrap();
    assert_eq!(app.r.awaiting(), Some(1));
    let (x, y) = seat_at(&app, 2);
    press(&mut app, "x d");
    assert_eq!(cursor_of(&app, 2), Some((x + 1, y)), "seat 2's own cursor");
    assert_eq!(cursor_of(&app, 1), None);
    assert_eq!(app.r.awaiting(), Some(1), "seat 1's turn goes on");
    app.draw();
    assert!(app.view.hover.is_some(), "the hover shows the cursor moved last");
    press(&mut app, "x");
    assert_eq!(cursor_of(&app, 2), None);
}

// ---- the verbs menu (M 8.1) ----

/// The screen cell of the docked menu's row `i`.
fn verb_row(app: &App, i: usize) -> (usize, usize) {
    let r = app.view.docked_rects["VERBS"];
    (r.x + 1, r.y + i)
}

fn rclick() -> Binding {
    Binding::mouse(termplate_core::keys::Mouse::RightClick)
}

fn robin(app: &App) -> (usize, rlml_core::rt::value::Pos) {
    let e = app.r.w.s.entities_on(0).into_iter().find(|&e| app.r.w.s.get(e).kind == "ROBIN").unwrap();
    (e, app.r.w.s.at(e).unwrap())
}

fn examined(app: &App) -> usize {
    app.r.w.log.iter().filter(|l| l.key == "MESSAGE.EXAMINE").count()
}

#[test]
fn the_verbs_menu_shows_the_turns_shortcuts() {
    let m = model(&ryhma_walking());
    let mut app = App::new(&m, 7, (80, 24)).unwrap();
    let rows = screen(&mut app);
    let row = |rows: &str, label: &str| rows.lines().find(|l| l.contains(label)).unwrap().trim_end().to_string();
    assert!(row(&rows, "Odota").ends_with('.'), "seat 1 waits with `.`: {rows}");
    assert!(row(&rows, "Vaihda").ends_with("tab"));
    assert!(row(&rows, "Tutki").ends_with('i'), "EXAMINE on `i`");
    assert!(row(&rows, "Lyö").ends_with("Lyö"), "no key for STRIKE_AT");
    press(&mut app, ".");
    assert_eq!(app.r.awaiting(), Some(2));
    let rows = screen(&mut app);
    assert!(row(&rows, "Odota").ends_with('q'), "seat 2's turn: its key `q` (M 8.1)");
}

#[test]
fn a_verb_without_targets_runs_at_once() {
    let m = model(&ryhma_walking());
    let mut app = App::new(&m, 7, (80, 24)).unwrap();
    app.draw();
    let (x, y) = verb_row(&app, 2);
    app.mouse_press(click(), x, y, ms(10)).unwrap();
    assert_eq!(app.r.awaiting(), Some(2), "Odota waited for seat 1");
    assert!(app.verb.is_none());
}

#[test]
fn a_verb_takes_its_target_from_a_click() {
    let m = model(&ryhma_walking());
    let mut app = App::new(&m, 7, (80, 24)).unwrap();
    app.draw();
    let (x, y) = verb_row(&app, 0);
    app.mouse_press(click(), x, y, ms(10)).unwrap();
    assert_eq!(app.verb.as_ref().map(|v| v.command.as_str()), Some("EXAMINE"));
    // the sentence names the target under the pointer
    let (_, rp) = robin(&app);
    let (sx, sy) = screen_of(&app, rp.x, rp.y);
    app.pointer_at(sx, sy);
    app.draw();
    assert_eq!(app.view.sentence.as_deref(), Some("Tutki Robin"));
    assert_eq!(app.view.docked["VERBS"].chosen, Some(0), "the chosen verb is marked");
    // a wall fits no target: the hover says so, and no time goes
    let (wx, wy) = screen_of(&app, 0, 0);
    app.mouse_press(click(), wx, wy, ms(20)).unwrap();
    app.draw();
    assert_eq!(app.view.hover.as_deref(), Some("Siinä ei ole mitään tutkittavaa."));
    assert!(app.verb.is_some());
    // Robin fits
    app.mouse_press(click(), sx, sy, ms(30)).unwrap();
    assert_eq!(examined(&app), 1);
    assert!(app.verb.is_none(), "the command ran");
    assert_eq!(app.r.awaiting(), Some(1), "EXAMINE is free: still seat 1's turn");
    assert_eq!(app.r.w.text("fi", &app.r.w.log.last().unwrap().key, &app.r.w.log.last().unwrap().args), "Robin: Robin, uskollinen seuralainen.");
}

#[test]
fn a_right_click_drops_the_verb() {
    let m = model(&ryhma_walking());
    let mut app = App::new(&m, 7, (80, 24)).unwrap();
    app.draw();
    let (x, y) = verb_row(&app, 0);
    app.mouse_press(click(), x, y, ms(10)).unwrap();
    assert!(app.verb.is_some());
    app.mouse_press(rclick(), x, y, ms(20)).unwrap();
    assert!(app.verb.is_none());
}

#[test]
fn the_verbs_menu_from_the_keys() {
    let m = model(&ryhma_walking());
    let mut app = App::new(&m, 7, (80, 24)).unwrap();
    press(&mut app, "v");
    app.draw();
    assert_eq!(app.view.docked["VERBS"].focused, Some(0), "`focus VERBS` gives the menu the keys");
    press(&mut app, "j k enter");
    assert!(app.focus.is_none(), "choosing gives the keys back");
    // from the keys, EXAMINE asks its target in a list (M 8)
    assert!(app.menus.layers.last().is_some_and(|l| l.menu.full == rlml_app::app::ASK_MENU), "the list of what to examine");
    let (rob, _) = robin(&app);
    let i = match &app.asking.as_ref().unwrap().ui {
        rlml_app::app::AskUi::List { choices, .. } => choices.iter().position(|v| *v == rlml_core::rt::value::Value::Entity(rob)).unwrap(),
        other => panic!("{other:?}"),
    };
    let letter = (b'a' + i as u8) as char;
    press(&mut app, &letter.to_string());
    assert_eq!(examined(&app), 1, "Robin's letter chose Robin");
    assert!(app.asking.is_none() && !app.menus.is_open());
    // `back` in the focused menu gives the keys back without a verb
    press(&mut app, "v backspace");
    assert!(app.focus.is_none() && app.verb.is_none());
}

#[test]
fn the_verb_belongs_to_the_turn() {
    let m = model(&ryhma_walking());
    let mut app = App::new(&m, 7, (80, 24)).unwrap();
    app.draw();
    let (x, y) = verb_row(&app, 0);
    app.mouse_press(click(), x, y, ms(10)).unwrap();
    assert!(app.verb.is_some());
    press(&mut app, ".");
    app.draw();
    assert!(app.verb.is_none(), "seat 1's turn ended");
}

// ---- the verb coin and the mouse in menus (M 8.1) ----

fn coin_open(app: &App) -> bool {
    app.menus.layers.last().is_some_and(|l| l.menu.full == rlml_app::app::COIN_MENU)
}

#[test]
fn a_right_click_opens_the_verbs_that_fit_the_cell() {
    let m = model(&ryhma_walking());
    let mut app = App::new(&m, 7, (80, 24)).unwrap();
    app.draw();
    let (_, rp) = robin(&app);
    let (sx, sy) = screen_of(&app, rp.x, rp.y);
    app.mouse_press(rclick(), sx, sy, ms(10)).unwrap();
    assert!(coin_open(&app), "the coin opened");
    let labels: Vec<String> = app.menus.layers.last().unwrap().menu.items.iter().map(|it| it.label.clone().unwrap()).collect();
    assert_eq!(labels, ["Tutki  i"], "EXAMINE fits Robin, with its key; Robin is no foe, so no Lyö");
    // the coin opens next to the cell
    app.draw();
    let w = app.menu_drawn_window().unwrap();
    assert_eq!((w.x, w.y), (sx + 1, sy), "next to the cell");
    // a click on its row runs the verb on Robin
    let (rx, ry) = (w.x + 2, w.y + 1);
    app.mouse_press(click(), rx, ry, ms(20)).unwrap();
    assert!(!coin_open(&app));
    assert_eq!(examined(&app), 1);
}

#[test]
fn nothing_opens_where_no_verb_fits() {
    let m = model(&ryhma_walking());
    let mut app = App::new(&m, 7, (80, 24)).unwrap();
    app.draw();
    let (sx, sy) = screen_of(&app, 0, 0);
    app.mouse_press(rclick(), sx, sy, ms(10)).unwrap();
    assert!(!app.menus.is_open(), "a wall fits no verb");
}

#[test]
fn the_coin_from_the_look_cursor() {
    let m = model(&ryhma_walking());
    let mut app = App::new(&m, 7, (80, 24)).unwrap();
    let hero = seat_at(&app, 1);
    let (_, rp) = robin(&app);
    let step = if rp.x > hero.0 { "l" } else { "h" };
    press(&mut app, &format!("c {step} v"));
    assert!(coin_open(&app), "`verbs` on the cursor's cell");
    press(&mut app, "enter");
    assert_eq!(examined(&app), 1);
    assert!(cursor_of(&app, 1).is_some(), "the staying cursor stays");
}

#[test]
fn menus_take_the_mouse() {
    let m = model(&ryhma_walking());
    let mut app = App::new(&m, 7, (80, 24)).unwrap();
    press(&mut app, "esc");
    assert!(app.menus.is_open(), "the pause menu");
    app.draw();
    let w = app.menu_drawn_window().unwrap();
    // the pointer highlights the second row
    app.pointer_at(w.x + 2, w.y + 2);
    assert_eq!(app.menus.layers.last().unwrap().cursor, 1);
    // a click outside the window is `back`
    app.mouse_press(click(), 0, 0, ms(10)).unwrap();
    assert!(!app.menus.is_open());
    // a click on a row chooses it: Jatka closes the menu
    press(&mut app, "esc");
    app.draw();
    let w = app.menu_drawn_window().unwrap();
    app.mouse_press(click(), w.x + 2, w.y + 1, ms(20)).unwrap();
    assert!(!app.menus.is_open(), "`resume`");
}

// ---- asking the parameters (M 8) ----

/// The group example with commands that ask, bound to keys of part 1:
/// `i` EXAMINE (from), `p` PEEK (aim 3), `o` SHOVE (a direction), `u`
/// POINT (aim at the other members).
fn ryhma_asking(targeting: &str) -> MemFiles {
    let mut f = ryhma_walking();
    let g = f.game.get_mut("game.rlml").unwrap();
    g.push_str(
        "\n```roguescript\ncommand PEEK(to: pos aim 3) cost 0:\n    log(MESSAGE.EXAMINE, WHAT: actor)\n\ncommand SHOVE(dir: dir ask) cost 0:\n    log(MESSAGE.EXAMINE, WHAT: actor)\n\ncommand POINT(to: pos aim 9 at players where it != actor) cost 0:\n    log(MESSAGE.EXAMINE, WHAT: actor)\n```\n",
    );
    if !targeting.is_empty() {
        *g = g.replace("- round 250ms\n", &format!("- round 250ms\n- targeting {targeting}\n"));
    }
    let ui = f.game.get_mut("ui/main.rlml").unwrap();
    *ui = ui.replace("- cursor c\n", "- cursor c\n- PEEK p\n- SHOVE o\n- POINT u\n");
    f
}

fn asked(app: &App) -> Option<String> {
    app.asking.as_ref().map(|a| match &a.ui {
        rlml_app::app::AskUi::None => "none".to_string(),
        rlml_app::app::AskUi::List { .. } => "list".to_string(),
        rlml_app::app::AskUi::Aim { .. } => "aim".to_string(),
        rlml_app::app::AskUi::Direction { .. } => "direction".to_string(),
    })
}

fn aim_at(app: &App) -> Option<(i32, i32)> {
    match app.asking.as_ref().map(|a| &a.ui) {
        Some(rlml_app::app::AskUi::Aim { pos, .. }) => Some((pos.x, pos.y)),
        _ => None,
    }
}

#[test]
fn a_key_bound_command_asks_its_from_parameter() {
    let m = model(&ryhma_asking(""));
    let mut app = App::new(&m, 7, (80, 24)).unwrap();
    press(&mut app, "i");
    assert_eq!(asked(&app).as_deref(), Some("list"), "a list of what to examine, no runtime error");
    press(&mut app, "backspace");
    assert_eq!(asked(&app), None, "`back` drops the command");
    assert!(!app.menus.is_open());
    assert_eq!((app.r.awaiting(), app.r.w.s.game.time, examined(&app)), (Some(1), 0, 0), "and takes no time");
}

#[test]
fn the_aiming_cursor_stays_in_range() {
    let m = model(&ryhma_asking(""));
    let mut app = App::new(&m, 7, (80, 24)).unwrap();
    let (x, y) = seat_at(&app, 1);
    press(&mut app, "p");
    assert_eq!(aim_at(&app), Some((x, y)), "no candidates: the cursor starts at the player");
    press(&mut app, "j j j j");
    assert!(aim_at(&app).is_some_and(|(_, ay)| ay - y <= 3), "`aim 3`: {:?}", aim_at(&app));
    app.draw();
    assert!(app.view.cursors.iter().any(|p| Some((p.x, p.y)) == aim_at(&app)), "the aim is marked");
    press(&mut app, "enter");
    assert_eq!((asked(&app), examined(&app)), (None, 1));
}

#[test]
fn a_direction_is_one_key() {
    let m = model(&ryhma_asking(""));
    let mut app = App::new(&m, 7, (80, 24)).unwrap();
    press(&mut app, "o");
    assert_eq!(asked(&app).as_deref(), Some("direction"));
    press(&mut app, "l");
    assert_eq!((asked(&app), examined(&app)), (None, 1));
}

#[test]
fn targeting_auto_and_list() {
    let m = model(&ryhma_asking("auto"));
    let mut app = App::new(&m, 7, (80, 24)).unwrap();
    press(&mut app, "u");
    assert_eq!((asked(&app), examined(&app)), (None, 1), "`auto` takes the nearest candidate without asking");
    let m = model(&ryhma_asking("list"));
    let mut app = App::new(&m, 7, (80, 24)).unwrap();
    press(&mut app, "u");
    assert_eq!(asked(&app).as_deref(), Some("list"), "`list` shows the candidates");
    press(&mut app, "enter");
    assert_eq!((asked(&app), examined(&app)), (None, 1));
    // `cursor` starts at the nearest candidate
    let m = model(&ryhma_asking("cursor"));
    let mut app = App::new(&m, 7, (80, 24)).unwrap();
    let (_, rp) = robin(&app);
    press(&mut app, "u");
    assert_eq!(aim_at(&app), Some((rp.x, rp.y)), "Robin is the nearest other member");
}

#[test]
fn the_mouse_aims_and_gives_directions() {
    let m = model(&ryhma_asking(""));
    let mut app = App::new(&m, 7, (80, 24)).unwrap();
    app.draw();
    let (x, y) = seat_at(&app, 1);
    press(&mut app, "p");
    let (sx, sy) = screen_of(&app, x, y + 2);
    app.pointer_at(sx, sy);
    assert_eq!(aim_at(&app), Some((x, y + 2)), "the pointer aims");
    app.mouse_press(click(), sx, sy, ms(10)).unwrap();
    assert_eq!((asked(&app), examined(&app)), (None, 1));
    press(&mut app, "o");
    app.mouse_press(rclick(), sx, sy, ms(20)).unwrap();
    assert_eq!(asked(&app), None, "rclick goes back");
    press(&mut app, "o");
    app.mouse_press(click(), sx, sy, ms(30)).unwrap();
    assert_eq!((asked(&app), examined(&app)), (None, 2), "a click gives the direction towards it");
}

#[test]
fn a_batch_of_events_cuts_pointer_moves_to_their_last_cell() {
    use rlml_app::events::{Event, Feed};
    let f = mini_with(|_| {});
    let m = model(&f);
    let mut app = App::new(&m, 7, (40, 12)).unwrap();
    let mut feed = Feed::default();
    let x0 = player_x(&app);
    let moves = (0..30).map(|i| Event::Move { x: i % 20, y: 3 });
    let resized = feed.apply(&mut app, moves.chain([Event::Key(key("l")), Event::Move { x: 5, y: 6 }, Event::Resize(60, 20)]), ms(10)).unwrap();
    assert!(resized);
    assert_eq!(player_x(&app), x0 + 1, "the key between the moves goes in its place");
    assert_eq!(app.pointer, Some((5, 6)), "the last move before the resize");
    assert_eq!(app.draw().width, 60);
    // the batch ends where the app quits
    let quit = Event::Key(Binding::key(KeyCode::Char('c'), Mods { ctrl: true, ..Mods::default() }));
    assert!(!feed.apply(&mut app, [quit, Event::Resize(30, 10)], ms(20)).unwrap());
    assert!(app.quit);
    assert_eq!(app.draw().width, 60);
}
