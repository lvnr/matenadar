#!/bin/sh
# Builds Matenadar for WebAssembly into dist/: matenadar.wasm, the engine
# wrapper (matenadar.js), the worker (worker.js), the page's client
# (client.js), their types, and REVISION (the commit built). With --test,
# also runs it in Node: the API test, and a sample through PDF and EPUB.
#
#   scripts/wasm.sh [--test] [--embedded-fonts]
set -eu
cd "$(dirname "$0")/.."

test=0
features=""
for arg in "$@"; do
  case "$arg" in
    --test) test=1 ;;
    --embedded-fonts) features="--features embedded-fonts" ;;
    *) echo "unknown option: $arg" >&2; exit 2 ;;
  esac
done

CARGO_INCREMENTAL=0 cargo build --profile wasm --target wasm32-unknown-unknown -p matenadar-wasm $features
mkdir -p dist
built=target/wasm32-unknown-unknown/wasm/matenadar_wasm.wasm
if command -v wasm-opt >/dev/null; then
  wasm-opt -Oz --enable-bulk-memory --enable-nontrapping-float-to-int --enable-sign-ext --enable-mutable-globals "$built" -o dist/matenadar.wasm
else
  cp "$built" dist/matenadar.wasm
fi
cp crates/matenadar-wasm/js/matenadar.js crates/matenadar-wasm/js/worker.js crates/matenadar-wasm/js/client.js \
  crates/matenadar-wasm/js/matenadar.d.ts crates/matenadar-wasm/js/client.d.ts dist/
git rev-parse HEAD > dist/REVISION
if [ -n "$(git status --porcelain -- crates Cargo.toml Cargo.lock)" ]; then echo "+uncommitted" >> dist/REVISION; fi
ls -l dist/matenadar.wasm

if [ "$test" = 1 ]; then
  fonts=$(ls "${CARGO_HOME:-$HOME/.cargo}"/registry/src/*/typst-assets-*/files/fonts/LibertinusSerif-*.otf 2>/dev/null)
  if [ -z "${MATENADAR_BIN:-}" ] && [ -x target/debug/matenadar ]; then MATENADAR_BIN=target/debug/matenadar; export MATENADAR_BIN; fi
  # shellcheck disable=SC2086
  node crates/matenadar-wasm/tests/node.mjs dist/matenadar.wasm $fonts
  # A sample from its folder through layout, PDF and both EPUBs, with times and peak memory.
  node crates/matenadar-wasm/tests/smoke.mjs dist/matenadar.wasm samples/puzzles --fonts "$(dirname "$(echo "$fonts" | head -1)")"
fi
