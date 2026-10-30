# The Matenadar document model (version 1)

A document is JSON. It is the only source of truth: the engine generates its
layout program from it on every build and never stores that program.

```json
{
  "matenadar": "1",
  "meta": {"title": "…", "author": "…", "lang": "en-US"},
  "fonts": ["design/fonts"],
  "design": {"colors": {}, "fonts": {}, "base": {}, "styles": {}, "spans": {}, "masters": {}, "components": {}, "hyphenation": {}, "figures": {}},
  "content": [ {"type": "section", …}, {"type": "page", …}, {"type": "spread", …} ],
  "print": {"ink": "black"}
}
```

- `fonts`: font folders, relative to the asset root, searched before the
  system's fonts.
- **Asset root:** image, SVG and font paths are relative to it. On the command
  line it defaults to the document's folder; `--root` sets it. Nothing outside
  it can be read.
- `print`: how the book is printed, which the checks hold it to.
  - `ink`: `"black"` (a black-ink interior: anything drawn in colour is
    reported, `color-in-black-ink`) or `"color"`.
  - `filter`: a filter for every raster picture that sets none of its own,
    such as `{"gray": true, "contrast": 3}` (see Pictures). A picture opts
    out with `"filter": {}`.

## Units, colours, coordinates

- **Lengths.** A number is points. A string can be `"12pt"`, `"0.5in"`,
  `"3mm"`, `"1.2cm"`, `"16px"` (0.75 pt), `"10tpt"` (TeX points, 1/72.27 in),
  `"1.5em"` (relative to the text size) or `"50%"` (of the containing box).
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
  "dashed" | "dotted" | [3, 2], "dash_offset": 1.5, "cap": "round",
  "join": "round"}`. Box borders
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
| `size` | length; `em` or `%` are of the surrounding size |
| `weight` | 100–900, or `thin` … `regular` `medium` `semibold` `bold` … `black` |
| `italic` | bool |
| `stretch` | width in % for fonts with widths |
| `tracking` | letter spacing (`"0.12em"`). Every letter is followed by it, the last letter of a run included (so splitting a word into runs never changes its width), except the paragraph's last. Letter-spaced text drops ligatures unless `ligatures` is set. |
| `word_spacing` | extra space between words |
| `color` | colour |
| `case` | `upper`, `lower`, `small-caps`, `all-small-caps` |
| `features` | OpenType features: `["onum", "ss01", "-liga", "salt=2"]` |
| `baseline` | raises (positive) or lowers the text |
| `underline`, `strike` | bool |
| `lang` | BCP 47, for hyphenation and shaping |
| `outline` | a stroke around each glyph (also fakes a bold) |
| `hyphenate`, `ligatures` | bool |
| `overhang` | let punctuation and hyphens hang slightly into the margin at line ends (default true) |
| `line_height` | see paragraph settings; on a run, `0` keeps a large letter or mark from opening its line |
| `halo` | a halo behind each glyph for type over pictures: a stroke, `{"width": 2, "color": "#fff"}` |
| `variations` | variable-font axes, such as `{"opsz": 12, "SOFT": 50}`. `weight`, `stretch` and `size` already drive `wght`, `wdth` and `opsz`, so a variable font needs no static instances. |

Paragraph settings:

| field | meaning |
|---|---|
| `inherits` | another style to start from |
| `line_height` | baseline to baseline. A bare number is a multiple of the size; a length is absolute. Lines sit as in CSS: half the extra leading above, half below, with the font's ascent and descent rounded to whole CSS pixels as browsers do. A run in a larger size opens its line in proportion. |
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
| `linebreaks` | `simple` (one line at a time, as browsers do) or `optimized` (the whole paragraph at once, Knuth–Plass, for evener lines). By default justified text is optimized and ragged text simple. |
| `wrap` | `pretty`: the paragraph never ends on a lone word (its last two words stay together) |
| `justification` | how far justified lines may stretch and shrink: `{"spacing": {"min": "100%", "max": "150%"}, "tracking": {"min": "-0.01em", "max": "0.02em"}}`. `min: "100%"` never tightens word spaces. |
| `drop_cap` | `{"lines": 3, "font", "color", "weight", "gap"}`. The cap's top lines up with the first line's capitals and its foot with the baseline of line `lines`. It applies to paragraphs of plain text, and an opening quote goes with the cap. |

### Hyphenation

Typst hyphenates by the language's patterns. `hyphenation.exceptions` lists
words whose breaks you choose instead: hyphens mark the only places a word
may break, and a word written without hyphens never breaks.

```json
"hyphenation": {"exceptions": ["Mate-na-dar", "pro-gram", "Levon"]}
```

Matching ignores case. The exceptions apply in paragraphs that hyphenate:
justified ones, unless `hyphenate` is false, and any with `hyphenate: true`.

### Figures

`figures` turns numbering on. Every figure with a caption then gets a number
(`numbered: false` leaves one out; `numbered: true` numbers one without a
caption), set before the caption as a label:

```json
"figures": {"label": "Fig.", "per": "chapter", "separator": ". ", "span": "figlabel"}
```

- `label`: the word before the number (default `"Figure"`). A no-break space
  joins it to the number.
- `per`: `"book"` (default: 1, 2, 3… through the book) or `"chapter"`: the
  count restarts in each section, after the section's `chapter` var when it
  has one (`3.1`, `3.2`).
- `separator`: between the number and the caption's words (default `". "`).
- `span`: a character style for the label and number.

Figures are numbered in the order they stand in the document, wherever a float
or plate puts them; print and EPUB number them alike. The list of figures
(`lof`) and references (`{"ref": "fig-id", "show": "label"}`) find them by id.

### Masters

A master is a page size, a flow area and what is drawn on every page:

```json
"body": {
  "width": "6in", "height": "9in", "bleed": "0.125in",
  "margins": {"top": "0.8in", "bottom": "0.9in", "inside": "0.8in", "outside": "0.7in"},
  // or the same on every page: {"top", "bottom", "left", "right"}
  "columns": 1, "gutter": "12pt",
  "fill": "paper",
  "style": "body",
  "notes": {"width": "1.1in", "gap": "0.15in", "side": "outside", "style": "note", "spacing": "6pt"},
  "background": [ elements… ],
  "foreground": [ elements… ],
  "inherits": "another master"
}
```

Margins mirror (`inside` is the spine side) unless `left` or `right` is
given. A note column's `side` is `outside` (default), `inside`, `left` or
`right`.

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
  section can start on the right side is a **blank** page. A document whose
  first item starts on a `verso` opens with a blank page.
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
master's background and foreground. Pages take `start` too.

### Spreads

A spread is two facing pages under one picture: a verso and the recto after
it. It always opens on a verso (a blank page is left before it if needed).

```json
{"type": "spread", "id": "s-dig", "master": "page", "src": "art/dig-site.jpg",
 "focus": [0.5, 0.6], "gutter": "0.125in", "alt": "…",
 "left": {"elements": [ … ]}, "right": {"elements": [ … ]}}
```

- The picture covers both trims and their outer bleeds (top, bottom and each
  page's outside edge; there is no bleed at the spine), `cover`-fitted, with
  `crop` and `focus` as for images.
- `gutter`: how much of each page the binding hides at the spine. That strip
  is repeated on both sides of the fold, so the picture reads across it
  without losing what is there. Default 0, for lay-flat bindings; perfect-bound
  paperbacks want about 0.125 in.
- `left` and `right` are the pages: `elements` placed over the picture from
  that page's trim corner (words over art, see text frames), and their own
  `crop` and `focus`, which move only that page's half. A side can have its
  own `src`: a spread drawn as two halves, each covering its page and its
  outer bleed.
- `filter`, `layers`, `fill`, `title`, `vars`, `page_from` and `numbering`
  as for pages.

### On the page of a node

`on_page` (at the document's top level) changes the page where a node
lands, keyed by the node's id, so the change follows the node when the text
reflows:

```json
"on_page": {"f-plate-3": {"hide": ["folio", "running"]}}
```

- `hide`: master elements not drawn on that page. Each entry is an element's
  `id`, the style of a text element (`"folio"`, `"running"`), or `"all"`.
  Use it for a page with a full-bleed picture, or a part title.
- A key whose node is gone does nothing.

## Elements (placed)

Every element takes:

| field | meaning |
|---|---|
| `id` | optional; must start with a letter and use only letters, digits, `-`, `_`, `.`, `:` |
| `x`, `y` | position |
| `w`, `h` | size |
| `anchor` | which point of the element `x`/`y` place: `top-left` (default), `top`, `top-right`, `left`, `center`, `right`, `bottom-left`, `bottom`, `bottom-right`. Text frames also take `baseline-left`, `baseline` and `baseline-right`, which place the first line's baseline (as TikZ does). |
| `rotate` | degrees clockwise, about the centre or `origin` |
| `origin` | the point `rotate` turns about, which stays put: an anchor word (`top-left`…) or `[x, y]` from the element's corner |
| `opacity` | 0–1, for everything in the element: text, fills, strokes, drawings and photographs. Printers prefer `tint` on drawings. |
| `stroke_align` | where a stroke lies on the edge: `center` (default), `inside` (as CSS borders and most design tools draw it), `outside` |
| `skew` | degrees of horizontal slant |
| `when` | conditions, all of which must hold: `recto`, `verso`, `first`, `blank`, and their `not-` forms (`not-first`, `not-blank`) |
| `decorative` | art, not reading text. Its text is drawn as glyph outlines in the PDF, so display type may run into the margins and bleed without KDP's previewer (or any reader) seeing text there, and margin checks skip it. The layout report still lists it. |
| `shadow` | a hard shadow: `{"dx": 3, "dy": 3, "color": "#0004"}` (needs `w` and `h`, except on text frames) |

The types:

- **`text`.** A frame of text, `w` wide, or without `w` as wide as its
  longest line (a pill, a tag).
  - Content: `text` (a blank line starts a paragraph, a newline breaks the
    line), `content` (runs), or `blocks`.
  - Options: `style`, `set` (overrides), `valign` (`top`, `middle`,
    `bottom`; needs `h`), `inset`, `fill`, `stroke`, `radius`, `columns`,
    `gutter`, `clip` (see below).
  - `fit: "shrink"` scales the type down until the text fits `h`, no smaller
    than `min_size`.
  - `fit: "width"` scales the type up or down until the longest line fills
    `w`, between `min_size` and `max_size`.
  - A frame with an `h` and an `id` reports overflow when its text doesn't fit.
  - **Words over art.** A frame set over a picture can have:
    - `scrim`: a soft panel behind it, so its words read on any part of the
      art: `{"color": "#fff", "opacity": 0.8, "feather": "10pt", "inset":
      "8pt", "radius": "6pt", "shape": "rect" | "ellipse"}`. `feather` fades
      the edge out; `inset` is how far the panel reaches past the frame.
    - `safe`: `true` keeps the frame inside the master's margins, or a length
      keeps it that far in from the trim on every side. The frame is moved,
      never resized, when it would cross (at layout, so a frame whose height
      depends on its words is kept in too). It applies to frames placed on a
      page itself; it needs a `w`.
    - A halo per glyph: `"set": {"halo": {"width": 2, "color": "#fff"}}`.
- **`image`.**
  - `src` is a raster image or an SVG.
  - `focus: [fx, fy]` chooses which part of a cropped (`cover`) image stays in
    view, as fractions: `[0.85, 0.5]` keeps the right side. Needs `w` and `h`
    in absolute units.
  - `fit` is `cover` (default; fills the box and crops), `contain` or
    `stretch`.
  - `crop: {"x", "y", "w", "h"}` shows only that part of the picture (fractions
    of its width and height from its top-left corner; `x` and `y` default to
    0, `w` and `h` to 1), which `fit` and `focus` then treat as the whole.
  - Also: `radius`, `stroke`, `flip` (`x`, `y`, `both`), `clip` (a shape).
  - `alt`: what the picture shows, for readers who can't see it (ebooks).
  - `filter`: changes a raster picture's tones before it is placed (an SVG
    keeps its own): `{"gray": true, "contrast": 3, "black": 0.02, "white":
    0.98, "gamma": 1.1}`.
    - `gray`: shades of grey from Rec. 709 luma, for black-ink interiors.
    - `black`, `white`: input levels (the tones that become black and white).
    - `gamma`: midtones; above 1 lightens.
    - `contrast`: an S-curve about the middle grey (0 none, 3 moderate, 8
      strong) that never clips the ends.

    Photographs stay JPEG; other pictures become PNG. The result is cached,
    so editing around a filtered picture costs nothing more.
- **`rect`**, **`ellipse`**: `w`, `h`, `fill`, `stroke`, `radius` (rect only).
- **`line`**: `x1`, `y1`, `x2`, `y2`, `stroke`.
- **`path`**: `d` (SVG path data) in a `w` × `h` box, whose coordinates are
  points unless `viewbox` (`[x, y, w, h]`) is given. Also `fill` and `stroke`.
- **`svg`**: a drawing from `src`, or inline `markup`; `fit`; `viewbox`
  (`[x, y, w, h]`, to show part of the drawing); `tint: {"color", "amount"}`
  mixes every colour in it towards one (a print-safe fade, or a duotone).
- **`group`**:
  - `children` are placed from the group's corner.
  - `scale` (1 = actual size), `clip` (see below), `fill`, `stroke`,
    `radius`.
- **`use`**: an instance of a component: `component`, `props`.
- **Clipping.** Text frames, images and groups take `clip`: `true` cuts
  what they hold to their box (a group needs `w` and `h`), `"ellipse"` to
  the ellipse in that box, and SVG path data (`"M 0 72 L 36 0 L 72 72 Z"`,
  in points from the element's corner) to any outline. For round portraits,
  type in a circle, torn and die-cut edges.
- **`qr`**: `data`, `w` (the code's size), `color`, `background`, `quiet`
  (modules), `ecc` (`L`, `M`, `Q`, `H`). Drawn as one vector shape.
- **`lettering`**: display text drawn as vector outlines, not live text. Use it
  for type that runs into the bleed or must never reflow.
  - Settings: `text`, any character setting (`font`, `size`, `weight`,
    `stretch`, `variations`, `tracking`, `features`, `color`, `outline`,
    `case`), `span`, `w` with `align`, `line_height`.
  - `x`/`y` place the top of the font's ascender, as CSS does.
  - **Text on a path**: `path` (SVG path data, in points from `x`/`y`) sets
    the letters standing on the path, turned to it; `start` (a length or a
    share, `"50%"`) and `align` (`left`, `center`, `right`) say where along
    it the text starts, centres or ends. For seals, postmarks and badges.
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
| `figure` | `src`, `w`, `h`, `fit`, `crop` and `focus` (as for images), `align`, `caption`/`caption_content`, `caption_style`, `float` (`top`, `bottom`), `stroke`, `radius`, `alt`, `filter` (as for images), `numbered`, `short` (the caption the list of figures gives); `plate`: `half` (half the text area, at the foot of a page unless `float: top`), `full` (a page of its own within the margins: the whole picture as large as it fits, its caption under it) or `bleed` (a page of its own to the bleed, the master's elements hidden there, the caption on a panel at the foot of the text area). Plates float: the text runs on past them. |
| `box` | `blocks`, `style`, `fill`, `stroke`, `stroke_align`, `radius`, `inset`, `w`, `h`, `align` (places the box; its text keeps its own alignment), `breakable`, `rotate`, `space_before`, `space_after`, `margin_left`/`margin_right` (negative hangs the box into the margins) |
| `table` | `columns` (lengths, `"auto"`, `"1fr"`), `header`, `rows` (cells: a string or `{"text"/"content"/"blocks", "colspan", "rowspan", "fill", "align", "style", "set"}`), `style`, `header_style`, `header_fill`, `zebra`, `stroke`, `rules` (book-style horizontal rules in place of the grid: `{"top", "header", "rows", "bottom", "columns"}`, each a stroke), `inset`, `align` (per column), `valign` (`top`, `middle`, `bottom`), `row_gap`, `repeat_header` (default true) |
| `list` | `items` (strings or lists of blocks), `ordered`, `marker` (or a numbering pattern such as `"1."`), `style`, `indent`, `gap`, `spacing`, `start`, `marker_style` (a span for the markers), `marker_shape` (`{"shape": "ellipse" \| "rect", "w", "h", "fill", "stroke", "radius"}`: a disc or tile behind each marker, centred on it), `marker_align` (`left`, or `right` for numbers, the default) |
| `code` | `text` (spaces kept), `style`, `fill`, `stroke`, `inset`, `radius`, `breakable` |
| `canvas` | an inline drawing: `w` (default the column's width), `h` (a length, or `"1fr"` for what is left), `elements`, `align`, `fill`, `stroke`, `radius`, `clip`. Percentages inside are of the canvas. |
| `use` | a component instance (made of blocks, or of elements as a canvas) |
| `page-break` | `id`; `to`: `recto` or `verso`; `weak: true` (no break where the page has nothing on it yet) |
| `space` | `h` (a length, or `"1fr"` to push what follows to the foot), `weak` |
| `columns` | `count`, `gutter`, `blocks`, `rule` (a stroke between columns), `balance: true` (blocks shared out so the columns end level; blocks stay whole — for indexes) |
| `column-break` | continue in the next column |
| `margin` | a note in the master's note column, level with the top edge of the block before it (`at: "here"` for where it stands); `dy`; `text`/`content`/`blocks`. Several notes after one block stack under each other; notes never overlap. |
| `toc` | the contents: `depth`, `style` (default `toc-1`, `toc-2`…), `leader` (such as `"."`) |
| `lof` | the list of figures: every numbered figure's number, caption and page; `style` (default `lof`, then `toc-1`), `leader` |
| `keep` | `blocks` that stay on one page |

## Inline content (runs)

`content` is a list:

- **Text.** A plain string, or a run: `{"text": "…", "em", "strong", "span",
  "link", "sup", "sub", "nobreak", "mark", …any character setting}`.
- **Marks.** `mark` is a pen mark that follows where the run lands:
  `{"kind": "underline" | "strike" | "highlight" | "box" | "ring", "stroke",
  "fill", "offset", "extent", "radius", "inset"}`.
- **Footnote.** `{"note": [ … ]}`.
- **Leader.** `{"leader": "."}` fills the rest of the line with dots, for
  contents and index lines; `{"leader": ""}` fills it with space, pushing what
  follows to the right edge. Also `gap`, `span`, and `fixed: true` (dots
  exactly `gap` apart, as TeX sets them). `{"leader": "rule", "stroke",
  "lift", "gap"}` draws a line across the space: running feet, form lines.
- **Space.** `{"space": "0.13in"}` is horizontal space inside a line.
- **Field.** `{"field": "page" | "pages" | "heading" | "section.title" |
  "section.<var>" | "doc.title" | "doc.author"}`, filled in at layout.
- **Page reference.** `{"ref": "node-id"}` gives the page number;
  `{"ref": "node-id", "show": "title"}` gives the title. For a numbered
  figure, `"show": "number"` gives its number (`3.2`) and `"show": "label"`
  its label (`Figure 3.2`).
- **Line break.** `{"br": true}`.
- **Inline image.** `{"image": "path", "h": "1em", "baseline": "0.2em"}`, or
  inline SVG: `{"image": "", "markup": "<svg …>", "h": "1em"}`.

## Operations

Every change is an operation, and applying one returns its inverse. A node is
addressed by `id`, or by `at`, a JSON pointer from the root.

```json
{"op": "set", "id": "p12", "path": "/style", "value": "lede"}
{"op": "unset", "id": "p12", "path": "/set/size"}
{"op": "insert", "id": "ch1", "field": "flow", "index": 3, "value": {"type": "p", "id": "p99", "text": "…"}}
{"op": "insert", "after": "p12", "value": {"type": "p", "id": "p99", "text": "…"}}
{"op": "remove", "id": "p99"}
{"op": "move", "id": "fig2", "to": {"id": "ch2"}, "field": "flow", "index": 0}
{"op": "insert-text", "id": "p12", "offset": 40, "text": "very "}
{"op": "delete-text", "id": "p12", "start": 40, "end": 45}
{"op": "set", "at": "/design/styles/body", "path": "/size", "value": 10.5}
```

`insert` with `before` or `after` a sibling's id keeps its place when the
list around it changes, so an edit replayed on a document recomposed from a
changed manuscript still lands where it was made. Prefer ids to `index` and
`at` in anything kept.

## Checks

`matenadar check` reports:

- `overflow`: a frame's text doesn't fit.
- `text-in-margin`: live text closer to the trim than KDP allows. That is
  0.25 in outside, and at the spine, KDP's minimum for the page count.
- `low-resolution`: a raster image under 300 ppi at its printed size.
- `color-in-black-ink`: on a black-ink interior (`print.ink`, or `--ink
  black`), a picture with colour in it (more than a few clearly coloured
  pixels, or an overall cast), or text or a shape in a colour that isn't a
  grey. One report per element and page.
- `missing-glyph`: no font has the glyph.
- `fallback-font`: a family no style names was used.
- `widow`, `orphan`.
- `missing-file`.
- `compiler`: notices from the layout engine.

## Templates

A template makes a book kind's whole document from a short description (a
spec). The document is then edited like any other. Items keep the ids their
spec gives them (`id`; otherwise their place, as `story-03`), so edits kept as
operations land where they were made when the spec changes and the document
is composed again. `matenadar template <kind> <spec.json> -o doc.json`; in
WebAssembly, `template({kind, spec})`. Every spec takes `design`: changes to
the template's design, merged into it object by object (`{"styles": {"title":
{"size": 48}}}`).

### `picture-book`

Story pages are pictures to the bleed with their words lettered in the art
(the default); front and back matter is typeset.

```json
{"trim": "8.5x11", "pages": 32, "ends": "separate", "title": "…", "subtitle": "…",
 "author": "…", "illustrator": "…", "publisher": "…", "copyright": "…", "dedication": "…",
 "fonts": {"display": "…", "text": "…"}, "font_dirs": ["fonts"],
 "story": [
   {"src": "art/p03.jpg", "alt": "…"},
   {"spread": true, "src": "art/p04-05.jpg", "gutter": "0.125in",
    "left": {"text": "Words set over the art, on request."}},
   {"spread": true, "left": {"src": "art/p06.jpg"}, "right": {"src": "art/p07.jpg"}}
 ],
 "back": [{"heading": "About the author", "text": "…"}]}
```

- `trim`: `8.5x8.5`, `8x10` or `8.5x11` (inches), with a 0.125 in bleed
  (`bleed` changes it). `pages`: 24, 32 or 40.
- `ends: "separate"` (default; the endpapers are their own sheets): page 1 is
  the title page (or with `half_title`, the half title, and the title page on
  page 3), its back the copyright and dedication, and the story runs from
  there.
- `ends: "self"` (the first and last leaves are the endpapers): pages 1 and N
  are pasted to the boards, 2–3 and N−2–N−1 are the endpapers (`endpapers`:
  `{"src"}`, or `front` and `back`), 4 the copyright and dedication, 5 the
  title page, and the story runs from 6 to N−3.
- A story item is a page (`src`, with `crop`, `focus` and `alt`) or a spread
  (`spread: true`, as the spread item). `text` sets words over the art in a
  frame at the top of the page, kept in the safe zone on a soft panel;
  `words` places any elements over it.
- A spread must open on a left-hand (even) page, and the story and back
  matter must fit: the template says which item breaks the plan rather than
  leaving a page blank in the middle of a story. Pages left over at the end
  are blank.
- `title_art` puts a picture behind the title page, with the title on soft
  panels over it.

## EPUB

`matenadar epub` makes an EPUB 3 from the same model, checked with
epubcheck in the tests.

- **Reflowable** (the default): one XHTML file per section, the design's
  styles as CSS in `rem` and `em` (so readers can still change the size),
  headings as the contents, footnotes as EPUB notes. Placed pages and
  canvases become pictures, drawn by the engine at `--dpi`.
- **Fixed** (`--layout fixed`): every page a picture, for page-led books
  (comics, puzzles, picture books), with the page's text kept for search and
  read-aloud (each page's text is its picture's alt text).
- `--embed-fonts` adds the fonts the book uses (only the faces it needs);
  `--cover` sets the cover image, and `--identifier` the book's ID (an ISBN
  URN). Without one, the ID is derived from the title and author.

## WebAssembly and the page editor

`scripts/wasm.sh` builds the engine for the browser into `dist/`:

- `matenadar.wasm` (about 24 MB, 7 MB with brotli; no imports) and
  `matenadar.js`, the engine with a synchronous API;
- `worker.js`, which runs it in a worker, and `client.js`, the page's side,
  which gives every method as a promise;
- `matenadar.d.ts` and `client.d.ts`, the types; `REVISION`, the commit built.

It lays out exactly as the command line does: the same document, fonts and
pictures give the same pages, and `fingerprint` (a hash per page) shows it.
Give the browser every face the book uses (bold and italic included):
`matenadar fonts --used <doc.json>` lists the faces a layout drew with and
the file each came from, and `fontsUsed()` gives the browser's. The command
line also has Typst's own faces; `--embedded-fonts` adds them to the
WebAssembly build (about 5 MB).

```js
import { MatenadarClient } from "./client.js";
const mt = await MatenadarClient.start({ base: "/matenadar/<commit>/" });
const { fonts, files } = await mt.assets(doc);         // what to fetch
for (const font of fontFiles) await mt.addFont(font);  // ArrayBuffers
for (const path of files) await mt.addFile(path, await fetchBytes(path));
const { pages, diagnostics } = await mt.open(doc);
pageEl.innerHTML = await mt.svg(1, { bleed: true });
const hit = await mt.hit(1, x, y);                     // points from the trim's corner
const done = await mt.setBlock(hit.id, { space_before: "12pt" });
for (const n of done.changed_pages) redraw(n);
await mt.undo();
const { words, layout } = await mt.patch();
```

The engine keeps the document and an undo history. Each edit is one step,
laid out once, and returns the pages to draw again, the page count and the
diagnostics (checked again only on the pages that changed):

| | |
|---|---|
| Loading | `assets(doc)`, `addFont(bytes)`, `addFile(path, bytes)`, `open(doc, check)`, `fonts()`, `close()` |
| Pages | `pages()` (trim, bleed per side, recto or verso, master, margins), `spreads()`, `svg(page, {bleed})`, `png(page, {scale, bleed})`, `rgba(page, {scale, bleed})` (for a canvas), `pdf()`, `fingerprint()`, `fontsUsed()` |
| Finding | `hit(page, x, y)` (the innermost node, its extent on the page, the byte offset in its words, the element around it and every ancestor), `rects(id)`, `node(id)`, `lines(id)`, `locate`, `position`, `layout`, `document()` |
| Words | `setWords(id, runs)`, `insertText(id, offset, text)` and `deleteText(id, start, end)` (marks kept), `insertBlock({after}, block)`, `removeBlock(id)`; `splitBlock(id, offset, newId, end)` (the words after the offset go to a new block after it, with its style and overrides but not those of a paragraph's start; a heading split at its end is followed by a paragraph; with `end`, the words `offset..end` go first, as Enter over a selection does, in the same step), `joinBlock(id)` (to the paragraph or heading before it: `{into, offset}`), `markText(id, start, end, field, value)` (`em`, `strong`, `span`, `color`, `size`, `tracking`, `case`… on a range of a paragraph's, heading's or text frame's words; null takes the mark off) |
| The caret | offsets are bytes of the node's plain text: `caret(id, offset, {upstream})` (`{page, x, y, h}`, `y` the line's top; where a line breaks inside a word, `upstream` keeps the caret at the end of the line before), `offsetAt(id, page, x, y)` (the nearest place in the words to a point), `selectionRects(id, start, end)` (a rectangle per line), `textLines(id)` (`[{page, rect, start, end}]`). They follow the glyphs, so they work in text frames (whose words are the frame's, blank lines included) and beside a drop cap |
| Source | `source()`: the Typst program the engine generated, `{text, prelude, nodes: [{id, kind, start, end}]}` with UTF-16 offsets, for showing it beside the page |
| Pictures | `setPicture(id, {src, crop, fit, focus, w, h, x, y, opacity})` |
| Blocks | `setBlock(id, {space_before, space_after, align, keep_with_next, keep_together, …})`: a paragraph's or heading's style overrides |
| Pages | `moveFrame(id, {x, y, w, h, rotate})` for placed elements; `breakBefore(id, "page" \| "recto" \| "verso" \| null)` forces or takes back a break (a weak `page-break` with the id `<id>--break`, or the style's `page_break_before` turned off); `hideOnPage(id, ["folio"])`; `nudge(id, -1 \| 1)` sets a paragraph a line shorter or longer by the least tracking that does it (at most 0.02 em) |
| Any | `apply(ops)` |
| History | `undo()`, `redo()`, `rebase()` (after saving the patch) |
| Results | `diagnostics()`, `check(options)`, `patch()` |

`patch()` gives the edits since the document opened, every one addressed by
id: `words` (changes to paragraphs', headings', figures' and lists' words,
for the manuscript, with the ids `touched`) and `layout` (everything else,
each field folded to its last setting, settings back to where they started
dropped). Replayed on the opened document, words first, they give the same
pages: `crates/matenadar-typst/tests/editor.rs` checks it.

`demo/editor/` is a page editor on the samples:
`scripts/wasm.sh && python3 scripts/demo.py && python3 -m http.server 5199`,
then `http://localhost:5199/demo/editor/`.

## Command line

```
matenadar build <doc.json> [-o out.pdf] [--png DIR] [--svg DIR] [--dpi N] [--pages 1-3,7] [--bleed] [--check] [--json]
matenadar epub <doc.json> [-o out.epub] [--layout reflowable|fixed] [--dpi 200] [--embed-fonts] [--cover image.jpg] [--identifier urn:isbn:…]
matenadar check <doc.json> [--safe 0.25in] [--gutter 0.375in] [--min-ppi 300] [--ink black] [--json]
matenadar validate <doc.json>
matenadar template <kind> <spec.json> [-o doc.json]
matenadar layout <doc.json> [--pages 1-3]        # every node on every page, with its extent
matenadar locate <doc.json> --page N --x PT --y PT
matenadar fingerprint <doc.json>                  # a hash per page, as the WebAssembly build gives
matenadar apply <doc.json> <ops.json> [-o new.json]
matenadar fonts [--fonts DIR] [--used <doc.json>]  # the families found; with --used, the faces a layout drew with and their files
matenadar bench <doc.json> [--edits 50]           # layout and edit timings
matenadar serve                                   # one document kept open, driven by JSON lines on stdin
```

Common options: `--root DIR`, `--fonts DIR` (repeatable), `--no-system-fonts`.
