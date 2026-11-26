# matenadar

Prebuilt native CLI for [Matenadar](https://github.com/lvnr/matenadar)—lay out, inspect, check, edit, and export books to PDF, PNG, SVG, and EPUB 3.

## Quick start

```bash
npx matenadar --version
npx matenadar build book.json -o book.pdf --check
```

Or install globally:

```bash
npm install -g matenadar
matenadar help
```

> Looking for the browser / Node WebAssembly library instead? Install [`@matenadar/wasm`](https://www.npmjs.com/package/@matenadar/wasm).

## How it works

This package installs the prebuilt native Rust binary for your OS and CPU via npm `optionalDependencies` (`@matenadar/cli-*`)—no `postinstall` download scripts, compatible with `--ignore-scripts`, `pnpm`, and `bunx`.

Supported platforms:
- macOS Apple Silicon (`darwin-arm64`)
- macOS Intel (`darwin-x64`)
- Linux x86_64 glibc (`linux-x64`)
- Linux ARM64 glibc (`linux-arm64`)
- Windows x86_64 (`win32-x64`)

## License

Apache-2.0
