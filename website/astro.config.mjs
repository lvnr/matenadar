// @ts-check
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

const GITHUB = 'https://github.com/lvnr/matenadar';

// The deployed origin, for canonical links and the sitemap. Override with
// SITE_URL when deploying elsewhere (SITE_URL=https://docs.example.com).
const site = process.env.SITE_URL || 'https://matenadar.levships.com';

export default defineConfig({
	site,
	output: 'static',
	build: { inlineStylesheets: 'auto' },
	// Keep font files as files (inlined as base64 they bloat every stylesheet);
	// anything else small, such as a short stylesheet, may still be inlined.
	vite: {
		build: {
			assetsInlineLimit: (file, content) => (/\.(woff2?|ttf|otf)$/.test(file) ? false : content.length < 4096),
		},
	},
	// Section roots without pages of their own go to their first page.
	redirects: {
		'/docs/getting-started': '/docs/getting-started/install/',
		'/docs/model': '/docs/model/overview/',
		'/docs/guides': '/docs/guides/operations/',
		'/docs/reference': '/docs/reference/cli/',
	},
	integrations: [
		starlight({
			title: 'Matenadar',
			description:
				'A typesetting engine for anything that prints, for people and agents: a JSON model laid out with Typst to PDF, SVG, PNG and EPUB 3, in the terminal and in the browser.',
			favicon: '/favicon.svg',
			social: [
				{ icon: 'github', label: 'GitHub', href: GITHUB },
				{ icon: 'npm', label: 'npm', href: 'https://www.npmjs.com/package/@matenadar/wasm' },
			],
			editLink: { baseUrl: `${GITHUB}/edit/main/website/` },
			credits: false,
			// src/pages/404.astro renders the not-found page with StarlightPage.
			disable404Route: true,
			tableOfContents: { minHeadingLevel: 2, maxHeadingLevel: 3 },
			head: [
				{ tag: 'meta', attrs: { name: 'theme-color', content: '#ffffff', media: '(prefers-color-scheme: light)' } },
				{ tag: 'meta', attrs: { name: 'theme-color', content: '#09090b', media: '(prefers-color-scheme: dark)' } },
				{ tag: 'meta', attrs: { property: 'og:image', content: new URL('/og.jpg', site).href } },
				{ tag: 'meta', attrs: { property: 'og:image:width', content: '1200' } },
				{ tag: 'meta', attrs: { property: 'og:image:height', content: '630' } },
				{ tag: 'meta', attrs: { name: 'twitter:image', content: new URL('/og.jpg', site).href } },
			],
			customCss: [
				'@fontsource-variable/instrument-sans/standard.css',
				'@fontsource-variable/instrument-sans/standard-italic.css',
				'@fontsource/instrument-serif/400.css',
				'@fontsource/jetbrains-mono/400.css',
				'@fontsource/jetbrains-mono/500.css',
				'./src/styles/tokens.css',
				'./src/styles/starlight.css',
			],
			components: {
				SiteTitle: './src/components/starlight/SiteTitle.astro',
				Footer: './src/components/starlight/Footer.astro',
			},
			sidebar: [
				{
					label: 'Getting started',
					items: [
						{ label: 'Introduction', slug: 'docs' },
						{ label: 'Installing Matenadar', slug: 'docs/getting-started/install' },
						{ label: 'Quick start', slug: 'docs/getting-started/quick-start' },
					],
				},
				{
					label: 'The document model',
					items: [
						{ label: 'Overview', slug: 'docs/model/overview' },
						{ label: 'The design', slug: 'docs/model/design' },
						{ label: 'Content', slug: 'docs/model/content' },
					],
				},
				{
					label: 'Guides',
					items: [
						{ label: 'Operations', slug: 'docs/guides/operations' },
						{ label: 'Preflight checks', slug: 'docs/guides/checks' },
						{ label: 'EPUB 3', slug: 'docs/guides/epub' },
						{ label: 'Working with agents', slug: 'docs/guides/agents' },
					],
				},
				{
					label: 'Reference',
					items: [
						{ label: 'Command line', slug: 'docs/reference/cli' },
						{ label: 'WebAssembly API', slug: 'docs/reference/wasm' },
						{ label: 'Rust crates', slug: 'docs/reference/rust' },
					],
				},
			],
		}),
	],
});
