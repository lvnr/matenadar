# I Just Had It — four pages in Matenadar

The book (`~/projects/amazon-kdp/projects/i-just-had-it`) was set as one HTML page at a time and printed by Chrome
(`design/build/book.py` + `design/book.css`); its scenes are SVGs composed by code (`design/build/scene.py`) from
960 traced drawings. This sample rebuilds four pages from the book's sources: the copy from
`manuscript/scenes.json` and `manuscript/matter.json`, palettes and walls from `planning/layouts.json`, the
finished scene SVGs and their answer positions from `build/scenes/`, the traced drawings from
`visuals/sprites/*.json` (re-inked in `make.py` exactly as `book.py` does), and the fonts in `design/fonts`
(the SI* files are already static Fraunces cuts, so no instancing was needed).

```
python3 samples/i-just-had-it/make.py          # doc.json, plus build/tick.svg and build/thumbs/*.svg
MAGICK_CONFIGURE_PATH=$PWD/samples/i-just-had-it/build/magick python3 scripts/review.py samples/i-just-had-it
```

`build/magick/type.xml` only gives ImageMagick a default font for the compare sheets' labels (see the Idioms
README).

## Pages

| original | rebuilt | page | fidelity | what still differs |
|---|---|---|---|---|
| 5 | 1 | how this works: SILead head, numbered steps with hanging tomato numerals, legend of drawings in a ruled 1 : 1.5 : 1 frame, bottom tally | 9.5/10 | All text sits 0.72 pt (one CSS px) lower than Chrome's; the legend and drawings are exact. Same line breaks. |
| 6 | 2 | notice No. 1: wall, pinned card, meta row spread by leaders, fitted poster type, framed and clipped phone drawing, ruled Courier facts, tick-box list, nine rotated tear-off tabs, one torn out | 9/10 | Pixel-level match (1.6 % of pixels differ, nearly all antialiasing). The picture's height is worked out in `make.py` (CSS gave it the leftover height: GAPS 3), so a different notice with other line counts depends on that arithmetic. |
| 7 | 3 | scene No. 1: the book's 1.1 MB scene SVG full bleed, pill tag with number disc, pill foot | 9.5/10 | The pill tag is 0.64 pt higher than Chrome's (its padding is uneven; see GAPS 4). The scene itself is the book's file. |
| 66 | 4 | answers 1: centred key slot with ring icons, three scenes at 40.6 % cropped to their frames, faded to their paper, ringed | 9/10 | Faded colours match to within 1/255. The dashed cat ring starts its dashes at a different point (no dash phase), and the Courier text sits 0.72 pt low as on p. 5. |

Build: 4 pages, layout about 75 ms, about 1.5 s in all; most of it is the four large scene SVGs.

## How it is built

- One master, `sheet` (7 × 10 in, 0.125 in bleed on top, bottom and outside); no running heads or folios in this
  book. Page fills are the wall, the scene ground or the paper.
- **Notice.** The card is a rect with the 1.5 pt border inset by half its width (CSS borders lie inside the box);
  the pin and its highlight are ellipses. The meta row is one paragraph with two `{"leader": ""}` (they share the
  space as `justify-content: space-between` does) and a `rule_below`. Poster kicker and head are one-line frames
  with `fit: shrink` at their largest size (they hit their maxima here). Facts and the also-list are tables in
  one frame; tick boxes are an inline SVG image with a hanging indent. The tabs are nine rotated text frames
  (`rotate: -90`, reading bottom to top) between dashed lines; the torn tab is a wall-coloured rect over the
  card's edge with three 1.5 pt lines.
- **Picture.** The lost thing is the traced drawing re-inked in the scene's palette and the instance's ink order,
  centred in a clipping `group` filled with the ground; at 2 in tall it is clipped top and bottom as in the book.
- **Scene.** `svg` with `src: book/build/scenes/phone.svg` over the whole sheet, `decorative`. The tag and the foot
  are pills drawn by a `box` mark round the run (`fill`, `stroke`, `radius`, `inset`): the inset is the pill's
  right padding, a 5 pt line height makes the pill the right height, and no-break spaces open the room for the
  number disc, which is an ellipse with its numeral set over it.
- **Answers.** Each thumbnail is a clipping group: the scene SVG with the answer pages' viewBox (`0 9 513 738`,
  written to `build/thumbs/` because an `svg` element cannot take its own viewBox), set with `fit: cover` (the
  book's `xMidYMid slice`); the scene's paper at 50 % over it, which mixes exactly as `scene.py`'s `fade=0.5`; and
  the rings as ellipses in the unfaded inks, scaled from `build/scenes/<id>.json` with `scene.py`'s ring rule.
  The key slot's frame is placed at its centred position from monospace arithmetic, not `valign` (GAPS 2).

## The 0.72 pt

Text in this book's fonts (Fraunces cuts, Courier Prime, League Gothic) lands 0.72 pt (one CSS px) lower in the
rebuild than in Chrome's PDF, everywhere, while shapes and SVGs land exactly. The Idioms sample, set the same way,
has no such offset. It is too small for a reader to see; shifting every frame up by a pixel would hide it but not
explain it, so it is left as found.
