#!/bin/sh
# Downloads prebuilt CLI archives from a GitHub Release, builds @matenadar/wasm
# locally (unless --skip-wasm), stages all 7 npm packages, and publishes them
# interactively (works with npm passkey / WebAuthn 2FA for initial 0.1.0 bootstrap
# before linking npm OIDC Trusted Publishing).
#
#   scripts/publish-npm-from-release.sh [v0.1.0] [--dry-run] [--skip-wasm]
set -eu
cd "$(dirname "$0")/.."

TAG=""
DRY_RUN=""
SKIP_WASM=0

for arg in "$@"; do
  case "$arg" in
    --dry-run) DRY_RUN="--dry-run" ;;
    --skip-wasm) SKIP_WASM=1 ;;
    v*) TAG="$arg" ;;
    *)
      if [ -z "$TAG" ]; then
        TAG="v$arg"
      else
        echo "unknown option: $arg" >&2
        exit 2
      fi
      ;;
  esac
done

if [ -z "$TAG" ]; then
  VER="$(node -p 'JSON.parse(require("fs").readFileSync("packages/cli/package.json","utf8")).version')"
  TAG="v$VER"
fi
VER="${TAG#v}"

TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT INT TERM

echo "Downloading GitHub Release ${TAG} archives..."
gh release download "$TAG" -R lvnr/matenadar -p "matenadar-${TAG}-*" -D "$TMPDIR/archives"

mkdir -p "$TMPDIR/extracted"
for target in aarch64-apple-darwin x86_64-apple-darwin x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu; do
  mkdir -p "$TMPDIR/extracted/$target"
  tar -xzf "$TMPDIR/archives/matenadar-${TAG}-${target}.tar.gz" -C "$TMPDIR/extracted/$target"
done
mkdir -p "$TMPDIR/extracted/x86_64-pc-windows-msvc"
unzip -q "$TMPDIR/archives/matenadar-${TAG}-x86_64-pc-windows-msvc.zip" -d "$TMPDIR/extracted/x86_64-pc-windows-msvc"

node scripts/stage-npm-cli.mjs \
  --version "$VER" \
  --out out/npm \
  --bin "aarch64-apple-darwin=$TMPDIR/extracted/aarch64-apple-darwin/matenadar" \
  --bin "x86_64-apple-darwin=$TMPDIR/extracted/x86_64-apple-darwin/matenadar" \
  --bin "x86_64-unknown-linux-gnu=$TMPDIR/extracted/x86_64-unknown-linux-gnu/matenadar" \
  --bin "aarch64-unknown-linux-gnu=$TMPDIR/extracted/aarch64-unknown-linux-gnu/matenadar" \
  --bin "x86_64-pc-windows-msvc=$TMPDIR/extracted/x86_64-pc-windows-msvc/matenadar.exe"

if [ "$SKIP_WASM" = 0 ]; then
  scripts/wasm.sh --test
fi

publish_if_new() {
  dir="$1"
  name="$(node -p "JSON.parse(require('fs').readFileSync('$dir/package.json','utf8')).name")"
  ver="$(node -p "JSON.parse(require('fs').readFileSync('$dir/package.json','utf8')).version")"
  if [ -z "$DRY_RUN" ] && npm view "${name}@${ver}" version >/dev/null 2>&1; then
    echo "Skipping ${name}@${ver} (already published)"
    return 0
  fi
  echo "Publishing ${name}@${ver} ${DRY_RUN}..."
  (cd "$dir" && npm publish --access public $DRY_RUN)
}

for dir in \
  out/npm/cli-darwin-arm64 \
  out/npm/cli-darwin-x64 \
  out/npm/cli-linux-x64-gnu \
  out/npm/cli-linux-arm64-gnu \
  out/npm/cli-win32-x64-msvc \
  out/npm/matenadar \
  packages/wasm
do
  publish_if_new "$dir"
done
