# 06 Release 0.2.0

Status: resolved
Blocked by: 01, 02, 03, 04, 05

## Goal
Ship Track history and Liked tracks as 0.2.0.

## Acceptance criteria
- [x] Version bumped to 0.2.0 in `package.json`, `src-tauri/tauri.conf.json` and `src-tauri/Cargo.toml`; `Cargo.lock` refreshed.
- [x] `CHANGELOG.md` updated per `AGENTS.md` (Unreleased renamed, fresh empty Unreleased, compare links, user-facing entries from `git log`).
- [x] `pnpm check` passes. Tagging `v0.2.0` is left to the user.
