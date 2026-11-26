---
title: Working with agents
description: How an AI agent reads, checks and edits a Matenadar book. The layout report, locate, check --json, fingerprints, and the serve JSON-lines protocol.
---

Matenadar was built to be driven by programs as well as people. An agent never has to read a PDF: it can ask the engine what landed where, map a point back to the model, read checks as codes, and change the book with operations that come back with their inverses.

## The loop

1. **Validate.** `matenadar validate book.json` names each problem by its path in the document, such as `content[0].flow[0].style: no style "lede"`. With `--json` the list is `[{"path", "message"}]`, and the command exits with status 1 when there are problems.
2. **Lay out and report.** `matenadar layout book.json --pages 12-14` gives every node’s box on those pages.
3. **Check.** `matenadar check book.json --json` gives issues with stable codes, pages and node ids.
4. **Change.** Write [operations](/docs/guides/operations/) against node ids and apply them; keep the inverses.
5. **Repeat** until `check` is clean, then build.

For a long session, use [`serve`](#serve) so the book stays laid out between steps.

## The layout report

```console
$ matenadar layout book.json --pages 43
```

```json
[
  {
    "page": 43,
    "label": "43",
    "width": 432.0,
    "height": 648.0,
    "nodes": [
      {"id": "ch3", "kind": "section", "rect": {"x0": 57.6, "y0": 57.6, "x1": 316.8, "y1": 583.2}},
      {"id": "p-14", "kind": "p", "rect": {"x0": 57.6, "y0": 132.5, "x1": 316.8, "y1": 196.1}}
    ]
  }
]
```

(Abridged.) Each page has its number, its printed `label` (`"iv"`, `"43"`), its trim size in points, and every node on it with its `kind` and its extent there: a `rect` with `x0`, `y0`, `x1`, `y1` in points from the trim’s top-left corner. A node that runs over several pages is listed on each. Leave out `--pages` for the whole book; `1-3,7` and `5-` work too.

## Locate

```console
$ matenadar locate book.json --page 43 --x 212 --y 148
{"node":"p-14","offset":118}
```

The node under a point and the byte offset in its words, or `null` when nothing is there. Together with the report, this maps a reviewer’s mark on a proof (“this word, here”) straight to the model.

## Checks as data

`check --json` gives `{"severity", "code", "page", "node", "message"}` for each issue. Act on `code`; the [preflight guide](/docs/guides/checks/) lists every code and what usually fixes it.

## Fingerprints

```console
$ matenadar fingerprint book.json
{"pages":48,"fingerprints":["…","…"]}
```

A hash per page. The WebAssembly build’s `fingerprint()` gives the same hashes for the same document, fonts and pictures, so an agent can confirm that what a person sees in the browser editor is what the command line will print, and see exactly which pages an edit changed.

## Timing

```console
$ matenadar bench book.json --edits 50
```

```json
{
  "pages": 48,
  "first_layout_ms": 412.3,
  "edits": 50,
  "edit_p50_ms": 21.4,
  "edit_p95_ms": 33.0,
  "edit_max_ms": 41.7,
  "mean_pages_changed": 1.3
}
```

`bench` lays the book out, then makes small edits (inserting a word at the start of paragraphs with ids, each undone) and reports how long it takes to lay it out again: what an editor or an agent loop feels. The numbers here are illustrative.

## `serve`

`matenadar serve` keeps one document laid out in memory, driven by **JSON lines** on stdin. Each request is one line:

```json
{"id": 1, "method": "open", "params": {"path": "book.json", "root": "."}}
```

and gets one line on stdout, with the same `id`:

```json
{"id": 1, "result": {"pages": 48, "layout_ms": 412.3, "problems": [], "warnings": []}}
```

or `{"id": 1, "error": "…"}`. A line that isn’t JSON gets `{"id": null, "error": "not JSON: …"}`. Blank lines are skipped. Every method except `open` and `quit` needs a document open first.

### Methods

| Method | Params | Result |
|---|---|---|
| `open` | `doc` (the document itself) or `path`; `root` (default `.`); `fonts` (folders); `system_fonts` (default true) | `{pages, layout_ms, problems, warnings}`; an invalid document is an error listing its problems |
| `apply` | `ops`: a list of operations | the changes, with inverses and changed pages, as `matenadar apply` prints them |
| `render` | `page`, `out` (a file), `format` (`png`, the default, or `svg`), `dpi` (100), `bleed` (false) | `{out}` |
| `pdf` | `out` | `{out, pages}` |
| `check` | `safe` (`"0.25in"`), `min_ppi` | the issues |
| `layout` | `pages` (a list of numbers; all when left out) | the layout report |
| `locate` | `page`, `x`, `y` | `{node, offset}` or `null` |
| `position` | `id` | where the node starts, `{page, x, y}`, or `null` |
| `document` | — | the current document |
| `save` | `out` | `{out}` |
| `quit` | — | `true`, then the server stops |

### A session

```text
→ {"id":1,"method":"open","params":{"path":"book.json"}}
← {"id":1,"result":{"pages":48,"layout_ms":412.3,"problems":[],"warnings":[]}}
→ {"id":2,"method":"check","params":{"safe":"0.25in"}}
← {"id":2,"result":[{"severity":"warning","code":"widow","page":44,"node":"p-188","message":"p-188 carries one line to the top of page 44"}]}
→ {"id":3,"method":"apply","params":{"ops":[{"op":"set","id":"p-188","path":"/style","value":"tight"}]}}
← {"id":3,"result":[{"inverse":{"op":"set","id":"p-188","path":"/style","value":"body"},"changed_pages":[43,44],"page_count":48,"compile":18.2}]}
→ {"id":4,"method":"render","params":{"page":44,"format":"png","dpi":150,"out":"p44.png"}}
← {"id":4,"result":{"out":"p44.png"}}
→ {"id":5,"method":"save","params":{"out":"book.json"}}
← {"id":5,"result":{"out":"book.json"}}
→ {"id":6,"method":"quit"}
← {"id":6,"result":true}
```

From a program, start `matenadar serve` as a child process, write one request per line, and read one reply per line. Replies come back in order.

```js
import { spawn } from 'node:child_process';
import { createInterface } from 'node:readline';

const engine = spawn('matenadar', ['serve']);
const replies = createInterface({ input: engine.stdout });
const waiting = new Map();
let next = 1;
replies.on('line', (line) => {
	const { id, result, error } = JSON.parse(line);
	const pending = waiting.get(id);
	waiting.delete(id);
	error ? pending.reject(new Error(error)) : pending.resolve(result);
});
const call = (method, params = {}) =>
	new Promise((resolve, reject) => {
		const id = next++;
		waiting.set(id, { resolve, reject });
		engine.stdin.write(JSON.stringify({ id, method, params }) + '\n');
	});

await call('open', { path: 'book.json' });
const issues = await call('check');
```

## Habits that help

- **Give everything an id** you will want to find or change. Reports, checks and operations all speak in ids.
- **Insert with `before` and `after`**, not `index`, so kept edits survive a recomposed manuscript.
- **Read codes, not messages.** The codes are stable.
- **Use `decorative`** for display type and art, so margin checks are about reading text.
- **Keep inverses.** An agent can always take back what it did, in reverse order.
- **Compare fingerprints** to know which pages an edit changed, without diffing pictures.
