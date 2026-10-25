# Idioms of the World, Drawn Literally — five pages in Matenadar

The book (`~/projects/amazon-kdp/projects/idioms-of-the-world`) was set as one HTML file per page and printed by
Chrome (`scripts/build.py` + `scripts/book.css`). This sample rebuilds five of its pages from the book's own
sources: every word from `manuscript/idioms.json`, the plates from `visuals/art/print/`, the fonts from
`design/fonts/`, and the page numbers by replaying the book's pagination in `make.py` (front matter, then per
chapter an opener, one page per plate, an "also said" page, with the interlude before `nuts`). The index's page
numbers come out the same as the book's.

```
python3 samples/idioms-of-the-world/make.py              # doc.json (headlines unbalanced: see below)
BALANCE=1 python3 samples/idioms-of-the-world/make.py    # with text-wrap: balance, once the engine compiles it
MAGICK_CONFIGURE_PATH=$PWD/samples/idioms-of-the-world/build/magick \
  python3 scripts/review.py samples/idioms-of-the-world
```

`build/magick/type.xml` only gives ImageMagick a default font (Helvetica) for the labels on the compare sheets; on
this machine `magick -annotate` has no font configured and `review.py` stops without it.

## Pages

| original | rebuilt | page | fidelity | what still differs |
|---|---|---|---|---|
| 47 | 1 | plate page (care-ar): running head, plate on its ground, Arabic word-for-word strip, romanization, bottom-anchored note | 8/10 | The literal breaks "she-camel in / it nor a he-camel." where the book balances it to "she- / camel in it nor a he-camel." (`balance` does not compile). Everything else lands within 1 px at 100 dpi, the RTL strip included. |
| 76 | 2 | interlude chart: headings sized by count, superscript counts, says-lists, upright-in-italic notes | 9.5/10 | Sub-pixel only (0.9 % of pixels differ). |
| 111 | 3 | chapter opener (devil): full-bleed ground, 400 pt numeral as outlines off the page, Chrome-style faux-bold title, cast list at 90 % | 8.5/10 | Title breaks "Speak of the / devil." instead of "Speak of / the devil." (balance). The numeral (a `lettering` element, so KDP sees a shape) matches the book's outline to the pixel; the faux bold is `outline` at size/32, as Skia does it. |
| 42 | 4 | "also said" list (rain): ruled two-column rows, CJK and Thai originals, bottom-anchored tail | 9.5/10 | Sub-pixel only. |
| 148 | 6 | index of animals: two ruled sections, four balanced columns, hanging turnovers | 8.5/10 | Columns share out exactly as the book's (9/10/9/8). Three turnovers break one number later ("horse 27, 44, / 126" for "27, / 44, 126"): the book's `text-wrap: pretty` evens out the two lines. |

Rebuilt page 5 is the blank verso that the index's `start: verso` inserts. Build: 6 pages, layout about 40 ms,
about 0.65 s in all (`build/report.json`).

## How it is built

- One master, `page` (6 × 9 in, 0.125 in bleed on top, bottom and outside), whose foreground draws the folio
  (`{"field": "page"}`, mirrored with `when: recto|verso`) and the running foot; each page sets `page_from` to its
  number in the book. Openers use a second master whose folio is ink at 86 % (`#231f1cdb`) and has no foot.
- Everything else is placed per page in trim coordinates, the CSS turned into points (the text column is 0.56 in
  from the spine and 0.44 in from the outside edge). Text sits on Chrome's baselines because Matenadar's
  `line_height` follows the CSS model (half the leading above and below), so frames are placed at the CSS box tops.
- Fonts: `book/design/fonts` directly. The book's composite "Book Serif" is a font stack (Noto Serif, then Naskh
  Arabic, Hebrew, Armenian, Georgian, Devanagari, Bengali, Tamil, Thai, Ethiopic, SC); CJK runs name their Noto
  Serif SC/JP/KR face by language. Archivo Narrow comes from the three static cuts by weight.
- Opener numeral: `lettering` (Young Serif, 400 pt, `lnum`, −0.04 em), coloured 91 % of the ground, placed where
  the CSS line box puts it (baseline 4.8 pt under the media bottom), `decorative`.
- The word-for-word strip is a two-row table of `auto` columns (words, then glosses), reversed for right-to-left
  languages so it reads right to left and packs to the left; the 0.13 in gap is each cell's right inset, and the
  gloss's 0.012 in gap is extra leading on its line.
- Rules: the "also said" rows and chart groups draw their hairlines as `rule_above` on each cell's or group's first
  paragraph plus a zero-height canvas line under the last one, because per-side borders do not parse (GAPS 2).
  Chrome lays a 0.75 pt border below its edge where a Typst line is centred on it, so gaps carry the half-width.
- The index is one flowing frame: kicker, title, ruled section heads and `columns` with `balance: true`. Entries
  carry their 0.12 in gap as a bottom inset because balanced columns drop block spacing (GAPS 8).

## Small compensations worth knowing

- Chrome's letter-spacing also follows the last letter of a run; Typst's does not. Right-aligned tracked lines
  (running head, folio, foot) take a `margin_right` of one tracking, and the spaces after the index keys, "No. N"
  and "says" are widened by the same amount (GAPS 5).
- Superscript counts beside chart headings are `baseline` raises computed from the CSS (`vertical-align: top` plus
  0.04 in); they come out where Chrome put them.
