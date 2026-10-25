# Third Shift Handbook — engine gaps

Most valuable first. Each has what the book does, what the sample does instead, and the JSON I'd want.

## 1. Per-glyph transforms on runs (the handwriting)

Every handwritten letter in the book is turned (±2.4°), nudged and scaled; every word is turned (±1.5°) and
drifts. Runs can carry `baseline` and `size`, so the sample keeps the nudges, the drift and the size wobble,
and loses both turns. A visual `scale` that leaves the advance alone (as a CSS transform does) would also
remove the advance bookkeeping in `make.py`.

```json
{"text": "a", "rotate": -1.7, "scale": 1.03, "baseline": 0.2}
```

## 2. Runs that do not open the line

The book sets substituted letters (Ruth's 1.7 em full stop, her Kalam `k`) with `line-height: 0`, so they
never change the line's height. In Matenadar any run larger than the paragraph's size opens its line
(a 21 pt full stop in 12.4 pt text pushed the next line down 7–10 pt). The sample sets the paragraph at the
largest letter size and fakes the large full stop with `outline`.

```json
{"text": ".", "size": "1.7em", "line_height": 0}
```

## 3. Tracking at run boundaries

Tracking is applied between letters inside a run but not after a run's last letter, so one-letter runs
(the handwriting) get none at all, and CSS-style trailing tracking is lost everywhere. Walt (+0.02 em) and
Coyle (−0.01 em) lines came out 3–6 pt off, enough to change line breaks. The sample puts each word's
missing tracking into the space before it.

Repro: paragraph `"set": {"font": "Bad Script", "size": 13, "tracking": "-1pt"}`
- `"content": ["caution."]` → 27.98 pt wide (tracked)
- `"content": [{"text": "c", "size": 13}, {"text": "a", "size": 13}, … ]` → 34.98 pt (untracked)
- `[{"text": "cau", "baseline": 0.1}, {"text": "tion.", "baseline": 0.2}]` → 28.98 (one boundary lost)

## 4. Unmirrored page plans and a right-hand note column on both sides

The handbook prints the same plan on both sides: column at the left, handwriting margin at the right, the
whole sheet moved 0.05 in toward the spine. Matenadar masters mirror `inside`/`outside`, and the note column's
`side` is `outside`, so a section that flowed onto a verso would put the column and the notes on the wrong
sides. The sample only flows text on a recto and places the verso page by hand.

```json
"rules": {"mirror": false, "margins": {"left": "0.62in", "right": "1.98in", "top": "0.72in", "bottom": "0.82in"},
          "spine_shift": "0.05in", "notes": {"side": "right", "width": "1.46in", "gap": "0.14in"}}
```

## 5. Pen marks drawn like pens

`mark` underline/strike are straight rules. The book's marks are seeded polylines with a vertex every 14 px
jittered in x and y (`layout.js`), a strike is sometimes doubled, and a struck note is gone through twice
per row plus a slash across the lot. The sample uses straight marks, a second invisible copy of the note for
the second pass, and a path for the slash.

```json
{"mark": {"kind": "strike", "stroke": {"width": 1.58, "color": "pencil-dk"}, "wobble": 0.9, "seed": 7, "passes": 2}}
{"type": "text", "struck": {"stroke": {…}, "passes": 2, "slash": true, "seed": 7}, …}
```

## 6. Justification without tightening

Chrome's justification only stretches spaces; Matenadar's also tightens them, so two Schola paragraphs on
p7 fit a word more per line than the book ("No man on the / shift", "before the eleven / o'clock").

```json
"body": {"align": "justify", "word_spacing": {"min": "100%"}}
```

## 7. Spacing between non-paragraph blocks

Canvas, table and box blocks get a fixed 8 pt gap above and below that the model cannot set (a base
`space_after: 0` does not change it); only a `space` block replaces it. A box's `space_before` is ignored
when it has `rotate`. The form on p75 needs a `space` block between every row.

Repro (text frame, base 10/12): `[p, table, table, canvas, canvas, p]` → rows 20 pt apart instead of 12.
Rotated: `[p, {"type": "box", "space_before": 17, "rotate": -0.9, …}, p]` → the box ignores the 17 pt and
the next paragraph overlaps it.

```json
{"type": "canvas", "h": 10.6, "space_before": 0, "space_after": 2.2, …}
```

## 8. Boxes that hang outside the measure

The typed slip has `margin: 17pt -3pt 12pt -3pt`: 3 pt wider than the column on both sides. A box can be
wider than the column but only grows to the right, so the slip sits 3 pt right of the book's.

```json
{"type": "box", "margin_left": -3, "margin_right": -3, …}
```

## 9. Rotation about the top-left corner

Notes turn about their top-left corner (`transform-origin: 0 0`). Matenadar turns about the centre, so the
sample computes a corrected position from an estimated height. A pivot would remove the estimate.

```json
{"type": "text", "rotate": -1.95, "origin": "top-left", …}
```

## 10. Margin notes against a box

A `margin` note after a box meets the box's first line (inside its inset) instead of its top edge. The note
answering the slip needed `dy` −8.75 pt.

## Bugs

- **A box's `align` leaks into its text.** `{"type": "box", "w": 250, "align": "center", "blocks": [{"type":
  "p", "set": {"align": "left"}, "text": "…"}]}` centres the paragraph's lines too.
- **Table cells given as `blocks` widen `auto` columns.** `"columns": ["auto", "1fr", "auto", 68.8]` with a
  cell `{"blocks": [{"type": "space", "h": 8.4}, {"type": "p", "text": "THIS COPY IS ISSUED TO"}]}`: both auto
  columns take the room and the `1fr` column collapses to 0. The same cell as `{"text": …}` is right. Cells
  also ignore `set` (only `style` works).
