# Engine gaps found rebuilding Idioms of the World

Most valuable first. "Bug" means the model already promises the feature.

## 1. Bug: `balance` does not compile

Any paragraph style with `"balance": true` fails the whole build:

```
error: layout failed: unknown variable: mt-balanced
```

`mt-par` in `crates/matenadar-typst/src/prelude.typ` calls `mt-balanced`, which is defined about 140 lines later;
Typst closures only see what is defined before them. Moving `mt-balanced` above `mt-par` should fix it. Minimal
repro:

```json
{"matenadar": "1", "meta": {"title": "t"},
 "design": {"styles": {"b": {"balance": true}}, "masters": {"m": {"width": "4in", "height": "4in"}}},
 "content": [{"type": "page", "id": "p", "master": "m", "elements": [
   {"type": "text", "x": 20, "y": 20, "w": 200, "style": "b", "text": "I have neither a she-camel in it nor a he-camel."}]}]}
```

The book balances every headline (plate literals, opener titles, "also" headings, sayings). `make.py` keeps
`balance` out of doc.json unless `BALANCE=1`; pages 47 and 111 are the ones whose breaks change.

## 2. Bug: per-side borders draw a 1 pt black box

`{"top": {...}, "bottom": {...}}` on a `box` block, a table's `stroke`, a text frame or a shape is read as one
stroke with no colour and no width, so every side gets a 1 pt black line. `Border` is `#[serde(untagged)]` with
`All(Stroke)` first, and `StrokeSpec` (all fields optional, unknown fields ignored) accepts the sides object.
Trying `Sides` first, or `#[serde(deny_unknown_fields)]` on `StrokeSpec`, would fix it.

```json
{"type": "box", "stroke": {"top": {"width": 0.75, "color": "#cfc8bd"}}, "blocks": [{"type": "p", "text": "rule on top only"}]}
```

Here the rows' hairlines are `rule_above` on each cell's first paragraph plus a canvas line under the last row.

## 3. Bug: a frame's `valign` reaches into table cells

A text frame with `"valign": "bottom"` (or `"middle"`) wraps its content in `align(bottom, …)`, and tables pass
`align: left`, so their cells inherit the vertical part: in a row whose cells differ in height, the shorter cell
sits at the bottom. Passing `left + top` (or the cell's own vertical alignment) would stop it.

```json
{"type": "text", "x": 10, "y": 10, "w": 90, "h": 100, "valign": "bottom", "blocks": [
  {"type": "table", "columns": [40, 50], "rows": [["sits at the bottom", "one two three four five six"]]}]}
```

Not hit on these pages (the bottom-anchored notes and tails hold no tables); the I Just Had It sample ran into it.

## 4. Text-wrap "pretty": no short last lines

The index entries, opener ledes and plate notes use CSS `text-wrap: pretty`. Matenadar breaks greedily, so two
index turnovers end on a single number where the book evens the lines ("devil 106, 112, / 127" for "devil 106, /
112, 127"). Wanted:

```json
"index-entry": {"size": 10.6, "line_height": 1.52, "hanging": "0.12in", "wrap": "pretty"}
```

(Typst's optimized line breaking, or a last-line minimum, would do.)

## 5. Tracking after the last letter

CSS letter-spacing follows every letter, the last included; Typst's tracking stops at a run's end. Tracked
labels that are followed by other text or aligned right land one tracking off. Here `make.py` adds a
`margin_right` of one tracking to right-aligned labels and widens the following space (`word_spacing`) after
"No. N", "says" and the index keys; without it the index's turnovers come out different. Wanted, as a style
switch (or as the default for CSS-like documents):

```json
"head": {"font": "label", "size": 7.3, "tracking": "0.17em", "tracking_trailing": true}
```

## 6. Inline spacing

`.no { margin-right: 0.6em }` and `sup { margin-left: 0.06in }` are spaces with a set width. Runs can only widen
a real space with `word_spacing`, which also depends on the font's space and its tracking, so the values were
found by measuring the render. Wanted:

```json
"content": [{"text": "No. 5", "span": "head-b"}, {"space": "0.6em"}, "How the world says ", …]
```

## 7. Superscript aligned to the line top

The chart's counts are `vertical-align: top` plus 0.04 in. `baseline` reproduces it, but the raise has to be
computed per heading size from the font's ascent and descent. Wanted:

```json
{"text": "10", "span": "count", "align_to": "line-top", "dy": "0.04in"}
```

## 8. Balanced columns drop block spacing

`{"type": "columns", "balance": true}` measures and stacks each block on its own, so `space_after` between
entries is lost. The index entries carry their 0.12 in gap as a bottom `inset` instead, which also leaves that gap
under each column's last entry (CSS drops it), so the next section head's `space_before` is reduced by hand.

## 9. Chrome's hairlines and borders

Not an engine gap so much as a convention to choose: CSS borders take space and lie inside the box (a 0.75 pt top
border pushes the content down 0.75 pt), while strokes in Matenadar are centred on the edge and take none. Every
ruled row here has a ±0.375 pt correction. A `stroke` placement option (`"align": "inside"`) on boxes, rules and
shapes would let CSS designs carry over without arithmetic.

## 10. Opacity on text

The opener folio (86 %) and cast list (90 %) are `#rrggbbaa` colours, which works. A `opacity` on a frame or
element would be closer to the source and cover non-text (the cast list's opacity applies to its whole block).
