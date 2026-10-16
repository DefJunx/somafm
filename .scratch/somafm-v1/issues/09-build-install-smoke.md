# 09 Local build & install + manual smoke run

Status: ready-for-agent
Blocked by: 05, 07, 08

## Goal
Make the app buildable and installable per platform for personal use and run the full smoke checklist on the installed Linux build.

## Spec sections
- [§10 Build & install](../spec.md#10-build--install-per-platform)
- [§9 Testing & DoD](../spec.md#9-testing--the-check-command) (smoke checklist)

## Acceptance criteria
- [ ] Linux: documented toolbox `dnf install` line and `pnpm tauri build --no-bundle` produce a working binary; a `.desktop` template and icon are in the repo, and an install script or documented commands copy them to `~/.local/bin`, `~/.local/share/applications` and `~/.local/share/icons`.
- [ ] macOS (`--bundles app`, copy to /Applications, quarantine note) and Windows (`--bundles nsis`, per-user, SmartScreen note) steps are written in the repo README or build doc, marked unverified.
- [ ] Bundle identifier is `dev.defjunx.somafm`; no updater plugin, no launch-at-login.
- [ ] Full smoke checklist S1-S15 passes on the installed Linux build; any failure is fixed or filed as a new ticket.

## Smoke checklist
S16, S17. Shared Definition of Done in [§9 Testing & DoD](../spec.md#9-testing--the-check-command) applies.

## Notes
- Do not layer packages on the host. macOS/Windows steps are documentation only and never block this slice.
