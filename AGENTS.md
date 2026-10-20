## Agent skills

### Issue tracker

Issues are local markdown files under `.scratch/<feature>/`. See `docs/agents/issue-tracker.md`.

### Triage labels

Default five-role vocabulary (`needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`). See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: one `GLOSSARY.md` + `docs/adr/` at the repo root. See `docs/agents/domain.md`.

## Versioning

Versions follow [Semantic Versioning](https://semver.org/). Release tags are `v<version>` (they trigger `.github/workflows/release.yml`) and match the version in `package.json`, `src-tauri/tauri.conf.json` and `src-tauri/Cargo.toml` (refresh `Cargo.lock` after bumping).

On every version bump, update `CHANGELOG.md` ([Keep a Changelog](https://keepachangelog.com/en/1.1.0/)) in the same commit: rename `[Unreleased]` to `[<version>] - <date>`, open a fresh empty `[Unreleased]` above it, and update the compare links at the bottom. Fill `[Unreleased]` from `git log <last tag>..HEAD`, grouped under Added / Changed / Deprecated / Removed / Fixed / Security, written for users.
