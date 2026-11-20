#!/bin/sh
# Builds the browser version into www/: the wasm module (pkg/) and the
# game (game.json, bundle.sh). Serve www/ over http, as a module worker
# does not load from file://:
#   crates/rlml-web/build.sh ~/kapteeni-aaltola/roguelike
#   python3 -m http.server -d crates/rlml-web/www 8000
# Needs `rustup target add wasm32-unknown-unknown` and
# `cargo install wasm-bindgen-cli --version 0.2.130` (Cargo.toml's).
set -e
HERE=$(cd "$(dirname "$0")" && pwd)
GAME=${1:-$HOME/kapteeni-aaltola/roguelike}
cd "$HERE/../.."
cargo build -p rlml-web --release --target wasm32-unknown-unknown
wasm-bindgen --target web --no-typescript --out-dir "$HERE/www/pkg" target/wasm32-unknown-unknown/release/rlml_web.wasm
"$HERE/bundle.sh" "$GAME" "$HERE/www/game.json"
