# Christmas Word Search: Hidden Messages & Festive Facts — five pages in Matenadar

The book (`~/projects/amazon-kdp/projects/q4-02-christmas-word-search`, 178 pages, 8 × 10 in, no bleed, grayscale)
was set by a project XeLaTeX + TikZ engine (`design/build/pages.py`, `frontmatter.py`, `make_book.py`): every page
is a TikZ canvas with absolute coordinates. The exact final source is `tmp/interior-build/interior.tex`. This sample
rebuilds five pages from the book's own data and art.

| original | rebuilt | page | fidelity | what still differs |
|---|---|---|---|---|
| 4 | 2 | How to Solve: badge steps, arrow table, worked example with loops | 9.5 | nothing visible; words within 0.4 pt |
| 6 | 4 | chapter 1 plate: knit bands, double frame, potrace woodcut | 9.5 | the band art sits ~0.25 pt lower |
| 7 | 5 | chapter 1 opener: centred display type, ornament, mini contents with dotted leaders, direction key | 9 | leader dots are spread evenly, so they don't sit on TikZ's fixed 4 pt pitch; one intro line 0.5 pt right |
| 8 | 6 | puzzle 1: 17 × 13 grid, word bank, key arrows, hidden-message blanks, running foot | 9.5 | nothing visible; words within 0.4 pt |
| 127 | 7 | solutions 1 and 2: grey capsules with white edges, bold message letters, grey fillers, side column | 9.5 | side column 0.2 pt high; "HIDDEN MESSAGE" word gap 0.55 pt wide |

`pdftotext -bbox` puts every word on all five pages within 0.4 pt of the original, apart from the four noted
above. Word widths match to 0.01 pt. Grids, capsules, frames and blanks overlay pixel for pixel at 300 dpi.
Physical pages 1 and 3 are fillers that keep each page on its original side.

Build: `python3 make.py` (≈0.5 s; it re-runs itself under `uv run --with fonttools --with uharfbuzz` for
measuring), then `python3 ../../scripts/review.py .` with `MAGICK_CONFIGURE_PATH=$PWD/build/magick` for
the sheet labels. The engine lays out the 7 sheets in ≈35 ms (≈0.44 s for the whole `matenadar build`).
Chaparral Pro (licensed Adobe font) and Source Sans Pro come from `/Library/Fonts`, as in the book.

## How it is built

- **Data**: `planning/content-v2.json` (titles, words, levels, types, messages), `puzzles/v2/layouts.json`
  (grids and placements). The book's `design/build/pages.py` is imported read-only, without writing bytecode. It
  supplies the geometry constants (`BLOCK_W`, `CELL`, `GRID_TOP`, the drops under the grid, `SOL_TOPS`), the
  word-bank widths table and `blanks_layout()` for the write-in lines. `frontmatter.py` and `make_book.py` are
  parsed with `ast` (not run) for the How to Solve text, the arrow-table rows, the worked example, the chapter
  intros and motifs.
- **Art**: the knit bands and motifs are the book's own SVGs (`design/art/knit/*.svg`, stitch rectangles), placed
  stretched to 6.035 × 0.19 in as the PDFs were. The plate is the potrace SVG (`design/art/plates/chapter-01.svg`),
  contain-fitted in the frame. The direction arrows are SVG markup that transcribes TikZ's `Stealth` tip
  (`pgflibraryarrows.meta`): the capped 0.709 pt outline, front and back miters, and the inset.
- **Grids** are `letter-grid` elements. The puzzle uses Source Sans Pro Semibold 17.5 in 0.355 in cells. The
  solutions use Regular, with `weight: 700` on leftover message cells and `color: soft` (0.42 grey) on fact fillers.
  Every word placement becomes a highlight capsule with thickness 0.8 cell, fill grey 0.80 and a white 0.9 pt edge.
  The engine's capsule (length + thickness, radius thickness/2) is exactly the book's (r = 0.4 cell, radius
  0.995 r). The worked example's loops are stroke-only highlights with thickness 0.84. The rounded frames are
  separate `rect`s, because the book's frame sits 0.045 in outside the cells.
- **TikZ nodes → frames**: TikZ anchors text by its baseline (`base west/east/base`), or by the top of the first
  line's ink (`north` with `text width`). make.py places each frame so its first baseline lands there. It computes
  the offset from the fonts' vertical metrics (the engine's CSS line model) and, for `north` nodes, the ink height of
  the first line from HarfBuzz. Right and centre anchors are right- and centre-aligned frames. The engine's hanging
  punctuation is taken back out on single lines. The How to Solve steps follow TeX's vertical rules:
  `\vspace`, `\baselineskip`, `\lineskip`, and the badge's height and depth inside the step's first line.
- **TeX points**: every size, leading and skip in the book is in TeX points (1/72.27 in), so make.py scales them by
  72/72.27. Without that, lines drift 0.07 pt each and words 0.4% wide.
- **Running foot**: the master's foreground holds the folio (`{"field": "page"}`, outside edge by `when:
  recto/verso`) and the centred chapter name (`{"field": "section.title"}` from the page's `title`). Pages opt in
  with `layers: true`, and the plate doesn't. `page_from` numbers the sheets 4, 6, 7, 8, 127. "Solution on page 127",
  "Solutions begin on page 127" and "Puzzle on page 8" are live `ref`s. Puzzles 2–10 aren't in the sample, so their
  page numbers are computed.
- **Leaders**: the engine's inline `leader`, using a Chaparral period sized and lowered to TikZ's 1.15 pt round
  dots on the baseline, with 0.08 in spacers either side.

## Line breaking is TeX's, computed in make.py

TikZ `text width` nodes are ragged with finite glue (`\rightskip 0pt plus 2em`, centred: both skips), so TeX
evens the lines out and stretches the spaces a little. LaTeX's `\raggedright` parboxes may shrink spaces, and
microtype hangs end punctuation. The engine's ragged text is first-fit with fixed spaces, so make.py runs a small
Knuth–Plass over the words (HarfBuzz widths). It passes the engine explicit line breaks and each line's glue set as
`word_spacing`. Frames get 6 pt of slack so the engine never re-breaks. Two measured details: TikZ nodes without a
node font take 10 pt Latin Modern's 3.32 pt spaces with no stretch (the solutions' side column), and XeTeX's
LetterSpace does not widen interword spaces.

`MT_NATIVE_BREAKS=1 python3 make.py` writes `build/doc-native.json`, which uses the engine's own breaking instead.
Then step 4 of How to Solve pushes "blanks." to the next line, the chapter intro fills its first two lines greedily
("…works its way in: a / wreath, …"), the worked example breaks after "left", and the solution message breaks after
"house" and "single". Line counts and everything else stay the same.
