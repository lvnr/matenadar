# Matenadar website

The landing page (`/`) and the documentation (`/docs/`), built with
[Astro](https://astro.build) and [Starlight](https://starlight.astro.build)
as a static site for Cloudflare Workers Static Assets.

```sh
npm install
npm run dev       # http://localhost:4321
npm run build     # → dist/
npm run preview
npm run deploy    # build, then `wrangler deploy` (wrangler.jsonc)
```

- **Node:** Astro 7 needs Node 22.23.4 or later (`.node-version` pins 26).
- **Site URL:** canonical links and the sitemap use
  `https://matenadar-docs.levships.com`; set `SITE_URL` to build for another
  origin.
- **Content:** the docs are in `src/content/docs/docs/`, distilled from
  `docs/model.md`, `crates/matenadar/src/{main,serve,lib}.rs` and
  `crates/matenadar-wasm/js/matenadar.d.ts`. Keep them in step with those files.
- **Theme:** `src/styles/tokens.css` holds the colours and faces shared by
  `src/styles/landing.css` (the landing page) and `src/styles/starlight.css`
  (the docs); `ec.config.mjs` themes the code blocks. Geist and JetBrains
  Mono are self-hosted through Fontsource.
- **Cache:** `prebuild` clears Astro's content cache, which otherwise keeps
  rendered pages pointing at an earlier code-block stylesheet after
  `ec.config.mjs` changes.
- The WebAssembly module is not served from here (it is larger than
  Cloudflare's 25 MiB file limit); the docs point at jsDelivr.
