// Runs the WebAssembly build in Node: gives it a font and a picture, lays a
// document out, edits it and renders it. `scripts/wasm.sh --test` runs it.
//
//   node crates/matenadar-wasm/tests/node.mjs dist/matenadar.wasm FONT.otf

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { Matenadar } from "../js/matenadar.js";

const here = dirname(fileURLToPath(import.meta.url));
const [wasm, font] = process.argv.slice(2);
assert.ok(wasm && font, "usage: node.mjs MODULE.wasm FONT");

const mt = await Matenadar.load(readFileSync(wasm));
assert.ok(mt.addFont(readFileSync(font)) >= 1, "the font has a face");
const family = mt.fonts()[0];
mt.addFile("images/black.png", readFileSync(join(here, "../../matenadar-typst/tests/black.png")));

assert.throws(() => mt.svg(1), /open a document first/);

const doc = {
  matenadar: "1",
  meta: { title: "In the browser" },
  design: {
    base: { font: family, size: 11, line_height: 1.4 },
    masters: { page: { width: "5in", height: "8in", margins: { top: 54, bottom: 54, inside: 54, outside: 54 } } },
  },
  content: [
    {
      type: "section",
      id: "s1",
      master: "page",
      flow: [
        { type: "h", id: "h1", level: 1, text: "A page laid out in WebAssembly" },
        { type: "p", id: "p1", text: "The engine runs where the editor does, with no server between them." },
      ],
    },
    {
      type: "page",
      id: "plate",
      master: "page",
      elements: [
        { type: "image", id: "ph", src: "images/black.png", x: 72, y: 72, w: 72, h: 72, fit: "stretch", opacity: 0.5 },
        { type: "qr", id: "code", data: "https://example.com", x: 72, y: 180, w: 72, h: 72 },
      ],
    },
  ],
};

const opened = mt.open(doc);
assert.equal(opened.pages, 2);
assert.deepEqual(opened.problems, []);

const svg = mt.svg(1);
assert.match(svg, /^<svg/);
assert.ok(svg.length > 1000);

const png = mt.png(2, { dpi: 72 });
assert.deepEqual([...png.slice(0, 4)], [0x89, 0x50, 0x4e, 0x47]);

const pdf = mt.pdf();
assert.equal(new TextDecoder().decode(pdf.slice(0, 5)), "%PDF-");

const at = mt.position("p1");
assert.equal(at.page, 1);
const hit = mt.locate(1, at.x + 20, at.y + 6);
assert.equal(hit?.node, "p1");

const [change] = mt.apply([{ op: "insert-text", id: "p1", offset: 0, text: "Today " }]);
assert.deepEqual(change.changed_pages, [1]);
assert.equal(change.inverse.op, "delete-text");
assert.match(JSON.stringify(mt.document()), /Today The engine/);

const report = mt.layout([1]);
assert.equal(report.length, 1);
assert.ok(JSON.stringify(report).includes("p1"));

const issues = mt.check({ min_ppi: 300 });
assert.ok(Array.isArray(issues));
assert.ok(issues.some((i) => JSON.stringify(i).includes("ph")), "an 8-pixel picture an inch wide is too coarse for print");

assert.throws(() => mt.apply([{ op: "insert-text", id: "nope", offset: 0, text: "x" }]));
assert.equal(mt.open(doc).pages, 2, "a failed operation leaves the engine usable");

console.log(`ok: ${opened.pages} pages, svg ${svg.length} chars, pdf ${pdf.length} bytes`);
