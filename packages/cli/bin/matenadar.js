#!/usr/bin/env node
"use strict";

const { spawnSync } = require("node:child_process");

const PLATFORMS = {
  "darwin-arm64": { pkg: "@matenadar/cli-darwin-arm64", bin: "bin/matenadar" },
  "darwin-x64": { pkg: "@matenadar/cli-darwin-x64", bin: "bin/matenadar" },
  "linux-x64": { pkg: "@matenadar/cli-linux-x64-gnu", bin: "bin/matenadar" },
  "linux-arm64": { pkg: "@matenadar/cli-linux-arm64-gnu", bin: "bin/matenadar" },
  "win32-x64": { pkg: "@matenadar/cli-win32-x64-msvc", bin: "bin/matenadar.exe" },
};

function resolveBinary() {
  if (process.env.MATENADAR_BIN) {
    return process.env.MATENADAR_BIN;
  }

  const key = `${process.platform}-${process.arch}`;
  const entry = PLATFORMS[key];
  if (!entry) {
    console.error(
      `matenadar: no prebuilt npm binary for ${process.platform}/${process.arch}.\n` +
        `Install from source with: cargo install matenadar`
    );
    process.exit(1);
  }

  try {
    return require.resolve(`${entry.pkg}/${entry.bin}`);
  } catch {
    console.error(
      `matenadar: could not find platform package "${entry.pkg}".\n` +
        `Make sure optional dependencies were installed (avoid --no-optional / --omit=optional),\n` +
        `or install natively via Homebrew (brew install lvnr/tap/matenadar) or Cargo (cargo install matenadar).`
    );
    process.exit(1);
  }
}

const bin = resolveBinary();
const result = spawnSync(bin, process.argv.slice(2), {
  stdio: "inherit",
  windowsHide: false,
});

if (result.error) {
  console.error(`matenadar: failed to run ${bin}: ${result.error.message}`);
  process.exit(1);
}

if (result.signal) {
  process.kill(process.pid, result.signal);
}

process.exit(result.status ?? 1);
