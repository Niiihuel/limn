#!/bin/sh
# Builds the gallery for the browser into public/wasm, where the pages
# embed it, and lists its components in src/data/pages.json. Needs the
# wasm32-unknown-unknown target and a wasm-bindgen of the version in
# Cargo.lock: `cargo install wasm-bindgen-cli --version <it>`.
set -eu
cd "$(dirname "$0")/.."
# The components, one page each: the site gives each its address.
# Written whole or not at all: a failed build must not leave the list empty.
cargo run -q -p limn-gallery -- --pages > site/src/data/pages.json.new
mv site/src/data/pages.json.new site/src/data/pages.json
# Size over speed: every visitor downloads this.
CARGO_PROFILE_RELEASE_OPT_LEVEL=s CARGO_PROFILE_RELEASE_LTO=true \
CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1 CARGO_PROFILE_RELEASE_STRIP=true \
  cargo build --release -p limn-gallery --target wasm32-unknown-unknown
PATH="$PWD/target/tools/bin:$PATH" wasm-bindgen --target web --no-typescript \
  --out-dir site/public/wasm target/wasm32-unknown-unknown/release/limn-gallery.wasm
