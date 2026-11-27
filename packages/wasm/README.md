# @matenadar/wasm

WebAssembly build of [matenadar](https://github.com/lvnr/matenadar)—lay out, edit, inspect, check, and render documents to SVG, PNG, PDF, and EPUB 3 in browsers, Node, Bun, and Deno.

## Install

```bash
npm install @matenadar/wasm
```

## Usage (Node / Bun / Deno)

```js
import { Matenadar } from "@matenadar/wasm";

const mt = await Matenadar.load();
const { pages, diagnostics } = await mt.openFrom(doc, {
  root: "./book",
  fonts: ["./fonts"],
});

const svg = mt.svg(1);
const pdf = mt.pdf();
const epub = mt.epub({ layout: "reflowable" });
```

## Usage (Browser with Web Worker)

```js
import { MatenadarClient } from "@matenadar/wasm/client";

const mt = await MatenadarClient.start();
// Or serve matenadar.wasm from jsDelivr while keeping the worker same-origin:
// const mt = await MatenadarClient.start({
//   wasm: "https://cdn.jsdelivr.net/npm/@matenadar/wasm@0.3.0/dist/matenadar.wasm",
// });

await mt.addFont(await (await fetch("/fonts/LibertinusSerif-Regular.otf")).arrayBuffer());
const { pages } = await mt.open(doc);
const svg = await mt.svg(1);
```

See the [documentation](https://matenadar.levships.com/docs/reference/wasm/) and [main repository](https://github.com/lvnr/matenadar) for the full document model and editing API.

## License

Apache-2.0
