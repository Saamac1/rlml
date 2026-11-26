//! `rlml check GAME` loads a game and reports load errors and warnings
//! (M 2). The engine library is found with `--lib DIR`, the `RLML_LIB`
//! environment variable or the `lib` folder next to the workspace.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use rlml_core::load::{Files, Root};
use rlml_core::model::Kind;

struct DiskFiles {
    game: PathBuf,
    engine: Option<PathBuf>,
}

impl DiskFiles {
    fn root(&self, root: Root) -> Option<&Path> {
        match root {
            Root::Game => Some(&self.game),
            Root::Engine => self.engine.as_deref(),
        }
    }
}

impl Files for DiskFiles {
    fn read(&self, root: Root, path: &str) -> Option<String> {
        let p = self.root(root)?.join(path);
        if !p.is_file() {
            return None;
        }
        std::fs::read_to_string(p).ok()
    }

    fn list(&self, root: Root, dir: &str) -> Option<Vec<String>> {
        let entries = std::fs::read_dir(self.root(root)?.join(dir)).ok()?;
        let mut out = Vec::new();
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if e.path().is_dir() {
                out.push(format!("{name}/"))
            } else {
                out.push(name)
            }
        }
        Some(out)
    }
}

const USAGE: &str = "usage: rlml check GAME [--lib DIR]
       rlml run GAME [--seed N] [--keys \"KEY KEY …\"] [--log-out FILE] [--lib DIR]
       rlml play GAME [--seed N] [--log-out FILE] [--data DIR] [--dev] [--lib DIR]
       rlml replay GAME LOG [--lib DIR]
       rlml inspect GAME [--seed N] [--keys \"KEY …\"] [PATH …] [--lib DIR]
       rlml bench GAME [--seed N] [--keys \"KEY …\" | --keys-file FILE] [--repeat N] [--lib DIR]

check loads the game and reports load errors and warnings (M 2).
run plays a new game without a screen: each key is one player's turn,
and the map, HP and log are printed after it. Keys are the key names of
the game's `keys` contexts, separated by spaces (`l l > j`).
play plays a new game in the terminal: the game's keys, esc for its
pause menu where it binds one, ctrl+c to leave at once. A runtime error
ends the game and is printed after the terminal is restored.
--log-out writes the session's input log (M 4.4) to FILE; with play it
is the last game's log, written also when a runtime error ends it.
play keeps the game's saves (DIR/saves) and profile (DIR/profile) in
DIR, by default the user data folder's rlml/GAME (~/.local/share on
Linux). --dev is development mode: f12 opens the inspector over the
game, which stands while it is open, and f5 reads and checks the files
again and goes on with them between actions (M 5.7).
replay plays an input log and prints a transcript as run does: the
frame after each row of the log.
inspect plays the keys as run does and prints the game's state at each
PATH (RLML-kehitystyokalut.md step 3): game, maps, map.NAME, entities,
entity#N, seats, wait, wait#N or scenes, and below them by name, as
game.turn or entity#12.HP.now. Without a PATH it lists the roots.
bench plays the keys as run does, --repeat times, without drawing, and
prints how long loading and playing took. Built with `--features
stats`, it also prints the interpreter's counters: statements, body
copies, name lookups, calls, events and handlers. --keys-file reads the
keys from FILE, as the playthrough tests keep them.

GAME is the game's main file or its folder (then game.rlml, game.md or
game.rlml.md). The engine library holds lib/std.rlml; it is found with
--lib, the RLML_LIB environment variable or next to this workspace.";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut game = None;
    let mut lib = std::env::var_os("RLML_LIB").map(PathBuf::from);
    let mut seed: u64 = 1;
    let mut keys = String::new();
    let mut log_out: Option<PathBuf> = None;
    let mut log_in: Option<PathBuf> = None;
    let mut data: Option<PathBuf> = None;
    let mut i = 0;
    let command = args.first().cloned().unwrap_or_default();
    let mut repeat: u32 = 1;
    let mut paths: Vec<String> = vec![];
    let mut dev = false;
    if !["check", "run", "play", "replay", "bench", "inspect"].contains(&command.as_str()) {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    }
    i += 1;
    while i < args.len() {
        match args[i].as_str() {
            "--lib" if i + 1 < args.len() => {
                lib = Some(PathBuf::from(&args[i + 1]));
                i += 1;
            }
            "--seed" if command != "check" && i + 1 < args.len() => {
                match args[i + 1].parse() {
                    Ok(n) => seed = n,
                    Err(_) => {
                        eprintln!("--seed takes a number");
                        return ExitCode::from(2);
                    }
                }
                i += 1;
            }
            "--log-out" if (command == "run" || command == "play") && i + 1 < args.len() => {
                log_out = Some(PathBuf::from(&args[i + 1]));
                i += 1;
            }
            "--dev" if command == "play" => dev = true,
            "--data" if command == "play" && i + 1 < args.len() => {
                data = Some(PathBuf::from(&args[i + 1]));
                i += 1;
            }
            "--keys"
                if ["run", "bench", "inspect"].contains(&command.as_str())
                    && i + 1 < args.len() =>
            {
                keys = args[i + 1].clone();
                i += 1;
            }
            "--keys-file" if command == "bench" && i + 1 < args.len() => {
                match std::fs::read_to_string(&args[i + 1]) {
                    Ok(k) => keys = k,
                    Err(e) => {
                        eprintln!("{}: {e}", args[i + 1]);
                        return ExitCode::FAILURE;
                    }
                }
                i += 1;
            }
            "--repeat" if command == "bench" && i + 1 < args.len() => {
                match args[i + 1].parse() {
                    Ok(n) if n >= 1 => repeat = n,
                    _ => {
                        eprintln!("--repeat takes a number from 1");
                        return ExitCode::from(2);
                    }
                }
                i += 1;
            }
            "-h" | "--help" => {
                println!("{USAGE}");
                return ExitCode::SUCCESS;
            }
            a if game.is_none() && !a.starts_with('-') => game = Some(PathBuf::from(a)),
            a if command == "inspect" && !a.starts_with('-') => paths.push(a.to_string()),
            a if command == "replay" && log_in.is_none() && !a.starts_with('-') => {
                log_in = Some(PathBuf::from(a))
            }
            a => {
                eprintln!("unexpected argument `{a}`\n\n{USAGE}");
                return ExitCode::from(2);
            }
        }
        i += 1;
    }
    let Some(game) = game else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    let (root, main) = if game.is_dir() {
        let main = ["game.rlml", "game.rlml.md", "game.md"]
            .into_iter()
            .find(|m| game.join(m).is_file());
        match main {
            Some(m) => (game.clone(), m.to_string()),
            None => {
                eprintln!("{}: no game.rlml, game.rlml.md or game.md", game.display());
                return ExitCode::from(2);
            }
        }
    } else {
        let root = game.parent().map(Path::to_path_buf).unwrap_or_default();
        (
            root,
            game.file_name().unwrap().to_string_lossy().to_string(),
        )
    };
    // the workspace's own `lib` folder holds the standard library
    let engine = lib.or_else(|| {
        Some(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."))
            .filter(|p| p.join("lib/std.rlml").is_file())
    });
    let files = DiskFiles { game: root, engine };
    let loading = std::time::Instant::now();
    let (model, report) = rlml_core::check_game(&files, &main);
    let loaded = loading.elapsed();
    if command == "play" {
        if report.has_errors() {
            for d in report.errors() {
                eprintln!("{d}");
            }
            return ExitCode::FAILURE;
        }
        let game_name = model
            .named_game()
            .map(|g| g.name.clone())
            .unwrap_or_default();
        let data = data.or_else(|| rlml_tui::saves::user_data(&game_name));
        // development mode: f5 reads and checks the files again; each
        // model read stays in memory, as the game borrows it (M 5.7)
        let reloader: Option<rlml_tui::term::Reloader<'_>> = dev.then(|| {
            Box::new(move || {
                let (m, report) = rlml_core::check_game(&files, &main);
                if report.has_errors() {
                    return Err(report.errors().map(|d| d.to_string()).collect());
                }
                let m: &'static rlml_core::model::Model = Box::leak(Box::new(m));
                Ok(m)
            }) as rlml_tui::term::Reloader<'_>
        });
        return match rlml_tui::term::play(
            &model,
            seed,
            log_out.as_deref(),
            data.as_deref(),
            reloader,
        ) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("{e}");
                ExitCode::FAILURE
            }
        };
    }
    if command == "run" {
        if report.has_errors() {
            for d in report.errors() {
                eprintln!("{d}");
            }
            return ExitCode::FAILURE;
        }
        let keys: Vec<&str> = keys.split_whitespace().collect();
        return match rlml_core::rt::play::transcript_and_log(&model, seed, &keys) {
            Ok((t, log)) => {
                print!("{t}");
                if let Some(path) = &log_out
                    && let Err(e) = write_log(path, &log)
                {
                    eprintln!("{e}");
                    return ExitCode::FAILURE;
                }
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("{e}");
                ExitCode::FAILURE
            }
        };
    }
    if command == "inspect" {
        if report.has_errors() {
            for d in report.errors() {
                eprintln!("{d}");
            }
            return ExitCode::FAILURE;
        }
        let keys: Vec<&str> = keys.split_whitespace().collect();
        let run = || -> Result<(String, bool), String> {
            let mut s = rlml_core::rt::play::Session::start(&model, seed)?;
            for k in &keys {
                s.press(k)?;
            }
            let mut out = String::new();
            let mut failed = false;
            let paths = if paths.is_empty() {
                vec![String::new()]
            } else {
                paths.clone()
            };
            for p in paths {
                match s.r.inspect(&p) {
                    Ok(node) => {
                        for line in node.lines(&p) {
                            out.push_str(&line);
                            out.push('\n');
                        }
                    }
                    // a wrong path is told in its place, and the rest go on
                    Err(e) => {
                        failed = true;
                        out.push_str(&format!("{e}\n"));
                    }
                }
            }
            Ok((out, failed))
        };
        return match run() {
            Ok((text, failed)) => {
                print!("{text}");
                if failed {
                    ExitCode::FAILURE
                } else {
                    ExitCode::SUCCESS
                }
            }
            Err(e) => {
                eprintln!("{e}");
                ExitCode::FAILURE
            }
        };
    }
    if command == "bench" {
        if report.has_errors() {
            for d in report.errors() {
                eprintln!("{d}");
            }
            return ExitCode::FAILURE;
        }
        let keys: Vec<&str> = keys.split_whitespace().collect();
        return match bench(&model, seed, &keys, repeat, loaded) {
            Ok(text) => {
                print!("{text}");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("{e}");
                ExitCode::FAILURE
            }
        };
    }
    if command == "replay" {
        if report.has_errors() {
            for d in report.errors() {
                eprintln!("{d}");
            }
            return ExitCode::FAILURE;
        }
        let Some(path) = log_in else {
            eprintln!("replay needs an input log\n\n{USAGE}");
            return ExitCode::from(2);
        };
        let log = match std::fs::read_to_string(&path)
            .map_err(|e| e.to_string())
            .and_then(|t| rlml_core::rt::input::InputLog::read(&t))
        {
            Ok(l) => l,
            Err(e) => {
                eprintln!("{}: {e}", path.display());
                return ExitCode::FAILURE;
            }
        };
        return match rlml_core::rt::play::replay_transcript(&model, &log) {
            Ok(t) => {
                print!("{t}");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("{e}");
                ExitCode::FAILURE
            }
        };
    }
    for d in &report.0 {
        println!("{d}");
    }
    let errors = report.errors().count();
    let warnings = report.warnings().count();
    if errors == 0 {
        let count = |k: Kind| model.defs.values().filter(|d| d.kind == k).count();
        println!(
            "ok: {} files, {} entities, {} components, {} maps, {} commands, {} handlers, {} languages",
            model.files.len(),
            count(Kind::Entity),
            count(Kind::Component),
            count(Kind::Map),
            count(Kind::Command),
            model
                .code
                .iter()
                .filter(|d| d.kind == rlml_core::rogue::DeclKind::Handler)
                .count(),
            model
                .languages
                .iter()
                .map(|f| f.language.as_str())
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
        );
    }
    println!("{errors} errors, {warnings} warnings");
    if errors == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn write_log(path: &Path, log: &rlml_core::rt::input::InputLog) -> Result<(), String> {
    let text = log.write()?;
    std::fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))
}

/// `rlml bench`: the keys `repeat` times from a new game, timed, and the
/// interpreter's counters when built with them (RLML-kehitystyokalut.md
/// step 1).
fn bench(
    m: &rlml_core::model::Model,
    seed: u64,
    keys: &[&str],
    repeat: u32,
    loaded: std::time::Duration,
) -> Result<String, String> {
    use rlml_core::rt::stats;
    let starting = std::time::Instant::now();
    let mut s = rlml_core::rt::play::Session::start(m, seed)?;
    let started = starting.elapsed();
    stats::reset();
    let playing = std::time::Instant::now();
    for _ in 0..repeat {
        for k in keys {
            s.press(k)?;
        }
    }
    let played = playing.elapsed();
    let counts = stats::read();
    let presses = keys.len() as u64 * u64::from(repeat);
    let ms = |d: std::time::Duration| d.as_secs_f64() * 1000.0;
    let mut out = format!(
        "seed {seed}: {} keys × {repeat} = {presses} presses\n",
        keys.len()
    );
    out.push_str(&format!("load and check  {:10.2} ms\n", ms(loaded)));
    out.push_str(&format!("new game        {:10.2} ms\n", ms(started)));
    out.push_str(&format!(
        "play            {:10.2} ms  ({:.3} ms a press)\n",
        ms(played),
        ms(played) / presses.max(1) as f64
    ));
    let g = &s.r.w.s.game;
    out.push_str(&format!("game time {}, turn {}\n", g.time, g.turn()));
    match counts {
        Some(counts) => {
            for (c, n) in counts {
                out.push_str(&format!(
                    "{:<15} {n:>12}  ({:.1} a press)\n",
                    c.name(),
                    n as f64 / presses.max(1) as f64
                ));
            }
        }
        None => out.push_str("counters: build with `--features stats`\n"),
    }
    if let Some(times) = stats::times() {
        for (sp, ns) in times {
            let t = ns as f64 / 1e6;
            out.push_str(&format!(
                "{:<15} {t:10.2} ms  ({:.0} % of play)\n",
                sp.name(),
                100.0 * t / ms(played).max(1e-9)
            ));
        }
    }
    Ok(out)
}
