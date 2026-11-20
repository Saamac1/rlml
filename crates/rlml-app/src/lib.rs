//! The game as a front end drives it (M 7, M 8): keys, pointer, pads,
//! time and window size come in, frames go out. No terminal, threads or
//! clock of its own, so the terminal (`rlml-tui`) and the browser
//! (`rlml-web`) share it.

pub mod app;
pub mod cursor;
pub mod draw;
pub mod events;
pub mod input;
pub mod menu;
pub mod pad;
pub mod panel;
