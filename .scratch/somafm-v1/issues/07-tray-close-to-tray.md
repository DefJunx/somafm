# 07 Tray & close-to-tray

Status: resolved
Blocked by: 04, 06

## Goal
Add the system tray menu and make closing the window hide it while playback continues.

## Spec sections
- [§7 Tray, media keys](../spec.md#7-tray-media-keys-os-overlay-close-to-tray) (close-to-tray, tray menu, per-OS)
- [§3 Architecture](../spec.md#3-architecture-rust-vs-frontend) (commands reused; no new ones)

## Acceptance criteria
- [ ] Closing the window hides it (never destroys it); audio continues; Quit only via tray menu or Cmd/Ctrl+Q.
- [ ] Tray menu: disabled now-playing line (Station - artist - title, "Reconnecting...", or error text), Play/Stop (label follows state, "Play" in `error`), Favourites submenu (click plays), Show/Hide, Quit. Menu updates from `player-state` and `settings-changed`.
- [ ] Tray code is thin: it calls the same `play`/`stop`/show/hide paths and holds no logic; state-to-label mapping is a pure function with table tests.
- [ ] Left-click toggles the window where the OS supports click events; monochrome tray icon (template image on macOS).
- [ ] Feed refresh is skipped while the window is hidden.

## Smoke checklist
S12, S13. Shared Definition of Done in [§9 Testing & DoD](../spec.md#9-testing--the-check-command) applies.

## Notes
- Linux AppIndicator tray is menu-only (no click events); Show/Hide in the menu covers it. Stock GNOME needs the AppIndicator extension; document, do not work around.
- macOS keeps the dock icon and reopens the window on dock click.
