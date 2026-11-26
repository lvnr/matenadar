/**
 * Compact, readable JSON, as `scripts/jsonfmt.py` writes it: a value stays
 * on one line when it fits, and short members share a line.
 */

function one(v: unknown): string {
	if (Array.isArray(v)) return `[${v.map(one).join(', ')}]`;
	if (v && typeof v === 'object') return `{${Object.entries(v).map(([k, x]) => `${JSON.stringify(k)}: ${one(x)}`).join(', ')}}`;
	return JSON.stringify(v);
}

export function format(v: unknown, width = 68, indent = 0, lead = 0): string {
	const flat = one(v);
	if (lead + flat.length <= width || typeof v !== 'object' || v === null) return flat;
	const pad = ' '.repeat(indent + 2);
	if (Array.isArray(v)) {
		if (!v.length) return '[]';
		if (v.every((x) => typeof x !== 'object' || x === null)) {
			const rows: string[] = [];
			let cur: string[] = [];
			for (const x of v) {
				const s = one(x);
				if (cur.length && pad.length + [...cur, s].join(', ').length + 1 > width) {
					rows.push(cur.join(', '));
					cur = [];
				}
				cur.push(s);
			}
			rows.push(cur.join(', '));
			return `[\n${rows.map((r) => pad + r).join(',\n')}\n${' '.repeat(indent)}]`;
		}
		return `[\n${v.map((x) => pad + format(x, width, indent + 2, indent + 2)).join(',\n')}\n${' '.repeat(indent)}]`;
	}
	const entries = Object.entries(v as Record<string, unknown>);
	if (!entries.length) return '{}';
	const rows: string[] = [];
	let cur: string[] = [];
	for (const [k, x] of entries) {
		const key = `${JSON.stringify(k)}: `;
		const s = key + format(x, width, indent + 2, indent + 2 + key.length);
		if (s.includes('\n')) {
			if (cur.length) rows.push(cur.join(', '));
			cur = [];
			rows.push(s);
			continue;
		}
		if (cur.length && pad.length + [...cur, s].join(', ').length + 1 > width) {
			rows.push(cur.join(', '));
			cur = [];
		}
		cur.push(s);
	}
	if (cur.length) rows.push(cur.join(', '));
	return `{\n${rows.map((r) => pad + r).join(',\n')}\n${' '.repeat(indent)}}`;
}
