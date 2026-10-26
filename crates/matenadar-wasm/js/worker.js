// The engine in a worker, so layout never blocks the page. client.js
// starts it and forwards calls: {id, method, args} in, {id, result} or
// {id, error} out. Bytes travel as transferred buffers.

import { Matenadar } from "./matenadar.js";

let mt = null;

self.onmessage = async (event) => {
  const { id, method, args = [] } = event.data;
  try {
    if (method === "load") {
      mt = await Matenadar.load(fetch(args[0]));
      self.postMessage({ id, result: true });
      return;
    }
    if (!mt) throw new Error("the engine is still loading");
    if (method.startsWith("#") || typeof mt[method] !== "function") throw new Error(`no method "${method}"`);
    const result = mt[method](...args);
    self.postMessage({ id, result }, transferables(result));
  } catch (error) {
    self.postMessage({ id, error: String(error?.message ?? error) });
  }
};

function transferables(result) {
  if (result instanceof Uint8Array) return [result.buffer];
  if (result && result.data instanceof Uint8ClampedArray) return [result.data.buffer];
  return [];
}
