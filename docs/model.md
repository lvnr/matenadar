# The Matenadar document model (version 1)

A document is JSON. It is the only source of truth: the engine generates its
layout program from it on every build and never stores that program.

```json
{
  "matenadar": "1",
  "meta": {"title": "…", "author": "…", "lang": "en-US"},
  "fonts": ["design/fonts"],
  "design": {"colors": {}, "fonts": {}, "base": {}, "styles": {}, "spans": {}, "masters": {}, "components": {}},
  "content": [ {"type": "section", …}, {"type": "page", …} ]
}
```

- `fonts`: font folders, relative to the asset root, searched before the
  system's fonts.
- **Asset root:** image, SVG and font paths are relative to it. On the command
  line it defaults to the document's folder; `--root` sets it. Nothing outside
  it can be read.

## Units, colours, coordinates

- **Lengths.** A number is points. A string can be `"12pt"`, `"0.5in"`,
  `"3mm"`, `"1.2cm"`, `"16px"` (0.75 pt), `"1.5em"` (relative to the text
  size) or `"50%"` (of the containing box).
- **Colours.** `"#rgb"`, `"#rrggbb"`, `"#rrggbbaa"` (with alpha), a name from
  `design.colors`, `"cmyk(0%, 20%, 40%, 90%)"`, `"luma(40%)"`, or `"none"`.
- **Coordinates.** Placed elements are measured from the top-left corner of the
  page's **trim**: x to the right, y down. The bleed lies at negative
  coordinates and beyond the trim's width and height. A master's `bleed` is on
  the top, bottom and outside edges, as KDP wants. That means the right edge
  of a recto (odd) page and the left edge of a verso.
- **Fills.** A colour, or a gradient:
  - `{"gradient": "linear", "angle": 90, "stops": [["#fff", 0], ["#000", 1]]}`
  - `{"gradient": "radial", "stops": […], "center": [0.5, 0.5], "radius": 0.5}`
- **Strokes.** A colour, or `{"width": "0.5pt", "color": "#000", "dash":
  "dashed" | "dotted" | [3, 2], "cap": "round", "join": "round"}`. Box borders
  can also be given per side: `{"top": …, "bottom": …}`.
- **Insets** and **radius.** One length, or `{"top", "right", "bottom", "left",
  "x", "y"}` (insets) or `{"top_left", …}` (radius).

## The design

### Colours and font stacks

- `colors` maps names to colours.
- `fonts` maps names to a family or a list of families, such as `"serif":
  ["IBM Plex Serif", "Noto Serif"]`. A style's `font` may name a stack. Later
  families supply glyphs the first one lacks.

### Styles

`base` is the default for all text. `styles` holds paragraph styles; `spans`
holds character styles. A paragraph style has every character setting plus
paragraph settings.

Character settings (also usable inline on runs, and as `set` overrides):

| field | meaning |
|---|---|
| `font` | family, stack name, or list |
| `size` | length |
| `weight` | 100–900, or `thin` … `regular` `medium` `semibold` `bold` … `black` |
| `italic` | bool |
| `stretch` | width in % for fonts with widths |
| `tracking` | letter spacing (`"0.12em"`) |
| `word_spacing` | extra space between words |
| `color` | colour |
| `case` | `upper`, `lower`, `small-caps`, `all-small-caps` |
| `features` | OpenType features: `["onum", "ss01", "-liga", "salt=2"]` |
| `baseline` | raises (positive) or lowers the text |
| `underline`, `strike` | bool |
| `lang` | BCP 47, for hyphenation and shaping |
| `outline` | a stroke around each glyph (also fakes a bold) |
| `hyphenate`, `ligatures` | bool |
| `variations` | variable-font axes, such as `{"opsz": 12, "SOFT": 50}`. `weight`, `stretch` and `size` already drive `wght`, `wdth` and `opsz`, so a variable font needs no static instances. |

Paragraph settings:

| field | meaning |
|---|---|
| `inherits` | another style to start from |
| `line_height` | baseline to baseline. A bare number is a multiple of the size (CSS model: half the extra leading above, half below); a length is absolute. |
| `align` | `left`, `right`, `center`, `justify`, `outside`, `inside` |
| `indent` | first-line indent. Not applied after a heading, break or anything but a paragraph, unless `indent_first`. |
| `hanging` | indent of all lines but the first |
| `margin_left`, `margin_right` | indent of the whole paragraph |
| `space_before`, `space_after` | adjacent spaces collapse to the larger, as in CSS |
| `keep_with_next`, `keep_together` | bool |
| `allow_widows`, `allow_orphans` | widows and orphans are prevented by default |
| `page_break_before` | bool |
| `balance` | lines of equal length (CSS `text-wrap: balance`) |
| `rule_above`, `rule_below` | `{"stroke", "width", "gap"}` |
| `fill`, `inset` | a background box behind the paragraph |

### Masters

A master is a page size, a flow area and what is drawn on every page:

```json
"body": {
  "width": "6in", "height": "9in", "bleed": "0.125in",
  "margins": {"top": "0.8in", "bottom": "0.9in", "inside": "0.8in", "outside": "0.7in"},
  "columns": 1, "gutter": "12pt",
  "fill": "paper",
  "style": "body",
  "notes": {"width": "1.1in", "gap": "0.15in", "side": "outside", "style": "note", "spacing": "6pt"},
  "background": [ elements… ],
  "foreground": [ elements… ],
  "inherits": "another master"
}
```

Elements in `background` and `foreground` can carry `"when"`. The conditions
are listed under Elements below.

### Components

A component is a named, reusable composition with parameters:

```json
"components": {
  "plate": {
    "params": {"src": null, "caption": "", "tint": "#eee"},
    "w": "4in", "h": "3in",
    "elements": [
      {"type": "rect", "x": 0, "y": 0, "w": "4in", "h": "3in", "fill": "{{tint}}"},
      {"type": "image", "src": "{{src}}", "x": 6, "y": 6, "w": "3.8in", "h": "2.6in"},
      {"type": "text", "x": 6, "y": "2.7in", "w": "3.8in", "style": "caption", "text": "{{caption}}"}
    ]
  }
}
```

- An instance's `props` replace the `{{name}}` placeholders.
- A string that is exactly `"{{name}}"` takes the prop's value as it is: a
  number, a list of elements, an object. A placeholder inside a longer string
  takes the prop's text.
- `null` marks a required parameter.
- A component made of `blocks` instead of `elements` is used in flows.

## Content

### Sections

A section is a flow poured through pages of a master:

```json
{"type": "section", "id": "ch1", "master": "body", "start": "recto",
 "numbering": "1", "page_from": 1, "title": "The Machine", "vars": {"chapter": "One"},
 "style": "body", "flow": [ blocks… ]}
```

- `start`: `next` (default), `recto`, `verso`, or `continue` (on the same page
  as the section before, if it uses the same master). A page left empty so a
  section can start on the right side is a **blank** page.
- `numbering`: `1`, `i`, `I`, `a`, `A`. It carries over to later items.
- `page_from` restarts the page count.
- `title` and `vars` feed fields in the master's layers:
  `{"field": "section.title"}`, `{"field": "section.chapter"}`.

### Pages

A page is laid out by hand:

```json
{"type": "page", "id": "p48", "master": "body", "layers": false, "fill": "#e8e2d0",
 "title": "…", "vars": {}, "elements": [ elements… ]}
```

The master gives the page its size and bleed. `layers: true` also draws the
master's background and foreground.

## Elements (placed)

Every element takes:

| field | meaning |
|---|---|
| `id` | optional; must start with a letter and use only letters, digits, `-`, `_`, `.`, `:` |
| `x`, `y` | position |
| `w`, `h` | size |
| `anchor` | which point of the element `x`/`y` place: `top-left` (default), `top`, `top-right`, `left`, `center`, `right`, `bottom-left`, `bottom`, `bottom-right` |
| `rotate` | degrees clockwise, about the centre |
| `skew` | degrees of horizontal slant |
| `when` | conditions, all of which must hold: `recto`, `verso`, `first`, `blank`, and their `not-` forms (`not-first`, `not-blank`) |
| `decorative` | art that may cross the trim; margin checks skip it |

The types:

- **`text`.** A frame of text, `w` wide.
  - Content: `text` (a blank line starts a paragraph, a newline breaks the
    line), `content` (runs), or `blocks`.
  - Options: `style`, `set` (overrides), `valign` (`top`, `middle`,
    `bottom`; needs `h`), `inset`, `fill`, `stroke`, `radius`, `columns`,
    `gutter`, `clip`.
  - `fit: "shrink"` scales the type down until the text fits `h`, no smaller
    than `min_size`.
  - A frame with an `h` and an `id` reports overflow when its text doesn't fit.
- **`image`.**
  - `src` is a raster image or an SVG.
  - `fit` is `cover` (default; fills the box and crops), `contain` or
    `stretch`.
  - Also: `radius`, `stroke`, `flip` (`x`, `y`, `both`).
- **`rect`**, **`ellipse`**: `w`, `h`, `fill`, `stroke`, `radius` (rect only).
- **`line`**: `x1`, `y1`, `x2`, `y2`, `stroke`.
- **`path`**: `d` (SVG path data) in a `w` × `h` box, whose coordinates are
  points unless `viewbox` (`[x, y, w, h]`) is given. Also `fill` and `stroke`.
- **`svg`**: a drawing from `src`, or inline `markup`; `fit`.
- **`group`**:
  - `children` are placed from the group's corner.
  - `scale` (1 = actual size), `clip` (needs `w` and `h`), `fill`, `stroke`,
    `radius`.
- **`use`**: an instance of a component: `component`, `props`.
- **`qr`**: `data`, `w` (the code's size), `color`, `background`, `quiet`
  (modules), `ecc` (`L`, `M`, `Q`, `H`).
- **`lettering`**: display text drawn as vector outlines, not live text. Use it
  for type that runs into the bleed or must never reflow.
  - Settings: `text`, any character setting (`font`, `size`, `weight`,
    `tracking`, `features`, `color`, `outline`, `case`), `span`, `w` with
    `align`, `line_height`.
  - `x`/`y` place the top of the font's ascender, as CSS does.
- **`letter-grid`**: word searches, crosswords and letter puzzles.
  - `rows`: one string per row, or rows of cells, each `"A"` or `{"text",
    "fill", "color", "weight", "span", "number", "block"}`.
  - Layout: `cell` (size), `cell_h`.
  - Type: `style` (a paragraph or character style), `set`.
  - Drawing: `lines` (between cells), `frame` (outer border), `fill`,
    `radius`.
  - `highlights`: `{"from": [row, col], "to": [row, col], "fill", "stroke",
    "thickness"}`. These are capsules over found words, drawn under the
    letters; `"shape": "cells"` fills each cell instead.

## Blocks (in flows and frames)

| type | fields |
|---|---|
| `p` | `id`, `style`, `text` or `content` (runs), `set` |
| `h` | `level`, `style` (defaults to `h1`, `h2`… when the design has them), `text`/`content`, `outline` (listed in the contents), `short` (title for running heads and contents) |
| `break` | a scene break: `style`, `text` (the ornament) |
| `figure` | `src`, `w`, `h`, `fit`, `align`, `caption`/`caption_content`, `caption_style`, `float` (`top`, `bottom`), `stroke`, `radius` |
| `box` | `blocks`, `style`, `fill`, `stroke`, `radius`, `inset`, `w`, `h`, `align`, `breakable`, `rotate`, `space_before`, `space_after` |
| `table` | `columns` (lengths, `"auto"`, `"1fr"`), `header`, `rows` (cells: a string or `{"text"/"content"/"blocks", "colspan", "rowspan", "fill", "align", "style"}`), `style`, `header_style`, `header_fill`, `zebra`, `stroke`, `inset`, `align` (per column), `repeat_header` (default true) |
| `list` | `items` (strings or lists of blocks), `ordered`, `marker` (or a numbering pattern such as `"1."`), `style`, `indent`, `gap`, `spacing`, `start` |
| `code` | `text` (spaces kept), `style`, `fill`, `stroke`, `inset`, `radius`, `breakable` |
| `canvas` | an inline drawing: `w` (default the column's width), `h`, `elements`, `align`, `fill`, `clip` |
| `use` | a component instance (made of blocks, or of elements as a canvas) |
| `page-break` | `to`: `recto` or `verso` |
| `space` | `h`, `weak` |
| `columns` | `count`, `gutter`, `blocks` |
| `margin` | a note in the master's note column, level with the block before it (`at: "here"` for where it stands); `dy`; `text`/`content`/`blocks`. Notes stack and don't overlap. |
| `toc` | the contents: `depth`, `style` (default `toc-1`, `toc-2`…), `leader` (such as `"."`) |
| `keep` | `blocks` that stay on one page |

## Inline content (runs)

`content` is a list:

- **Text.** A plain string, or a run: `{"text": "…", "em", "strong", "span",
  "link", "sup", "sub", "nobreak", …any character setting}`.
- **Footnote.** `{"note": [ … ]}`.
- **Field.** `{"field": "page" | "pages" | "heading" | "section.title" |
  "section.<var>" | "doc.title" | "doc.author"}`, filled in at layout.
- **Page reference.** `{"ref": "node-id"}` gives the page number;
  `{"ref": "node-id", "show": "title"}` gives the title.
- **Line break.** `{"br": true}`.
- **Inline image.** `{"image": "path", "h": "1em", "baseline": "0.2em"}`.

## Operations

Every change is an operation, and applying one returns its inverse. A node is
addressed by `id`, or by `at`, a JSON pointer from the root.

```json
{"op": "set", "id": "p12", "path": "/style", "value": "lede"}
{"op": "unset", "id": "p12", "path": "/set/size"}
{"op": "insert", "id": "ch1", "field": "flow", "index": 3, "value": {"type": "p", "id": "p99", "text": "…"}}
{"op": "remove", "id": "p99"}
{"op": "move", "id": "fig2", "to": {"id": "ch2"}, "field": "flow", "index": 0}
{"op": "insert-text", "id": "p12", "offset": 40, "text": "very "}
{"op": "delete-text", "id": "p12", "start": 40, "end": 45}
{"op": "set", "at": "/design/styles/body", "path": "/size", "value": 10.5}
```

## Checks

`matenadar check` reports:

- `overflow`: a frame's text doesn't fit.
- `text-in-margin`: live text closer to the trim than KDP allows. That is
  0.25 in outside, and at the spine, KDP's minimum for the page count.
- `low-resolution`: a raster image under 300 ppi at its printed size.
- `missing-glyph`: no font has the glyph.
- `fallback-font`: a family no style names was used.
- `widow`, `orphan`.
- `missing-file`.
- `compiler`: notices from the layout engine.

## Command line

```
matenadar build <doc.json> [-o out.pdf] [--png DIR] [--svg DIR] [--dpi N] [--pages 1-3,7] [--bleed] [--check] [--json]
matenadar check <doc.json> [--safe 0.25in] [--gutter 0.375in] [--min-ppi 300] [--json]
matenadar validate <doc.json>
matenadar layout <doc.json> [--pages 1-3]        # every node on every page, with its extent
matenadar locate <doc.json> --page N --x PT --y PT
matenadar apply <doc.json> <ops.json> [-o new.json]
matenadar fonts [--fonts DIR]
```

Common options: `--root DIR`, `--fonts DIR` (repeatable), `--no-system-fonts`.
