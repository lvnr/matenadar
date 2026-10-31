# Black ink: a colour picture reported, then filtered

Two pages of a black-ink interior (`"print": {"ink": "black"}`) showing one of The Attention Repair
Manual's colour cover designs, as an "also in print" page might.

```
python3 make.py && ../../bin/matenadar check doc.json
```

Page 1 has the picture as delivered and its heading in a blue. The check reports both:

```
  error color-in-black-ink [p1 delivered-cover]: a colour picture (92% of it clearly in colour) on a black-ink page; give pictures a grey filter ("filter": {"gray": true}) and set type and shapes in greys
  error color-in-black-ink [p1 delivered-heading]: text in #1f4fa0 on a black-ink page; give pictures a grey filter ("filter": {"gray": true}) and set type and shapes in greys
```

Page 2 has the same picture with `"filter": {"gray": true, "contrast": 3, "black": 0.02, "white": 0.98}`
(Rec. 709 luma, levels and an S-curve that keeps black and white) and its heading in the ink: nothing to
report. The yellow ground prints as a light grey and the blue panel as a dark one, with its white words
still clear.
