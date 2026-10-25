# Matenadar

A book and page layout engine for people and agents. A document is a JSON
model of sections, pages, styles, masters and components. Matenadar lays it
out with [Typst](https://typst.app) and renders it to PDF, PNG or SVG. It can
also say what landed where, check pages the way a printer or KDP's previewer
would, and apply edits as operations that can be undone.

It is the engine behind Aldus's typesetting, and Imprimo's successor. The plan
it grew from is `~/projects/imprimo/docs/plans/2026-10-25-imprimo-v1.md`; the
NEO-related parts and the imports were dropped for now.

## Why this shape

- **The model is the only source of truth.** The Typst program is generated on
  every build and never stored or shown, so people, the Aldus engine and AI
  agents all change books through the same model and operations.
- **Typst does the hard typesetting:** Knuth–Plass line breaking, hyphenation,
  widow and orphan control, footnotes, tables that split, variable fonts, and
  right-to-left and CJK text. Builds are fast and deterministic.
- **Designed pages are first-class.** The model has fixed pages with placed
  elements, masters with conditional layers, margin notes, letter grids,
  lettering drawn as outlines, QR codes, inline SVG, pen marks, shadows and
  components. Books that once needed a hand-written HTML/CSS engine can be
  expressed directly.
- **Agents are first-class.** Errors carry JSON paths. `layout` reports every
  node's extent on every page. `locate` maps a point back to a node and
  character. `check` gives codes an agent can act on. `serve` keeps a book open
  and lays it out again in milliseconds after each operation.

## Layout

```
crates/matenadar-model   the document model, validation, components, operations (no Typst)
crates/matenadar-typst   generator, Typst world, session, inspection, checks
crates/matenadar-cli     the `matenadar` command
docs/model.md            the model, field by field
samples/                 pages of real books rebuilt in Matenadar, with compare sheets
scripts/review.py        builds a sample and sets it beside the original book's pages
```

## Quick start

```bash
cargo build
target/debug/matenadar build tests/fixtures/smoke.json -o smoke.pdf --png out --check
target/debug/matenadar layout tests/fixtures/smoke.json --pages 3
target/debug/matenadar bench tests/fixtures/smoke.json
```

`docs/model.md` documents the model. `matenadar help` lists the commands.

## Status

- The model, generator, checks, operations, `serve` and `bench` work.
- Eight of Levon's books are partly rebuilt under `samples/`, each with notes
  on how close it gets and which engine gaps remain.
- Not done yet:
  - a WebAssembly build for the browser editor;
  - EPUB from the model;
  - columns that balance themselves;
  - text on a path without SVG;
  - opacity of whole groups.
