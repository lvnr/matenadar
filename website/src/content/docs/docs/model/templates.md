---
title: Templates
description: Whole documents for a book kind from a short spec. The picture-book, a-plus and coloring templates, and how their ids keep edits in place.
---

A template makes a book kind’s whole document from a short description, a **spec**. The document is then edited like any other.

```console
$ matenadar template picture-book spec.json -o doc.json
picture-book → doc.json (21 items)
```

Without `-o` the document is printed to stdout. In WebAssembly it is `template(kind, spec)`. The kinds are `picture-book`, `a-plus` and `coloring`.

**Ids are kept.** Items keep the ids their spec gives them (`id`; otherwise their place, such as `story-03`), so edits kept as [operations](/docs/guides/operations/) land where they were made when the spec changes and the document is composed again.

**Every spec takes `design`:** changes to the template’s design, merged into it object by object:

```json
{"design": {"styles": {"title": {"size": 48}}}}
```

## `picture-book`

Story pages are pictures to the bleed with their words lettered in the art (the default); front and back matter is typeset.

```json
{"trim": "8.5x11", "pages": 32, "ends": "separate", "title": "…", "subtitle": "…",
 "author": "…", "illustrator": "…", "publisher": "…", "copyright": "…", "dedication": "…",
 "fonts": {"display": "…", "text": "…"}, "font_dirs": ["fonts"],
 "story": [
   {"src": "art/p03.jpg", "alt": "…"},
   {"spread": true, "src": "art/p04-05.jpg", "gutter": "0.125in",
    "left": {"text": "Words set over the art, on request."}},
   {"spread": true, "left": {"src": "art/p06.jpg"}, "right": {"src": "art/p07.jpg"}}
 ],
 "back": [{"heading": "About the author", "text": "…"}]}
```

- **`trim`**: `8.5x8.5`, `8x10` or `8.5x11` (inches), with a 0.125 in bleed (`bleed` changes it).
- **`pages`**: 24, 32 or 40.
- **`ends: "separate"`** (default; the endpapers are their own sheets): page 1 is the title page (or with `half_title`, the half title, and the title page on page 3), its back the copyright and dedication, and the story runs from there.
- **`ends: "self"`** (the first and last leaves are the endpapers): pages 1 and N are pasted to the boards, 2–3 and N−2–N−1 are the endpapers (`endpapers`: `{"src"}`, or `front` and `back`), 4 the copyright and dedication, 5 the title page, and the story runs from 6 to N−3.
- **Story items.** A story item is a page (`src`, with `crop`, `focus` and `alt`) or a spread (`spread: true`, as the [spread](/docs/model/content/#spreads) item). `text` sets words over the art in a frame at the top of the page, kept in the safe zone on a soft panel; `words` places any elements over it.
- **The plan must fit.** A spread must open on a left-hand (even) page, and the story and back matter must fit: the template says which item breaks the plan rather than leaving a page blank in the middle of a story. Pages left over at the end are blank.
- **`title_art`** puts a picture behind the title page, with the title on soft panels over it.

## `a-plus`

A book’s A+ modules as the store shows them, for a proof before upload: one page as wide as the store’s module area that grows with the modules (`"grow": true`), each module’s pictures in their upload slots, cropped as the store crops them.

```json
{"width": "desktop", "title": "…", "marketplace": "Amazon.com",
 "modules": [
   {"layout": "image-header-with-text", "label": "…", "heading": "…", "body": "…",
    "images": [{"src": "a-plus/01.jpg", "alt": "…"}]},
   {"layout": "comparison-chart", "comparisons": [{"title": "…", "asin": "…", "src": "…"}],
    "rows": [{"label": "Pages", "value": "32"}]}
 ]}
```

- **`width`**: `desktop` (970 px, the default), `mobile` (480 px), or pixels. Render at 96 dpi for one pixel per CSS pixel: `matenadar build doc.json --png proof --dpi 96`.
- **Up to seven modules.** `heading: false` leaves out the proof’s own heading; `font` sets the text’s family (the store’s sans by default).
- A picture’s `w` and `h` give its upload’s own size where the store has changed the slot, and `focus` where the crop keeps it.

Slots, in pixels:

| Layout | Slot |
|---|---|
| `image-header-with-text` | 970 × 600 |
| `image-text-overlay` | 970 × 300 (its words on a panel over the picture; under it on a phone) |
| `single-left-image`, `single-right-image`, `single-image-highlights`, `three-images-text`, `multiple-images` | 300 × 300 |
| `single-image-sidebar` | 300 × 400 |
| `four-images-text` | 220 × 220 |
| `comparison-chart` | 150 × 300 |
| `company-logo` | 600 × 180 |
| any other layout | 970 × 300 |

## `coloring`

Each picture has a right-hand page to itself and its back is left blank, so markers that bleed through spoil nothing; the front matter is typeset.

```json
{"trim": "8.5x11", "title": "…", "subtitle": "…", "author": "…", "copyright": "…",
 "belongs_to": true, "back_note": "…", "test_page": "Try your pen here",
 "pages": [{"src": "art/p01.svg", "title": "The Lamplighter", "alt": "…"},
           {"src": "art/p02.svg", "full": true}]}
```

- **`trim`**: `8.5x11` (default), `8x10`, `8.5x8.5`, `7x10` or `6x9`; `margin` (default 0.5 in) and `inside` (0.75 in) keep pictures clear of the edges.
- **The plan:** the title page, the copyright page, “this book belongs to” (`belongs_to`: true, false or its words), a blank page if needed so the pictures open on a right-hand page, then each picture and its back.
- **Pictures** fit inside the margins, under their `title` if they have one; `full: true` gives a picture the whole page: to the bleed with `bleed: true`, to the trim otherwise (art drawn with its own white edge).
- **`single_sided: false`** runs pictures on both sides; `back_note` sets small words at the foot of each blank back; `folios` numbers the pictures’ pages; `test_page` (true or its heading) closes the book with squares to try colours on.
