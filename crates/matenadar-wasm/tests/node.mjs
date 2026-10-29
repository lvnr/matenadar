// Runs the WebAssembly build in Node: gives it a font and a picture, lays a
// document out, edits it and renders it. `scripts/wasm.sh --test` runs it.
//
//   node crates/matenadar-wasm/tests/node.mjs dist/matenadar.wasm FONT.otf…
//
// With MATENADAR_BIN set to the command line's binary, it also checks that
// the binary lays the edited document out to the same pages.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { Matenadar } from "../js/matenadar.js";

const here = dirname(fileURLToPath(import.meta.url));
const [wasm, ...fontFiles] = process.argv.slice(2);
assert.ok(wasm && fontFiles.length, "usage: node.mjs MODULE.wasm FONT…");

const mt = await Matenadar.load(readFileSync(wasm));
for (const font of fontFiles) assert.ok(mt.addFont(readFileSync(font)) >= 1, "the font has a face");
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

const change = mt.apply([{ op: "insert-text", id: "p1", offset: 0, text: "Today " }]);
assert.deepEqual(change.changed_pages, [1]);
assert.ok(change.can_undo && !change.can_redo);
assert.match(JSON.stringify(mt.document()), /Today The engine/);

const report = mt.layout([1]);
assert.equal(report.length, 1);
assert.ok(JSON.stringify(report).includes("p1"));

const issues = mt.check({ min_ppi: 300 });
assert.ok(Array.isArray(issues));
assert.ok(issues.some((i) => JSON.stringify(i).includes("ph")), "an 8-pixel picture an inch wide is too coarse for print");

assert.throws(() => mt.apply([{ op: "insert-text", id: "nope", offset: 0, text: "x" }]));
assert.equal(mt.open(doc).pages, 2, "a failed operation leaves the engine usable");

// ---- The page editor ----

const pages = mt.pages();
assert.equal(pages.length, 2);
assert.equal(pages[0].side, "recto");
assert.deepEqual(pages[0].trim, { w: 360, h: 576 });
assert.equal(pages[0].margins.inside, 54);
assert.deepEqual(mt.spreads(), [[1], [2]]);

const px = mt.rgba(2, { scale: 1 });
assert.equal(px.width, 360);
assert.equal(px.data.length, 360 * 576 * 4);

const p1 = mt.position("p1");
const hit1 = mt.hit(1, p1.x + 20, p1.y + 6);
assert.equal(hit1.id, "p1");
assert.ok(hit1.offset >= 0);
assert.equal(mt.hit(2, 108, 108).id, "ph");

const before = mt.fingerprint();
const spaced = mt.setBlock("p1", { space_before: "36pt", align: "center" });
assert.deepEqual(spaced.changed_pages, [1]);
assert.ok(spaced.can_undo);
assert.equal(mt.node("p1").set.align, "center");
const undone = mt.undo();
assert.ok(undone.can_redo);
assert.deepEqual(mt.fingerprint(), before, "undo restores the pages");
mt.redo();

mt.setWords("p1", ["New words for ", { text: "the first", em: true }, " paragraph."]);
assert.match(JSON.stringify(mt.node("p1").content), /the first/);
mt.setPicture("ph", { crop: { x: 0.25, w: 0.5 }, fit: "cover" });
mt.hideOnPage("p1", ["folio-none"]);
mt.breakBefore("p1", "recto");
assert.equal(mt.position("p1").page % 2, 1);

const patch = mt.patch();
assert.deepEqual(patch.words.map((op) => `${op.op} ${op.path}`), ["unset /text", "set /content"]);
assert.deepEqual(patch.touched, ["p1"]);
assert.deepEqual(patch.layout.map((op) => `${op.op} ${op.path ?? ""}`.trim()), [
  "set /set/space_before",
  "set /set/align",
  "set /crop",
  "set /fit",
  "set /on_page/p1",
  "insert",
]);
assert.ok(Array.isArray(mt.diagnostics()));
assert.ok(mt.fontsUsed().some((f) => f.family === family && f.pages.includes(1)));

// ---- Words on the page ----

const c0 = mt.caret("p1", 0);
assert.equal(c0.page, mt.position("p1").page);
assert.ok(c0.h > 5);
const c4 = mt.caret("p1", 4);
assert.ok(c4.x > c0.x && c4.y === c0.y, "further into the words, further right");
assert.equal(mt.offsetAt("p1", c4.page, c4.x + 0.01, c4.y + c4.h / 2), 4);
assert.equal(mt.textLines("p1").length, mt.lines("p1"));
assert.equal(mt.selectionRects("p1", 0, 3).length, 1);
const words = mt.node("p1").content;
assert.ok(mt.splitBlock("p1", 4, "p1b").can_undo);
assert.equal(mt.node("p1").text, "New ");
const joined = mt.joinBlock("p1b");
assert.deepEqual([joined.into, joined.offset], ["p1", 4]);
assert.deepEqual(mt.node("p1").content, words, "split and joined, the words are as they were");
mt.markText("p1", 0, 3, "strong", true);
assert.deepEqual(mt.node("p1").content[0], { text: "New", strong: true });
const source = mt.source();
assert.ok(source.prelude > 1000 && source.text.length > source.prelude);
const p1node = source.nodes.find((n) => n.id === "p1");
assert.ok(p1node && source.text.slice(p1node.start, p1node.end).includes("<p1>"), "the source map finds p1 in the program");

if (process.env.MATENADAR_BIN) {
  // The same document and edits, laid out by the command line, give the same pages.
  const { execFileSync } = await import("node:child_process");
  const { mkdtempSync, writeFileSync, mkdirSync, copyFileSync } = await import("node:fs");
  const { tmpdir } = await import("node:os");
  const dir = mkdtempSync(join(tmpdir(), "matenadar-parity-"));
  mkdirSync(join(dir, "images"));
  mkdirSync(join(dir, "fonts"));
  copyFileSync(join(here, "../../matenadar-typst/tests/black.png"), join(dir, "images/black.png"));
  // Only the fonts the browser has (the command line also has Typst's own faces).
  fontFiles.forEach((font, i) => copyFileSync(font, join(dir, `fonts/font-${i}.otf`)));
  writeFileSync(join(dir, "doc.json"), JSON.stringify(mt.document()));
  const out = JSON.parse(execFileSync(process.env.MATENADAR_BIN, ["fingerprint", join(dir, "doc.json"), "--root", dir, "--fonts", join(dir, "fonts"), "--no-system-fonts"], { encoding: "utf8" }));
  assert.deepEqual(out.fingerprints, mt.fingerprint(), "the command line lays out the same pages");
  console.log("parity: the command line's fingerprints match");
}

console.log(`ok: ${opened.pages} pages, svg ${svg.length} chars, pdf ${pdf.length} bytes`);
