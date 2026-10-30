// Matenadar in the browser (or Node, Deno, Bun): a document laid out in
// memory, edited the way a page view edits it, rendered to SVG, pixels,
// PNG or PDF. Calls are synchronous: in a page, run this in a worker
// (worker.js) and talk to it through client.js.
//
//   import { Matenadar } from "./matenadar.js";
//   const mt = await Matenadar.load(fetch("matenadar.wasm"));
//   const { fonts, files } = mt.assets(doc);
//   for (const font of fontFiles) mt.addFont(font);
//   for (const path of files) mt.addFile(path, await fetchBytes(path));
//   const { pages, diagnostics } = mt.open(doc);
//   const svg = mt.svg(1);
//   const hit = mt.hit(1, x, y);                    // points from the trim's corner
//   const done = mt.setFields(hit.id, { space_before: "12pt" }, { style: true });
//   for (const n of done.changed_pages) redraw(n);
//   mt.undo();
//   const { words, layout } = mt.patch();
//
// Types: matenadar.d.ts.

const encoder = new TextEncoder();
const decoder = new TextDecoder();

export class Matenadar {
  #x;

  /** Loads the module from a Response (or a promise of one), bytes or a compiled module. */
  static async load(source) {
    source = await source;
    let instance;
    if (source instanceof WebAssembly.Module) {
      instance = await WebAssembly.instantiate(source, {});
    } else if (typeof Response !== "undefined" && source instanceof Response) {
      ({ instance } = await WebAssembly.instantiate(await source.arrayBuffer(), {}));
    } else {
      ({ instance } = await WebAssembly.instantiate(source, {}));
    }
    return new Matenadar(instance.exports);
  }

  constructor(exports) {
    this.#x = exports;
  }

  // ---- Loading ----

  /** The font folders and files a document needs, as it names them under its root. */
  assets(doc) {
    return this.#call("assets", { doc });
  }

  /** Adds a font file (OTF, TTF or a collection). Returns how many faces it holds. */
  addFont(data) {
    return this.#done(this.#x.mt_font(...this.#put(bytes(data))), "json").faces;
  }

  /** Adds or replaces a picture or drawing at `path` under the document's root. */
  addFile(path, data) {
    this.#done(this.#x.mt_file(...this.#put(encoder.encode(path)), ...this.#put(bytes(data))), "json");
  }

  /** Lays a document out. `check`: `{safe: "0.25in", gutter: "0.5in", min_ppi: 300}`. */
  open(doc, check) {
    return this.#call("open", check ? { doc, check } : { doc });
  }

  /** A book kind's whole document from a spec (`picture-book`…); see docs/model.md, Templates. */
  template(kind, spec) {
    return this.#call("template", { kind, spec });
  }

  /** The font families given. */
  fonts() {
    return this.#call("fonts", {});
  }

  close() {
    this.#call("close", {});
  }

  // ---- Pages ----

  /** Every page's trim, bleed, side, master and margins, in points. */
  pages() {
    return this.#call("pages", {});
  }

  /** Pages as they face each other: [[1], [2, 3], …]. */
  spreads() {
    return this.#call("spreads", {});
  }

  /** A page as SVG text. */
  svg(page, { bleed = false } = {}) {
    return this.#call("svg", { page, bleed }, "text");
  }

  /** A page as PNG bytes; `scale` is pixels per point (or give `dpi`). */
  png(page, { scale, dpi, bleed = false } = {}) {
    return this.#call("png", { page, scale, dpi, bleed }, "bytes");
  }

  /** A page as pixels for a canvas: `{width, height, data}` (straight RGBA). */
  rgba(page, { scale = 1, bleed = false } = {}) {
    const out = this.#call("rgba", { page, scale, bleed }, "bytes");
    const head = new DataView(out.buffer, out.byteOffset, 8);
    return { width: head.getUint32(0, true), height: head.getUint32(4, true), data: new Uint8ClampedArray(out.buffer, out.byteOffset + 8) };
  }

  /** The whole document as PDF bytes. */
  pdf() {
    return this.#call("pdf", {}, "bytes");
  }

  /** A hash per page; `matenadar fingerprint` gives the same for the same pages. */
  fingerprint() {
    return this.#call("fingerprint", {});
  }

  /** The faces the layout drew with, and on which pages; `matenadar fonts --used` gives the server's. */
  fontsUsed() {
    return this.#call("fonts_used", {});
  }

  // ---- Finding ----

  /** What is under a point, or null: `{id, kind, page, rect, offset?, element?, ancestors}`. */
  hit(page, x, y) {
    return this.#call("hit", { page, x, y });
  }

  /** A node's extent on each page it reaches. */
  rects(id) {
    return this.#call("rects", { id });
  }

  /** A node as JSON, or null. */
  node(id) {
    return this.#call("node", { id });
  }

  /** How many lines a paragraph sets in. */
  lines(id) {
    return this.#call("lines", { id });
  }

  /** The node and byte offset under a point, or null. */
  locate(page, x, y) {
    return this.#call("locate", { page, x, y });
  }

  /** Where a node starts, `{page, x, y}`, or null. */
  position(id) {
    return this.#call("position", { id });
  }

  /** Every node on every page (or those listed), with its extent. */
  layout(pages) {
    return this.#call("layout", pages ? { pages } : {});
  }

  /** The document as it stands. */
  document() {
    return this.#call("document", {});
  }

  // ---- Words on the page (offsets are bytes of the node's plain text, UTF-8) ----

  /** The caret before a byte of a node's words, `{page, x, y, h}` (y is the line's top), or null. `upstream` keeps it at the end of a line that breaks there. */
  caret(id, offset, { upstream = false } = {}) {
    return this.#call("caret", { id, offset, upstream });
  }

  /** The place in a node's words nearest a point on a page, or null. */
  offsetAt(id, page, x, y) {
    return this.#call("offset_at", { id, page, x, y });
  }

  /** A rectangle per line over the words `start..end`: `[{page, rect}]`. */
  selectionRects(id, start, end) {
    return this.#call("selection_rects", { id, start, end });
  }

  /** The lines a node's words set in: `[{page, rect, start, end}]`. */
  textLines(id) {
    return this.#call("text_lines", { id });
  }

  /** The program the engine generated, `{text, prelude, nodes: [{id, kind, start, end}]}`, in UTF-16 offsets. */
  source() {
    return this.#call("source", {});
  }

  // ---- Editing: each call is one step to undo ----

  /** Matenadar operations, applied together. */
  apply(ops) {
    return this.#call("apply", { ops: Array.isArray(ops) ? ops : [ops] });
  }

  /** A paragraph's or heading's words, as runs (marks are run fields). */
  setWords(id, content) {
    return this.#call("set_words", { id, content });
  }

  insertText(id, offset, text) {
    return this.apply({ op: "insert-text", id, offset, text });
  }

  deleteText(id, start, end) {
    return this.apply({ op: "delete-text", id, start, end });
  }

  /** A block placed after (or `{before}`) a sibling: `insertBlock({after: "p-1"}, block)`. */
  insertBlock(where, block) {
    return this.apply({ op: "insert", ...where, value: block });
  }

  removeBlock(id) {
    return this.apply({ op: "remove", id });
  }

  /** Splits a paragraph or heading at a byte of its words; the words after go to a new block `newId` after it. With `end`, the words `offset..end` go first (Enter over a selection), in the same step. */
  splitBlock(id, offset, newId, end = offset) {
    return this.#call("split_block", { id, offset, end, new_id: newId });
  }

  /** Joins a paragraph or heading to the one before it: `{into, offset, ...edited}`, where its words now start. */
  joinBlock(id) {
    return this.#call("join_block", { id });
  }

  /** Marks the words `start..end`: `em`, `strong`, `span`, `color`, `size`, `tracking`, `case`…; null takes the mark off. */
  markText(id, start, end, field, value) {
    return this.#call("mark_text", { id, start, end, field, value });
  }

  /** Fields of a node (null removes one). With `{style: true}`, a paragraph's or heading's style overrides. */
  setFields(id, fields, { style = false } = {}) {
    return this.#call("set_fields", { id, fields, style });
  }

  /** A picture: `src`, `crop` ({x, y, w, h} fractions), `fit`, `focus`, `w`, `h`, `x`, `y`, `opacity`… */
  setPicture(id, fields) {
    return this.setFields(id, fields);
  }

  /** Spacing, alignment and keeps: `space_before`, `space_after`, `align`, `keep_with_next`, `keep_together`… */
  setBlock(id, fields) {
    return this.setFields(id, fields, { style: true });
  }

  /** A placed element's frame: `{x, y, w, h, rotate}`. */
  moveFrame(id, frame) {
    return this.setFields(id, frame);
  }

  /** Starts a node on a new page ("page", "recto", "verso"), or with null takes such a break back. */
  breakBefore(id, to = "page") {
    return this.#call("break_before", { id, to });
  }

  /** Leaves master elements (a folio, a running head) off the page where the node lands; [] puts them back. */
  hideOnPage(id, hide) {
    return this.#call("hide_on_page", { id, hide });
  }

  /** Sets a paragraph a line shorter (-1) or longer (+1) by the least tracking that does it. */
  nudge(id, lines) {
    return this.#call("nudge", { id, lines });
  }

  undo() {
    return this.#call("undo", {});
  }

  redo() {
    return this.#call("redo", {});
  }

  // ---- Results ----

  /** Issues now: overflow, widows, orphans, text in the margin, missing glyphs, low resolution… */
  diagnostics() {
    return this.#call("diagnostics", {});
  }

  /** A full check with other settings. */
  check(options = {}) {
    return this.#call("check", options);
  }

  /** The edits since the document opened (or the last rebase): `{words, touched, layout}`. */
  patch() {
    return this.#call("patch", {});
  }

  /** Makes the document as it stands the patch's base (after saving the patch); forgets undo. */
  rebase() {
    this.#call("rebase", {});
  }

  // ---- The protocol ----

  #call(method, params, kind = "json") {
    const request = encoder.encode(JSON.stringify({ method, params }));
    return this.#done(this.#x.mt_call(...this.#put(request)), kind);
  }

  #put(data) {
    const ptr = this.#x.mt_alloc(data.length) >>> 0;
    new Uint8Array(this.#x.memory.buffer, ptr, data.length).set(data);
    return [ptr, data.length];
  }

  #done(status, kind) {
    const ptr = this.#x.mt_result_ptr() >>> 0;
    const out = new Uint8Array(this.#x.memory.buffer, ptr, this.#x.mt_result_len() >>> 0).slice();
    if (status !== 0) throw new Error(decoder.decode(out));
    if (kind === "bytes") return out;
    const text = decoder.decode(out);
    return kind === "text" ? text : JSON.parse(text);
  }
}

function bytes(data) {
  if (data instanceof Uint8Array) return data;
  if (ArrayBuffer.isView(data)) return new Uint8Array(data.buffer, data.byteOffset, data.byteLength);
  return new Uint8Array(data);
}
