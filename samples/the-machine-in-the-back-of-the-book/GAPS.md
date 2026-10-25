# Gaps: The Machine in the Back of the Book

Most valuable first. Each has what the book needed, what the sample does instead, and the JSON I would
like to write.

## Bugs (engine as of 2026-10-25 23:36)

### 1. `list` and `code` blocks don't compile

Any list or code block fails the whole build:

```json
{"matenadar": "1", "meta": {"title": "x"},
 "design": {"masters": {"m": {"width": "6in", "height": "9in", "margins": {"top": 50, "bottom": 50, "inside": 50, "outside": 50}}}},
 "content": [{"type": "section", "id": "s", "master": "m", "flow": [{"type": "list", "items": ["a", "b"]}]}]}
```

`error: layout failed: the character '#' is not valid in code (hint: you are already in code mode)`.
Same with `{"type": "code", "text": "abc"}`. The sample sets lists as two-column tables and code as a
box of paragraphs with NBSPs (`USE_LIST_BLOCK` in make.py switches back when this is fixed).

### 2. A rotated box drops its `space_before` and `space_after`

```json
{"type": "p", "text": "two", "set": {"space_after": 2}},
{"type": "box", "space_before": 20, "rotate": -1.2, "blocks": [{"type": "p", "text": "rotated"}]}
```

The box sits 2 pt under "two" (unrotated: 20 pt), and the next block touches it. Workaround: a `space`
block before it. (Hit in the Field Guide sample, recorded here too.)

### 3. A second margin block anchors to the next block, not under the first

```json
{"type": "p", "text": "…"},
{"type": "margin", "text": "Red pen"},
{"type": "margin", "text": "Blue pen"}
```

"Blue pen" lands level with the paragraph *after* this one. The book stacks margin items that follow
one block, 7 pt apart (the master's `notes.spacing`). Workaround: one margin block holding both notes.

### 4. `lettering` ignores `weight` and `stretch` on a variable font

`{"type": "lettering", "text": "LIFE", "font": "Archivo", "weight": 800, "stretch": 68}` draws the
default instance; the same settings in a text frame are right. The label page uses text frames.

### 5. `nobreak` breaks anyway when the run is wider than the measure

CSS `white-space: nowrap` overflows instead. The table's first column ("$61 $62 $63", nowrap in the
book) uses NBSPs.

## Features

### 6. Hyphenation exceptions and limits (p. 9 loses a line)

Chrome on macOS hyphenated "memo-ry" and left "describes" whole; Typst's patterns give "mem-ory" and
"de-scribes", and the first paragraph comes out a line short. An exception list (TeX's `\hyphenation`)
would let a sample match its original exactly, and limits would let a design set CSS's
`hyphenate-limit-chars`:

```json
"base": {"hyphenate": true, "hyphenation": {"min_word": 7, "before": 3, "after": 3,
                                             "exceptions": ["memo-ry", "describes", "com-puter"]}}
```

### 7. Rules on tables: header rule, row rules, per-row insets

Every table in the book is "booktabs": a heavy rule under the header, hairlines under rows, no
verticals, header cells padded differently from body cells and bottom-aligned. Bottom-only cell strokes
now work, but one `stroke` and one `inset` per table means the header is a second table. Wanted:

```json
{"type": "table", "columns": [50, 26, "1fr"],
 "header": [["Opcodes", "Name", "What it does"]],
 "header_rule": {"width": 1.1, "color": "ink"}, "header_inset": {"top": 0, "bottom": 2.5, "right": 6},
 "row_rule": {"width": 0.45, "color": "rule"}, "inset": {"top": 2.6, "bottom": 2.6, "right": 6},
 "rows": […]}
```

Also: spacing at the start or end of a table cell (`space_before`, a `space` block) is dropped, so cell
padding can only come from the table's inset; and a cell stroke sits on the grid line while a CSS border
adds its width to the row (the sample adds half of each rule to the insets).

### 8. List markers with their own style

The book's ordered markers are Plex Sans Condensed SemiBold 8.6 pt teal beside a 9.6 pt serif item;
bullets are teal discs. `list` has `marker` and `style` but no marker style:

```json
{"type": "list", "ordered": true, "start": 8, "indent": "0.21in", "spacing": 3.5,
 "marker_style": "ol-mark", "marker_gap": "4pt", "items": […]}
```

### 9. A section's first page on its own master (chapter openers)

The opener is master `chapter` with `when: first` layers and the art as a canvas at the head of the flow,
because a section takes one master and an image `src` can't come from a section variable. InDesign
and CSS paged media both give the first page its own master; the art would then be a field:

```json
{"type": "section", "master": "flow", "first_master": "opener", "vars": {"num": "1", "art": "book/build/art/ch01.png"}, …}
"opener": {"inherits": "flow", "margins": {"top": "4.3in"},
           "background": [{"type": "image", "src": "{{section.art}}", "x": "3.025in", "y": "1.36in", "w": "3.6in", "h": "2.4in", "fit": "contain"}]}
```

### 10. Hand-drawn marks

The struck paragraph's red lines are one quadratic stroke per line, nudged by a seeded generator so
no two lines are the same. `mark` draws straight lines:

```json
{"text": "…", "mark": {"kind": "strike", "stroke": {"width": 0.94, "color": "red", "cap": "round"},
                       "extent": 1.5, "offset": -2.6, "wobble": {"amount": 1.2, "seed": 7}}}
```

### 11. Margin notes never above the column

With `dy: -2.2` (handwriting sits high) a note beside a block at the column's top rises above it; the
book clamps it to the column top. The sample sets `dy: 0` there by hand. A `notes.clamp: true` (or
clamping by default) would do it.

### 12. Section-level style sets (tightening levels)

The book re-pours a flow at "t1"/"t2" (tighter leading and spacing) when that saves a near-empty last
page. The sample writes `body-t2`, `h2-t2`, `li-t2` and names them block by block:

```json
{"type": "section", "style_set": "t2", …}
"design": {"style_sets": {"t2": {"body": {"line_height": "13.6pt", "space_after": 4.5}, "h2": {"space_before": 9, "space_after": 4}}}}
```

## Small things worth documenting

- `rule_above.gap` is measured from the top of the rule (the bar's own width is inside the gap).
- Canvas, box and table blocks default to 0.8 em above and below; the first block of a frame drops its
  `space_before`.
- `skew` positive slants the tops of letters to the right after rotation is applied about the centre;
  `"skew": 13.97` with `"rotate": 13.87` reproduces CSS `matrix(0.972 0.24 -0.467 0.883 …)`. An explicit
  `"matrix": [a, b, c, d, e, f]` on groups would save the decomposition.
- A CSS-compatibility switch for stroke widths (snap to whole CSS pixels, minimum one) would remove the
  `bw()` helper in make.py: `"design": {"compat": {"snap_strokes": "css-px"}}`.
