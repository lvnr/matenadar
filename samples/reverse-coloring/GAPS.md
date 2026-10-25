# Engine gaps found rebuilding Reverse Coloring

Most valuable first.

## 1. Bug: `balance: true` fails to compile

Any paragraph style with `balance: true` stops the build:

```
error: layout failed: unknown variable: mt-balanced
```

In `prelude.typ`, `mt-par` calls `mt-balanced`, but `mt-balanced` is defined further down. Typst `let` bindings
only see earlier definitions. Minimal repro:

```json
{"matenadar": "1", "meta": {},
 "design": {"styles": {"t": {"size": 20, "balance": true}},
            "masters": {"m": {"width": "4in", "height": "3in"}}},
 "content": [{"type": "page", "id": "p", "master": "m", "elements": [
   {"type": "text", "x": 36, "y": 36, "w": 200, "style": "t", "text": "A balanced paragraph of several words here"}]}]}
```

Workaround here: make.py measures the balanced width itself and narrows the frame (`ENGINE_BALANCE = False`).

## 2. Place a frame by its first baseline

CSS flex rows aligned on the baseline (`.label`: a 50 pt numeral next to a 7.7 pt caps label) need the second
frame's `y` from the fonts' ascender and descender. make.py reads them with fontTools. A frame option to anchor on
the first baseline would make that one line of JSON:

```json
{"type": "text", "x": "1.2in", "y": "0.72in", "y_from": "baseline", "style": "part", "text": "An edge"}
```

A row primitive would also work: a `text` frame whose content is several boxes aligned on one baseline, with gaps
(`{"type": "row", "align": "baseline", "gap": "0.16in", "items": [...]}`).

## 3. `text-wrap: pretty`

Chrome's `pretty` keeps a lone short word off the last line ("know what / to add." rather than "know what to /
add."). There is no equivalent, so make.py binds the last two words with a no-break space. Wanted:

```json
"teach": {"font": "RCSans", "size": 11.4, "line_height": "16.6pt", "wrap": "pretty"}
```

(or `"last_line_min_words": 2`).

## 4. Inline horizontal space

CSS margins inside a line (`.stuck b{margin-right:.08in}`, `.stuck i{margin:0 .07in}`, and the flex `gap`) have
no inline equivalent. make.py emulates them with a space run whose `word_spacing` is the margin minus the font's
space width, so it needs the font file. Wanted:

```json
{"content": [{"text": "Stuck?", "span": "stuck-head"}, {"space": "0.08in"}, "Planets, with a ring around one"]}
```

or `margin_left` / `margin_right` on a run.

## 5. Rotation about a chosen origin

`writing-mode: vertical-rl` text works as a frame turned 90°, but `rotate` turns about the frame's centre. So the
position of a frame of unknown length has to be solved for (make.py uses a fixed 7 in frame and solves for its
centre). `"rotate_origin": "top-left"` would let `x`/`y` name the corner the reader sees first, or a
`"writing": "vertical"` frame would do the same.

## Not gaps

Not gaps, but worth knowing:

- Outside-only bleed, page fills that cover the bleed, exact-box stretched JPEGs, inline SVG markup, `valign`,
  rotated frames, per-run colour and tracking, `case: upper`, and named font families from a folder all worked as
  written.
- The remaining 0.6–0.7 pt vertical offset is Chrome's pixel snapping of line boxes, not an engine error.
