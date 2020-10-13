#!/usr/bin/env bash

set -euo pipefail
set -x

cd "$(dirname $0)/.."

cargo build --target wasm32-unknown-unknown --package pf2e-csheet-frontend "$@"

static="$(pwd)/static"
if [[ -z "${1:-}" ]]; then
    target="$(pwd)/target/wasm32-unknown-unknown/debug"
else
    target="$(pwd)/target/wasm32-unknown-unknown/release"
fi
rm -f pf2e-csheet-backend/static/pf2e-csheet_frontend*
wasm-bindgen --target web --no-typescript --out-dir="$static" "$target/pf2e_csheet_frontend.wasm"
cp "$target"/pf2e_csheet_frontend* "$static"
