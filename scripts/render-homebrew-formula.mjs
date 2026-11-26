#!/usr/bin/env node
// Renders Formula/matenadar.rb for lvnr/homebrew-tap from a release version and SHA256SUMS.txt.
//
// Usage:
//   node scripts/render-homebrew-formula.mjs --version 0.1.0 --checksums SHA256SUMS.txt --out Formula/matenadar.rb

import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";

function parseArgs(argv) {
  let version = null;
  let checksums = null;
  let out = null;
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a === "--version") version = argv[++i];
    else if (a === "--checksums") checksums = argv[++i];
    else if (a === "--out") out = argv[++i];
    else throw new Error(`unknown argument: ${a}`);
  }
  if (!version || !checksums) {
    throw new Error("usage: render-homebrew-formula.mjs --version X.Y.Z --checksums SHA256SUMS.txt [--out FILE]");
  }
  version = version.replace(/^v/, "");
  return { version, checksums: resolve(checksums), out: out ? resolve(out) : null };
}

function parseChecksums(text) {
  const map = new Map();
  for (const line of text.split(/\r?\n/)) {
    const m = line.trim().match(/^([0-9a-fA-F]{64})\s+\*?(.+)$/);
    if (m) map.set(m[2].trim(), m[1].toLowerCase());
  }
  return map;
}

const { version, checksums, out } = parseArgs(process.argv.slice(2));
const sums = parseChecksums(readFileSync(checksums, "utf8"));
const tag = `v${version}`;
const base = `https://github.com/lvnr/matenadar/releases/download/${tag}`;

function entry(target) {
  const file = `matenadar-${tag}-${target}.tar.gz`;
  const sha = sums.get(file);
  if (!sha) throw new Error(`missing checksum for ${file} in ${checksums}`);
  return { url: `${base}/${file}`, sha };
}

const macArm = entry("aarch64-apple-darwin");
const macIntel = entry("x86_64-apple-darwin");
const linuxArm = entry("aarch64-unknown-linux-gnu");
const linuxIntel = entry("x86_64-unknown-linux-gnu");

const formula = `class Matenadar < Formula
  desc "Typesetting engine for anything that prints, for people and agents"
  homepage "https://github.com/lvnr/matenadar"
  version "${version}"
  license "Apache-2.0"

  on_macos do
    on_arm do
      url "${macArm.url}"
      sha256 "${macArm.sha}"
    end
    on_intel do
      url "${macIntel.url}"
      sha256 "${macIntel.sha}"
    end
  end

  on_linux do
    on_arm do
      url "${linuxArm.url}"
      sha256 "${linuxArm.sha}"
    end
    on_intel do
      url "${linuxIntel.url}"
      sha256 "${linuxIntel.sha}"
    end
  end

  def install
    bin.install "matenadar"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/matenadar --version")
  end
end
`;

if (out) {
  mkdirSync(dirname(out), { recursive: true });
  writeFileSync(out, formula);
  console.log(`wrote ${out}`);
} else {
  process.stdout.write(formula);
}
