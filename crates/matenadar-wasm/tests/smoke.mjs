// A sample book through the WebAssembly build in Node, as a server would run
// it: open the document from its folder (its own fonts and pictures), lay it
// out, render a page, export the PDF and both EPUBs. Prints each step's time,
// the sizes, and the peak memory (the module's and the process's).
//
//   node crates/matenadar-wasm/tests/smoke.mjs dist/matenadar.wasm samples/picture-book [--fonts DIR|FILE]… [--out DIR] [--json]
//
// `--fonts` adds faces the document doesn't carry in its own font folders
// (Typst's Libertinus, or the system faces the command line found).

import assert from "node:assert/strict";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { Matenadar } from "../js/matenadar.js";

const args = process.argv.slice(2);
const [wasm, folder] = args;
assert.ok(wasm && folder, "usage: smoke.mjs MODULE.wasm SAMPLE_FOLDER [--fonts PATH]… [--out DIR] [--json]");
const fonts = args.flatMap((a, i) => (a === "--fonts" ? [args[i + 1]] : []));
const out = args.includes("--out") ? args[args.indexOf("--out") + 1] : null;
const asJson = args.includes("--json");

const times = {};
const memory = {};
let mt = null;
const time = async (name, f) => {
  const t0 = performance.now();
  const r = await f();
  times[name] = Math.round(performance.now() - t0);
  if (mt) memory[name] = +(mt.memory() / 2 ** 20).toFixed(1);
  return r;
};

mt = await time("load", async () => Matenadar.load(readFileSync(wasm)));
const doc = JSON.parse(readFileSync(join(folder, "doc.json"), "utf8"));
const opened = await time("open", () => mt.openFrom(doc, { root: folder, fonts }));
assert.ok(opened.pages > 0, "the document lays out");
const svg = await time("svg_page_1", () => mt.svg(1));
assert.match(svg, /^<svg/);
const png = await time("png_page_1_150dpi", () => mt.png(1, { dpi: 150 }));
assert.deepEqual([...png.slice(0, 4)], [0x89, 0x50, 0x4e, 0x47]);
const pdf = await time("pdf", () => mt.pdf());
assert.equal(new TextDecoder().decode(pdf.slice(0, 5)), "%PDF-");
const reflowable = await time("epub_reflowable", () => mt.epub({ layout: "reflowable" }));
const fixed = await time("epub_fixed", () => mt.epub({ layout: "fixed" }));
for (const e of [reflowable, fixed]) assert.equal(new TextDecoder().decode(e.slice(30, 38)), "mimetype", "an EPUB opens with its mimetype");
const issues = await time("check", () => mt.check({}));

const report = {
  sample: folder,
  pages: opened.pages,
  missing: opened.missing,
  problems: opened.problems.length,
  issues: issues.length,
  ms: times,
  module_mb_after: memory,
  bytes: { pdf: pdf.length, epub_reflowable: reflowable.length, epub_fixed: fixed.length },
  memory_mb: { module: +(mt.memory() / 2 ** 20).toFixed(1), process_peak_rss: +(process.resourceUsage().maxRSS / 1024).toFixed(1) },
};
if (out) {
  mkdirSync(out, { recursive: true });
  writeFileSync(join(out, "book.pdf"), pdf);
  writeFileSync(join(out, "book.epub"), reflowable);
  writeFileSync(join(out, "book-fixed.epub"), fixed);
}
if (asJson) {
  console.log(JSON.stringify(report));
} else {
  console.log(`${folder}: ${report.pages} pages${report.missing.length ? `, ${report.missing.length} files missing` : ""}`);
  for (const [k, v] of Object.entries(times)) console.log(`  ${k.padEnd(18)} ${String(v).padStart(6)} ms   module ${memory[k]} MB`);
  console.log(`  pdf ${(pdf.length / 1e6).toFixed(1)} MB, epub ${(reflowable.length / 1e6).toFixed(1)} MB, fixed epub ${(fixed.length / 1e6).toFixed(1)} MB`);
  console.log(`  memory: module ${report.memory_mb.module} MB, process peak ${report.memory_mb.process_peak_rss} MB`);
}
