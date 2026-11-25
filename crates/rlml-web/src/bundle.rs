//! The game as one file (`bundle.sh`): `{ "main": "game.rlml", "game":
//! {path: text}, "engine": {path: text} }`, the game's paths from its
//! folder and the engine's from the repository (`lib/std.rlml`), as the
//! disk loader finds them.

use rlml_core::load::MemFiles;
use rlml_core::model::Model;

/// The bundle's files as the loader reads them.
pub fn files(
    game: impl IntoIterator<Item = (String, String)>,
    engine: impl IntoIterator<Item = (String, String)>,
) -> MemFiles {
    MemFiles {
        game: game.into_iter().collect(),
        engine: engine.into_iter().collect(),
    }
}

/// The checked game, or its load and check errors one a line.
pub fn load(files: &MemFiles, main: &str) -> Result<Model, String> {
    let (m, report) = rlml_core::check_game(files, main);
    if report.has_errors() {
        return Err(report
            .errors()
            .map(|d| d.to_string())
            .collect::<Vec<_>>()
            .join("\n"));
    }
    Ok(m)
}
