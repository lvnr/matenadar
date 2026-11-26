// Browser / Web Worker / edge IO adapter (zero node:* imports so bundlers stay clean).

export function isUrl() {
  return true;
}

function toUrl(where) {
  return where instanceof URL ? where : new URL(String(where), globalThis.location?.href);
}

export async function loadWasmBytes(source, defaultUrl) {
  source = await (source ?? defaultUrl);
  if (source instanceof WebAssembly.Module) return { module: source };
  if (typeof source === "string" || source instanceof URL) {
    source = await fetch(toUrl(source));
  }
  if (typeof Response !== "undefined" && source instanceof Response) {
    if (!source.ok) throw new Error(`${source.url || "matenadar.wasm"}: ${source.status}`);
    if (
      typeof WebAssembly.instantiateStreaming === "function" &&
      /\bapplication\/wasm\b/i.test(source.headers.get("content-type") ?? "")
    ) {
      return { response: source };
    }
    return { bytes: await source.arrayBuffer() };
  }
  return { bytes: source };
}

export async function under(root, path) {
  return new URL(path, toUrl(String(root).endsWith("/") ? root : `${root}/`));
}

export async function read(where) {
  const res = await fetch(toUrl(where));
  if (!res.ok) throw new Error(`${where}: ${res.status}`);
  return new Uint8Array(await res.arrayBuffer());
}

export async function fontsAt(source, toBytes) {
  if (source instanceof Uint8Array || source instanceof ArrayBuffer || ArrayBuffer.isView(source)) {
    return [toBytes(source)];
  }
  return [await read(source)];
}
