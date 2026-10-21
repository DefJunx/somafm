# 05 Release pipeline: signed artifacts and latest.json

Status: ready-for-agent
Blocked by: 02, 03

## Goal
The release workflow produces updater artifacts, signatures and `latest.json` with notes from `CHANGELOG.md`.

## Spec sections
- [Decisions](../spec.md#decisions-agreed-do-not-reopen) (Signing, Manifest)
- [ADR 0002](../../../docs/adr/0002-auto-update.md)

## Acceptance criteria
- [ ] Public key from ticket 02 placed in `src-tauri/tauri.conf.json`; `createUpdaterArtifacts` enabled; the workflow passes the signing secrets to `tauri build`/the bundle steps.
- [ ] A script (e.g. `scripts/changelog-section.*`, unit-tested with a sample changelog) extracts the `## [<version>]` section of `CHANGELOG.md`.
- [ ] The release job assembles `latest.json` (version, notes, pub_date, per-platform `url` + `signature`) and uploads it plus the `.sig` files as release assets alongside the existing artifacts; `SHA256SUMS` and attestations still cover everything.
- [ ] The Linux entry is omitted or flagged per ticket 06; macOS and Windows entries are present.
- [ ] The workflow remains valid (lint with `actionlint` if available) and a dry-run notes extraction for the current version is shown in the report. The first real run happens with the next tag.
- [ ] `pnpm check` passes.
