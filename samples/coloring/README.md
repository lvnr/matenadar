# One Pen: Winter Lights, from the coloring template

`matenadar template coloring spec.json`: four of the book's activity pages (`coloring/pNN-*/pNN-*.activity.svg`),
each on a right-hand page with its back left blank but for the book's instruction line, after a typeset title page,
copyright page and "this book belongs to" page, and a page of squares to try the pen on at the end. The pictures are
whole 8.5 x 11 in pages with their own white edge, so they go in as `full` pages at the trim. Adobe Caslon Pro, as
the book.

```
python3 samples/coloring/make.py      # spec.json, then doc.json through the template
bin/matenadar build samples/coloring/doc.json -o samples/coloring/build/coloring.pdf --png samples/coloring/build/png
```

| page | shows |
|---|---|
| 1–3 | title, copyright, "this book belongs to" |
| 4 | blank, so the pictures open on a right-hand page |
| 5, 7, 9, 11 | the pictures |
| 6, 8, 10, 12 | their backs: blank, with the note small at the foot |
| 13–14 | the test page and its back |

Looked at: all fourteen pages at 50 dpi.
