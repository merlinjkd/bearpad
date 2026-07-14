<script lang="ts">
	import { onMount, onDestroy } from "svelte";
	import { tabManager } from "../stores/tabs.svelte.js";
	import { settings } from "../stores/settings.svelte.js";
	import { t } from '../utils/i18n.js';

	import { EditorView, keymap, lineNumbers, highlightSpecialChars, drawSelection, highlightActiveLine, highlightActiveLineGutter, placeholder, scrollPastEnd } from "@codemirror/view";
	import { EditorState, Compartment, Prec } from "@codemirror/state";
	import { defaultKeymap, history, historyKeymap, indentLess, indentMore, undo as cmUndo, redo as cmRedo } from "@codemirror/commands";
	import { markdown, markdownLanguage } from "@codemirror/lang-markdown";
	import { syntaxHighlighting, defaultHighlightStyle, syntaxTree } from "@codemirror/language";
	import { languages } from "@codemirror/language-data";
	import { searchKeymap, openSearchPanel, closeSearchPanel, findNext, findPrevious } from "@codemirror/search";
	import { autocompletion, completionKeymap, CompletionContext } from "@codemirror/autocomplete";
	import { oneDark } from "@codemirror/theme-one-dark";
	import { openUrl } from "@tauri-apps/plugin-opener";
	import { writeText, readText, readImage } from "@tauri-apps/plugin-clipboard-manager";
	import { invoke } from "@tauri-apps/api/core";

	let {
		value = $bindable(),
		language = "markdown",
		onsave,
		onnew,
		onopen,
		onclose,
		onreveal,
		ontoggleEdit,
		ontoggleLive,
		ontoggleSplit,
		onhome,
		onnextTab,
		onprevTab,
		onundoClose,
		onscrollsync,
		zoomLevel = $bindable(100),
		theme = "system",
	} = $props<{
		value: string;
		language?: string;
		onsave?: () => void;
		onnew?: () => void;
		onopen?: () => void;
		onclose?: () => void;
		onreveal?: () => void;
		ontoggleEdit?: () => void;
		ontoggleLive?: () => void;
		ontoggleSplit?: () => void;
		onhome?: () => void;
		onnextTab?: () => void;
		onprevTab?: () => void;
		onundoClose?: () => void;
		onscrollsync?: (line: number, ratio?: number) => void;
		zoomLevel?: number;
		isSplit?: boolean;
		theme?: string;
	}>();

	let container: HTMLDivElement;
	let view: EditorView;
	let isApplyingExternalScroll = false;

	const settingsCompartment = new Compartment();
	const themeCompartment = new Compartment();
	const wrapCompartment = new Compartment();
	const lineNumbersCompartment = new Compartment();
	const activeLineCompartment = new Compartment();
	const whitespaceCompartment = new Compartment();
	const fontSizeCompartment = new Compartment();
	const fontCompartment = new Compartment();

	let cursorPosition = $state<{ line: number; col: number } | null>(null);
	let selectionCount = $state(0);
	let cursorCount = $state(0);
	let wordCount = $state(0);
	let currentLanguage = $state("markdown");
	let currentTabId = tabManager.activeTabId;
	let uiLanguage = $state(settings.language);

	const managedImages: {
		embed: string;
		filename: string;
		parentDir: string;
	}[] = $state([]);

	$effect(() => {
		uiLanguage = settings.language;
	});

	// ─── Theme helpers ──────────────────────────────────────────────

	function isDark(): boolean {
		if (theme === "system") {
			return window.matchMedia("(prefers-color-scheme: dark)").matches;
		}
		return theme === "dark";
	}

	function makeThemeExt(): import("@codemirror/state").Extension {
		const dark = isDark();
		const bg = dark ? "#181818" : "#FDFDFD";
		const gutterBg = dark ? "#1e1e1e" : "#f5f5f5";
		const gutterText = dark ? "#858585" : "#6e6e6e";
		const gutterActiveBg = dark ? "#2a2d2e" : "#e8e8e8";
		const activeLineBg = dark ? "#2a2d2e44" : "#e8e8e844";
		const cursorColor = dark ? "#aeafad" : "#333333";
		const selBg = dark ? "#264f78" : "#add6ff";
		const textColor = dark ? "#d4d4d4" : "#333333";
		const gutterBorder = dark ? "#333" : "#ddd";

		return EditorView.theme({
			"&": { backgroundColor: bg, color: textColor, height: "100%" },
			".cm-gutters": { backgroundColor: gutterBg, color: gutterText, border: "none", borderRight: `1px solid ${gutterBorder}` },
			".cm-activeLineGutter": { backgroundColor: gutterActiveBg },
			".cm-activeLine": { backgroundColor: activeLineBg },
			".cm-cursor": { borderLeftColor: cursorColor },
			".cm-selectionBackground": { backgroundColor: selBg },
			".cm-focused .cm-selectionBackground": { backgroundColor: selBg },
			".cm-matchingBracket": { backgroundColor: "#4b4b4b" },
			".cm-scroller": { fontFamily: "inherit" },
		});
	}

	// ─── Text helpers ───────────────────────────────────────────────

	function getSelectedText(): string {
		const sel = view.state.selection.main;
		return sel.empty ? '' : view.state.sliceDoc(sel.from, sel.to);
	}

	function replaceSelections(text: string) {
		const sel = view.state.selection;
		const changes = sel.ranges.map(range => ({
			from: range.from,
			to: range.to,
			insert: text,
		}));
		view.dispatch({ changes });
	}

	function posToLineCol(pos: number): { line: number; col: number } {
		const line = view.state.doc.lineAt(pos);
		return { line: line.number, col: pos - line.from + 1 };
	}

	// ─── Format toggling (bold/italic/underline) ────────────────────

	function toggleFormat(marker: string, type: "wrap" | "tag" = "wrap") {
		const sel = view.state.selection.main;
		if (sel.empty) return;
		const text = view.state.sliceDoc(sel.from, sel.to);

		if (type === "wrap") {
			if (text.startsWith(marker) && text.endsWith(marker)) {
				const newText = text.slice(marker.length, -marker.length);
				view.dispatch({
					changes: { from: sel.from, to: sel.to, insert: newText },
					selection: { anchor: sel.from, head: sel.from + newText.length },
				});
			} else {
				view.dispatch({
					changes: { from: sel.from, to: sel.to, insert: `${marker}${text}${marker}` },
					selection: { anchor: sel.from, head: sel.from + text.length + marker.length * 2 },
				});
			}
		} else if (type === "tag") {
			const [startTag, endTag] = marker.split("|");
			if (text.startsWith(startTag) && text.endsWith(endTag)) {
				const newText = text.slice(startTag.length, -endTag.length);
				view.dispatch({
					changes: { from: sel.from, to: sel.to, insert: newText },
					selection: { anchor: sel.from, head: sel.from + newText.length },
				});
			} else {
				view.dispatch({
					changes: { from: sel.from, to: sel.to, insert: `${startTag}${text}${endTag}` },
					selection: { anchor: sel.from, head: sel.from + text.length + startTag.length + endTag.length },
				});
			}
		}
	}

	// ─── Completion provider ────────────────────────────────────────

	const completionSource = async (context: CompletionContext) => {
		const match = context.matchBefore(/(!?\[.*\]\(|<img.*src=["']|src=["'])([^"')]*)$/i);
		if (!match) return null;
		const prefix = match.text;

		const isEmbed = /(!?\[.*\]\(|<img.*src=["']|src=["'])$/i.test(prefix);
		if (!isEmbed && !prefix.includes('/') && !prefix.includes('\\')) return null;

		const tab = tabManager.activeTab;
		if (!tab?.path) return null;

		const lastSlash = Math.max(
			tab.path.lastIndexOf("\\"),
			tab.path.lastIndexOf("/"),
		);
		const parentDir = tab.path.substring(0, lastSlash);
		const imgDirName = settings.imageDirectory || "img";

		try {
			const [currentEntries, imgEntries] = await Promise.all([
				invoke("list_directory_contents", { path: parentDir })
					.then((r) => r as string[]).catch(() => []),
				invoke("list_directory_contents", { path: `${parentDir}/${imgDirName}` })
					.then((r) => r as string[]).catch(() => []),
			]);

			const currentPath = match.text.match(/[^(\[<"']*$/)?.[0] || "";
			const options = [
				...currentEntries.filter(e => !currentPath || e.startsWith(currentPath)).map(e => ({
					label: e,
					type: "file" as const,
				})),
				...imgEntries.filter(e => !currentPath || e.startsWith(currentPath)).map(e => ({
					label: `${imgDirName}/${e}`,
					type: "file" as const,
				})),
			];

			return { from: match.from + match.text.length - currentPath.length, options };
		} catch {
			return null;
		}
	};

	// ─── Editor creation ────────────────────────────────────────────

	function createEditor() {
		const state = EditorState.create({
			doc: value,
			extensions: [
				history(),
				lineNumbers(),
				highlightActiveLine(),
				highlightActiveLineGutter(),
				drawSelection(),
				highlightSpecialChars(),
				scrollPastEnd(),
				keymap.of([
					...defaultKeymap,
					...historyKeymap,
					...searchKeymap,
					...completionKeymap,

					// ── Format ──
					{ key: "Mod-b", run: () => { toggleFormat("**"); return true; } },
					{ key: "Mod-i", run: () => { toggleFormat("*"); return true; } },
					{ key: "Mod-u", run: () => { toggleFormat("<u>|</u>", "tag"); return true; } },

					// ── Insert table ──
					{ key: "Mod-k t", run: () => {
						const sel = view.state.selection.main;
						const cols = 3, rows = 2;
						let table = "\n";
						table += "| " + Array(cols).fill("Header").join(" | ") + " |\n";
						table += "| " + Array(cols).fill("---").join(" | ") + " |\n";
						for (let i = 0; i < rows; i++) {
							table += "| " + Array(cols).fill("Cell").join(" | ") + " |\n";
						}
						table += "\n";
						view.dispatch({
							changes: { from: sel.from, to: sel.to, insert: table },
							selection: { anchor: sel.from + table.length },
						});
						return true;
					} },

					// ── File ops ──
					{ key: "Mod-n", run: () => { onnew?.(); return true; } },
					{ key: "Mod-t", run: () => { onnew?.(); return true; } },
					{ key: "Mod-o", run: () => { onopen?.(); return true; } },
					{ key: "Mod-s", run: () => { onsave?.(); return true; } },
					{ key: "Mod-w", run: () => { onclose?.(); return true; } },
					{ key: "Mod-Shift-r", run: () => { onreveal?.(); return true; } },

					// ── View toggles ──
					{ key: "Mod-e", run: () => { ontoggleEdit?.(); return true; } },
					{ key: "Mod-l", run: () => { ontoggleLive?.(); return true; } },
					{ key: "Mod-\\", run: () => { ontoggleSplit?.(); return true; } },

					// ── Tab nav ──
					{ key: "Mod-Tab", run: () => { onnextTab?.(); return true; } },
					{ key: "Mod-Shift-Tab", run: () => { onprevTab?.(); return true; } },
					{ key: "Mod-Shift-t", run: () => { onundoClose?.(); return true; } },

					// ── Find ──
					{ key: "Mod-f", run: () => { openSearchPanel(view); return true; } },
					{ key: "Mod-g", run: () => { findNext(view); return true; } },
					{ key: "Mod-Shift-g", run: () => { findPrevious(view); return true; } },
				]),

				markdown({ base: markdownLanguage, codeLanguages: languages }),
				syntaxHighlighting(defaultHighlightStyle),
				autocompletion({ override: [completionSource] }),
				placeholder(t('editor.placeholder', uiLanguage) || 'Start typing...'),

				// ── Update listener (content changes, cursor, selection) ──
				EditorView.updateListener.of((update) => {
					if (update.docChanged) {
						const newValue = update.state.doc.toString();
						if (value !== newValue) {
							value = newValue;
							if (tabManager.activeTabId) {
								tabManager.updateTabRawContent(tabManager.activeTabId, newValue);
							}
						}
						wordCount = (newValue.match(/\S+/g) || []).filter((w) => /\w/.test(w)).length;
					}

					if (update.selectionSet || update.docChanged) {
						const sel = update.state.selection;
						cursorCount = sel.ranges.length;
						if (sel.main.empty) {
							selectionCount = 0;
						} else {
							selectionCount = sel.ranges.reduce((acc, range) => {
								return acc + (range.to - range.from);
							}, 0);
						}
						const head = sel.main.head;
						const line = update.state.doc.lineAt(head);
						cursorPosition = { line: line.number, col: head - line.from + 1 };
					}

					if (update.focusChanged || update.docChanged || update.selectionSet) {
						emitScrollSync();
					}
				}),

				// ── Compartments ──
				settingsCompartment.of([]),
				themeCompartment.of(makeThemeExt()),
				wrapCompartment.of([]),
				lineNumbersCompartment.of([]),
				activeLineCompartment.of([]),
				whitespaceCompartment.of([]),
				fontSizeCompartment.of([]),
				fontCompartment.of([]),

				// Dark theme as base
				EditorView.theme({
					"&": { height: "100%" },
					".cm-scroller": { fontFamily: "inherit" },
				}),
			],
		});

		view = new EditorView({
			state,
			parent: container,
		});

		// Apply initial settings
		applySettings();
		applyTheme();
	}

	// ─── Settings application ───────────────────────────────────────

	function applySettings() {
		if (!view) return;

		// Font size
		const fontSize = settings.editorFontSize * (zoomLevel / 100);
		fontSizeCompartment.reconfigure([
			EditorView.theme({ "&": { fontSize: `${fontSize}px` } }),
		]);

		// Font family
		fontCompartment.reconfigure([
			EditorView.theme({ ".cm-scroller": { fontFamily: settings.editorFont } }),
		]);

		// Line wrapping
		const wrapExt = settings.wordWrap !== "off"
			? EditorView.lineWrapping
			: [];
		wrapCompartment.reconfigure(wrapExt);

		// Line numbers
		const lnExt = settings.lineNumbers !== "off"
			? lineNumbers()
			: [];
		lineNumbersCompartment.reconfigure(lnExt);

		// Active line highlight
		const alExt = settings.renderLineHighlight
			? [highlightActiveLine(), highlightActiveLineGutter()]
			: [];
		activeLineCompartment.reconfigure(alExt);

		// Whitespace
		const wsExt = settings.showWhitespace
			? highlightSpecialChars()
			: [];
		whitespaceCompartment.reconfigure(wsExt);

		// Settings that don't have CM6 equivalents:
		// - minimap: not supported by CM6
		// - occurrencesHighlight: not supported
	}

	function applyTheme() {
		if (!view) return;
		themeCompartment.reconfigure(makeThemeExt());
	}

	// ─── Scroll sync ────────────────────────────────────────────────

	function emitScrollSync() {
		if (!view || isApplyingExternalScroll) return;
		if (!onscrollsync) return;

		const pos = view.state.selection.main.head;
		const line = view.state.doc.lineAt(pos);
		const scrollTop = view.scrollDOM.scrollTop;
		const height = view.scrollDOM.clientHeight;
		const linePos = view.coordsAtPos(pos);
		if (!linePos) return;
		const editorRect = view.dom.getBoundingClientRect();
		const ratio = (linePos.top - editorRect.top) / height;
		onscrollsync(line.number, ratio);
	}

	// ─── Init ───────────────────────────────────────────────────────

	onMount(() => {
		// Override window.open for external URLs
		const originalOpen = window.open;
		window.open = function (url?: string | URL, target?: string, features?: string) {
			if (typeof url === "string" && (url.startsWith("http://") || url.startsWith("https://"))) {
				openUrl(url);
				return null;
			}
			return originalOpen.apply(this, arguments as any);
		};

		createEditor();

		// Restore view state
		const tab = tabManager.activeTab;
		if (tab) {
			if (tab.editorViewState) {
				const vs = tab.editorViewState as any;
				if (vs.scrollTop != null) {
					requestAnimationFrame(() => {
						view.scrollDOM.scrollTop = vs.scrollTop;
					});
				}
				if (vs.cursorLine && vs.cursorCol) {
					const line = view.state.doc.line(vs.cursorLine);
					const pos = line.from + Math.min(vs.cursorCol - 1, line.length);
					view.dispatch({
						selection: { anchor: pos },
						scrollIntoView: true,
					});
				}
			} else if (tab.anchorLine > 0 || tab.scrollPercentage > 0) {
				requestAnimationFrame(() => {
					if (tab.anchorLine > 0) {
						const line = view.state.doc.line(Math.max(1, tab.anchorLine - 2));
						view.dispatch({
							selection: { anchor: line.from },
							scrollIntoView: true,
						});
					} else if (tab.scrollPercentage > 0) {
						const docHeight = view.state.doc.length;
						const targetPos = Math.floor(docHeight * tab.scrollPercentage);
						const line = view.state.doc.lineAt(targetPos);
						view.dispatch({
							selection: { anchor: line.from },
							scrollIntoView: true,
						});
					}
				});
			} else {
				view.focus();
			}
		} else {
			view.focus();
		}

		// Override navigator.clipboard.readText for cross-app clipboard
		try {
			if (navigator.clipboard && typeof navigator.clipboard.readText === 'function') {
				Object.defineProperty(navigator.clipboard, 'readText', {
					value: async () => {
						try {
							const text = await readText();
							return text ?? '';
						} catch (e) {
							console.error('Tauri clipboard readText error:', e);
							return '';
						}
					},
					writable: true,
					configurable: true,
				});
			}
		} catch (e) {
			console.warn('Could not override navigator.clipboard.readText:', e);
		}

		// Intercept clipboard events on the editor DOM
		const shouldIntercept = () => {
			if (!view) return false;
			return view.hasFocus || (container && container.contains(document.activeElement));
		};

		const onCopy = async (e: ClipboardEvent) => {
			if (!shouldIntercept()) return;
			e.preventDefault();
			e.stopPropagation();
			await handleCopy();
		};
		const onCut = async (e: ClipboardEvent) => {
			if (!shouldIntercept()) return;
			e.preventDefault();
			e.stopPropagation();
			await handleCut();
		};
		const onPaste = async (e: ClipboardEvent) => {
			if (!shouldIntercept()) return;
			e.preventDefault();
			e.stopPropagation();
			await handlePaste();
		};

		view.dom.addEventListener('copy', onCopy, true);
		view.dom.addEventListener('cut', onCut, true);
		view.dom.addEventListener('paste', onPaste, true);
		window.addEventListener('paste', onPaste, true);

		// Zoom via scroll
		const wheelListener = (e: WheelEvent) => {
			if (e.ctrlKey || e.metaKey) {
				e.preventDefault();
				e.stopPropagation();
				if (e.deltaY < 0) {
					zoomLevel = Math.min(zoomLevel + 10, 500);
				} else {
					zoomLevel = Math.max(zoomLevel - 10, 25);
				}
			}
		};
		container.addEventListener("wheel", wheelListener, { capture: true });

		// Scroll sync
		const onScroll = () => {
			emitScrollSync();
		};
		view.scrollDOM.addEventListener('scroll', onScroll);

		// Undo image clean-up (simplified — CM6 doesn't expose undo events directly)
		// We track image insertions via managedImages and detect undo via updateListener

		// Theme listener
		const mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
		mediaQuery.addEventListener("change", applyTheme);

		// Cleanup
		return () => {
			window.open = originalOpen;
			mediaQuery.removeEventListener("change", applyTheme);
			container.removeEventListener("wheel", wheelListener, { capture: true });

			if (view && currentTabId) {
				// Save view state
				const vs: any = {};
				vs.scrollTop = view.scrollDOM.scrollTop;
				const head = view.state.selection.main.head;
				const line = view.state.doc.lineAt(head);
				vs.cursorLine = line.number;
				vs.cursorCol = head - line.from + 1;
				tabManager.updateTabEditorState(currentTabId, vs);

				const scrollHeight = view.scrollDOM.scrollHeight;
				const clientHeight = view.scrollDOM.clientHeight;
				if (scrollHeight > clientHeight) {
					const percentage = view.scrollDOM.scrollTop / (scrollHeight - clientHeight);
					tabManager.updateTabScrollPercentage(currentTabId, percentage);
				}

				const scrollTop = view.scrollDOM.scrollTop;
				tabManager.updateTabAnchorLine(currentTabId, Math.round(scrollTop / 20) + 3);
			}

			view.dom.removeEventListener('copy', onCopy, true);
			view.dom.removeEventListener('cut', onCut, true);
			view.dom.removeEventListener('paste', onPaste, true);
			window.removeEventListener('paste', onPaste, true);
			view.scrollDOM.removeEventListener('scroll', onScroll);

			view.destroy();
		};
	});

	// ─── Tab switching ──────────────────────────────────────────────

	$effect(() => {
		const activeTabId = tabManager.activeTabId;
		const content = value;

		if (!view) return;

		if (activeTabId !== currentTabId) {
			if (currentTabId) {
				const vs: any = {};
				vs.scrollTop = view.scrollDOM.scrollTop;
				const head = view.state.selection.main.head;
				const line = view.state.doc.lineAt(head);
				vs.cursorLine = line.number;
				vs.cursorCol = head - line.from + 1;
				tabManager.updateTabEditorState(currentTabId, vs);
			}

			currentTabId = activeTabId;

			if (view.state.doc.toString() !== content) {
				view.dispatch({
					changes: { from: 0, to: view.state.doc.length, insert: content },
				});
			}

			const tab = tabManager.activeTab;
			if (tab?.editorViewState) {
				const vs = tab.editorViewState as any;
				requestAnimationFrame(() => {
					if (vs.scrollTop != null) view.scrollDOM.scrollTop = vs.scrollTop;
					if (vs.cursorLine && vs.cursorCol) {
						const l = view.state.doc.line(vs.cursorLine);
						const pos = l.from + Math.min(vs.cursorCol - 1, l.length);
						view.dispatch({
							selection: { anchor: pos },
							scrollIntoView: true,
						});
					}
				});
			} else {
				view.scrollDOM.scrollTop = 0;
				view.dispatch({ selection: { anchor: 0 } });
			}
		} else {
			if (view.state.doc.toString() !== content) {
				view.dispatch({
					changes: { from: 0, to: view.state.doc.length, insert: content },
				});
			}
		}
	});

	// ─── Settings effect ────────────────────────────────────────────

	$effect(() => {
		if (view) {
			// This effect re-runs when any of these settings change
			void settings.minimap;
			void settings.wordWrap;
			void settings.lineNumbers;
			void settings.renderLineHighlight;
			void settings.occurrencesHighlight;
			void settings.editorFontSize;
			void settings.editorFont;
			void settings.showWhitespace;
			applySettings();
		}
	});

	$effect(() => {
		if (view && theme) {
			applyTheme();
		}
	});

	// ─── Exported API ───────────────────────────────────────────────

	export async function handleCopy(): Promise<void> {
		if (!view) return;
		const text = getSelectedText();
		if (!text) return;
		await writeText(text);
	}

	export async function handleCut(): Promise<void> {
		if (!view) return;
		const text = getSelectedText();
		if (!text) return;
		await writeText(text);
		const sel = view.state.selection.main;
		view.dispatch({
			changes: { from: sel.from, to: sel.to, insert: '' },
		});
	}

	export async function handlePaste(): Promise<void> {
		if (!view) return;
		try {
			// Try image paste first
			const img = await readImage().catch(() => null);
			if (img && tabManager.activeTab?.path) {
				const rgba = await img.rgba();
				const size = await img.size();
				const scale = settings.macosImageScaling ? 0.5 : 1.0;
				const base64Image = await invoke<string>("encode_png", {
					rgba: Array.from(rgba),
					width: size.width,
					height: size.height,
					scale,
				});
				const ext = "png";
				const filename = `paste_${Date.now()}.${ext}`;
				const tabPath = tabManager.activeTab.path;
				const dirMatch = tabPath.match(/^(.*)[/\\][^/\\]+$/);
				if (dirMatch) {
					const parentDir = dirMatch[1];
					const imgDirName = settings.imageDirectory || "img";
					const relPath = (await invoke("save_image", {
						parentDir,
						filename,
						base64Data: base64Image,
						imageDirectory: imgDirName,
					})) as string;
					const escapedPath = relPath.replace(/ /g, "%20").replace(/^\//, "");
					const embed = `![alt](${escapedPath})`;

					const sel = view.state.selection.main;
					const pos = sel.head;
					view.dispatch({
						changes: { from: pos, to: pos, insert: embed },
						selection: { anchor: pos + embed.length },
					});
					return;
				}
			}

			// Text paste
			let rawText = '';
			try {
				rawText = await navigator.clipboard.readText();
				if (!rawText) rawText = await readText() ?? '';
			} catch {
				rawText = await readText() ?? '';
			}
			if (!rawText) return;

			const text = rawText.trim();
			const urlRegex = /^(?:(?:https?|file|tauri):\/\/|www\.)[^\s]{2,}$/i;
			const isUrl = urlRegex.test(text);

			const sel = view.state.selection;

			if (!isUrl) {
				replaceSelections(rawText);
				return;
			}

			// URL paste — wrap selection or insert as link
			if (!sel.main.empty) {
				const selectedText = view.state.sliceDoc(sel.main.from, sel.main.to);
				const linkUrl = text.toLowerCase().startsWith("www.") ? `http://${text}` : text;
				view.dispatch({
					changes: { from: sel.main.from, to: sel.main.to, insert: `[${selectedText}](${linkUrl})` },
					selection: { anchor: sel.main.from + 1, head: sel.main.from + 1 + selectedText.length },
				});
			} else {
				const displayText = text.replace(/^(?:https?|file|tauri):\/\/|www\./i, "");
				const linkUrl = text.toLowerCase().startsWith("www.") ? `http://${text}` : text;
				const template = `[${displayText}](${linkUrl})`;
				view.dispatch({
					changes: { from: sel.main.from, to: sel.main.to, insert: template },
					selection: { anchor: sel.main.from + 1, head: sel.main.from + 1 + displayText.length },
				});
			}
		} catch (err) {
			console.error("Paste failed:", err);
		}
	}

	export function handleSelectAll(): void {
		if (!view) return;
		view.dispatch({
			selection: { anchor: 0, head: view.state.doc.length },
		});
	}

	export function syncScrollToLine(line: number, ratio: number = 0) {
		if (!view) return;
		const doc = view.state.doc;
		const safeLine = Math.max(1, Math.min(doc.lines, line));
		const lineObj = doc.line(safeLine);
		const coords = view.coordsAtPos(lineObj.from);
		if (!coords) return;
		const editorRect = view.dom.getBoundingClientRect();
		const targetScroll = view.scrollDOM.scrollTop + (coords.top - editorRect.top) - editorRect.height * ratio;

		if (Math.abs(view.scrollDOM.scrollTop - targetScroll) <= 5) return;
		isApplyingExternalScroll = true;
		view.scrollDOM.scrollTo({ top: targetScroll, behavior: 'smooth' });
		requestAnimationFrame(() => {
			isApplyingExternalScroll = false;
		});
	}

	export async function handleDroppedFile(path: string, x: number, y: number) {
		if (!view || !tabManager.activeTab?.path) return;
		const pos = view.posAtCoords({ x, y });
		if (pos == null) return;

		const tabPath = tabManager.activeTab.path;
		const match = tabPath.match(/^(.*)[/\\][^/\\]+$/);
		if (!match) return;
		const parentDir = match[1];

		try {
			const imgDirName = settings.imageDirectory || "img";
			const relPath = (await invoke("copy_file_to_img", {
				srcPath: path,
				parentDir,
				imageDirectory: imgDirName,
			})) as string;
			const escapedPath = relPath.replace(/ /g, "%20").replace(/^\//, "");
			const embed = `![alt](${escapedPath})`;

			view.dispatch({
				changes: { from: pos, to: pos, insert: embed },
				selection: { anchor: pos + embed.length },
			});

			const actualFilename = path.split(/[/\\]/).pop() || "image";
			managedImages.push({ embed, filename: actualFilename, parentDir });
		} catch (err) {
			console.error("Failed to copy dropped file:", err);
		}
	}

	let dragCaretDecoration: string[] = [];
	export function updateDragCaret(x: number, y: number) {
		if (!view) return;
		const pos = view.posAtCoords({ x, y });
		if (pos == null) {
			hideDragCaret();
			return;
		}
		// CM6 doesn't have decorations API like Monaco.
		// Add a transient class to indicate caret position.
		view.dom.classList.add('drag-caret');
	}

	export function hideDragCaret() {
		if (!view) return;
		view.dom.classList.remove('drag-caret');
	}

	export function revealHeader(text: string) {
		if (!view) return;
		const doc = view.state.doc;
		const escapedText = text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
		const regex = new RegExp(`^#+\\s+.*${escapedText}.*$`, "m");
		const fullText = doc.toString();
		const match = regex.exec(fullText);
		if (match) {
			const startPos = match.index;
			const line = doc.lineAt(startPos);
			view.dispatch({
				selection: { anchor: line.from },
				scrollIntoView: true,
			});
			view.focus();
		} else {
			const fallbackIdx = fullText.toLowerCase().indexOf(escapedText.toLowerCase());
			if (fallbackIdx >= 0) {
				const line = doc.lineAt(fallbackIdx);
				view.dispatch({
					selection: { anchor: line.from },
					scrollIntoView: true,
				});
				view.focus();
			}
		}
	}

	export const undo = () => {
		view?.focus();
		cmUndo(view);
	}

	export const redo = () => {
		view?.focus();
		cmRedo(view);
	}

	export const triggerFind = () => {
		if (!view) return;
		view.focus();
		openSearchPanel(view);
	}

	export const getValue = () => view?.state.doc.toString() || "";
	export const setValue = (val: string) => {
		if (view) {
			view.dispatch({
				changes: { from: 0, to: view.state.doc.length, insert: val },
			});
		}
	};
	export const focus = () => view?.focus();
	export const getViewState = () => {
		if (!view) return null;
		return {
			scrollTop: view.scrollDOM.scrollTop,
			cursorLine: view.state.selection.main.head,
		};
	};
	export const restoreViewState = (state: any) => {
		if (!view || !state) return;
		if (state.scrollTop != null) {
			requestAnimationFrame(() => {
				view.scrollDOM.scrollTop = state.scrollTop;
			});
		}
		if (state.cursorLine != null) {
			const pos = state.cursorLine;
			const safePos = Math.min(pos, view.state.doc.length);
			view.dispatch({ selection: { anchor: safePos }, scrollIntoView: true });
		}
	};
	export const revealLine = (line: number) => {
		if (!view) return;
		const safeLine = Math.max(1, Math.min(view.state.doc.lines, line));
		const lineObj = view.state.doc.line(safeLine);
		view.dispatch({
			selection: { anchor: lineObj.from },
			scrollIntoView: true,
		});
	};

	export const hasSelection = () => {
		return view ? !view.state.selection.main.empty : false;
	}

	export const transformSelection = (type: 'lowercase' | 'uppercase' | 'propercase') => {
		if (!view) return;
		const sel = view.state.selection;
		if (sel.main.empty) return;

		const changes = sel.ranges.map(range => {
			const text = view.state.sliceDoc(range.from, range.to);
			let newText = text;
			if (type === 'lowercase') newText = text.toLowerCase();
			else if (type === 'uppercase') newText = text.toUpperCase();
			else if (type === 'propercase') newText = text.toLowerCase().replace(/\b\w/g, (c) => c.toUpperCase());
			return { from: range.from, to: range.to, insert: newText };
		});

		view.dispatch({ changes });
	}

	onDestroy(() => {
		// Cleanup is handled in onMount's return function
	});
</script>

<div class="editor-outer">
	<div
		class="editor-container"
		bind:this={container}
	></div>
</div>

{#if settings.statusBar}
	<div class="status-bar">
		<div class="status-item">
			{t('editor.status.lineCol', settings.language).replace('{{line}}', (cursorPosition?.line ?? 1).toString()).replace('{{col}}', (cursorPosition?.col ?? 1).toString())}
		</div>
		{#if selectionCount > 0}
			<div class="status-item">
				{t('editor.status.selected', settings.language).replace('{{count}}', selectionCount.toString())}
			</div>
		{:else if cursorCount > 1}
			<div class="status-item">
				{t('editor.status.selections', settings.language).replace('{{count}}', cursorCount.toString())}
			</div>
		{/if}
		{#if settings.wordCount}
			<div class="status-item">
				{t('editor.status.words', settings.language).replace('{{count}}', wordCount.toString())}
			</div>
		{/if}
		<div class="status-item">
			{zoomLevel}%
		</div>
		<div class="status-item">
			{currentLanguage}
		</div>
		<div class="status-item">{t('editor.status.crlf')}</div>
		<div class="status-item">{t('editor.status.utf8')}</div>
	</div>
{/if}

<style>
	.editor-outer {
		flex: 1;
		height: 100%;
		width: 100%;
		display: flex;
		background-color: var(--color-canvas-default);
		overflow: hidden;
	}

	.editor-container {
		height: 100%;
		width: 100%;
		min-width: 0;
	}

	.editor-container :global(.cm-editor) {
		height: 100%;
	}

	.editor-container :global(.cm-scroller) {
		font-family: inherit;
		overflow: auto;
	}

	:global(.drag-caret) {
		border-left: 2px solid var(--color-accent-fg);
		outline: none;
	}

	.status-bar {
		padding: 0 10px;
		font-family: "Segoe UI", Tahoma, Geneva, Verdana, sans-serif;
		font-size: var(--ui-font-size, 12px);
		background: var(--bg-tertiary);
		border-top: 1px solid var(--color-border-muted);
		color: var(--text-primary);
		display: flex;
		align-items: center;
		justify-content: flex-end;
		min-height: 22px;
		gap: 20px;
		user-select: none;
	}

	.status-item {
		opacity: 0.8;
	}
</style>
