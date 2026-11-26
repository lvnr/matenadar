#!/usr/bin/env node
// Stages publishable npm packages for the native `matenadar` CLI and its
// per-platform binary packages (`@matenadar/cli-*`).
//
// Usage:
//   node scripts/stage-npm-cli.mjs --version 0.1.0 --out out/npm \
//     --bin aarch64-apple-darwin=/path/to/matenadar \
//     --bin x86_64-unknown-linux-gnu=/path/to/matenadar ...
//
// If --meta-only is passed, only stages `out/npm/matenadar` (wrapper package).

import { chmodSync, copyFileSync, cpSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const REPO_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");

export const TARGETS = {
  "aarch64-apple-darwin": {
    pkg: "@matenadar/cli-darwin-arm64",
    dir: "cli-darwin-arm64",
    os: ["darwin"],
    cpu: ["arm64"],
    bin: "matenadar",
    desc: "macOS Apple Silicon (arm64)",
  },
  "x86_64-apple-darwin": {
    pkg: "@matenadar/cli-darwin-x64",
    dir: "cli-darwin-x64",
    os: ["darwin"],
    cpu: ["x64"],
    bin: "matenadar",
    desc: "macOS Intel (x64)",
  },
  "x86_64-unknown-linux-gnu": {
    pkg: "@matenadar/cli-linux-x64-gnu",
    dir: "cli-linux-x64-gnu",
    os: ["linux"],
    cpu: ["x64"],
    libc: ["glibc"],
    bin: "matenadar",
    desc: "Linux x86_64 (glibc)",
  },
  "aarch64-unknown-linux-gnu": {
    pkg: "@matenadar/cli-linux-arm64-gnu",
    dir: "cli-linux-arm64-gnu",
    os: ["linux"],
    cpu: ["arm64"],
    libc: ["glibc"],
    bin: "matenadar",
    desc: "Linux ARM64 (glibc)",
  },
  "x86_64-pc-windows-msvc": {
    pkg: "@matenadar/cli-win32-x64-msvc",
    dir: "cli-win32-x64-msvc",
    os: ["win32"],
    cpu: ["x64"],
    bin: "matenadar.exe",
    desc: "Windows x86_64 (MSVC)",
  },
};

function parseArgs(argv) {
  let version = null;
  let outDir = join(REPO_ROOT, "out", "npm");
  const bins = new Map();
  let metaOnly = false;

  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i];
    if (arg === "--version") {
      version = argv[++i];
    } else if (arg === "--out") {
      outDir = resolve(argv[++i]);
    } else if (arg === "--meta-only") {
      metaOnly = true;
    } else if (arg === "--bin") {
      const spec = argv[++i];
      const eq = spec.indexOf("=");
      if (eq === -1) throw new Error(`invalid --bin spec (expected TARGET=PATH): ${spec}`);
      bins.set(spec.slice(0, eq), resolve(spec.slice(eq + 1)));
    } else {
      throw new Error(`unknown argument: ${arg}`);
    }
  }

  if (!version) {
    const cliPkg = JSON.parse(readFileSync(join(REPO_ROOT, "packages/cli/package.json"), "utf8"));
    version = cliPkg.version;
  }
  return { version, outDir, bins, metaOnly };
}

function stageWrapper(version, outDir) {
  const dest = join(outDir, "matenadar");
  rmSync(dest, { recursive: true, force: true });
  mkdirSync(dest, { recursive: true });
  cpSync(join(REPO_ROOT, "packages/cli/bin"), join(dest, "bin"), { recursive: true });
  chmodSync(join(dest, "bin/matenadar.js"), 0o755);
  copyFileSync(join(REPO_ROOT, "packages/cli/README.md"), join(dest, "README.md"));
  copyFileSync(join(REPO_ROOT, "LICENSE"), join(dest, "LICENSE"));

  const pkg = JSON.parse(readFileSync(join(REPO_ROOT, "packages/cli/package.json"), "utf8"));
  pkg.version = version;
  for (const key of Object.keys(pkg.optionalDependencies)) {
    pkg.optionalDependencies[key] = version;
  }
  writeFileSync(join(dest, "package.json"), JSON.stringify(pkg, null, 2) + "\n");
  console.log(`staged ${pkg.name}@${version} -> ${dest}`);
}

function stageTarget(target, binPath, version, outDir) {
  const meta = TARGETS[target];
  if (!meta) throw new Error(`unsupported target: ${target}`);

  const dest = join(outDir, meta.dir);
  rmSync(dest, { recursive: true, force: true });
  mkdirSync(join(dest, "bin"), { recursive: true });

  const destBin = join(dest, "bin", meta.bin);
  copyFileSync(binPath, destBin);
  if (!meta.bin.endsWith(".exe")) chmodSync(destBin, 0o755);
  copyFileSync(join(REPO_ROOT, "LICENSE"), join(dest, "LICENSE"));

  const pkg = {
    name: meta.pkg,
    version,
    description: `Prebuilt Matenadar CLI binary for ${meta.desc}`,
    license: "Apache-2.0",
    author: "Levon Arakelyan",
    homepage: "https://github.com/lvnr/matenadar",
    repository: {
      type: "git",
      url: "git+https://github.com/lvnr/matenadar.git",
    },
    bugs: {
      url: "https://github.com/lvnr/matenadar/issues",
    },
    os: meta.os,
    cpu: meta.cpu,
    ...(meta.libc ? { libc: meta.libc } : {}),
    files: ["bin/", "README.md", "LICENSE"],
    publishConfig: {
      access: "public",
    },
  };

  writeFileSync(join(dest, "package.json"), JSON.stringify(pkg, null, 2) + "\n");
  writeFileSync(
    join(dest, "README.md"),
    `# ${meta.pkg}\n\nPrebuilt native \`matenadar\` binary for ${meta.desc}.\n\nInstall via [\`matenadar\`](https://www.npmjs.com/package/matenadar) instead of depending on this package directly:\n\n\`\`\`bash\nnpx matenadar --version\n\`\`\`\n`
  );
  console.log(`staged ${meta.pkg}@${version} -> ${dest}`);
}

const { version, outDir, bins, metaOnly } = parseArgs(process.argv.slice(2));
mkdirSync(outDir, { recursive: true });
stageWrapper(version, outDir);
if (!metaOnly) {
  for (const [target, binPath] of bins.entries()) {
    stageTarget(target, binPath, version, outDir);
  }
}
