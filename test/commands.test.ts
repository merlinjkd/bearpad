import { test } from 'node:test';
import assert from 'node:assert/strict';
import { transformCase } from '../src/lib/commands.ts';

test('lowercase', () => {
	assert.equal(transformCase('HeLLo WORLD', 'lowercase'), 'hello world');
});

test('uppercase', () => {
	assert.equal(transformCase('HeLLo WORLD', 'uppercase'), 'HELLO WORLD');
});

test('propercase capitalizes each word', () => {
	assert.equal(transformCase('hello world', 'propercase'), 'Hello World');
});

// Deliberate rule (v0.1.20, "Title Case respects apostrophes"): the letter
// after an apostrophe stays lowercase — "ISN'T" -> "Isn't". Known ceiling:
// proper names like "O'Neil" also lose the capital ("O'neil"); fixing that
// needs a dictionary, not a regex.
test('propercase leaves the letter after an apostrophe lowercase', () => {
	assert.equal(transformCase("O'NEIL ISN'T HERE", 'propercase'), "O'neil Isn't Here");
	assert.equal(transformCase("o'neil", 'propercase'), "O'neil");
});
