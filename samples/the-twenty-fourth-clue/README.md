# The Twenty-Fourth Clue — five pages in Matenadar

The book: `~/projects/amazon-kdp/projects/the-twenty-fourth-clue`, 8.5 × 11 in, no bleed, black ink.
The published interior was one HTML document (`scripts/e2/build.py`, `book.css`, `parts.py`) printed by Chrome.

```
python3 samples/the-twenty-fourth-clue/make.py            # writes doc.json and build/art/*.svg
python3 scripts/review.py samples/the-twenty-fourth-clue  # builds, writes build/compare-NNN.png
```

`make.py` needs nothing but Python 3. It reads the day-1 manuscript (`manuscript/days/day-01.md`),
`book2/spec/days.json`, the day-14 puzzle spec and drawing (`puzzles/e2/day-14/`), the town map drawing
(`puzzles/e2/map/town.html`), the stamp and plate pictures, and the bundled fonts' vertical metrics.

## Pages

| book | rebuilt | page | fidelity | what still differs |
|---|---|---|---|---|
| 2 | 2 | parcel label | 9/10 | Nothing a reader would see; the postmark's SVG type is a hair lighter than Chrome's |
| 6 | 4 | route map of the town | 9.5/10 | The map is the book's own SVG; identical at 200 dpi |
| 10 | 6 | day 1 story opener | 9/10 | One paragraph breaks differently: Chrome's `text-wrap: pretty` keeps "ASK SOMEONE." together; Matenadar's ragged breaking is greedy |
| 13 | 7 | day 1 plate | 9.5/10 | Identical (same JPEG, same crop, same frame); the Kalam caption may sit a fraction of a point off |
| 63 | 9 | sleeve 14 | 9/10 | Soap and key board are the book's drawing; head, wax-paper note, board strip, steps and answer row are Matenadar's and match |

The rebuilt document has 9 pages: Matenadar decides recto and verso by the physical page, so empty pages
(1, 3, 5, 8) keep every page on its side. Each page sets `page_from` so the folios read 6, 10, 13, 63.

Build: 9 pages, layout ≈ 45 ms, ≈ 0.5 s for the PDF, ≈ 0.85 s with PNGs at 100 dpi. `check`: no issues.

## How each page is made

- **Running foot.** Master foreground, one frame per side (`when: verso/recto`): a three-column table
  `auto | 1fr | auto` holds the folio (Besley 700, lining), a canvas with a 1 px rule at 100% width, and the
  run text (League Gothic, 0.14 em, caps) from `section.run`. The rule sits 3 pt above the baseline, as the
  CSS `translateY(-3pt)` put it.
- **Parcel label (p2).** One group turned −1.4°: an outline-only offset shadow (a 1.4 pt stroked rect at 4,4
  under the card, which is what `box-shadow: 4pt 4pt 0 -0.7pt #fff, 4pt 4pt 0 0.7pt #000` draws), the card,
  the black strip, three form rows (tables `auto | 1fr` with a canvas rule), Iona's four lines in Kalam,
  and the postmark: the book's SVG (from `parts.postmark`, rebuilt in `make.py`) inside a group turned 9°,
  its day number set as live Besley Black over it.
- **Map (p6).** `town.html` placed as an SVG (2,059 paths, text in League Gothic and Kalam with white
  halos). Its margin note was text inside the SVG; here it is a Matenadar text frame turned −90°, placed
  from the note's baseline origin.
- **Story opener (p10).** A `section` on the story master (0.8 / 1.2 in). The flow is: a canvas for the
  envelope face (native perforated stamp drawn with `path` and a scaled group, the day picture fitted
  `contain`, the postmark, the kicker and the two-line Besley title), a canvas for the sleeve line, the
  14 story paragraphs (Literata 12.4/17.8, ragged, hyphenated, 1.35 em indents, old-style figures, the
  small-caps lead-in as a span), and the wave · OPEN SLEEVE 1 · arrow hand-off.
- **Plate (p13).** The 2172 × 2896 JPEG `cover`-fitted inside a 1 px frame with a 1 px outline 3 pt out,
  Kalam caption centred.
- **Sleeve 14 (p63).** Head (black tab, italic description, difficulty icons, 2 px rule), the soap drawing,
  the wax-paper note rebuilt from `wrapper()` (pinked polygon, hard black copy, dashed folds, five Kalam
  lines, turned −1.3°), the board's sheet, shadow and title strip natively, the keys as the book's SVG,
  WHAT TO DO with Literata steps, and the answer row with its double postmark ring.

## What I learned about the original

- Chrome floors CSS border widths to whole CSS pixels: the book's 0.8, 1 and 1.4 pt borders all print at
  0.75 pt, and 2.2 pt rules at 1.5 pt. The rebuild uses the printed widths.
- `font-optical-sizing: auto` in Chrome sets `opsz` to the size in CSS px (12.4 pt body → opsz 16.53), not
  points. The rebuild sets `variations.opsz` to size × 4/3; with it the story text is glyph-for-glyph
  identical.
- The postmark's day number inherited the body's old-style figures inside the SVG. An SVG renderer does
  not apply `font-variant-numeric`, so the number is live type here.
- CSS letter-spacing also follows the last letter. Right-aligned tracked text and flex gaps in the book
  sit one tracking further left than Matenadar's; the foot and the label rows compensate.

## Files

- `make.py` — builds `doc.json`; writes `build/art/town.svg` (the map minus its margin note) and
  `build/art/d14-soap.svg`, `build/art/d14-board.svg` (the day-14 drawing cut around the parts set natively).
- `doc.json` — the document. Its SVG `src` paths point into `build/art/`, so run `make.py` first.
- `GAPS.md` — what the engine could not say, with the JSON I'd want to write.
