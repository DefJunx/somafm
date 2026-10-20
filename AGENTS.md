## Agent skills

### Issue tracker

Issues are local markdown files under `.scratch/<feature>/`. See `docs/agents/issue-tracker.md`.

### Triage labels

Default five-role vocabulary (`needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`). See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: one `GLOSSARY.md` + `docs/adr/` at the repo root. See `docs/agents/domain.md`.

## Versioning

Versions follow [Semantic Versioning](https://semver.org/). Release tags are `v<version>` (they trigger `.github/workflows/release.yml`) and match the version in `package.json`, `src-tauri/tauri.conf.json` and `src-tauri/Cargo.toml` (refresh `Cargo.lock` after bumping).
