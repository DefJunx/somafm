# 02 Generate and safeguard updater signing keys

Status: ready-for-human
Blocked by:

## Goal
Create the updater signing key pair and store it safely. Only the owner can do this.

## Spec sections
- [Decisions](../spec.md#decisions-agreed-do-not-reopen) (Signing)
- [ADR 0002](../../../docs/adr/0002-auto-update.md)

## Acceptance criteria
- [ ] `pnpm tauri signer generate` run locally with a strong password.
- [ ] Private key and password added as GitHub Actions secrets (e.g. `TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`).
- [ ] Public key handed to the agent for `tauri.conf.json` (ticket 05); the private key is never committed.
- [ ] Private key and password backed up offline (password manager or offline media). Losing it means existing installs can never update.
