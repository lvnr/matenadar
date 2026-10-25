# A Field Guide to Software Bugs: five pages in Matenadar

The book was set as one fixed page per HTML `<section>` (no text flows across pages): `scripts/build.py`
assembles the pages, `scripts/species_pages.py` builds species pages and the pages that suffer from their
own bug, and Chrome prints them, failing the build if any entry overflows its box. This sample rebuilds
five of its 98 pages from the book's own sources: the species texts (`book/text/*.txt`, parsed as
`scripts/content.py` does), `book/orders.json`, `book/meta.json`, the finished build's page map
(`build/book-report.json`, for the opener's page numbers), the composited plates and grounds
(`build/plates/`, `build/pagebg/`) and the handwriting fonts (`assets/fonts/`). Adobe Caslon Pro, Big
Caslon and JetBrains Mono are the Mac's installed fonts, as they were for the book.

```
python3 make.py && python3 ../../scripts/review.py .
```

## Pages

All five are hand-laid `page`s, as the book's were. The running head and foot are components
(`head-recto/verso`, `foot-recto/verso`) used on the pages and again inside the recursive plate.

| original | rebuilt | page | how | score |
|---|---|---|---|---|
| 11 | 1 | Order 0 opener | one text frame of centred blocks: mono order number, Big Caslon title, italic gloss, rule, measure-limited description, a four-column table with page numbers | 9.5 / 10 |
| 13 | 3 | The Integer Overflow | plate image with edge and neat lines and figure labels, caption, a species column frame (name, Latin name and status pill over a rule, small-caps labelled entry, sighting with rule above, the editor's hand rotated −1.2°), mono folio | 9.5 / 10 |
| 25 | 5 | The Buffer Overflow | the whole-page ground JPEG under everything, the plate's frame drawn over it, left caption, a column narrowed by 1.1 in, the Memory Leak ink blot over everything | 9.5 / 10 |
| 26 | 6 | The Stack Overflow | the page inside its own plate four levels deep: nested groups scaled by 0.3511 each, each holding a page rect, the head and foot components, a blank plate with the next level, caption and species column; a larger blot | 9 / 10 |
| 37 | 7 | The Infinite Loop | the round plate PNG, then the ring as SVG markup (circle and `textPath`) | 9 / 10 |

Rebuilt pages 2 and 4 are blanks that keep every page on its side. Build: 7 pages, layout about 40 ms,
about 0.5 s for the PDF; `matenadar check` reports no issues, and the four species columns are frames
with an `id` and `h`, so an entry that outgrows its box is reported as `overflow` (the book's fit check).

Measured against the original with PyMuPDF (text baselines matched line by line): every line of the
main text on all five pages lands within 1 pt of the original, x within 1 pt (captions 0.9 pt right).
Line breaks match everywhere. The blot is the book's own generator rerun (Python's `random.Random`
with the same seeds), so it is the same shape to the tenth of a unit.

## What differs, and why

- **The ring (p. 37) is SVG markup**, not Matenadar text: there is no text on a path. The book tuned
  size and letter-spacing by hand in Chrome so the eight repeats close at nine o'clock; here
  `textLength` on the `textPath` makes them close exactly. Visually the same, join included.
- **The recursive plate (p. 26)** is built by make.py calling the same element functions at each level
  (a page can't be placed inside itself as a component). The smallest levels differ by a fraction of a
  point.
- **Page numbers are literal.** Fixed pages can't restart the page count, and the rebuilt document has
  blanks between them, so folios come from the text (the species' own `folio` for p. 13) and the
  opener's table numbers from the book's page map rather than from `{"ref": …}`.
- **The editor's hand** turns about its right end in the book; a box turns about its centre, so it is
  lowered by half its width × sin 1.2° to put that end back.

## Chrome habits copied on purpose

- **Border widths snap to whole CSS pixels**, never under one: the plate's 0.35 pt hairline, the
  0.5 pt neat line and rules all printed 0.75 pt (measured at 1200 dpi). Inside the zoomed recursive
  plate this makes every frame one device pixel or more, which is why the small pages look heavy-lined
  in the book; `bw(width, zoom)` reproduces it at every level.
- **Optional ligatures are off in letter-spaced text** (the sighting's −0.003 em): without
  `ligatures: false` the "Th" ligature shortened a line and "forty" moved up a line.
- **Trailing letter-spacing** stays inside centred and boxed text: the opener's "ORDER 0" and title
  carry a right margin of one letter's tracking, the status pill a wider right inset, and the label
  gap includes the tracking after "marks.".
- `line-height: normal` is written as the font's own ratio (Caslon 1.2, Big Caslon 1.209, JetBrains
  Mono 1.32).

## What worked without fuss

The page-sized ground at 300 ppi into the bleed; plates as images with frames over them; the
per-glyph font substitution for the editor's capital L (a font stack `["Field Hand L", "Caveat"]`, exactly
the book's `font-family: "HandL", "Hand"`); Caveat at wght 500 from the variable font; `all-small-caps`
labels with tracking; oldstyle figures by feature; nested scaled groups for the page-in-a-page; components
with props; auto-width text frames for the status pill (anchored top-right); the overflow check.
