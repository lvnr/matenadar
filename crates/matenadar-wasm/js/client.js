// The page's side of the engine: starts worker.js and gives every method
// of Matenadar (matenadar.js) as a promise, in call order.
//
//   import { MatenadarClient } from "./client.js";
//   const mt = await MatenadarClient.start({ base: "/matenadar/<commit>/" });
//   await mt.addFont(await (await fetch(url)).arrayBuffer());
//   const { pages } = await mt.open(doc);
//   const svg = await mt.svg(1);
//
// `base` is the folder holding worker.js, matenadar.js and matenadar.wasm.

const METHODS = [
  "assets", "addFont", "addFile", "open", "fonts", "close",
  "pages", "spreads", "svg", "png", "rgba", "pdf", "fingerprint", "fontsUsed",
  "hit", "rects", "node", "lines", "locate", "position", "layout", "document",
  "caret", "offsetAt", "selectionRects", "textLines", "source",
  "apply", "setWords", "insertText", "deleteText", "insertBlock", "removeBlock",
  "splitBlock", "joinBlock", "markText",
  "setFields", "setPicture", "setBlock", "moveFrame", "breakBefore", "hideOnPage", "nudge", "undo", "redo",
  "diagnostics", "check", "patch", "rebase",
];

export class MatenadarClient {
  #worker;
  #next = 1;
  #waiting = new Map();

  /** Starts the worker and loads the engine into it. */
  static async start({ base, worker } = {}) {
    const root = new URL(base ?? "./", typeof location === "undefined" ? import.meta.url : location.href);
    const w = worker ?? new Worker(new URL("worker.js", root), { type: "module" });
    const client = new MatenadarClient(w);
    await client.call("load", [new URL("matenadar.wasm", root).href]);
    return client;
  }

  constructor(worker) {
    this.#worker = worker;
    worker.onmessage = (event) => {
      const { id, result, error } = event.data;
      const pending = this.#waiting.get(id);
      if (!pending) return;
      this.#waiting.delete(id);
      if (error !== undefined) pending.reject(new Error(error));
      else pending.resolve(result);
    };
    worker.onerror = (event) => {
      const error = new Error(event.message || "the engine's worker failed");
      for (const pending of this.#waiting.values()) pending.reject(error);
      this.#waiting.clear();
    };
  }

  /** Calls a method by name. Byte arguments (fonts, pictures) are transferred, not copied. */
  call(method, args = []) {
    const id = this.#next++;
    const transfer = args.filter((a) => a instanceof ArrayBuffer || ArrayBuffer.isView(a)).map((a) => (a instanceof ArrayBuffer ? a : a.buffer));
    return new Promise((resolve, reject) => {
      this.#waiting.set(id, { resolve, reject });
      this.#worker.postMessage({ id, method, args }, transfer);
    });
  }

  terminate() {
    this.#worker.terminate();
  }
}

for (const name of METHODS) {
  MatenadarClient.prototype[name] = function (...args) {
    return this.call(name, args);
  };
}
