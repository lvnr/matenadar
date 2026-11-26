/**
 * A very small highlighter for the landing page's specimens, run at build
 * time, so the page ships no highlighting code. Classes (styled in
 * landing.css) follow the docs' manuscript pigments: k key, s string,
 * n number, c constant, kw keyword, f function or command, o option,
 * p punctuation, cm comment, pr prompt, out output.
 */

export type Lang = 'json' | 'sh' | 'js';
export interface Line {
	/** The text of the line. */
	t: string;
	/** A node of the specimen drawing this line describes. */
	ref?: string;
}

const esc = (s: string) => s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
const span = (cls: string, s: string) => `<span class="${cls}">${esc(s)}</span>`;

const JSON_RE = /("(?:[^"\\]|\\.)*")(\s*:)?|(-?\d+(?:\.\d+)?)|\b(true|false|null)\b|([{}[\],:])|(…)/g;

function json(text: string): string {
	let out = '';
	let last = 0;
	for (const m of text.matchAll(JSON_RE)) {
		out += esc(text.slice(last, m.index));
		const [, str, colon, num, con, punct, ell] = m;
		if (str !== undefined) out += colon ? span('k', str) + span('p', colon) : span('s', str);
		else if (num !== undefined) out += span('n', num);
		else if (con !== undefined) out += span('c', con);
		else if (punct !== undefined) out += span('p', punct);
		else if (ell !== undefined) out += span('cm', ell);
		last = m.index! + m[0].length;
	}
	return out + esc(text.slice(last));
}

const JS_KEYWORDS = 'import|from|export|const|let|await|async|for|of|if|return|new|function|true|false|null';
const JS_RE = new RegExp(
	[
		String.raw`(\/\/.*$)`,
		String.raw`('(?:[^'\\]|\\.)*'|"(?:[^"\\]|\\.)*"|\x60(?:[^\x60\\]|\\.)*\x60)`,
		String.raw`\b(${JS_KEYWORDS})\b`,
		String.raw`\b(\d+(?:\.\d+)?)\b`,
		String.raw`([A-Za-z_$][\w$]*)(?=\()`,
		String.raw`([A-Za-z_$][\w$]*)(?=\s*:)`,
		String.raw`([{}()[\],.;:=>])`,
	].join('|'),
	'g',
);

function js(text: string): string {
	let out = '';
	let last = 0;
	for (const m of text.matchAll(JS_RE)) {
		out += esc(text.slice(last, m.index));
		const [, cm, str, kw, num, fn, key, punct] = m;
		if (cm !== undefined) out += span('cm', cm);
		else if (str !== undefined) out += span('s', str);
		else if (kw !== undefined) out += span('kw', kw);
		else if (num !== undefined) out += span('n', num);
		else if (fn !== undefined) out += span('f', fn);
		else if (key !== undefined) out += span('k', key);
		else if (punct !== undefined) out += span('p', punct);
		last = m.index! + m[0].length;
	}
	return out + esc(text.slice(last));
}

const SH_RE = /(#.*$)|('(?:[^'\\]|\\.)*'|"(?:[^"\\]|\\.)*")|(\s--?[a-z][\w-]*)|(\|)/g;

function shCommand(text: string): string {
	// The command's first word is a function; the rest is arguments.
	const first = /^(\S+)(.*)$/.exec(text);
	if (!first) return esc(text);
	const [, word, rest] = first;
	let out = span('f', word);
	let last = 0;
	for (const m of rest.matchAll(SH_RE)) {
		out += esc(rest.slice(last, m.index));
		const [, cm, str, opt, pipe] = m;
		if (cm !== undefined) out += span('cm', cm);
		else if (str !== undefined) out += span('s', str);
		else if (opt !== undefined) out += span('o', opt);
		else if (pipe !== undefined) out += span('p', pipe);
		last = m.index! + m[0].length;
	}
	return out + esc(rest.slice(last));
}

function sh(text: string): string {
	if (text.startsWith('$ ')) return span('pr', '$') + ' ' + shCommand(text.slice(2));
	if (text.startsWith('# ')) return span('cm', text);
	if (/^\s*[[{"]/.test(text) || /^\s*[\]}]/.test(text)) return `<span class="out">${json(text)}</span>`;
	if (text.startsWith('→ ') || text.startsWith('← ')) {
		return span(text.startsWith('→') ? 'pr' : 'f', text[0]) + ' ' + `<span class="out">${json(text.slice(2))}</span>`;
	}
	return span('out', text);
}

export function highlight(line: string, lang: Lang): string {
	if (lang === 'json') return json(line);
	if (lang === 'js') return js(line);
	return sh(line);
}

/** Lines as HTML, each a span the page can light up with its specimen node. */
export function render(lines: Line[], lang: Lang): string {
	return lines
		.map(({ t, ref }) => {
			const body = highlight(t, lang) || ' ';
			return ref ? `<span class="ln" data-ref="${ref}">${body}</span>` : `<span class="ln">${body}</span>`;
		})
		.join('');
}
