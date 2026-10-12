// Clipboard content arrives as plain text, HTML, or both. BearPad is a text
// editor, so EVERY paste is reduced to plain text here — the user asked for that
// explicitly, and it is also what makes a browser copy survive: browsers put an
// HTML flavour on the clipboard immediately, while the plain-text flavour can be
// delayed-rendered and come back empty when read from Rust.
//
// Deliberately DOM-free (regex + entity decoding) so it runs in the app AND in
// plain `node --test` with no jsdom dependency.

const ENTITIES: Record<string, string> = {
	amp: '&',
	lt: '<',
	gt: '>',
	quot: '"',
	apos: "'",
	nbsp: '\u00a0',
	hellip: '\u2026',
	mdash: '\u2014',
	ndash: '\u2013',
	rsquo: '\u2019',
	lsquo: '\u2018',
	rdquo: '\u201d',
	ldquo: '\u201c',
	bull: '\u2022',
	middot: '\u00b7',
	copy: '\u00a9',
	reg: '\u00ae',
	trade: '\u2122',
	deg: '\u00b0',
	times: '\u00d7',
	divide: '\u00f7',
};

export function decodeEntities(text: string): string {
	return text.replace(/&(#x?[0-9a-fA-F]+|[a-zA-Z]+);/g, (whole, body: string) => {
		if (body[0] === '#') {
			const hex = body[1] === 'x' || body[1] === 'X';
			const code = parseInt(hex ? body.slice(2) : body.slice(1), hex ? 16 : 10);
			return Number.isFinite(code) && code > 0 && code <= 0x10ffff ? String.fromCodePoint(code) : whole;
		}
		return ENTITIES[body.toLowerCase()] ?? whole;
	});
}

/**
 * Reduce clipboard HTML to plain text. Browsers hand over a whole document
 * wrapper, and this is a fragment, not a full document, so it is handled by
 * marker + block detection rather than a parser.
 */
export function htmlToText(html: string): string {
	if (!html) return '';
	let out = html;
	// Chromium marks the real payload; take only that when the markers are present.
	const start = out.indexOf('<!--StartFragment-->');
	const end = out.indexOf('<!--EndFragment-->');
	if (start !== -1 && end !== -1 && end > start) {
		out = out.slice(start + '<!--StartFragment-->'.length, end);
	}
	out = out
		.replace(/<!--[\s\S]*?-->/g, '')
		.replace(/<(script|style)\b[^>]*>[\s\S]*?<\/\1>/gi, '');
	// Block-level boundaries become newlines, otherwise a copied paragraph, list
	// or table row arrives as one run-on line. Cells become tabs, matching what
	// browsers put in the plain-text flavour of a copied table. Closers emit the
	// break; an OPENER only adds one when the text before it is not already at a
	// line start (lookbehind), which is what keeps "text<p>next</p>" from running
	// together without doubling every "</p><p>" pair.
	out = out
		.replace(/<br\s*\/?>/gi, '\n')
		.replace(/<\/t[dh]\s*>/gi, '	')
		.replace(/<\/(p|div|li|tr|h[1-6]|blockquote|pre|section|article|header|footer|table)\s*>/gi, '\n')
		.replace(/(?<!\n)<(p|div|h[1-6]|blockquote|pre|section|article|header|footer|li|tr|table)\b[^>]*>/gi, '\n$&');
	out = out.replace(/<[^>]+>/g, '');
	return decodeEntities(out)
		.replace(/[ 	]+(?=\n)/g, '')
		.trim();
}

/** Normalise newlines and trim the padding a copy tends to carry. */
export function normalizePasted(text: string): string {
	return text
		.replace(/\r\n?/g, '\n')
		.replace(/[ 	]+(?=\n)/g, '')
		.replace(/\n{3,}/g, '\n\n')
		.trim();
}

/**
 * Pick the flavour to insert. Plain text wins when present (it already carries
 * the source's line breaks); HTML is converted when that is all there is.
 */
export function toPlainText(flavours: { text?: string | null; html?: string | null }): string {
	const text = flavours.text ?? '';
	if (text.length > 0) return normalizePasted(text);
	return normalizePasted(htmlToText(flavours.html ?? ''));
}
