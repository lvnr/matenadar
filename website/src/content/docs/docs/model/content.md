---
title: Content
description: Sections that flow through masters, pages laid out by hand, two-page spreads, on_page, placed elements, blocks and inline runs.
---

`content` is the book, in order. Each item is one of three kinds:

- a **section**: a flow of blocks poured through the pages of a master,
- a **page**: laid out by hand from placed elements,
- a **spread**: two facing pages under one picture.

## Sections

A section is a flow poured through pages of a master:

```json
{"type": "section", "id": "ch1", "master": "body", "start": "recto",
 "numbering": "1", "page_from": 1, "title": "The Machine", "vars": {"chapter": "One"},
 "style": "body", "flow": [
   {"type": "h", "id": "h1", "level": 1, "text": "The Machine"},
   {"type": "p", "id": "p1", "text": "…"}
 ]}
```

| Field | Meaning |
|---|---|
| `master` | the master the section’s pages are cut from |
| `start` | `next` (default), `recto`, `verso`, or `continue` (on the same page as the section before, if it uses the same master). A page left empty so a section can start on the right side is a **blank** page. A document whose first item starts on a `verso` opens with a blank page. |
| `numbering` | `1`, `i`, `I`, `a`, `A`. It carries over to later items. |
| `page_from` | restarts the page count |
| `title`, `vars` | feed fields in the master’s layers: `{"field": "section.title"}`, `{"field": "section.chapter"}` |
| `style` | the default paragraph style of the flow |
| `flow` | the [blocks](#blocks) |

## Pages

A page is laid out by hand:

```json
{"type": "page", "id": "p48", "master": "body", "layers": false, "fill": "#e8e2d0",
 "title": "…", "vars": {}, "elements": [ ]}
```

The master gives the page its size and bleed. `layers: true` also draws the master’s background and foreground (a folio on a puzzle page). Pages take `start` too.

### Repeated pages

`repeat` lays the page out again and again: dated journal pages, a page per prompt. Each time, the page’s vars get `n` (1, 2, 3…), `date` (with `dates`) and that time’s own `vars`, for fields such as `{"field": "section.date", "format": "[weekday], [day padding:none] [month repr:long]"}`.

```json
{"type": "page", "id": "day", "repeat": {"count": 7, "dates": {"from": "2026-11-02", "step": 1},
 "vars": [{"prompt": "…"}]}, "elements": [ ]}
```

`count` defaults to the number of `vars` (or 1). The page’s id, and a reference to it, point at its first time.

## Spreads

A spread is two facing pages under one picture: a verso and the recto after it. It always opens on a verso (a blank page is left before it if needed).

```json
{"type": "spread", "id": "s-dig", "master": "page", "src": "art/dig-site.jpg",
 "focus": [0.5, 0.6], "gutter": "0.125in", "alt": "…",
 "left": {"elements": [ ]}, "right": {"elements": [ ]}}
```

- The picture covers both trims and their outer bleeds (top, bottom and each page’s outside edge; there is no bleed at the spine), `cover`-fitted, with `crop` and `focus` as for [images](#image).
- `gutter`: how much of each page the binding hides at the spine. That strip is repeated on both sides of the fold, so the picture reads across it without losing what is there. Default 0, for lay-flat bindings; perfect-bound paperbacks want about 0.125 in.
- `left` and `right` are the pages: `elements` placed over the picture from that page’s trim corner (words over art; see [text frames](#text)), and their own `crop` and `focus`, which move only that page’s half. A side can have its own `src`: a spread drawn as two halves, each covering its page and its outer bleed.
- `filter`, `layers`, `fill`, `title`, `vars`, `page_from` and `numbering` work as for pages.

## On the page of a node

`on_page`, at the document’s top level, changes the page where a node lands, keyed by the node’s id, so the change follows the node when the text reflows:

```json
"on_page": {"f-plate-3": {"hide": ["folio", "running"]}}
```

- `hide`: master elements not drawn on that page. Each entry is an element’s `id`, the style of a text element (`"folio"`, `"running"`), or `"all"`. Use it for a page with a full-bleed picture, or a part title.
- A key whose node is gone does nothing.

## Elements

Placed elements make up pages, spread sides, master layers, components, canvases and groups. Every element takes:

| Field | Meaning |
|---|---|
| `id` | optional; must start with a letter and use only letters, digits, `-`, `_`, `.`, `:` |
| `x`, `y` | position |
| `w`, `h` | size |
| `anchor` | which point of the element `x`/`y` place: `top-left` (default), `top`, `top-right`, `left`, `center`, `right`, `bottom-left`, `bottom`, `bottom-right`. Text frames also take `baseline-left`, `baseline` and `baseline-right`, which place the first line’s baseline (as TikZ does). |
| `rotate` | degrees clockwise, about the centre or `origin` |
| `origin` | the point `rotate` turns about, which stays put: an anchor word (`top-left`…) or `[x, y]` from the element’s corner |
| `opacity` | 0–1, for everything in the element: text, fills, strokes, drawings and photographs. Printers prefer `tint` on drawings. |
| `stroke_align` | where a stroke lies on the edge: `center` (default), `inside` (as CSS borders and most design tools draw it), `outside` |
| `skew` | degrees of horizontal slant |
| `when` | conditions, all of which must hold: `recto`, `verso`, `first`, `blank`, and their `not-` forms |
| `decorative` | art, not reading text. Its text is drawn as glyph outlines in the PDF, so display type may run into the margins and bleed without KDP’s previewer (or any reader) seeing text there, and margin checks skip it. The layout report still lists it. |
| `shadow` | a hard shadow: `{"dx": 3, "dy": 3, "color": "#0004"}` (needs `w` and `h`, except on text frames) |

### `text`

A frame of text, `w` wide, or without `w` as wide as its longest line (a pill, a tag).

- **Content:** `text` (a blank line starts a paragraph, a newline breaks the line), `content` ([runs](#inline-content-runs)), or `blocks`.
- **Options:** `style`, `set` (overrides), `valign` (`top`, `middle`, `bottom`; needs `h`), `inset`, `fill`, `stroke`, `radius`, `columns`, `gutter`, `clip`.
- **Fitting:** `fit: "shrink"` scales the type down until the text fits `h`, no smaller than `min_size`. `fit: "width"` scales the type up or down until the longest line fills `w`, between `min_size` and `max_size`.
- **Overflow:** a frame with an `h` and an `id` reports `overflow` when its text doesn’t fit.

**Words over art.** A frame set over a picture can have:

- `scrim`: a soft panel behind it, so its words read on any part of the art: `{"color": "#fff", "opacity": 0.8, "feather": "10pt", "inset": "8pt", "radius": "6pt", "shape": "rect" | "ellipse"}`. `feather` fades the edge out; `inset` is how far the panel reaches past the frame.
- `safe`: `true` keeps the frame inside the master’s margins, or a length keeps it that far in from the trim on every side. The frame is moved, never resized, when it would cross (at layout, so a frame whose height depends on its words is kept in too). It applies to frames placed on a page itself, and needs a `w`.
- a halo per glyph: `"set": {"halo": {"width": 2, "color": "#fff"}}`.

```json
{"type": "text", "id": "words", "x": "0.6in", "y": "0.7in", "w": "3.2in", "safe": true,
 "text": "Every evening, Ani lit the lamps along the hill road.",
 "scrim": {"color": "#fffaf2", "opacity": 0.85, "feather": "10pt", "inset": "8pt", "radius": "6pt"}}
```

### `image`

- `src` is a raster image, an SVG or a PDF (its first page, drawn as vectors: a puzzle or a page of art from another tool).
- `fit` is `cover` (default; fills the box and crops), `contain` or `stretch`.
- `focus: [fx, fy]` chooses which part of a cropped (`cover`) image stays in view, as fractions: `[0.85, 0.5]` keeps the right side. Needs `w` and `h` in absolute units.
- `crop: {"x", "y", "w", "h"}` shows only that part of the picture (fractions of its width and height from its top-left corner; `x` and `y` default to 0, `w` and `h` to 1), which `fit` and `focus` then treat as the whole.
- Also `radius`, `stroke`, `flip` (`x`, `y`, `both`) and `clip`.
- `alt`: what the picture shows, for readers who can’t see it (ebooks).
- `filter` changes a raster picture’s tones before it is placed (an SVG keeps its own): `{"gray": true, "contrast": 3, "black": 0.02, "white": 0.98, "gamma": 1.1}`.
  - `gray`: shades of grey from Rec. 709 luma, for black-ink interiors.
  - `black`, `white`: input levels (the tones that become black and white).
  - `gamma`: midtones; above 1 lightens.
  - `contrast`: an S-curve about the middle grey (0 none, 3 moderate, 8 strong) that never clips the ends.

  Photographs stay JPEG; other pictures become PNG. The result is cached, so editing around a filtered picture costs nothing more.

### Shapes and drawings

- **`rect`**, **`ellipse`**: `w`, `h`, `fill`, `stroke`, `radius` (rect only).
- **`line`**: `x1`, `y1`, `x2`, `y2`, `stroke`.
- **`path`**: `d` (SVG path data) in a `w` × `h` box, whose coordinates are points unless `viewbox` (`[x, y, w, h]`) is given. Also `fill` and `stroke`.
- **`svg`**: a drawing from `src`, or inline `markup`; `fit`; `viewbox` (to show part of the drawing); `tint: {"color", "amount"}` mixes every colour in it towards one (a print-safe fade, or a duotone).
- **`group`**: `children` placed from the group’s corner; `scale` (1 = actual size), `clip`, `fill`, `stroke`, `radius`.
- **`use`**: an instance of a [component](/docs/model/design/#components): `component`, `props`.
- **`qr`**: `data`, `w` (the code’s size), `color`, `background`, `quiet` (modules), `ecc` (`L`, `M`, `Q`, `H`). Drawn as one vector shape.

**Clipping.** Text frames, images and groups take `clip`: `true` cuts what they hold to their box (a group needs `w` and `h`), `"ellipse"` to the ellipse in that box, and SVG path data (`"M 0 72 L 36 0 L 72 72 Z"`, in points from the element’s corner) to any outline. For round portraits, type in a circle, torn and die-cut edges.

### `lettering`

Display text drawn as vector outlines, not live text. Use it for type that runs into the bleed or must never reflow.

- Settings: `text`, any character setting (`font`, `size`, `weight`, `stretch`, `variations`, `tracking`, `features`, `color`, `outline`, `case`), `span`, `w` with `align`, `line_height`.
- `x`/`y` place the top of the font’s ascender, as CSS does.
- **Text on a path:** `path` (SVG path data, in points from `x`/`y`) sets the letters standing on the path, turned to it; `start` (a length or a share, `"50%"`) and `align` (`left`, `center`, `right`) say where along it the text starts, centres or ends. For seals, postmarks and badges.

```json
{"type": "lettering", "id": "seal", "text": "THE LAMPLIGHTER", "font": "Libertinus Serif", "size": 14,
 "tracking": "0.2em", "x": "1in", "y": "1in", "path": "M 0 120 A 120 120 0 0 1 240 120",
 "start": "50%", "align": "center", "decorative": true}
```

### `letter-grid`

Word searches, crosswords, Sudoku and letter puzzles.

- `rows`: one string per row, or rows of cells, each `"A"` or `{"text", "fill", "color", "weight", "span", "number", "block", "given", "circle"}`.
- The puzzle and its solution are the same data: `blank: true` leaves the letters out except in cells marked `given`, keeping numbers, blocks and circles.
- `regions` draws heavier lines where one region meets another: boxes `[columns, rows]` (`[3, 3]` for Sudoku), or one string per row naming each cell’s region (`["aabb", "acbb", …]`, any shapes). `region_lines` is their stroke (default the frame’s, or 1.5 pt); without a `frame` they also go round the grid.
- Layout: `cell` (size), `cell_h`. Type: `style` (a paragraph or character style), `set`. Drawing: `lines` (between cells), `frame` (outer border), `fill`, `radius`.
- `highlights`: `{"from": [row, col], "to": [row, col], "fill", "stroke", "thickness"}`. These are capsules over found words, drawn under the letters; `"shape": "cells"` fills each cell instead.

### Book kinds’ elements

Each is drawn from its data, as vector, so an edit to the data redraws it; a solution page is the same element with `solved: true`. Lines are SVG; words are live text in the element’s `style`.

- **`maze`**: `rows`, one string per row of cells, a hexadecimal digit per cell giving its walls (1 top, 2 right, 4 bottom, 8 left, added up: `f` is closed); or `segments` (`[x1, y1, x2, y2]` in cells) with `size` (`[columns, rows]`) for mazes that aren’t a grid. `cell` (default the width over the columns), `start` and `end` (a cell `[row, column]`, or a point in cells: a dot and a ring), `solution` (cells or points in order), `solved`, `walls`, `path` (strokes), `marks` (the start’s and end’s colour). Walls along one line are drawn as one stroke.
- **`logic-grid`**: `categories` (`[{"name", "items": [...]}]`, two or more, all the same length), set in the usual staircase: the first category down the side with the others across the top, the last ones down the side under it. `marks`: `{"a": [category, item], "b": [category, item], "mark": "o"}` (`"o"` a dot, `"x"` a cross, or up to three letters). `cell` (default 14 pt), `style`, `lines`, `frame`.
- **`dots`** (dot-to-dot): `points` in the `viewbox`’s units (default points), numbered from `start` (default 1) beside each dot, outside the turn the line makes there (`labels` gives `[dx, dy]` per point instead). `breaks` (point indices) start new strokes; a stroke that ends on its first point is closed, and that point isn’t numbered again. `solved` draws the line (`line`); `dot` and `size` (the dot’s and the number’s size), `color`, `style`.
- **`ruling`**: journal and planner rulings over `w` × `h`: `pattern` (`lines`, `dots`, `grid`, `graph`, `isodots`), `pitch` (default 0.3 in for lines, 0.2 in otherwise), `stroke`; `graph` makes every `major`-th line (default 5) `major_stroke`; `lines` take `first` (the first line’s depth) and `margin` (a vertical rule that far in, `margin_stroke`). Grids and dots are centred, so what is left over is even.
- **`calendar`**: a month (`year`, `month`) from its first weekday, a box a day: `week_start` (`monday`, default, or `sunday`), `days` (seven names), `style` (day numbers), `header_style` and `header` (the weekday row’s height), `lines`, `outside: true` (the next and last months’ days, in `outside_color`).
- **`craft`** (paper-craft sheets): `faces` (polygons, `points`, with `fill`, or a picture `src` with `crop`, `focus` and `filter`, cut to the face; `label`), `edges` (`from`, `to`, `kind`: `cut` (default), `mountain`, `valley` or `score`; `tab: {"depth", "angle", "side", "label"}` adds a glue tab and makes the edge a fold), `marks` (`at`, `kind`: a registration target or `dot`, `label`), `viewbox`, the line kinds’ strokes (`cut` solid, `mountain` dash-dot, `valley` dashed, `score` dotted), `glue` (the tabs’ colour), `scissors: true` (on the first cut line).
- **`panels`** (comic pages): `rows` of `panels` (`{"h": "1fr", "panels": [...]}`; heights and widths are lengths or shares, `"2fr"`), `gutter`, `border`, `radius`, `fill`. A panel has `id`, `w`, a picture (`src`, `crop`, `focus`, `filter`, `alt`) cut to it, `fill`, `border`, and `elements` placed from its corner (balloons, captions) and free to break its border.
- **`balloon`**: `text` or `content` in `style` (default the design’s `balloon` style) with `set`, wrapped at `w` in lines of even length. `shape`: `speech` (default) and `whisper` (dashed) ellipses, a `thought` cloud, a `shout` burst, a `caption` box. `tail` is the point the tail runs to (in the same coordinates as `x`, `y`); the balloon and its tail are one outline. `fill`, `stroke`, `inset`.

## Blocks

Blocks make up flows, text frames, boxes and block components.

| Type | Fields |
|---|---|
| `p` | `id`, `style`, `text` or `content` (runs), `set` |
| `h` | `level`, `style` (defaults to `h1`, `h2`… when the design has them), `text`/`content`, `outline` (listed in the contents), `short` (title for running heads and contents) |
| `break` | a scene break: `style`, `text` (the ornament) |
| `figure` | `src`, `w`, `h`, `fit`, `crop` and `focus` (as for images), `align`, `caption`/`caption_content`, `caption_style`, `float` (`top`, `bottom`), `stroke`, `radius`, `alt`, `filter`, `numbered`, `short` (the caption the list of figures gives); `plate` (see below) |
| `box` | `blocks`, `style`, `fill`, `stroke`, `stroke_align`, `radius`, `inset`, `w`, `h`, `align` (places the box; its text keeps its own alignment), `breakable`, `rotate`, `space_before`, `space_after`, `margin_left`/`margin_right` (negative hangs the box into the margins) |
| `table` | `columns` (lengths, `"auto"`, `"1fr"`), `header`, `rows` (cells: a string or `{"text"/"content"/"blocks", "colspan", "rowspan", "fill", "align", "style", "set"}`), `style`, `header_style`, `header_fill`, `zebra`, `stroke`, `rules` (book-style horizontal rules in place of the grid: `{"top", "header", "rows", "bottom", "columns"}`, each a stroke), `inset`, `align` (per column), `valign`, `row_gap`, `repeat_header` (default true) |
| `list` | `items` (strings or lists of blocks), `ordered`, `marker` (or a numbering pattern such as `"1."`), `style`, `indent`, `gap`, `spacing`, `start`, `marker_style` (a span for the markers), `marker_shape` (a disc or tile behind each marker: `{"shape": "ellipse" or "rect", "w", "h", "fill", "stroke", "radius"}`), `marker_align` (`left`, or `right` for numbers, the default) |
| `code` | `text` (spaces kept), `style`, `fill`, `stroke`, `inset`, `radius`, `breakable` |
| `canvas` | an inline drawing: `w` (default the column’s width), `h` (a length, or `"1fr"` for what is left), `elements`, `align`, `fill`, `stroke`, `radius`, `clip`. Percentages inside are of the canvas. |
| `use` | a component instance (made of blocks, or of elements as a canvas) |
| `page-break` | `id`; `to`: `recto` or `verso`; `weak: true` (no break where the page has nothing on it yet) |
| `space` | `h` (a length, or `"1fr"` to push what follows to the foot), `weak` |
| `columns` | `count`, `gutter`, `blocks`, `rule` (a stroke between columns), `balance: true` (blocks shared out so the columns end level; blocks stay whole, as for indexes) |
| `column-break` | continue in the next column |
| `margin` | a note in the master’s note column, level with the top edge of the block before it (`at: "here"` for where it stands); `dy`; `text`/`content`/`blocks`. Several notes after one block stack under each other; notes never overlap. |
| `toc` | the contents: `depth`, `style` (default `toc-1`, `toc-2`…), `leader` (such as `"."`) |
| `lof` | the list of figures: every numbered figure’s number, caption and page; `style` (default `lof`, then `toc-1`), `leader` |
| `keep` | `blocks` that stay on one page |

**Plates.** A figure’s `plate` is `half` (half the text area, at the foot of a page unless `float: top`), `full` (a page of its own within the margins: the whole picture as large as it fits, its caption under it) or `bleed` (a page of its own to the bleed, the master’s elements hidden there, the caption on a panel at the foot of the text area). Plates float: the text runs on past them.

## Inline content (runs)

`content` is a list. Each item is one of:

- **Text.** A plain string, or a run: `{"text": "…", "em", "strong", "span", "link", "sup", "sub", "nobreak", "mark", …any character setting}`.
- **Marks.** `mark` is a pen mark that follows where the run lands: `{"kind": "underline" | "strike" | "highlight" | "box" | "ring", "stroke", "fill", "offset", "extent", "radius", "inset"}`.
- **Footnote.** `{"note": [ … ]}`.
- **Leader.** `{"leader": "."}` fills the rest of the line with dots, for contents and index lines; `{"leader": ""}` fills it with space, pushing what follows to the right edge. Also `gap`, `span`, and `fixed: true` (dots exactly `gap` apart, as TeX sets them). `{"leader": "rule", "stroke", "lift", "gap"}` draws a line across the space: running feet, form lines.
- **Space.** `{"space": "0.13in"}` is horizontal space inside a line.
- **Field.** `{"field": "page" | "pages" | "heading" | "section.title" | "section.<var>" | "doc.title" | "doc.author"}`, filled in at layout. A value that is a date (`YYYY-MM-DD`) shows as `format` says, in Typst’s date format: `"[weekday], [month repr:long] [day padding:none]"`.
- **Page reference.** `{"ref": "node-id"}` gives the page number; `{"ref": "node-id", "show": "title"}` gives the title. For a numbered figure, `"show": "number"` gives its number (`3.2`) and `"show": "label"` its label (`Figure 3.2`).
- **Line break.** `{"br": true}`.
- **Inline image.** `{"image": "path", "h": "1em", "baseline": "0.2em"}`, or inline SVG: `{"image": "", "markup": "<svg …>", "h": "1em"}`.

```json
{"type": "p", "id": "p-12", "content": [
  "The scribe signed the ", {"text": "colophon", "em": true},
  {"note": [{"type": "p", "text": "From the Greek for a finishing stroke."}]},
  "; the list of scribes is on page ", {"ref": "scribes"}, "."
]}
```
