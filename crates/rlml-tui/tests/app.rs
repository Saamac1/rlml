//! The game loop with a clock of its own (M 6.1.2, M 7.2, M 8).

use std::path::Path;
use std::time::Duration;

use rlml_core::load::MemFiles;
use rlml_core::model::Model;
use rlml_core::rt::eval::{BodyKind, Env};
use rlml_core::rt::run::Status;
use rlml_tui::app::App;
use termplate_core::keys::{Binding, KeyCode, Mods, parse_binding};

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
    let mut app = App::new(&m, 7, (40, 12)).unwrap();
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
    use rlml_tui::draw::ansi256;
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
    for size in [(20, 12), (80, 24)] {
        let mut app = App::new(&m, 7, size).unwrap();
        let frame = app.draw();
        let r = app.view.map_rect.expect("the map is drawn");
        assert_eq!((r.x, r.y, r.w, r.h), (0, 0, 20, 8), "the written 20x8 in {size:?}: what the leash measures (M 7.1)");
        assert_eq!(app.r.w.view_dims(), (20, 8));
        let rows = frame.rows();
        assert!(rows[..8].iter().all(|row| row.chars().skip(20).all(|c| c == ' ')), "nothing drawn beside the map");
        let (cx, _) = app.view.camera[&0];
        assert!((0..=20).contains(&cx), "the camera has a place on the 40-wide cave: {cx}");
    }
}
