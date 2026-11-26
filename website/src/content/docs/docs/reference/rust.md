---
title: Rust crates
description: The matenadar umbrella crate and its feature flags, and the matenadar-model, matenadar-typst and matenadar-epub crates it re-exports.
---

Matenadar is a Rust workspace (edition 2024, Apache-2.0). Four crates are published on crates.io, all at version 0.1.0:

| Crate | What it holds | Depends on Typst |
|---|---|---|
| [`matenadar`](https://crates.io/crates/matenadar) | the umbrella crate, and the `matenadar` command | through its features |
| [`matenadar-model`](https://crates.io/crates/matenadar-model) | the document model, validation, components, templates and the operations that change a document | no |
| [`matenadar-typst`](https://crates.io/crates/matenadar-typst) | the generator, the Typst world, sessions, rendering, hit-testing, the layout report and checks | yes (Typst 0.15) |
| [`matenadar-epub`](https://crates.io/crates/matenadar-epub) | EPUB 3, reflowable or one picture per page | yes |

## `matenadar`

```sh
cargo add matenadar
```

A thin facade over the workspace that also builds the `matenadar` binary (`cargo install matenadar`):

- `matenadar::model` is `matenadar-model`;
- `matenadar::typst` is `matenadar-typst` (feature `typst`);
- `matenadar::epub` is `matenadar-epub` (feature `epub`).

It re-exports the names most programs need at its root: `Document`, `Design`, `Item`, `Block`, `Element`, `Inline`, `Run`, `Style`, `TextStyle`, `Master`, `Component`, `Color`, `Length`, `Meta`, `OnPage`, `Place`, `Print`, `Op`, `OpError`, `Problem`, `VERSION` and `validate`; and with `typst`, `Session`, `Paths`, `Editor`, `Change`, `Changes`, `CheckOptions`, `Issue`, `Severity`, `PageInfo`, `Files`, `Fonts`, `UsedFont`, `SourceExport`, `SourceNode`, `EngineError`, `check`, `inspect` and `source_of`.

### Features

| Feature | Default | What it turns on |
|---|---|---|
| `typst` | yes | `matenadar-typst`: layout, rendering, inspection, checks |
| `epub` | yes | `matenadar-epub` (and `typst`) |
| `embedded-fonts` | yes | Typst’s own faces built in, in `matenadar-typst` and `matenadar-epub` |
| `cli` | yes | what the `matenadar` binary needs (`typst`, `epub`, `anyhow`) |

For a library without the command:

```toml
[dependencies]
matenadar = { version = "0.1", default-features = false, features = ["typst", "epub", "embedded-fonts"] }
```

With `default-features = false` and no features, you get the model alone.

## Laying a book out

```rust
use std::path::PathBuf;

use matenadar::{check, inspect, validate, CheckOptions, Document, Paths, Session};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let doc = Document::from_json(&std::fs::read_to_string("book/book.json")?)?;

    let problems = validate(&doc);
    for p in &problems {
        eprintln!("{}: {}", p.path, p.message);
    }
    if !problems.is_empty() {
        return Err("the document has problems".into());
    }

    let paths = Paths {
        root: PathBuf::from("book"),        // pictures, drawings and font folders are read under it
        font_dirs: vec![PathBuf::from("fonts")],
        system_fonts: true,
    };
    let session = Session::new(doc, paths)?;
    println!("{} pages in {:?}", session.page_count(), session.compile_time());

    std::fs::write("book.pdf", session.pdf()?)?;
    if let Some(png) = session.png(1, 150.0, false) {
        std::fs::write("p-001.png", png)?;
    }
    if let Some(svg) = session.svg(1, true) {
        std::fs::write("p-001.svg", svg)?;
    }

    for issue in check(&session, &CheckOptions::default()) {
        println!("{:?} {} {:?} {:?}: {}", issue.severity, issue.code, issue.page, issue.node, issue.message);
    }

    let report = inspect::report_on(&session, Some(&[1]));
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
```

### `Session`

A `Session` holds one document laid out, and lays it out again after each operation.

| Method | Returns |
|---|---|
| `Session::new(doc, paths)` | `Result<Session, EngineError>` |
| `page_count()` | `usize` |
| `compile_time()` | the last layout’s `Duration` |
| `problems()`, `warnings()` | `&[String]`: problems generating the layout (a missing file), and the layout engine’s warnings |
| `pdf()` | `Result<Vec<u8>, EngineError>` |
| `png(page, dpi, bleed)` | `Option<Vec<u8>>` |
| `svg(page, bleed)` | `Option<String>` |
| `rgba(page, scale, bleed)` | `Option<(u32, u32, Vec<u8>)>` |
| `pages()` | `Vec<PageInfo>` |
| `apply(&op)` | `Result<Change, EngineError>`: `inverse`, `changed_pages`, `page_count`, `compile` |
| `apply_all(&ops)` | `Result<Changes, EngineError>`, the operations applied together |
| `locate(page, x, y)` | `Option<(String, usize)>`: the node and byte offset under a point |
| `position(id)` | `Option<(usize, f64, f64)>`: where a node starts |
| `fingerprints()` | `Vec<String>`, a hash per page |
| `fonts_used()` | `Vec<UsedFont>` |
| `document()` | `&Document` |

`CheckOptions` has `safe_pt` (default 18, that is 0.25 in), `gutter_pt` (`None` for KDP’s minimum for the page count), `min_ppi` (300) and `black_ink` (false). `inspect::report(&session)` reports every page; `report_on(&session, Some(&pages))` the pages listed.

### Changing it

```rust
let op: matenadar::Op = serde_json::from_str(
    r#"{"op": "insert-text", "id": "p-2", "offset": 0, "text": "Today, "}"#,
)?;
let change = session.apply(&op)?;            // session must be `mut`
println!("pages to redraw: {:?}", change.changed_pages);
session.apply(&change.inverse)?;             // and back
std::fs::write("book.json", session.document().to_json())?;
```

## `matenadar-model`

The model with **no Typst dependency**, for tools that read, write, validate, template or transform documents.

- `Document::from_json(&str) -> Result<Document, String>` reads a document and refuses a `matenadar` version other than `VERSION` (`"1"`); errors carry the path where reading stopped. `to_json()` writes it, pretty-printed.
- `validate(&Document) -> Vec<Problem>`: each `Problem` has a `path` (`content[0].flow[0].style`) and a `message`.
- `template::compose(kind, &spec) -> Result<Document, String>`, with `template::KINDS` = `picture-book`, `coloring`, `a-plus`.
- `ops`: `Op` (serde, tagged by `"op"`) and the `Target` it addresses (`id` or `at`).
- `walk`: visits every node of a document.

```toml
[dependencies]
matenadar-model = "0.1"
```

## `matenadar-typst`

Lays a document out with Typst 0.15: generation, the Typst world (fonts and files), `Session`, rendering, hit-testing (`Editor`, `caret`), inspection (`inspect`) and checks (`check`). Its `embedded-fonts` feature (default) builds Typst’s own faces in; native builds also scan the system’s fonts.

## `matenadar-epub`

EPUB 3 from a document:

```rust
use matenadar::epub::{build, Layout, Options};

let bytes = build(&doc, &paths, &Options { layout: Layout::Fixed, ..Default::default() })?;
std::fs::write("book.epub", bytes)?;
```

`Options` has `layout` (`Reflowable`, the default, or `Fixed`), `dpi` (200), `embed_fonts` (false), `cover` (a path), `cover_data` (bytes and their extension, for hosts without files), `identifier` (an ISBN as `urn:isbn:…`), `live_text` (true) and `modified`. `build_from(&session, &paths, fonts, &opts)` makes one from a session already laid out. Its `embedded-fonts` feature (default) follows `matenadar-typst`’s.

## The WebAssembly crate

`matenadar-wasm` is built on its own by `scripts/wasm.sh` and published to npm as [`@matenadar/wasm`](/docs/reference/wasm/), not to crates.io.
