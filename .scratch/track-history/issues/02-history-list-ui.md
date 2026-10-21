# 02 Station history list on the Player screen

Status: resolved
Blocked by: 01

## Goal
The Player screen lists the Station history under the Now playing track.

## Spec sections
- [Decisions](../spec.md#decisions-agreed-do-not-reopen) (Station history source, Display)

## Acceptance criteria
- [x] On opening the Player screen the frontend calls `get_station_history` for the Active station; rows show time and "artist – title" (data font per the Theme), about 20 entries, scrollable.
- [x] It refetches when the Now playing title changes while the Player screen is visible; it does not fetch on the Grid, when the window is hidden, or on a timer.
- [x] A failed fetch shows a quiet "History unavailable"; no automatic retry. A later title change may try again.
- [x] Switching Station discards the previous list; a stale response for a previous Station is ignored.
- [x] Vitest with `mockIPC` covers: fetch on open, refetch on title change, no fetch on Grid, failure message, stale response ignored.
- [x] `pnpm check` passes.

## Manual verification (user)
1. Open a playing Station: history appears under the track.
2. Wait for a track change: the list updates once.
3. Go offline, wait for a track change: "History unavailable", no spinner loop.

## Comments

Manual verification done by the user on 2026-10-21. Shipped in 0.2.0.
