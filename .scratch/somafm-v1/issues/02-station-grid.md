# 02 Station grid: feed fetch + cache + offline fallback, Tile wall

Status: ready-for-agent
Blocked by: 01

## Goal
Rust fetches and caches the SomaFM feed; the Grid renders the Tile wall with genre chips and offline behaviour.

## Spec sections
- [§3 Architecture](../spec.md#3-architecture-rust-vs-frontend) (Station types, `get_stations`, `refresh_stations`, `stations-updated`)
- [§4 Station feed & cache](../spec.md#4-station-feed--cache)
- [§6 UI](../spec.md#6-ui) (Grid, filter chips except Favourites, empty state)

## Acceptance criteria
- [ ] `feed` parses `channels.json` into `Station`s (genres split on `|`, `aacp` playlists dropped, missing/extra fields tolerated), test-first with synthetic fixtures; one `#[ignore]` live parse check.
- [ ] Fetch at startup and every 5 min while the window is visible; last good copy cached in the app cache dir; failed refresh retries on backoff 30 s, 60 s, ... up to 5 min (tested against a stub HTTP server).
- [ ] `get_stations`, `refresh_stations` and the `stations-updated` event work per spec; `loaded=false` when there is no cache and no success.
- [ ] Grid shows uniform square Tiles in feed order with hover play button, All + genre filter chips, image-failure fallback (name on plain surface), and the "Can't reach SomaFM" + Retry empty state; Vitest covers store mapping and chips.
- [ ] Tile click is wired to a no-op placeholder (play comes in 03).

## Smoke checklist
S1, S2. Shared Definition of Done in [§9 Testing & DoD](../spec.md#9-testing--the-check-command) applies.

## Notes
- Tile images load directly from SomaFM URLs in the webview; never download or commit them.
- Tests use invented stations only.
