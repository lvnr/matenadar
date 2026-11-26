---
title: Preflight checks
description: Check pages the way a printer or KDP's previewer would. Every code matenadar check reports, its severity, the options, and the JSON an agent reads.
---

`matenadar check` lays a document out and inspects the pages the way a printer, or KDP’s previewer, would. Every issue has a stable **code**, a **severity**, and where it can, the **page** and **node** it is about.

```console
$ matenadar check book.json --safe 0.25in
  error text-in-margin [p12 caption-3]: 2 runs of live text sit closer than KDP allows to the trim (0.250 in outside, 0.375 in at the spine); mark art as decorative or move it
  warning widow [p44 p-188]: p-188 carries one line to the top of page 44
```

With no issues it prints `no issues on 48 pages`. The command exits with status **1 when any issue is an error**, so it can gate a build or a CI job. `matenadar build --check` runs the same checks after building.

## The codes

| Code | Severity | What it means |
|---|---|---|
| `overflow` | error | a frame’s text doesn’t fit (a text frame with an `h` and an `id`): “the text of *id* runs *n* pt past its frame” |
| `text-in-margin` | error | live text closer to the trim than KDP allows: 0.25 in outside, and at the spine KDP’s minimum for the page count |
| `low-resolution` | error | a raster image under the minimum resolution (300 ppi) at its printed size |
| `color-in-black-ink` | error | on a black-ink interior, a picture with colour in it (more than a few clearly coloured pixels, or an overall cast), or text or a shape in a colour that isn’t a grey; one report per element and page |
| `missing-glyph` | error | no font has the glyph |
| `missing-file` | error | a file the document names (a picture or a drawing) isn’t under the asset root |
| `fallback-font` | warning | a family no style names was used (a fallback for missing glyphs?) |
| `widow` | warning | a paragraph carries one line to the top of the next page |
| `orphan` | warning | a paragraph leaves one line at the foot of a page |
| `compiler` | error or warning | notices from the layout engine |

### KDP’s gutter

Without `--gutter`, the spine-side margin is KDP’s minimum for the book’s page count:

| Pages | Inside margin |
|---|---|
| up to 150 | 0.375 in |
| 151–300 | 0.5 in |
| 301–500 | 0.625 in |
| 501–700 | 0.75 in |
| over 700 | 0.875 in |

## Options

| Option | Meaning | Default |
|---|---|---|
| `--safe LENGTH` | how far inside the trim live text must stay | `0.25in` |
| `--gutter LENGTH` | the spine-side margin | KDP’s minimum for the page count |
| `--min-ppi N` | the resolution raster images must reach | `300` |
| `--ink black` | hold the pages to a black-ink interior even when the document doesn’t say it is one (`print.ink`); `--ink color` is the default | — |
| `--json` | the issues as JSON | — |

The common options apply too: `--root`, `--fonts`, `--no-system-fonts`.

## As JSON

```console
$ matenadar check book.json --json
```

```json
[
  {
    "severity": "error",
    "code": "text-in-margin",
    "page": 12,
    "node": "caption-3",
    "message": "2 runs of live text sit closer than KDP allows to the trim (0.250 in outside, 0.375 in at the spine); mark art as decorative or move it"
  },
  {
    "severity": "warning",
    "code": "widow",
    "page": 44,
    "node": "p-188",
    "message": "p-188 carries one line to the top of page 44"
  }
]
```

`page` and `node` are left out when an issue has neither. Read `code`, not `message`: the codes are stable, the wording may change.

## Fixing what they find

- **`text-in-margin` on display type.** If the type is art (a title across the bleed, a big numeral), mark the element `"decorative": true`. It is drawn as glyph outlines, so neither KDP’s previewer nor any reader sees text there, and the margin check skips it. Otherwise move it in, or give a text frame `"safe": true` to keep it inside the master’s margins.
- **`overflow`.** Give the frame more room, or let it fit its words with `"fit": "shrink"` and a `min_size`.
- **`widow` and `orphan`.** They are prevented by default, so a report usually means something else won: a `keep_together`, a box that can’t break, or a page too short to fix. In the editor, `nudge(id, -1)` sets a paragraph a line shorter by the least tracking that does it (at most 0.02 em).
- **`color-in-black-ink`.** Give pictures a grey filter, `"filter": {"gray": true}`, or set one for every picture with `print.filter`, and set type and shapes in greys.
- **`low-resolution`.** Use a larger file, or print the picture smaller.

## In `serve` and WebAssembly

`serve` has a `check` method with `safe` and `min_ppi`. In WebAssembly, `open(doc, check)` and `openFrom(doc, {check})` take the options (`safe`, `gutter`, `min_ppi`, `ink`), every edit returns the `diagnostics` for the pages it changed, and `check(options)` runs a full check with other settings.
