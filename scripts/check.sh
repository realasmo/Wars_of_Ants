#!/usr/bin/env bash
# Full verification: Rust (fmt/clippy/test), WASM build, client build.
set -euo pipefail
cd "$(dirname "$0")/.."

echo "== cargo fmt =="
cargo fmt --all -- --check

echo "== cargo clippy =="
cargo clippy --workspace --all-targets -- -D warnings

echo "== cargo test =="
cargo test --workspace

echo "== wasm-pack build core =="
(cd core && wasm-pack build --target web --dev)

echo "== client build =="
(cd client && npm ci && npm run build)

echo "== ALL GREEN =="
