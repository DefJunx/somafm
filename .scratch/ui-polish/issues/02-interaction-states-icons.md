# 02 Shared interaction states and icon set

Status: resolved
Blocked by: 01

## Goal
Give every interactive control consistent focus, hover and disabled states, a styled volume slider, and replace text glyphs with one inline SVG icon set.

## Spec sections
- [Decisions](../spec.md#decisions-agreed-do-not-reopen) (Interaction pass)
- [Audit findings](../spec.md#audit-findings)

## Acceptance criteria
- [ ] Visible `:focus-visible` ring (teal, via the focus token) on Tiles, play buttons, chips, NowPlayingBar buttons, Player buttons and the volume slider; fixes `Grid.svelte` ~100-111. No ring on plain mouse click.
- [ ] Hover and disabled styles for all buttons and chips (Listen-style: `#444` bg, 8px radius, teal hover); fixes chips (`Grid.svelte` ~87-91) and `NowPlayingBar.svelte` ~35. Disabled controls are dimmed, `cursor: not-allowed`, and ignore hover.
- [ ] Volume `input[type=range]` styled for WebKitGTK/WebView2/WKWebView (`::-webkit-slider-*`; add `::-moz-*` only if harmless): track, thumb, filled portion, hover/focus states.
- [ ] A small shared icon module (e.g. `src/lib/icons` or one Svelte component) with inline SVGs: play, pause, stop, star (outline + filled), back, volume, mute, warning. Uses `currentColor`, has `aria-hidden` plus accessible names on the parent button. No new dependency in `package.json`.
- [ ] Favourite button uses the star icons instead of the star characters (`Player.svelte` ~64-74); transport buttons use play/pause/stop icons; back button uses the back icon; volume/mute uses its icons.
- [ ] Existing accessible names (aria-labels/text queried by tests) preserved. `src/lib/*.test.ts` and `src/views/views.test.ts` green; update only assertions that depended on the star characters, and note it.
- [ ] `pnpm check:web` passes.

## Manual verification (user)
1. Tab through the Grid and Player screen: every control shows a clear teal ring; mouse clicks show none.
2. Hover chips, Tiles and NowPlayingBar buttons: visible change. Disabled buttons (e.g. before a Station is loaded) look dimmed and do not react.
3. Drag the volume slider: styled track and thumb, fill follows the value.
4. Star, play/pause/stop, back and volume icons render crisp and tint with the text colour.

## Notes
- Icons only; no layout redesign here (Tile in 03, Player in 04).
