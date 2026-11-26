// Matenadar in the browser, Web Workers, and edge runtimes (zero `node:*` imports).
//
//   import { Matenadar } from "@matenadar/wasm";
//   const mt = await Matenadar.load(); // or pass a CDN URL / fetch() Response

import { createMatenadar } from "./core.js";
import * as io from "./io-browser.js";

export const Matenadar = createMatenadar(io, new URL("./matenadar.wasm", import.meta.url));
