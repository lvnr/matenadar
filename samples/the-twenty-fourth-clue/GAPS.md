# The Twenty-Fourth Clue — engine gaps

Most valuable first. Each has what the book does, what the sample does instead, and the JSON I'd want.

## 1. Ragged text that breaks like `text-wrap: pretty`

The story is ragged right with `text-wrap: pretty`. Chrome moved "All" down a line so the paragraph would
not end on "SOMEONE." alone; Matenadar's ragged breaking is greedy, so one paragraph on p10 breaks
differently (the only visible difference on that page). Optimal breaking for ragged text would fix it.

```json
"story": {"align": "left", "hyphenate": true, "wrap": "pretty"}
```

## 2. Text on a path

The postmark's ALDERWICK POST OFFICE / ROUTE 24 run along arcs. The sample keeps them as SVG `<textPath>`;
the type is then the SVG renderer's, not Matenadar's (no features, no checks, no live text options).

```json
{"type": "lettering", "text": "ALDERWICK POST OFFICE", "font": "League Gothic", "size": 12.6, "tracking": 1.9,
 "path": "M 18.8 62 A 43.2 43.2 0 0 1 105.2 62", "start": "50%", "align": "center"}
```

## 3. OpenType features for SVG text

The book's inline SVG text inherited the body's `font-variant-numeric: oldstyle-nums`, so the postmark's
"1" is an old-style figure. The SVG renderer ignores `font-feature-settings` and `font-variant-numeric`
(tried attribute and style forms), so the number came out lining and twice as tall. The sample now sets the
number as a Matenadar text frame over the drawing, turned with it.

```json
{"type": "svg", "src": "postmark.svg", "features": ["onum"]}
```

## 4. Rotation about a chosen origin

Rotated paper (the card, the stamp, the postmark, the note) is turned about its centre in the book, which
matches Matenadar. But several layers nest: the postmark turns 9° about its own box inside a card turned
−1.4°, and its SVG turns −6° about the stamp's centre (62, 62). The sample needs a 124 × 124 group just to
get that pivot. A pivot would make it one element.

```json
{"type": "group", "rotate": -6, "origin": [62, 62], "children": [ … ]}
```

## 5. Shadows with spread / outline-only shadows

The parcel card's shadow is `box-shadow: 4pt 4pt 0 -0.7pt #fff, 4pt 4pt 0 0.7pt #000`: a 1.4 pt black
ring around an offset copy, not a filled one. `shadow` only fills. The sample draws a stroked rect under the
card.

```json
{"type": "rect", "shadow": {"dx": 4, "dy": 4, "stroke": {"width": 1.4, "color": "#000"}, "fill": "#fff"}}
```

## 6. CSS tracking semantics (trailing letter-spacing)

CSS letter-spacing also follows the last letter, so every tracked label's box is one tracking wider than its
ink. Right-aligned tracked text (the verso foot) and flex gaps after tracked labels (form rows) land
0.7–2 pt differently. The sample shrinks frames and moves rules by hand.

```json
"base": {"tracking_mode": "css"}
```

## 7. Optical size in CSS pixels

Chrome's `font-optical-sizing: auto` sets `opsz` to the size in CSS px (12.4 pt → 16.53). Matenadar drives
`opsz` from the size in points. With `variations.opsz` set by hand on every Literata style the type matches
exactly; a document switch would save that.

```json
"design": {"optical_size": "css-px"}
```

## 8. A flexible rule inside a line

The running foot is folio · rule (flex 1) · run text. The sample uses a three-column table with a canvas line
at `"100%"` in the `1fr` cell, plus hand-tuned offsets for the baseline. An inline leader that draws a rule
would be the natural way to say it.

```json
{"content": [{"field": "page", "style": "folio"}, {"leader": "rule", "stroke": {"width": 0.75}, "lift": 3, "gap": 10},
             {"field": "section.run", "style": "run"}]}
```

## 9. Recto and verso of an excerpt

`when: recto/verso` and inside/outside margins follow the physical page index, not the page number, and a
`page` does not take `start` (it is accepted and ignored). An excerpt of a book needs empty pages to keep
each page on its side (this sample has four).

```json
{"type": "page", "side": "verso", "page_from": 10}
```

## 10. Non-rectangular clipping

The soap's key hollow is hatching clipped by the key outline with an even-odd hole; the map is clipped to its
inner frame. Group `clip` is a rectangle only, so both stay in the book's SVG.

```json
{"type": "group", "clip": {"d": "M … Z M … Z", "rule": "evenodd"}, "children": [ … ]}
```

## Bugs

- **`lettering` ignores `weight` on variable fonts.** Besley 800 lettering draws Besley Regular; a text frame
  with the same settings is right.
  `{"type": "lettering", "x": 100, "y": 400, "text": "The Chute", "font": "Besley", "weight": 800, "size": 28}`
- **A percentage size is rejected.** `{"text": "A", "size": "104%"}` → `layout failed: expected length, found
  ratio`. `"1.04em"` works. The book's lead-in is `font-size: 104%`.
- **Fixed in this round, used here:** `balance: true` crashed the build (`unknown variable: mt-balanced`); the
  day title now uses it.
