import { test } from 'node:test';
import assert from 'node:assert/strict';
import { tags } from '@lezer/highlight';
import { markdownHighlight } from '../src/lib/highlighting.ts';
import { defaultHighlightStyle } from '@codemirror/language';

// The bug: the stock style claimed the markup tags and painted them #404740, which
// is 1.74:1 on the dark background — the `#`, `-`, `*`, backtick and brackets were
// barely visible while typing. A rule matching a tag is what paints it; no rule
// means the character inherits the theme's text colour.
const MARK_TAGS: [string, unknown][] = [
	['meta', tags.meta],
	['processingInstruction', tags.processingInstruction],
	['punctuation', tags.punctuation],
	['contentSeparator', tags.contentSeparator],
];

// What the stock style actually claims. `punctuation` it leaves alone, so it is in
// the list above (ours must not claim it either) but not in this one.
const STOCK_PAINTED: [string, unknown][] = [
	['meta', tags.meta],
	['processingInstruction', tags.processingInstruction],
	['contentSeparator', tags.contentSeparator],
];

test('the stock style is what paints markdown marks (guards the regression)', () => {
	// If this ever stops being true upstream, the reason for this test has changed.
	// The `^` of a superscript and the `~` of a strikethrough are marks too, and they
	// are what was reported: both came out #404740 on the dark background.
	for (const [name, tag] of STOCK_PAINTED) {
		assert.notEqual(
			defaultHighlightStyle.style([tag as never]),
			null,
			`defaultHighlightStyle no longer colours ${name}; re-check the fix`,
		);
	}
});

test('no markdown mark tag is claimed by a highlight rule', () => {
	for (const [name, tag] of MARK_TAGS) {
		assert.equal(
			markdownHighlight.style([tag as never]),
			null,
			`${name} is styled, so those characters do not inherit the text colour`,
		);
	}
});

test('structure survives: emphasis, strong and links are still marked up', () => {
	assert.notEqual(markdownHighlight.style([tags.emphasis]), null);
	assert.notEqual(markdownHighlight.style([tags.strong]), null);
	assert.notEqual(markdownHighlight.style([tags.link]), null);
	assert.notEqual(markdownHighlight.style([tags.heading]), null);
});
