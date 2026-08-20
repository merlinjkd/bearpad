<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { EditorView, keymap, ViewPlugin, Decoration, drawSelection, type DecorationSet, type ViewUpdate } from '@codemirror/view';
	import { EditorState, Compartment, StateEffect, StateField, RangeSetBuilder } from '@codemirror/state';
	import { defaultKeymap, history, historyKeymap, undo, redo } from '@codemirror/commands';
	import { markdown, markdownLanguage } from '@codemirror/lang-markdown';
	import { syntaxHighlighting, defaultHighlightStyle, syntaxTree } from '@codemirror/language';
	import { searchKeymap, search, highlightSelectionMatches, openSearchPanel, selectMatches, replaceAll } from '@codemirror/search';
	import { autocompletion, completionKeymap } from '@codemirror/autocomplete';
	import { writeText, readText } from '@tauri-apps/plugin-clipboard-manager';
	import { invoke } from '@tauri-apps/api/core';
	import { transformCase, type TransformType } from './commands';

	function syncDirty(dirty: boolean) {
		onDirtyChange?.(dirty);
	}

	export interface EditorExposed {
		hasSelection: () => boolean;
		handleCut: () => Promise<void>;
		handleCopy: () => Promise<void>;
		handlePaste: () => Promise<void>;
		undo: () => void;
		redo: () => void;
		handleSelectAll: () => void;
		openFind: () => void;
		findAll: () => void;
		replaceAllMatches: () => void;
		transformSelection: (type: 'lowercase' | 'uppercase' | 'propercase') => void;
		markSaved: () => void;
		isDirty: () => boolean;
		getContent: () => string;
		recheckSpelling: () => void;
		getMisspellingAt: (x: number, y: number) => { from: number; to: number; text: string } | null;
		replaceRange: (from: number, to: number, text: string) => void;
	}

	let {
		onReady,
		doc = '',
		onDirtyChange,
		theme = 'dark',
		fontSize = 18,
		fontFamily = "\"Consolas\", 'SF Mono', 'Fira Code', 'Cascadia Code', monospace",
		wordWrap = true,
		spellcheck = true,
		spellLang = 'en_US',
		cursorBlink = false,
		textColor = '',
		onStatusChange,
	}: {
		onReady?: (ref: EditorExposed) => void;
		doc?: string;
		onDirtyChange?: (dirty: boolean) => void;
		onStatusChange?: (status: { line: number; col: number; selCount: number }) => void;
		theme?: string;
		fontSize?: number;
		fontFamily?: string;
		wordWrap?: boolean;
		spellcheck?: boolean;
		spellLang?: string;
		cursorBlink?: boolean;
		textColor?: string;
	} = $props();

	let container: HTMLDivElement;
	let view: EditorView;
	let dirty = false;

	const themeCompartment = new Compartment();
	const fontSizeCompartment = new Compartment();
	const fontFamilyCompartment = new Compartment();
	const wrapCompartment = new Compartment();
	const spellcheckCompartment = new Compartment();
	const blinkCompartment = new Compartment();
	const textColorCompartment = new Compartment();

	// Empty string = theme default; otherwise override the editor text color.
	// !important beats the per-token highlight colors (defaultHighlightStyle
	// paints .cm-content spans with syntax colors via hashed classes).
	const textColorStyle = (c: string) =>
		c
			? EditorView.theme({
					'& .cm-content': { color: `${c} !important` },
					// cover plain text and non-link tokens but keep links + errors on theme colors
					'& .cm-content span:not(.bp-link):not(.bp-link *)': { color: `${c} !important` },
					'& .cm-content .cm-spell-error': { color: '#80DEEA !important' },
				})
			: [];

	// Kill the base-theme blink animation when cursorBlink is off (default).
	// !important: the base rule and this one tie on specificity, and sheet
	// order between StyleModule mounts is not reliably later — important wins.
	const cursorBlinkStyle = (blink: boolean) =>
		blink ? [] : EditorView.theme({ '&.cm-focused > .cm-scroller > .cm-cursorLayer': { animation: 'none !important' } });

	// ─── link color (dark themes) ─────────────────────────
	// Bulletproof override: mark every markdown Link node with our own
	// stable class via a decoration plugin, then color it with theme CSS
	// (higher specificity than any HighlightStyle rule, no reliance on
	// module ordering). Works identically on every platform/build.
	const linkMark = Decoration.mark({ class: 'bp-link' });
	const linkClassPlugin = ViewPlugin.fromClass(
		class {
			decorations: DecorationSet;
			constructor(view: EditorView) {
				this.decorations = this.build(view);
			}
			update(update: ViewUpdate) {
				if (update.docChanged || update.viewportChanged) {
					this.decorations = this.build(update.view);
				}
			}
			build(view: EditorView): DecorationSet {
				const builder = new RangeSetBuilder<Decoration>();
				syntaxTree(view.state).iterate({
					enter: (node) => {
						if (node.name === 'Link' || node.name === 'URL') {
							builder.add(node.from, node.to, linkMark);
						}
					},
				});
				return builder.finish();
			}
		},
		{ decorations: (v) => v.decorations }
	);
	const darkLinkOverride = {
		'.cm-content .bp-link': { color: '#80DEEA !important' },
		'.cm-content .bp-link *': { color: '#80DEEA !important' },
	};

	const setSpellErrors = StateEffect.define<{ from: number; to: number }[]>();
	const recheckSpell = StateEffect.define<null>();

	const spellField = StateField.define<DecorationSet>({
		create: () => Decoration.none,
		update(deco, tr) {
			deco = deco.map(tr.changes);
			for (const e of tr.effects) {
				if (e.is(setSpellErrors)) {
					deco = Decoration.set(
						e.value.map((r) => Decoration.mark({ class: 'cm-spell-error' }).range(r.from, r.to))
					);
				}
			}
			return deco;
		},
	});

	// must be provided to the decorations facet or CM never draws the marks
	const spellDecorations = EditorView.decorations.from(spellField, (d) => d);

	let langRef = $state(spellLang);

	const spellUnderline = EditorView.baseTheme({
		'.cm-spell-error': { textDecoration: 'underline wavy #e74c3c', textDecorationSkipInk: 'none' },
	});
	// squiggle matches the theme's link color (light: default link #219;
	// dark modes: #80DEEA), overriding the base red.
	const spellSquiggle = (color: string) => ({ '.cm-spell-error': { textDecorationColor: color } });

	function spellCheckPlugin() {
		let timer: number | undefined;
		let gen = 0;
		return ViewPlugin.fromClass(
			class {
				update(update: ViewUpdate) {
					const isRecheck = update.transactions.some((tr) =>
						tr.effects.some((e) => e.is(recheckSpell))
					);
					if (!update.docChanged && !isRecheck) return;
					clearTimeout(timer);
					const myGen = ++gen;
					timer = window.setTimeout(async () => {
						if (myGen !== gen) return;
						try {
							const text = update.state.doc.toString();
							const hits = await invoke<{ start: number; end: number; word: string }[]>(
								'spell_check',
								{ text, lang: langRef }
							);
							if (myGen !== gen) return;
							update.view.dispatch({
								effects: setSpellErrors.of(hits.map((h) => ({ from: h.start, to: h.end }))),
							});
						} catch {
							/* not running inside Tauri (dev browser) */
						}
					}, 400);
				}
				destroy() {
					clearTimeout(timer);
				}
			}
		);
	}

	function computeTheme(themeName: string) {
		const searchPanelDark = {
			// the wrapper carries the light base-theme bg; must paint it too
			'.cm-panels': {
				backgroundColor: '#252526',
				color: '#d4d4d4',
			},
			'.cm-panel.cm-search': {
				backgroundColor: '#252526',
				color: '#d4d4d4',
			},
			'.cm-panel.cm-search input': {
				backgroundColor: '#3c3c3c',
				color: '#d4d4d4',
				border: '1px solid #555',
			},
			'.cm-panel.cm-search button': {
				backgroundColor: '#e0e0e0',
				color: '#1e1e1e',
				border: '1px solid #888',
			},
			'.cm-panel.cm-search label': { color: '#d4d4d4' },
			// the close X sits above the panel; give it a visible glyph + hover
			'.cm-panel.cm-search [name=close]': {
				backgroundColor: 'transparent',
				color: '#d4d4d4',
				fontSize: '14px',
				padding: '2px 6px',
				cursor: 'pointer',
			},
			'.cm-panel.cm-search [name=close]:hover': {
				color: '#ffffff',
				backgroundColor: '#3a3a3a',
			},
		};
		if (themeName === 'light') {
			return EditorView.theme({
				'&': { backgroundColor: '#ffffff', color: '#333333', height: '100%' },
				'.cm-gutters': { backgroundColor: '#f5f5f5', color: '#999999', border: 'none' },
				'.cm-activeLineGutter': { backgroundColor: '#e8e8e8' },
				'.cm-activeLine': { backgroundColor: '#f0f0f044' },
				'.cm-cursor': { borderLeft: '2px solid #323232' },
				'.cm-selectionBackground': { backgroundColor: '#add6ff' },
				'.cm-focused .cm-selectionBackground': { backgroundColor: '#add6ff' },
				'.cm-matchingBracket': { backgroundColor: '#d4d4d4' },
				...spellSquiggle('#219'),
				});
		}
		return EditorView.theme({
			'&': { backgroundColor: '#1e1e1e', color: '#d4d4d4', height: '100%' },
			'.cm-gutters': { backgroundColor: '#252526', color: '#858585', border: 'none' },
			'.cm-activeLineGutter': { backgroundColor: '#2a2d2e' },
			'.cm-activeLine': { backgroundColor: '#2a2d2e44' },
			'.cm-cursor': { borderLeft: '2px solid #fafafa' },
			'.cm-selectionBackground': { backgroundColor: '#264f78' },
			'.cm-focused .cm-selectionBackground': { backgroundColor: '#264f78' },
			'.cm-matchingBracket': { backgroundColor: '#4b4b4b' },
			...searchPanelDark,
			...darkLinkOverride,
			...spellSquiggle('#80DEEA'),
		});
	}

	function computeFontSize(size: number) {
		return EditorView.theme({
			'&': { fontSize: `${size}px` },
		});
	}

	function computeFontFamily(family: string) {
		return EditorView.theme({
			'.cm-scroller': { fontFamily: family },
		});
	}

	function createEditor() {
		const state = EditorState.create({
			doc,
			extensions: [
				history(),
				drawSelection(),
				keymap.of([...defaultKeymap, ...historyKeymap, ...searchKeymap, ...completionKeymap]),
				search({ top: true }),
				highlightSelectionMatches(),
				markdown({ base: markdownLanguage }),
				linkClassPlugin,
				syntaxHighlighting(defaultHighlightStyle),
				blinkCompartment.of(cursorBlinkStyle(cursorBlink)),
				textColorCompartment.of(textColorStyle(textColor)),
				autocompletion(),
				themeCompartment.of(computeTheme(theme)),
				fontSizeCompartment.of(computeFontSize(fontSize)),
				fontFamilyCompartment.of(computeFontFamily(fontFamily)),
				wrapCompartment.of(wordWrap ? [EditorView.lineWrapping] : []),
				spellcheckCompartment.of(spellcheck ? [spellUnderline, spellField, spellDecorations, spellCheckPlugin()] : []),
				EditorView.updateListener.of((update) => {
					if (update.docChanged) {
						dirty = true;
						syncDirty(true);
					}
					// cursor / selection moved (or doc changed) -> refresh status bar
					if (update.selectionSet || update.docChanged) {
						const sel = update.state.selection.main;
						const head = sel.head;
						const line = update.state.doc.lineAt(head).number;
						const col = head - update.state.doc.lineAt(head).from + 1;
						const selCount = sel.empty ? 0 : update.state.sliceDoc(sel.from, sel.to).length;
						onStatusChange?.({ line, col, selCount });
					}
				}),
			],
		});

		view = new EditorView({
			state,
			parent: container,
		});

		// Seed the status bar with the initial cursor position.
		onStatusChange?.({ line: 1, col: 1, selCount: 0 });
	}

	function getSelectedText(): string {
		const sel = view.state.selection.main;
		return sel.empty ? '' : view.state.sliceDoc(sel.from, sel.to);
	}

	function replaceSelection(text: string) {
		const sel = view.state.selection.main;
		if (sel.empty) return;
		view.dispatch({
			changes: { from: sel.from, to: sel.to, insert: text },
			selection: { anchor: sel.from + text.length },
		});
	}

	$effect(() => {
		const th = theme;
		if (!view) return;
		view.dispatch({
			effects: [themeCompartment.reconfigure(computeTheme(th))],
		});
	});

	$effect(() => {
		const s = fontSize;
		if (!view) return;
		view.dispatch({
			effects: fontSizeCompartment.reconfigure(computeFontSize(s)),
		});
	});

	$effect(() => {
		const f = fontFamily;
		if (!view) return;
		view.dispatch({
			effects: fontFamilyCompartment.reconfigure(computeFontFamily(f)),
		});
	});

	$effect(() => {
		const w = wordWrap;
		if (!view) return;
		view.dispatch({
			effects: wrapCompartment.reconfigure(w ? [EditorView.lineWrapping] : []),
		});
	});

	$effect(() => {
		const s = spellcheck;
		if (!view) return;
		view.dispatch({
			effects: spellcheckCompartment.reconfigure(s ? [spellUnderline, spellField, spellDecorations, spellCheckPlugin()] : []),
		});
	});

	$effect(() => {
		if (!view || langRef === spellLang) return;
		langRef = spellLang;
		view.dispatch({ effects: recheckSpell.of(null) });
	});

	$effect(() => {
		const c = textColor;
		if (!view) return;
		view.dispatch({
			effects: textColorCompartment.reconfigure(textColorStyle(c)),
		});
	});

	$effect(() => {
		const b = cursorBlink;
		if (!view) return;
		view.dispatch({
			effects: blinkCompartment.reconfigure(cursorBlinkStyle(b)),
		});
	});

	onMount(() => {
		createEditor();

		if (onReady) {
			onReady({
				hasSelection: () => !view.state.selection.main.empty,

				handleCopy: async () => {
					const text = getSelectedText();
					if (!text) return;
					try {
						await writeText(text);
					} catch {
						await navigator.clipboard.writeText(text);
					}
				},

				handleCut: async () => {
					const text = getSelectedText();
					if (!text) return;
					try {
						await writeText(text);
					} catch {
						await navigator.clipboard.writeText(text);
					}
					const sel = view.state.selection.main;
					view.dispatch({
						changes: { from: sel.from, to: sel.to, insert: '' },
					});
				},

				handlePaste: async () => {
					let rawText = '';
					try {
						rawText = (await readText()) ?? '';
						if (!rawText) rawText = await navigator.clipboard.readText();
					} catch {
						try {
							rawText = await navigator.clipboard.readText();
						} catch {
							rawText = '';
						}
					}
					if (!rawText) return;
					const sel = view.state.selection.main;
					view.dispatch({
						changes: { from: sel.from, to: sel.to, insert: rawText },
						selection: { anchor: sel.from + rawText.length },
					});
				},

				undo: () => {
					undo(view);
				},

				redo: () => {
					redo(view);
				},

				handleSelectAll: () => {
					view.dispatch({
						selection: { anchor: 0, head: view.state.doc.length },
					});
				},

				openFind: () => {
					openSearchPanel(view);
				},

				findAll: () => {
					selectMatches(view);
				},

				replaceAllMatches: () => {
					replaceAll(view);
				},

				transformSelection: (type: TransformType) => {
					const text = getSelectedText();
					if (!text) return;
					replaceSelection(transformCase(text, type));
				},

				markSaved: () => {
					dirty = false;
					syncDirty(false);
				},

				isDirty: () => dirty,

				getContent: () => view.state.doc.toString(),

				recheckSpelling: () => {
					view.dispatch({ effects: recheckSpell.of(null) });
				},

				getMisspellingAt: (x, y) => {
					const pos = view.posAtCoords({ x, y });
					if (pos == null) return null;
					const set = view.state.field(spellField, false);
					if (!set) return null;
					let res: { from: number; to: number; text: string } | null = null;
					set.between(pos - 1, pos + 1, (from, to) => {
						if (!res && from <= pos && pos < to) {
							res = { from, to, text: view.state.sliceDoc(from, to) };
						}
					});
					return res;
				},

				replaceRange: (from, to, text) => {
					view.dispatch({ changes: { from, to, insert: text } });
				},
				});
		}
	});

	onDestroy(() => {
		view?.destroy();
	});
</script>

<div class="editor-host" bind:this={container}></div>