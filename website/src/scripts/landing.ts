/**
 * The landing page's behaviour: theme, header, install tabs, copying, the
 * hero's glyph, the construction layer, motion on arrival and on scroll,
 * the examples' pointing and inspecting, the runtimes, the screens behind
 * the closing and the wordmark at the foot. Motion is the `motion`
 * library's DOM API.
 */
import { animate, inView, scroll, stagger } from 'motion';
import { mountGlyph } from './glyph';
import { mountGuides } from './guides';

const reduced = matchMedia('(prefers-reduced-motion: reduce)').matches;
const fine = matchMedia('(hover: hover) and (pointer: fine)').matches;
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
		area: glyph?.closest('section'),
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

// ------------------------------------------------------------------ construction

const guides = mountGuides({ reduced });

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

	// From the page to the JSON: the innermost box under the pointer lights
	// its lines and brings them into view.
	for (const sheet of $$('.sheet', plate)) {
		const boxes = $$('.box', sheet)
			.map((b) => ({ id: b.dataset.id || '', r: (b.dataset.r || '').split(',').map(Number) }))
			.filter((b) => b.r.length === 4)
			.sort((a, b) => (a.r[2] - a.r[0]) * (a.r[3] - a.r[1]) - (b.r[2] - b.r[0]) * (b.r[3] - b.r[1]));
		if (!boxes.length) continue;
		sheet.classList.add('is-pointable');
		sheet.addEventListener('pointermove', (e) => {
			const rect = sheet.getBoundingClientRect();
			const u = (e.clientX - rect.left) / rect.width;
			const v = (e.clientY - rect.top) / rect.height;
			const hit = boxes.find((b) => u >= b.r[0] && u <= b.r[2] && v >= b.r[1] && v <= b.r[3]);
			light(hit ? hit.id : '', true);
		});
		sheet.addEventListener('pointerleave', () => light('', false));
	}

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

// ------------------------------------------------------------------ same pixels

/** A hash worked out again: the digits churn, then settle on the same ones. */
function scramble(el: HTMLElement, delay = 0) {
	const final = el.dataset.hash || (el.dataset.hash = el.textContent || '');
	const hex = '0123456789abcdef';
	const begin = performance.now() + delay;
	const dur = 700;
	const tick = (now: number) => {
		const t = (now - begin) / dur;
		if (t < 0) return requestAnimationFrame(tick);
		let out = '';
		for (let i = 0; i < final.length; i++) out += final[i] === ' ' || i / final.length < t ? final[i] : hex[(Math.random() * 16) | 0];
		el.textContent = out;
		if (t < 1) requestAnimationFrame(tick);
		else el.textContent = final;
	};
	requestAnimationFrame(tick);
}

const runtimes = $('[data-runtimes]');
if (runtimes) {
	const section = runtimes.closest<HTMLElement>('section')!;
	const tabs = $$<HTMLButtonElement>('[role="tab"]', runtimes);
	const runs = $$('.press__run', section);
	const scan = $('.press__scan', section);
	const fp = $('.press__hash', section);
	let current = 0;
	let auto = !reduced;
	const select = (i: number, how: 'auto' | 'pointer' | 'key' = 'auto') => {
		if (how !== 'auto') auto = false;
		if (i === current && how === 'auto') return;
		current = i;
		tabs.forEach((t, k) => {
			const on = k === i;
			t.setAttribute('aria-selected', String(on));
			t.tabIndex = on ? 0 : -1;
			const panel = document.getElementById(t.getAttribute('aria-controls') || '');
			if (panel) panel.hidden = !on;
		});
		runs.forEach((r) => r.classList.toggle('is-on', r.dataset.run === tabs[i].dataset.rt));
		if (how === 'key') tabs[i].focus();
		guides.refresh(section);
		if (reduced) return;
		const hash = $('.rt__panel:not([hidden]) .rt__hash', runtimes);
		if (hash) scramble(hash);
		if (fp) scramble(fp, 260);
		const panel = $('.rt__panel:not([hidden])', runtimes);
		if (panel) animate($$('.rt__text, .rt__ln, .rt__out, .rt__more', panel), { opacity: [0, 1], transform: ['translateY(6px)', 'translateY(0px)'] }, { duration: 0.5, delay: stagger(0.04), ease });
		if (scan) animate(scan, { transform: ['translateY(-12%)', 'translateY(1120%)'], opacity: [0, 1, 1, 0] }, { duration: 0.95, ease: 'linear' });
		setTimeout(() => guides.pulse(section, 'connector'), 120);
	};
	tabs.forEach((t, i) => {
		t.addEventListener('click', () => select(i, 'pointer'));
		t.addEventListener('keydown', (e) => {
			const next = e.key === 'ArrowDown' || e.key === 'ArrowRight' ? 1 : e.key === 'ArrowUp' || e.key === 'ArrowLeft' ? -1 : 0;
			if (!next) return;
			e.preventDefault();
			select((i + next + tabs.length) % tabs.length, 'key');
		});
	});
	// The runtimes take turns while the section is in view, until someone picks one.
	inView(runtimes, () => {
		const id = window.setInterval(() => auto && select((current + 1) % tabs.length), 5200);
		return () => clearInterval(id);
	});
}

// ------------------------------------------------------------------ the closing's screens

// Four screens at the classic angles make a rosette; the pointer turns them
// by a degree or two against each other, and the moiré moves.
const rosette = $('[data-rosette]');
if (rosette && fine && !reduced) {
	const section = rosette.closest<HTMLElement>('section')!;
	const plates = $$('.rosette__plate', rosette);
	const turn = [
		[1.6, 0.5],
		[-1.2, 0.8],
		[0.6, -1.4],
		[-0.7, -0.9],
	];
	let tx = 0;
	let ty = 0;
	let x = 0;
	let y = 0;
	let raf = 0;
	const tick = () => {
		x += (tx - x) * 0.07;
		y += (ty - y) * 0.07;
		plates.forEach((p, i) => (p.style.transform = `rotate(${(x * turn[i][0] + y * turn[i][1]).toFixed(3)}deg)`));
		rosette.style.setProperty('--rx', `${50 + x * 18}%`);
		rosette.style.setProperty('--ry', `${62 + y * 14}%`);
		raf = Math.abs(tx - x) + Math.abs(ty - y) > 0.0008 ? requestAnimationFrame(tick) : 0;
	};
	section.addEventListener('pointermove', (e) => {
		const r = section.getBoundingClientRect();
		tx = (e.clientX - r.left) / r.width - 0.5;
		ty = (e.clientY - r.top) / r.height - 0.5;
		if (!raf) raf = requestAnimationFrame(tick);
	});
	section.addEventListener('pointerleave', () => {
		tx = ty = 0;
		if (!raf) raf = requestAnimationFrame(tick);
	});
}

// ------------------------------------------------------------------ the wordmark at the foot

// Screened, until the pointer passes over it: there the ink is solid.
const footword = $('[data-footword]');
if (footword && fine) {
	const footer = footword.closest<HTMLElement>('footer')!;
	footer.addEventListener('pointermove', (e) => {
		const r = footword.getBoundingClientRect();
		footword.style.setProperty('--mx', `${e.clientX - r.left}px`);
		footword.style.setProperty('--my', `${e.clientY - r.top}px`);
		footword.classList.add('is-lit');
	});
	footer.addEventListener('pointerleave', () => footword.classList.remove('is-lit'));
}
