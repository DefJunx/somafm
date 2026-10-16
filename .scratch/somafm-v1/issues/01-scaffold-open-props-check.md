# 01 Scaffold + Open Props + check script

Status: ready-for-agent
Blocked by: none

## Goal
Create the Tauri 2 app skeleton from the create-tauri-app Svelte-TS template with Open Props and a single `check` command.

## Spec sections
- [§1 Purpose & scope](../spec.md#1-purpose--scope)
- [§2 Stack & project layout](../spec.md#2-stack--project-layout)
- [§9 Testing & DoD](../spec.md#9-testing--the-check-command) (check command, Vitest/mockIPC setup)

## Acceptance criteria
- [ ] Project scaffolded with the create-tauri-app Svelte + TypeScript template (plain Vite, pnpm), bundle identifier `dev.defjunx.somafm`, Rust module skeleton (`feed`, `player`, `media_controls`, `tray`, `settings`, `commands`) and `src/lib/api.ts` with the TS types from spec §3.
- [ ] Open Props installed and its tokens imported globally; a placeholder `Grid` and `Player` view exist and render.
- [ ] `pnpm check` runs `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`, `svelte-check`, `vitest run` in order and passes clean; one trivial passing test each in Rust and Vitest (with `mockIPC` wired) proves the harness.
- [ ] Original generic radio/waveform SVG icon committed and expanded with `pnpm tauri icon`, plus a monochrome tray icon; no SomaFM artwork or branding anywhere.
- [ ] `pnpm tauri dev` opens a window on Linux.

## Smoke checklist
None (no S-IDs owned). Shared Definition of Done in [§9 Testing & DoD](../spec.md#9-testing--the-check-command) applies.

## Notes
- Run scaffolding inside the Fedora 44 toolbox (spec §10). Do not add dependencies beyond what later slices need; add those in the slice that uses them.
