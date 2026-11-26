#!/usr/bin/env node
// Keeps Cargo.toml workspace version, internal crate versions, and npm package
// versions strictly in sync.
//
// Usage:
//   node scripts/bump-version.mjs 0.2.0

import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const REPO_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const next = (process.argv[2] ?? "").replace(/^v/, "");
if (!/^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$/.test(next)) {
  console.error("usage: node scripts/bump-version.mjs <semver>");
  process.exit(2);
}

// 1. Root Cargo.toml ([workspace.package] version + internal workspace dep versions)
const cargoPath = join(REPO_ROOT, "Cargo.toml");
let cargo = readFileSync(cargoPath, "utf8");
cargo = cargo.replace(
  /(\[workspace\.package\][\s\S]*?^version\s*=\s*")[^"]+(")/m,
  `$1${next}$2`
);
for (const dep of ["matenadar-model", "matenadar-typst", "matenadar-epub"]) {
  cargo = cargo.replace(
    new RegExp(`(^${dep}\\s*=\\s*\\{\\s*version\\s*=\\s*")[^"]+(")`, "m"),
    `$1${next}$2`
  );
}
writeFileSync(cargoPath, cargo);

// 2. packages/wasm/package.json
const wasmPkgPath = join(REPO_ROOT, "packages/wasm/package.json");
const wasmPkg = JSON.parse(readFileSync(wasmPkgPath, "utf8"));
wasmPkg.version = next;
writeFileSync(wasmPkgPath, JSON.stringify(wasmPkg, null, 2) + "\n");

// 3. packages/cli/package.json
const cliPkgPath = join(REPO_ROOT, "packages/cli/package.json");
const cliPkg = JSON.parse(readFileSync(cliPkgPath, "utf8"));
cliPkg.version = next;
for (const key of Object.keys(cliPkg.optionalDependencies ?? {})) {
  cliPkg.optionalDependencies[key] = next;
}
writeFileSync(cliPkgPath, JSON.stringify(cliPkg, null, 2) + "\n");

console.log(`Updated workspace & npm package versions to ${next}.`);
console.log("Run `cargo check --workspace` to update Cargo.lock, then commit and tag v" + next + ".");
