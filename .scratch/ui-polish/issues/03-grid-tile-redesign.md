# 03 Grid and Tile redesign, Skeleton tiles, Status badge

Status: resolved
Blocked by: 02

## Goal
Redesign the Grid and Tile: artwork with Station name and genre below, Status badge off the artwork, Skeleton tiles while loading.

## Spec sections
- [Decisions](../spec.md#decisions-agreed-do-not-reopen) (Tile, Interaction pass)
- [Audit findings](../spec.md#audit-findings) (Grid renders nothing while loading, paused dot animates)

## Acceptance criteria
- [ ] Tile layout: square artwork on top; Station name (heading font) and genre (muted) below, each single-line with ellipsis overflow. No listeners or current-track data on Tiles.
- [ ] Hover (and keyboard focus) play control stays on the artwork, green, using the play icon; Active station keeps its marker (ring) per GLOSSARY.
- [ ] Status badge sits below the artwork next to the name (not over the art), icon + short label for playing / paused / connecting / error; the dot animates only while `connecting`/`playing` as appropriate and never when paused; error uses the warning icon; badge has an accessible name.
- [ ] Image-failure fallback (name on plain surface) still works.
- [ ] While `!store.ready`, the Grid renders a fixed set of Skeleton tiles (non-interactive, `aria-hidden`/`aria-busy` on the Grid, subtle shimmer disabled under `prefers-reduced-motion`). A warm cache must not flash the empty state; the "Can't reach SomaFM" + Retry empty state still shows only after a failed load.
- [ ] Chips keep behaviour (All + genre + Favourites) and use the 02 states.
- [ ] `src/lib/stations.test.ts`, `genres.test.ts` and `src/views/views.test.ts` green; add/adjust `views.test.ts` cases for Skeleton tiles while not ready and for badge placement per status. `pnpm check:web` passes.

## Manual verification (user)
1. Launch with a cold cache/offline-then-online: Skeleton tiles appear first, then real Tiles replace them without the empty state flashing.
2. Tiles show art, name and genre below; long names truncate.
3. Play a Station, pause it, then break the stream: badge below the art shows playing, paused (static), error (warning icon) correctly.
4. Hover and Tab to a Tile: play control appears on the art.
5. Go offline with no cache: empty state + Retry still appear.
