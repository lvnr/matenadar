/**
 * The hero's Մ: the mark's geometry printed as a halftone, one dot per
 * screen cell, in three plates (cyan, magenta, yellow) that multiply to
 * black when they are in register. On a dark screen they are light, and
 * add up to white. The pointer pushes the dots; they spring home, and the
 * plates drift apart while they move, as a misregistered print does.
 *
 * Geometry is on the mark's 100-unit grid: stems at x 22.5 and 64.5, the
 * bowl a half circle of radius 21 about (43.5, 58), the arm at y 21, one
 * 14-unit stroke throughout.
 */

type Part = 'left stem' | 'bowl' | 'right stem' | 'arm';

interface Dot {
	hx: number;
	hy: number;
	x: number;
	y: number;
	vx: number;
	vy: number;
	r: number;
	delay: number;
}

const T = 7; // half the stroke

function box(px: number, py: number, cx: number, cy: number, hx: number, hy: number) {
	const qx = Math.abs(px - cx) - hx;
	const qy = Math.abs(py - cy) - hy;
	return Math.hypot(Math.max(qx, 0), Math.max(qy, 0)) + Math.min(Math.max(qx, qy), 0);
}

/** Signed distance to each part of the glyph, in grid units (negative inside). */
function parts(x: number, y: number): [Part, number][] {
	const bowl = Math.max(Math.abs(Math.hypot(x - 43.5, y - 58) - 21) - T, 58 - y);
	return [
		['left stem', box(x, y, 22.5, 36, T, 22)],
		['bowl', bowl],
		['right stem', box(x, y, 64.5, 36, T, 22)],
		['arm', box(x, y, 71, 21, 13.5, T)],
	];
}

function sdf(x: number, y: number) {
	let d = Infinity;
	for (const [, v] of parts(x, y)) d = Math.min(d, v);
	return d;
}

export function partAt(x: number, y: number): Part | null {
	let best: Part | null = null;
	let d = 1.2;
	for (const [p, v] of parts(x, y)) {
		if (v < d) {
			d = v;
			best = p;
		}
	}
	return best;
}

export interface GlyphOptions {
	/** Called with the pointer in grid units and the part under it. */
	onPoint?: (x: number, y: number, part: Part | null) => void;
	reduced?: boolean;
}

export function mountGlyph(canvas: HTMLCanvasElement, opts: GlyphOptions = {}) {
	const ctx = canvas.getContext('2d', { alpha: true });
	if (!ctx) return () => {};
	let dots: Dot[] = [];
	let size = 0; // css px for 100 units
	let dpr = 1;
	let pitch = 1.5; // screen cell, in grid units
	let dark = document.documentElement.dataset.theme === 'dark';
	let running = false;
	let visible = true;
	let raf = 0;
	let last = 0;
	let born = 0;
	const pointer = { x: -1e4, y: -1e4, px: -1e4, py: -1e4, vx: 0, vy: 0, inside: false };
	let shock: { x: number; y: number; t: number } | null = null;

	function layout() {
		const rect = canvas.getBoundingClientRect();
		size = Math.min(rect.width, rect.height);
		dpr = Math.min(window.devicePixelRatio || 1, size > 520 ? 2 : 1.6);
		canvas.width = Math.round(rect.width * dpr);
		canvas.height = Math.round(rect.height * dpr);
		// Fewer, larger dots on small screens.
		pitch = size < 380 ? 2.1 : size < 560 ? 1.75 : 1.5;
		const ang = Math.PI / 4; // the key plate's screen angle
		const ca = Math.cos(ang);
		const sa = Math.sin(ang);
		const next: Dot[] = [];
		const ox = (rect.width - size) / 2;
		const oy = (rect.height - size) / 2;
		for (let i = -110; i <= 110; i++) {
			for (let j = -110; j <= 110; j++) {
				const u = i * pitch;
				const v = j * pitch;
				const gx = 50 + u * ca - v * sa;
				const gy = 50 + u * sa + v * ca;
				if (gx < 8 || gx > 92 || gy < 8 || gy > 92) continue;
				const d = sdf(gx, gy);
				// Ink coverage across a soft edge, as a scanned stroke would have.
				const cov = Math.min(1, Math.max(0, 0.5 - d / 2.6));
				if (cov <= 0.02) continue;
				const hx = ox + (gx / 100) * size;
				const hy = oy + (gy / 100) * size;
				next.push({
					hx,
					hy,
					x: hx,
					y: hy,
					vx: 0,
					vy: 0,
					r: Math.sqrt(cov) * pitch * 0.47 * (size / 100),
					delay: gx / 100,
				});
			}
		}
		if (!dots.length && !opts.reduced) {
			// The first pass: dots arrive from scattered positions, left to right.
			for (const d of next) {
				const a = Math.random() * Math.PI * 2;
				const k = 30 + Math.random() * 120;
				d.x = d.hx + Math.cos(a) * k;
				d.y = d.hy + Math.sin(a) * k;
			}
			born = performance.now();
		}
		dots = next;
		draw(performance.now());
	}

	function colors() {
		return dark
			? { mode: 'screen' as GlobalCompositeOperation, inks: ['#00e5ff', '#ff2fb0', '#ffe600'] }
			: { mode: 'multiply' as GlobalCompositeOperation, inks: ['#00a3e0', '#e5007d', '#ffd200'] };
	}

	function step(dt: number, now: number) {
		const R = size * 0.16;
		const speed = Math.min(3, Math.hypot(pointer.vx, pointer.vy) / 900);
		const push = (0.9 + speed) * size * 18;
		const k = 120; // spring
		const c = 13; // damping
		let energy = 0;
		const age = (now - born) / 1000;
		for (const d of dots) {
			if (age < d.delay * 0.9) continue;
			let ax = -k * (d.x - d.hx) - c * d.vx;
			let ay = -k * (d.y - d.hy) - c * d.vy;
			if (pointer.inside) {
				const dx = d.x - pointer.x;
				const dy = d.y - pointer.y;
				const dist = Math.hypot(dx, dy) || 1;
				if (dist < R) {
					const f = (1 - dist / R) ** 2 * push;
					ax += (dx / dist) * f - (dy / dist) * f * 0.25;
					ay += (dy / dist) * f + (dx / dist) * f * 0.25;
				}
			}
			if (shock) {
				const t = (now - shock.t) / 1000;
				const ring = t * size * 1.4;
				const dx = d.x - shock.x;
				const dy = d.y - shock.y;
				const dist = Math.hypot(dx, dy) || 1;
				const band = Math.abs(dist - ring);
				if (band < size * 0.06 && t < 0.9) {
					const f = (1 - band / (size * 0.06)) * size * 90 * (1 - t);
					ax += (dx / dist) * f;
					ay += (dy / dist) * f;
				}
			}
			d.vx += ax * dt;
			d.vy += ay * dt;
			d.x += d.vx * dt;
			d.y += d.vy * dt;
			energy += Math.abs(d.vx) + Math.abs(d.vy) + Math.abs(d.x - d.hx) + Math.abs(d.y - d.hy);
		}
		if (shock && now - shock.t > 1000) shock = null;
		return energy / Math.max(1, dots.length);
	}

	function draw(now: number) {
		const { mode, inks } = colors();
		const w = canvas.width;
		const h = canvas.height;
		ctx!.setTransform(1, 0, 0, 1, 0, 0);
		ctx!.globalCompositeOperation = 'source-over';
		ctx!.clearRect(0, 0, w, h);
		ctx!.setTransform(dpr, 0, 0, dpr, 0, 0);
		ctx!.globalCompositeOperation = mode;
		const age = (now - born) / 1000;
		const breathe = opts.reduced || !running ? 0 : now / 1000;
		// Each plate is offset along the dot's motion: cyan behind, magenta
		// ahead, yellow across. In register when the dot is still.
		const shifts = [
			[-0.022, 0],
			[0.022, 0],
			[0, 0.026],
		];
		for (let p = 0; p < 3; p++) {
			ctx!.fillStyle = inks[p];
			ctx!.beginPath();
			const [sa, sb] = shifts[p];
			for (const d of dots) {
				const show = opts.reduced ? 1 : Math.min(1, Math.max(0, (age - d.delay * 0.9) / 0.35));
				if (show <= 0) continue;
				const wave = 1 + 0.07 * Math.sin(breathe * 1.3 + d.hx * 0.018 + d.hy * 0.011);
				const r = d.r * show * wave;
				const x = d.x + d.vx * sa - d.vy * sb;
				const y = d.y + d.vy * sa + d.vx * sb;
				ctx!.moveTo(x + r, y);
				ctx!.arc(x, y, r, 0, Math.PI * 2);
			}
			ctx!.fill();
		}
	}

	function frame(now: number) {
		raf = 0;
		const dt = Math.min(0.033, (now - (last || now)) / 1000 || 0.016);
		last = now;
		// Pointer velocity, smoothed.
		const ivx = (pointer.x - pointer.px) / Math.max(dt, 0.001);
		const ivy = (pointer.y - pointer.py) / Math.max(dt, 0.001);
		pointer.vx += (ivx - pointer.vx) * 0.35;
		pointer.vy += (ivy - pointer.vy) * 0.35;
		pointer.px = pointer.x;
		pointer.py = pointer.y;
		// Two substeps keep the spring steady at low frame rates.
		step(dt / 2, now);
		const energy = step(dt / 2, now);
		draw(now);
		// At rest and untouched, stop drawing until the pointer comes back.
		const settled = !pointer.inside && !shock && energy < 0.02 && now - born > 2600;
		if (settled) {
			running = false;
			draw(now);
		}
		if (running && visible) raf = requestAnimationFrame(frame);
	}

	function start() {
		if (opts.reduced) return draw(performance.now());
		running = true;
		if (!raf && visible) {
			last = 0;
			raf = requestAnimationFrame(frame);
		}
	}

	function toGrid(e: PointerEvent) {
		const rect = canvas.getBoundingClientRect();
		const ox = (rect.width - size) / 2;
		const oy = (rect.height - size) / 2;
		return { cx: e.clientX - rect.left, cy: e.clientY - rect.top, gx: ((e.clientX - rect.left - ox) / size) * 100, gy: ((e.clientY - rect.top - oy) / size) * 100 };
	}

	const host = canvas.parentElement || canvas;
	function move(e: PointerEvent) {
		const p = toGrid(e);
		if (!pointer.inside) {
			pointer.px = p.cx;
			pointer.py = p.cy;
		}
		pointer.x = p.cx;
		pointer.y = p.cy;
		pointer.inside = true;
		opts.onPoint?.(p.gx, p.gy, partAt(p.gx, p.gy));
		start();
	}
	function leave() {
		pointer.inside = false;
		pointer.x = pointer.y = -1e4;
		opts.onPoint?.(NaN, NaN, null);
	}
	function down(e: PointerEvent) {
		if (opts.reduced) return;
		const p = toGrid(e);
		shock = { x: p.cx, y: p.cy, t: performance.now() };
		start();
	}
	host.addEventListener('pointermove', move, { passive: true });
	host.addEventListener('pointerleave', leave);
	host.addEventListener('pointerdown', down, { passive: true });

	const ro = new ResizeObserver(() => layout());
	ro.observe(canvas);
	const io = new IntersectionObserver(([entry]) => {
		visible = entry.isIntersecting;
		if (visible) start();
	});
	io.observe(canvas);
	const mo = new MutationObserver(() => {
		dark = document.documentElement.dataset.theme === 'dark';
		draw(performance.now());
	});
	mo.observe(document.documentElement, { attributes: true, attributeFilter: ['data-theme'] });
	const vis = () => {
		visible = !document.hidden;
		if (visible) start();
	};
	document.addEventListener('visibilitychange', vis);

	layout();
	start();

	return () => {
		running = false;
		cancelAnimationFrame(raf);
		ro.disconnect();
		io.disconnect();
		mo.disconnect();
		document.removeEventListener('visibilitychange', vis);
		host.removeEventListener('pointermove', move);
		host.removeEventListener('pointerleave', leave);
		host.removeEventListener('pointerdown', down);
	};
}
