# 03 Update check: plugin, schedule and state (Rust)

Status: ready-for-agent
Blocked by: 01

## Goal
The app checks for a newer version on launch and every 24 h and exposes the result to the frontend.

## Spec sections
- [Decisions](../spec.md#decisions-agreed-do-not-reopen) (Updater, Prompt, Manifest)
- [ADR 0002](../../../docs/adr/0002-auto-update.md)

## Acceptance criteria
- [ ] `tauri-plugin-updater` (and `tauri-plugin-process` if needed for restart) added, with capabilities and the endpoint `https://github.com/DefJunx/somafm/releases/latest/download/latest.json`; the endpoint is overridable for tests. The public key placeholder is clearly marked until ticket 02 completes.
- [ ] An `update` module with a pure state reducer (`Idle`, `Available { version, notes, date }`, `Snoozed`) and scheduling rules, unit-tested: runs on launch and every 24 h only when `checkForUpdates` is on; turning the setting off stops checks; "Later" snoozes until the next launch; network errors are silent.
- [ ] Emits an `update-available` event and has a `get_update_state` command; commands `install_update` (download, install, then restart only after the user asked) and `snooze_update`.
- [ ] Integration test against a stub manifest server: newer version -> `Available` with notes; same version -> `Idle`; invalid manifest -> `Idle`, no crash.
- [ ] Playback is never stopped by checking or downloading; only the restart in `install_update` ends it.
- [ ] `pnpm check` passes.
