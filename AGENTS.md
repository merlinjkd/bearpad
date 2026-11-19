## Agent skills

### Issue tracker

Issues live in GitHub Issues for merlinjkd/bearpad. See `docs/agents/issue-tracker.md`.

### Triage labels

Five canonical roles, labels equal their names: needs-triage, needs-info, ready-for-agent, ready-for-human, wontfix. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: root `CONTEXT.md` + `docs/adr/`. See `docs/agents/domain.md`.

## Releases

Versions are bumped by hand, in four places that must agree: `package.json`,
`src-tauri/Cargo.toml`, `src-tauri/Cargo.lock` (the `bearpad` package entry), and
`src-tauri/tauri.conf.json`. CI creates the tag and the draft release on a push to
`main`; tags are never created by hand.

The next release from this tree is **0.8.0** — a deliberate jump from 0.4.0, chosen
to leave room before 1.0. Jim plans to reach 1.0 soon after.
