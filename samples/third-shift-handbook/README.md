# Third Shift Handbook — five pages in Matenadar

The book: `~/projects/amazon-kdp/projects/third-shift-handbook`, 6 × 9 in, no bleed, colour. The published
interior was one HTML document built from page-structured markdown (`scripts/build.py`, `scripts/hb/`),
with margin-note positions and pen marks computed in Chrome at print time (`layout.js`).

```
uv run --with fonttools python3 -B samples/third-shift-handbook/make.py   # writes doc.json
python3 scripts/review.py samples/third-shift-handbook                    # builds, writes build/compare-NNN.png
```

`make.py` imports the book's own `scripts/hb/engine.py` (the manuscript parser, the HANDS table, its seeds)
and `scripts/hb/diagram.py` (the machine drawing) with bytecode writing switched off, so nothing is written
into the book. fontTools reads advances and vertical metrics from the bundled fonts.

## Pages

| book | rebuilt | page | fidelity | what still differs |
|---|---|---|---|---|
| 1 | 1 | title page | 8.5/10 | The struck note's scribbles are straight marks, not the book's wobbly double passes; letters do not turn |
| 7 | 3 | Section 1 opener (folio 1) | 8/10 | Two justified paragraphs break differently (Matenadar may tighten spaces, Chrome never does); the slip sits 3 pt right; letters do not turn |
| 25 | 5 | NOTES page (folio 19) | 9/10 | Line for line identical, on the rules; only the per-letter turns are missing |
| 30 | 6 | No. 3 machine (folio 24) | 9/10 | The drawing is the book's SVG; key, struck figure and both notes match |
| 75 | 7 | Notice of Reprimand (hand folio 65) | 9/10 | The form matches row for row; the tick is a little shorter; letters do not turn |

Seven pages: an empty page 2 (inserted by the section's `start: recto`) and page 4 keep pages 7 and 25 on
the recto. `page_from` sets the folios 1, 19, 24, 65.

Build: 7 pages, layout ≈ 45 ms, ≈ 0.5 s for the PDF, ≈ 0.8 s with PNGs. `check`: no issues.

## The handwriting

Every note is set from the book's source with the book's own seeded jitter. `hand()` in `make.py` makes the
same random draws in the same order as `engine.render_hand` (the note sizes come out exactly as measured in
the book: 13.01, 12.49, 12.22, 12.36 …), and turns them into runs:

- each letter's vertical nudge and each word's drifting baseline → run `baseline`;
- each letter's size wobble → run `size` (the paragraph is set at the largest possible size, so a nudged
  letter never opens its line);
- Ruth's `k` from Kalam Light at 0.9 em, Coyle's figures from Kalam Light at 0.8 em → run `font`/`size`;
  Ruth's 1.7 em full stop → a heavier dot (`outline`), because a larger run opens the line;
- `_x_` → an underline mark, `~x~` → a strike mark, both in the hand's pen width; a struck note gets a strike
  on every row, a second invisible copy with its strike lower, and the seeded slash across it;
- CSS lays letters out at their unscaled size and tracks every letter; one-letter runs take no tracking in
  Matenadar. Each word's width difference (scale, the wider full stop, the tracking) goes into the space
  before it, so every line ends where the book's did.
- Notes turn about their top-left corner in the book and about their centre in Matenadar; the position is
  corrected for that (`tl_rotation`).

What cannot be said: each letter's ±2.4° turn and each word's ±1.5° turn (see GAPS 1).

## How each page is made

- **Masters.** `rules` (3.4 in column, a 1.46 in note column on the right) and `wide` (5 in), both with the
  running foot as master foreground: a 1 px rule, then a table `1fr | auto | auto` (Rev. · title · folio,
  mirrored on the verso) with the smaller type lowered to the folio's baseline. The book's plan is not
  mirrored (print left, margin right on both sides, the sheet shifted 0.05 in toward the spine); the sample
  only flows text on a recto, so this held (see GAPS 4).
- **Title page.** Hand-placed: yellow rule, Jost Bold tracked, the title stack in League Gothic with an
  `outline` of size/32 standing in for Chrome's synthetic bold, the issue panel as two tables
  (`auto | 1fr | auto | 0.9 in`), Walt's two free notes.
- **Section opener.** A `section` flow: a canvas for the yellow chip and title, the mascot as the first
  `margin` block (notes then stack below it), h2 with the yellow chip as a highlight mark, justified
  Schola with old-style figures, the typed slip as a rotated `box` with seeded tape (the same draws as
  `parts.d_slip`), Coyle's entry as a rotated box, Ruth's two notes as `margin` blocks anchored as the book
  anchors them (`at=recall`, then the slip).
- **NOTES page.** Head, 24 blue rules as lines, one text frame with Ruth's five notes at a fixed 0.29 in line
  height lowered 0.045 in, and a one-rule `space_before` for each skip.
- **Machine.** Display head, `diagram.machine_svg()` placed as SVG (font names mapped to the real
  families), the key as one table (marker canvas | label, two columns 0.3 in apart), Walt's "38. NOT 36."
  free over the drawing, Ruth below.
- **Notice of Reprimand.** A group turned −0.7° holding a pink text frame whose blocks are the form's rows
  (canvases for label/value rows and check rows, boxes for the writing areas, explicit `space` blocks for
  the CSS margins), two tape strips, the red serial; the tick drawn from the box corners with the two
  seeded numbers `layout.js` uses (mulberry32 and FNV-1a are reimplemented and agree with Node); three notes
  below; Ruth's hand-written folio.

## What I learned about the original

- **The title page's clerk entries are not in the clerk's hand.** `.issue p` (Schola 8.4/11.4, 9 pt above)
  also matches the `<p>` inside the handwriting, so the printed book shows "W. R. Tolliver", "0417",
  "A. Gagnon", "11-14-79" in Schola with old-style figures. The rebuild copies what was printed.
- Chrome floors CSS borders to whole pixels: the slip's and form's 1–1.2 pt "shadow" edges print at the same
  0.75 pt as the 0.5 pt sides, so the thick-edge shadow never shows. The rebuild draws the printed widths.
- Chrome's synthetic bold on League Gothic (the title) is matched by `outline` 2.56 pt (82/32).

## Files

- `make.py` — builds `doc.json` (≈ 300 KB: one run per handwritten letter).
- `GAPS.md` — what the engine could not say, with the JSON I'd want to write, and bugs with repros.
