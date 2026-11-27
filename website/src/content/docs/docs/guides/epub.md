---
title: EPUB 3
description: Make an EPUB 3 from the same model as the print book. Reflowable text with the design as CSS, or fixed layout with live, selectable words over every page.
---

`matenadar epub` makes an EPUB 3 from the same model as the print book. The output is checked with epubcheck in **matenadar**’s tests.

```console
$ matenadar epub book.json -o book.epub
book.epub (1.4 MB, 410 ms)
```

Without `-o` it writes next to the document, with the extension `.epub`.

## Reflowable (the default)

For books that read as running text.

- One XHTML file per section.
- The design’s styles become CSS in `rem` and `em`, so readers can still change the size.
- Headings become the contents (the EPUB navigation).
- Footnotes become EPUB notes.
- Hand-laid pages and canvases become pictures, drawn by the engine at `--dpi` (default 200 on the command line).
- `--embed-fonts` puts the fonts the book uses in it: only the faces it needs, and only those licensed for embedding.

## Fixed layout

```console
$ matenadar epub book.json -o book.epub --layout fixed
```

For page-led books (comics, puzzles, picture books): every page as laid out for print.

- Each page is a picture drawn **without its typeset words**, and the words are set over it as **live SVG text** in the book’s own faces (embedded), word by word where the engine set them. Readers can select, search and read them aloud.
- Lettering and other type drawn as outlines stays in the picture, as do runs in faces that can’t go in a book (a collection, a variable instance, or a face whose licence forbids embedding).
- `--no-live-text` makes every page one picture, its text kept as the picture’s alt text.

## Options

| Option | Meaning | Default |
|---|---|---|
| `-o FILE` | where to write | the document’s path with `.epub` |
| `--layout reflowable` or `fixed` | the kind of EPUB | `reflowable` |
| `--dpi N` | resolution of page and drawing pictures | `200` |
| `--embed-fonts` | reflowable: put the book’s faces in it (licensed ones only) | off |
| `--no-live-text` | fixed: leave the words in the pictures | off |
| `--cover FILE` | the cover image | the first hand-laid page, if the book opens with one |
| `--identifier ID` | the book’s identifier, such as `urn:isbn:9780000000000` | derived from the title and author |
| `--json` | print `{"epub", "bytes", "total_ms"}` | — |

The common options apply too: `--root`, `--fonts`, `--no-system-fonts`.

## Metadata

The EPUB takes its title, author and language from `meta`. Give pictures an `alt`: reflowable books carry it with each picture, and fixed-layout pages without live text keep their words there.

## In WebAssembly

```js
const bytes = mt.epub({ layout: 'fixed', dpi: 150, liveText: true, identifier: 'urn:isbn:…' });
```

`epub({layout, dpi, embedFonts, liveText, identifier, cover, modified})` gives the EPUB as bytes, as `matenadar epub` makes it. `cover` names a file given with `addFile`. `modified` is when the book was made (`CCYY-MM-DDThh:mm:ssZ`) and defaults to now, which the module can’t read itself, so the client passes it. In WebAssembly `dpi` defaults to 150.
