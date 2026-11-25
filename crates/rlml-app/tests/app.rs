//! The game loop with a clock of its own (M 6.1.2, M 7.2, M 8).

use std::path::Path;
use std::time::Duration;

use rlml_app::app::App;
use rlml_core::load::MemFiles;
use rlml_core::model::Model;
use rlml_core::rt::eval::{BodyKind, Env};
use rlml_core::rt::run::Status;
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
            out.insert(
                p.strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/"),
                std::fs::read_to_string(&p).unwrap(),
            );
        }
    }
}

/// The minigame as it is.
fn mini_as_is() -> MemFiles {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut f = MemFiles::default();
    read_tree(&repo, &repo.join("lib"), &mut f.engine);
    let game = repo.join("esimerkit/mini");
    read_tree(&game, &game, &mut f.game);
    f
}

/// The minigame without its phone layout, buttons, `walk` and pause
/// menu's Save, which the tests of those engine parts set up as they
/// need, and `edit` on its game.rlml.
fn mini_with(edit: impl FnOnce(&mut String)) -> MemFiles {
    let mut f = mini_as_is();
    let g = f.game.get_mut("game.rlml").unwrap();
    *g = g.replace(
        "- layout MAIN\n- layout PHONE when portrait and width < 70\n- walk STD.MOVE\n",
        "",
    );
    let ui = f.game.get_mut("ui/main.rlml").unwrap();
    let (start, end) = (
        ui.find("layout PHONE\n").unwrap(),
        ui.find("log LOG history").unwrap(),
    );
    *ui = format!("{}{}", &ui[..start], &ui[end..]).replace("- SAVE save\n", "");
    edit(f.game.get_mut("game.rlml").unwrap());
    f
}

fn model(f: &MemFiles) -> Model {
    let (m, r) = rlml_core::check_game(f, "game.rlml");
    assert!(
        !r.has_errors(),
        "{:?}",
        r.errors().map(|d| d.to_string()).collect::<Vec<_>>()
    );
    m
}

fn key(s: &str) -> Binding {
    parse_binding(s).unwrap()
}

/// The app's input log, written and read back, replays to the same game
/// (M 4.4): what keys, clicks, `idle`, travels and verbs ran is in it.
fn replays_the_same(m: &Model, app: &App) {
    use rlml_core::rt::input::{InputLog, Replay};
    let text = app.r.input_log().write().unwrap();
    let log = InputLog::read(&text).unwrap();
    let r = Replay::run(m, &log).unwrap_or_else(|e| panic!("{e}\n{text}"));
    for (i, (a, b)) in r.w.s.maps.iter().zip(&app.r.w.s.maps).enumerate() {
        assert!(
            a == b,
            "map {i} differs: memory, which routes use, is state (M 3.1)\n{text}"
        );
    }
    assert!(r.w.s == app.r.w.s, "the replayed state differs\n{text}");
    assert_eq!(r.w.rng, app.r.w.rng);
    assert_eq!(r.w.log, app.r.w.log);
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
    assert_eq!(
        frame.rows()[2].chars().nth(10 + (x + 2) as usize),
        Some('@')
    );
    replays_the_same(&m, &app);
}

#[test]
fn without_a_player_rounds_go_at_the_round_pace() {
    let f = mini_with(|_| {});
    let m = model(&f);
    let mut app = App::new(&m, 7, (40, 12)).unwrap();
    let p = app.r.w.s.player.unwrap();
    app.r
        .w
        .destroy(p, &Env::new(BodyKind::Handler, "test", None, "test"))
        .unwrap();
    app.key(key("z"), ms(0)).unwrap();
    assert_eq!(
        app.status,
        Status::NoPlayer { rounds: 1 },
        "`pass` runs the next round at once"
    );
    let turn = app.r.w.s.game.turn();
    app.tick(ms(100)).unwrap();
    assert_eq!(app.r.w.s.game.turn(), turn, "`round 250ms`: not yet");
    app.tick(ms(250)).unwrap();
    assert_eq!(app.r.w.s.game.turn(), turn + 1);
    app.tick(ms(500)).unwrap();
    app.tick(ms(750)).unwrap();
    assert_eq!(
        app.r.w.s.game.turn(),
        turn + 3,
        "a round every 250 ms (M 6.1.2)"
    );
}

#[test]
fn idle_gives_the_turn_away() {
    let f = mini_with(|g| *g = g.replace("- round 250ms", "- round 250ms\n- idle 1s STD.WAIT"));
    let m = model(&f);
    let mut app = App::new(&m, 7, (40, 12)).unwrap();
    app.tick(ms(900)).unwrap();
    assert_eq!(app.r.w.s.game.time, 0);
    app.tick(ms(1000)).unwrap();
    assert_eq!(
        app.r.w.s.game.time, 120,
        "`idle 1s STD.WAIT` waited for the player"
    );
    app.key(key("s"), ms(1500)).unwrap();
    app.tick(ms(2400)).unwrap();
    assert_eq!(app.r.w.s.game.time, 240, "a key starts the time again");
    replays_the_same(&m, &app);
}

#[test]
fn idle_with_the_players_ai() {
    let f = mini_with(|g| *g = g.replace("- round 250ms", "- round 250ms\n- idle 1s ai"));
    let m = model(&f);
    let mut app = App::new(&m, 7, (40, 12)).unwrap();
    app.tick(ms(1000)).unwrap();
    assert!(
        app.r.w.s.game.time > 0,
        "the player's HUNTER chose a command (M 6.1.2)"
    );
    replays_the_same(&m, &app);
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
    assert!(
        !app.frames.is_empty(),
        "the rat moved in sight between the player's turns"
    );
    assert!(
        app.frames.windows(2).all(|w| w[0] != w[1]),
        "only changed moments are frames"
    );
    let g = mini_with(|_| {});
    let m2 = model(&g);
    let mut quiet = App::new(&m2, 7, (40, 12)).unwrap();
    quiet.key(key("s"), ms(0)).unwrap();
    assert!(
        quiet.frames.is_empty(),
        "`frame` 0: only the state at the next input (D 9.3)"
    );
}

#[test]
fn ctrl_c_quits_and_resizing_redraws() {
    let f = mini_with(|_| {});
    let m = model(&f);
    let mut app = App::new(&m, 7, (40, 12)).unwrap();
    app.resize(60, 20);
    assert_eq!((app.draw().width, app.draw().height), (60, 20));
    app.key(
        Binding::key(
            KeyCode::Char('c'),
            Mods {
                ctrl: true,
                ..Mods::default()
            },
        ),
        ms(0),
    )
    .unwrap();
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
    assert!(
        s.contains(" Tauko ") && s.contains("Jatka") && s.contains("Lopeta"),
        "{s}"
    );
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
    let shown: Vec<&str> = layer
        .rows
        .iter()
        .map(|r| layer.menu.items[r.item].key.as_str())
        .collect();
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

/// The group example past its join screen: keyboard part 1 chose the
/// game, part 2 joins with `e`, and `enter` starts it (M 8.2).
fn joined(m: &Model, seed: u64, size: (usize, usize)) -> rlml_core::rt::world::RtResult<App<'_>> {
    let mut app = App::new(m, seed, size)?;
    assert!(app.joining.is_some(), "more than one seat: the join screen");
    app.key(key("e"), ms(0))?;
    app.key(key("enter"), ms(0))?;
    assert!(app.joining.is_none() && app.playing, "both seats joined");
    Ok(app)
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
fn idle_is_each_seats_own() {
    let mut f = ryhma();
    let g = f.game.get_mut("game.rlml").unwrap();
    *g = g.replace("- round 250ms\n", "- round 250ms\n- idle 1s STD.WAIT\n");
    let m = model(&f);
    let mut app = joined(&m, 7, (80, 24)).unwrap();
    assert_eq!(app.r.awaiting(), Some(1));
    // seat 2's press on seat 1's turn does not start seat 1's timer again
    app.key(key("x"), ms(800)).unwrap();
    app.tick(ms(999)).unwrap();
    assert_eq!(app.r.awaiting(), Some(1), "still seat 1's wait");
    app.tick(ms(1000)).unwrap();
    assert_ne!(
        app.r.awaiting(),
        Some(1),
        "`idle` ended seat 1's turn after 1s from its wait (M 6.1.10)"
    );
    // seat 2's wait began at 1000 ms: its own timer
    let n = app.r.awaiting().unwrap();
    assert_eq!(n, 2);
    app.tick(ms(1900)).unwrap();
    assert_eq!(app.r.awaiting(), Some(2), "seat 2 has its own second");
    // seat 2's own press starts its timer again
    app.key(key("x"), ms(1950)).unwrap();
    app.key(key("x"), ms(1960)).unwrap();
    app.tick(ms(2500)).unwrap();
    assert_eq!(
        app.r.awaiting(),
        Some(2),
        "the awaited seat's press starts its timer again"
    );
    app.tick(ms(2960)).unwrap();
    assert_eq!(app.r.awaiting(), Some(1), "and then `idle` ends its turn");
    replays_the_same(&m, &app);
}

#[test]
fn the_turn_marker_flashes_and_blinks() {
    let f = ryhma();
    let m = model(&f);
    let mut app = joined(&m, 7, (40, 14)).unwrap();
    assert!(app.players_turn());
    let p = app.r.w.s.player.unwrap();
    let reversed = |app: &mut App| {
        let frame = app.draw();
        frame
            .cells
            .iter()
            .any(|c| c.ch == '@' && c.style.as_ref().is_some_and(|s| s.attrs.reverse))
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
    assert_eq!(
        ansi256([0xf0, 0xd0, 0x40]),
        221,
        "the example's seat 1 yellow"
    );
    assert_eq!(ansi256([0x60, 0xa0, 0xff]), 75, "and seat 2 blue");
    assert_eq!(
        ansi256([0x4a, 0x4a, 0x5a]),
        239,
        "a grey takes the grey ramp"
    );
}

#[test]
fn the_camera_follows_the_group_and_falls_back_to_the_player() {
    use rlml_core::rt::state::Place;
    use rlml_core::rt::value::Pos;
    let f = ryhma();
    let m = model(&f);
    let mut app = joined(&m, 7, (80, 24)).unwrap();
    let members = app.r.w.s.players();
    let player = app.r.w.s.player.unwrap();
    let place = |app: &mut App, xs: &[i32]| {
        for (&e, &x) in members.iter().zip(xs) {
            app.r.w.s.get_mut(e).place = Place::Map(Pos { map: 0, x, y: 4 });
        }
    };
    // the map shows 20 cells of the 40-wide cave, even in a wide window
    let xs: Vec<i32> = members
        .iter()
        .map(|&e| if e == player { 20 } else { 10 })
        .collect();
    place(&mut app, &xs);
    app.draw();
    assert_eq!(
        app.view.camera[&0].0, 5,
        "the group's box 10..20 in the middle (M 7.1)"
    );
    // spread wider than the area: the camera follows the player
    let xs: Vec<i32> = members
        .iter()
        .map(|&e| if e == player { 30 } else { 2 })
        .collect();
    place(&mut app, &xs);
    app.draw();
    assert_eq!(
        app.view.camera[&0].0, 20,
        "the player at 30, clamped to the map's edge"
    );
    // together again: back to the group
    let xs: Vec<i32> = members
        .iter()
        .map(|&e| if e == player { 14 } else { 12 })
        .collect();
    place(&mut app, &xs);
    app.draw();
    assert_eq!(app.view.camera[&0].0, 3);
}

#[test]
fn the_map_shows_its_written_size_in_a_wide_window() {
    let f = ryhma();
    let m = model(&f);
    for size in [(34, 13), (80, 24)] {
        let mut app = joined(&m, 7, size).unwrap();
        let frame = app.draw();
        let r = app.view.map_rect.expect("the map is drawn");
        assert_eq!(
            (r.x, r.y, r.w, r.h),
            (0, 0, 20, 8),
            "the written 20x8 in {size:?}: what the leash measures (M 7.1)"
        );
        assert_eq!(app.r.w.view_dims(), (20, 8));
        let rows = frame.rows();
        assert!(
            rows[0]
                .chars()
                .skip(20)
                .collect::<String>()
                .starts_with("Tutki"),
            "beside the map is the verbs menu: {:?}",
            rows[0]
        );
        assert!(
            rows[4..8]
                .iter()
                .all(|row| row.chars().skip(20).all(|c| c == ' ')),
            "and below its rows nothing"
        );
        let (cx, cy) = app.view.camera[&0];
        assert!(
            (0..=20).contains(&cx),
            "the camera has a place on the 40-wide cave: {cx}"
        );
        // the map is drawn: the player shows where `cell_at` finds it
        let p = app.r.w.s.at(app.r.w.s.player.unwrap()).unwrap();
        let (sx, sy) = ((p.x - cx) as usize, (p.y - cy) as usize);
        assert_eq!(rows[sy].chars().nth(sx), Some('@'), "{rows:#?}");
        assert_eq!(rlml_app::draw::cell_at(&app.view, 0, sx, sy), Some(p));
        assert_eq!(
            rlml_app::draw::cell_at(&app.view, 0, 30, sy),
            None,
            "beside the map is no map cell"
        );
    }
    // a window too small for the layout draws no map, and keeps no rect
    let mut app = joined(&m, 7, (80, 24)).unwrap();
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
    let mut app = joined(&m, 7, (80, 24)).unwrap();
    assert_eq!(app.seat_devices.get(&1), Some(&Device::Keyboard(1)));
    assert_eq!(
        app.seat_devices.get(&2),
        Some(&Device::Keyboard(2)),
        "part 2 joined with `e` (M 8.2)"
    );
    assert_eq!(app.r.awaiting(), Some(1));
    // seat 2's key on seat 1's turn is dropped
    app.key(key("."), ms(10)).unwrap_or(());
    let t = app.r.w.s.game.time;
    assert_eq!(app.r.awaiting(), Some(2), "seat 1 waited with `.`");
    app.key(key("."), ms(20)).unwrap();
    assert_eq!(
        (app.r.awaiting(), app.r.w.s.game.time),
        (Some(2), t),
        "`.` is part 1's: dropped on seat 2's turn"
    );
    app.key(key("q"), ms(30)).unwrap();
    assert_eq!(app.r.awaiting(), Some(1), "`q` is part 2's: seat 2 waited");
    // the join screen chose the devices: a pad that connects takes no seat
    app.pad(0, PadPress::parse("south").unwrap(), ms(40))
        .unwrap();
    assert_eq!(app.r.awaiting(), Some(1), "a free pad's press is dropped");
    assert_eq!(app.seat_devices.get(&2), Some(&Device::Keyboard(2)));
    app.pad_disconnected(0);
    assert_eq!(app.seat_devices.get(&2), Some(&Device::Keyboard(2)));
    replays_the_same(&m, &app);
}

#[test]
fn a_pad_joins_on_the_join_screen() {
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
    // one keyboard part: the pad joins on the join screen with `south`,
    // and the keyboard starts the game (M 8.2)
    app.pad(3, PadPress::parse("south").unwrap(), ms(0))
        .unwrap();
    app.key(key("enter"), ms(0)).unwrap();
    assert!(app.playing);
    assert_eq!(app.seat_devices.get(&2), Some(&Device::Pad(3)));
    // seat 1 waits from the keyboard; the pad is seat 2's
    app.pad(3, PadPress::parse("south").unwrap(), ms(10))
        .unwrap();
    assert_eq!(app.r.awaiting(), Some(1), "the pad does not play seat 1");
    app.key(key("."), ms(20)).unwrap();
    assert_eq!(app.r.awaiting(), Some(2));
    app.key(key("."), ms(30)).unwrap();
    assert_eq!(
        app.r.awaiting(),
        Some(2),
        "the keyboard does not play the pad's seat"
    );
    app.pad(3, PadPress::parse("south").unwrap(), ms(40))
        .unwrap();
    assert_eq!(app.r.awaiting(), Some(1), "`south` is WAIT on the pad");
    // a menu belongs to the device that opened it
    app.pad(3, PadPress::parse("start").unwrap(), ms(50))
        .unwrap();
    assert!(
        app.menus.is_open(),
        "`start` opens the pause menu from the pad (ALWAYS)"
    );
    app.key(key("down"), ms(60)).unwrap();
    assert_eq!(
        app.menus.layers.last().unwrap().cursor,
        0,
        "the keyboard does not drive the pad's menu"
    );
    app.pad(3, PadPress::parse("dpad.down").unwrap(), ms(70))
        .unwrap();
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
    assert!(
        app.r.travelling(1) || player_at(&app) == (x + 2, y),
        "the travel started"
    );
    run_ticks(&mut app, 10, 10);
    assert_eq!(player_at(&app), (x + 2, y), "two steps east");
    assert!(!app.r.travelling(1), "arrived");
    assert_eq!(app.r.w.s.game.time, 240, "each step is a turn of its own");
    replays_the_same(&m, &app);
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
    assert_eq!(
        (player_at(&app).1, app.r.w.s.game.time),
        (y1, t),
        "and did not move east"
    );
    replays_the_same(&m, &app);
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
    let rats: Vec<_> = app
        .r
        .w
        .s
        .entities_on(0)
        .into_iter()
        .filter(|&e| app.r.w.s.get(e).kind.ends_with("RAT"))
        .collect();
    let (seen, unseen): (Vec<_>, Vec<_>) = rats
        .into_iter()
        .map(|e| app.r.w.s.at(e).unwrap())
        .partition(|p| view.contains(&(p.x, p.y)));
    let (sx, sy) = screen_of(&app, seen[0].x, seen[0].y);
    app.pointer_at(sx, sy);
    app.draw();
    assert_eq!(
        app.view.hover.as_deref(),
        Some("Iso harmaa rotta. Se on nälkäinen."),
        "the rat in the player's room"
    );
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
    let mut app = joined(&m, 7, (80, 24)).unwrap();
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
        .map(|(dx, dy)| rlml_core::rt::value::Pos {
            x: pb.x + dx,
            y: pb.y + dy,
            ..pb
        })
        .find(|&p| !app.r.w.blocks_tiles(p, "move", Some(b)) && app.r.w.s.entities_at(p).is_empty())
        .unwrap();
    let (sx, sy) = screen_of(&app, target.x, target.y);
    app.mouse_press(click(), sx, sy, ms(20)).unwrap();
    run_ticks(&mut app, 20, 5);
    assert_eq!(app.r.w.s.at(b), Some(target), "seat 2's hero went");
    assert_eq!(app.r.w.s.at(a), Some(pa), "seat 1's did not");
    replays_the_same(&m, &app);
}

// ---- the staying cursor (M 8.1, M 8.2) ----

fn ryhma_walking() -> MemFiles {
    let mut f = ryhma();
    assert!(
        f.game["game.rlml"].contains("- walk WALK"),
        "the group example walks by click"
    );
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
    let mut app = joined(&m, 7, (80, 24)).unwrap();
    let (x, y) = seat_at(&app, 1);
    press(&mut app, "c");
    assert_eq!(
        cursor_of(&app, 1),
        Some((x, y)),
        "it starts at the seat's hero"
    );
    press(&mut app, "j j");
    assert_eq!(cursor_of(&app, 1), Some((x, y + 2)));
    assert_eq!(app.r.w.s.game.time, 0, "moving the cursor takes no time");
    app.draw();
    assert!(
        app.view.cursors.iter().any(|p| (p.x, p.y) == (x, y + 2)),
        "the cursor is marked on the map"
    );
    press(&mut app, "enter");
    seat_2_waits_while_ticking(&mut app, 40);
    assert_eq!(seat_at(&app, 1), (x, y + 2), "travelled to the cursor");
    assert_eq!(
        cursor_of(&app, 1),
        Some((x, y + 2)),
        "the cursor stays (M 8.1)"
    );
    press(&mut app, "c");
    assert_eq!(cursor_of(&app, 1), None, "`cursor` closes it");
    replays_the_same(&m, &app);
}

#[test]
fn with_the_cursor_open_other_keys_do_nothing() {
    let m = model(&ryhma_walking());
    let mut app = joined(&m, 7, (80, 24)).unwrap();
    press(&mut app, "c .");
    assert_eq!(
        (app.r.awaiting(), app.r.w.s.game.time),
        (Some(1), 0),
        "`.` did not wait: the cursor's context is modal"
    );
    press(&mut app, "backspace");
    assert_eq!(cursor_of(&app, 1), None, "`back` closes the cursor");
    press(&mut app, ".");
    assert_eq!(app.r.awaiting(), Some(2), "and the game's keys work again");
}

#[test]
fn an_order_on_another_seats_turn_runs_on_the_seats_next_one() {
    let m = model(&ryhma_walking());
    let mut app = joined(&m, 7, (80, 24)).unwrap();
    let (x, y) = seat_at(&app, 1);
    press(&mut app, ".");
    assert_eq!(app.r.awaiting(), Some(2));
    // seat 1 plans on seat 2's turn
    press(&mut app, "c j j enter");
    assert_eq!(cursor_of(&app, 1), Some((x, y + 2)));
    assert!(app.r.orders.contains_key(&1), "an order, not a travel");
    assert_eq!(seat_at(&app, 1), (x, y), "nothing moved yet");
    app.draw();
    assert!(
        app.view
            .cursors
            .iter()
            .filter(|p| (p.x, p.y) == (x, y + 2))
            .count()
            >= 2,
        "the cursor and the order's target are marked"
    );
    // seat 2 waits; seat 1's order runs
    press(&mut app, "q");
    assert_eq!(app.r.awaiting(), Some(1));
    seat_2_waits_while_ticking(&mut app, 40);
    assert_eq!(seat_at(&app, 1), (x, y + 2), "the order took seat 1 there");
    assert!(!app.r.orders.contains_key(&1));
    replays_the_same(&m, &app);
}

#[test]
fn back_cancels_the_order_first() {
    let m = model(&ryhma_walking());
    let mut app = joined(&m, 7, (80, 24)).unwrap();
    press(&mut app, ". c j enter");
    assert!(app.r.orders.contains_key(&1));
    press(&mut app, "backspace");
    assert!(!app.r.orders.contains_key(&1), "the order went");
    assert!(cursor_of(&app, 1).is_some(), "the cursor stayed");
    press(&mut app, "backspace");
    assert_eq!(cursor_of(&app, 1), None);
    replays_the_same(&m, &app);
}

#[test]
fn seat_2_moves_its_cursor_from_its_keyboard_part_on_seat_1s_turn() {
    let m = model(&ryhma_walking());
    let mut app = joined(&m, 7, (80, 24)).unwrap();
    assert_eq!(app.r.awaiting(), Some(1));
    let (x, y) = seat_at(&app, 2);
    press(&mut app, "x d");
    assert_eq!(cursor_of(&app, 2), Some((x + 1, y)), "seat 2's own cursor");
    assert_eq!(cursor_of(&app, 1), None);
    assert_eq!(app.r.awaiting(), Some(1), "seat 1's turn goes on");
    app.draw();
    assert!(
        app.view.hover.is_some(),
        "the hover shows the cursor moved last"
    );
    press(&mut app, "x");
    assert_eq!(cursor_of(&app, 2), None);
    replays_the_same(&m, &app);
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
    let e = app
        .r
        .w
        .s
        .entities_on(0)
        .into_iter()
        .find(|&e| app.r.w.s.get(e).kind == "ROBIN")
        .unwrap();
    (e, app.r.w.s.at(e).unwrap())
}

fn examined(app: &App) -> usize {
    app.r
        .w
        .log
        .iter()
        .filter(|l| l.key == "MESSAGE.EXAMINE")
        .count()
}

#[test]
fn the_verbs_menu_shows_the_turns_shortcuts() {
    let m = model(&ryhma_walking());
    let mut app = joined(&m, 7, (80, 24)).unwrap();
    let rows = screen(&mut app);
    let row = |rows: &str, label: &str| {
        rows.lines()
            .find(|l| l.contains(label))
            .unwrap()
            .trim_end()
            .to_string()
    };
    assert!(
        row(&rows, "Odota").ends_with('.'),
        "seat 1 waits with `.`: {rows}"
    );
    assert!(row(&rows, "Vaihda").ends_with("tab"));
    assert!(row(&rows, "Tutki").ends_with('i'), "EXAMINE on `i`");
    assert!(row(&rows, "Lyö").ends_with("Lyö"), "no key for STRIKE_AT");
    press(&mut app, ".");
    assert_eq!(app.r.awaiting(), Some(2));
    let rows = screen(&mut app);
    assert!(
        row(&rows, "Odota").ends_with('q'),
        "seat 2's turn: its key `q` (M 8.1)"
    );
}

#[test]
fn a_verb_without_targets_runs_at_once() {
    let m = model(&ryhma_walking());
    let mut app = joined(&m, 7, (80, 24)).unwrap();
    app.draw();
    let (x, y) = verb_row(&app, 2);
    app.mouse_press(click(), x, y, ms(10)).unwrap();
    assert_eq!(app.r.awaiting(), Some(2), "Odota waited for seat 1");
    assert!(app.verb.is_none());
    replays_the_same(&m, &app);
}

#[test]
fn a_verb_takes_its_target_from_a_click() {
    let m = model(&ryhma_walking());
    let mut app = joined(&m, 7, (80, 24)).unwrap();
    app.draw();
    let (x, y) = verb_row(&app, 0);
    app.mouse_press(click(), x, y, ms(10)).unwrap();
    assert_eq!(
        app.verb.as_ref().map(|v| v.command.as_str()),
        Some("EXAMINE")
    );
    // the sentence names the target under the pointer
    let (_, rp) = robin(&app);
    let (sx, sy) = screen_of(&app, rp.x, rp.y);
    app.pointer_at(sx, sy);
    app.draw();
    assert_eq!(app.view.sentence.as_deref(), Some("Tutki Robin"));
    assert_eq!(
        app.view.docked["VERBS"].chosen,
        Some(0),
        "the chosen verb is marked"
    );
    // a wall fits no target: the hover says so, and no time goes
    let (wx, wy) = screen_of(&app, 0, 0);
    app.mouse_press(click(), wx, wy, ms(20)).unwrap();
    app.draw();
    assert_eq!(
        app.view.hover.as_deref(),
        Some("Siinä ei ole mitään tutkittavaa.")
    );
    assert!(app.verb.is_some());
    // Robin fits
    app.mouse_press(click(), sx, sy, ms(30)).unwrap();
    assert_eq!(examined(&app), 1);
    assert!(app.verb.is_none(), "the command ran");
    assert_eq!(
        app.r.awaiting(),
        Some(1),
        "EXAMINE is free: still seat 1's turn"
    );
    assert_eq!(
        app.r.w.text(
            "fi",
            &app.r.w.log.last().unwrap().key,
            &app.r.w.log.last().unwrap().args
        ),
        "Robin: Robin, uskollinen seuralainen."
    );
    replays_the_same(&m, &app);
}

#[test]
fn a_right_click_drops_the_verb() {
    let m = model(&ryhma_walking());
    let mut app = joined(&m, 7, (80, 24)).unwrap();
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
    let mut app = joined(&m, 7, (80, 24)).unwrap();
    press(&mut app, "v");
    app.draw();
    assert_eq!(
        app.view.docked["VERBS"].focused,
        Some(0),
        "`focus VERBS` gives the menu the keys"
    );
    press(&mut app, "j k enter");
    assert!(app.focus.is_none(), "choosing gives the keys back");
    // from the keys, EXAMINE asks its target in a list (M 8)
    assert!(
        app.menus
            .layers
            .last()
            .is_some_and(|l| l.menu.full == rlml_app::app::ASK_MENU),
        "the list of what to examine"
    );
    let (rob, _) = robin(&app);
    let i = match &app.asking.as_ref().unwrap().ui {
        rlml_app::app::AskUi::List { choices, .. } => choices
            .iter()
            .position(|v| *v == rlml_core::rt::value::Value::Entity(rob))
            .unwrap(),
        other => panic!("{other:?}"),
    };
    let letter = (b'a' + i as u8) as char;
    press(&mut app, &letter.to_string());
    assert_eq!(examined(&app), 1, "Robin's letter chose Robin");
    assert!(app.asking.is_none() && !app.menus.is_open());
    // `back` in the focused menu gives the keys back without a verb
    press(&mut app, "v backspace");
    assert!(app.focus.is_none() && app.verb.is_none());
    replays_the_same(&m, &app);
}

#[test]
fn the_verb_belongs_to_the_turn() {
    let m = model(&ryhma_walking());
    let mut app = joined(&m, 7, (80, 24)).unwrap();
    app.draw();
    let (x, y) = verb_row(&app, 0);
    app.mouse_press(click(), x, y, ms(10)).unwrap();
    assert!(app.verb.is_some());
    press(&mut app, ".");
    app.draw();
    assert!(app.verb.is_none(), "seat 1's turn ended");
    replays_the_same(&m, &app);
}

// ---- the verb coin and the mouse in menus (M 8.1) ----

fn coin_open(app: &App) -> bool {
    app.menus
        .layers
        .last()
        .is_some_and(|l| l.menu.full == rlml_app::app::COIN_MENU)
}

#[test]
fn a_right_click_opens_the_verbs_that_fit_the_cell() {
    let m = model(&ryhma_walking());
    let mut app = joined(&m, 7, (80, 24)).unwrap();
    app.draw();
    let (_, rp) = robin(&app);
    let (sx, sy) = screen_of(&app, rp.x, rp.y);
    app.mouse_press(rclick(), sx, sy, ms(10)).unwrap();
    assert!(coin_open(&app), "the coin opened");
    let labels: Vec<String> = app
        .menus
        .layers
        .last()
        .unwrap()
        .menu
        .items
        .iter()
        .map(|it| it.label.clone().unwrap())
        .collect();
    assert_eq!(
        labels,
        ["Tutki  i"],
        "EXAMINE fits Robin, with its key; Robin is no foe, so no Lyö"
    );
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
    let mut app = joined(&m, 7, (80, 24)).unwrap();
    app.draw();
    let (sx, sy) = screen_of(&app, 0, 0);
    app.mouse_press(rclick(), sx, sy, ms(10)).unwrap();
    assert!(!app.menus.is_open(), "a wall fits no verb");
}

#[test]
fn the_coin_from_the_look_cursor() {
    let m = model(&ryhma_walking());
    let mut app = joined(&m, 7, (80, 24)).unwrap();
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
    let mut app = joined(&m, 7, (80, 24)).unwrap();
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
        *g = g.replace(
            "- round 250ms\n",
            &format!("- round 250ms\n- targeting {targeting}\n"),
        );
    }
    let ui = f.game.get_mut("ui/main.rlml").unwrap();
    *ui = ui.replace(
        "- cursor c\n",
        "- cursor c\n- PEEK p\n- SHOVE o\n- POINT u\n",
    );
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
    let mut app = joined(&m, 7, (80, 24)).unwrap();
    press(&mut app, "i");
    assert_eq!(
        asked(&app).as_deref(),
        Some("list"),
        "a list of what to examine, no runtime error"
    );
    press(&mut app, "backspace");
    assert_eq!(asked(&app), None, "`back` drops the command");
    assert!(!app.menus.is_open());
    assert_eq!(
        (app.r.awaiting(), app.r.w.s.game.time, examined(&app)),
        (Some(1), 0, 0),
        "and takes no time"
    );
    replays_the_same(&m, &app);
}

#[test]
fn the_aiming_cursor_stays_in_range() {
    let m = model(&ryhma_asking(""));
    let mut app = joined(&m, 7, (80, 24)).unwrap();
    let (x, y) = seat_at(&app, 1);
    press(&mut app, "p");
    assert_eq!(
        aim_at(&app),
        Some((x, y)),
        "no candidates: the cursor starts at the player"
    );
    press(&mut app, "j j j j");
    assert!(
        aim_at(&app).is_some_and(|(_, ay)| ay - y <= 3),
        "`aim 3`: {:?}",
        aim_at(&app)
    );
    app.draw();
    assert!(
        app.view
            .cursors
            .iter()
            .any(|p| Some((p.x, p.y)) == aim_at(&app)),
        "the aim is marked"
    );
    press(&mut app, "enter");
    assert_eq!((asked(&app), examined(&app)), (None, 1));
    replays_the_same(&m, &app);
}

#[test]
fn a_direction_is_one_key() {
    let m = model(&ryhma_asking(""));
    let mut app = joined(&m, 7, (80, 24)).unwrap();
    press(&mut app, "o");
    assert_eq!(asked(&app).as_deref(), Some("direction"));
    press(&mut app, "l");
    assert_eq!((asked(&app), examined(&app)), (None, 1));
    replays_the_same(&m, &app);
}

#[test]
fn targeting_auto_and_list() {
    let m = model(&ryhma_asking("auto"));
    let mut app = joined(&m, 7, (80, 24)).unwrap();
    press(&mut app, "u");
    assert_eq!(
        (asked(&app), examined(&app)),
        (None, 1),
        "`auto` takes the nearest candidate without asking"
    );
    let m = model(&ryhma_asking("list"));
    let mut app = joined(&m, 7, (80, 24)).unwrap();
    press(&mut app, "u");
    assert_eq!(
        asked(&app).as_deref(),
        Some("list"),
        "`list` shows the candidates"
    );
    press(&mut app, "enter");
    assert_eq!((asked(&app), examined(&app)), (None, 1));
    // `cursor` starts at the nearest candidate
    let m = model(&ryhma_asking("cursor"));
    let mut app = joined(&m, 7, (80, 24)).unwrap();
    let (_, rp) = robin(&app);
    press(&mut app, "u");
    assert_eq!(
        aim_at(&app),
        Some((rp.x, rp.y)),
        "Robin is the nearest other member"
    );
    replays_the_same(&m, &app);
}

#[test]
fn the_mouse_aims_and_gives_directions() {
    let m = model(&ryhma_asking(""));
    let mut app = joined(&m, 7, (80, 24)).unwrap();
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
    assert_eq!(
        (asked(&app), examined(&app)),
        (None, 2),
        "a click gives the direction towards it"
    );
    replays_the_same(&m, &app);
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
    let resized = feed
        .apply(
            &mut app,
            moves.chain([
                Event::Key(key("l")),
                Event::Move { x: 5, y: 6 },
                Event::Resize(60, 20),
            ]),
            ms(10),
        )
        .unwrap();
    assert!(resized);
    assert_eq!(
        player_x(&app),
        x0 + 1,
        "the key between the moves goes in its place"
    );
    assert_eq!(app.pointer, Some((5, 6)), "the last move before the resize");
    assert_eq!(app.draw().width, 60);
    // the batch ends where the app quits
    let quit = Event::Key(Binding::key(
        KeyCode::Char('c'),
        Mods {
            ctrl: true,
            ..Mods::default()
        },
    ));
    assert!(
        !feed
            .apply(&mut app, [quit, Event::Resize(30, 10)], ms(20))
            .unwrap()
    );
    assert!(app.quit);
    assert_eq!(app.draw().width, 60);
}

#[test]
fn the_fog_dims_remembered_cells_out_of_view() {
    // a remembered floor cell out of view, drawn without and with the veil
    let looks = |layers: &str| {
        let layers = layers.to_string();
        let f = mini_with(move |g| g.push_str(&layers));
        let m = model(&f);
        let mut app = App::new(&m, 7, (40, 12)).unwrap();
        let map = app.r.w.s.active_map().unwrap();
        let view = app.r.w.player_view(map);
        let m_ = &app.r.w.s.maps[map];
        let cell = (0..m_.height)
            .flat_map(|y| (0..m_.width).map(move |x| (x, y)))
            .find(|&(x, y)| {
                !view.contains(&(x, y)) && m_.tile(0, x, y).is_some_and(|t| t.ends_with("FLOOR"))
            })
            .unwrap();
        app.r.w.s.maps[map].remembered.push(cell);
        let seen = *view
            .iter()
            .find(|&&(x, y)| {
                app.r.w.s.maps[map]
                    .tile(0, x, y)
                    .is_some_and(|t| t.ends_with("FLOOR"))
                    && app
                        .r
                        .w
                        .s
                        .entities_at(rlml_core::rt::value::Pos { map, x, y })
                        .is_empty()
            })
            .unwrap();
        let cells = app.r.w.render_cells(map);
        (
            cells[cell.1 as usize][cell.0 as usize],
            cells[seen.1 as usize][seen.0 as usize],
        )
    };
    let (plain, seen) = looks("\nlayers\n- GROUND map remember\n- THINGS entities\n- FOG fog\n");
    let (veiled, seen_veiled) =
        looks("\nlayers\n- GROUND map remember\n- THINGS entities\n- FOG fog alpha 50%\n");
    assert_eq!(
        plain.fg,
        Some([0x4a, 0x4a, 0x5a]),
        "the floor's own color without alpha"
    );
    assert_eq!(veiled.fg, Some([0x25, 0x25, 0x2d]), "half way to black");
    assert_eq!(seen, seen_veiled, "what is in view is not dimmed");
    let (toward, _) = looks(
        "\nlayers\n- GROUND map remember\n- THINGS entities\n- FOG fog alpha 100% color 102030\n",
    );
    assert_eq!(
        (toward.fg, toward.bg),
        (Some([0x10, 0x20, 0x30]), Some([0x10, 0x20, 0x30]))
    );
}

#[test]
fn a_log_fades_from_the_newest_message() {
    let f =
        mini_with(|g| g.push_str("\npanels\n- LOG fade 60%\n\nlogs\n- STD.MESSAGE fg ffffff\n"));
    let m = model(&f);
    let mut app = App::new(&m, 7, (40, 12)).unwrap();
    app.view.truecolor = true;
    for _ in 0..3 {
        app.r.w.log.push(rlml_core::rt::world::LogEntry {
            key: "STD.MESSAGE.COMBAT.MISS".into(),
            args: vec![],
            speech: false,
        });
    }
    let frame = app.draw();
    let fg = |y: usize| frame.get(0, y).and_then(|c| c.style).and_then(|s| s.fg);
    // the log is the last 3 rows: oldest, middle, newest
    assert_eq!(
        fg(9),
        Some(termplate_core::style::Color::Rgb(0x66, 0x66, 0x66)),
        "the oldest shown at 60%"
    );
    assert_eq!(
        fg(10),
        Some(termplate_core::style::Color::Rgb(0xb3, 0xb3, 0xb3)),
        "the middle at 30%"
    );
    assert_eq!(
        fg(11),
        Some(termplate_core::style::Color::Rgb(0xff, 0xff, 0xff)),
        "the newest as it is"
    );
}

#[test]
fn a_docked_menu_shows_the_keys_of_engine_actions_and_menus() {
    let f = mini_with(|g| {
        *g = g.replace("- keys GAME\n", "- keys GAME\n- layout WITH_KEYS\n");
        g.push_str("\nlayout WITH_KEYS\n\n```\nmk\nll\n```\n\nlegend:\n- m map\n- k menu KEYS size.width 14\n- l LOG size.height 3\n\nmenu KEYS\n- WAIT\n- LOOK look\n- PAUSE open PAUSE\n\nkeys GAME\n- look e\n");
    });
    let m = model(&f);
    let mut app = App::new(&m, 7, (40, 12)).unwrap();
    app.draw();
    let keys: Vec<String> = app
        .view
        .docked
        .values()
        .next()
        .unwrap()
        .rows
        .iter()
        .map(|r| r.key.clone())
        .collect();
    assert_eq!(keys, ["s", "e", "esc"]);
}

#[test]
fn the_log_is_read_back_to_its_start() {
    let f = mini_with(|g| g.push_str("\nkeys GAME\n- read LOG o\n"));
    let f = {
        let mut f = f;
        let ui = f.game.get_mut("ui/main.rlml").unwrap();
        *ui = ui.replace("log LOG history 50", "log LOG history all");
        f
    };
    let m = model(&f);
    let mut app = App::new(&m, 7, (40, 12)).unwrap();
    for n in 0..30 {
        app.r.w.log.push(rlml_core::rt::world::LogEntry {
            key: format!("STD.MESSAGE.COMBAT.N{n:02}"),
            args: vec![],
            speech: false,
        });
    }
    let row = |app: &mut App, y: usize| app.draw().row(y).trim_end().to_string();
    // a row's message without a scroll button's label beside it
    let msg = |app: &mut App, y: usize| {
        app.draw()
            .row(y)
            .split_whitespace()
            .next()
            .unwrap_or("")
            .to_string()
    };
    press(&mut app, "o");
    assert!(app.reading.is_some());
    assert_eq!(
        row(&mut app, 11),
        "STD.MESSAGE.COMBAT.N29",
        "the newest at the bottom"
    );
    // older messages are hidden above: the up button, and none down
    let (up, down) = app.reading.as_ref().unwrap().arrows;
    assert!(down.is_none());
    let up = up.expect("older above");
    app.mouse_press(click(), up.x, up.y, ms(15)).unwrap();
    // an empty line between messages: message k is line 2k
    assert_eq!(
        row(&mut app, 11),
        "STD.MESSAGE.COMBAT.N26",
        "half a view up"
    );
    assert!(
        app.reading.as_ref().unwrap().arrows.1.is_some(),
        "newer below now"
    );
    press(&mut app, "end");
    assert_eq!(row(&mut app, 0), "", "the gap above the oldest shown");
    assert_eq!(
        msg(&mut app, 1),
        "STD.MESSAGE.COMBAT.N24",
        "the whole window is the log"
    );
    press(&mut app, "k");
    assert_eq!(msg(&mut app, 10), "STD.MESSAGE.COMBAT.N28");
    press(&mut app, "pageup");
    assert_eq!(
        msg(&mut app, 10),
        "STD.MESSAGE.COMBAT.N22",
        "a page is the view's height"
    );
    press(&mut app, "home");
    assert_eq!(
        row(&mut app, 0),
        "STD.MESSAGE.COMBAT.N00",
        "back to the first message"
    );
    press(&mut app, "k");
    assert_eq!(row(&mut app, 0), "STD.MESSAGE.COMBAT.N00", "not past it");
    app.mouse_press(
        Binding::mouse(termplate_core::keys::Mouse::WheelDown),
        5,
        5,
        ms(20),
    )
    .unwrap();
    assert_eq!(
        msg(&mut app, 1),
        "STD.MESSAGE.COMBAT.N02",
        "the wheel scrolls three lines"
    );
    press(&mut app, "end");
    assert_eq!(row(&mut app, 11), "STD.MESSAGE.COMBAT.N29");
    // the game's time stops while reading, and esc closes
    let turn = app.r.w.s.game.turn();
    app.tick(ms(5000)).unwrap();
    assert_eq!(app.r.w.s.game.turn(), turn);
    press(&mut app, "esc");
    assert!(app.reading.is_none());
}

/// The minigame with a narrow portrait layout: the map over a tabs area
/// with a pad panel and the log.
fn mini_phone() -> MemFiles {
    mini_with(|g| {
        *g = g.replace(
            "- keys GAME\n",
            "- keys GAME\n- layout MAIN\n- layout PHONE when portrait and width < 50\n",
        );
        g.push_str(
            "\nlayout PHONE\n\n```\nm\nt\n```\n\nlegend:\n- m map\n- t tabs PAD LOG size.height 5\n\n\
             panel PAD\n\n```\n nnnnn\n sssss\n ddddd\n```\n\nlegend:\n- n button MOVE.NORTH text PAD.N\n- s button MOVE.SOUTH text PAD.S\n- d button tab LOG text PAD.L\n\nkeys GAME\n- tab.next t\n",
        );
    })
}

/// The middle cell of the button whose action is `command`.
fn button_of(app: &App, command: &str) -> (usize, usize) {
    let (r, _) = app
        .view
        .buttons
        .iter()
        .find(|(_, a)| matches!(a, rlml_app::menu::ItemAction::Command(c) if c.ends_with(command)))
        .expect(command);
    (r.x + r.w / 2, r.y + r.h / 2)
}

/// The middle cell of tab `i`'s title.
fn tab_title(app: &App, i: usize) -> (usize, usize) {
    let (_, _, r) = app.view.tab_rects.iter().find(|(_, n, _)| *n == i).unwrap();
    (r.x + r.w / 2, r.y)
}

#[test]
fn the_window_chooses_the_layout() {
    let m = model(&mini_phone());
    let mut app = App::new(&m, 7, (40, 12)).unwrap();
    // 40×12 cells is landscape at 1:2 cells: the default, with its log rows
    app.draw();
    assert!(app.view.tab_rects.is_empty(), "MAIN has no tabs");
    // 30×30 is portrait and narrow: the tabs at the bottom, evenly wide, the
    // pad first and centred across
    app.resize(30, 30);
    let f = app.draw();
    assert_eq!(
        f.row(25).trim_end(),
        "      PAD            LOG",
        "{:#?}",
        f.rows()
    );
    assert_eq!(f.row(26).trim(), "PAD.N");
    // the same cells at a wide window's pixels are landscape again
    app.pixels = Some((900, 600));
    app.draw();
    assert!(app.view.tab_rects.is_empty());
}

#[test]
fn a_button_acts_and_held_it_repeats() {
    let m = model(&mini_phone());
    let mut app = App::new(&m, 7, (30, 30)).unwrap();
    app.draw();
    let (x, y) = player_at(&app);
    // the north button walks a step
    let (bx, by) = button_of(&app, "MOVE.NORTH");
    app.mouse_press(click(), bx, by, ms(10)).unwrap();
    assert_eq!(player_at(&app), (x, y - 1));
    // held, the south button repeats after its delay until it is let go
    app.releases = true;
    app.draw();
    let (bx, by) = button_of(&app, "MOVE.SOUTH");
    app.mouse_press(click(), bx, by, ms(100)).unwrap();
    assert_eq!(player_at(&app), (x, y));
    app.tick(ms(300)).unwrap();
    assert_eq!(player_at(&app), (x, y), "not before the delay");
    app.tick(ms(460)).unwrap();
    assert_eq!(player_at(&app), (x, y + 1));
    app.tick(ms(610)).unwrap();
    assert_eq!(player_at(&app), (x, y + 2));
    app.mouse_release();
    app.tick(ms(2000)).unwrap();
    assert_eq!(player_at(&app), (x, y + 2), "let go, it stops");
}

#[test]
fn tabs_change_by_click_and_by_action() {
    let m = model(&mini_phone());
    let mut app = App::new(&m, 7, (30, 30)).unwrap();
    app.r.w.log.push(rlml_core::rt::world::LogEntry {
        key: "STD.MESSAGE.COMBAT.N1".into(),
        args: vec![],
        speech: false,
    });
    app.draw();
    // a click on the LOG title shows the log under the tab row
    let (tx, ty) = tab_title(&app, 1);
    app.mouse_press(click(), tx, ty, ms(10)).unwrap();
    assert_eq!(app.draw().row(26).trim_end(), "STD.MESSAGE.COMBAT.N1");
    // tab.next goes round to the pad, and a button's `tab LOG` comes back
    press(&mut app, "t");
    assert_eq!(app.draw().row(26).trim(), "PAD.N");
    let (r, _) = app
        .view
        .buttons
        .iter()
        .find(|(_, a)| matches!(a, rlml_app::menu::ItemAction::Tab(_)))
        .cloned()
        .unwrap();
    app.mouse_press(click(), r.x, r.y, ms(20)).unwrap();
    assert_eq!(app.draw().row(26).trim_end(), "STD.MESSAGE.COMBAT.N1");
}

#[test]
fn a_held_button_shows_its_panel_until_let_go() {
    let f = mini_with(|g| {
        *g = g.replace(
            "- keys GAME\n",
            "- keys GAME\n- layout MAIN\n- layout PHONE when portrait\n",
        );
        g.push_str(
            "\nlayout PHONE\n\n```\nm\nt\n```\n\nlegend:\n- m map\n- t tabs PAD size.height 4\n\n\
             panel PAD\n\n```\nhhhhh\n```\n\nlegend:\n- h button hold HELP text PAD.H\n\n\
             panel HELP\n\n```\nxxxxxxxxxx\n```\n\nlegend:\n- x text HELP.TEXT\n\n\
             panels\n- button border rounded fg ffff00 bg 000000\n",
        );
    });
    let m = model(&f);
    let mut app = App::new(&m, 7, (20, 30)).unwrap();
    app.releases = true;
    let frame = app.draw();
    // with the button style the tab row is three rows of buttons
    assert_eq!(
        frame.row(26).chars().next(),
        Some('╭'),
        "{:#?}",
        frame.rows()
    );
    let (r, _) = app.view.buttons[0].clone();
    app.mouse_press(click(), r.x + 1, r.y, ms(10)).unwrap();
    let f = app.draw();
    let map = app.view.map_rect.unwrap();
    // over the band under the map, which stays in sight
    let at = f
        .rows()
        .iter()
        .position(|r| r.trim_end() == "HELP.TEXT")
        .expect("the panel shows");
    assert_eq!(at, map.y + map.h, "{:#?}", f.rows());
    app.mouse_release();
    assert!(
        !app.draw()
            .rows()
            .iter()
            .any(|r| r.trim_end() == "HELP.TEXT"),
        "let go, it closes at once"
    );
}

#[test]
fn a_tap_on_a_read_log_opens_it_and_center_centres_a_panel() {
    let f = mini_with(|g| {
        *g = g.replace(
            "- keys GAME\n",
            "- keys GAME\n- layout MAIN\n- layout PHONE when portrait\n",
        );
        g.push_str(
            "\nlayout PHONE\n\n```\nm\nl\np\n```\n\nlegend:\n- m map\n- l LOG size.height 3 read\n- p PAD size.height 6 center\n\n\
             panel PAD\n\n```\nhhhh\n```\n\nlegend:\n- h button MOVE.NORTH text PAD.H\n",
        );
    });
    let m = model(&f);
    let mut app = App::new(&m, 7, (20, 30)).unwrap();
    let frame = app.draw();
    // the 4×1 pad in the middle of its 20×6 area at rows 24..30
    let (r, _) = app.view.buttons[0].clone();
    assert_eq!((r.x, r.y, r.w), (8, 26, 4), "{:#?}", frame.rows());
    // a click on the log opens the reading view
    app.mouse_press(click(), 3, 22, ms(10)).unwrap();
    assert!(app.reading.is_some());
}

#[test]
fn layout_rows_need_one_default() {
    let f = mini_with(|g| {
        *g = g.replace(
            "- keys GAME\n",
            "- keys GAME\n- layout MAIN\n- layout MAIN\n- layout MAIN when height <> 3\n",
        )
    });
    let (_, r) = rlml_core::check_game(&f, "game.rlml");
    let errors: Vec<String> = r.errors().map(|d| d.to_string()).collect();
    assert!(
        errors
            .iter()
            .any(|e| e.contains("exactly one has no `when`, here 2")),
        "{errors:?}"
    );
    assert!(
        errors.iter().any(|e| e.contains("`<>` is no comparison")),
        "{errors:?}"
    );
}

#[test]
fn a_modal_panel_scrolls_when_it_does_not_fit_and_buttons_hide_and_press() {
    let rows: String = (0..20).map(|_| "LLLLLLLLLL\n").collect();
    let f = mini_with(move |g| {
        *g = g.replace(
            "- keys GAME\n",
            "- keys GAME\n- layout MAIN\n- layout PHONE when portrait\n",
        );
        g.push_str(&format!(
            "\nlayout PHONE\n\n```\nm\np\n```\n\nlegend:\n- m map\n- p PAD size.height 3\n\n\
             panel PAD\n\n```\nhhhh oooo\n```\n\nlegend:\n- h button open LIST text PAD.H\n- o button MOVE.NORTH text PAD.O hide when player.at.x == 2\n\n\
             panel LIST\n\n```\n{rows}```\n\nlegend:\n- L value [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]\n\n\
             panels\n- button fg 000000 bg ffff00\n- button.pressed bg ffffff\n"
        ));
    });
    let m = model(&f);
    let mut app = App::new(&m, 7, (20, 30)).unwrap();
    app.view.truecolor = true;
    app.releases = true;
    app.draw();
    // the player stands at x 2: the second button is hidden
    assert_eq!(app.view.buttons.len(), 1);
    // held down, a button is white, and yellow again when let go
    let (r, _) = app.view.buttons[0].clone();
    let bg = |app: &mut App| {
        app.draw()
            .get(r.x, r.y)
            .and_then(|c| c.style)
            .and_then(|s| s.bg)
    };
    assert_eq!(
        bg(&mut app),
        Some(termplate_core::style::Color::Rgb(0xff, 0xff, 0x00))
    );
    app.mouse_press(click(), r.x, r.y, ms(10)).unwrap();
    assert!(
        app.reading.as_ref().is_some_and(|r| r.panel),
        "open LIST opens the panel"
    );
    app.mouse_release();
    // over the band under the map, which stays in sight: 3 rows of 15, so
    // it scrolls, down to its last row only
    let f = app.draw();
    let map = app.view.map_rect.unwrap();
    let band = map.y + map.h;
    assert_eq!(f.row(band).trim(), "1", "{:#?}", f.rows());
    assert!(
        f.rows()[..band].iter().any(|r| r.contains('@')),
        "the map shows"
    );
    // at its top only the down button: a tap on it scrolls, then up shows
    let (up, down) = app.reading.as_ref().unwrap().arrows;
    assert!(up.is_none());
    let down = down.expect("more below");
    assert!(f.row(down.y).trim_end().ends_with('↓'));
    app.mouse_press(click(), down.x, down.y, ms(35)).unwrap();
    assert!(
        app.reading.is_some(),
        "the button scrolls and does not close"
    );
    assert_eq!(app.draw().row(band).trim_start().chars().next(), Some('2'));
    assert!(
        app.reading.as_ref().unwrap().arrows.0.is_some(),
        "now something above"
    );
    press(&mut app, "home");
    app.mouse_press(
        Binding::mouse(termplate_core::keys::Mouse::WheelDown),
        1,
        1,
        ms(40),
    )
    .unwrap();
    assert!(
        app.draw().row(band).trim_start().starts_with("4 "),
        "the wheel scrolls three rows"
    );
    press(&mut app, "end");
    assert_eq!(
        app.draw().row(29).split_whitespace().next(),
        Some("15"),
        "the last row at the bottom"
    );
    app.mouse_press(click(), 1, 1, ms(50)).unwrap();
    assert!(app.reading.is_none(), "a tap closes it");
}

#[test]
fn the_teletype_types_new_messages_and_a_press_shows_them_at_once() {
    let f = mini_with(|g| g.push_str("\npanels\n- LOG baud 1200\n"));
    let m = model(&f);
    let mut app = App::new(&m, 7, (40, 12)).unwrap();
    // the start's messages typed out
    for t in 1..200 {
        app.tick(ms(t * 50)).unwrap();
    }
    let log_row = |app: &mut App| {
        app.draw()
            .rows()
            .into_iter()
            .find(|r| r.contains("N00"))
            .map(|r| r.trim_end().to_string())
    };
    app.tick(ms(10_000)).unwrap();
    app.r.w.log.push(rlml_core::rt::world::LogEntry {
        key: "STD.MESSAGE.COMBAT.N00".into(),
        args: vec![],
        speech: false,
    });
    assert_eq!(log_row(&mut app), None, "not typed yet");
    app.tick(ms(10_080)).unwrap();
    // 1200 baud is 120 characters a second: 80 ms types 9
    let typed = app
        .draw()
        .rows()
        .into_iter()
        .find(|r| r.contains("STD.MESS"))
        .unwrap();
    assert_eq!(typed.trim_end(), "STD.MESSA▌");
    assert!(app.wait() <= ms(30), "the typing keeps the frames coming");
    // a press shows the rest at once
    app.key(key("x"), ms(10_100)).unwrap();
    assert_eq!(log_row(&mut app).as_deref(), Some("STD.MESSAGE.COMBAT.N00"));
}

#[test]
fn the_reading_view_over_a_read_log_keeps_its_lines_and_shows_a_panel_under_it() {
    let f = mini_with(|g| {
        *g = g.replace(
            "- keys GAME\n",
            "- keys GAME\n- layout MAIN\n- layout PHONE when portrait\n",
        );
        g.push_str(
            "\nlayout PHONE\n\n```\nm\nl\np\n```\n\nlegend:\n- m map\n- l LOG size.height 3 read BAG\n- p PAD size.height 6\n\n\
             panel PAD\n\n```\nhhhh\n```\n\nlegend:\n- h button MOVE.NORTH text PAD.H\n\n\
             panel BAG\n\n```\nbbbb\n```\n\nlegend:\n- b value \"bag!\"\n",
        );
    });
    let m = model(&f);
    let mut app = App::new(&m, 7, (20, 30)).unwrap();
    app.r.w.log.clear();
    for n in 0..12 {
        app.r.w.log.push(rlml_core::rt::world::LogEntry {
            key: format!("STD.MESSAGE.COMBAT.N{n:02}"),
            args: vec![],
            speech: false,
        });
    }
    let before = app.draw();
    let at = |f: &termplate_core::frame::Frame, y: usize| f.row(y).trim_end().to_string();
    // the log's three rows at 21..24, each message cut to two lines
    assert_eq!(at(&before, 23), "11");
    app.mouse_press(click(), 3, 22, ms(10)).unwrap();
    let f = app.draw();
    for y in 21..24 {
        assert_eq!(
            at(&f, y).split_whitespace().next(),
            at(&before, y).split_whitespace().next(),
            "row {y} stays"
        );
    }
    // above them the rest of their messages, and older ones over the
    // map, an empty line between those
    assert_eq!(at(&f, 20), "STD.MESSAGE.COMBAT.N");
    assert_eq!(at(&f, 19), "");
    assert_eq!(at(&f, 18).split_whitespace().next(), Some("09"));
    // the panel under the log, where the pad was
    assert!(
        f.rows().iter().skip(24).any(|r| r.contains("bag!")),
        "{:#?}",
        f.rows()
    );
    assert!(!f.rows().iter().skip(24).any(|r| r.contains("PAD.H")));
}

#[test]
fn right_and_bottom_place_a_panel_and_a_used_keyboard_shows_keys_on_buttons() {
    let f = mini_with(|g| {
        *g = g.replace(
            "- keys GAME\n",
            "- keys GAME\n- layout MAIN\n- layout PHONE when portrait\n",
        );
        g.push_str(
            "\nlayout PHONE\n\n```\nm\nl\np\n```\n\nlegend:\n- m map\n- l LOG size.height 3\n- p PAD size.height 6 right bottom\n\n\
             panel PAD\n\n```\nhhhhh\n```\n\nlegend:\n- h button MOVE.NORTH text PAD.H\n",
        );
    });
    let mut f = f;
    f.game
        .get_mut("text/fi.rlml")
        .unwrap()
        .push_str("\nPAD.H yl\nPAD.H.KEYS yl k\n");
    let m = model(&f);
    let mut app = App::new(&m, 7, (20, 30)).unwrap();
    let frame = app.draw();
    // the 5×1 pad at the right bottom of its 20×6 area at rows 24..30
    let (r, _) = app.view.buttons[0].clone();
    assert_eq!((r.x, r.y, r.w), (15, 29, 5), "{:#?}", frame.rows());
    let label = |app: &mut App| app.draw().row(29).trim().to_string();
    assert_eq!(label(&mut app), "yl");
    app.key(key("x"), ms(10)).unwrap();
    assert_eq!(
        label(&mut app),
        "yl k",
        "the keys once the keyboard is used"
    );
}

#[test]
fn a_button_look_colors_it_over_the_buttons_row() {
    let f = mini_with(|g| {
        *g = g.replace(
            "- keys GAME\n",
            "- keys GAME\n- layout MAIN\n- layout PHONE when portrait\n",
        );
        g.push_str(
            "\nlayout PHONE\n\n```\nm\nl\np\n```\n\nlegend:\n- m map\n- l LOG size.height 3\n- p PAD size.height 3\n\n\
             panel PAD\n\n```\nhhhh oooo\n```\n\nlegend:\n- h button MOVE.NORTH text PAD.H\n- o button MOVE.SOUTH text PAD.O look dark\n\n\
             panels\n- button fg 000000 bg ffff00\n- button.dark bg 888800\n- button.pressed bg ffffff\n",
        );
    });
    let m = model(&f);
    let mut app = App::new(&m, 7, (20, 30)).unwrap();
    app.view.truecolor = true;
    app.draw();
    let cells: Vec<_> = app.view.buttons.iter().map(|b| b.0).collect();
    let bg = |app: &mut App, r: termplate_core::layout::Rect| {
        app.draw()
            .get(r.x, r.y)
            .and_then(|c| c.style)
            .and_then(|s| s.bg)
    };
    use termplate_core::style::Color::Rgb;
    assert_eq!(bg(&mut app, cells[0]), Some(Rgb(0xff, 0xff, 0x00)));
    assert_eq!(
        bg(&mut app, cells[1]),
        Some(Rgb(0x88, 0x88, 0x00)),
        "its own look"
    );
    let fg = app
        .draw()
        .get(cells[1].x, cells[1].y)
        .and_then(|c| c.style)
        .and_then(|s| s.fg);
    assert_eq!(fg, Some(Rgb(0, 0, 0)), "the rest from `button`");
}

// ---- saving (M 5.2, M 5.4) ----

/// The minigame with `save` on `w`, the `autosave` section `autosave`
/// and `edit` on its caves.
fn saving(autosave: &str, edit_caves: impl FnOnce(&mut String)) -> MemFiles {
    let mut f = mini_with(|g| g.push_str(&format!("\n{autosave}\n")));
    let ui = f.game.get_mut("ui/main.rlml").unwrap();
    *ui = ui.replace("- pass z", "- pass z\n- save w");
    edit_caves(f.game.get_mut("maps/caves.rlml").unwrap());
    f
}

fn with_store<'m>(m: &'m Model) -> App<'m> {
    let mut app = App::new(m, 7, (40, 12)).unwrap();
    app.set_store(Box::new(rlml_core::rt::store::MemStore::default()));
    app
}

fn saved(app: &App) -> Vec<String> {
    let mut names: Vec<String> = app
        .saves
        .as_ref()
        .unwrap()
        .list()
        .iter()
        .map(|l| l.slot.name())
        .collect();
    names.sort();
    names
}

#[test]
fn quit_saves_and_the_engine_menu_loads_it() {
    let m = model(&saving("", |_| {}));
    let mut app = with_store(&m);
    press(&mut app, "s s");
    let state = app.r.w.s.clone();
    press(&mut app, "esc j enter");
    assert!(!app.playing);
    assert_eq!(
        saved(&app),
        ["quit"],
        "without `autosave`, leaving has its own slot (D 3.4)"
    );
    let s = screen(&mut app);
    assert!(
        s.contains("Load game"),
        "the engine's menu offers the saves: {s}"
    );
    press(&mut app, "j enter");
    let s = screen(&mut app);
    assert!(s.contains("quit · CAVE_A · turn 2"), "{s}");
    press(&mut app, "enter");
    assert!(app.playing && app.players_turn());
    assert!(
        app.r.w.s == state,
        "the game goes on from where it was left (M 5.2)"
    );
    press(&mut app, "s");
    assert_eq!(app.r.w.s.game.turn(), 3);
}

#[test]
fn the_players_save_takes_its_own_slot() {
    let m = model(&saving("autosave\n- slots 2\n- manual 2", |_| {}));
    let mut app = with_store(&m);
    press(&mut app, "s w");
    assert_eq!(app.note.as_deref(), Some("saved: save 1"));
    assert!(app.playing);
    press(&mut app, "s w");
    assert_eq!(saved(&app), ["manual-1", "manual-2"]);
}

#[test]
fn a_permadeath_save_ends_the_game() {
    let m = model(&saving("autosave\n- slots 1", |_| {}));
    let mut app = with_store(&m);
    press(&mut app, "s w");
    assert!(
        !app.playing,
        "`slots 1`: the player's save ends the game (M 5.4)"
    );
    assert_eq!(saved(&app), ["game"]);
}

#[test]
fn no_save_on_a_nosave_map() {
    let m = model(&saving("", |c| {
        *c = c.replace("map CAVE_A\n", "map CAVE_A nosave\n")
    }));
    let mut app = with_store(&m);
    press(&mut app, "w");
    assert_eq!(app.note.as_deref(), Some("the game cannot be saved here"));
    press(&mut app, "esc j enter");
    assert!(
        saved(&app).is_empty(),
        "leaving does not save on a `nosave` map either (M 5.2)"
    );
}

#[test]
fn the_save_item_is_dim_on_a_nosave_map() {
    let mut f = saving("", |c| {
        *c = c.replace("map CAVE_A\n", "map CAVE_A nosave\n")
    });
    let ui = f.game.get_mut("ui/main.rlml").unwrap();
    *ui = ui.replace(
        "menu PAUSE\n- RESUME\n",
        "menu PAUSE\n- RESUME\n- SAVE save\n",
    );
    let m = model(&f);
    let mut app = with_store(&m);
    press(&mut app, "esc");
    let layer = app.menus.layers.last().unwrap();
    let save = layer
        .rows
        .iter()
        .find(|r| layer.menu.items[r.item].key == "SAVE")
        .unwrap();
    assert!(!save.enabled, "the item shows dim (M 5.2)");
}

#[test]
fn automatic_saves_fill_the_stack() {
    let m = model(&saving("autosave\n- slots 2\n- on input every 2", |_| {}));
    let mut app = with_store(&m);
    press(&mut app, "s");
    assert_eq!(saved(&app), ["auto-1"], "the start's wait and the next one");
    press(&mut app, "s s s s s");
    assert_eq!(
        saved(&app),
        ["auto-1", "auto-2"],
        "two slots, the oldest replaced"
    );
    let list = app.saves.as_ref().unwrap().list();
    assert_eq!(
        list[0].save.now.s.game.turn(),
        5,
        "the newest is the last even wait"
    );
}

// ---- the profile (M 5.6) ----

#[test]
fn the_profile_lives_over_games_and_the_engine_menu_resets_it() {
    use rlml_core::rt::profile::FILE;
    use rlml_core::rt::store::{MemStore, Store};
    let mut f = mini_with(|g| {
        *g = g.replace(
            "- PLAYER HERO\n",
            "- PLAYER HERO\n\nprofile MINI\n- UNLOCKS\n",
        );
        g.push_str("\ncomponent UNLOCKS\n- wins number\n\n```roguescript\ncommand WIN() cost 0:\n    profile.UNLOCKS.wins += 1\n    return true\n```\n");
    });
    let ui = f.game.get_mut("ui/main.rlml").unwrap();
    *ui = ui.replace("- pass z", "- pass z\n- WIN w");
    let m = model(&f);
    let mut app = App::new(&m, 7, (40, 12)).unwrap();
    app.set_store(Box::new(MemStore::default()));
    app.set_profile_store(Box::new(MemStore::default()), true)
        .unwrap();
    let wins = |app: &App| {
        app.r
            .w
            .s
            .profile
            .iter()
            .find(|c| c.name == "UNLOCKS")
            .and_then(|c| c.get("wins"))
            .and_then(|v| v.as_number())
    };
    assert_eq!(wins(&app), Some(0));
    press(&mut app, "w w");
    assert_eq!(wins(&app), Some(2));
    assert!(
        app.profile
            .as_ref()
            .unwrap()
            .store
            .read(FILE)
            .is_some_and(|t| t.contains("(n 2)")),
        "written after the action"
    );
    // a new game from the engine's menu sees it
    press(&mut app, "esc j enter");
    let s = screen(&mut app);
    assert!(s.contains("Reset profile"), "{s}");
    press(&mut app, "enter");
    assert!(app.playing);
    assert_eq!(wins(&app), Some(2), "the next game's snapshot (M 5.1)");
    // reset from the engine's menu: New game, Load game, Reset profile, Exit
    press(&mut app, "esc j enter j j enter");
    assert_eq!(app.note.as_deref(), Some("the profile was reset"));
    assert!(app.profile.as_ref().unwrap().store.read(FILE).is_none());
    press(&mut app, "k k enter");
    assert_eq!(wins(&app), Some(0), "the profile starts over");
}

// ---- the minigame's touch buttons (esimerkit/mini/ui/main.rlml) ----

/// The middle cell of the minigame's button whose action is `action`
/// written as its legend does: a command, `pass` or `open PAUSE`.
fn pad_button(app: &App, action: &str) -> (usize, usize) {
    use rlml_app::menu::ItemAction;
    let found = app.view.buttons.iter().find(|(_, a)| match a {
        ItemAction::Command(c) => c.ends_with(&format!(".{action}")) || c == action,
        ItemAction::Engine(e) => e == action,
        ItemAction::Open { menu, .. } => format!("open {menu}") == action,
        _ => false,
    });
    let (r, _) = found.unwrap_or_else(|| panic!("no button {action}: {:?}", app.view.buttons));
    (r.x + r.w / 2, r.y + r.h / 2)
}

fn tap(app: &mut App, action: &str, t: u64) {
    app.draw();
    let (x, y) = pad_button(app, action);
    app.mouse_press(click(), x, y, ms(t)).unwrap();
    app.mouse_release();
}

#[test]
fn the_minigames_buttons_show_on_a_phone_held_upright() {
    let m = model(&mini_as_is());
    let mut app = App::new(&m, 7, (80, 24)).unwrap();
    app.draw();
    assert!(
        app.view.buttons.is_empty(),
        "a landscape window has no buttons"
    );
    // a phone held upright: 40 columns, 60 rows
    app.resize(40, 60);
    let s = screen(&mut app);
    for t in ["↑", "↓", "←", "→", "odota", "valikko"] {
        assert!(s.contains(t), "{t}:\n{s}");
    }
    assert_eq!(app.view.buttons.len(), 6);
}

#[test]
fn the_minigames_buttons_play_it() {
    let m = model(&mini_as_is());
    let mut app = App::new(&m, 7, (40, 60)).unwrap();
    let (x, y) = player_at(&app);
    tap(&mut app, "MOVE.EAST", 10);
    assert_eq!(player_at(&app), (x + 1, y));
    tap(&mut app, "MOVE.WEST", 20);
    tap(&mut app, "MOVE.SOUTH", 30);
    tap(&mut app, "MOVE.NORTH", 40);
    assert_eq!(player_at(&app), (x, y));
    let time = app.r.w.s.game.time;
    tap(&mut app, "WAIT", 50);
    assert_eq!(app.r.w.s.game.time, time + 120, "odota waits a turn");
    // valikko opens the pause menu, which has Save
    tap(&mut app, "open PAUSE", 70);
    assert!(app.menus.is_open());
    let s = screen(&mut app);
    assert!(s.contains("Tallenna"), "{s}");
}

#[test]
fn a_tap_on_the_minigames_map_walks_and_stairs_take_you_down() {
    let m = model(&mini_as_is());
    let mut app = App::new(&m, 7, (40, 60)).unwrap();
    // a tap on the map walks there (`walk STD.MOVE`)
    app.draw();
    let (x, y) = player_at(&app);
    let (sx, sy) = screen_of(&app, x + 1, y + 1);
    app.mouse_press(click(), sx, sy, ms(10)).unwrap();
    app.mouse_release();
    run_ticks(&mut app, 10, 5);
    assert_eq!(player_at(&app), (x + 1, y + 1), "walked to the tapped cell");
    // a tap on the stairs walks onto them, and the step takes the player
    // down (`EXIT ... step true`)
    let stairs = app.r.w.s.maps[0].label("START")[0];
    app.r.w.s.maps[0].remembered = (0..8).flat_map(|y| (0..20).map(move |x| (x, y))).collect();
    app.draw();
    let (sx, sy) = screen_of(&app, stairs.0, stairs.1);
    app.mouse_press(click(), sx, sy, ms(1000)).unwrap();
    app.mouse_release();
    run_ticks(&mut app, 1000, 40);
    let p = app.r.w.s.at(app.r.w.s.player.unwrap()).unwrap();
    assert_eq!(app.r.w.s.maps[p.map].name, "CAVE_B", "down the stairs");
}

#[test]
fn a_sized_map_area_is_placed_as_a_panel() {
    // the minigame's phone layout: `- m map size 20x8 center`
    let m = model(&mini_as_is());
    let mut app = App::new(&m, 7, (40, 40)).unwrap();
    app.draw();
    let r = app.view.map_rect.unwrap();
    assert_eq!(
        (r.x, r.w, r.h),
        (10, 20, 8),
        "centred across the 40 columns (D 11.1)"
    );
    // `right bottom` puts it in the area's bottom right corner
    let mut f = mini_with(|g| *g = g.replace("- keys GAME\n", "- keys GAME\n- layout CORNER\n"));
    f.game.get_mut("ui/main.rlml").unwrap().push_str("\nlayout CORNER\n\n```\nm\nl\n```\n\nlegend:\n- m map size 10x4 right bottom\n- l LOG size.height 3\n");
    let m = model(&f);
    let mut app = App::new(&m, 7, (40, 12)).unwrap();
    app.draw();
    let r = app.view.map_rect.unwrap();
    assert_eq!(
        (r.x, r.y, r.w, r.h),
        (30, 0, 10, 4),
        "at the right edge; the map's row is as high as the map, and the log takes the room left"
    );
    // a tap still finds its cell
    let p = app.r.w.s.at(app.r.w.s.player.unwrap()).unwrap();
    let map = app.r.w.s.active_map().unwrap();
    let (sx, sy) = screen_of(&app, p.x, p.y);
    assert_eq!(rlml_app::draw::cell_at(&app.view, map, sx, sy), Some(p));
}

// ---- the join screen, joining and leaving (M 8.2, moninpeli vaihe 3) ----

#[test]
fn the_join_screen_starts_the_group_when_both_seats_joined() {
    use rlml_app::input::Device;
    let m = model(&ryhma());
    let mut app = App::new(&m, 7, (80, 24)).unwrap();
    let s = screen(&mut app);
    assert!(
        s.contains("Liity peliin") && s.contains("Istuin 1  näppäimistön osa 1"),
        "{s}"
    );
    press(&mut app, "enter");
    assert!(app.joining.is_some(), "`seats 2..2`: one seat cannot start");
    press(&mut app, "e");
    assert!(screen(&mut app).contains("Istuin 2  näppäimistön osa 2"));
    press(&mut app, "x");
    assert!(
        !screen(&mut app).contains("Istuin 2"),
        "`back` from part 2 takes its seat away"
    );
    press(&mut app, "e e");
    assert!(
        app.playing,
        "`e` again joins, and on a joined part confirms"
    );
    assert_eq!(app.seat_devices.get(&2), Some(&Device::Keyboard(2)));
    assert_eq!(app.r.w.s.seats.len(), 2);
}

/// The minigame for one or two seats: a pad joins mid-game with `start`
/// and pauses with `select`, and the pause menu has Leave.
fn mini_two_seats() -> MemFiles {
    let mut f = mini_with(|g| {
        *g = g.replace("- keys GAME\n", "- keys GAME\n- seats 1..2\n");
        g.push_str("\npad GAME\n- join start\n- WAIT south\n\npad ALWAYS\n- open PAUSE select\n");
    });
    let ui = f.game.get_mut("ui/main.rlml").unwrap();
    *ui = ui.replace(
        "menu PAUSE\n- RESUME\n",
        "menu PAUSE\n- RESUME\n- LEAVE leave_seat\n",
    );
    f
}

#[test]
fn a_pad_joins_mid_game_and_leaves_from_the_pause_menu() {
    use rlml_app::input::{Device, PadPress};
    let m = model(&mini_two_seats());
    let mut app = App::new(&m, 7, (40, 12)).unwrap();
    press(&mut app, "enter");
    assert!(
        app.playing && app.r.w.s.seats.len() == 1,
        "`seats 1..2`: one seat may start"
    );
    // one seat: the pause menu has no Leave
    press(&mut app, "esc");
    assert!(
        !screen(&mut app).contains("Leave"),
        "the last seat leaves by quitting"
    );
    press(&mut app, "esc");
    // a free pad's `start` joins seat 2 (JOIN_PLAYER gives it a hero)
    app.pad(5, PadPress::parse("start").unwrap(), ms(10))
        .unwrap();
    assert_eq!(app.r.w.s.seats.len(), 2);
    assert_eq!(app.seat_devices.get(&2), Some(&Device::Pad(5)));
    let member = app.r.w.s.active(2).expect("seat 2 has a character");
    // the pad pauses and leaves: its seat goes, its hero follows
    app.pad(5, PadPress::parse("select").unwrap(), ms(20))
        .unwrap();
    assert!(app.menus.is_open());
    app.pad(5, PadPress::parse("dpad.down").unwrap(), ms(30))
        .unwrap();
    app.pad(5, PadPress::parse("south").unwrap(), ms(40))
        .unwrap();
    assert_eq!(app.r.w.s.seats.len(), 1, "seat 2 left");
    assert!(app.r.w.s.seat(1).is_some(), "seat 1 kept its number");
    assert_eq!(app.r.w.s.seat_of(member), None);
    assert!(app.r.w.s.get(member).has("STD.FOLLOW"), "PART_PLAYER");
    assert!(!app.seat_devices.contains_key(&2));
    replays_the_same(&m, &app);
}

#[test]
fn a_loaded_game_asks_a_device_for_each_saved_seat() {
    use rlml_app::input::PadPress;
    let m = model(&mini_two_seats());
    let mut app = App::new(&m, 7, (40, 12)).unwrap();
    app.set_store(Box::new(rlml_core::rt::store::MemStore::default()));
    press(&mut app, "enter");
    app.pad(5, PadPress::parse("start").unwrap(), ms(10))
        .unwrap();
    let pad_hero = app.r.w.s.active(2).unwrap();
    // quit saves both seats; Load game opens the join screen for them
    press(&mut app, "esc j j enter");
    assert!(!app.playing);
    press(&mut app, "j enter enter");
    assert!(app.joining.is_some(), "the save has two seats");
    let s = screen(&mut app);
    assert!(
        s.contains("Seat 1  waiting for a device") && s.contains("Seat 2  waiting for a device"),
        "{s}"
    );
    // the keyboard takes seat 1, seat 2 is skipped, and the game goes on
    press(&mut app, "enter tab enter");
    assert!(app.playing);
    assert_eq!(
        app.r.w.s.seats.len(),
        1,
        "the skipped seat left before the first input"
    );
    assert_eq!(app.r.w.s.seat_of(pad_hero), None);
    assert!(app.r.w.s.get(pad_hero).has("STD.FOLLOW"), "as by `part`");
}

// ---- the camera's target (M 7.1, moninpeli vaihe 4) ----

#[test]
fn follow_camera_watches_a_rat_and_comes_back() {
    let mut f = mini_with(|g| {
        g.push_str(
            "\n```roguescript\ncommand WATCH() cost 0:\n    follow(camera, (entities(in: tiles()) where it has STD.HP and it != player).last)\n    return true\n\n\
             command HOME() cost 0:\n    follow(camera)\n    return true\n```\n",
        );
    });
    let ui = f.game.get_mut("ui/main.rlml").unwrap();
    *ui = ui.replace("- pass z", "- pass z\n- WATCH v\n- HOME b");
    // a map area smaller than the cave, so the camera has somewhere to go
    *ui = ui.replace("- m map\n", "- m map size 8x5\n");
    let m = model(&f);
    let mut app = App::new(&m, 7, (40, 12)).unwrap();
    app.draw();
    let map = app.r.w.s.active_map().unwrap();
    let home = app.view.camera[&map];
    press(&mut app, "v");
    let rat = app.r.w.s.camera.expect("WATCH set the target");
    app.draw();
    let p = app.r.w.s.at(rat).unwrap();
    let (cx, cy) = app.view.camera[&map];
    assert!(
        (cx..cx + 8).contains(&p.x) && (cy..cy + 5).contains(&p.y),
        "the rat at {p:?} is in view from {:?}",
        (cx, cy)
    );
    assert_ne!((cx, cy), home, "the camera moved to the rat");
    // the target goes with a save (M 7.1)
    let save = rlml_core::rt::save::Save::read(&app.r.save().write()).unwrap();
    assert_eq!(save.now.s.camera, Some(rat));
    // a destroyed target leaves the camera where it was
    let env = Env::new(BodyKind::Handler, "test", None, "test");
    app.r.w.destroy(rat, &env).unwrap();
    app.draw();
    assert_eq!(app.view.camera[&map], (cx, cy));
    // `follow(camera)` gives back the default: the player
    press(&mut app, "b");
    assert_eq!(app.r.w.s.camera, None);
    app.draw();
    assert_eq!(app.view.camera[&map], home);
}

// ---- scenes (RS 9, RLML-kohtaukset.md step 1) ----

fn scene_game() -> MemFiles {
    let mut f = mini_with(|g| {
        g.push_str(
            "\n```roguescript\non begin() as OPENING:\n    start(GREETING(player))\n\n\
             scene GREETING(hero: entity):\n    say(LINE.ONE, speaker: hero)\n    wait confirm\n    say(LINE.TWO)\n    say(LINE.THREE)\n```\n",
        );
    });
    let t = f.game.get_mut("text/fi.rlml").unwrap();
    t.push_str("\nLINE.ONE Tervetuloa luolaan, matkalainen.\nLINE.TWO Rotat ovat nälkäisiä.\nLINE.THREE Onnea matkaan.\n");
    f
}

#[test]
fn a_scene_shows_its_lines_and_takes_confirm_tap_and_skip() {
    let m = model(&scene_game());
    let mut app = App::new(&m, 7, (40, 16)).unwrap();
    assert_eq!(
        app.status,
        Status::Scene,
        "begin's scene before the first turn"
    );
    let s = screen(&mut app);
    assert!(
        s.contains("Tervetuloa luolaan") && s.contains(" sinä "),
        "the line and its speaker (M 7.4):\n{s}"
    );
    assert!(app.r.w.save_blocked(), "no saving in a scene (M 5.2)");
    let time = app.r.w.s.game.time;
    press(&mut app, "l");
    assert!(
        app.r.in_scene() && app.r.w.s.game.time == time,
        "the game's keys do nothing in a scene"
    );
    press(&mut app, "enter");
    assert!(
        screen(&mut app).contains('▼'),
        "`wait confirm`: a mark only"
    );
    // a tap confirms too, for a phone
    app.mouse_press(click(), 5, 5, ms(10)).unwrap();
    assert!(screen(&mut app).contains("Rotat ovat"));
    press(&mut app, "tab");
    assert!(!app.r.in_scene(), "`tab` skipped the rest (RS 9.8)");
    assert!(app.players_turn());
    assert!(
        app.r
            .w
            .log
            .iter()
            .any(|l| l.key == "LINE.THREE" && l.speech),
        "the skipped line went to the log without a window (RS 9.8)"
    );
    assert!(
        !screen(&mut app).contains("Tervetuloa"),
        "the minigame's log does not take SPEECH"
    );
    replays_the_same(&m, &app);
}

// ---- dialogues: the Oz example (RS 9.7, RLML-kohtaukset.md step 2) ----

fn oz() -> MemFiles {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut f = MemFiles::default();
    read_tree(&repo, &repo.join("lib"), &mut f.engine);
    let game = repo.join("esimerkit/kohtaus");
    read_tree(&game, &game, &mut f.game);
    f
}

#[test]
fn the_oz_example_plays_its_dialogues() {
    let m = model(&oz());
    let mut app = App::new(&m, 7, (60, 24)).unwrap();
    // the Guardian of the Gates asks a name: typed into the field
    assert!(screen(&mut app).contains("Seis!"));
    press(&mut app, "enter");
    for c in "Dorothy".chars() {
        app.key(Binding::key(KeyCode::Char(c), Mods::default()), ms(10))
            .unwrap();
    }
    assert!(
        screen(&mut app).contains("Dorothy_"),
        "the field shows what is typed"
    );
    press(&mut app, "enter");
    assert!(
        screen(&mut app).contains("vihreät lasit") || screen(&mut app).contains("lasit lukitaan")
    );
    press(&mut app, "enter");
    assert!(app.players_turn());
    let me = app.r.w.s.player.unwrap();
    assert_eq!(
        app.r
            .w
            .name_text("fi", &rlml_core::rt::value::Value::Entity(me)),
        "Dorothy"
    );
    // Oz: HOME, WITCH dimmed until he has spoken his price, BYE
    press(&mut app, "k k k enter");
    let s = screen(&mut app);
    assert!(
        s.contains("Haluan kotiin Kansasiin.") && s.contains("Kerro vielä noidasta."),
        "the choice as a menu:\n{s}"
    );
    let layer = app.menus.layers.last().unwrap();
    assert_eq!(layer.menu.full, rlml_app::app::DIALOGUE_MENU);
    assert_eq!(
        layer.rows.iter().map(|r| r.enabled).collect::<Vec<_>>(),
        [true, false, true],
        "WITCH is dim (RS 9.7)"
    );
    press(&mut app, "esc");
    press(&mut app, "esc");
    assert!(
        app.menus
            .layers
            .last()
            .is_some_and(|l| l.menu.full == rlml_app::app::DIALOGUE_MENU),
        "a dialogue has no back; pause opens and closes over it"
    );
    press(&mut app, "enter");
    assert!(screen(&mut app).contains("Lännen noita"));
    press(&mut app, "enter");
    // HEAD_GLOWS, a timeline: the app's ticks run the pan, its line
    // stops the clock, and confirming the line ends it (RS 9.3, RS 9.5)
    assert!(app.r.in_scene() && app.r.scene_clock_runs());
    app.tick(ms(1000)).unwrap();
    assert!(
        !screen(&mut app).contains("Pää hehkuu"),
        "the pan runs from 0ms to 2000ms"
    );
    app.tick(ms(2500)).unwrap();
    assert!(screen(&mut app).contains("Pää hehkuu"));
    press(&mut app, "enter");
    assert!(
        app.players_turn(),
        "HEAD_GLOWS ended with its line; its loop ended with it"
    );
    // the Scarecrow, along row 3 out of the patrol's way: THINK opens
    // WHAT in the next choice
    press(&mut app, "j h h h enter");
    assert!(screen(&mut app).contains("aivoja"));
    press(&mut app, "enter");
    let s = screen(&mut app);
    assert!(s.contains("Mitä nurkassa liikkuu?"), "{s}");
    press(&mut app, "enter");
    assert!(
        screen(&mut app).contains("Päät eivät köhi."),
        "promised: the screen's secret"
    );
    press(&mut app, "enter");
    assert!(app.players_turn());
    // to the screen: down to row 5, west to the wall, and into it
    let at = |app: &App| app.r.w.s.at(app.r.w.s.player.unwrap()).unwrap();
    while at(&app).y < 5 {
        press(&mut app, "j");
    }
    while at(&app).x > 1 {
        press(&mut app, "h");
    }
    press(&mut app, "j");
    // SCREEN_FALLS (RS 9.1): rows, a duration, a mark and a condition
    assert!(
        screen(&mut app).contains("Toto haukahtaa"),
        "{}",
        screen(&mut app)
    );
    press(&mut app, "enter");
    app.tick(ms(10_000)).unwrap();
    assert!(screen(&mut app).contains("kalju vanha mies"), "REVEAL");
    let screens = |app: &App| {
        app.r
            .w
            .s
            .entities
            .iter()
            .filter(|e| e.alive && e.kind.ends_with("SCREEN"))
            .count()
    };
    assert_eq!(screens(&app), 0, "the screen fell at 500ms");
    press(&mut app, "enter");
    assert!(
        screen(&mut app).contains("Olen vain tavallinen mies"),
        "promised: OZ.CONFESSES"
    );
    press(&mut app, "enter");
    app.tick(ms(20_000)).unwrap();
    assert!(screen(&mut app).contains("ilmapallo"), "REVEAL+1s");
    let door = app.r.w.s.maps[0].tile(0, 7, 7).map(str::to_string);
    assert!(
        door.as_deref().is_some_and(|t| t.ends_with("DOOR.open")),
        "{door:?}"
    );
    press(&mut app, "enter");
    app.tick(ms(30_000)).unwrap();
    assert!(app.players_turn());
    // the patrol's steps are in-between frames, `frame 40ms` apart (M 7.2)
    let soldier = |app: &App| {
        let s = &app.r.w.s;
        (0..s.entities.len())
            .find(|&e| s.get(e).alive && s.get(e).kind.ends_with("SOLDIER"))
            .and_then(|e| s.at(e))
    };
    let mut moved = false;
    for _ in 0..4 {
        app.frames.clear();
        let before = soldier(&app);
        press(&mut app, "s");
        if soldier(&app) != before {
            moved = true;
            assert!(!app.frames.is_empty(), "a step shows as a frame");
        }
    }
    assert!(moved, "the soldier walks");
    // the soldier's report is a window on the map, and the main screen
    // has no log; `o` reads it
    let mut waited = 0;
    while !screen(&mut app).contains("Salissa on kaikki hyvin.") {
        assert!(waited < 40, "the patrol reports at round 28 of its loop");
        press(&mut app, "s");
        waited += 1;
    }
    assert!(app.r.in_scene(), "the game waits for the report (RS 9.9)");
    press(&mut app, "enter");
    assert!(app.players_turn());
    assert!(
        !screen(&mut app).contains("Salissa on kaikki hyvin."),
        "no log on the main screen"
    );
    press(&mut app, "o");
    let s = screen(&mut app);
    assert!(
        s.contains("Loki") && s.contains("Salissa on kaikki hyvin."),
        "the log's history:\n{s}"
    );
    press(&mut app, "esc");
    replays_the_same(&m, &app);
}
