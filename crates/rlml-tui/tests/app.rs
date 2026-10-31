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
