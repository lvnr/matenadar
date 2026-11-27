---
title: The document model
description: The shape of a matenadar document (version 1), the asset root, print settings, and the units, colours, coordinates, fills and strokes every field uses.
sidebar:
  label: Overview
---

A **matenadar** document is JSON. It is the only source of truth: the engine generates its layout program from it on every build and never stores that program. This section of the docs distils [`docs/model.md`](https://github.com/lvnr/matenadar/blob/main/docs/model.md), the field-by-field reference in the repository.

## The document

```json
{
  "matenadar": "1",
  "meta": {"title": "…", "author": "…", "lang": "en-US"},
  "fonts": ["design/fonts"],
  "design": {
    "colors": {}, "fonts": {}, "base": {}, "styles": {}, "spans": {},
    "masters": {}, "components": {}, "hyphenation": {}, "figures": {}
  },
  "content": [
    {"type": "section", "…": "…"},
    {"type": "page", "…": "…"},
    {"type": "spread", "…": "…"}
  ],
  "on_page": {},
  "print": {"ink": "black", "max_ppi": 300},
  "pdf": {"standard": "a-3b", "tagged": true, "attachments": []}
}
```

| Field | What it is |
|---|---|
| `matenadar` | the model version. This engine reads `"1"` and refuses anything else. |
| `meta` | `title`, `author`, `subject`, `keywords` and `lang` (BCP 47). Fields such as `{"field": "doc.title"}` read them, and PDF/UA and EPUB use them. |
| `fonts` | font folders, relative to the asset root, searched before the system’s fonts |
| `design` | colours, font stacks, text styles, masters, components, hyphenation exceptions and figure numbering: [the design](/docs/model/design/) |
| `content` | the document, in order: flowing sections, hand-laid pages and two-page spreads: [content](/docs/model/content/) |
| `on_page` | changes to the page where a node lands, keyed by the node’s id: [on the page of a node](/docs/model/content/#on-the-page-of-a-node) |
| `print` | how the document is printed, which the checks and exporter hold it to |
| `pdf` | PDF compliance (`standard`, `standards`, `tagged`) and embedded `/AF` file attachments |

### The asset root

Image, SVG and font paths are relative to the **asset root**. On the command line it defaults to the document’s folder; `--root DIR` sets it. Nothing outside it can be read. In WebAssembly, `openFrom(doc, {root})` reads under a folder (Node) or a base URL (browsers).

### Print settings

- `ink`: `"black"` for a black-ink interior, so anything drawn in colour is reported as `color-in-black-ink`, or `"color"`.
- `filter`: a filter for every raster picture that sets none of its own, such as `{"gray": true, "contrast": 3}`. A picture opts out with `"filter": {}`. See [pictures](/docs/model/content/#image).
- `max_ppi`: optional target maximum PPI for deterministic Lanczos-3 downsampling of oversized raster images during PDF export (SVGs and images already at or below `max_ppi` are left untouched).

### PDF compliance and attachments

- `standard`: single PDF standard shorthand (`"1.4"`–`"2.0"`, `"a-1b"`, `"a-1a"`, `"a-2b"`, `"a-2u"`, `"a-2a"`, `"a-3b"`, `"a-3u"`, `"a-3a"`, `"a-4"`, `"a-4f"`, `"a-4e"`, `"ua-1"`).
- `standards`: list of compatible standards (e.g. `["a-3b", "ua-1"]`).
- `tagged`: emit tagged PDF structure (default `true`).
- `attachments`: embedded files (`/EmbeddedFiles`, `/AF` for PDF/A-3 / Factur-X): `{"name", "src", "mime", "description", "relationship"}`, where `relationship` is `"data"`, `"source"`, `"alternative"`, `"supplement"` or `"unspecified"`.

### Ids

Any node may have an `id`. It must start with a letter and use only letters, digits, `-`, `_`, `.` and `:`. Ids are how the layout report, `locate`, checks and operations name things, so give one to anything you will want to find or change.

## Units

A number is **points** (1/72 in). A string can carry a unit:

| Unit | Example | Meaning |
|---|---|---|
| `pt` | `"12pt"` | points |
| `in` | `"0.5in"` | inches |
| `mm`, `cm` | `"3mm"`, `"1.2cm"` | millimetres, centimetres |
| `px` | `"16px"` | CSS pixels, 0.75 pt |
| `tpt` | `"10tpt"` | TeX points, 1/72.27 in |
| `em` | `"1.5em"` | relative to the text size |
| `%` | `"50%"` | of the containing box |

## Colours

- `"#rgb"`, `"#rrggbb"`, or `"#rrggbbaa"` (with alpha)
- a name from `design.colors`
- `"cmyk(0%, 20%, 40%, 90%)"`
- `"luma(40%)"`
- `"none"`

## Coordinates

Placed elements are measured from the **top-left corner of the page’s trim**: x to the right, y down, in points unless a unit says otherwise.

The bleed lies at negative coordinates and beyond the trim’s width and height. A master’s `bleed` is on the top, bottom and outside edges, as standard perfect-bound printers expect: the right edge of a recto (odd) page and the left edge of a verso. A full-bleed rectangle on a 6 × 9 in page with a 0.125 in bleed is therefore:

```json
{"type": "rect", "x": "-0.125in", "y": "-0.125in", "w": "6.25in", "h": "9.25in", "fill": "accent"}
```


## Fills

A colour, or a gradient:

```json
{"gradient": "linear", "angle": 90, "stops": [["#fff", 0], ["#000", 1]]}
{"gradient": "radial", "stops": [["#fff", 0], ["#000", 1]], "center": [0.5, 0.5], "radius": 0.5}
```

## Strokes

A colour, or:

```json
{"width": "0.5pt", "color": "#000", "dash": [3, 2], "dash_offset": 1.5, "cap": "round", "join": "round"}
```

`dash` is `"dashed"`, `"dotted"` or a list of lengths. Box borders can also be given per side: `{"top": …, "bottom": …}`.

## Insets and radius

One length, or per side: `{"top", "right", "bottom", "left", "x", "y"}` for insets and `{"top_left", …}` for radius.

## What next

- [The design](/docs/model/design/): colours, fonts, styles, masters and components.
- [Content](/docs/model/content/): sections, pages, spreads, elements, blocks and inline runs.
