//! The JavaScript side's calls (`wasm-bindgen`): the page maps its keys
//! with `key_binding`, and the worker plays a `WebGame`.

use std::time::Duration;

use js_sys::{Object, Reflect};
use rlml_app::events::Event;
use termplate_core::keys::parse_binding;
use wasm_bindgen::prelude::*;

use crate::game::Game;

/// The binding name of a `keydown` (`k`, `K`, `ctrl+q`, `space`, `up`),
/// or `undefined` for a key the terminal has no name for.
#[wasm_bindgen]
pub fn key_binding(key: &str, code: &str, ctrl: bool, alt: bool, shift: bool) -> Option<String> {
    crate::keys::binding(key, code, ctrl, alt, shift).map(|b| b.name())
}

fn ms(t: f64) -> Duration {
    Duration::from_secs_f64(t.max(0.0) / 1000.0)
}

fn err(e: String) -> JsValue {
    JsValue::from_str(&e)
}

/// `bundle[field]` as (path, text) pairs.
fn tree(bundle: &JsValue, field: &str) -> Result<Vec<(String, String)>, JsValue> {
    let obj = Reflect::get(bundle, &JsValue::from_str(field))?;
    if !obj.is_object() {
        return Err(err(format!("game.json: `{field}` is missing")));
    }
    let mut out = Vec::new();
    for pair in Object::entries(obj.unchecked_ref::<Object>()) {
        let pair: js_sys::Array = pair.unchecked_into();
        let (path, text) = (pair.get(0).as_string(), pair.get(1).as_string());
        match (path, text) {
            (Some(p), Some(t)) => out.push((p, t)),
            _ => return Err(err(format!("game.json: `{field}` holds a file that is no text"))),
        }
    }
    Ok(out)
}

#[wasm_bindgen]
pub struct WebGame {
    g: Game,
}

#[wasm_bindgen]
impl WebGame {
    /// Loads and checks the bundle (`game.json` parsed) and starts a game
    /// in a window of `cols`×`rows` cells. Load and check errors are
    /// thrown, one a line.
    #[wasm_bindgen(constructor)]
    pub fn new(bundle: JsValue, seed: u32, cols: usize, rows: usize) -> Result<WebGame, JsValue> {
        let main = Reflect::get(&bundle, &JsValue::from_str("main"))?.as_string().unwrap_or_else(|| "game.rlml".into());
        let files = crate::bundle::files(tree(&bundle, "game")?, tree(&bundle, "engine")?);
        let m = crate::bundle::load(&files, &main).map_err(err)?;
        // the app borrows the model, and a wasm-bindgen class carries no
        // borrow: the model lives as long as the page
        let m = Box::leak(Box::new(m));
        Ok(WebGame { g: Game::new(m, seed as u64, (cols, rows)).map_err(err)? })
    }

    /// A key by its binding name, for the next step.
    pub fn key(&mut self, name: &str) {
        if let Ok(b) = parse_binding(name) {
            self.g.push(Event::Key(b));
        }
    }

    pub fn has_events(&self) -> bool {
        self.g.has_events()
    }

    /// The events since the last step; `now` in ms.
    pub fn input(&mut self, now: f64) -> Result<(), JsValue> {
        self.g.input(ms(now)).map_err(err)
    }

    pub fn tick(&mut self, now: f64) -> Result<(), JsValue> {
        self.g.tick(ms(now)).map_err(err)
    }

    /// The next in-between frame's changed cells (`cells.rs`).
    pub fn next_frame(&mut self) -> Option<Vec<u32>> {
        self.g.next_frame()
    }

    /// Now's changed cells (`cells.rs`).
    pub fn draw(&mut self) -> Vec<u32> {
        self.g.draw()
    }

    pub fn quit(&self) -> bool {
        self.g.quit()
    }

    pub fn restart(&mut self) -> Result<(), JsValue> {
        self.g.restart().map_err(err)
    }

    pub fn wait_ms(&self) -> f64 {
        self.g.wait().as_secs_f64() * 1000.0
    }

    pub fn frame_ms(&self) -> f64 {
        self.g.frame_time().as_secs_f64() * 1000.0
    }

    pub fn bound_keys(&self) -> Vec<String> {
        self.g.bound_keys().into_iter().collect()
    }

    pub fn log_lines(&self) -> Vec<String> {
        self.g.log_lines()
    }
}
