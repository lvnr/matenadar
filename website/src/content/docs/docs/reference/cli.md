---
title: Command line
description: Every matenadar command, flag, default, output and exit status, from crates/matenadar/src/main.rs and serve.rs.
---

```text
matenadar <command> [arguments] [options]
```

| Command | What it does |
|---|---|
| [`build`](#build) | lays a document out to PDF, and optionally PNG and SVG pages, with an optional check |
| [`epub`](#epub) | makes an EPUB 3, reflowable or fixed layout |
| [`check`](#check) | preflight checks: overflow, text in the margin, resolution, ink, glyphs, widows, orphans |
| [`validate`](#validate) | checks the model without laying it out |
| [`layout`](#layout) | every node on every page, with its extent |
| [`locate`](#locate) | the node and byte offset under a point |
| [`fingerprint`](#fingerprint) | a hash per page, the same as the WebAssembly build’s |
| [`apply`](#apply) | applies operations and prints their inverses |
| [`fonts`](#fonts) | the font families found, or the faces a layout drew with |
| [`bench`](#bench) | layout and edit timings |
| [`serve`](#serve) | one document kept open, driven by JSON lines on stdin |
| `help` | the usage text (also `--help`, `-h`, or no command) |
| `--version` | prints `matenadar 0.3.0` (also `version`, `-V`) |

## Common options

| Option | Meaning |
|---|---|
| `--root DIR` | the asset root: pictures, drawings and the document’s font folders are read under it. Default: the document’s folder. |
| `--fonts DIR` | another font folder. Repeatable. |
| `--no-system-fonts` | don’t use the system’s fonts |
| `--json` | machine-readable output, where a command has it |

### How arguments are read

- The document is the first positional argument; `apply` takes a second (`ops.json`).
- `--bleed`, `--check`, `--json`, `--no-system-fonts`, `--embed-fonts`, `--no-live-text`, `--used` and `--no-tagged` are switches. Every other `--option`, and `-o`, takes the next argument as its value.
- Given twice, an option’s last value wins, except `--fonts`, `--pdf-standard` and `--attach`, which add up.
- `--pages` takes a list of pages and ranges: `1-3,7`, or `5-` for page 5 to the end. Pages outside the document are ignored.
- Lengths are as in the model: `0.25in`, `6mm`, `18pt`, `18`.

### Exit status

| Status | Meaning |
|---|---|
| `0` | done |
| `1` | `check`, or `build --check`, found an issue of severity error; or `validate` found problems |
| `2` | an error: a missing or unreadable file, an invalid document, a bad option. The message is printed to stderr as `error: …`. |

Commands that lay a document out validate it first, and stop with every problem listed (`the document has problems:`) when it isn’t valid.

## build

```text
matenadar build <doc.json> [-o out.pdf] [--png DIR] [--svg DIR] [--dpi N] [--pages 1-3,7] [--bleed] [--check]
                [--pdf-standard STD] [--no-tagged] [--attach SPEC] [--max-ppi N] [--pdf-timestamp ISO] [--json]
```

Lays the document out and writes the PDF.

| Option | Meaning | Default |
|---|---|---|
| `-o FILE` | the PDF | the document’s path with `.pdf` |
| `--png DIR` | also write pages as PNG, `DIR/p-001.png`… | — |
| `--svg DIR` | also write pages as SVG, `DIR/p-001.svg`… | — |
| `--dpi N` | PNG resolution | `100` |
| `--pages LIST` | which pages to write as PNG and SVG (the PDF has them all) | all |
| `--bleed` | draw PNG and SVG pages with their bleed | off |
| `--check` | run the [checks](#check) after building (with `check`’s options) | off |
| `--pdf-standard STD` | PDF standard (repeatable): `1.4`–`2.0`, `a-1b`–`a-4e`, `ua-1` | from `doc.pdf` (`1.7`) |
| `--no-tagged` | disable tagged PDF structure | tagged on |
| `--attach SPEC` | embed a file (repeatable): `path` or `name:path[:mime[:relationship[:description]]]` | — |
| `--max-ppi N` | deterministically downsample oversized raster images to at most `N` PPI | from `doc.print.max_ppi` |
| `--pdf-timestamp ISO` | UTC creation timestamp (`YYYY-MM-DD` or `YYYY-MM-DDTHH:MM:SSZ`) | deterministic fallback when required |

```console
$ matenadar build book.json -o book.pdf --png proofs --dpi 150 --pages 1-4 --check
48 pages → book.pdf (layout 412 ms, total 980 ms)
  warning widow [p44 p-188]: p-188 carries one line to the top of page 44
```

With `--json`:

```json
{"pdf": "book.pdf", "pages": 48, "layout_ms": 412.3, "total_ms": 980.1, "problems": [], "warnings": [], "issues": []}
```

`problems` are problems generating the layout, such as a missing file; `warnings` are the layout engine’s; `issues` are the checks’ (empty without `--check`).

## epub

```text
matenadar epub <doc.json> [-o out.epub] [--layout reflowable|fixed] [--dpi 200] [--embed-fonts] [--no-live-text] [--cover image.jpg] [--identifier urn:isbn:…] [--json]
```

| Option | Meaning | Default |
|---|---|---|
| `-o FILE` | the EPUB | the document’s path with `.epub` |
| `--layout` | `reflowable` or `fixed` | `reflowable` |
| `--dpi N` | resolution of page and drawing pictures | `200` |
| `--embed-fonts` | put the book’s faces in it (those licensed for it) | off |
| `--no-live-text` | fixed layout: every page one picture, its text as alt text | off |
| `--cover FILE` | the cover image | the first hand-laid page, if the book opens with one |
| `--identifier ID` | the book’s ID, an ISBN URN | derived from the title and author |

Prints `book.epub (1.4 MB, 410 ms)`, or with `--json` `{"epub": "book.epub", "bytes": 1432200, "total_ms": 410.2}`. See [EPUB 3](/docs/guides/epub/).

## check

```text
matenadar check <doc.json> [--preset kdp] [--safe 0.25in] [--gutter 0.375in] [--min-ppi 300] [--ink black] [--json]
```

| Option | Meaning | Default |
|---|---|---|
| `--preset NAME` | print-on-demand preset (`kdp`) | — |
| `--safe LENGTH` | how far inside the trim live text must stay | `0.25in` |
| `--gutter LENGTH` | the spine-side margin | page-count-scaled spine gutter |
| `--min-ppi N` | the resolution raster images must reach | `300` |
| `--ink black` or `color` | hold the pages to a black-ink interior | `color` (or the document’s `print.ink`) |

Prints `no issues on N pages`, or one line per issue: `  <severity> <code> [p<page> <node>]: <message>`. With `--json`, a list of `{"severity", "code", "page", "node", "message"}`. Exits with 1 when any issue is an error. See [preflight checks](/docs/guides/checks/) for the codes.

## validate

```text
matenadar validate <doc.json> [--json]
```

Checks the model without laying it out: styles, masters and components that are named but missing, colours, ids and the like. Prints `valid`, or one `path: message` line per problem (`content[0].flow[0].style: no style "lede"`). With `--json`, a list of `{"path", "message"}`. Exits with 1 when there are problems.

A document that isn’t JSON, or doesn’t match the model, is an error before validation, with the path where reading stopped. A document whose `matenadar` isn’t `"1"` is refused.

## layout

```text
matenadar layout <doc.json> [--pages 1-3]
```

Prints the layout report as JSON: for each page, `{"page", "label", "width", "height", "nodes": [{"id", "kind", "rect": {"x0", "y0", "x1", "y1"}}]}`, in points from the trim’s top-left corner. See [working with agents](/docs/guides/agents/#the-layout-report).

## locate

```text
matenadar locate <doc.json> --page N --x PT --y PT
```

Prints `{"node":"p-14","offset":118}`, the node under the point and the byte offset in its words, or `null`. `x` and `y` are points from the trim’s top-left corner.

## fingerprint

```text
matenadar fingerprint <doc.json>
```

Prints `{"pages": N, "fingerprints": [...]}`, a hash per page. The WebAssembly build’s `fingerprint()` gives the same for the same document, fonts and pictures.

## apply

```text
matenadar apply <doc.json> <ops.json> [-o new.json]
```

Applies one operation, or a list, in order. Writes the document to `-o`, or **back to `doc.json`** without it, and prints a list of changes: `{"inverse", "changed_pages", "page_count", "compile"}` per operation. If any operation fails, nothing is written. See [operations](/docs/guides/operations/).

## fonts

```text
matenadar fonts [--root DIR] [--fonts DIR] [--no-system-fonts]
matenadar fonts --used <doc.json>
```

Without `--used`, prints the font families it can find, one per line: the system’s (unless `--no-system-fonts`), Typst’s own, and any `--fonts` folders.

With `--used`, lays the document out and prints the faces it drew with as JSON: `family`, `weight`, `italic`, `stretch` (percent), the `file` and `index` it came from, and the `pages` it is on. Give the WebAssembly build the same files, and its pages will match.

## bench

```text
matenadar bench <doc.json> [--edits 50]
```

Lays the document out, then makes `--edits` small edits (inserting a word at the start of paragraphs that have ids and plain text, each one undone again) and prints the timings as JSON: `pages`, `first_layout_ms`, `edits`, `edit_p50_ms`, `edit_p95_ms`, `edit_max_ms` and `mean_pages_changed`.

## serve

```text
matenadar serve
```

Keeps one document laid out in memory, driven by JSON lines on stdin: `{"id", "method", "params"}` in, `{"id", "result"}` or `{"id", "error"}` out, one line each. The methods are `open`, `apply`, `render`, `pdf`, `notes`, `check`, `layout`, `locate`, `position`, `document`, `save` and `quit`. See [`serve`](/docs/guides/agents/#serve) for their parameters and results.

:::note[Debugging the generator]
`matenadar debug-typst <doc.json>` prints the Typst program generated for a document. It is a tool for working on **matenadar** itself, not part of the stable interface: documents are always changed through the model.
:::
