/**
 * The construction layer. Hairlines run through the whole page as they run
 * through a drawing before it is inked: two rails at the edges of the
 * sheet, the baselines and cap heights of the big headlines, the trims of
 * the example pages carried out to the rails with crop ticks and their
 * sizes, rings where something is centred. Each section draws its own in
 * an SVG behind its content, measured from the page as laid out, so the
 * lines scroll with it and meet the next section's.
 *
 * Near the pointer a light runs along them and leaves a fading trail. Its
 * ink follows where the line sits on the page (cyan at the head, through
 * cobalt and violet to magenta and orange at the foot), it burns brightest
 * in the middle of the screen, and on the rails it carries on into the
 * next section.
 */

const NS = 'http://www.w3.org/2000/svg';
type Pt = [number, number];
interface Box {
	x: number;
	y: number;
	w: number;
	h: number;
	x2: number;
	y2: number;
}

interface Guide {
	pts: Pt[];
	cum: number[];
	total: number;
	last: number;
	key?: string;
	touch?: Pulse;
}

interface Sprite {
	grad: SVGLinearGradientElement;
	stops: SVGStopElement[];
	glow: SVGPathElement;
	core: SVGPathElement;
}

interface Pulse {
	layer: Layer;
	g: Guide;
	s: number;
	dir: 1 | -1;
	speed: number;
	len: number;
	age: number;
	life: number;
	amp: number;
	handed: boolean;
	still: boolean;
	sprite?: Sprite;
}

interface Layer {
	el: HTMLElement;
	kind: string;
	svg: SVGSVGElement;
	defs: SVGDefsElement;
	base: SVGGElement;
	fx: SVGGElement;
	guides: Guide[];
	free: Sprite[];
	top: number;
	width: number;
	height: number;
	visible: boolean;
	drawn: boolean;
	uid: string;
}

// ------------------------------------------------------------------ geometry

const crisp = (v: number) => Math.round(v) + 0.5;

function box(x: number, y: number, w: number, h: number): Box {
	return { x, y, w, h, x2: x + w, y2: y + h };
}

/** An element's box in a layer's coordinates, ignoring transforms (the
 *  reveals and drifts move things; the construction stays where they rest). */
function offsetBox(el: HTMLElement, root: HTMLElement): Box {
	let x = 0;
	let y = 0;
	let n: HTMLElement | null = el;
	while (n && n !== root) {
		x += n.offsetLeft;
		y += n.offsetTop;
		const parent = n.offsetParent as HTMLElement | null;
		if (!parent) break;
		// A parent that is not the root but contains it: measure from the root instead.
		if (parent !== root && parent.contains(root)) {
			const r = root.getBoundingClientRect();
			const p = parent.getBoundingClientRect();
			x -= r.left - p.left;
			y -= r.top - p.top;
			break;
		}
		n = parent;
	}
	return box(x, y, el.offsetWidth, el.offsetHeight);
}

function union(boxes: Box[]): Box {
	const x = Math.min(...boxes.map((b) => b.x));
	const y = Math.min(...boxes.map((b) => b.y));
	const x2 = Math.max(...boxes.map((b) => b.x2));
	const y2 = Math.max(...boxes.map((b) => b.y2));
	return box(x, y, x2 - x, y2 - y);
}

function circlePts(cx: number, cy: number, r: number, n = 96): Pt[] {
	const pts: Pt[] = [];
	for (let i = 0; i <= n; i++) {
		const a = (i / n) * Math.PI * 2 - Math.PI / 2;
		pts.push([cx + Math.cos(a) * r, cy + Math.sin(a) * r]);
	}
	return pts;
}

function guide(pts: Pt[], key?: string): Guide {
	const cum = [0];
	for (let i = 1; i < pts.length; i++) cum.push(cum[i - 1] + Math.hypot(pts[i][0] - pts[i - 1][0], pts[i][1] - pts[i - 1][1]));
	return { pts, cum, total: cum[cum.length - 1], last: 0, key };
}

/** The point at distance s along a guide, and the direction there. */
function at(g: Guide, s: number): { p: Pt; t: Pt } {
	s = Math.max(0, Math.min(g.total, s));
	let i = 1;
	while (i < g.pts.length - 1 && g.cum[i] < s) i++;
	const a = g.pts[i - 1];
	const b = g.pts[i];
	const seg = g.cum[i] - g.cum[i - 1] || 1;
	const u = (s - g.cum[i - 1]) / seg;
	return { p: [a[0] + (b[0] - a[0]) * u, a[1] + (b[1] - a[1]) * u], t: [(b[0] - a[0]) / seg, (b[1] - a[1]) / seg] };
}

/** The part of a guide between two distances, as path data. */
function span(g: Guide, s0: number, s1: number) {
	const a = Math.max(0, Math.min(s0, s1));
	const b = Math.min(g.total, Math.max(s0, s1));
	const pts: Pt[] = [at(g, a).p];
	for (let i = 1; i < g.pts.length - 1; i++) if (g.cum[i] > a && g.cum[i] < b) pts.push(g.pts[i]);
	pts.push(at(g, b).p);
	return 'M' + pts.map((p) => `${p[0].toFixed(1)} ${p[1].toFixed(1)}`).join('L');
}

/** Distance from a point to a guide, and how far along it the nearest point is. */
function nearest(g: Guide, x: number, y: number) {
	let best = Infinity;
	let s = 0;
	for (let i = 1; i < g.pts.length; i++) {
		const [ax, ay] = g.pts[i - 1];
		const [bx, by] = g.pts[i];
		const dx = bx - ax;
		const dy = by - ay;
		const l2 = dx * dx + dy * dy || 1;
		const u = Math.max(0, Math.min(1, ((x - ax) * dx + (y - ay) * dy) / l2));
		const d = Math.hypot(x - (ax + dx * u), y - (ay + dy * u));
		if (d < best) {
			best = d;
			s = g.cum[i - 1] + u * Math.sqrt(l2);
		}
	}
	return { d: best, s };
}

// ------------------------------------------------------------------ type metrics

const metricsCache = new Map<string, { asc: number; cap: number; xh: number; up: number }>();
const mctx = document.createElement('canvas').getContext('2d');

function metrics(cs: CSSStyleDeclaration) {
	const font = `${cs.fontStyle} ${cs.fontWeight} ${cs.fontSize} ${cs.fontFamily}`;
	let m = metricsCache.get(font);
	if (m || !mctx) return m ?? { asc: 0.9 * parseFloat(cs.fontSize), cap: 0.7 * parseFloat(cs.fontSize), xh: 0.5 * parseFloat(cs.fontSize), up: 0.75 * parseFloat(cs.fontSize) };
	mctx.font = font;
	const H = mctx.measureText('H');
	const x = mctx.measureText('x');
	const d = mctx.measureText('d');
	m = { asc: H.fontBoundingBoxAscent, cap: H.actualBoundingBoxAscent, xh: x.actualBoundingBoxAscent, up: d.actualBoundingBoxAscent };
	metricsCache.set(font, m);
	return m;
}

interface Line {
	y: number;
	x0: number;
	x1: number;
	cap: number;
	xh: number;
	up: number;
}

/** The baselines of an element's lines of text, in the layer's coordinates. */
function lines(el: HTMLElement, root: HTMLElement): Line[] {
	const origin = offsetBox(el, root);
	const er = el.getBoundingClientRect();
	const out: Line[] = [];
	const walker = document.createTreeWalker(el, NodeFilter.SHOW_TEXT);
	for (let n = walker.nextNode(); n; n = walker.nextNode()) {
		if (!n.textContent?.trim() || !n.parentElement) continue;
		const m = metrics(getComputedStyle(n.parentElement));
		const range = document.createRange();
		range.selectNodeContents(n);
		for (const r of range.getClientRects()) {
			if (r.width < 2) continue;
			const y = origin.y + (r.top - er.top) + m.asc;
			const x0 = origin.x + (r.left - er.left);
			const x1 = x0 + r.width;
			const same = out.find((l) => Math.abs(l.y - y) < 6);
			if (same) {
				same.x0 = Math.min(same.x0, x0);
				same.x1 = Math.max(same.x1, x1);
				same.cap = Math.max(same.cap, m.cap);
				same.xh = Math.max(same.xh, m.xh);
				same.up = Math.max(same.up, m.up);
			} else out.push({ y, x0, x1, cap: m.cap, xh: m.xh, up: m.up });
		}
	}
	return out.sort((a, b) => a.y - b.y);
}

// ------------------------------------------------------------------ rails

/** The rails sit between the edge of the screen and the text column. */
function rails(): [number, number] | null {
	const vw = document.documentElement.clientWidth;
	if (vw < 720) return null;
	const pad = Math.min(56, Math.max(20, vw * 0.04));
	const left = Math.max(0, (vw - 1360) / 2) + pad;
	const r = Math.min(44, Math.max(12, left - 24));
	return [crisp(r), crisp(vw - r) - 1];
}

// ------------------------------------------------------------------ drawing

type Opts = { shine?: boolean; key?: string; draw?: boolean };

function pen(L: Layer) {
	const mk = <K extends keyof SVGElementTagNameMap>(tag: K, attrs: Record<string, string | number>, parent: Element = L.base) => {
		const e = document.createElementNS(NS, tag);
		for (const k in attrs) e.setAttribute(k, String(attrs[k]));
		parent.append(e);
		return e;
	};
	const poly = (pts: Pt[], cls = 'g', o: Opts = {}) => {
		if (o.draw !== false) {
			const d = 'M' + pts.map((p) => `${p[0].toFixed(1)} ${p[1].toFixed(1)}`).join('L');
			mk('path', cls.includes('dash') ? { d, class: cls } : { d, class: cls, pathLength: 1 });
		}
		if (o.shine !== false && pts.length > 1) L.guides.push(guide(pts, o.key));
	};
	const line = (x1: number, y1: number, x2: number, y2: number, cls = 'g', o: Opts = {}) => {
		if (Math.abs(x1 - x2) < 0.01) x1 = x2 = crisp(x1);
		if (Math.abs(y1 - y2) < 0.01) y1 = y2 = crisp(y1);
		if (Math.hypot(x2 - x1, y2 - y1) < 2) return;
		poly(
			[
				[x1, y1],
				[x2, y2],
			],
			cls,
			o,
		);
	};
	return {
		poly,
		line,
		ghost: (pts: Pt[], key?: string) => poly(pts, '', { draw: false, key }),
		circle(cx: number, cy: number, r: number, cls = 'g', o: Opts = {}) {
			if (o.draw !== false) mk('circle', cls.includes('dash') ? { cx, cy, r, class: cls } : { cx, cy, r, class: cls, pathLength: 1 });
			if (o.shine !== false) L.guides.push(guide(circlePts(cx, cy, r), o.key));
		},
		node(x: number, y: number, s = 5) {
			mk('rect', { x: crisp(x) - s / 2, y: crisp(y) - s / 2, width: s, height: s, class: 'g-node' });
		},
		reg(x: number, y: number, r = 7) {
			mk('circle', { cx: x, cy: y, r, class: 'g-reg' });
			mk('path', { d: `M${x - r - 4} ${crisp(y)}H${x + r + 4}M${crisp(x)} ${y - r - 4}V${y + r + 4}`, class: 'g-reg' });
		},
		label(x: number, y: number, text: string, anchor: 'start' | 'middle' | 'end' = 'start', rotate = 0) {
			const t = mk('text', { x, y, class: 'g-label', 'text-anchor': anchor, ...(rotate ? { transform: `rotate(${rotate} ${x} ${y})` } : {}) });
			t.textContent = text;
		},
		/** Crop ticks at a corner: dx, dy point away from the sheet. */
		ticks(x: number, y: number, dx: number, dy: number, gap = 7, len = 15) {
			mk('path', {
				d: `M${x + dx * gap} ${crisp(y)}H${x + dx * (gap + len)}M${crisp(x)} ${y + dy * gap}V${y + dy * (gap + len)}`,
				class: 'g-tick',
			});
		},
		/** A dimension along x, with end ticks and the measure in the middle. */
		dim(x1: number, x2: number, y: number, text: string) {
			const mid = (x1 + x2) / 2;
			const half = text.length * 3.1 + 8;
			line(x1, y, mid - half, y, 'g g-dim', { shine: false });
			line(mid + half, y, x2, y, 'g g-dim', { shine: false });
			L.guides.push(guide([[x1, crisp(y)], [x2, crisp(y)]]));
			mk('path', { d: `M${crisp(x1)} ${y - 4}V${y + 4}M${crisp(x2)} ${y - 4}V${y + 4}`, class: 'g-tick' });
			this.label(mid, y + 3.5, text, 'middle');
		},
		/** A dimension along y. */
		vdim(x: number, y1: number, y2: number, text: string) {
			const mid = (y1 + y2) / 2;
			const half = text.length * 3.1 + 8;
			line(x, y1, x, mid - half, 'g g-dim', { shine: false });
			line(x, mid + half, x, y2, 'g g-dim', { shine: false });
			L.guides.push(guide([[crisp(x), y1], [crisp(x), y2]]));
			mk('path', { d: `M${x - 4} ${crisp(y1)}H${x + 4}M${x - 4} ${crisp(y2)}H${x + 4}`, class: 'g-tick' });
			this.label(x + 3.5, mid, text, 'middle', -90);
		},
	};
}
type Pen = ReturnType<typeof pen>;

// ------------------------------------------------------------------ what each section draws

function railsOf(L: Layer, p: Pen, from = 0) {
	const r = rails();
	if (!r) return null;
	p.line(r[0], from, r[0], L.height, 'g g-rail', { key: 'rail-l' });
	p.line(r[1], from, r[1], L.height, 'g g-rail', { key: 'rail-r' });
	return r;
}

/** The section's top rule (the CSS border is hidden once this draws it). */
function edge(L: Layer, p: Pen, r: [number, number] | null) {
	const cs = getComputedStyle(L.el);
	if (parseFloat(cs.borderTopWidth) > 0) {
		p.line(0, 0, L.width, 0, 'g g-edge');
		if (r) {
			p.node(r[0], 0.5);
			p.node(r[1], 0.5);
		}
	}
	if (parseFloat(cs.borderBottomWidth) > 0) {
		p.line(0, L.height - 1, L.width, L.height - 1, 'g g-edge');
		if (r) {
			p.node(r[0], L.height - 0.5);
			p.node(r[1], L.height - 0.5);
		}
	}
}

/** Baseline and cap height of a headline, out to the rails. */
function headline(L: Layer, p: Pen, h: HTMLElement, r: [number, number] | null) {
	const ls = lines(h, L.el);
	if (!ls.length) return;
	const x0 = r ? r[0] : 0;
	const x1 = r ? r[1] : L.width;
	const first = ls[0];
	const last = ls[ls.length - 1];
	p.line(x0, first.y - first.cap, x1, first.y - first.cap, 'g g-dash');
	p.line(x0, first.y, x1, first.y, 'g g-base');
	// The labels sit just past the words, as annotations of them; at the rail
	// when the line runs all the way across.
	const after = first.x1 + 22 < x1 - 80;
	const lx = after ? first.x1 + 22 : x1 - 8;
	const anchor = after ? 'start' : 'end';
	p.label(lx, first.y - first.cap - 6, 'cap height', anchor);
	p.label(lx, first.y - 6, 'baseline', anchor);
	p.node(first.x0, first.y);
	if (last !== first) {
		p.line(x0, last.y, x1, last.y, 'g g-base');
		p.node(last.x0, last.y);
	}
}

interface SheetOpts {
	head?: Element | null;
	caption?: Element | null;
	dims?: string;
	flip?: boolean;
	dimAbove?: number;
	from?: number;
	to?: number;
	nodes?: boolean;
}

/** A page's trim carried out to the rails, crop ticks at its corners, its size. */
function sheet(L: Layer, p: Pen, trim: Box, outer: Box, r: [number, number] | null, o: SheetOpts) {
	const from = o.from ?? (r ? r[0] : 0);
	const to = o.to ?? (r ? r[1] : L.width);
	p.line(from, trim.y, to, trim.y);
	p.line(from, trim.y2, to, trim.y2);
	const top = o.head ? offsetBox(o.head as HTMLElement, L.el).y2 + 22 : outer.y - 64;
	const bottom = o.caption ? offsetBox(o.caption as HTMLElement, L.el).y - 12 : outer.y2 + 64;
	p.line(trim.x, top, trim.x, bottom);
	p.line(trim.x2, top, trim.x2, bottom);
	// Ticks just outside the paper, on the trim lines.
	const gx = trim.x - outer.x + 7;
	const gy = trim.y - outer.y + 7;
	p.ticks(trim.x, trim.y, -1, -1, Math.max(7, gx), 14);
	p.ticks(trim.x2, trim.y, 1, -1, Math.max(7, outer.x2 - trim.x2 + 7), 14);
	p.ticks(trim.x, trim.y2, -1, 1, Math.max(7, gy), 14);
	p.ticks(trim.x2, trim.y2, 1, 1, Math.max(7, outer.y2 - trim.y2 + 7), 14);
	if (r && o.nodes !== false) {
		for (const y of [trim.y, trim.y2]) {
			if (from <= r[0] + 1) p.node(r[0], y);
			if (to >= r[1] - 1) p.node(r[1], y);
		}
	}
	const [dw, dh] = (o.dims || '').split('|');
	if (dw) p.dim(trim.x, trim.x2, outer.y - (o.dimAbove ?? 20), dw);
	if (dh) {
		const room = o.flip ? outer.x - (r ? r[0] : 0) : (r ? r[1] : L.width) - outer.x2;
		if (room > 34) p.vdim(o.flip ? outer.x - 18 : outer.x2 + 18, trim.y, trim.y2, dh);
	}
}

function plateSheet(L: Layer, p: Pen, plate: HTMLElement, r: [number, number] | null, extra: Partial<SheetOpts> = {}) {
	const sheets = [...plate.querySelectorAll<HTMLElement>('.sheet')].filter((s) => s.offsetParent !== null);
	if (!sheets.length) return;
	const outer = union(sheets.map((s) => offsetBox(s, L.el)));
	const t = sheets.length === 1 ? sheets[0].querySelector<HTMLElement>('[data-trim]') : null;
	const trim = t ? offsetBox(t, L.el) : outer;
	const stage = plate.querySelector<HTMLElement>('[data-stage]');
	const kind = [...plate.classList].find((c) => c.startsWith('plate--') && c !== 'plate--flip')?.slice(7);
	sheet(L, p, trim, outer, r, {
		head: plate.querySelector('.plate__head'),
		caption: plate.querySelector('.plate__caption'),
		dims: stage?.dataset.dims,
		flip: plate.classList.contains('plate--flip'),
		dimAbove: kind === 'cover' ? 54 : 20,
		...extra,
	});
	// The fold of a spread, marked above and below.
	if (sheets.length === 2) {
		const g = offsetBox(sheets[1], L.el).x;
		p.line(g, outer.y - 26, g, outer.y - 8, 'g g-dash', { shine: false });
		p.line(g, outer.y2 + 8, g, outer.y2 + 26, 'g g-dash', { shine: false });
	}
}

const builders: Record<string, (L: Layer, p: Pen) => void> = {
	hero(L, p) {
		const r = rails();
		if (r) {
			// The rails start at the bottom crop marks and run down the page.
			p.line(r[0], L.height - 18, r[0], L.height, 'g g-rail', { key: 'rail-l' });
			p.line(r[1], L.height - 18, r[1], L.height, 'g g-rail', { key: 'rail-r' });
		}
		// The glyph's construction (its own SVG draws it): light runs along it too.
		const glyph = L.el.querySelector<HTMLElement>('[data-glyph]');
		if (!glyph) return;
		const b = offsetBox(glyph, L.el);
		const k = b.w / 100;
		const T = (x: number, y: number): Pt => [b.x + x * k, b.y + y * k];
		for (const ln of glyph.querySelectorAll<SVGLineElement>('.hero__construct line')) {
			const a = T(+ln.getAttribute('x1')!, +ln.getAttribute('y1')!);
			const c = T(+ln.getAttribute('x2')!, +ln.getAttribute('y2')!);
			a[0] = Math.max(0, a[0]);
			c[0] = Math.min(L.width, c[0]);
			c[1] = Math.min(L.height, c[1]);
			p.ghost([a, c]);
		}
		for (const c of glyph.querySelectorAll<SVGCircleElement>('.hero__construct circle')) {
			const [cx, cy] = T(+c.getAttribute('cx')!, +c.getAttribute('cy')!);
			p.ghost(circlePts(cx, cy, +c.getAttribute('r')! * k));
		}
	},
	marquee(L, p) {
		const r = railsOf(L, p);
		edge(L, p, r);
	},
	intro(L, p) {
		const r = railsOf(L, p);
		for (const h of L.el.querySelectorAll<HTMLElement>('[data-baselines]')) headline(L, p, h, r);
	},
	plate(L, p) {
		const r = railsOf(L, p);
		edge(L, p, r);
		plateSheet(L, p, L.el, r);
		// The title's baseline, from the rail to just past the title.
		const t = L.el.querySelector<HTMLElement>('.plate__title');
		if (t) {
			const ls = lines(t, L.el);
			const last = ls[ls.length - 1];
			if (last) {
				p.line(r ? r[0] : 0, last.y, last.x1 + 28, last.y, 'g g-base');
				p.node(last.x1 + 28, last.y);
			}
		}
	},
	duo(L, p) {
		const r = railsOf(L, p);
		edge(L, p, r);
		const plates = [...L.el.querySelectorAll<HTMLElement>('.plate')];
		plates.forEach((pl, i) => {
			const col = offsetBox(pl, L.el);
			plateSheet(L, p, pl, r, {
				from: i === 0 && r ? r[0] : col.x - 16,
				to: i === plates.length - 1 && r ? r[1] : col.x2 + 16,
				flip: i === 0,
			});
		});
	},
	aldus(L, p) {
		const r = railsOf(L, p);
		edge(L, p, r);
		for (const h of L.el.querySelectorAll<HTMLElement>('[data-baselines]')) headline(L, p, h, r);
		// The bookplate: a double rule, its corners pinned, carried out to the rail.
		const bp = L.el.querySelector<HTMLElement>('[data-bookplate]');
		if (bp) {
			const b = offsetBox(bp, L.el);
			const rect = (q: Box, cls: string) =>
				p.poly(
					[
						[crisp(q.x), crisp(q.y)],
						[crisp(q.x2), crisp(q.y)],
						[crisp(q.x2), crisp(q.y2)],
						[crisp(q.x), crisp(q.y2)],
						[crisp(q.x), crisp(q.y)],
					],
					cls,
				);
			rect(b, 'g g-strong');
			rect(box(b.x + 7, b.y + 7, b.w - 14, b.h - 14), 'g g-dash');
			for (const [x, y] of [
				[b.x, b.y],
				[b.x2, b.y],
				[b.x, b.y2],
				[b.x2, b.y2],
			])
				p.node(x, y, 7);
			if (r) {
				p.line(b.x2, b.y, r[1], b.y);
				p.line(b.x2, b.y2, r[1], b.y2);
				p.node(r[1], b.y);
				p.node(r[1], b.y2);
			}
		}
		// The wall's rows sit between two rules.
		for (const row of L.el.querySelectorAll<HTMLElement>('.wall__row')) {
			const img = row.querySelector<HTMLElement>('img');
			if (!img) continue;
			const b = offsetBox(img, L.el);
			p.line(0, b.y - 14, L.width, b.y - 14);
			p.line(0, b.y2 + 14, L.width, b.y2 + 14, 'g g-dash');
		}
	},
	talks(L, p) {
		const r = railsOf(L, p);
		edge(L, p, r);
		for (const h of L.el.querySelectorAll<HTMLElement>('[data-baselines]')) headline(L, p, h, r);
		const page = L.el.querySelector<HTMLElement>('[data-proof-page]');
		if (page) {
			const b = offsetBox(page, L.el);
			sheet(L, p, b, b, r, { head: L.el.querySelector('.talks__head'), caption: L.el.querySelector('.proof__caption'), dims: '6 in|9 in', flip: true });
		}
		const caps = L.el.querySelector<HTMLElement>('[data-caps]');
		if (caps) {
			const c = offsetBox(caps, L.el);
			const x0 = c.x + parseFloat(getComputedStyle(caps).paddingLeft);
			p.ghost([
				[x0, c.y + 0.5],
				[c.x2 - parseFloat(getComputedStyle(caps).paddingRight), c.y + 0.5],
			]);
			if (r) p.line(r[0], c.y, x0, c.y, 'g g-dash');
		}
	},
	anywhere(L, p) {
		const r = railsOf(L, p);
		edge(L, p, r);
		for (const h of L.el.querySelectorAll<HTMLElement>('[data-baselines]')) headline(L, p, h, r);
		const tabs = [...L.el.querySelectorAll<HTMLElement>('.rt__tab')];
		tabs.forEach((t, i) => {
			const b = offsetBox(t, L.el);
			if (i === 0) p.line(r ? r[0] : 0, b.y, b.x2, b.y);
			p.line(r ? r[0] : 0, b.y2, b.x2, b.y2);
			if (r) {
				if (i === 0) p.node(r[0], b.y);
				p.node(r[0], b.y2);
			}
		});
		const s = L.el.querySelector<HTMLElement>('[data-press-sheet]');
		if (s) {
			const b = offsetBox(s, L.el);
			sheet(L, p, b, b, r, { head: L.el.querySelector('.anywhere__head'), caption: L.el.querySelector('[data-press-fp]'), dims: '210 mm|297 mm' });
			// From the hash the code returns to the one under the page.
			const out = L.el.querySelector<HTMLElement>('.rt__panel:not([hidden]) .rt__hash');
			const fp = L.el.querySelector<HTMLElement>('.press__hash');
			if (out && fp) {
				const o = offsetBox(out, L.el);
				const f = offsetBox(fp, L.el);
				const y0 = crisp(o.y + o.h / 2);
				const y1 = crisp(f.y + f.h / 2);
				const gx = crisp(Math.max(o.x2 + 24, b.x - 28));
				const pts: Pt[] = [
					[o.x2 + 12, y0],
					[gx, y0],
					[gx, y1],
					[f.x - 12, y1],
				];
				p.poly(pts, 'g g-accent g-dash', { key: 'connector' });
				p.node(o.x2 + 12, y0, 5);
				p.node(f.x - 12, y1, 5);
			}
		}
	},
	closing(L, p) {
		const r = railsOf(L, p);
		edge(L, p, r);
		for (const h of L.el.querySelectorAll<HTMLElement>('[data-baselines]')) headline(L, p, h, r);
		// A registration target behind the command: rings, an axis, a cross.
		const cmd = L.el.querySelector<HTMLElement>('[data-closing-cmd]');
		if (cmd) {
			const c = offsetBox(cmd, L.el);
			const cx = c.x + c.w / 2;
			const cy = c.y + c.h / 2;
			const k = Math.min(1, L.width / 1440);
			p.circle(cx, cy, 118 * k);
			p.circle(cx, cy, 176 * k, 'g g-dash');
			p.circle(cx, cy, 252 * k, 'g g-accent');
			p.line(r ? r[0] : 0, cy, r ? r[1] : L.width, cy, 'g g-dash');
			p.line(cx, cy - 290 * k, cx, cy + 290 * k);
			p.reg(cx, cy - 252 * k, 6);
			p.reg(cx, cy + 252 * k, 6);
			if (r) {
				p.node(r[0], cy);
				p.node(r[1], cy);
			}
		}
	},
	footer(L, p) {
		const r = railsOf(L, p);
		edge(L, p, r);
		const slug = L.el.querySelector<HTMLElement>('.footer__slug');
		if (slug) {
			const s = offsetBox(slug, L.el);
			p.line(r ? r[0] : 0, s.y, r ? r[1] : L.width, s.y);
		}
		// The wordmark set on its lines, as a specimen sheet would show it.
		const word = L.el.querySelector<HTMLElement>('.footword__screen');
		if (word) {
			const ls = lines(word, L.el);
			const l = ls[0];
			if (l) {
				const x0 = 0;
				const x1 = L.width;
				const lab = r ? r[1] - 8 : L.width - 12;
				p.line(x0, l.y - l.up, x1, l.y - l.up, 'g g-dash');
				p.line(x0, l.y - l.xh, x1, l.y - l.xh, 'g g-accent');
				p.line(x0, l.y, x1, l.y, 'g g-base');
				p.label(lab, l.y - l.up - 6, 'ascender', 'end');
				p.label(lab, l.y - l.xh - 6, `x-height ${(l.xh / parseFloat(getComputedStyle(word).fontSize)).toFixed(2)} em`, 'end');
			}
		}
	},
};

// ------------------------------------------------------------------ colour

const LIGHT = ['#00a3e0', '#2f45ff', '#7a3cff', '#e5007d', '#ff6a00'];
const DARK = ['#3ad6ff', '#7d8bff', '#b98cff', '#ff4fb0', '#ffa24d'];
const rgb = (h: string) => [parseInt(h.slice(1, 3), 16), parseInt(h.slice(3, 5), 16), parseInt(h.slice(5, 7), 16)];
const RAMPS = { light: LIGHT.map(rgb), dark: DARK.map(rgb) };

/** The ink at a fraction of the page's height. */
function ink(f: number, dark: boolean) {
	const ramp = dark ? RAMPS.dark : RAMPS.light;
	const x = Math.max(0, Math.min(0.9999, f)) * (ramp.length - 1);
	const i = Math.floor(x);
	const u = x - i;
	const a = ramp[i];
	const b = ramp[i + 1];
	return `rgb(${Math.round(a[0] + (b[0] - a[0]) * u)} ${Math.round(a[1] + (b[1] - a[1]) * u)} ${Math.round(a[2] + (b[2] - a[2]) * u)})`;
}

// ------------------------------------------------------------------ the layer

export interface Guides {
	refresh(el?: Element | null): void;
	pulse(el: Element | null, key: string): void;
}

export function mountGuides({ reduced }: { reduced: boolean }): Guides {
	const layers: Layer[] = [];
	const pulses: Pulse[] = [];
	const fine = matchMedia('(hover: hover) and (pointer: fine)').matches;
	let raf = 0;
	let last = 0;
	let uid = 0;

	for (const el of document.querySelectorAll<HTMLElement>('[data-guides]')) {
		const svg = document.createElementNS(NS, 'svg');
		svg.setAttribute('class', 'guides');
		svg.setAttribute('aria-hidden', 'true');
		const defs = document.createElementNS(NS, 'defs');
		const base = document.createElementNS(NS, 'g');
		const fx = document.createElementNS(NS, 'g');
		fx.setAttribute('class', 'guides__fx');
		svg.append(defs, base, fx);
		// Behind everything the section holds, above its own background.
		el.prepend(svg);
		el.classList.add('has-guides');
		layers.push({ el, kind: el.dataset.guides || '', svg, defs, base, fx, guides: [], free: [], top: 0, width: 0, height: 0, visible: false, drawn: reduced, uid: `gl${uid++}` });
	}

	function build(L: Layer) {
		for (const q of pulses.filter((q) => q.layer === L)) kill(q);
		L.base.replaceChildren();
		L.guides = [];
		L.width = L.el.clientWidth;
		L.height = L.el.clientHeight;
		L.top = L.el.getBoundingClientRect().top + window.scrollY;
		const p = pen(L);
		try {
			builders[L.kind]?.(L, p);
		} catch (err) {
			console.warn('guides:', L.kind, err);
		}
		if (!L.drawn) L.base.classList.add('is-pending');
		if (L.visible && !L.drawn) drawIn(L);
	}

	function drawIn(L: Layer) {
		L.drawn = true;
		const paths = [...L.base.querySelectorAll<SVGGeometryElement>('[pathLength]')];
		const rest = [...L.base.children].filter((e) => !e.hasAttribute('pathLength'));
		paths.forEach((e, i) =>
			e.animate([{ strokeDashoffset: 1 }, { strokeDashoffset: 0 }], { duration: 1400, delay: Math.min(i * 40, 700), easing: 'cubic-bezier(0.22, 1, 0.36, 1)', fill: 'backwards' }),
		);
		rest.forEach((e, i) => e.animate([{ opacity: 0 }, { opacity: 1 }], { duration: 700, delay: 300 + Math.min(i * 25, 600), fill: 'backwards' }));
		L.base.classList.remove('is-pending');
	}

	function buildAll() {
		for (const L of layers) build(L);
	}

	// ---------------------------------------------------------------- pulses

	function sprite(L: Layer): Sprite {
		const s = L.free.pop();
		if (s) {
			s.glow.style.display = s.core.style.display = '';
			return s;
		}
		const id = `${L.uid}-${uid++}`;
		const grad = document.createElementNS(NS, 'linearGradient');
		grad.id = id;
		grad.setAttribute('gradientUnits', 'userSpaceOnUse');
		const stops = [0, 1, 2].map(() => {
			const st = document.createElementNS(NS, 'stop');
			grad.append(st);
			return st;
		});
		L.defs.append(grad);
		const glow = document.createElementNS(NS, 'path');
		glow.setAttribute('class', 'g-glow');
		glow.setAttribute('stroke', `url(#${id})`);
		const core = document.createElementNS(NS, 'path');
		core.setAttribute('class', 'g-core');
		core.setAttribute('stroke', `url(#${id})`);
		L.fx.append(glow, core);
		return { grad, stops, glow, core };
	}

	function kill(q: Pulse) {
		const i = pulses.indexOf(q);
		if (i >= 0) pulses.splice(i, 1);
		if (q.sprite) {
			q.sprite.glow.style.display = q.sprite.core.style.display = 'none';
			q.layer.free.push(q.sprite);
			q.sprite = undefined;
		}
		if (q.g.touch === q) q.g.touch = undefined;
	}

	function spawn(q: Omit<Pulse, 'sprite' | 'age' | 'handed'> & { age?: number }) {
		const full: Pulse = { ...q, age: q.age ?? 0, handed: false };
		pulses.push(full);
		if (pulses.length > 90) kill(pulses[0]);
		if (!raf) {
			last = 0;
			raf = requestAnimationFrame(frame);
		}
		return full;
	}

	function render(q: Pulse, dark: boolean) {
		const g = q.g;
		const head = q.still ? q.s + q.len / 2 : q.s;
		const tail = q.still ? q.s - q.len / 2 : q.s - q.dir * q.len;
		const a = Math.max(0, Math.min(g.total, tail));
		const b = Math.max(0, Math.min(g.total, head));
		const fade = Math.max(0, 1 - q.age / q.life);
		if (Math.abs(b - a) < 1 || fade <= 0) {
			if (q.sprite) {
				q.sprite.glow.style.display = q.sprite.core.style.display = 'none';
				q.layer.free.push(q.sprite);
				q.sprite = undefined;
			}
			return;
		}
		if (!q.sprite) q.sprite = sprite(q.layer);
		const s = q.sprite;
		const pt = at(g, b).p;
		const docY = q.layer.top + pt[1];
		const color = ink(docY / document.documentElement.scrollHeight, dark);
		const vy = (docY - window.scrollY) / window.innerHeight;
		const centre = 0.55 + 0.45 * Math.cos(Math.max(-1, Math.min(1, vy - 0.5)) * Math.PI);
		const alpha = Math.min(1, q.amp * fade * centre * (q.still ? 0.8 : 1));
		const pa = at(g, a).p;
		s.grad.setAttribute('x1', pa[0].toFixed(1));
		s.grad.setAttribute('y1', pa[1].toFixed(1));
		s.grad.setAttribute('x2', pt[0].toFixed(1));
		s.grad.setAttribute('y2', pt[1].toFixed(1));
		const offs = q.still ? [0, 0.5, 1] : [0, 0.62, 1];
		const ops = q.still ? [0, alpha, 0] : [0, alpha * 0.55, alpha];
		s.stops.forEach((st, i) => {
			st.setAttribute('offset', String(offs[i]));
			st.setAttribute('stop-color', color);
			st.setAttribute('stop-opacity', ops[i].toFixed(3));
		});
		const d = span(g, a, b);
		s.glow.setAttribute('d', d);
		s.core.setAttribute('d', d);
	}

	function frame(now: number) {
		raf = 0;
		const dt = Math.min(0.05, last ? (now - last) / 1000 : 0.016);
		last = now;
		const dark = document.documentElement.dataset.theme === 'dark';
		for (const q of [...pulses]) {
			q.age += dt;
			if (!q.still) {
				q.s += q.dir * q.speed * dt;
				q.speed *= Math.exp(-0.9 * dt);
				// A rail's light goes on into the next section's rail.
				if (q.g.key && !q.handed && (q.dir > 0 ? q.s > q.g.total : q.s < 0)) {
					q.handed = true;
					const i = layers.indexOf(q.layer);
					const next = layers[i + q.dir];
					const g2 = next?.guides.find((g) => g.key === q.g.key);
					if (next && g2) {
						spawn({ layer: next, g: g2, s: q.dir > 0 ? q.s - q.g.total : g2.total + q.s, dir: q.dir, speed: q.speed, len: q.len, life: q.life, amp: q.amp * 0.92, still: false, age: q.age });
					}
				}
			}
			const gone = q.age >= q.life || (!q.still && (q.dir > 0 ? q.s - q.len > q.g.total : q.s + q.len < 0));
			if (gone) kill(q);
			else render(q, dark);
		}
		if (pulses.length) raf = requestAnimationFrame(frame);
	}

	// ---------------------------------------------------------------- pointer

	const ptr = { cx: -1e4, cy: -1e4, x: -1e4, y: -1e4, vx: 0, vy: 0, t: 0, on: false };
	const R = 64;

	function sample() {
		if (!ptr.on) return;
		const now = performance.now();
		const x = ptr.cx + window.scrollX;
		const y = ptr.cy + window.scrollY;
		const dt = Math.max(8, now - ptr.t) / 1000;
		if (ptr.t && dt < 0.25) {
			ptr.vx += ((x - ptr.x) / dt - ptr.vx) * 0.5;
			ptr.vy += ((y - ptr.y) / dt - ptr.vy) * 0.5;
		} else {
			ptr.vx = ptr.vy = 0;
		}
		ptr.x = x;
		ptr.y = y;
		ptr.t = now;
		probe(now);
	}

	function probe(now: number) {
		const speed = Math.hypot(ptr.vx, ptr.vy);
		for (const L of layers) {
			if (!L.visible) continue;
			const lx = ptr.x;
			const ly = ptr.y - L.top;
			if (ly < -R || ly > L.height + R) continue;
			for (const g of L.guides) {
				const n = nearest(g, lx, ly);
				if (n.d > R) continue;
				const prox = 1 - n.d / R;
				// A glow under the pointer that slides along with it.
				if (g.touch) {
					g.touch.s = n.s;
					g.touch.age = 0;
					g.touch.amp = Math.max(g.touch.amp * 0.6, prox * 0.55);
				} else {
					g.touch = spawn({ layer: L, g, s: n.s, dir: 1, speed: 0, len: 90, life: 0.45, amp: prox * 0.55, still: true });
				}
				if (speed < 80 || now - g.last < 110) continue;
				g.last = now;
				const t = at(g, n.s).t;
				const along = ptr.vx * t[0] + ptr.vy * t[1];
				const dir: 1 | -1 = Math.abs(along) > speed * 0.3 ? (along > 0 ? 1 : -1) : Math.random() < 0.5 ? -1 : 1;
				const v = Math.min(2200, 260 + Math.abs(along) * 0.8 + speed * 0.3);
				const amp = prox * Math.min(1, 0.45 + speed / 1800);
				spawn({ layer: L, g, s: n.s, dir, speed: v, len: Math.min(360, 90 + speed * 0.14), life: 1.25 + Math.random() * 0.5, amp, still: false });
				if (speed > 1300 && Math.random() < 0.55) {
					spawn({ layer: L, g, s: n.s, dir: dir > 0 ? -1 : 1, speed: v * 0.6, len: 80 + speed * 0.06, life: 0.9, amp: amp * 0.6, still: false });
				}
			}
		}
	}

	if (fine && !reduced) {
		window.addEventListener(
			'pointermove',
			(e) => {
				if (e.pointerType === 'touch') return;
				ptr.cx = e.clientX;
				ptr.cy = e.clientY;
				ptr.on = true;
				sample();
			},
			{ passive: true },
		);
		document.documentElement.addEventListener('pointerleave', () => {
			ptr.on = false;
			ptr.t = 0;
		});
		window.addEventListener('scroll', sample, { passive: true });
	}

	// ---------------------------------------------------------------- upkeep

	const io = new IntersectionObserver(
		(entries) => {
			for (const en of entries) {
				const L = layers.find((l) => l.el === en.target);
				if (!L) continue;
				L.visible = en.isIntersecting;
				if (L.visible && !L.drawn) drawIn(L);
			}
		},
		{ rootMargin: '0px 0px -8% 0px' },
	);

	let timer = 0;
	const soon = () => {
		clearTimeout(timer);
		timer = window.setTimeout(buildAll, 140);
	};
	let ready = false;
	const sizes = new WeakMap<Element, string>();
	const ro = new ResizeObserver((entries) => {
		if (!ready) return;
		for (const en of entries) {
			const key = `${Math.round(en.contentRect.width)}x${Math.round(en.contentRect.height)}`;
			if (sizes.get(en.target) === key) continue;
			sizes.set(en.target, key);
			soon();
		}
	});

	(document.fonts?.ready ?? Promise.resolve()).then(() => {
		buildAll();
		ready = true;
		for (const L of layers) {
			sizes.set(L.el, `${Math.round(L.el.clientWidth)}x${Math.round(L.el.clientHeight)}`);
			ro.observe(L.el);
			io.observe(L.el);
		}
	});
	window.addEventListener('resize', soon);

	return {
		refresh(el) {
			const L = layers.find((l) => l.el === el);
			if (L) build(L);
			else soon();
		},
		pulse(el, key) {
			const L = layers.find((l) => l.el === el);
			const g = L?.guides.find((x) => x.key === key);
			if (!L || !g || reduced) return;
			spawn({ layer: L, g, s: 0, dir: 1, speed: Math.max(700, g.total * 1.6), len: Math.min(260, g.total * 0.45), life: 1.6, amp: 1, still: false });
		},
	};
}
