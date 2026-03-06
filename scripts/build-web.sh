#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR=$(cd "$(dirname "$0")/.." && pwd)
OUT_DIR="$ROOT_DIR/dist/web"
TARGET_DIR="$ROOT_DIR/target/wasm32-unknown-unknown/release"

cd "$ROOT_DIR"
RUSTC_WRAPPER= cargo build --target wasm32-unknown-unknown --release
mkdir -p "$OUT_DIR"
cp "$ROOT_DIR/web/index.html" "$OUT_DIR/index.html"
cp "$ROOT_DIR/web/mq_js_bundle.js" "$OUT_DIR/mq_js_bundle.js"
cp "$TARGET_DIR/mcommand.wasm" "$OUT_DIR/mcommand.wasm"

echo "web bundle ready at $OUT_DIR"
