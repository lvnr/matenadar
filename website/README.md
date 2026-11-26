# Matenadar website

The front page (`/`), the documentation (`/docs/`) and the 3D desk
(`/fancy/`) at [matenadar.levships.com](https://matenadar.levships.com),
built with [Astro](https://astro.build) and
[Starlight](https://starlight.astro.build) as a static site for Cloudflare
Workers Static Assets.

```sh
npm install
npm run dev       # http://localhost:4321
npm run build     # → dist/
npm run preview
npm run deploy    # build, the 3D desk into dist/fancy/, then `wrangler deploy`
```

- **Node:** Astro 7 needs Node 22.23.4 or later (`.node-version` pins 26).
- **Site URL:** canonical links, the social card and the sitemap use
  `https://matenadar.levships.com`; set `SITE_URL` to build for another
  origin.
- **Examples:** every page the front page shows is the engine's own output.
  The documents are in `examples/` (one folder each, with its art; the open
  licence fonts come from `examples/fonts/fetch.sh`).
  `scripts/render-examples.py` builds them with `../target/release/matenadar`
  and writes the renders, PDFs and JSON to `public/examples/`, the layout
  reports and timings to `src/data/examples.json`, and the social card
  (`examples/og/doc.json`) to `public/og.jpg`. Run it after an example or the
  engine changes; the site itself never needs the engine.
  `scripts/jsonfmt.py` formats an example's JSON the way the page shows it.
- **Content:** the docs are in `src/content/docs/docs/`, distilled from
  `docs/model.md`, `crates/matenadar/src/{main,serve,lib}.rs` and
  `crates/matenadar-wasm/js/matenadar.d.ts`. Keep them in step with those files.
- **Theme:** `src/styles/tokens.css` holds the colours and faces shared by
  `src/styles/landing.css` (the front page) and `src/styles/starlight.css`
  (the docs); `ec.config.mjs` themes the code blocks with the same colours.
  Instrument Sans, Instrument Serif and JetBrains Mono are self-hosted
  through Fontsource. The front page's motion uses [Motion](https://motion.dev);
  its Մ is drawn on a canvas by `src/scripts/glyph.ts`.
- **The 3D desk:** `scripts/fancy.sh` builds
  [matenadar-site](https://github.com/lvnr/matenadar-site) (a checkout at
  `FANCY_SRC`, by default `~/projects/matenadar-site`) with `--base /fancy/`
  into `dist/fancy/`. `npm run deploy` stops if the checkout is missing.
- **The old docs host:** `matenadar-docs.levships.com` redirects every path to
  the same path here: `redirect/` (`npm run deploy:redirect`).
- **Account:** when the Cloudflare login sees more than one account, set
  `CLOUDFLARE_ACCOUNT_ID` to the one that holds `levships.com` for both
  deploys.
- **Cache:** `prebuild` clears Astro's content cache, which otherwise keeps
  rendered pages pointing at an earlier code-block stylesheet after
  `ec.config.mjs` changes. `public/_headers` sets the caching.
- The WebAssembly module is not served from here (it is larger than
  Cloudflare's 25 MiB file limit); the docs point at jsDelivr.
