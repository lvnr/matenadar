# How Dinosaurs Worked, as a 32-page picture book

The picture-book template (`matenadar template picture-book spec.json`) composing a 32-page,
separately ended, 8.5 x 11 in book from Whynarium: How Dinosaurs Worked. The story pages are the book's
own lettered, full-bleed pages; the title, copyright and back matter are typeset by the template from the
book's own words.

```
python3 make.py && python3 ../../scripts/review.py . --dpi 100
```

`make.py` writes `spec.json` (the template's input), draws `art/dig-site.svg` and runs the template into
`doc.json`. The spec is the interesting file: everything below comes from it.

## The plan

| pages | what | how |
|---|---|---|
| 1 | title page | typeset: title, subtitle, author, publisher, in the safe area |
| 2 | copyright | typeset from `release/copyright-page.md`, at the foot of the verso |
| 3 | "Welcome to the Whynarium!" | the book's page 3, to the bleed (recto: bleed on the right) |
| 4–5 | the dig site | a spread: one vector picture across both pages and their outer bleeds, with a 0.125 in gutter repeated at the fold; words typeset over the art on request (`text` on the verso, a placed frame on the recto), kept in the safe zone on feathered panels |
| 6–30 | the story | the book's pages 6–30, each on the side it had in the book |
| 31–32 | back matter | typeset: "About this book" from the description, "For grown-ups" from the copyright copy |

Pages keep the book's sides because a page's bleed is on its outside edge: a recto's art bleeds right, a
verso's left. The template places every story picture from the bleed on the page's outside.

## What was checked

- `matenadar check`: no issues on 32 pages (live text inside the safe zone; every picture at 300 ppi or more).
- Every page looked at (40 dpi contact rows, 80 dpi for the front pages and the spread).
- Compare sheets (`build/compare-*.png`): pages 3 and 12 show the book's same pictures in the same place
  (RMSE 4% at 40 dpi, from resampling; the book's live folio is gone, as picture books have none); page 1
  is the typeset title where the book had its cover art; pages 4–5 replace the book's two contents pages.
- The fixed-layout EPUB of the same document: see the WebAssembly part (`matenadar epub --layout fixed`).

## Notes

- Cooper Hewitt's 2014 files number their weights 701–714 (Thin to Heavy Italic), so asking for regular or
  bold picks Thin. The spec's `design` asks for the faces by those numbers (705 Book, 707 Medium, 711 Bold).
- The spread's picture is drawn here because none of the book's pictures is a two-page one; it shows the
  element, not the book's art.
