# Bearpad

A fast, local-first markdown/text editor for macOS, Windows, and Linux. Built with Tauri 2, Svelte 5, and CodeMirror 6.

## Features

- Multi-tab editing with per-tab dirty tracking
- English spellcheck (Hunspell) with squiggly underlines; skips code/URLs; add-to-dictionary via right-click (persisted to `custom_words.txt`)
- Case transforms: lowercase / UPPERCASE / Title Case (apostrophe-aware) from the Edit menu or right-click, selection-gated
- Native cut/copy/paste (clipboard-manager plugin first, fallback to execCommand)
- Native open/save/close dialogs, unsaved-changes guard on close
- Themes: dark, light, github-dark (follows system by default)
- Font size and family settings, word wrap toggle
- In-window HTML menu bar (18px, platform-aware Ctrl/⌘ shortcuts)

## Development

Prereqs: Node 20+, Rust stable, [Tauri 2 prerequisites](https://tauri.app/start/prerequisites/).

```sh
npm install
npm run tauri dev
```

Tests:

```sh
npm test          # TS unit tests (node --experimental-strip-types)
cargo test        # Rust (spellcheck) tests — run from src-tauri/
```

## Release builds

```sh
npm run tauri build
```

CI (`.github/workflows/build.yml`) builds macOS + Windows installers and a Linux `.deb` on tag pushes.

## Project conventions

See `AGENTS.md` — issues are tracked in GitHub Issues (`merlinjkd/bearpad`), triage labels are defined in `docs/agents/`.
