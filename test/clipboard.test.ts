import { test } from 'node:test';
import assert from 'node:assert/strict';
import { htmlToText, toPlainText, normalizePasted, decodeEntities } from '../src/lib/clipboard.ts';

// A browser copy carries an HTML flavour immediately and a plain-text flavour
// that can be delayed-rendered — so the HTML path is what saves a paste. These
// cases pin the shapes that actually arrive from Chromium/Edge/Firefox/Notepad.

test('plain text flavour wins when present', () => {
	assert.equal(toPlainText({ text: 'hello\nworld', html: '<b>ignored</b>' }), 'hello\nworld');
});

test('falls back to HTML when the plain-text flavour is missing', () => {
	// This is the browser case: HTML present, text flavour empty.
	assert.equal(toPlainText({ text: '', html: '<b>Hello</b> world' }), 'Hello world');
	assert.equal(toPlainText({ html: '<b>Hello</b> world' }), 'Hello world');
});

test('chromium document wrapper keeps only the marked fragment', () => {
	const html =
		"<meta charset='utf-8'><html><body><!--StartFragment--><b>New York</b> is big<!--EndFragment--></body></html>";
	assert.equal(htmlToText(html), 'New York is big');
});

test('block boundaries become newlines so lines do not run together', () => {
	assert.equal(htmlToText('<p>one</p><p>two</p>'), 'one\ntwo');
	assert.equal(htmlToText('a<br>b'), 'a\nb');
	assert.equal(htmlToText('<ul><li>first</li><li>second</li></ul>'), 'first\nsecond');
});

test('a paragraph after inline text still gets its own line', () => {
	// Caught in the running app: only closers emitting a break made this come out
	// as "New York is bigSecond para".
	assert.equal(htmlToText('<b>New York</b> is big<p>Second para</p>'), 'New York is big\nSecond para');
	assert.equal(htmlToText('text<div>block</div>more'), 'text\nblock\nmore');
});

test('table rows come out one per line, cells tab-separated', () => {
	const html = '<table><tr><td>Name</td><td>Qty</td></tr><tr><td>Bolt</td><td>4</td></tr></table>';
	assert.equal(htmlToText(html), 'Name	Qty\nBolt	4');
});

test('entities are decoded, including numeric and hex', () => {
	assert.equal(htmlToText('a &amp; b &lt;tag&gt;'), 'a & b <tag>');
	assert.equal(htmlToText('caf&#233; &#x2014; done'), 'café \u2014 done');
	assert.equal(htmlToText('50&nbsp;kg'), '50\u00a0kg');
});

test('escaped markup stays literal text', () => {
	// &lt;b&gt; is TEXT the user copied, not a tag — it must survive as-is.
	assert.equal(htmlToText('&lt;b&gt;not bold&lt;/b&gt;'), '<b>not bold</b>');
});

test('script and style content is dropped', () => {
	assert.equal(htmlToText('keep<script>evil()</script>me'), 'keepme');
	assert.equal(htmlToText('keep<style>.x{color:red}</style>me'), 'keepme');
});

test('windows line endings normalise and padding is trimmed', () => {
	assert.equal(normalizePasted('one\r\ntwo\r\n'), 'one\ntwo');
	assert.equal(normalizePasted('\n\n\none\n\n\n\ntwo\n\n\n'), 'one\n\ntwo');
	assert.equal(normalizePasted('   spaced   \n'), 'spaced');
});

test('empty clipboard yields empty text, never junk', () => {
	assert.equal(htmlToText(''), '');
	assert.equal(toPlainText({}), '');
	assert.equal(toPlainText({ text: '', html: '' }), '');
	assert.equal(decodeEntities('&notarealentity;'), '&notarealentity;');
});
