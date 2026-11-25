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
            if e.path().is_dir() { out.push(format!("{name}/")) } else { out.push(name) }
        }
        Some(out)
    }
}

const USAGE: &str = "usage: rlml check GAME [--lib DIR]
       rlml run GAME [--seed N] [--keys \"KEY KEY …\"] [--log-out FILE] [--lib DIR]
       rlml play GAME [--seed N] [--log-out FILE] [--saves DIR] [--lib DIR]
       rlml replay GAME LOG [--lib DIR]

check loads the game and reports load errors and warnings (M 2).
run plays a new game without a screen: each key is one player's turn,
and the map, HP and log are printed after it. Keys are the key names of
the game's `keys` contexts, separated by spaces (`l l > j`).
play plays a new game in the terminal: the game's keys, esc for its
pause menu where it binds one, ctrl+c to leave at once. A runtime error
ends the game and is printed after the terminal is restored.
--log-out writes the session's input log (M 4.4) to FILE; with play it
is the last game's log, written also when a runtime error ends it.
play keeps its saves in DIR, by default the user data folder's
rlml/GAME/saves (~/.local/share on Linux).
replay plays an input log and prints a transcript as run does: the
frame after each row of the log.

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
    let mut saves: Option<PathBuf> = None;
    let mut i = 0;
    let command = args.first().cloned().unwrap_or_default();
    if !["check", "run", "play", "replay"].contains(&command.as_str()) {
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
            "--saves" if command == "play" && i + 1 < args.len() => {
                saves = Some(PathBuf::from(&args[i + 1]));
                i += 1;
            }
            "--keys" if command == "run" && i + 1 < args.len() => {
                keys = args[i + 1].clone();
                i += 1;
            }
            "-h" | "--help" => {
                println!("{USAGE}");
                return ExitCode::SUCCESS;
            }
            a if game.is_none() && !a.starts_with('-') => game = Some(PathBuf::from(a)),
            a if command == "replay" && log_in.is_none() && !a.starts_with('-') => log_in = Some(PathBuf::from(a)),
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
        let main = ["game.rlml", "game.rlml.md", "game.md"].into_iter().find(|m| game.join(m).is_file());
        match main {
            Some(m) => (game.clone(), m.to_string()),
            None => {
                eprintln!("{}: no game.rlml, game.rlml.md or game.md", game.display());
                return ExitCode::from(2);
            }
        }
    } else {
        let root = game.parent().map(Path::to_path_buf).unwrap_or_default();
        (root, game.file_name().unwrap().to_string_lossy().to_string())
    };
    // the workspace's own `lib` folder holds the standard library
    let engine = lib.or_else(|| Some(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")).filter(|p| p.join("lib/std.rlml").is_file()));
    let files = DiskFiles { game: root, engine };
    let (model, report) = rlml_core::check_game(&files, &main);
    if command == "play" {
        if report.has_errors() {
            for d in report.errors() {
                eprintln!("{d}");
            }
            return ExitCode::FAILURE;
        }
        let game_name = model.named_game().map(|g| g.name.clone()).unwrap_or_default();
        let saves = saves.or_else(|| rlml_tui::saves::user_data(&game_name).map(|d| d.join("saves")));
        return match rlml_tui::term::play(&model, seed, log_out.as_deref(), saves.as_deref()) {
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
        let log = match std::fs::read_to_string(&path).map_err(|e| e.to_string()).and_then(|t| rlml_core::rt::input::InputLog::read(&t)) {
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
            model.code.iter().filter(|d| d.kind == rlml_core::rogue::DeclKind::Handler).count(),
            model.languages.iter().map(|f| f.language.as_str()).collect::<std::collections::BTreeSet<_>>().len(),
        );
    }
    println!("{errors} errors, {warnings} warnings");
    if errors == 0 { ExitCode::SUCCESS } else { ExitCode::FAILURE }
}

fn write_log(path: &Path, log: &rlml_core::rt::input::InputLog) -> Result<(), String> {
    let text = log.write()?;
    std::fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))
}
