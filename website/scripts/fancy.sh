#!/bin/sh
# Builds the 3D desk (github.com/lvnr/matenadar-site) for /fancy/ and puts it
# in dist/fancy/, so one deploy serves the site, the docs and the desk.
#
#   FANCY_SRC=~/projects/matenadar-site sh scripts/fancy.sh
#
# Run it after `astro build`. It stops with an error when the desk's checkout
# is missing, so a deploy never quietly drops /fancy/.
set -eu

src="${FANCY_SRC:-$HOME/projects/matenadar-site}"
here="$(cd "$(dirname "$0")/.." && pwd)"
dist="$here/dist"

if [ ! -f "$src/package.json" ]; then
	echo "fancy: no checkout of matenadar-site at $src (set FANCY_SRC)" >&2
	exit 1
fi
if [ ! -d "$dist" ]; then
	echo "fancy: no $dist; run astro build first" >&2
	exit 1
fi

out="$(mktemp -d)"
trap 'rm -rf "$out"' EXIT

(
	cd "$src"
	[ -d node_modules ] || npm ci --no-audit --no-fund
	npm run build -- --base /fancy/ --outDir "$out" --emptyOutDir
)

rm -rf "$dist/fancy"
mkdir -p "$dist/fancy"
# Everything but the Vite plugin's wrangler.json: this deploy has its own.
(cd "$out" && tar cf - --exclude ./wrangler.json .) | (cd "$dist/fancy" && tar xf -)
echo "fancy: $(du -sh "$dist/fancy" | cut -f1) in dist/fancy/"
