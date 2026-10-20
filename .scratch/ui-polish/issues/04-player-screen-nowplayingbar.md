# 04 Player screen and NowPlayingBar

Status: resolved
Blocked by: 03

## Goal
Polish the Player screen and the NowPlayingBar: typography, status differentiation, long-name handling, and consistent controls.

## Spec sections
- [Decisions](../spec.md#decisions-agreed-do-not-reopen) (Palette, Fonts, Interaction pass)
- [Audit findings](../spec.md#audit-findings) (station `h1` overflow, undifferentiated status, paused dot)

## Acceptance criteria
- [ ] Station `h1` uses heading font + gold; long names wrap to at most two lines then ellipsis (`line-clamp`) with the full name in `title`; no layout break.
- [ ] Now playing text uses the data (monospace) font and also truncates safely.
- [ ] Status text is differentiated per state (connecting/reconnecting, playing, paused, error by kind) using the Status badge look from 03 (icon + colour + text); paused shows no animation; error shows the warning icon and Retry.
- [ ] Primary Play/Pause is the green action (Listen-style 8px radius); Stop, Favourite, Back and Retry use the `#444` button style with 02 states; volume row uses the styled slider with mute/volume icon.
- [ ] NowPlayingBar uses the same button/status components and tokens, shows status text with Retry, and truncates the Station name and track.
- [ ] No new screens, settings or donation links.
- [ ] `src/views/views.test.ts` and all `src/lib/*.test.ts` green; extend `views.test.ts` for status rendering (paused has no animated element) and the Retry path. `pnpm check:web` passes.

## Manual verification (user)
1. Play a Station with a long name/track (or temporarily mock one): text truncates, layout intact.
2. Pause: status reads paused with a static marker; resume: playing.
3. Kill the network: error state shows warning icon, message and Retry in both the Player screen and NowPlayingBar; Retry works.
4. Check the NowPlayingBar on the Grid for the same states and focus/hover behaviour.
