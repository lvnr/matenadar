# Gaps: A Field Guide to Software Bugs

Most valuable first. Each has what the book needed, what the sample does instead, and the JSON I would
like to write.

## Bugs (engine as of 2026-10-25 23:36)

### 1. A rotated box drops its `space_before` and `space_after`

```json
{"type": "p", "text": "two", "set": {"space_after": 2}},
{"type": "box", "space_before": 20, "rotate": -1.2, "blocks": [{"type": "p", "text": "rotated"}]},
{"type": "p", "text": "three"}
```

The box sits 2 pt under "two" instead of 20 pt, and "three" touches it. Without `rotate` both spaces
work. The editor's hand (p. 13) uses a `space` block before the box.

### 2. A mark's `inset` takes one length only

`{"text": "NEGATIVE", "mark": {"kind": "box", "inset": {"x": 5, "y": 2}}}` fails to parse
("data did not match any variant of untagged enum Inline"); `"inset": 2` works. A status pill needs
5 pt at the sides and 2 pt above and below (CSS padding `2.2pt 5pt 1.8pt`). The pill is an
auto-width text frame instead, which works but sits outside the text, so it can't follow a line.

### 3. Space at the start of a frame or a table cell is dropped

The opener's first block has `space_before: 1.25in` (CSS `margin-top`); the frame ignores it, so the
frame's own `y` carries the drop. Inside table cells, `space_before`, `space_after` and `space` blocks
are all dropped, so cell padding can only come from the table's `inset`.

## Features

### 4. Text on a path

The Infinite Loop's caption runs round the plate eight times and closes seamlessly. The sample places
the book's SVG (`textPath`, with `textLength` so the repeats close) as markup, so the type is not
Matenadar's own (no styles, no checks, no fallback). Wanted:

```json
{"type": "text-path", "x": 0, "y": "0.24in", "w": "4.8in", "h": "3.9in", "viewbox": [0, 0, 480, 390],
 "d": "M 240 185 m -147 0 a 147 147 0 1 1 294 0 a 147 147 0 1 1 -294 0",
 "style": "ring", "text": "follows the one in front, which ", "repeat": 8, "fit": "spacing"}
```

### 5. A page (or block) placed inside itself

The Stack Overflow's plate holds the page, four levels deep, scaled by 0.3511 each time. The sample
rebuilds every level from Python functions (nested groups with `scale`). A way to place a laid-out
page, with recursion stopped at a depth, would make it one element:

```json
{"type": "page-view", "page": "stack-overflow", "x": "1.3467in", "y": "0.34in", "scale": 0.3511,
 "depth": 4, "outline": {"width": 1.6, "color": "rule"}, "region": "trim"}
```

### 6. Page numbers on hand-laid pages

A `page` can't set or restart its number, so in a sample (or any book that interleaves fixed pages
with blanks) `{"field": "page"}` and `{"ref": "…"}` give the position in the file, not the book's
number. The folios are literal vars here, and the opener's table takes numbers from the book's page
map. Wanted on pages as on sections:

```json
{"type": "page", "id": "integer-overflow", "master": "page", "page_from": 13, "folio": "-2147483648", …}
```

(`folio` overriding the printed number while the count goes on, as p. 13's joke needs.)

### 7. Rotation about a chosen point

The editor's hand turns about its right end on a recto and its left end on a verso (CSS
`transform-origin`). Elements and boxes turn about their centre, so the sample shifts the box by
half its width × sin 1.2°. Wanted: `"rotate": -1.2, "origin": "right"` (any anchor name, or `[x, y]`).

### 8. Chrome compatibility switches

Three Chrome behaviours decide where lines and rules fall in this book, and the sample imitates each by
hand:

- stroke widths snap to whole CSS pixels, never under one (0.35 pt and 0.5 pt rules print 0.75 pt;
  inside a 0.35× zoom every frame is still a device pixel);
- optional ligatures are off in letter-spaced text (otherwise a sighting re-breaks);
- tracking after the last letter stays inside the line when it is centred or right-aligned, and
  inside a box's width.

```json
"design": {"compat": {"snap_strokes": "css-px", "ligatures_with_tracking": false, "trailing_tracking": true}}
```

### 9. `line-height: normal`

CSS `normal` is the font's own ascender + descender + line gap (Caslon 1.2, Big Caslon 1.209,
JetBrains Mono 1.32). The sample writes the ratios out per font; `"line_height": "normal"` would
take them from the font.

## Worked as hoped

Per-glyph substitution through a font stack (the editor's capital L from a one-letter font), Caveat
at wght 500 straight from the variable font, `all-small-caps` with tracking, `onum`, page-sized
grounds into the bleed, scaled nested groups, components with props, auto-width text frames, the
overflow check on a frame with `id` and `h`, and paths with a viewbox for the blot.
