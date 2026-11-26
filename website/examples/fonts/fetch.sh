#!/bin/sh
# The examples' faces: all SIL Open Font License, from github.com/google/fonts.
# Run from anywhere: sh website/examples/fonts/fetch.sh
set -e
cd "$(dirname "$0")"
B=https://github.com/google/fonts/raw/main/ofl
for f in \
  "instrumentsans/InstrumentSans%5Bwdth,wght%5D.ttf" \
  "instrumentsans/InstrumentSans-Italic%5Bwdth,wght%5D.ttf" \
  "instrumentserif/InstrumentSerif-Regular.ttf" \
  "instrumentserif/InstrumentSerif-Italic.ttf" \
  "fraunces/Fraunces%5BSOFT,WONK,opsz,wght%5D.ttf" \
  "fraunces/Fraunces-Italic%5BSOFT,WONK,opsz,wght%5D.ttf" \
  "jetbrainsmono/JetBrainsMono%5Bwght%5D.ttf" \
  "bricolagegrotesque/BricolageGrotesque%5Bopsz,wdth,wght%5D.ttf" \
  "geist/Geist%5Bwght%5D.ttf" \
  "newsreader/Newsreader%5Bopsz,wght%5D.ttf" \
  "newsreader/Newsreader-Italic%5Bopsz,wght%5D.ttf"; do
  name=$(basename "$f" | sed 's/%5B/[/; s/%5D/]/')
  [ -f "$name" ] || curl -fsSL "$B/$f" -o "$name"
done
echo "fonts ready in $(pwd)"
