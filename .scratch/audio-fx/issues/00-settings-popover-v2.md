# 00 Settings popover and settings schema v2

Status: resolved
Blocked by:

## Goal
Add the "Fade audio" and "Show visualizer" toggles to `settings.json` (schema v2) and a Settings popover in the Grid header.

## Spec sections
- [Decisions](../spec.md#decisions-agreed-do-not-reopen) (Fade audio, Show visualizer)

## Acceptance criteria
- [x] `SCHEMA_VERSION` becomes 2 with the first MIGRATIONS entry (v1 -> v2) adding `fadeAudio` (true) and `showVisualizer` (true); a migration test loads a v1 file and gets those defaults with other keys intact. Lenient per-key fallback and corrupt -> `settings.json.bak` still hold.
- [x] When creating a fresh v2 file, `showVisualizer` defaults to false if the OS prefers reduced motion; the frontend passes that preference to Rust once (command or init argument) and it is only applied on first creation/migration, never overriding a saved value.
- [x] `set_fade_audio` and `set_show_visualizer` commands; both keys appear in `SettingsView` and `settings-changed`; reducer-style Rust tests cover each setter.
- [x] `src/lib/settings.svelte.ts` exposes both values; a gear button in the Grid header opens a small popover with two labelled toggles (keyboard accessible, Escape closes, focus returns to the gear).
- [x] Vitest with `mockIPC` covers toggling each setting and the popover open/close.
- [x] `pnpm check` passes.

## Notes
- Nothing consumes the toggles yet; tickets 01, 04 and 05 read them.
