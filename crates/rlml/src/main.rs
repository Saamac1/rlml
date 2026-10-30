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

GAME is the game's main file or its folder (then game.rlml, game.md or
game.rlml.md). The engine library holds lib/std.rlml; it is found with
--lib, the RLML_LIB environment variable or next to this workspace.";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut game = None;
    let mut lib = std::env::var_os("RLML_LIB").map(PathBuf::from);
    let mut i = 0;
    if args.first().map(String::as_str) != Some("check") {
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
            "-h" | "--help" => {
                println!("{USAGE}");
                return ExitCode::SUCCESS;
            }
            a if game.is_none() && !a.starts_with('-') => game = Some(PathBuf::from(a)),
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
