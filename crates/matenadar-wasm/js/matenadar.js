// Matenadar in the browser (or Node, Deno, Bun): one document laid out in
// memory, edited with operations, rendered to SVG, PNG or PDF.
//
//   import { Matenadar } from "./matenadar.js";
//   const mt = await Matenadar.load(fetch("matenadar.wasm"));
//   mt.addFont(await (await fetch("fonts/Body-Regular.otf")).arrayBuffer());
//   mt.addFile("images/plate.jpg", await (await fetch("images/plate.jpg")).arrayBuffer());
//   const { pages } = mt.open(doc);
//   page.innerHTML = mt.svg(1);
//   const [change] = mt.apply([{ op: "insert-text", id: "p1", offset: 0, text: "Once " }]);
//   for (const n of change.changed_pages) redraw(n);
//
// Calls are synchronous; run them in a worker to keep a page responsive.

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

  /** Adds a font file (OTF, TTF or a collection). Returns how many faces it holds. */
  addFont(data) {
    return this.#done(this.#x.mt_font(...this.#put(bytes(data))), "json").faces;
  }

  /** Adds or replaces a picture or drawing at `path` under the document's asset root. */
  addFile(path, data) {
    this.#done(this.#x.mt_file(...this.#put(encoder.encode(path)), ...this.#put(bytes(data))), "json");
  }

  /** Lays a document out. Returns `{pages, problems, warnings}`. */
  open(doc) {
    return this.#call("open", { doc });
  }

  /** Applies operations in order. Returns one change per operation: `{inverse, changed_pages, page_count}`. */
  apply(ops) {
    return this.#call("apply", { ops });
  }

  /** Page `page` (from 1) as SVG text. */
  svg(page, { bleed = false } = {}) {
    return this.#call("svg", { page, bleed }, "text");
  }

  /** Page `page` as PNG bytes. */
  png(page, { dpi = 100, bleed = false } = {}) {
    return this.#call("png", { page, dpi, bleed }, "bytes");
  }

  /** The whole document as PDF bytes. */
  pdf() {
    return this.#call("pdf", {}, "bytes");
  }

  /** Print checks: `{safe: "0.25in", min_ppi: 300}`. Returns the issues found. */
  check(options = {}) {
    return this.#call("check", options);
  }

  /** Where everything landed, per page (all pages, or those listed). */
  layout(pages) {
    return this.#call("layout", pages ? { pages } : {});
  }

  /** The node and text offset under a point (points from the trim's top-left), or null. */
  locate(page, x, y) {
    return this.#call("locate", { page, x, y });
  }

  /** Where a node starts, `{page, x, y}`, or null. */
  position(id) {
    return this.#call("position", { id });
  }

  /** The document as it stands after the operations applied. */
  document() {
    return this.#call("document", {});
  }

  /** The font families given. */
  fonts() {
    return this.#call("fonts", {});
  }

  close() {
    this.#call("close", {});
  }

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
