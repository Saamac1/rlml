//! Frames of the STD minigame drawn into memory (M 7.1–7.4, D 9, D 11).

use std::path::Path;

use rlml_app::draw::View;
use rlml_core::load::MemFiles;
use rlml_core::model::Model;
use rlml_core::rt::play::Session;
use rlml_core::rt::state::Place;
use rlml_core::rt::value::Pos;
use termplate_core::frame::Frame;
use termplate_core::style::Color;

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

fn mini() -> MemFiles {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut f = MemFiles::default();
    read_tree(&repo, &repo.join("lib"), &mut f.engine);
    let game = repo.join("esimerkit/mini");
    read_tree(&game, &game, &mut f.game);
    f
}

fn model(f: &MemFiles) -> Model {
    let (m, r) = rlml_core::check_game(f, "game.rlml");
    assert!(!r.has_errors());
    m
}

fn fg(frame: &Frame, x: usize, y: usize) -> Option<Color> {
    frame.get(x, y).and_then(|c| c.style).and_then(|s| s.fg)
}

fn bg(frame: &Frame, x: usize, y: usize) -> Option<Color> {
    frame.get(x, y).and_then(|c| c.style).and_then(|s| s.bg)
}

#[test]
fn the_layout_map_and_colors() {
    let f = mini();
    let m = model(&f);
    let mut s = Session::start(&m, 7).unwrap();
    let mut view = View::default();
    let frame = view.draw(&mut s.r.w, "fi", 40, 12);
    let rows = frame.rows();
    // `m` over `l`, the log 3 rows high: the map area is 40×9 (D 11.1);
    // CAVE_A is 20×8, so it is centred: 10 columns in, 0 rows down (M 7.1)
    // the player sees its own room only: the wall at column 5 hides the rest
    assert_eq!(rows[0][10..30].trim_end(), "######");
    assert_eq!(rows[2].chars().nth(12), Some('@'), "{rows:#?}");
    assert_eq!(
        fg(&frame, 10, 0),
        Some(rlml_app::draw::rgb([0x7a, 0x7a, 0x8e], false)),
        "WALL fg STONE"
    );
    assert_eq!(
        bg(&frame, 10, 0),
        Some(rlml_app::draw::rgb([0x1a, 0x1a, 0x22], false)),
        "WALL bg DARK"
    );
    assert_eq!(
        fg(&frame, 11, 1),
        Some(rlml_app::draw::rgb([0x4a, 0x4a, 0x5a], false)),
        "FLOOR"
    );
    assert!(
        rows[9..].iter().all(|r| r.trim().is_empty()),
        "an empty log"
    );
}

#[test]
fn truecolor_is_the_front_ends_choice() {
    let f = mini();
    let m = model(&f);
    let mut s = Session::start(&m, 7).unwrap();
    // without the flag the 256-color palette, with it 24 bits
    assert!(matches!(
        fg(&View::default().draw(&mut s.r.w, "fi", 40, 12), 10, 0),
        Some(Color::Ansi(_))
    ));
    let mut view = View {
        truecolor: true,
        ..View::default()
    };
    assert_eq!(
        fg(&view.draw(&mut s.r.w, "fi", 40, 12), 10, 0),
        Some(Color::Rgb(0x7a, 0x7a, 0x8e))
    );
}

#[test]
fn the_log_shows_the_newest_messages() {
    let f = mini();
    let m = model(&f);
    let mut s = Session::start(&m, 7).unwrap();
    for k in ["s", "s", "s", "j", "h"] {
        s.press(k).unwrap();
    }
    let mut view = View::default();
    let frame = view.draw(&mut s.r.w, "fi", 40, 12);
    let log: Vec<String> = frame.rows()[9..]
        .iter()
        .map(|r| r.trim_end().to_string())
        .collect();
    assert_eq!(
        log,
        [
            "Otat 2 vahinkoa.",
            "rotta kuolee.",
            "rotta ottaa 3 vahinkoa."
        ]
    );
}

#[test]
fn the_camera_follows_on_a_map_wider_than_its_area() {
    let f = mini();
    let m = model(&f);
    let mut s = Session::start(&m, 7).unwrap();
    let mut view = View::default();
    // a map area 12 wide: CAVE_A (20 wide) does not fit
    let frame = view.draw(&mut s.r.w, "fi", 12, 12);
    assert_eq!(
        view.camera[&0].0, 0,
        "the player at x 2: the camera stops at the edge"
    );
    assert_eq!(frame.rows()[2].chars().nth(2), Some('@'));
    let p = s.r.w.s.player.unwrap();
    s.r.w.s.get_mut(p).place = Place::Map(Pos {
        map: 0,
        x: 15,
        y: 2,
    });
    let frame = view.draw(&mut s.r.w, "fi", 12, 12);
    assert_eq!(
        view.camera[&0].0, 8,
        "the player in the middle, clamped to the map"
    );
    assert_eq!(frame.rows()[2].chars().nth(7), Some('@'));
}

#[test]
fn a_too_small_window_says_so() {
    let f = mini();
    let m = model(&f);
    let mut s = Session::start(&m, 7).unwrap();
    let frame = View::default().draw(&mut s.r.w, "fi", 30, 3);
    assert!(
        frame.rows()[0].starts_with("The window is too small"),
        "{:?}",
        frame.rows()
    );
}

#[test]
fn a_panel_fills_its_fields() {
    let mut f = mini();
    let ui = f.game["ui/main.rlml"].replace(
        "```\nm\nl\n```\n\nlegend:\n- m map",
        "```\nms\nll\n```\n\nlegend:\n- m map\n- s SIDE(3) size.width 8",
    );
    f.game.insert("ui/main.rlml".into(), ui);
    f.game.insert(
        "ui/side.rlml".into(),
        "panel SIDE(n: number)\n\n```\n[TTTTTT]\nA=vvvvvv\nB=ppp\nLLLLLLLL\nLLLLLLLL\nLLLLLLLL\n```\n\nlegend:\n- T text SIDE.TITLE center\n- v value text_of(SIDE.SUM, N: 2 + 2)\n- p param n right\n- L value [text_of(SIDE.TITLE), text_of(n), SIDE.HERO]\n".into(),
    );
    let fi = f.game["text/fi.rlml"].clone()
        + "\nSIDE.TITLE Ohje\nSIDE.SUM yht. {N}\nSIDE.HERO {#GLYPH.HERO} sinä\n";
    f.game.insert("text/fi.rlml".into(), fi);
    let m = model(&f);
    let mut s = Session::start(&m, 7).unwrap();
    let frame = View::default().draw(&mut s.r.w, "fi", 40, 12);
    let rows = frame.rows();
    // 8 columns at the right; the fields are filled by their alignment
    let side: Vec<String> = rows
        .iter()
        .take(6)
        .map(|r| r.chars().skip(32).collect())
        .collect();
    // a list shows one member a line (M 7.7)
    assert_eq!(
        side,
        [
            "[ Ohje ]",
            "A=yht. 4",
            "B=  3   ",
            "Ohje    ",
            "3       ",
            "@ sinä  "
        ],
        "{rows:#?}"
    );
    // a text key as a name shows `{#TILE}` as the tile's glyph in its
    // colors (D 10, M 7.7)
    assert_eq!(
        fg(&frame, 32, 5),
        Some(rlml_app::draw::rgb([255, 255, 255], false))
    );
    assert_eq!(fg(&frame, 34, 5), None);
}
