#!/usr/bin/env bash
set -euo pipefail

PROJECT_ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
CURRENT_DIR="$PROJECT_ROOT/wasm"

# TODO: Ensure that version is up to date
cp "$PROJECT_ROOT/README.md" "$CURRENT_DIR/"
cp "$PROJECT_ROOT/LICENSE.md" "$CURRENT_DIR/"

# Remove outputs from a previous build so stale renamed or generated files cannot leak in
rm -rf \
    "$CURRENT_DIR/stylua.bundler" \
    "$CURRENT_DIR/stylua.node-cjs" \
    "$CURRENT_DIR/stylua.node-esm" \
    "$CURRENT_DIR/stylua.deno" \
    "$CURRENT_DIR/stylua.web"
rm -f "$CURRENT_DIR"/styluau_lib*

cargo build --manifest-path "$PROJECT_ROOT/Cargo.toml" \
    --target wasm32-unknown-unknown --release \
    --features lua52,lua53,lua54,luajit,luau,cfxlua

WASM_PATH="$PROJECT_ROOT/target/wasm32-unknown-unknown/release/styluau_lib.wasm"

# install wasm-bindgen-cli matching the version in Cargo.toml
WASM_BINDGEN_VERSION=$(sed -n 's/.*wasm-bindgen = { version = "\([^"]*\)".*/\1/p' "$PROJECT_ROOT/Cargo.toml")
command -v wasm-bindgen && [ "$(wasm-bindgen --version | awk '{print $2}')" = "$WASM_BINDGEN_VERSION" ] || cargo install wasm-bindgen-cli --version "$WASM_BINDGEN_VERSION" --locked

# check wasm-bindgen version
wasm-bindgen --version

wasm-bindgen "$WASM_PATH" --target bundler --out-dir "$CURRENT_DIR/stylua.bundler"
wasm-bindgen "$WASM_PATH" --target nodejs --out-dir "$CURRENT_DIR/stylua.node-cjs"
wasm-bindgen "$WASM_PATH" --target experimental-nodejs-module --out-dir "$CURRENT_DIR/stylua.node-esm"
wasm-bindgen "$WASM_PATH" --target deno --out-dir "$CURRENT_DIR/stylua.deno"
wasm-bindgen "$WASM_PATH" --target web --out-dir "$CURRENT_DIR/stylua.web"

cp -R "$CURRENT_DIR/stylua.bundler/." "$CURRENT_DIR/"

cp "$CURRENT_DIR/stylua.node-cjs/styluau_lib.js" "$CURRENT_DIR/styluau_lib_node.cjs"
cp "$CURRENT_DIR/stylua.node-cjs/styluau_lib.d.ts" "$CURRENT_DIR/styluau_lib_node.d.cts"

cp "$CURRENT_DIR/stylua.node-esm/styluau_lib.js" "$CURRENT_DIR/styluau_lib_node.mjs"
cp "$CURRENT_DIR/stylua.node-esm/styluau_lib.d.ts" "$CURRENT_DIR/styluau_lib_node.d.mts"

cp "$CURRENT_DIR/stylua.deno/styluau_lib.js" "$CURRENT_DIR/styluau_lib_deno.js"
cp "$CURRENT_DIR/stylua.deno/styluau_lib.d.ts" "$CURRENT_DIR/styluau_lib_deno.d.ts"

cp "$CURRENT_DIR/stylua.web/styluau_lib.js" "$CURRENT_DIR/styluau_lib_web.js"
cp "$CURRENT_DIR/stylua.web/styluau_lib.d.ts" "$CURRENT_DIR/styluau_lib_web.d.ts"

echo "Build complete!"

# Run smoke tests if --test flag is passed
if [ "${1:-}" = "--test" ]; then
    echo "Running smoke tests..."
    cd "$CURRENT_DIR" && npm test
fi
