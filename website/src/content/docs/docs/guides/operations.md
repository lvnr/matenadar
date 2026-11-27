---
title: Operations
description: Every change to a matenadar document is an operation with an inverse. The seven operations, how they address nodes, and how to apply, undo and replay them.
---

Every change to a document is an **operation**, and applying one returns its **inverse**: the operation that takes it back. The command line, the `serve` loop, the WebAssembly editor and AI agents all change documents this way, so any edit can be undone, kept, sent to someone else and replayed.

## The operations

```json
{"op": "set", "id": "p12", "path": "/style", "value": "lede"}
{"op": "unset", "id": "p12", "path": "/set/size"}
{"op": "insert", "id": "ch1", "field": "flow", "index": 3, "value": {"type": "p", "id": "p99", "text": "…"}}
{"op": "insert", "after": "p12", "value": {"type": "p", "id": "p99", "text": "…"}}
{"op": "remove", "id": "p99"}
{"op": "move", "id": "fig2", "to": {"id": "ch2"}, "field": "flow", "index": 0}
{"op": "insert-text", "id": "p12", "offset": 40, "text": "very "}
{"op": "delete-text", "id": "p12", "start": 40, "end": 45}
{"op": "set", "at": "/design/styles/body", "path": "/size", "value": 10.5}
```

| Operation | What it does |
|---|---|
| `set` | sets a field: `path` is a JSON pointer inside the node (`"/style"`, `"/set/size"`). Missing objects on the way are created. |
| `unset` | removes a field |
| `insert` | inserts `value` into the array `field` of the node (at `index`, or at the end without one), or next to a sibling: `before` or `after` an id |
| `remove` | removes the node from the array that holds it |
| `move` | moves the node into the array `field` of `to`, at `index` |
| `insert-text` | inserts `text` at a byte `offset` of a paragraph’s or heading’s plain text |
| `delete-text` | deletes the byte range `start..end` of the plain text |

### Addressing a node

A node is addressed by **`id`**, or by **`at`**, a JSON pointer from the document’s root (`""` is the root itself, `"/design/styles/body"` a style).

`insert` with `before` or `after` a sibling’s id keeps its place when the list around it changes, so an edit replayed on a document recomposed from a changed manuscript still lands where it was made. **Prefer ids to `index` and `at` in anything you keep.**

### Text offsets

`insert-text` and `delete-text` count **bytes of the node’s plain text** (UTF-8), not characters: `é` is two bytes, `Մ` two, `—` three. Marks on the words are kept.

## On the command line

`apply` takes a file with one operation or a list of them:

```console
$ matenadar apply book.json ops.json -o next.json
```

It applies the operations in order, writes the document to `-o` (or **back to the input file** when there is no `-o`), and prints a list with one change per operation:

```json
[
  {
    "inverse": {"op": "delete-text", "id": "p12", "start": 40, "end": 45},
    "changed_pages": [12, 13],
    "page_count": 48,
    "compile": 9.6
  }
]
```

| Field | Meaning |
|---|---|
| `inverse` | the operation that undoes this one |
| `changed_pages` | 1-based numbers of the pages whose layout differs, including new pages |
| `page_count` | the number of pages after the operation |
| `compile` | the layout time, in milliseconds |

To undo a batch, apply its inverses in **reverse order**. An operation that fails stops the command with an error (exit status 2) and nothing is written.

## In `serve`

The [`serve` loop](/docs/guides/agents/#serve) keeps one document open and takes operations as JSON lines:

```json
{"id": 7, "method": "apply", "params": {"ops": [{"op": "set", "id": "p12", "path": "/style", "value": "lede"}]}}
```

The reply’s `result` is the same list of changes. The document stays in memory until you `save` it.

## In WebAssembly

The engine keeps an undo history: each call is one step, laid out once.

```js
const done = mt.apply([{ op: 'set', id: 'p12', path: '/style', value: 'lede' }]);
done.changed_pages; // pages to draw again
mt.undo();
mt.redo();
```

Edits return an `Edited` result: `changed_pages`, `page_count`, `diagnostics` (checked again only on the pages that changed), `can_undo` and `can_redo`. The editor’s helpers (`insertText`, `setBlock`, `moveFrame`, `breakBefore`…) are operations underneath. See [the WebAssembly reference](/docs/reference/wasm/#editing).

### The patch

`patch()` gives the edits since the document opened (or the last `rebase()`), every one addressed by id:

- **`words`**: changes to paragraphs’, headings’, figures’ and lists’ words, for the manuscript, with the ids `touched`;
- **`layout`**: everything else, each field folded to its last setting, and settings returned to where they started dropped.

Replayed on the opened document, words first, they give the same pages. With the command line:

```console
$ matenadar apply book.json words.json -o book.json
$ matenadar apply book.json layout.json -o book.json
```

`apply(ops, {part: "layout"})` keeps a whole batch in `patch.layout`, including existing paragraphs moved into new frames, for design-only compositions. The default classifies manuscript moves and text edits into `patch.words`. The caller of the design-only form must preserve the source words: the option controls how edits are kept, not what they may change. Undo and redo retain the batch’s classification.

After saving a patch, `rebase()` makes the document as it stands the patch’s base, and forgets the undo history.
