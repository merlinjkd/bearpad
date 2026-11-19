import { HighlightStyle } from "@codemirror/language";
import { tags } from "@lezer/highlight";

/**
 * Markdown highlighting for the editor, deliberately without colours.
 *
 * The editor used the stock `defaultHighlightStyle`, whose own doc comment says
 * "works well with light themes". It paints every markup character with its
 * `tags.meta` colour, #404740: the `#`, `-`, `*`, backtick and bracket of markdown
 * syntax, and the `^` of a superscript and `~` of a strikethrough, which are what
 * were reported. On the dark editor background (#1E1E1E) that is 1.74:1 contrast,
 * well under the 4.5:1 minimum: those characters were cut out of the text while
 * typing. Light mode was unaffected at 7.76:1, which is why only dark mode was
 * reported. The same palette paints `contentSeparator` #219, also unreadable
 * there, so the rule below claims neither.
 *
 * Structure is carried by weight, slant and underline instead, so every character
 * keeps the theme's text colour. Links are the deliberate exception and get their
 * cyan from the `bp-link` decoration in Editor.svelte, not from here.
 */
export const markdownHighlight = HighlightStyle.define([
	{ tag: tags.heading, fontWeight: "bold", textDecoration: "underline" },
	{ tag: tags.strong, fontWeight: "bold" },
	{ tag: tags.emphasis, fontStyle: "italic" },
	{ tag: tags.strikethrough, textDecoration: "line-through" },
	{ tag: tags.link, textDecoration: "underline" },
]);
