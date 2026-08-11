import type { EditorExposed } from './Editor.svelte';

export type TransformType = 'lowercase' | 'uppercase' | 'propercase';

export function transformCase(text: string, type: TransformType): string {
	if (type === 'lowercase') return text.toLowerCase();
	if (type === 'uppercase') return text.toUpperCase();
	// Title Case: capitalize the first letter of each word, but not the letter
	// after an apostrophe (O'NEIL -> O'Neil, ISN'T -> Isn't).
	return text.toLowerCase().replace(/\b(?<!['\u2019])\w/g, (c) => c.toUpperCase());
}

export interface Command {
	label: string;
	shortcut?: () => string;
	run: () => void;
	disabled?: () => boolean;
}

export type CommandMap = Record<string, Command>;

type EditorLike = Pick<
	EditorExposed,
	| 'hasSelection'
	| 'handleCut'
	| 'handleCopy'
	| 'handlePaste'
	| 'undo'
	| 'redo'
	| 'handleSelectAll'
	| 'openFind'
	| 'transformSelection'
>;

// The one definition of the editor command set. The menu bar, the context
// menu, and (future) keybindings all consume it — adding a command here
// lights it up everywhere.
export function editorCommands(ed: () => EditorLike | null): CommandMap {
	const isMac = /mac/i.test(navigator.platform);
	const mod = isMac ? '⌘' : 'Ctrl+';
	const sel = () => ed();
	const hasSel = () => !!sel()?.hasSelection();
	return {
		undo: { label: 'Undo', shortcut: () => mod + 'Z', run: () => sel()?.undo() },
		redo: { label: 'Redo', shortcut: () => (isMac ? '⇧⌘' : 'Ctrl+Shift+') + 'Z', run: () => sel()?.redo() },
		cut: { label: 'Cut', shortcut: () => mod + 'X', run: () => sel()?.handleCut() },
		copy: { label: 'Copy', shortcut: () => mod + 'C', run: () => sel()?.handleCopy() },
		paste: { label: 'Paste', shortcut: () => mod + 'V', run: () => sel()?.handlePaste() },
		selectAll: { label: 'Select All', shortcut: () => mod + 'A', run: () => sel()?.handleSelectAll() },
		find: { label: 'Find & Replace...', shortcut: () => mod + 'F', run: () => sel()?.openFind() },
		lowercase: { label: 'lowercase', run: () => sel()?.transformSelection('lowercase'), disabled: () => !hasSel() },
		uppercase: { label: 'UPPERCASE', run: () => sel()?.transformSelection('uppercase'), disabled: () => !hasSel() },
		propercase: { label: 'Title Case', run: () => sel()?.transformSelection('propercase'), disabled: () => !hasSel() },
	};
}
