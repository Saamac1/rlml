//! RLML loader: parsing, loading and load-time checks (M 2). No I/O: files
//! come through [`load::Files`].

pub mod check;
pub mod devmod;
pub mod error;
pub mod fields;
pub mod load;
pub mod model;
pub mod names;
pub mod rogue;
pub mod rs;
pub mod rt;
pub mod syntax;
pub mod token;

use error::Report;
use load::Files;
use model::Model;

/// Loads the game from `main` and runs every load-time check.
pub fn check_game(files: &dyn Files, main: &str) -> (Model, Report) {
    check_game_with(files, main, &load::Layers::default())
}

/// `check_game` with the active mods and the player's `overrides/` loaded
/// after the game's own files (D 3.2, D 3.6).
pub fn check_game_with(files: &dyn Files, main: &str, layers: &load::Layers) -> (Model, Report) {
    let (loaded, mut report) = load::load_layers(files, main, layers);
    let model = model::build(loaded, &mut report);
    check::check(&model, main, &mut report);
    (model, report)
}
