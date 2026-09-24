<script lang="ts">
	import { onMount } from 'svelte';
	import Editor, { type EditorExposed } from './lib/Editor.svelte';
	import ContextMenu from './lib/ContextMenu.svelte';
	import SettingsModal from './lib/SettingsModal.svelte';
	import { invoke } from '@tauri-apps/api/core';
	import { open, save as showSaveDialog, confirm as showConfirm } from '@tauri-apps/plugin-dialog';
	import { getCurrentWindow } from '@tauri-apps/api/window';
	import bearpawIcon from './assets/bearpaw.png';
import {
	FilePlus, FolderOpen, Save, Info, Power,
	Undo2, Redo2, Scissors, Copy, ClipboardPaste, Search,
	ZoomIn, ZoomOut, RotateCcw, Moon, Sun, WrapText, SpellCheck,
	CaseLower, CaseUpper, CaseSensitive,
} from 'lucide-svelte';

	import { editorCommands } from './lib/commands';

	// Platform detection used ONLY to choose which titlebar slot holds the window
	// controls. Deliberately navigator.userAgent, not @tauri-apps/plugin-os: it is
	// synchronous, needs no Cargo crate, no capability entry and no async first-paint
	// flash. See the cross-platform UI standard in the tauri-desktop-app skill.
	const isMac = navigator.userAgent.includes('Macintosh');

	type Theme = 'dark' | 'light' | 'system';

	const FILTERS = [
		{ name: 'Text', extensions: ['txt'] },
		{ name: 'Markdown', extensions: ['md'] },
	];

	// Put the chosen default format first so the native save dialog presets it.
	function prioritizedFilters(f: 'txt' | 'md') {
		if (f === 'md') return [FILTERS[1], FILTERS[0]];
		return FILTERS;
	}

	interface TabState {
		id: number;
		path: string | null;
		doc: string;
		ref: EditorExposed | null;
		name?: string;
		lastChange?: number;
	}

	let tabs = $state<TabState[]>([{ id: 0, path: null, doc: '', ref: null, name: 'new 1' }]);
	let activeTabId = $state(0);
	let tabSeq = 1;
	let newTabSeq = 2;
	let dirtyMap = $state<Record<number, boolean>>({});
	let status = $state({ line: 1, col: 1, selCount: 0 });
	let showSettings = $state(false);
	let showAbout = $state(false);
	let appVersion = $state('');
	let theme = $state<Theme>('dark');
	let fontSize = $state(18);
	let fontFamily = $state("\"Consolas\", 'SF Mono', 'Fira Code', 'Cascadia Code', monospace");
	let uiFontSize = $state(16);
	let wordWrap = $state(true);
	let spellcheck = $state(true);
	let spellLang = $state('en_US');
	let cursorBlink = $state(false);
	let textColor = $state('');
	let defaultFormat = $state<'txt' | 'md'>('txt');
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
			icon?: any;
		}[];
	}[] = [
		{
			label: 'File',
			items: [
				{ label: 'New', icon: FilePlus, action: () => newFile() },
				{ label: 'Open...', icon: FolderOpen, action: () => openFile() },
				{ separator: true },
				{ label: 'Save', icon: Save, action: () => saveFile() },
				{ label: 'Save As...', icon: Save, action: () => saveFileAs() },
				{ separator: true },
				{ label: 'About BearPad...', icon: Info, action: () => openAbout() },
				{ separator: true },
				{ label: 'Exit', icon: Power, action: () => getCurrentWindow().close() },
			],
		},
		{
			label: 'Edit',
			items: [
				{ ...menuItem('undo'), icon: Undo2 },
				{ ...menuItem('redo'), icon: Redo2 },
				{ separator: true },
				{ ...menuItem('cut'), icon: Scissors },
				{ ...menuItem('copy'), icon: Copy },
				{ ...menuItem('paste'), icon: ClipboardPaste },
				{ separator: true },
				{ ...menuItem('find'), icon: Search },
			],
		},
		{
			label: 'View',
			items: [
				{
					label: 'Zoom In',
					icon: ZoomIn,
					action: () => handleSettingsChange({ fontSize: Math.min(32, fontSize + 1) }),
				},
				{
					label: 'Zoom Out',
					icon: ZoomOut,
					action: () => handleSettingsChange({ fontSize: Math.max(10, fontSize - 1) }),
				},
				{ label: 'Reset Zoom', icon: RotateCcw, action: () => handleSettingsChange({ fontSize: 18 }) },
				{ separator: true },
				{
					label: 'Toggle Theme',
					icon: resolvedTheme === 'dark' ? Sun : Moon,
					action: () =>
						handleSettingsChange({
							theme: resolvedTheme === 'dark' ? 'light' : 'dark',
						}),
				},
				{
					label: 'Toggle Word Wrap',
					icon: WrapText,
					action: () => handleSettingsChange({ wordWrap: !wordWrap }),
				},
				{
					label: 'Toggle Spell Check',
					icon: SpellCheck,
					action: () => handleSettingsChange({ spellcheck: !spellcheck }),
				},
			],
		},
		{
			label: 'Text',
			items: [
				{ ...menuItem('lowercase'), icon: CaseLower },
				{ ...menuItem('uppercase'), icon: CaseUpper },
				{ ...menuItem('propercase'), icon: CaseSensitive },
			],
		},
	];

	// ─── helpers ────────────────────────────────────────

	function fileName(tab: TabState | null) {
		if (!tab?.path) return tab?.name ?? 'new';
		// Windows paths use '\' — splitting on '/' alone returns the whole path there.
		return tab.path.split(/[/\\]/).pop() || 'new';
	}

	function activeTab() {
		return tabs.find((t) => t.id === activeTabId) ?? null;
	}

	function updateTitle() {
		try {
			const tab = activeTab();
			const win = getCurrentWindow();
			// setTitle returns a Promise, so the try/catch below only guards
			// SYNCHRONOUS failures. A rejected call (e.g. the permission was
			// missing) surfaces as an unhandled rejection instead of a throw —
			// which is exactly how the missing core:window:allow-set-title
			// capability stayed invisible. Catch it explicitly.
			win
				.setTitle(`BearPad — ${fileName(tab)}${tab?.ref?.isDirty() ? ' ●' : ''}`)
				.catch(() => {});
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
			untitledSeq: newTabSeq,
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
						name: r.name ?? `new ${newTabSeq}`,
					};
					tabs.push(tab);
				}
				activeTabId = tabs[0]?.id ?? null;
				newTabSeq = Math.max(newTabSeq, (rec.untitledSeq ?? 1) + tabs.length);
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
		const tab: TabState = { id: tabSeq++, path: null, doc: '', ref: null, name: `new ${newTabSeq++}` };
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
		const selected = await showSaveDialog({
			filters: prioritizedFilters(defaultFormat),
			defaultPath: `untitled.${defaultFormat}`,
		});
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
			if (s.defaultFormat === 'md' || s.defaultFormat === 'txt') defaultFormat = s.defaultFormat;
		} catch { /* defaults */ }
		resolveTheme();
	}

	async function saveSettings() {
		try {
			await invoke('write_settings', {
				json: JSON.stringify({ theme, fontSize, uiFontSize, fontFamily, wordWrap, spellcheck, spellLang, cursorBlink, textColor, defaultFormat }),
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
		patch: Partial<{ theme: Theme; fontSize: number; uiFontSize: number; fontFamily: string; wordWrap: boolean; spellcheck: boolean; spellLang: string; cursorBlink: boolean; textColor: string; defaultFormat: 'txt' | 'md' }>,
	) {
		if (patch.theme !== undefined) theme = patch.theme;
		if (patch.fontSize !== undefined) fontSize = patch.fontSize;
		if (patch.uiFontSize !== undefined) uiFontSize = patch.uiFontSize;
		if (patch.fontFamily !== undefined) fontFamily = patch.fontFamily;
		if (patch.wordWrap !== undefined) wordWrap = patch.wordWrap;
		if (patch.spellcheck !== undefined) spellcheck = patch.spellcheck;
		if (patch.spellLang !== undefined) spellLang = patch.spellLang;
		if (patch.cursorBlink !== undefined) cursorBlink = patch.cursorBlink;
		if (patch.textColor !== undefined) textColor = patch.textColor;
		if (patch.defaultFormat !== undefined) defaultFormat = patch.defaultFormat;
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
							for (const s of suggestions.slice(0, 5)) {
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

{#snippet windowControls()}
	{#if isMac}
		<!-- macOS: hand-drawn traffic lights in the native order (close / minimize / zoom)
		     and geometry — 12px circles, 8px apart, 20px in from the window edge, glyphs
		     appearing only on hover. The window is undecorated on every platform, so this
		     is a close approximation, not the system's own controls. -->
		<div class="title-bar-controls group/tl flex gap-2 pl-2 [-webkit-app-region:no-drag]">
			<button class="size-3 shrink-0 rounded-full ring-[0.5px] ring-inset ring-black/25 bg-[#ff5f57] cursor-default flex items-center justify-center" aria-label="Close" onclick={() => getCurrentWindow().close()}>
				<span class="tl-glyph text-[7px] leading-none text-[#4d0000] opacity-0 group-hover/tl:opacity-100">✕</span>
			</button>
			<button class="size-3 shrink-0 rounded-full ring-[0.5px] ring-inset ring-black/25 bg-[#febc2e] cursor-default flex items-center justify-center" aria-label="Minimize" onclick={() => getCurrentWindow().minimize()}>
				<span class="tl-glyph text-[7px] leading-none text-[#5a3d00] opacity-0 group-hover/tl:opacity-100">─</span>
			</button>
			<button class="size-3 shrink-0 rounded-full ring-[0.5px] ring-inset ring-black/25 bg-[#28c840] cursor-default flex items-center justify-center" aria-label="Zoom" onclick={() => getCurrentWindow().toggleMaximize()}>
				<span class="tl-glyph text-[7px] leading-none text-[#0d3d00] opacity-0 group-hover/tl:opacity-100">+</span>
			</button>
		</div>
	{:else}
		<div class="title-bar-controls flex gap-0.5 [-webkit-app-region:no-drag]">
			<button class="tb-btn w-10 h-[30px] border-0 bg-transparent text-title-text text-[13px] cursor-pointer flex items-center justify-center rounded hover:bg-menu-hover" aria-label="Minimize" onclick={() => getCurrentWindow().minimize()}>
				<span class="tb-glyph text-[13px] leading-none">─</span>
			</button>
			<button class="tb-btn w-10 h-[30px] border-0 bg-transparent text-title-text text-[13px] cursor-pointer flex items-center justify-center rounded hover:bg-menu-hover" aria-label="Maximize" onclick={() => getCurrentWindow().toggleMaximize()}>
				<span class="tb-glyph text-[13px] leading-none">□</span>
			</button>
			<button class="tb-btn tb-close w-10 h-[30px] border-0 bg-transparent text-title-text text-[13px] cursor-pointer flex items-center justify-center rounded hover:bg-[#e81123] hover:text-white" aria-label="Close" onclick={() => getCurrentWindow().close()}>
				<span class="tb-glyph text-[13px] leading-none">✕</span>
			</button>
		</div>
	{/if}
{/snippet}

<div class="app-root" data-theme={resolvedTheme} style="--ui-font-size:{uiFontSize}px">
	<div class="title-bar flex items-center gap-2.5 h-10 px-3 bg-title-bg border-b border-menu-border select-none shrink-0 [-webkit-app-region:drag]" data-tauri-drag-region>
		<!-- Three semantic slots: leading controls / content / trailing controls. Which
		     slot holds the controls is the ONLY platform difference, and it is structural
		     rather than a `flex-direction: row-reverse` hack, so the icon and title never
		     move. Controls are hand-drawn on every OS - no macOS overlay, no native
		     traffic lights. See the cross-platform UI standard in the tauri-desktop-app skill. -->
		<div class="titlebar-slot flex items-center gap-0.5" data-tauri-drag-region>
			{#if isMac}{@render windowControls()}{/if}
		</div>
		<div class="titlebar-content flex items-center gap-2.5 flex-1 min-w-0" data-tauri-drag-region>
			<img class="title-bar-icon w-6 h-6 rounded-[5px]" src={bearpawIcon} alt="" draggable="false" />
			<span class="title-bar-title text-[16px] font-semibold text-title-text" data-tauri-drag-region>BearPad - {fileName(activeTab())}</span>
		</div>
		<div class="titlebar-slot flex items-center gap-0.5" data-tauri-drag-region>
			{#if !isMac}{@render windowControls()}{/if}
		</div>
	</div>
	<div class="menu-bar flex items-stretch bg-menu-bg text-menu-text border-b border-menu-border select-none relative z-[1000] shrink-0" role="menubar">
		{#each menus as menu, i (menu.label)}
			<!-- Styling lives in utilities now; the `open` state is expressed as a class
			     on the item so hover and open share one background, and the `menu-bar`
			     class is retained because App.svelte's document click handler does
			     .closest('.menu-bar') to decide whether to close an open menu. -->
			<div
				class="menu-item group relative {openMenu === i ? 'bg-menu-hover' : ''} hover:bg-menu-hover"
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
				<span class="menu-label block px-3.5 py-2 text-ui cursor-default">{menu.label}</span>
				{#if openMenu === i}
					<div class="menu-dropdown absolute top-full left-0 min-w-[230px] bg-menu-bg border border-menu-border shadow-[0_6px_16px_rgba(0,0,0,0.4)] py-1" role="menu">
						{#each menu.items as item}
							{@const disabled = typeof item.disabled === 'function' ? item.disabled() : item.disabled}
							{#if item.separator}
								<div class="menu-sep h-px bg-menu-sep my-1 mx-2"></div>
							{:else}
								<button
									class="menu-action flex items-center gap-2.5 w-full text-left py-[5px] px-4 text-ui bg-transparent border-0 text-menu-text cursor-default hover:bg-[#094771] hover:text-white disabled:opacity-40 disabled:cursor-default disabled:hover:bg-transparent disabled:hover:text-menu-text"
									role="menuitem"
									disabled={disabled}
									onclick={(e) => {
										e.stopPropagation();
										if (disabled) return;
										item.action?.();
										openMenu = null;
									}}
								>
									{#if item.icon}
										<span class="menu-action-icon inline-flex items-center justify-center shrink-0 opacity-90">
											<svelte:component this={item.icon} size={15} strokeWidth={1.75} />
										</span>
									{/if}
									{item.label}
								</button>
							{/if}
						{/each}
					</div>
				{/if}
			</div>
		{/each}
		<div class="menu-spacer flex-1"></div>
		<button
			class="menu-gear flex items-center justify-center w-[34px] mr-1 border-0 rounded-md bg-transparent text-menu-text cursor-pointer hover:bg-menu-hover"
			aria-label="Settings"
			title="Settings"
			onclick={() => openSettings()}
		>
			<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
				<circle cx="12" cy="12" r="3"></circle>
				<path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 1 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 1 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"></path>
			</svg>
		</button>
	</div>

	<div class="tab-bar flex items-stretch bg-menu-bg text-menu-text border-b border-menu-border text-ui select-none shrink-0 overflow-x-auto" role="tablist">
		{#each tabs as tab (tab.id)}
			<div
				class="tab flex items-center gap-1.5 py-[5px] px-2.5 border-r border-menu-border cursor-default max-w-[200px] hover:bg-menu-hover {tab.id === activeTabId ? 'bg-menu-hover shadow-[inset_0_-2px_0_#094771]' : ''}"
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
				<span class="tab-title whitespace-nowrap overflow-hidden text-ellipsis">{fileName(tab)}</span>
				{#if dirtyMap[tab.id]}
					<span class="tab-dirty text-[#d4d4d4] text-[0.625em]" title="unsaved">●</span>
				{/if}
				<button
					class="tab-close bg-transparent border-0 text-menu-text cursor-pointer text-[0.85em] leading-none py-px px-1 rounded-[3px] hover:bg-[#094771] hover:text-white"
					aria-label="Close tab"
					onclick={(e) => {
						e.stopPropagation();
						closeTab(tab.id);
					}}
					>×</button>
			</div>
		{/each}
		<button
			class="tab-new self-center my-0 mx-1.5 py-0 px-2 border-0 bg-transparent text-menu-text text-[1.1em] leading-none cursor-pointer hover:text-white hover:bg-menu-hover hover:rounded-[4px]"
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
					onStatusChange={(s) => (status = s)}
					/>
			</div>
		{/each}
	</div>

	<div class="status-bar flex items-center justify-between gap-3 h-6 px-2.5 bg-menu-bg text-menu-text border-t border-menu-border text-[0.8125em] font-[-apple-system,BlinkMacSystemFont,'Segoe_UI',system-ui,sans-serif] select-none shrink-0" role="status">
		<span class="status-path overflow-hidden text-ellipsis whitespace-nowrap opacity-85" title={activeTab()?.path ?? ''}>
			{fileName(activeTab())}
		</span>
		<span class="status-right flex items-center gap-3.5 shrink-0">
			{#if status.selCount > 0}
				<span class="status-item whitespace-nowrap">{status.selCount} selected</span>
			{/if}
			<span class="status-item whitespace-nowrap">Ln {status.line}, Col {status.col}</span>
		</span>
	</div>

	{#if ctxMenu.show}
		<ContextMenu contextMenu={ctxMenu} onhide={hideMenu} />
	{/if}

	{#if showSettings}
		<SettingsModal
			settings={{ theme, fontSize, uiFontSize, fontFamily, wordWrap, spellcheck, spellLang, cursorBlink, textColor, defaultFormat }}
			onChange={handleSettingsChange}
			onClose={closeSettings}
		/>
	{/if}

	{#if showAbout}
		<div class="about-overlay fixed inset-0 z-[9999] bg-black/50 flex items-center justify-center" onclick={closeAbout} role="presentation">
			<div
				class="about-sheet bg-menu-bg border border-menu-border rounded-[10px] py-6 px-8 w-[340px] flex flex-col items-center gap-2 shadow-[0_16px_48px_rgba(0,0,0,0.6)] font-[-apple-system,BlinkMacSystemFont,'Segoe_UI',system-ui,sans-serif]"
				onclick={(e) => e.stopPropagation()}
				onkeydown={(e) => {
					if (e.key === 'Escape') closeAbout();
				}}
				role="dialog"
				aria-label="About BearPad"
				tabindex="-1"
			>
				<img class="about-icon w-[72px] h-[72px] rounded-2xl mb-1.5" src={bearpawIcon} alt="" draggable="false" />
				<h2 class="about-name m-0 text-[18px] font-semibold text-menu-text">BearPad</h2>
				<p class="about-version m-0 text-[14px] text-[#888]">Version {appVersion}</p>
				<p class="about-desc mt-1 mb-3 mx-0 w-full text-[13px] text-[#999] text-left leading-[1.4]">
					A small fast cross platform text and markdown editor. It features Scalable Fonts and UI, Text Transformation, Find and Replace, Multiple Tabs and Autosave. Built in Rust and Tauri.
				</p>
				<button class="about-ok py-1.5 px-7 border-0 rounded-md bg-[#094771] text-white text-[14px] cursor-pointer hover:bg-[#0a5a8f]" onclick={closeAbout}>OK</button>
			</div>
		</div>
	{/if}
</div>

<style>
	:global(body) {
		margin: 0;
		font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', system-ui, sans-serif;
		background: #000000;
		color: #F0F0F0;
		--menu-bg: #252526;
		--menu-border: #3c3c3c;
		--menu-hover: #37373d;
		--menu-sep: #3c3c3c;
		--menu-text: #F0F0F0;
		--title-bg: #252526;
		--title-text: #F0F0F0;
	}
	/* Light palette: Bearded Theme "Milkshake Vanilla Banana"
	   (editor bg/fg, menu bar bg/fg, title bar bg/fg only) */
	:global(body[data-theme="light"]) {
		background: #ece7da;
		color: #000000;
		--menu-bg: #efebe1;
		--menu-border: #d2c6a7;
		--menu-hover: #ddd4bd;
		--menu-sep: #d2c6a7;
		--menu-text: #000000;
		--title-bg: #d5c9ac;
		--title-text: #555045;
	}
	.app-root {
		display: flex;
		flex-direction: column;
		height: 100vh;
	}
	/* Menu-bar styling now lives in Tailwind utilities on the markup. The `menu-bar`
	   class is deliberately retained as a JS hook: App.svelte's document click handler
	   uses .closest('.menu-bar') to decide whether to close an open menu. */
	/* .status-bar / .status-path / .status-right / .status-item now use Tailwind
	   utilities on the markup. */
	/* .title-bar / .titlebar-slot / .titlebar-content / .title-bar-icon /
	   .title-bar-title / .title-bar-controls / .tb-btn / .tb-close / .tb-glyph now use
	   Tailwind utilities on the markup. The drag regions use the arbitrary property
	   [-webkit-app-region:drag] / [-webkit-app-region:no-drag] (no utility exists for
	   a vendor-prefixed property). The data-tauri-drag-region attributes are unchanged. */
	/* Three semantic slots. The content slot absorbs the free space, which pushes the
	   TRAILING controls slot to the far edge on Windows/Linux; on macOS the LEADING slot
	   holds the controls instead and the content simply follows it. No row-reverse, so
	   the icon and title never get relocated. */
	/* .about-overlay / .about-sheet / .about-icon / .about-name / .about-version /
	   .about-desc / .about-ok now use Tailwind utilities on the markup. Unlike the
	   settings modal, this dialog IS theme-aware (it reads --menu-bg / --menu-border /
	   --menu-text), so it maps to the bg-menu-bg / border-menu-border / text-menu-text
	   tokens and follows light/dark automatically. */
	/* .menu-spacer / .menu-gear / .menu-item / .menu-label / .menu-dropdown /
	   .menu-action / .menu-action-icon / .menu-sep now use Tailwind utilities on the
	   markup. Deleting these rules is REQUIRED, not cosmetic: Svelte scopes component
	   selectors to two classes (.menu-label.svelte-xxxxx), so an old rule outranks a
	   single-class utility on specificity and the utilities would silently do nothing. */
	/* .tab-bar / .tab / .tab-new / .tab-title / .tab-dirty / .tab-close now use
	   Tailwind utilities on the markup; the rules had to be DELETED (a scoped rule
	   outranks a single-class utility on specificity, so leaving them would silently
	   defeat every utility). .tab-pane below is deliberately NOT migrated: it is the
	   editor pane wrapper, a different surface. */
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
	.app-root[data-theme="light"] .menu-action:hover {
		background: var(--menu-hover);
		color: var(--menu-text);
	}
</style>