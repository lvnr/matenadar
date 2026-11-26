---
title: WebAssembly API
description: The @matenadar/wasm package. Matenadar.load, openFrom, MatenadarClient.start with a jsDelivr CDN module, and every method of the page editor's API, from matenadar.d.ts.
---

`@matenadar/wasm` is the engine built to WebAssembly. It lays out, edits, inspects, checks and renders books to SVG, PNG, PDF and EPUB 3 in browsers, workers, Node (18 and later), Bun and Deno, and it lays out exactly as the command line does: the same document, fonts and pictures give the same pages, which `fingerprint()` shows.

```sh
npm install @matenadar/wasm
```

The module, `matenadar.wasm`, is about 24 MB (7 MB with brotli) and has no imports and no file system of its own: you give it fonts and pictures as bytes, or let `openFrom` read them.

## Entry points

| Import | What it is |
|---|---|
| `@matenadar/wasm` | `Matenadar`, the engine with a synchronous API. Node gets the build that reads files; browsers, workers and bundlers get the one with no `node:*` imports. |
| `@matenadar/wasm/browser` | the browser build, explicitly |
| `@matenadar/wasm/client` | `MatenadarClient`: runs the engine in a worker and gives every method as a promise |
| `@matenadar/wasm/worker` | the worker script the client starts |
| `@matenadar/wasm/wasm` | the `matenadar.wasm` file itself |

## In Node, Bun and Deno

```js
import { readFile, writeFile } from 'node:fs/promises';
import { Matenadar } from '@matenadar/wasm';

const doc = JSON.parse(await readFile('./book/book.json', 'utf8'));

const mt = await Matenadar.load();
const { pages, diagnostics, missing } = await mt.openFrom(doc, {
	root: './book',
	fonts: ['./fonts'],
});

await writeFile('book.pdf', mt.pdf());
await writeFile('p1.svg', mt.svg(1, { bleed: true }));
await writeFile('book.epub', mt.epub({ layout: 'reflowable' }));
```

### `Matenadar.load(source?)`

Loads the module. With no argument it loads the `matenadar.wasm` bundled next to the package. It also takes a URL or path, a `Response` or a promise of one, bytes, or a compiled `WebAssembly.Module`. A response served as `application/wasm` is compiled while it streams.

### `openFrom(doc, {root, fonts, check})`

Lays a document out with what it names, read under `root`: a folder in Node, a base URL in a browser. It reads the fonts in the document’s font folders (Node only: a browser can’t list a folder) and its pictures, and `fonts` adds more folders, files or URLs. It returns `open`’s result and `missing`, the files that couldn’t be read.

### `addFonts(...sources)`

Fonts from folders (in Node, read through), font files, URLs or bytes: every `.otf`, `.ttf`, `.otc` and `.ttc` found. Returns how many faces were added.

## In the browser

Run the engine in a worker, so layout never blocks the page:

```js
import { MatenadarClient } from '@matenadar/wasm/client';

const mt = await MatenadarClient.start();

const font = await fetch('/fonts/LibertinusSerif-Regular.otf');
await mt.addFont(await font.arrayBuffer());

const { pages } = await mt.open(doc);
pageElement.innerHTML = await mt.svg(1, { bleed: true });
```

### From jsDelivr

The module is too large for some static hosts (Cloudflare’s limit is 25 MiB a file). Load it from the npm CDN, and keep the small worker on your own origin, as browsers require:

```js
const mt = await MatenadarClient.start({
	wasm: 'https://cdn.jsdelivr.net/npm/@matenadar/wasm@0.2.0/dist/matenadar.wasm',
});
```

### `MatenadarClient.start(options?)`

| Option | Meaning |
|---|---|
| `base` | the folder with `worker.js` and `matenadar.wasm` (default: the client module’s own folder) |
| `wasm` | a URL for `matenadar.wasm` that overrides `base`, such as the jsDelivr URL above |
| `worker` | a `Worker` you have already made |

It returns the client with every method of `Matenadar` as a promise, in call order. `call(method, args)` calls one by name; byte arguments (fonts, pictures) are transferred to the worker, not copied. `terminate()` stops the worker.

### Fonts and files in a browser

A browser can’t list folders, so ask the engine what a document needs and fetch it:

```js
const { fonts, files } = await mt.assets(doc); // font folders and files, as the document names them
for (const url of myFontUrls) await mt.addFont(await (await fetch(url)).arrayBuffer());
for (const path of files) await mt.addFile(path, await (await fetch(`/book/${path}`)).arrayBuffer());
const { pages, diagnostics } = await mt.open(doc);
```

Give the browser **every face the book uses**, bold and italic included. `matenadar fonts --used doc.json` lists the faces a command-line layout drew with and the file each came from; `fontsUsed()` gives the browser’s. The command line also has Typst’s own faces built in; a WebAssembly build made with `scripts/wasm.sh --embedded-fonts` adds them too (about 5 MB).

## Methods

Lengths are points; positions are from the trim’s top-left corner, x to the right and y down. Pages are 1-based. In the client every method returns a promise.

### Loading

| Method | Returns | |
|---|---|---|
| `assets(doc)` | `{fonts: string[], files: string[]}` | the font folders and files a document needs, as it names them under its root |
| `addFont(data)` | `number` | adds an OTF, TTF or collection; returns how many faces it holds |
| `addFonts(...sources)` | `Promise<number>` | fonts from folders (Node), files, URLs or bytes |
| `addFile(path, data)` | `void` | adds or replaces a picture or drawing at `path` under the root |
| `open(doc, check?)` | `Opened` | lays a document out: `{pages, problems, warnings, diagnostics}` |
| `openFrom(doc, {root, fonts, check})` | `Promise<Opened & {missing}>` | opens with what the document names, read under `root` |
| `fonts()` | `string[]` | the font families given |
| `close()` | `void` | closes the document |

### Pages

| Method | Returns | |
|---|---|---|
| `pages()` | `PageInfo[]` | every page’s trim, bleed per side, recto or verso, item, master and margins |
| `spreads()` | `number[][]` | pages as they face each other: `[[1], [2, 3], …]` |
| `svg(page, {bleed})` | `string` | a page as SVG |
| `png(page, {scale, dpi, bleed})` | `Uint8Array` | a page as PNG; `scale` is pixels per point (1 = 72 dpi), or give `dpi` |
| `rgba(page, {scale, bleed})` | `{width, height, data}` | straight RGBA pixels, for a canvas |
| `pdf()` | `Uint8Array` | the whole document as PDF |
| `fingerprint()` | `string[]` | a hash per page, as `matenadar fingerprint` gives |
| `fontsUsed()` | `UsedFont[]` | the faces the layout drew with, and on which pages |

### Finding

| Method | Returns | |
|---|---|---|
| `hit(page, x, y)` | `Hit \| null` | the innermost node under a point: its `id`, `kind`, `page`, `rect` on that page, the byte `offset` in its words, the `element` around it and every `ancestor` |
| `rects(id)` | `{page, rect}[]` | a node’s extent on each page it reaches |
| `node(id)` | JSON | a node as it stands, or `null` |
| `lines(id)` | `number` | how many lines a paragraph sets in |
| `locate(page, x, y)` | `{node, offset} \| null` | the node and byte offset under a point |
| `position(id)` | `{page, x, y} \| null` | where a node starts |
| `layout(pages?)` | page reports | every node on every page (or those listed), with its extent |
| `document()` | document | the document as it stands |
| `source()` | `{text, prelude, nodes}` | the Typst program the engine generated, with each node’s range in UTF-16 offsets, for showing beside the page |

### The caret

Offsets are **bytes** of the node’s plain text (UTF-8). They follow the glyphs, so they work in text frames (whose words are the frame’s, blank lines included) and beside a drop cap.

| Method | Returns | |
|---|---|---|
| `caret(id, offset, {upstream})` | `{page, x, y, h} \| null` | the caret before a byte; `y` is the line’s top. Where a line breaks inside a word, `upstream` keeps the caret at the end of the line before. |
| `offsetAt(id, page, x, y)` | `number \| null` | the nearest place in the words to a point |
| `selectionRects(id, start, end)` | `{page, rect}[]` | a rectangle per line over the words `start..end` |
| `textLines(id)` | `TextLine[]` | the lines a node’s words set in: `{page, rect, start, end}` |

### Editing

Each call is **one step to undo**, laid out once, and returns `Edited`: the `changed_pages` to draw again (including new ones), the `page_count`, the `diagnostics` (checked again only on the pages that changed), `can_undo`, `can_redo`, and `layout_ms` (0 in WebAssembly, which has no clock: time the call instead).

| Method | |
|---|---|
| `apply(ops, {part})` | operations, applied together; `{part: "layout"}` keeps the whole batch in the layout patch |
| `setWords(id, runs)` | a paragraph’s or heading’s words, as runs (marks are run fields) |
| `insertText(id, offset, text)` | inserts words at a byte offset (marks kept) |
| `deleteText(id, start, end)` | deletes the bytes `start..end` (marks kept) |
| `insertBlock({after} or {before}, block)` | a block placed after or before a sibling |
| `removeBlock(id)` | removes a block |
| `splitBlock(id, offset, newId, end?)` | the words after the offset go to a new block `newId` after it, with its style and overrides but not those of a paragraph’s start; a heading split at its end is followed by a paragraph. With `end`, the words `offset..end` go first (Enter over a selection), in the same step. |
| `joinBlock(id)` | joins a paragraph or heading to the one before it; returns `{into, offset}` too, where its words now start |
| `markText(id, start, end, field, value)` | marks a range of words: `em`, `strong`, `span`, `color`, `size`, `tracking`, `case`…; `null` takes the mark off |
| `setFields(id, fields, {style})` | a node’s fields (`null` removes one); with `{style: true}`, a paragraph’s or heading’s style overrides |
| `setPicture(id, fields)` | a picture: `src`, `crop` (`{x, y, w, h}` fractions), `fit`, `focus`, `w`, `h`, `x`, `y`, `opacity`… |
| `setBlock(id, fields)` | spacing, alignment and keeps: `space_before`, `space_after`, `align`, `keep_with_next`, `keep_together`… |
| `moveFrame(id, {x, y, w, h, rotate})` | a placed element’s frame |
| `breakBefore(id, to)` | starts a node on a new page (`"page"`, `"recto"`, `"verso"`), or with `null` takes such a break back: a weak `page-break` with the id `<id>--break`, or the style’s `page_break_before` turned off |
| `hideOnPage(id, hide)` | leaves master elements (`["folio"]`) off the page where the node lands; `[]` puts them back |
| `nudge(id, -1 or 1)` | sets a paragraph a line shorter or longer by the least tracking that does it (at most 0.02 em); also returns `lines_before`, `lines_after` and `tracking` |
| `undo()`, `redo()` | |

### Results

| Method | Returns | |
|---|---|---|
| `diagnostics()` | `Issue[]` | the issues now |
| `check(options)` | `Issue[]` | a full check with other settings: `{safe, gutter, min_ppi, ink}` |
| `patch()` | `{words, touched, layout}` | the edits since the document opened, or the last rebase (see [the patch](/docs/guides/operations/#the-patch)) |
| `rebase()` | `void` | makes the document as it stands the patch’s base, and forgets undo |
| `epub(options)` | `Uint8Array` | the EPUB 3: `{layout, dpi, embedFonts, liveText, identifier, cover, modified}` |
| `memory()` | `number` | the module’s memory in bytes; it only grows, so this is its peak |

## Types

```ts
interface PageInfo {
	page: number;
	label: string; // the page number as printed ("iv", "12")
	side: 'recto' | 'verso';
	item: string | null; // the section or page it belongs to
	master: string | null;
	trim: { w: number; h: number }; // the trim box is [0, 0, w, h]
	bleed: { top: number; right: number; bottom: number; left: number };
	margins: { top: number; bottom: number; inside: number; outside: number } | null;
}

interface Rect { x0: number; y0: number; x1: number; y1: number }

interface Hit {
	id: string;
	kind: string;
	page: number;
	rect: Rect;
	offset?: number;
	element?: { id: string; kind: string };
	ancestors: { id: string; kind: string }[];
}

interface Issue {
	severity: 'error' | 'warning' | 'info';
	code: string; // overflow, text-in-margin, low-resolution, color-in-black-ink, missing-glyph, …
	page?: number;
	node?: string;
	message: string;
}

interface CheckOptions {
	safe?: string | number; // default 0.25in
	gutter?: string | number; // default page-count-scaled spine gutter
	min_ppi?: number;
	ink?: 'black' | 'color';
}

interface EpubOptions {
	layout?: 'reflowable' | 'fixed';
	dpi?: number; // 150
	embedFonts?: boolean;
	liveText?: boolean; // default true
	identifier?: string; // urn:isbn:…
	cover?: string; // a file given with addFile
	modified?: string; // CCYY-MM-DDThh:mm:ssZ, default now
}

interface Patch {
	words: Op[];
	touched: string[];
	layout: Op[];
}
```

The full declarations ship in the package as `matenadar.d.ts` and `client.d.ts`.

## An editor in a few lines

```js
import { MatenadarClient } from '@matenadar/wasm/client';

const mt = await MatenadarClient.start({
	wasm: 'https://cdn.jsdelivr.net/npm/@matenadar/wasm@0.2.0/dist/matenadar.wasm',
});
for (const font of fontFiles) await mt.addFont(font); // ArrayBuffers
const { pages } = await mt.open(doc);
page.innerHTML = await mt.svg(1, { bleed: true });

page.addEventListener('click', async (event) => {
	const [x, y] = toPoints(event); // points from the trim's corner
	const hit = await mt.hit(1, x, y);
	if (!hit) return;
	const done = await mt.setBlock(hit.id, { space_before: '12pt' });
	for (const n of done.changed_pages) redraw(n);
});

// later: keep the edits as a patch the command line replays
const { words, layout } = await mt.patch();
```

`demo/editor/` in the repository is a page editor on the samples, built on this API: `scripts/wasm.sh && python3 scripts/demo.py && python3 -m http.server 5199`, then open `http://localhost:5199/demo/editor/`.
