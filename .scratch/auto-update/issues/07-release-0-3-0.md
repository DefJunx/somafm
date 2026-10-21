# 07 Release 0.3.0

Status: ready-for-agent
Blocked by: 01, 02, 03, 04, 05, 06

## Goal
Ship auto-update as 0.3.0. This is the first release that can update itself, so installs older than 0.3.0 must still be updated by hand.

## Acceptance criteria
- [ ] Version bumped to 0.3.0 in `package.json`, `src-tauri/tauri.conf.json` and `src-tauri/Cargo.toml`; `Cargo.lock` refreshed.
- [ ] `CHANGELOG.md` updated per `AGENTS.md`, noting that older installs need a manual update once.
- [ ] `pnpm check` passes. Tagging `v0.3.0` and verifying `latest.json` on the release is left to the user.
