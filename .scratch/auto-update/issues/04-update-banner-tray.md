# 04 Update banner and tray item

Status: ready-for-agent
Blocked by: 03

## Goal
The user sees a dismissible Update banner and a tray item when an update is available.

## Spec sections
- [Decisions](../spec.md#decisions-agreed-do-not-reopen) (Prompt)

## Acceptance criteria
- [ ] An Update banner (shown over the Grid and Player screen) lists the new version and release notes (expandable for long notes) with "Restart to update" and "Later". "Later" hides it until the next launch.
- [ ] "Restart to update" calls `install_update`, shows progress, and restarts; failure shows an error in the banner and playback is untouched.
- [ ] The tray menu gains "Update to vX.Y.Z" only while an update is `Available`; clicking it triggers the same flow (the banner shown, or confirm there; never install without a click on the banner or item). Tray logic stays free of business rules.
- [ ] The banner honours `prefers-reduced-motion` and uses Theme tokens; accessible roles (`status`/`region`).
- [ ] Vitest with `mockIPC` covers show, "Later", install and error; a tray menu test (existing tray test pattern) covers item presence.
- [ ] `pnpm check` passes.
