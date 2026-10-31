# I Just Had It, as a comic page

Friday morning from I Just Had It in five panels: the book's own scene SVGs (`build/scenes/<id>.svg`), each cropped
round the thing that was lost (its position from `build/scenes/<id>.json`), and the book's own words
(`manuscript/scenes.json`) in balloons and caption boxes. 7 x 10 in, the book's fonts (`design/fonts`).

```
python3 samples/comic/make.py
bin/matenadar build samples/comic/doc.json -o samples/comic/build/comic.pdf --png samples/comic/build/png
```

- `panels`: three rows of equal share, widths 1.25 : 1, 1, and 1 : 1, a 10 pt gutter, a 1.6 pt border; each
  panel's picture is cut to it.
- `balloon`: speech (phone, keys, shoe), thought (glasses), shout (keys), whisper (wallet), and captions for the
  times. The words wrap in lines of even length; each tail runs toward the speaker below the frame, and balloon and
  tail are one outline.

Looked at: the page at 80 and 110 dpi. Build: 1 page, layout about 200 ms (five large scene SVGs).
