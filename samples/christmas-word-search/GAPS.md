# Engine gaps found rebuilding Christmas Word Search

Most valuable first.

## 1. Place text by its baseline, or by its first line's ink

Every text node in a TikZ (or InDesign-by-coordinates) book is anchored by a baseline: `anchor=base west`, `base
east` or `base`. Paragraph nodes with `text width` are anchored by the top of the first line's *ink*, TeX's box
height. Matenadar frames are placed by the top of a CSS line box. So make.py has to read each font's ascender and
descender (to find where the engine will put the baseline), and shape the first line with HarfBuzz (for its ink
top). Wanted:

```json
{"type": "text", "x": "0.925in", "y": "9.58in", "baseline": true, "style": "folio", "content": [{"field": "page"}]}
{"type": "text", "x": "4.5in", "y": "4.36in", "w": "5.4in", "top": "ink", "style": "chapter-title", "text": "Home for the Holidays"}
```

`anchor` could also take `base-left`, `base`, `base-right`.

## 2. Ragged paragraphs that break like TeX

The engine's ragged text is first-fit with fixed spaces. TeX's differs in three ways that changed real lines here:

- TikZ `text width` nodes have `\rightskip 0pt plus 2em` (and `\leftskip` when centred), so TeX chooses evened-out
  breaks and stretches the spaces a little on each line. The chapter intro and the worked example break differently
  without this.
- `\raggedright` lets spaces *shrink* to fit a line. "…on the blanks." fits in TeX and wraps in the engine.
- microtype protrudes end punctuation (TeX), while Typst's hanging punctuation is fixed at 80% for `.` and `,`.
  The two disagree on centred and right-aligned lines, and the engine's version can't be turned off.

make.py runs Knuth–Plass itself and hands the engine explicit breaks and per-line `word_spacing`. Wanted, as style
settings:

```json
"intro": {"font": "Chaparral Pro", "italic": true, "size": 14.2, "align": "center",
          "linebreaks": "optimized", "ragged_stretch": "2em", "space": {"stretch": "0.2em", "shrink": "0.1em"},
          "protrusion": "microtype"}
```

The minimum useful piece is `"linebreaks": "optimized"` (Typst has it) plus `"overhang": false`.

## 3. A TeX point unit

LaTeX sizes are in TeX points (1/72.27 in). A sample copied as `"13.2pt"` is 0.37% too big, so lines drift
0.07 pt each. make.py multiplies every size, leading and skip by 72/72.27. A unit would make it literal: `"size":
"13.2tpt"`.

## 4. Leaders with a fixed pitch and padding

`{"leader": "."}` spreads its dots to fill the gap (Typst `repeat(justify: true)`). TikZ's leader is a dash
pattern: round 1.15 pt dots every 4 pt, starting 0.08 in after the title, stopping where the next dot would come
within 0.08 in of the number. So the dots sit on a fixed pitch. Also missing: a round dot that sits *on* the
baseline. make.py fakes it with a Chaparral period set at 10.3 pt (so its dot is 1.15 pt) and lowered 0.5 pt. Wanted:

```json
{"leader": "dots", "size": "1.15pt", "pitch": "4pt", "pad": "0.08in", "justify": false}
```

## 5. Drawn markers on list items or headings

The How to Solve steps are a numbered list whose markers are black discs with white numbers, hanging 0.62 in left
of each step. Because a list `marker` is text only, the five discs are placed absolutely, at positions make.py
works out from TeX's vertical rules. Wanted:

```json
{"type": "list", "ordered": true, "indent": "0.62in", "marker": {"component": "badge", "align": "first-baseline"}, "items": [...]}
```

A run `mark` of kind `disc` that does not take part in layout would also cover it.

## 6. Inline drawings without files

The arrow table puts TikZ arrows inside a line of text. Inline images need a file path, so the arrows are placed
absolutely instead. Wanted: inline `{"svg": "<svg …/>", "w": "0.2in", "baseline": "-0.035in"}`, or an inline
component instance.

## 7. Tracking widens interword spaces

`tracking` adds the letter-spacing to spaces too, while XeTeX's `LetterSpace` (and InDesign's tracking on word
spaces, optionally) does not. The tracked caps labels came out 0.85 pt wider per space, so the `caps` style
carries `word_spacing: -0.09em`. A `"tracking_spaces": false` setting would make that explicit.

## 8. Letter-grid frame outset

`frame` strokes the cell edge. The book's rounded frame sits 0.045 in outside the cells and is drawn after the
letters, so it is a separate `rect`. Wanted: `"frame": {"width": 0.9, "outset": "0.045in"}`.

## 9. `start` is ignored on a document's first item

`start: "verso"` on the first page or section does nothing, so a document cannot open on a left-hand page. Later
items respect it, by sheet parity. The sample uses an explicit blank page instead:

```json
{"matenadar": "1", "meta": {}, "design": {"masters": {"m": {"width": "4in", "height": "3in"}}},
 "content": [{"type": "page", "id": "a", "master": "m", "start": "verso", "elements": []},
             {"type": "page", "id": "b", "master": "m", "elements": []}]}
```

Expected: a blank sheet 1, then `a` on sheet 2. It gives `a` on sheet 1.

## Worked as written

These needed no workaround:

- `letter-grid` with per-cell `weight` and `color`, and highlight capsules with `fill`, `stroke` and `thickness`.
  The capsule geometry and draw order match the book's TikZ exactly, so the solution grids overlay pixel for pixel.
- Masters with recto/verso `when` layers, `{"field": "page"}` with `page_from`, `{"field": "section.title"}` from
  a page's `title`, and live `ref`s.
- External SVG art placed stretched or contained, SVG markup, `path` with many sub-paths (all 78 write-in blanks),
  `ellipse` discs, rounded `rect`s, and OpenType features (`lnum`, the Chaparral "Th" ligature).
