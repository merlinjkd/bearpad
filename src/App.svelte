<script lang="ts">
	import { onMount } from 'svelte';
	import Editor, { type EditorExposed } from './lib/Editor.svelte';
	import ContextMenu from './lib/ContextMenu.svelte';
	import SettingsModal from './lib/SettingsModal.svelte';
	import { invoke } from '@tauri-apps/api/core';
	import { open, save as showSaveDialog, confirm as showConfirm } from '@tauri-apps/plugin-dialog';
	import { getCurrentWindow } from '@tauri-apps/api/window';
	import bearpawIcon from './assets/bearpaw.png';

	const isMac = /mac/i.test(navigator.platform);
	import { editorCommands } from './lib/commands';

	type Theme = 'dark' | 'light' | 'system';

	const FILTERS = [
		{ name: 'Text', extensions: ['txt'] },
		{ name: 'Markdown', extensions: ['md', 'markdown'] },
	];

	interface TabState {
		id: number;
		path: string | null;
		doc: string;
		ref: EditorExposed | null;
		name?: string;
		lastChange?: number;
	}

	let tabs = $state<TabState[]>([{ id: 0, path: null, doc: '', ref: null, name: 'Untitled 1' }]);
	let activeTabId = $state(0);
	let tabSeq = 1;
	let untitledSeq = 2;
	let dirtyMap = $state<Record<number, boolean>>({});
	let showSettings = $state(false);
	let showAbout = $state(false);
	let appVersion = $state('');
	let theme = $state<Theme>('dark');
	let fontSize = $state(18);
	let fontFamily = $state("'SF Mono', 'Fira Code', 'Cascadia Code', monospace");
	let uiFontSize = $state(16);
	let wordWrap = $state(true);
	let spellcheck = $state(true);
	let spellLang = $state('en_US');
	let cursorBlink = $state(false);
	let textColor = $state('');
	let resolvedTheme = $state<'dark' | 'light'>('dark');
	let editorTheme = $state<'dark' | 'light'>('dark');

	let ctxMenu = $state<{ show: boolean; x: number; y: number; items: any[] }>({
		show: false,
		x: 0,
		y: 0,
		items: [],
	});

	// ─── menu bar (in-window HTML — native OS menus can't be resized) ───

	let openMenu = $state<number | null>(null);

	const cmds = editorCommands(() => activeTab()?.ref ?? null);

	function menuItem(id: keyof typeof cmds) {
		const c = cmds[id];
		return { label: c.label, action: c.run, disabled: c.disabled };
	}

	const menus: {
		label: string;
		items: {
			label?: string;
			separator?: boolean;
			action?: () => void;
			disabled?: boolean | (() => boolean);
		}[];
	}[] = [
		{
			label: 'File',
			items: [
				{ label: 'New', action: () => newFile() },
				{ label: 'Open...', action: () => openFile() },
				{ separator: true },
				{ label: 'Save', action: () => saveFile() },
				{ label: 'Save As...', action: () => saveFileAs() },
				{ separator: true },
				{ label: 'About BearPad...', action: () => openAbout() },
				{ separator: true },
				{ label: 'Exit', action: () => getCurrentWindow().close() },
			],
		},
		{
			label: 'Edit',
			items: [
				menuItem('undo'),
				menuItem('redo'),
				{ separator: true },
				menuItem('cut'),
				menuItem('copy'),
				menuItem('paste'),
				{ separator: true },
				menuItem('find'),
			],
		},
		{
			label: 'View',
			items: [
				{
					label: 'Zoom In',
					action: () => handleSettingsChange({ fontSize: Math.min(32, fontSize + 1) }),
				},
				{
					label: 'Zoom Out',
					action: () => handleSettingsChange({ fontSize: Math.max(10, fontSize - 1) }),
				},
				{ label: 'Reset Zoom', action: () => handleSettingsChange({ fontSize: 18 }) },
				{ separator: true },
				{
					label: 'Toggle Theme',
					action: () =>
						handleSettingsChange({
							theme: resolvedTheme === 'dark' ? 'light' : 'dark',
						}),
				},
				{
					label: 'Toggle Word Wrap',
					action: () => handleSettingsChange({ wordWrap: !wordWrap }),
				},
				{
					label: 'Toggle Spell Check',
					action: () => handleSettingsChange({ spellcheck: !spellcheck }),
				},
			],
		},
		{
			label: 'Text',
			items: [menuItem('lowercase'), menuItem('uppercase'), menuItem('propercase')],
		},
	];

	// ─── helpers ────────────────────────────────────────

	function fileName(tab: TabState | null) {
		if (!tab?.path) return tab?.name ?? 'Untitled';
		return tab.path.split('/').pop() || tab.path.split('\\').pop() || 'Untitled';
	}

	function activeTab() {
		return tabs.find((t) => t.id === activeTabId) ?? null;
	}

	function updateTitle() {
		try {
			const tab = activeTab();
			const win = getCurrentWindow();
			win.setTitle(`BearPad — ${fileName(tab)}${tab?.ref?.isDirty() ? ' ●' : ''}`);
		} catch {
			/* title is cosmetic; never let it break the mount chain */
		}
	}

	function syncRustDirty() {
		invoke('set_dirty', { dirty: tabs.some((t) => t.ref?.isDirty() ?? false) }).catch(() => {});
	}

	function handleDirtyChange(tabId: number, dirty: boolean) {
		dirtyMap[tabId] = dirty;
		const tab = tabs.find((t) => t.id === tabId);
		if (tab && dirty) tab.lastChange = Date.now();
		syncRustDirty();
		updateTitle();
	}

	// ─── autosave + crash recovery ──────────────────────

	// Persist untitled tabs (content or dirty) so a crash can restore them.
	async function updateRecovery() {
		const untitled = tabs.filter(
			(t) => !t.path && (dirtyMap[t.id] || (t.ref?.getContent() ?? '').length > 0)
		);
		const payload = JSON.stringify({
			untitledSeq,
			tabs: untitled.map((t) => ({ name: t.name, content: t.ref?.getContent() ?? '' })),
		});
		try {
			await invoke('write_recovery', { json: payload });
		} catch {
			/* not in Tauri (dev browser) */
		}
	}

	async function restoreRecovery() {
		try {
			const data = await invoke<string>('read_recovery');
			if (!data) return;
			const rec = JSON.parse(data);
			if (Array.isArray(rec.tabs) && rec.tabs.length) {
				for (const r of rec.tabs) {
					const tab: TabState = {
						id: tabSeq++,
						path: null,
						doc: r.content ?? '',
						ref: null,
						name: r.name ?? `Untitled ${untitledSeq}`,
					};
					tabs.push(tab);
				}
				activeTabId = tabs[0]?.id ?? null;
				untitledSeq = Math.max(untitledSeq, (rec.untitledSeq ?? 1) + tabs.length);
				updateTitle();
			}
		} catch {
			/* corrupt recovery: ignore */
		}
	}

	// Autosave: files with a path are written after 120s of idle changes;
	// untitled tabs go to the recovery store on the same cadence.
	setInterval(() => {
		const now = Date.now();
		for (const tab of tabs) {
			if (!tab.ref?.isDirty()) continue;
			if (!tab.lastChange || now - tab.lastChange < 120_000) continue;
			const content = tab.ref.getContent();
			if (tab.path) {
				invoke('write_file', { path: tab.path, content })
					.then(() => {
						tab.ref?.markSaved();
						updateTitle();
					})
					.catch(() => {});
			}
		}
		updateRecovery();
	}, 30_000);

	// ─── file operations ─────────────────────────────────

	async function newFile() {
		const tab: TabState = { id: tabSeq++, path: null, doc: '', ref: null, name: `Untitled ${untitledSeq++}` };
		tabs.push(tab);
		activeTabId = tab.id;
		updateTitle();
	}

	async function openFile() {
		const selected = await open({ filters: FILTERS, multiple: false });
		if (!selected) return;
		const path = selected as string;
		const existing = tabs.find((t) => t.path === path);
		if (existing) {
			activeTabId = existing.id;
			updateTitle();
			return;
		}
		try {
			const content = await invoke<string>('read_file', { path });
			const tab: TabState = { id: tabSeq++, path, doc: content, ref: null };
			tabs.push(tab);
			activeTabId = tab.id;
			updateTitle();
		} catch (e) {
			console.error('Failed to open file:', e);
		}
	}

	async function saveFile() {
		const tab = activeTab();
		if (!tab) return;
		if (tab.path) {
			const content = tab.ref?.getContent() ?? '';
			try {
				await invoke('write_file', { path: tab.path, content });
				tab.ref?.markSaved();
				updateTitle();
			} catch (e) {
				console.error('Failed to save file:', e);
			}
		} else {
			await saveFileAs();
		}
	}

	async function saveFileAs() {
		const tab = activeTab();
		if (!tab) return;
		const selected = await showSaveDialog({ filters: FILTERS, defaultPath: 'untitled.txt' });
		if (!selected) return;
		const path = selected as string;
		const content = tab.ref?.getContent() ?? '';
		try {
			await invoke('write_file', { path, content });
			tab.path = path;
			tab.ref?.markSaved();
			updateTitle();
			updateRecovery();
		} catch (e) {
			console.error('Failed to save file:', e);
		}
	}

	async function closeTab(id: number) {
		const idx = tabs.findIndex((t) => t.id === id);
		if (idx === -1) return;
		const tab = tabs[idx];
		if (tab.ref?.isDirty()) {
			const ok = await showConfirm('Discard unsaved changes?', {
				title: 'BearPad',
				kind: 'warning',
			});
			if (!ok) return;
		}
		tabs.splice(idx, 1);
		delete dirtyMap[id];
		updateRecovery();
		if (activeTabId === id) {
			activeTabId = tabs[Math.min(idx, tabs.length - 1)]?.id ?? null;
		}
		if (tabs.length === 0) {
			newFile();
		} else {
			syncRustDirty();
			updateTitle();
		}
	}

	// ─── settings ────────────────────────────────────────

	async function loadSettings() {
		try {
			const data = await invoke<string>('read_settings');
			const s = JSON.parse(data);
			if (s.theme) theme = s.theme;
			if (s.fontSize != null) fontSize = s.fontSize;
			if (s.uiFontSize != null) uiFontSize = s.uiFontSize;
			if (s.fontFamily) fontFamily = s.fontFamily;
			if (s.wordWrap != null) wordWrap = s.wordWrap;
			if (s.spellcheck != null) spellcheck = s.spellcheck;
			if (s.spellLang) spellLang = s.spellLang;
			if (s.cursorBlink != null) cursorBlink = s.cursorBlink;
			if (s.textColor) textColor = s.textColor;
		} catch { /* defaults */ }
		resolveTheme();
	}

	async function saveSettings() {
		try {
			await invoke('write_settings', {
				json: JSON.stringify({ theme, fontSize, uiFontSize, fontFamily, wordWrap, spellcheck, spellLang, cursorBlink, textColor }),
			});
		} catch (e) {
			console.error('Failed to save settings:', e);
		}
	}

	function resolveTheme() {
		if (theme === 'light') {
			resolvedTheme = 'light';
			editorTheme = 'light';
		} else if (theme === 'system') {
			resolvedTheme = window.matchMedia('(prefers-color-scheme: dark)').matches
				? 'dark'
				: 'light';
			editorTheme = resolvedTheme;
		} else {
			resolvedTheme = 'dark';
			editorTheme = 'dark';
		}
		// body needs the attr too — theme CSS vars are keyed on body[data-theme]
		document.body.dataset.theme = resolvedTheme;
	}

	function handleSettingsChange(
		patch: Partial<{ theme: Theme; fontSize: number; uiFontSize: number; fontFamily: string; wordWrap: boolean; spellcheck: boolean }>,
	) {
		if (patch.theme !== undefined) theme = patch.theme;
		if (patch.fontSize !== undefined) fontSize = patch.fontSize;
		if (patch.uiFontSize !== undefined) uiFontSize = patch.uiFontSize;
		if (patch.fontFamily !== undefined) fontFamily = patch.fontFamily;
		if (patch.wordWrap !== undefined) wordWrap = patch.wordWrap;
		if (patch.spellcheck !== undefined) spellcheck = patch.spellcheck;
		resolveTheme();
		saveSettings();
	}

	function openSettings() {
		showSettings = true;
	}

	async function openAbout() {
		if (!appVersion) {
			try {
				appVersion = await invoke<string>('app_version');
			} catch {
				appVersion = 'unknown';
			}
		}
		showAbout = true;
	}

	function closeAbout() {
		showAbout = false;
	}

	function closeSettings() {
		showSettings = false;
	}

	// ─── menu bar ────────────────────────────────────────

	onMount(async () => {
		await loadSettings();


		// Close confirmation lives in Rust (on_window_event + native dialog +
		// destroy) — the JS dialog path hangs on Windows in every variant.

		// System theme listener
		window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', () => {
			if (theme === 'system') resolveTheme();
		});

		updateTitle();
	});

	// ─── context menu handler ───────────────────────────

	async function onContextMenu(e: MouseEvent) {
		e.preventDefault();
		const isInsideEditor = !!(e.target as HTMLElement).closest('.cm-editor');
		const hasSelection = activeTab()?.ref?.hasSelection() || false;

		const items: any[] = [];

		if (isInsideEditor) {
			if (hasSelection) {
				items.push(
					{ label: cmds.cut.label, shortcut: cmds.cut.shortcut?.(), onClick: () => cmds.cut.run() },
					{ label: cmds.copy.label, shortcut: cmds.copy.shortcut?.(), onClick: () => cmds.copy.run() },
				);
			}
			items.push(
				{ label: cmds.paste.label, shortcut: cmds.paste.shortcut?.(), onClick: () => cmds.paste.run() },
				{ separator: true },
				{ label: cmds.undo.label, shortcut: cmds.undo.shortcut?.(), onClick: () => cmds.undo.run() },
				{ label: cmds.redo.label, shortcut: cmds.redo.shortcut?.(), onClick: () => cmds.redo.run() },
				{ separator: true },
				{
					label: cmds.lowercase.label,
					disabled: !hasSelection,
					onClick: () => {
						cmds.lowercase.run();
						hideMenu();
					},
				},
				{
					label: cmds.uppercase.label,
					disabled: !hasSelection,
					onClick: () => {
						cmds.uppercase.run();
						hideMenu();
					},
				},
				{
					label: cmds.propercase.label,
					disabled: !hasSelection,
					onClick: () => {
						cmds.propercase.run();
						hideMenu();
					},
				},
				{ separator: true },
			);

			// right-clicking a misspelled word offers corrections + learn
			const errWord = (e.target as HTMLElement)
				.closest('.cm-spell-error')
				?.textContent?.trim();
			if (errWord) {
				const miss = activeTab()?.ref?.getMisspellingAt(e.clientX, e.clientY);
				items.push({ separator: true });
				if (miss) {
					try {
						const suggestions = await invoke<string[]>(
							'suggest_spellings',
							{ word: miss.text, lang: spellLang }
						);
						if (suggestions.length) {
							items.push({ label: 'Suggestions', disabled: true });
							for (const s of suggestions.slice(0, 8)) {
								items.push({
									label: s,
									onClick: () => {
										activeTab()?.ref?.replaceRange(miss.from, miss.to, s);
										hideMenu();
									},
								});
							}
							items.push({ separator: true });
						}
					} catch {
						/* not in Tauri (dev browser) */
					}
				}
				items.push(
					{
						label: `Add "${errWord}" to dictionary`,
						onClick: async () => {
							try {
								await invoke('add_to_dictionary', { word: errWord });
								activeTab()?.ref?.recheckSpelling();
							} catch {
								/* ignore */
							}
							hideMenu();
						},
					},
				);
			}
		}

		items.push({
			label: cmds.selectAll.label,
			shortcut: cmds.selectAll.shortcut?.(),
			onClick: () => cmds.selectAll.run(),
		});

		ctxMenu = { show: true, x: e.clientX, y: e.clientY, items };
	}

	function hideMenu() {
		ctxMenu.show = false;
	}

	// ─── editor ready ───────────────────────────────────

	function onEditorReady() {
		updateTitle();
	}

	onMount(() => {
		restoreRecovery();
		// Context menu listener
		document.addEventListener('contextmenu', onContextMenu as unknown as EventListener);

		// View shortcuts handled in-page: Windows WebView2 swallows OS menu
		// accelerators when the webview has focus (and hijacks Ctrl+= as browser
		// zoom), so keydown is the one path that behaves identically everywhere.
		window.addEventListener('keydown', (e) => {
			const k = e.key;
			const mod = e.metaKey || e.ctrlKey;
			if (mod && !e.altKey) {
				if (k === '=' || k === '+') {
					e.preventDefault();
					handleSettingsChange({ fontSize: Math.min(32, fontSize + 1) });
				} else if (k === '-' || k === '_') {
					e.preventDefault();
					handleSettingsChange({ fontSize: Math.max(10, fontSize - 1) });
				} else if (k === '0') {
					e.preventDefault();
					handleSettingsChange({ fontSize: 18 });
				} else if (k === '\\') {
					e.preventDefault();
					handleSettingsChange({
						theme: resolvedTheme === 'dark' ? 'light' : 'dark',
					});
				} else if (k === 'n' || k === 'N') {
					e.preventDefault();
					newFile();
				} else if (k === 'o' || k === 'O') {
					e.preventDefault();
					openFile();
				} else if (k === 's' || k === 'S') {
					e.preventDefault();
					if (e.shiftKey) saveFileAs();
					else saveFile();
				} else if (k === 'w' || k === 'W' || k === 'q' || k === 'Q') {
					e.preventDefault();
					getCurrentWindow().close();
				} else if (k === ',') {
					e.preventDefault();
					openSettings();
				}
			} else if (e.altKey && !mod && (k === 'z' || k === 'Z')) {
				e.preventDefault();
				handleSettingsChange({ wordWrap: !wordWrap });
			}
		});
		document.addEventListener('click', (e) => {
			if (
				ctxMenu.show &&
				!(e.target as HTMLElement).closest('.custom-context-menu')
			) {
				hideMenu();
			}
			if (openMenu !== null && !(e.target as HTMLElement).closest('.menu-bar')) {
				openMenu = null;
			}
		});
	});
</script>

<div class="app-root" data-theme={resolvedTheme} style="--ui-font-size:{uiFontSize}px">
	<div class="title-bar" class:title-bar-mac={isMac} data-tauri-drag-region>
		<img class="title-bar-icon" src={bearpawIcon} alt="" draggable="false" />
		<span class="title-bar-title" data-tauri-drag-region>BearPad</span>
		{#if !isMac}
			<div class="title-bar-controls">
				<button class="tb-btn" aria-label="Minimize" onclick={() => getCurrentWindow().minimize()}>
					<span class="tb-glyph">─</span>
				</button>
				<button class="tb-btn" aria-label="Maximize" onclick={() => getCurrentWindow().toggleMaximize()}>
					<span class="tb-glyph">□</span>
				</button>
				<button class="tb-btn tb-close" aria-label="Close" onclick={() => getCurrentWindow().close()}>
					<span class="tb-glyph">✕</span>
				</button>
			</div>
		{/if}
	</div>
	<div class="menu-bar" role="menubar">
		{#each menus as menu, i (menu.label)}
			<div
				class="menu-item"
				class:open={openMenu === i}
				role="menuitem"
				tabindex="-1"
				onmouseenter={() => (openMenu !== null ? (openMenu = i) : null)}
				onclick={(e) => {
					e.stopPropagation();
					openMenu = openMenu === i ? null : i;
				}}
				onkeydown={(e) => {
					if (e.key === 'Enter' || e.key === ' ') {
						e.preventDefault();
						e.stopPropagation();
						openMenu = openMenu === i ? null : i;
					}
				}}
			>
				<span class="menu-label">{menu.label}</span>
				{#if openMenu === i}
					<div class="menu-dropdown" role="menu">
						{#each menu.items as item}
							{@const disabled = typeof item.disabled === 'function' ? item.disabled() : item.disabled}
							{#if item.separator}
								<div class="menu-sep"></div>
							{:else}
								<button
									class="menu-action"
									class:menu-disabled={disabled}
									role="menuitem"
									disabled={disabled}
									onclick={(e) => {
										e.stopPropagation();
										if (disabled) return;
										item.action?.();
										openMenu = null;
									}}
								>
									{item.label}
								</button>
							{/if}
						{/each}
					</div>
				{/if}
			</div>
		{/each}
		<div class="menu-spacer"></div>
		<button
			class="menu-gear"
			aria-label="Settings"
			title="Settings"
			onclick={() => openSettings()}
		>
			<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
				<circle cx="12" cy="12" r="3"></circle>
				<path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 1 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 1 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"></path>
			</svg>
		</button>
	</div>

	<div class="tab-bar" role="tablist">
		{#each tabs as tab (tab.id)}
			<div
				class="tab"
				class:active={tab.id === activeTabId}
				role="tab"
				tabindex="-1"
				aria-selected={tab.id === activeTabId}
				onclick={() => {
					activeTabId = tab.id;
					updateTitle();
				}}
				onkeydown={(e) => {
					if (e.key === 'Enter' || e.key === ' ') {
						e.preventDefault();
						activeTabId = tab.id;
						updateTitle();
					}
				}}
			>
				<span class="tab-title">{fileName(tab)}</span>
				{#if dirtyMap[tab.id]}
					<span class="tab-dirty" title="unsaved">●</span>
				{/if}
				<button
					class="tab-close"
					aria-label="Close tab"
					onclick={(e) => {
						e.stopPropagation();
						closeTab(tab.id);
					}}
					>×</button>
			</div>
		{/each}
		<button
			class="tab-new"
			aria-label="New tab"
			title="New tab"
			onclick={() => newFile()}
			>+</button>
	</div>

	<div class="editor-wrap">
		{#each tabs as tab (tab.id)}
			<div class="tab-pane" class:hidden={tab.id !== activeTabId}>
				<Editor
					doc={tab.doc}
					onReady={(ref) => {
						tab.ref = ref;
						onEditorReady();
					}}
					onDirtyChange={(dirty) => handleDirtyChange(tab.id, dirty)}
					theme={editorTheme}
					{fontSize}
					{fontFamily}
					{wordWrap}
					{spellcheck}
					{spellLang}
					{cursorBlink}
					{textColor}
				/>
			</div>
		{/each}
	</div>

	{#if ctxMenu.show}
		<ContextMenu contextMenu={ctxMenu} onhide={hideMenu} />
	{/if}

	{#if showSettings}
		<SettingsModal
			settings={{ theme, fontSize, uiFontSize, fontFamily, wordWrap, spellcheck, spellLang, cursorBlink }}
			onChange={handleSettingsChange}
			onClose={closeSettings}
		/>
	{/if}

	{#if showAbout}
		<div class="about-overlay" onclick={closeAbout} role="presentation">
			<div
				class="about-sheet"
				onclick={(e) => e.stopPropagation()}
				onkeydown={(e) => {
					if (e.key === 'Escape') closeAbout();
				}}
				role="dialog"
				aria-label="About BearPad"
				tabindex="-1"
			>
				<img class="about-icon" src={bearpawIcon} alt="" draggable="false" />
				<h2 class="about-name">BearPad</h2>
				<p class="about-version">Version {appVersion}</p>
				<p class="about-desc">
					A fast, local-first text editor for macOS, Windows, and Linux.
				</p>
				<button class="about-ok" onclick={closeAbout}>OK</button>
			</div>
		</div>
	{/if}
</div>

<style>
	:global(body) {
		margin: 0;
		font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', system-ui, sans-serif;
		background: #1e1e1e;
		color: #d4d4d4;
		--menu-bg: #252526;
		--menu-border: #3c3c3c;
		--menu-hover: #37373d;
		--menu-sep: #3c3c3c;
		--menu-text: #d4d4d4;
	}
	:global(body[data-theme="light"]) {
		background: #ffffff;
		color: #333333;
		--menu-bg: #f3f3f3;
		--menu-border: #d4d4d4;
		--menu-hover: #e2e2e2;
		--menu-sep: #d4d4d4;
		--menu-text: #1a1a1a;
	}
	.app-root {
		display: flex;
		flex-direction: column;
		height: 100vh;
	}
	.menu-bar {
		display: flex;
		align-items: stretch;
		background: var(--menu-bg);
		color: var(--menu-text);
		border-bottom: 1px solid var(--menu-border);
		user-select: none;
		position: relative;
		z-index: 1000;
		flex-shrink: 0;
	}
	.title-bar {
		display: flex;
		align-items: center;
		gap: 10px;
		height: 40px;
		padding: 0 12px;
		background: var(--menu-bg);
		border-bottom: 1px solid var(--menu-border);
		user-select: none;
		flex-shrink: 0;
		-webkit-app-region: drag;
	}
	.title-bar-mac {
		padding-left: 78px;
	}
	.title-bar-icon {
		width: 24px;
		height: 24px;
		border-radius: 5px;
	}
	.title-bar-title {
		font-size: 16px;
		font-weight: 600;
		color: var(--menu-text);
	}
	.title-bar-controls {
		margin-left: auto;
		display: flex;
		gap: 2px;
		-webkit-app-region: no-drag;
	}
	.tb-btn {
		width: 40px;
		height: 30px;
		border: none;
		background: transparent;
		color: var(--menu-text);
		font-size: 13px;
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
		border-radius: 4px;
	}
	.tb-btn:hover {
		background: var(--menu-hover);
	}
	.tb-close:hover {
		background: #e81123;
		color: #ffffff;
	}
	.tb-glyph {
		font-size: 13px;
		line-height: 1;
	}
	.about-overlay {
		position: fixed;
		inset: 0;
		z-index: 9999;
		background: rgba(0, 0, 0, 0.5);
		display: flex;
		align-items: center;
		justify-content: center;
	}
	.about-sheet {
		background: var(--menu-bg);
		border: 1px solid var(--menu-border);
		border-radius: 10px;
		padding: 24px 32px;
		width: 340px;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 8px;
		box-shadow: 0 16px 48px rgba(0, 0, 0, 0.6);
		font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', system-ui, sans-serif;
	}
	.about-icon {
		width: 72px;
		height: 72px;
		border-radius: 16px;
		margin-bottom: 6px;
	}
	.about-name {
		margin: 0;
		font-size: 18px;
		font-weight: 600;
		color: var(--menu-text);
	}
	.about-version {
		margin: 0;
		font-size: 14px;
		color: #888;
	}
	.about-desc {
		margin: 4px 0 12px;
		font-size: 13px;
		color: #999;
		text-align: center;
		line-height: 1.4;
	}
	.about-ok {
		padding: 6px 28px;
		border: none;
		border-radius: 6px;
		background: #094771;
		color: #ffffff;
		font-size: 14px;
		cursor: pointer;
	}
	.about-ok:hover {
		background: #0a5a8f;
	}
	.menu-spacer {
		flex: 1;
	}
	.menu-gear {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 30px;
		height: 28px;
		margin-right: 4px;
		border: none;
		border-radius: 6px;
		background: transparent;
		color: var(--menu-text);
		cursor: pointer;
	}
	.menu-gear:hover {
		background: var(--menu-hover);
	}
	.menu-item {
		position: relative;
	}
	.menu-label {
		display: block;
		padding: 8px 14px;
		font-size: var(--ui-font-size, 16px);
		cursor: default;
	}
	.menu-item.open .menu-label,
	.menu-item:hover .menu-label {
		background: var(--menu-hover);
	}
	.menu-dropdown {
		position: absolute;
		top: 100%;
		left: 0;
		min-width: 230px;
		background: var(--menu-bg);
		border: 1px solid var(--menu-border);
		box-shadow: 0 6px 16px rgba(0, 0, 0, 0.4);
		padding: 4px 0;
	}
	.menu-action {
		display: block;
		width: 100%;
		text-align: left;
		padding: 5px 16px;
		font-size: var(--ui-font-size, 16px);
		background: none;
		border: none;
		color: var(--menu-text);
		cursor: default;
	}
	.menu-action:hover {
		background: #094771;
		color: #ffffff;
	}
	.menu-action.menu-disabled,
	.menu-action:disabled {
		opacity: 0.4;
		cursor: default;
	}
	.menu-action:disabled:hover {
		background: none;
		color: var(--menu-text);
	}
	.menu-sep {
		height: 1px;
		background: var(--menu-sep);
		margin: 4px 8px;
	}
	.tab-bar {
		display: flex;
		align-items: stretch;
		background: var(--menu-bg);
		color: var(--menu-text);
		border-bottom: 1px solid var(--menu-border);
		font-size: var(--ui-font-size, 16px);
		user-select: none;
		flex-shrink: 0;
		overflow-x: auto;
	}
	.tab {
		display: flex;
		align-items: center;
		gap: 6px;
		padding: 5px 10px;
		border-right: 1px solid var(--menu-border);
		cursor: default;
		max-width: 200px;
	}
	.tab:hover {
		background: var(--menu-hover);
	}
	.tab.active {
		background: var(--menu-hover);
		box-shadow: inset 0 -2px 0 #094771;
	}
	.tab-new {
		align-self: center;
		margin: 0 6px;
		padding: 0 8px;
		border: none;
		background: transparent;
		color: var(--menu-text);
		font-size: 1.1em;
		line-height: 1;
		cursor: pointer;
	}
	.tab-new:hover {
		color: #ffffff;
		background: var(--menu-hover);
		border-radius: 4px;
	}
	.tab-title {
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.tab-dirty {
		color: #d4d4d4;
		font-size: 0.625em;
	}
	.tab-close {
		background: none;
		border: none;
		color: var(--menu-text);
		cursor: pointer;
		font-size: 0.85em;
		line-height: 1;
		padding: 1px 4px;
		border-radius: 3px;
	}
	.tab-close:hover {
		background: #094771;
		color: #ffffff;
	}
	.tab-pane {
		height: 100%;
	}
	.tab-pane.hidden {
		display: none;
	}
	.editor-wrap {
		flex: 1;
		min-height: 0;
	}
	:global(.editor-host) {
		height: 100%;
	}
	:global(.cm-editor) {
		height: 100%;
	}
	.app-root[data-theme="light"] .tab-dirty {
		color: #555555;
	}
	.app-root[data-theme="light"] .menu-bar {
		background: #f3f3f3;
		border-bottom-color: #d8d8d8;
	}
	.app-root[data-theme="light"] .menu-item.open .menu-label,
	.app-root[data-theme="light"] .menu-item:hover .menu-label {
		background: #e6e6e6;
	}
	.app-root[data-theme="light"] .menu-dropdown {
		background: #f3f3f3;
		border-color: #d0d0d0;
	}
	.app-root[data-theme="light"] .menu-action:hover {
		background: #0078d4;
		color: #ffffff;
	}
	.app-root[data-theme="light"] .menu-sep {
		background: #d8d8d8;
	}
</style>