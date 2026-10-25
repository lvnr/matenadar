# Reverse Coloring for People Who Can't Draw — five pages in Matenadar

The book (`~/projects/amazon-kdp/projects/reverse-coloring`, 108 pages, 8 × 10 in with bleed) was typeset by a
per-book HTML/CSS engine printed by headless Chrome (`design/build/book.py` + `design/book.css`), with the 50
paintings placed full-bleed by Bookforge. This sample rebuilds five of its pages from the same sources.

| original | rebuilt | page | fidelity | what still differs |
|---|---|---|---|---|
| 1 | 1 | title: full-bleed wash, balanced 68 pt title, subtitle, author | 9.5 | nothing visible; every word within 0.75 pt |
| 4 | 2 | Seven small marks: image → arrow → image → text rows | 9.5 | nothing visible; text stacks within 1 pt |
| 8 | 4 | lesson opener "An edge": heading, lesson, example, rule, label row, prompt, upside-down hints | 9.5 | nothing visible; the lesson's last line needed the `pretty` stand-in |
| 9 | 5 | painting edge-1: one JPEG over the whole sheet | 10 | identical |
| 10 | 6 | nudge page: number + label, balanced prompt, upside-down hints, dashed cut line, scissors, vertical note | 9.5 | nothing visible |

Measured with `pdftotext -bbox` on both PDFs, every word on every page is within 0.75 pt of the original. Most are
within 0.1 pt horizontally. The leftover is a uniform +0.6–0.7 pt vertical offset, where Chrome snapped line boxes
to whole CSS pixels. The cut line, scissors and vertical note match pixel for pixel at 300 dpi
(`build/cut.png` while it exists).

Build: `python3 make.py` (≈0.2 s; it re-runs itself under `uv run --with fonttools` for the font metrics), then
`python3 ../../scripts/review.py .`. The engine lays out the 6 sheets in ≈65 ms (≈0.45 s for the whole
`matenadar build` with PDF and PNGs). `review.py`'s labels need an ImageMagick font map here:
`MAGICK_CONFIGURE_PATH=$PWD/build/magick` (a copy of the book's `design/magick/type.xml`).

## How it is built

- **Sources**: `manuscript/copy.json` (all text), `planning/order.json` / `pages.json` (painting order and part),
  `planning/tints.json` (the four colour tokens of each page), `planning/art-paths.json` (accepted JPEGs),
  `visuals/examples/*.jpg`, and the book's static fonts `design/fonts/RC*.ttf`. `doc.json`'s `fonts` points at
  `book/design/fonts`, so the engine uses exactly the files Chrome used.
- **Geometry**: each CSS box from `book.css` becomes a frame at the same trim coordinates. The verso pages use the
  master's outside-only bleed, so `--bleed` output has the book's media size (585 × 738 pt), with the bleed on the left of
  versos and the right of rectos.
- **The flex row** `.label` (a 34/50 pt numeral and a 7.7 pt caps label on one baseline, 0.16 in apart) is two
  frames. make.py computes the second frame's x from the numeral's advance width and its y so both baselines meet, using the
  fonts' hhea metrics. This is what the engine applies to CSS line boxes.
- **The marks rows** use a fixed-height frame with `valign: middle`. That is exactly CSS's `align-items: center` here.
- **Upside-down hints**: a frame anchored `bottom-left`, `rotate: 180`. The run-in "STUCK?" and the " · "
  separators use runs, with `word_spacing` standing in for CSS margins (`margin-right: .08in`, `margin: 0 .07in`).
- **Vertical cut note**: a horizontal frame turned 90° about its centre. make.py solves for the centre so that the
  rotated text starts 1.00 in down with its right edge 0.54 in from the trim, as `writing-mode: vertical-rl` put it.
- **Cut line, scissors, arrow**: book.py's inline SVGs, colours filled in from the tint tokens.
- **Typography**: book.py's `typog()` (curly quotes, ×) and its no-break space after a lone "a" in the hints are
  reproduced in make.py.

## Two stand-ins (see GAPS.md)

- `text-wrap: balance` on the title, subtitle and prompts: the engine's `balance: true` currently fails to compile.
  make.py narrows those frames to the balanced width that it measures from the font: the narrowest width that keeps the line
  count. Flip `ENGINE_BALANCE = True` once the engine's balance works.
- `text-wrap: pretty` on the lesson, intro and mark lines: make.py binds the last two words with a no-break space
  when the last word is short. That is what moved "colors meet." and "to add." onto the original's lines.
