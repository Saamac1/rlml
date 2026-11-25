//! The browser version's Rust side, natively: keys, cells, the bundle
//! and the worker's loop on the STD minigame.

use std::path::Path;
use std::time::Duration;

use rlml_app::events::Event;
use rlml_web::cells::{self, BOLD, CHAR, UNDERLINE, WIDE};
use rlml_web::game::Game;
use rlml_web::keys::binding;
use termplate_core::frame::{Frame, Role};
use termplate_core::keys::parse_binding;
use termplate_core::style::{Attrs, Color, Style, StyleSheet};

fn name(key: &str, code: &str, ctrl: bool, alt: bool, shift: bool) -> Option<String> {
    binding(key, code, ctrl, alt, shift).map(|b| b.name())
}

#[test]
fn browser_keys_have_the_terminals_names() {
    let plain = |k: &str, code: &str| name(k, code, false, false, false);
    assert_eq!(plain("h", "KeyH").as_deref(), Some("h"));
    assert_eq!(plain(".", "Period").as_deref(), Some("."));
    assert_eq!(plain("ArrowUp", "ArrowUp").as_deref(), Some("up"));
    assert_eq!(plain("Escape", "Escape").as_deref(), Some("esc"));
    assert_eq!(plain("Enter", "Enter").as_deref(), Some("enter"));
    assert_eq!(plain(" ", "Space").as_deref(), Some("space"));
    assert_eq!(plain("+", "Minus").as_deref(), Some("plus"));
    assert_eq!(plain("-", "Slash").as_deref(), Some("minus"));
    assert_eq!(plain("F5", "F5").as_deref(), Some("f5"));
    assert_eq!(plain("ä", "Quote").as_deref(), Some("ä"));
    // the numpad's digits are digits, as `num8` reads
    assert_eq!(plain("8", "Numpad8"), Some(parse_binding("num8").unwrap().name()));
    // shift makes the character, and shift+tab is its own key
    assert_eq!(name("K", "KeyK", false, false, true).as_deref(), Some("K"));
    assert_eq!(name("?", "Slash", false, false, true).as_deref(), Some("?"));
    assert_eq!(name("Tab", "Tab", false, false, true).as_deref(), Some("shift+tab"));
    // ctrl and alt name the key's place when the layout gives another character
    assert_eq!(name("c", "KeyC", true, false, false).as_deref(), Some("ctrl+c"));
    assert_eq!(name("˙", "KeyH", false, true, false).as_deref(), Some("alt+h"));
    // keys the terminal has no name for
    for k in ["Shift", "Control", "Dead", "Unidentified", "CapsLock"] {
        assert_eq!(plain(k, ""), None, "{k}");
    }
    // a name reads back as the same binding
    for k in [("K", "KeyK", false, false, true), ("Tab", "Tab", false, false, true), (" ", "Space", false, false, false), ("q", "KeyQ", true, false, false)] {
        let b = binding(k.0, k.1, k.2, k.3, k.4).unwrap();
        assert_eq!(parse_binding(&b.name()).unwrap(), b);
    }
}

fn cells_of(buf: &[u32]) -> Vec<[u32; 4]> {
    buf[3..].chunks(4).map(|c| [c[0], c[1], c[2], c[3]]).collect()
}

#[test]
fn frames_go_as_changed_cells_with_resolved_looks() {
    let sheet = StyleSheet::default();
    let mut a = Frame::new(4, 2);
    a.put(0, 0, '@', Role::Normal);
    let full = cells::encode(&sheet, None, &a);
    assert_eq!(&full[..3], &[4, 2, 1], "width, height, every cell");
    assert_eq!(cells_of(&full).len(), 8);
    assert_eq!(cells_of(&full)[0], [0, '@' as u32, cells::FG, cells::BG]);

    let mut b = a.clone();
    b.put(1, 1, '#', Role::Selected);
    b.cells[6].style = Some(Style { fg: Some(Color::Rgb(1, 2, 3)), attrs: Attrs { bold: true, underline: true, ..Attrs::default() }, ..Style::default() });
    let diff = cells_of(&cells::encode(&sheet, Some(&a), &b));
    assert_eq!(diff.len(), 2, "{diff:?}");
    // Selected is reverse video: the window's colors swapped
    assert_eq!(diff[0], [5, '#' as u32, cells::BG, cells::FG]);
    assert_eq!(diff[1][1] & CHAR, ' ' as u32);
    assert_eq!(diff[1][1] & (BOLD | UNDERLINE), BOLD | UNDERLINE);
    assert_eq!(diff[1][2], 0x010203);

    // a wide character covers two cells and goes as one
    let mut c = b.clone();
    c.text(0, 1, 4, "界", Role::Normal);
    let diff = cells_of(&cells::encode(&sheet, Some(&b), &c));
    assert_eq!(diff.iter().map(|d| d[0]).collect::<Vec<_>>(), vec![4], "{diff:?}");
    assert_eq!(diff[0][1] & CHAR, '界' as u32);
    assert_ne!(diff[0][1] & WIDE, 0);

    // a new size sends every cell again
    assert_eq!(cells::encode(&sheet, Some(&c), &Frame::new(3, 3))[2], 1);
}

#[test]
fn the_256_color_palette() {
    assert_eq!(cells::ansi(16), 0x000000);
    assert_eq!(cells::ansi(196), 0xff0000);
    assert_eq!(cells::ansi(244), 0x808080);
    assert_eq!(cells::rgb(Color::Reset), None);
    assert_eq!(cells::rgb(Color::White), Some(0xffffff));
}

fn read_tree(root: &Path, dir: &Path, out: &mut Vec<(String, String)>) {
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if e.file_name().to_string_lossy().starts_with('.') {
            continue;
        }
        if p.is_dir() {
            read_tree(root, &p, out);
        } else {
            out.push((p.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/"), std::fs::read_to_string(&p).unwrap()));
        }
    }
}

fn mini() -> rlml_core::load::MemFiles {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let (mut g, mut e) = (vec![], vec![]);
    read_tree(&repo.join("esimerkit/mini"), &repo.join("esimerkit/mini"), &mut g);
    read_tree(&repo, &repo.join("lib"), &mut e);
    rlml_web::bundle::files(g, e)
}

#[test]
fn load_errors_come_as_lines() {
    let mut f = mini();
    f.game.insert("game.rlml".into(), "game G\n- start NOWHERE\n".into());
    let e = rlml_web::bundle::load(&f, "game.rlml").unwrap_err();
    assert!(e.contains("game.rlml"), "{e}");
    assert!(rlml_web::bundle::load(&f, "missing.rlml").is_err());
}

#[test]
fn the_workers_loop_plays_the_game() {
    let m = Box::leak(Box::new(rlml_web::bundle::load(&mini(), "game.rlml").unwrap()));
    let mut g = Game::new(m, 7, (40, 12)).unwrap();
    assert!(g.bound_keys().contains("l") && g.bound_keys().contains("ctrl+c"));
    let first = g.draw();
    assert_eq!(first[2], 1, "the first frame is whole");
    assert!(g.draw().len() == 3, "nothing changed, no cells");
    let x = |g: &Game| g.app.r.w.s.at(g.app.r.w.s.player.unwrap()).unwrap().x;
    let x0 = x(&g);
    g.push(Event::Key(parse_binding("l").unwrap()));
    assert!(g.has_events());
    g.step(Duration::from_millis(10)).unwrap();
    assert!(!g.has_events());
    assert_eq!(x(&g), x0 + 1);
    while g.next_frame().is_some() {}
    let moved = g.draw();
    assert_eq!(moved[2], 0);
    assert!(moved.len() > 3, "the player's move shows");
    // ctrl+c leaves, and a tab cannot close: a new game in the same window
    g.push(Event::Key(parse_binding("ctrl+c").unwrap()));
    g.step(Duration::from_millis(20)).unwrap();
    assert!(g.quit());
    g.restart().unwrap();
    assert!(!g.quit());
    assert_eq!(x(&g), x0);
    assert_eq!(g.draw()[2], 0, "the window keeps its cells");
}

#[test]
fn mouse_buttons_and_the_wheel_have_the_terminals_names() {
    use rlml_web::keys::{mouse, wheel};
    assert_eq!(mouse(0, false, false, false).unwrap().name(), "click");
    assert_eq!(mouse(1, false, false, false).unwrap().name(), "mclick");
    assert_eq!(mouse(2, false, false, true).unwrap().name(), "shift+rclick");
    assert_eq!(mouse(3, false, false, false), None, "back and forward have no names");
    assert_eq!(wheel(true, false, false, false).name(), "wheel.up");
    assert_eq!(wheel(false, true, false, false).name(), "ctrl+wheel.down");
    for b in [mouse(2, false, false, true).unwrap(), wheel(false, true, false, false)] {
        assert_eq!(parse_binding(&b.name()).unwrap(), b);
    }
}

#[test]
fn pads_raw_events_go_to_the_worker_as_words() {
    use rlml_app::pad::Raw;
    use rlml_web::pads::{decode, encode};
    for (id, raw) in [
        (0, Raw::Connected),
        (3, Raw::Disconnected),
        (1, Raw::Button { name: "dpad.up".into(), down: true }),
        (1, Raw::Button { name: "south".into(), down: false }),
        (2, Raw::Axis { name: "lstick.x".into(), value: -0.625 }),
    ] {
        assert_eq!(decode(&encode(id, &raw)), Some((id, raw)));
    }
    assert_eq!(decode("1 b south"), None);
    assert_eq!(decode("x c"), None);
}

#[test]
fn clicks_hover_and_a_new_size_in_the_workers_loop() {
    let mut f = mini();
    let g = f.game.get_mut("game.rlml").unwrap();
    *g = g.replace("- PLAYER HERO\n", "- PLAYER HERO\n- walk STD.MOVE\n");
    let m = Box::leak(Box::new(rlml_web::bundle::load(&f, "game.rlml").unwrap()));
    let mut g = Game::new(m, 7, (40, 12)).unwrap();
    g.draw();
    let at = |g: &Game| {
        let p = g.app.r.w.s.at(g.app.r.w.s.player.unwrap()).unwrap();
        (p.x, p.y)
    };
    let (x, y) = at(&g);
    let r = g.app.view.map_rect.unwrap();
    let (cx, cy) = g.app.view.camera[&g.app.r.w.s.active_map().unwrap()];
    let (sx, sy) = (r.x + (x + 2 - cx) as usize, r.y + (y - cy) as usize);
    // the pointer's moves count at their last cell, which the hover follows
    g.push(Event::Move { x: 0, y: 0 });
    g.push(Event::Move { x: sx, y: sy });
    g.step(Duration::from_millis(10)).unwrap();
    assert_eq!(g.app.pointer, Some((sx, sy)));
    // a click travels there, a step at a time
    g.push(Event::Press { binding: parse_binding("click").unwrap(), x: sx, y: sy });
    let mut now = 20;
    g.step(Duration::from_millis(now)).unwrap();
    while at(&g) != (x + 2, y) && now < 5000 {
        now += 100;
        g.step(Duration::from_millis(now)).unwrap();
    }
    assert_eq!(at(&g), (x + 2, y), "the travel arrived");
    // a new size: the next frame is whole and of that size
    while g.next_frame().is_some() {}
    g.draw();
    g.push(Event::Resize(50, 16));
    g.step(Duration::from_millis(now + 10)).unwrap();
    let f = g.draw();
    assert_eq!(&f[..3], &[50, 16, 1]);
    // the same size again redraws every cell, as a new cell size needs
    g.push(Event::Resize(50, 16));
    g.step(Duration::from_millis(now + 20)).unwrap();
    assert_eq!(g.draw()[2], 1);
}

#[test]
fn fits_tells_whether_the_layout_holds_in_a_window() {
    // a map of 20x8 cells over a log of 3 rows
    let mut f = mini();
    let ui = f.game.get("ui/main.rlml").unwrap().replace("- m map\n", "- m map size 20x8\n");
    f.game.insert("ui/main.rlml".into(), ui);
    let m = Box::leak(Box::new(rlml_web::bundle::load(&f, "game.rlml").unwrap()));
    let g = Game::new(m, 7, (40, 12)).unwrap();
    assert!(g.app.fits(20, 11, (400, 300)));
    assert!(g.app.fits(80, 30, (800, 600)));
    assert!(!g.app.fits(19, 11, (400, 300)), "the map is narrower than written");
    assert!(!g.app.fits(20, 10, (400, 300)), "the log loses a row");
}

/// What OPFS holds after the worker wrote `changes` over `files`.
fn written(files: &mut std::collections::BTreeMap<String, String>, changes: Vec<(String, Option<String>)>) {
    for (path, text) in changes {
        match text {
            Some(t) => {
                files.insert(path, t);
            }
            None => {
                files.remove(&path);
            }
        }
    }
}

#[test]
fn saves_outlive_the_page() {
    use rlml_core::rt::store::Slot;
    let m = Box::leak(Box::new(rlml_web::bundle::load(&mini(), "game.rlml").unwrap()));
    let mut opfs = std::collections::BTreeMap::new();
    // the first page: an empty folder, two waits, then Quit, the pause
    // menu's third item
    let mut g = Game::new(m, 7, (40, 12)).unwrap();
    g.storage(vec![]).unwrap();
    assert!(g.store_changes().is_empty(), "nothing written yet");
    for (i, k) in ["s", "s", "esc", "j", "j", "enter"].iter().enumerate() {
        g.push(Event::Key(parse_binding(k).unwrap()));
        g.step(Duration::from_millis(10 * i as u64 + 10)).unwrap();
    }
    let left = g.app.r.w.s.clone();
    let changes = g.store_changes();
    assert_eq!(changes.iter().map(|(p, _)| p.as_str()).collect::<Vec<_>>(), ["saves/quit"], "leaving saved in the data folder's saves/ (D 3.2)");
    written(&mut opfs, changes);
    assert!(g.store_changes().is_empty(), "each change is given once");
    // the page is loaded again: the worker reads the folder before the game starts
    let mut g = Game::new(m, 7, (40, 12)).unwrap();
    g.storage(opfs.clone().into_iter().collect()).unwrap();
    let saves = g.app.saves.as_mut().unwrap();
    assert_eq!(saves.list().len(), 1);
    let save = saves.load(m, Slot::Quit).unwrap();
    let r = rlml_core::rt::run::Runner::load(m, &save).unwrap();
    assert!(r.w.s == left, "the save of the first page loads as it was left");
    // a tab cannot close: a new game in the same window keeps the folder
    g.restart().unwrap();
    assert_eq!(g.app.saves.as_ref().unwrap().list().len(), 1);
}
