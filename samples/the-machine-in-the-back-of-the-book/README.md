# The Machine in the Back of the Book: five pages in Matenadar

The book was set with its own HTML/CSS engine: `scripts/build.py` composes the Markdown manuscript,
Chrome lays it out, `scripts/paginate.js` pours the flows, places the margin notes and adds the folios,
and a post-layout script draws the red strike lines. This sample rebuilds five of its 114 pages from the
same sources: the manuscript Markdown, `manuscript/documents/cost-review.md`, `manuscript/part2/notes.md`,
the assembled cartridge bytes (`machine/build/t4-sums.bin`, with the book's record and check-byte rules),
the prepared art in `build/art/` and the OFL fonts in `assets/fonts/` (Caveat and Archivo used straight
from their variable fonts).

```
python3 make.py && python3 ../../scripts/review.py .
```

## Pages

| original | rebuilt | page | how | score |
|---|---|---|---|---|
| 9 | 1 | Chapter 1 opener (recto) | section on master `chapter`: teal band, kicker, number and title from section fields (`when: first`); the art is a canvas at the top of the flow; text column and margin notes from the flow | 8 / 10 |
| 14 | 2 | Chapter 2, third page (verso, the tight "t2" pour) | section on master `flow`: list, red and blue notes, struck paragraph (`strike` marks), the underside figure with the owner's plate engraved by a rotated and skewed group, caption, list, paragraph | 9 / 10 |
| 27 | 3 | Castellan cost review | fixed page: one group rotated −0.5° holding the sheet, a text frame of blocks (letterhead canvas, typewriter tables with bottom-aligned header and dotted rules), the red pen beside rows 7–10 and the blue note rotated −1.5° | 9 / 10 |
| 29 | 5 | Chapter 7, second page (recto) | section on master `flow`: code continuation, two tables, margin note, build box, listing head, listing group with the QR code of the same bytes in the margin | 9.5 / 10 |
| 48 | 6 | Cartridge 1 label page (verso) | fixed page: tinted ground into the bleed, the cartridge drawn with rects (per-corner radii), ribs, label art, teal band with Archivo at wdth 125 and 68 | 9.5 / 10 |

Rebuilt page 4 is the blank that puts p. 29 on a recto. Build: 6 pages, layout about 40 ms, about
0.6 s for the PDF (most of it loading fonts); `matenadar check` reports no issues.

Measured against the original with PyMuPDF (text baselines of both PDFs, matched line by line):

| page | lines matched | baseline within 1 pt | worst |
|---|---|---|---|
| 9 | 24 / 29 | 6 | 14 pt (one line fewer in the first paragraph, see below) |
| 14 | 34 / 37 | 34 | 0.8 pt |
| 27 | 61 / 73 | 53 (all within 1.3 pt) | 1.3 pt |
| 29 | 46 / 53 | 46 | 0.7 pt |
| 48 | 4 / 4 | 4 | 0.5 pt |

(Unmatched lines are handwriting, where word spacing makes text extraction differ, not misplacement.)
x positions agree within 1 pt. The QR code decodes to exactly the book's payload (version 10, ECC M,
the book's 0.0185 in module); only its mask pattern differs.

## What differs, and why

- **p. 9, hyphenation.** Typst hyphenates with TeX patterns; Chrome on macOS used the system
  dictionary. The first paragraph breaks "It de-|scribes" here (Chrome: "It | describes"), so it takes
  three lines instead of four and everything below sits 14 pt higher. "memo-ry" becomes "mem-ory" in the
  third paragraph. A hyphenation exception list would fix both (GAPS.md).
- **p. 14, the red pen.** The book draws each strike as a quadratic curve with seeded jitter, 0.94 pt,
  round caps. Here a `strike` mark draws straight lines (same colour, width, position and overhang).
- **Lists are two-column tables** (`row_gap` for the item gap) because the `list` block does not compile
  in the current engine (GAPS.md). The marker column right-aligns "8." and draws the disc a third of an
  em across, as Chrome does.
- **Tables** are a header table over a body table: header and body cells have different padding and
  rules, and a table has one `inset` and one `stroke`. Rules are bottom-only cell strokes.
- **Two notes beside one block** (red, then blue) share one margin block with 7 pt between them; a
  second margin block anchors to the next block instead of stacking under the first.

## Chrome habits copied on purpose

These are not engine faults, but the book printed them, so `make.py` reproduces them:

- **Border widths snap to whole CSS pixels** (0.75 pt), never under one: the 1.1 pt and 0.45 pt table
  rules, the 0.4 pt listing hairlines and dotted rows all printed 0.75 pt; the 2 pt build-box rule and
  1.6 pt code rule printed 1.5 pt (measured at 1200 dpi). `bw()` applies the rule, and since a CSS border
  adds its width to a row while a cell stroke sits on the grid line, half of each rule goes into the insets.
- **Trailing letter-spacing** stays inside right-aligned lines in Chrome, so the recto running head and
  the cartridge number carry a right margin of one letter's tracking.
- **Half-pixel baselines** (13.5 / 14.25 pt alternating around 14 pt) are not imitated.
- `rule_above`'s `gap` is measured from the top of the rule, so the h2 bar uses `gap: 7.2` (2.2 pt bar +
  5 pt space), and every canvas, box and table carries explicit `space_before` / `space_after`, since
  non-paragraph blocks default to 0.8 em around them.

## What worked without fuss

Masters with recto/verso running heads and folios from section fields; the outer note column with
anchored handwriting in two fonts; full-bleed bands and grounds; per-corner radii; per-side box borders
(code rule, build box); rotated groups; rotate + skew for the engraved plate (the book's affine matrix
decomposed into a 13.87° rotation and a 13.97° skew lands on the brass plate exactly); Archivo at
wdth 68 and 125 from `weight` + `stretch`; the QR element at a fixed module size; table `valign: bottom`
for the cost review's wrapped "Unit saving" header; `case`, `tracking`, `features: ["zero"]`,
keep-with-next headings.
