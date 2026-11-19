import { test } from 'node:test';
import assert from 'node:assert/strict';
import { autosaveDirtyFiles } from '../src/lib/autosave.ts';

// A stand-in for EditorExposed with the same meaning it has in Editor.svelte:
// `content` changes as the user types, `markSaved()` records that the CURRENT
// content is what reached the disk.
function fakeEditor(initial: string) {
	let saved = initial;
	let content = initial;
	let markSavedCalls = 0;
	return {
		ref: {
			isDirty: () => content !== saved,
			getContent: () => content,
			markSaved: () => {
				markSavedCalls += 1;
				saved = content;
			},
		},
		type: (text: string) => {
			content += text;
		},
		markSavedCalls: () => markSavedCalls,
	};
}

// Control: nothing changes during the write, so the tab IS saved. This stays green
// before and after the fix — it is what makes the test below specific to the bug
// rather than to autosave in general.
test('autosave marks a tab saved when nothing changed during the write', async () => {
	const ed = fakeEditor('');
	ed.type('hello');
	const now = 200_000;
	const tab = { path: '/tmp/note.txt', lastChange: now - 130_000, ref: ed.ref };
	const written: string[] = [];

	await autosaveDirtyFiles([tab], async (_path, content) => {
		written.push(content);
	}, now);

	assert.deepEqual(written, ['hello']);
	assert.equal(ed.ref.isDirty(), false);
	assert.equal(ed.markSavedCalls(), 1);
});

// The bug: the write is in flight, the user types, and markSaved() then claims the
// tab is on disk. It is not.
test('autosave does not mark a tab saved when the user typed during the write', async () => {
	const ed = fakeEditor('');
	ed.type('hello');
	const now = 200_000;
	const tab = { path: '/tmp/note.txt', lastChange: now - 130_000, ref: ed.ref };
	const written: string[] = [];
	const gate = Promise.withResolvers<void>();
	const write = (_path: string, content: string) => {
		written.push(content);
		return gate.promise;
	};

	const done = autosaveDirtyFiles([tab], write, now);
	ed.type(' world'); // the user keeps typing while the disk write is in flight
	gate.resolve();
	await done;

	assert.deepEqual(written, ['hello'], 'the disk received the stale text');
	assert.equal(
		ed.ref.isDirty(),
		true,
		'the tab must stay dirty: " world" never reached the disk',
	);
	assert.equal(ed.markSavedCalls(), 0, 'a stale write must not clear the dirty mark');
});
