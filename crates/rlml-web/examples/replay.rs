//! Plays keys through the browser's game loop natively and prints the
//! log, to compare with `rlml run`, and the times of `draw` and the
//! steps:
//!   cargo run --release -p rlml-web --example replay -- GAMEDIR "l l k" [SEED]
//! Each key is pressed on the player's turn, as `rlml run` does; the
//! world's rounds in between go by on a clock of their own.

use std::path::Path;
use std::time::{Duration, Instant};

use rlml_web::game::Game;

fn read_tree(root: &Path, dir: &Path, skip: &[&str], out: &mut Vec<(String, String)>) {
    let mut entries: Vec<_> = std::fs::read_dir(dir).unwrap().flatten().collect();
    entries.sort_by_key(|e| e.file_name());
    for e in entries {
        let name = e.file_name().to_string_lossy().to_string();
        let p = e.path();
        let rel = p.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
        if name.starts_with('.') || skip.contains(&rel.as_str()) {
            continue;
        }
        if p.is_dir() {
            read_tree(root, &p, skip, out);
        } else if [".rlml", ".md", ".rogue"].iter().any(|x| name.ends_with(x)) {
            out.push((rel, std::fs::read_to_string(&p).unwrap()));
        }
    }
}

fn stats(name: &str, v: &mut [Duration]) {
    if v.is_empty() {
        return;
    }
    v.sort();
    let sum: Duration = v.iter().sum();
    let p = |q: f64| v[((v.len() - 1) as f64 * q) as usize];
    eprintln!("{name:>5}: n {:5}  mean {:7.3} ms  p95 {:7.3} ms  max {:7.3} ms", v.len(), sum.as_secs_f64() * 1e3 / v.len() as f64, p(0.95).as_secs_f64() * 1e3, p(1.0).as_secs_f64() * 1e3);
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let game = Path::new(args.first().expect("GAMEDIR"));
    let keys = args.get(1).cloned().unwrap_or_default();
    let seed: u64 = args.get(2).map(|s| s.parse().unwrap()).unwrap_or(1);
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let (mut g, mut e) = (vec![], vec![]);
    read_tree(game, game, &["testit"], &mut g);
    read_tree(&repo, &repo.join("lib"), &[], &mut e);
    let files = rlml_web::bundle::files(g, e);
    let m = rlml_web::bundle::load(&files, "game.rlml").unwrap_or_else(|e| panic!("{e}"));
    let m = Box::leak(Box::new(m));
    let mut game = Game::new(m, seed, (100, 34)).unwrap();
    let (mut draws, mut inputs, mut ticks, mut app_draws) = (vec![], vec![], vec![], vec![]);
    let mut now = Duration::ZERO;
    // `draw` is the app's frame and its changed cells; the app's frame
    // alone is timed apart, on an extra call
    let draw = |game: &mut Game, draws: &mut Vec<Duration>, app_draws: &mut Vec<Duration>| {
        while game.next_frame().is_some() {}
        let t = Instant::now();
        game.app.draw();
        app_draws.push(t.elapsed());
        let t = Instant::now();
        game.draw();
        draws.push(t.elapsed());
    };
    draw(&mut game, &mut draws, &mut app_draws);
    for k in keys.split_whitespace() {
        // the world's rounds until the player's turn
        for _ in 0..1000 {
            if game.app.players_turn() {
                break;
            }
            now += Duration::from_millis(50);
            let t = Instant::now();
            game.tick(now).unwrap();
            ticks.push(t.elapsed());
            draw(&mut game, &mut draws, &mut app_draws);
        }
        now += Duration::from_millis(10);
        game.push(rlml_app::events::Event::Key(termplate_core::keys::parse_binding(k).unwrap()));
        let t = Instant::now();
        game.input(now).unwrap();
        inputs.push(t.elapsed());
        let t = Instant::now();
        game.tick(now).unwrap();
        ticks.push(t.elapsed());
        draw(&mut game, &mut draws, &mut app_draws);
    }
    for l in game.log_lines() {
        println!("{l}");
    }
    stats("draw", &mut draws);
    stats("app", &mut app_draws);
    stats("input", &mut inputs);
    stats("tick", &mut ticks);
}
