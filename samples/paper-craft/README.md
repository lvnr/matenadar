# I Just Had It, as a seek-and-find cube

A sheet to cut, fold and glue into a cube whose six faces are I Just Had It's scenes (`build/scenes/<id>.svg`), each
cut square round the thing that was lost; what to find is the book's own headings. One `craft` element: the faces
(pictures cut to their squares), the cut lines, the five valley folds between faces, seven glue tabs (one for each
pair of edges that close up) and scissors on the first cut line. 7 x 10 in, the book's fonts.

```
python3 samples/paper-craft/make.py
bin/matenadar build samples/paper-craft/doc.json -o samples/paper-craft/build/cube.pdf --png samples/paper-craft/build/png
```

Looked at: the sheet at 80 dpi; the net folds to a cube on paper (the tabs pair the edges as `make.py` lists them).
