#!/bin/sh
# Builds Matenadar for WebAssembly into dist/: matenadar.wasm and the
# matenadar.js wrapper. With --test, also runs it in Node.
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
cp crates/matenadar-wasm/js/matenadar.js dist/
ls -l dist/matenadar.wasm

if [ "$test" = 1 ]; then
  font=$(ls "${CARGO_HOME:-$HOME/.cargo}"/registry/src/*/typst-assets-*/files/fonts/LibertinusSerif-Regular.otf 2>/dev/null | head -1)
  node crates/matenadar-wasm/tests/node.mjs dist/matenadar.wasm "$font"
fi
