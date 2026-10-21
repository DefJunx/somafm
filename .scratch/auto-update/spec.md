# Auto-update spec (0.3.0)

The app tells the user when a new version exists and can install it on request. Terms (Update banner) follow `GLOSSARY.md`. Decision record: [ADR 0002](../../docs/adr/0002-auto-update.md). This also introduces the shared Settings popover used by [audio-fx](../audio-fx/spec.md).

## Decisions (agreed, do not reopen)
- **Updater**: Tauri updater plugin. Check on launch and every 24 h when "Check for updates automatically" is on. Never install silently; never cut playback without asking.
- **Prompt**: a dismissible in-app banner with version, release notes, and "Restart to update" / "Later"; plus a tray item "Update to vX.Y.Z". "Later" snoozes until the next launch; there is no skip-this-version.
- **Signing**: `tauri signer generate`; private key and password as GitHub Actions secrets; public key in `tauri.conf.json`; offline backup of the private key (losing it means existing installs can never update). A `ready-for-human` issue.
- **Manifest**: the release job generates `latest.json` plus `.sig` files (`createUpdaterArtifacts`) as release assets. Endpoint `https://github.com/DefJunx/somafm/releases/latest/download/latest.json`. Notes come from that version's `CHANGELOG.md` section.
- **Linux**: notify-only. Same banner, but with a link to the release page and no in-place install, because the `tar.gz` + `install-linux.sh` flow is not updatable by Tauri. AppImage is a possible later ticket.
- **macOS**: included in the updater, marked unverified like the rest of macOS support.
- **Settings (shared)**: a gear button in the Grid header opens a small Settings popover with three toggles: "Check for updates automatically" (default on), "Fade audio" (default on), "Show visualizer" (default on; off when the OS prefers reduced motion). New camelCase keys in `settings.json` via a schema v1 -> v2 migration (the first entry in MIGRATIONS), included in `SettingsView` and `settings-changed`.

Source: grilling session with the user, 2026-10-21.

## Tickets
01 settings popover + v2 migration -> 02 signing keys (human) -> 03 update check (Rust) -> 04 Update banner + tray item -> 05 release manifest (needs 02) -> 06 Linux notify-only -> 07 release 0.3.0.

Every ticket: test-first per the v1 test strategy; the gate is `pnpm check` (use `pnpm check:toolbox` on this host).

## Out of scope
- Skip-this-version.
- Silent or forced installs.
- AppImage for Linux in-place updates.
- Configurable check interval or release channels.
- Verifying the macOS updater.
