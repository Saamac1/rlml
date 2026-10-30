//! RLML loader: parsing, loading and load-time checks (M 2). No I/O: files
//! come through [`load::Files`].

pub mod check;
pub mod error;
pub mod fields;
pub mod load;
pub mod model;
pub mod names;
pub mod rogue;
pub mod rs;
pub mod syntax;
pub mod token;

use error::Report;
use load::Files;
use model::Model;

/// Loads the game from `main` and runs every load-time check.
pub fn check_game(files: &dyn Files, main: &str) -> (Model, Report) {
    let (loaded, mut report) = load::load(files, main);
    let model = model::build(loaded, &mut report);
    check::check(&model, main, &mut report);
    (model, report)
}
