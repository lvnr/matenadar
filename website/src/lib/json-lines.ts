/**
 * JSON for the example panels, highlighted at build time: one entry per
 * line, each line carrying the `id` of the element it belongs to, so a line
 * and the box it made on the page can point at each other. The text is the
 * compact form `scripts/jsonfmt.py` writes: an object either fits on one
 * line or opens with `{` and closes with `}` at the same indentation.
 */

const esc = (s: string) => s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
const span = (cls: string, s: string) => `<span class="${cls}">${esc(s)}</span>`;
const RE = /("(?:[^"\\]|\\.)*")(\s*:)?|(-?\d+(?:\.\d+)?(?:e[+-]?\d+)?)|\b(true|false|null)\b|([{}[\],:])/g;

function highlight(text: string): string {
	let out = '';
	let last = 0;
	for (const m of text.matchAll(RE)) {
		out += esc(text.slice(last, m.index));
		const [, str, colon, num, con, punct] = m;
		if (str !== undefined) out += colon ? span('k', str) + span('p', colon) : span('s', str);
		else if (num !== undefined) out += span('n', num);
		else if (con !== undefined) out += span('c', con);
		else if (punct !== undefined) out += span('p', punct);
		last = m.index! + m[0].length;
	}
	return out + esc(text.slice(last));
}

export interface JsonLine {
	html: string;
	id?: string;
}

const indentOf = (s: string) => s.length - s.trimStart().length;
const idIn = (s: string) => /"id": "([^"]+)"/.exec(s)?.[1];

/** `ids`: the ids worth pointing at (elements the layout reports). */
export function jsonLines(text: string, ids?: Set<string>): JsonLine[] {
	const lines = text.replace(/\n$/, '').split('\n');
	const owner: (string | undefined)[] = new Array(lines.length).fill(undefined);
	const wanted = (id?: string) => !!id && (!ids || ids.has(id));
	// Outer objects first, so an inner object's id wins over its parent's.
	for (let i = 0; i < lines.length; i++) {
		if (!lines[i].trimEnd().endsWith('{')) continue;
		const ind = indentOf(lines[i]);
		let j = i + 1;
		while (j < lines.length && !(indentOf(lines[j]) === ind && lines[j].trimStart().startsWith('}'))) j++;
		let id: string | undefined;
		for (let k = i + 1; k < j; k++) {
			if (indentOf(lines[k]) === ind + 2 && wanted(idIn(lines[k]))) {
				id = idIn(lines[k]);
				break;
			}
		}
		if (id) for (let k = i; k <= j && k < lines.length; k++) owner[k] = id;
	}
	// Objects on one line.
	lines.forEach((l, i) => {
		const t = l.trim();
		const id = idIn(l);
		if (t.startsWith('{') && (t.endsWith('}') || t.endsWith('},')) && wanted(id)) owner[i] = id;
	});
	return lines.map((l, i) => ({ html: highlight(l), id: owner[i] }));
}
