//! RLML in a browser: the game runs in a Web Worker and sends the cells
//! that changed to the page, which draws them on a canvas as a terminal
//! would. Everything but the JavaScript glue (`wasm`) builds natively, so
//! it is tested with the rest of the workspace.

pub mod bundle;
pub mod cells;
pub mod game;
pub mod keys;

#[cfg(target_arch = "wasm32")]
mod wasm;
