# BearPad

A fast, local-first markdown/text editor for macOS, Windows, and Linux. Built with Tauri 2, Svelte 5, and CodeMirror 6.

## Why I built this

I am a Web Designer / Developer. I constantly have to Cut and Paste plain text back and forth. I also need to transform text from lowercase to UPPERCASE and Title Case frequently, as well as do find and replace. I use a lot of editors for various reasons, e.g. code, etc., but I just wanted a small, fast, cross platform text editor that handles these key tasks without unnecessary extras.

## Features

- Multi-tab editing with per-tab dirty tracking
- English spellcheck (Hunspell) with squiggly underlines; skips code/URLs; add-to-dictionary via right-click (persisted to `custom_words.txt`)
- Case transforms: lowercase / UPPERCASE / Title Case (apostrophe-aware) from the Text menu or right-click, selection-gated
- Find & Replace with Find All / Replace All from the Edit menu (Cmd/Ctrl-F)
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
