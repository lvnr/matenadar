# The Attention Repair Manual: numbered figures, a list of figures and a full-bleed plate

Chapters 1 and 2 of The Attention Repair Manual (6 x 9 in, black ink), set by Matenadar from the book's
own Markdown manuscript and drawings, to show figures as a nonfiction book needs them.

```
python3 make.py && python3 ../../scripts/review.py . --dpi 100
```

## What it shows

- **Numbered figures.** `design.figures` with the label `Fig.`, counted through the book. The book's
  drawings had their titles lettered in ("Fig. 1. The page you were protecting"); each is cropped just
  below that band (`crop`, the line measured by eye per drawing in `make.py`) and the engine sets the
  number and title as the caption instead, so the numbers stay right when figures move or are added.
- **A list of figures** (`lof`) after the contents, with leaders and page numbers.
- **A cross-reference**: "(Fig. 2, page 10)" in the paragraph that introduces the six suspects, as
  `{"ref": "fig-2", "show": "label"}` and `{"ref": "fig-2"}`. The book has no cross-references; this one is
  added to show how they read.
- **A full-bleed plate**: the Week 1 drawing opens chapter 2 on a page of its own, to the bleed, its folio
  hidden. The text runs on around it: the chapter's first page, then the plate, then the rest.
- **Black ink**: `print.ink` is `black` and `print.filter` greys every raster picture (`gray`, contrast 2);
  `matenadar check` reports no issues on 47 pages, and the PDF's pictures are one-channel grey.

Libertinus Serif (the engine's own) sets the text and Source Sans Pro (SIL OFL) the headings and captions;
the book itself used Warnock Pro and Halyard.
