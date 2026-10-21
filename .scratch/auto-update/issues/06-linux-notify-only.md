# 06 Linux notify-only updates

Status: ready-for-agent
Blocked by: 03, 04

## Goal
On Linux the Update banner announces new versions and links to the release page instead of installing.

## Spec sections
- [Decisions](../spec.md#decisions-agreed-do-not-reopen) (Linux)
- [ADR 0002](../../../docs/adr/0002-auto-update.md)

## Acceptance criteria
- [ ] On Linux the check reads the same manifest (or the release's version) but never calls the updater's install; `get_update_state` reports `canInstall: false` and a release page URL.
- [ ] The banner shows the version and notes with "Open release page" (system browser) and "Later" instead of "Restart to update"; the tray item opens the release page.
- [ ] Platform branching is a pure function, unit-tested for linux / macos / windows.
- [ ] Vitest covers the Linux banner variant. `pnpm check` passes.
