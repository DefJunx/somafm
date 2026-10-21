# 01 Settings popover and settings schema v2

Status: ready-for-agent
Blocked by:

## Goal
Add the three user toggles to `settings.json` (schema v2) and a Settings popover in the Grid header. Shared by auto-update and audio-fx.

## Spec sections
- [Decisions](../spec.md#decisions-agreed-do-not-reopen) (Settings (shared))

## Acceptance criteria
- [ ] `SCHEMA_VERSION` becomes 2 with the first MIGRATIONS entry (v1 -> v2) adding `checkForUpdates` (true), `fadeAudio` (true) and `showVisualizer` (true); a migration test loads a v1 file and gets those defaults with other keys intact. Lenient per-key fallback and corrupt -> `settings.json.bak` still hold.
- [ ] When creating a fresh v2 file, `showVisualizer` defaults to false if the OS prefers reduced motion; the frontend passes that preference to Rust once (command or init argument) and it is only applied on first creation/migration, never overriding a saved value.
- [ ] `set_check_for_updates`, `set_fade_audio`, `set_show_visualizer` commands; all three keys appear in `SettingsView` and `settings-changed`; reducer-style Rust tests cover each setter.
- [ ] `src/lib/settings.svelte.ts` exposes the three values; a gear button in the Grid header opens a small popover with three labelled toggles (keyboard accessible, Escape closes, focus returns to the gear).
- [ ] Vitest with `mockIPC` covers toggling each setting and the popover open/close.
- [ ] `pnpm check` passes.

## Notes
- Nothing consumes the toggles yet; tickets 03 here and the audio-fx tickets read them.
