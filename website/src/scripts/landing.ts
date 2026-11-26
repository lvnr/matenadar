/**
 * The landing page's behaviour: theme, header, install tabs, copying, the
 * hero's glyph, motion on arrival and on scroll, and the examples' pointing
 * and inspecting. Motion is the `motion` library's DOM API.
 */
import { animate, inView, scroll, stagger } from 'motion';
import { mountGlyph } from './glyph';

const reduced = matchMedia('(prefers-reduced-motion: reduce)').matches;
const ease = [0.22, 1, 0.36, 1] as const;
const $ = <T extends Element = HTMLElement>(s: string, root: ParentNode = document) => root.querySelector<T>(s);
const $$ = <T extends Element = HTMLElement>(s: string, root: ParentNode = document) => [...root.querySelectorAll<T>(s)];

// ------------------------------------------------------------------ theme

const toggle = $<HTMLButtonElement>('[data-theme-toggle]');
function setTheme(theme: 'light' | 'dark') {
	document.documentElement.dataset.theme = theme;
	try {
		localStorage.setItem('starlight-theme', theme);
	} catch {}
	toggle?.setAttribute('aria-label', theme === 'dark' ? 'Switch to light theme' : 'Switch to dark theme');
}
toggle?.addEventListener('click', () => setTheme(document.documentElement.dataset.theme === 'dark' ? 'light' : 'dark'));
if (document.documentElement.dataset.theme === 'dark') toggle?.setAttribute('aria-label', 'Switch to light theme');

// ------------------------------------------------------------------ header

const header = $('[data-header]');
const onScroll = () => header?.classList.toggle('is-scrolled', window.scrollY > 24);
onScroll();
window.addEventListener('scroll', onScroll, { passive: true });

// ------------------------------------------------------------------ tabs

for (const tabs of $$('[data-tabs]')) {
	const buttons = $$<HTMLButtonElement>('[role="tab"]', tabs);
	const select = (b: HTMLButtonElement, focus = false) => {
		for (const o of buttons) {
			const on = o === b;
			o.setAttribute('aria-selected', String(on));
			o.tabIndex = on ? 0 : -1;
			const panel = document.getElementById(o.getAttribute('aria-controls') || '');
			if (panel) panel.hidden = !on;
		}
		if (focus) b.focus();
	};
	for (const b of buttons) {
		b.addEventListener('click', () => select(b));
		b.addEventListener('keydown', (e) => {
			const i = buttons.indexOf(b);
			if (e.key === 'ArrowRight') select(buttons[(i + 1) % buttons.length], true);
			else if (e.key === 'ArrowLeft') select(buttons[(i - 1 + buttons.length) % buttons.length], true);
			else return;
			e.preventDefault();
		});
	}
}

// ------------------------------------------------------------------ copy

async function copy(text: string) {
	try {
		await navigator.clipboard.writeText(text);
	} catch {
		const t = document.createElement('textarea');
		t.value = text;
		t.style.position = 'fixed';
		t.style.opacity = '0';
		document.body.append(t);
		t.select();
		document.execCommand('copy');
		t.remove();
	}
}
document.addEventListener('click', async (e) => {
	const b = (e.target as Element).closest<HTMLButtonElement>('[data-copy], [data-copy-from]');
	if (!b) return;
	const from = b.dataset.copyFrom ? document.querySelector(b.dataset.copyFrom) : null;
	await copy(from ? from.textContent || '' : b.dataset.copy || '');
	b.classList.add('is-copied');
	if (b.classList.contains('code__copy')) b.textContent = 'Copied';
	setTimeout(() => {
		b.classList.remove('is-copied');
		if (b.classList.contains('code__copy')) b.textContent = 'Copy';
	}, 1600);
});

// ------------------------------------------------------------------ hero

const glyph = $('[data-glyph]');
const canvas = $<HTMLCanvasElement>('canvas', glyph || document);
const readout = $('[data-readout-text]');
const restText = readout?.textContent || '';
if (canvas) {
	mountGlyph(canvas, {
		reduced,
		onPoint(x, y, part) {
			if (!readout) return;
			if (Number.isNaN(x)) {
				readout.textContent = restText;
				return;
			}
			const xy = `x ${x.toFixed(1)}  y ${y.toFixed(1)}`;
			readout.textContent = part ? `${xy}  →  ${part}` : xy;
		},
	});
}

if (!reduced) {
	// The construction first, then the words, as a page is drawn up before it is set.
	const lines = $$<SVGGeometryElement>('.hero__construct line:not(.c-dash), .hero__construct circle:not(.c-dash)');
	animate(lines, { strokeDashoffset: [1, 0] }, { duration: 1.4, delay: stagger(0.06), ease });
	animate('.hero__construct .c-dash', { opacity: [0, 1] }, { duration: 0.8, delay: stagger(0.08, { startDelay: 0.5 }) });
	animate('.hero__construct .c-nodes, .glabel, .readout', { opacity: [0, 1] }, { duration: 0.6, delay: stagger(0.05, { startDelay: 0.9 }) });
	animate('.hero__title .line__in', { transform: ['translateY(105%)', 'translateY(0%)'] }, { duration: 1.1, delay: stagger(0.12, { startDelay: 0.15 }), ease });
	animate('[data-fade]', { opacity: [0, 1], transform: ['translateY(22px)', 'translateY(0px)'] }, { duration: 0.9, delay: stagger(0.1, { startDelay: 0.55 }), ease });
}

// ------------------------------------------------------------------ reveals

if (!reduced) {
	inView(
		'[data-reveal]',
		(el) => {
			animate(el, { opacity: [0, 1], transform: ['translateY(22px)', 'translateY(0px)'] }, { duration: 0.9, ease });
		},
		{ margin: '0px 0px -12% 0px' },
	);

	// The giant words drift against the scroll, past the edges of the screen.
	for (const word of $$('[data-drift]')) {
		const plate = word.closest('[data-plate]') as HTMLElement;
		const flip = plate.classList.contains('plate--flip');
		scroll(animate(word, { transform: flip ? ['translateX(12%)', 'translateX(-8%)'] : ['translateX(-12%)', 'translateX(8%)'] }, { ease: 'linear' }), {
			target: plate,
			offset: ['start end', 'end start'],
		});
	}
	const closing = $('.closing');
	if (closing) {
		scroll(animate('[data-drift-l]', { transform: ['translateX(-14%)', 'translateX(4%)'] }, { ease: 'linear' }), { target: closing, offset: ['start end', 'end start'] });
		scroll(animate('[data-drift-r]', { transform: ['translateX(10%)', 'translateX(-10%)'] }, { ease: 'linear' }), { target: closing, offset: ['start end', 'end start'] });
	}

	// A page arrives the way a sheet comes off the press: the plates pass
	// over it, then the paper settles.
	inView(
		'.stage',
		(stage) => {
			const inner = $('.stage__inner', stage) as HTMLElement;
			const sweep = animate(inner, { clipPath: ['inset(-60px 100% -60px -60px)', 'inset(-60px -60px -60px -60px)'] }, { duration: 1.25, ease });
			sweep.finished.then(() => (inner.style.clipPath = 'none'));
			const sheets = $$('.sheet', stage).filter((s) => getComputedStyle(s).display !== 'none');
			animate(sheets, { transform: ['translateY(28px)', 'translateY(0px)'] }, { duration: 1.4, delay: stagger(0.12), ease });
		},
		{ margin: '0px 0px -10% 0px' },
	);
}

// ------------------------------------------------------------------ plates

for (const plate of $$('[data-plate]')) {
	const lines = $$('.ln[data-id]', plate);
	const boxes = $$('.box', plate);
	const pre = $('.code__pre', plate);
	const byId = (id: string) => boxes.filter((b) => b.dataset.id === id);
	const linesOf = (id: string) => lines.filter((l) => l.dataset.id === id);
	let hot = '';
	const light = (id: string, scrollCode: boolean) => {
		if (id === hot) return;
		for (const el of [...byId(hot), ...linesOf(hot)]) el.classList.remove('is-hot');
		hot = id;
		if (!id) return;
		for (const el of [...byId(id), ...linesOf(id)]) el.classList.add('is-hot');
		const first = linesOf(id)[0];
		if (scrollCode && first && pre) {
			const top = first.offsetTop - pre.clientHeight / 3;
			const visible = first.offsetTop > pre.scrollTop && first.offsetTop < pre.scrollTop + pre.clientHeight - 30;
			if (!visible) pre.scrollTo({ top, behavior: reduced ? 'auto' : 'smooth' });
		}
	};
	plate.addEventListener('pointerover', (e) => {
		const t = e.target as HTMLElement;
		const line = t.closest<HTMLElement>('.ln[data-id]');
		const box = t.closest<HTMLElement>('.box');
		if (line) light(line.dataset.id || '', false);
		else if (box) light(box.dataset.id || '', true);
	});
	plate.addEventListener('pointerleave', () => light('', false));
	$('.code__pre', plate)?.addEventListener('pointerleave', () => light('', false));

	// Inspect: the layout report's boxes over the page.
	const inspect = $<HTMLButtonElement>('[data-inspect]', plate);
	inspect?.addEventListener('click', () => {
		const on = !plate.classList.contains('is-inspecting');
		plate.classList.toggle('is-inspecting', on);
		inspect.setAttribute('aria-pressed', String(on));
	});

	// Deck: thumbnails choose the slide; it moves on by itself until touched.
	const thumbs = $$<HTMLButtonElement>('.deck-nav__thumb', plate);
	if (thumbs.length) {
		const sheets = $$('.sheet', plate);
		let current = 0;
		let auto = !reduced;
		const show = (i: number) => {
			current = i;
			sheets.forEach((s, k) => s.classList.toggle('sheet--active', k === i));
			thumbs.forEach((t, k) => t.setAttribute('aria-selected', String(k === i)));
			if (!reduced) animate(sheets[i], { opacity: [0.4, 1], transform: ['scale(0.985)', 'scale(1)'] }, { duration: 0.5, ease });
		};
		thumbs.forEach((t, i) =>
			t.addEventListener('click', () => {
				auto = false;
				show(i);
			}),
		);
		let timer = 0;
		inView(plate, () => {
			timer = window.setInterval(() => auto && show((current + 1) % sheets.length), 4200);
			return () => clearInterval(timer);
		});
	}

	// The stage leans toward the pointer, on a spring.
	const stage = $('[data-tilt]', plate);
	const inner = stage && $('.stage__inner', stage);
	if (stage && inner && !reduced && matchMedia('(hover: hover)').matches) {
		let tx = 0;
		let ty = 0;
		let x = 0;
		let y = 0;
		let vx = 0;
		let vy = 0;
		let raf = 0;
		const tick = () => {
			const k = 0.08;
			const d = 0.78;
			vx = (vx + (tx - x) * k) * d;
			vy = (vy + (ty - y) * k) * d;
			x += vx;
			y += vy;
			inner.style.transform = `rotateY(${(x * 5).toFixed(3)}deg) rotateX(${(-y * 4).toFixed(3)}deg)`;
			if (Math.abs(tx - x) + Math.abs(ty - y) + Math.abs(vx) + Math.abs(vy) > 0.001) raf = requestAnimationFrame(tick);
			else raf = 0;
		};
		stage.addEventListener('pointermove', (e) => {
			const r = stage.getBoundingClientRect();
			tx = ((e.clientX - r.left) / r.width - 0.5) * 2;
			ty = ((e.clientY - r.top) / r.height - 0.5) * 2;
			stage.style.setProperty('--sx', `${((e.clientX - r.left) / r.width) * 100}%`);
			stage.style.setProperty('--sy', `${((e.clientY - r.top) / r.height) * 100}%`);
			if (!raf) raf = requestAnimationFrame(tick);
		});
		stage.addEventListener('pointerleave', () => {
			tx = ty = 0;
			if (!raf) raf = requestAnimationFrame(tick);
		});
	}
}

// ------------------------------------------------------------------ talks back

const proof = $('[data-proof]');
if (proof) {
	const flags = $$('.flag', proof);
	const issues = $$<HTMLButtonElement>('[data-issue]');
	const light = (node: string) => {
		for (const f of flags) f.classList.toggle('is-hot', f.dataset.node === node);
		for (const b of issues) b.classList.toggle('is-hot', b.dataset.issue === node);
	};
	for (const f of flags) {
		f.addEventListener('pointerenter', () => light(f.dataset.node || ''));
		f.addEventListener('pointerleave', () => light(''));
	}
	for (const b of issues) {
		b.addEventListener('pointerenter', () => light(b.dataset.issue || ''));
		b.addEventListener('pointerleave', () => light(''));
		b.addEventListener('focus', () => light(b.dataset.issue || ''));
		b.addEventListener('blur', () => light(''));
		b.addEventListener('click', () => light(b.dataset.issue || ''));
	}
}
