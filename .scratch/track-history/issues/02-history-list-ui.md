# 02 Station history list on the Player screen

Status: ready-for-agent
Blocked by: 01

## Goal
The Player screen lists the Station history under the Now playing track.

## Spec sections
- [Decisions](../spec.md#decisions-agreed-do-not-reopen) (Station history source, Display)

## Acceptance criteria
- [ ] On opening the Player screen the frontend calls `get_station_history` for the Active station; rows show time and "artist – title" (data font per the Theme), about 20 entries, scrollable.
- [ ] It refetches when the Now playing title changes while the Player screen is visible; it does not fetch on the Grid, when the window is hidden, or on a timer.
- [ ] A failed fetch shows a quiet "History unavailable"; no automatic retry. A later title change may try again.
- [ ] Switching Station discards the previous list; a stale response for a previous Station is ignored.
- [ ] Vitest with `mockIPC` covers: fetch on open, refetch on title change, no fetch on Grid, failure message, stale response ignored.
- [ ] `pnpm check` passes.

## Manual verification (user)
1. Open a playing Station: history appears under the track.
2. Wait for a track change: the list updates once.
3. Go offline, wait for a track change: "History unavailable", no spinner loop.
