# Engine gaps found rebuilding I Just Had It

Most valuable first. "Bug" means the model already promises the feature.

## 1. Bug: per-side borders draw a 1 pt black box

`{"top": …, "left": …}` borders parse as one empty stroke (1 pt black on every side); see the Idioms GAPS for the
cause and repro. Here it hit the torn tab (solid sides and top, a wall-coloured bottom edge), drawn instead as a
rect plus three lines, and the ruled fact rows, drawn as `rule_above` per cell plus a canvas line.

## 2. Bug: a frame's `valign` reaches into table cells

The facts and the also-list were first one bottom-anchored frame (`"valign": "bottom"`, as the CSS stacks them up
from the tabs). The cells then took bottom alignment too: "FALSE ALARMS" dropped to its row's second line and
"earplugs" to the bottom of a two-line row. The key slot on the answers page (`"valign": "middle"`) centred its
labels the same way. Both frames are now placed from computed tops. Repro:

```json
{"type": "text", "x": 10, "y": 10, "w": 90, "h": 100, "valign": "bottom", "blocks": [
  {"type": "table", "columns": [40, 50], "rows": [["sits at the bottom", "one two three four five six"]]}]}
```

## 3. Fill the height that is left (CSS `flex: 1`)

The notice is a flex column: the picture frame takes whatever height the poster type, facts, list and tabs leave.
Matenadar places frames by coordinates, so `make.py` computes the facts' and list's heights itself (easy only
because Courier is monospace) and gives the picture the rest. Wanted, inside a frame of fixed height:

```json
{"type": "text", "x": 70.6, "y": 67.7, "w": 349.8, "h": 525.5, "blocks": [
  {"type": "p", "style": "meta", "content": ["No. 1", {"leader": ""}, "Friday, 6:55 a.m.", {"leader": ""}, "The bed"]},
  {"type": "p", "style": "kicker", "text": "Missing"},
  {"type": "p", "style": "head", "text": "My phone"},
  {"type": "canvas", "h": "1fr", "fill": "#efe6ee", "stroke": {"width": 1.5, "color": "#2b2140"}, "clip": true,
   "elements": [{"type": "svg", "markup": "…", "x": "50%", "y": "50%", "anchor": "center", "w": 94.4, "h": 144}]},
  {"type": "table", "…": "facts"},
  {"type": "table", "…": "also in here"}]}
```

(Typst has fractional block heights, so `"h": "1fr"` on a canvas or box maps straight onto `block(height: 1fr)`.)
Percent positions inside a canvas would centre the drawing in whatever height it gets.

## 4. Boxes that hug their content, with uneven padding

The scene's tag ("1 Find the phone.") and foot ("I JUST HAD IT") are pills as wide as their text, padded 0.1 in
left, 0.26 in right and 0.085 in top and bottom. A text frame is always `w` wide, so the pills are `box` marks round
the run: one `inset` for all sides, the height set by shrinking the line height, the left room for the number disc
made of no-break spaces, and the vertical centring off by 0.64 pt. Wanted:

```json
{"type": "text", "x": 40.32, "y": 36, "w": "auto", "inset": {"top": 6.12, "right": 18.72, "bottom": 6.12, "left": 7.2},
 "fill": "#fcf6ee", "stroke": {"width": 1.5, "color": "#2b2140"}, "radius": 24.48, "style": "tag",
 "content": [{"inline": {"type": "ellipse", "w": 30.24, "h": 30.24, "fill": "#2b2140"}, "text": "1"}, {"space": "0.13in"}, "Find the phone."]}
```

and `"anchor": "bottom-right"` for the foot, which is set from its right edge.

## 5. Fit to width, growing to a maximum

The poster kicker and head are fitted by binary search between 8 pt and a maximum so one line fills the column.
`fit: "shrink"` only shrinks a frame whose text overflows its height; a single long word ("MISSING" at a wider
setting) overflows sideways and never triggers it, and nothing grows. Wanted:

```json
{"type": "text", "w": 349.8, "style": "kicker", "fit": "width", "max_size": 150, "min_size": 8, "text": "Missing"}
```

## 6. Viewbox and recolouring for placed SVGs

The answer pages show each scene from y = 9 pt (without its top bleed), and faded halfway to its paper.
`make.py` writes a copy of the scene with a new viewBox under `build/thumbs/`, and the fade is a paper-coloured
rect at 50 % over it (the same colours, but a transparency group in the PDF; the book itself kept to opaque fills
so nothing would rasterise). Wanted:

```json
{"type": "svg", "src": "book/build/scenes/phone.svg", "viewbox": [0, 9, 513, 738], "fit": "cover",
 "recolor": {"mix": "#fcf6ee", "amount": 0.5}}
```

## 7. Dash phase

The cat's dashed ring (`dash: [3, 9]`, round caps) starts its first dash at a different angle than the SVG
circle did, so dashes and gaps trade places. A `dash_offset` (or "start at 3 o'clock, clockwise" as SVG does)
would let rings match.

## 8. Table cells: vertical alignment, inset and row gap per cell

The answer key centres labels on 0.3 in icons, and the also-list has a 0.02 in row gap. Tables offer one inset
for every cell and no vertical alignment, so the labels carry a non-weak `space` block above them and the row gap
is every cell's bottom inset (the last row's spills under the table). Wanted:

```json
{"type": "table", "columns": ["0.4in", "1fr"], "row_gap": "0.11in", "valign": "middle", "rows": […]}
```

## 9. Lists with styled, hanging markers

The steps are a list whose numbers are 50 pt SILead in tomato, hung 0.78 in to the left and 0.1 in above the
step, each step at least 0.56 in tall. The `list` block's markers take the item's style, so the steps are a
two-column table with the numeral on a 35.6 pt line (which puts its baseline where Chrome's was) and the 0.26 in
gap as each cell's bottom inset. Wanted:

```json
{"type": "list", "ordered": true, "marker_style": "how-num", "marker_width": "0.78in", "marker_dy": "-0.1in",
 "item_min_h": "0.56in", "spacing": "0.26in", "items": ["The left page is the notice…", "…", "…"]}
```

## 10. Vertical metrics differ from Chrome by one CSS pixel for these fonts

Text set in the SI* Fraunces cuts, Courier Prime and League Gothic lands 0.72 pt lower than in Chrome's PDF on
every page, though placed from the same CSS box tops; shapes land exactly, and the Idioms fonts land exactly.
Probably a rounding of ascent or half-leading in Blink that Matenadar's CSS line model does not copy. Worth a
look before calling the line model "CSS-exact".
