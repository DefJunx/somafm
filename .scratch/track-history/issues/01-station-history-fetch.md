# 01 Station history fetch: songs endpoint client and command

Status: resolved
Blocked by:

## Goal
Rust can fetch and parse a Station's recent tracks from SomaFM's songs endpoint and expose them to the frontend.

## Spec sections
- [Decisions](../spec.md#decisions-agreed-do-not-reopen) (Station history source, Display)

## Acceptance criteria
- [x] A `history` fetch function in Rust (alongside `feed.rs`, same HTTP client conventions) GETs `https://somafm.com/songs/<id>.json` and parses it into entries `{ time, artist, title }` (about 20), leniently: malformed entries are skipped, unknown fields ignored. Parser unit tests use a synthetic fixture, never a committed SomaFM payload.
- [x] A thin `get_station_history(station_id)` command returns the entries or an error value the frontend can show as "History unavailable". No retry, no caching on disk.
- [x] Integration test against a stub server: success, HTTP error, invalid JSON, timeout all map to the expected result.
- [x] No request is made unless the command is called (no timer, nothing at startup).
- [x] `pnpm check` passes.

## Notes
- The base URL must be overridable for the stub server the same way the feed URL is.
