// Node / Bun / Deno IO adapter: local files and font folders, plus HTTP(S) URLs.

const fontFile = /\.(otf|ttf|otc|ttc)$/i;
const inNode = typeof process !== "undefined" && !!process.versions?.node;

export function isUrl(where) {
  const text = where instanceof URL ? where.href : String(where);
  if (/^file:/i.test(text)) return !inNode;
  return /^(https?|data|blob):/i.test(text) || !inNode;
}

function toUrl(where) {
  return where instanceof URL ? where : new URL(String(where), globalThis.location?.href);
}

async function nodePath(where) {
  if (where instanceof URL || /^file:/i.test(String(where))) {
    const { fileURLToPath } = await import("node:url");
    return fileURLToPath(where);
  }
  return String(where);
}

export async function loadWasmBytes(source, defaultUrl) {
  source = await (source ?? defaultUrl);
  if (source instanceof WebAssembly.Module) return { module: source };
  if (typeof source === "string" || source instanceof URL) {
    if (isUrl(source)) {
      source = await fetch(toUrl(source));
    } else {
      const { readFile } = await import("node:fs/promises");
      return { bytes: await readFile(await nodePath(source)) };
    }
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
  if (isUrl(root)) return new URL(path, toUrl(String(root).endsWith("/") ? root : `${root}/`));
  const { join } = await import("node:path");
  return join(await nodePath(root), path);
}

export async function read(where) {
  if (isUrl(where)) {
    const res = await fetch(toUrl(where));
    if (!res.ok) throw new Error(`${where}: ${res.status}`);
    return new Uint8Array(await res.arrayBuffer());
  }
  const { readFile } = await import("node:fs/promises");
  return new Uint8Array(await readFile(await nodePath(where)));
}

export async function fontsAt(source, toBytes) {
  if (source instanceof Uint8Array || source instanceof ArrayBuffer || ArrayBuffer.isView(source)) {
    return [toBytes(source)];
  }
  if (isUrl(source)) return [await read(source)];
  const { stat, readdir } = await import("node:fs/promises");
  const { join } = await import("node:path");
  const path = await nodePath(source);
  const info = await stat(path).catch(() => null);
  if (!info) return [];
  if (!info.isDirectory()) return [await read(path)];
  const names = (await readdir(path, { recursive: true })).filter((n) => fontFile.test(n)).sort();
  return Promise.all(names.map((n) => read(join(path, n))));
}
