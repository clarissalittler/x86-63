#!/usr/bin/env bash
set -euo pipefail

root_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root_dir"

cargo build --release -p x86-63-wasm --target wasm32-unknown-unknown

if command -v wasm-bindgen >/dev/null 2>&1; then
    wasm_bindgen="$(command -v wasm-bindgen)"
elif [[ -x /tmp/x86-63-tools/bin/wasm-bindgen ]]; then
    wasm_bindgen=/tmp/x86-63-tools/bin/wasm-bindgen
else
    echo "wasm-bindgen CLI 0.2.126 is required." >&2
    echo "Install it with: cargo install wasm-bindgen-cli --version 0.2.126 --locked" >&2
    exit 1
fi

mkdir -p apps/web/src/generated
"$wasm_bindgen" \
    target/wasm32-unknown-unknown/release/x86_63_wasm.wasm \
    --target web \
    --typescript \
    --out-name x86_63_wasm \
    --out-dir apps/web/src/generated
