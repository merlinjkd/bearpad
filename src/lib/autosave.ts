// Autosave rules, extracted from App.svelte so they can be tested. The statements
// below were previously inline inside a `setInterval`, where `npm test` could not
// reach them at all (audit finding 9). Behaviour is unchanged: the same tabs are
// selected by the same rules and the same write promise is fired without awaiting.

export interface AutosaveTarget {
	isDirty: () => boolean;
	getContent: () => string;
	markSaved: () => void;
}

export interface AutosaveTab {
	path: string | null;
	lastChange?: number;
	ref: AutosaveTarget | null;
}

/** A file with a path is written once it has been unchanged for this long. */
export const AUTOSAVE_IDLE_MS = 120_000;

/**
 * Write every dirty, idle, path-backed tab. Untitled tabs are the recovery
 * store's job, not this one's.
 *
 * Returns a promise for the writes so a caller (or a test) can wait; the interval
 * in App.svelte deliberately does not.
 */
export function autosaveDirtyFiles(
	tabs: AutosaveTab[],
	write: (path: string, content: string) => Promise<unknown>,
	now: number,
	onSaved?: () => void,
	idleMs: number = AUTOSAVE_IDLE_MS,
): Promise<unknown> {
	const writes: Promise<unknown>[] = [];
	for (const tab of tabs) {
		if (!tab.ref?.isDirty()) continue;
		if (!tab.lastChange || now - tab.lastChange < idleMs) continue;
		const content = tab.ref.getContent();
		if (!tab.path) continue;
		writes.push(
			write(tab.path, content)
				.then(() => {
					// The write carried a snapshot of the document. If it moved on while
					// the disk was busy, that snapshot is stale: leave the tab dirty so
					// the next pass writes the newer text. Clearing the flag here would
					// hide the loss — the tab would show itself saved over older bytes.
					if (tab.ref?.getContent() !== content) return;
					tab.ref?.markSaved();
					onSaved?.();
				})
				.catch(() => {}),
		);
	}
	return Promise.all(writes);
}
