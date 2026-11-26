// Matenadar in Node, Bun, Deno (and anywhere with filesystem or fetch access).
// For browser bundlers, package.json maps "." -> matenadar.browser.js so no
// `node:*` modules enter client bundles.
//
//   import { Matenadar } from "@matenadar/wasm";
//   const mt = await Matenadar.load();
//   const { pages, diagnostics } = await mt.openFrom(doc, { root: "./book", fonts: ["./fonts"] });
//   const pdf = mt.pdf();

import { createMatenadar } from "./core.js";
import * as io from "./io-node.js";

export const Matenadar = createMatenadar(io, new URL("./matenadar.wasm", import.meta.url));
