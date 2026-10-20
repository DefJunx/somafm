# UI polish spec

SomaFM-flavoured but modern. Polish only: no new screens, settings or donation links. Copy no SomaFM assets, markup or logo. Terms follow `GLOSSARY.md`.

## Decisions (agreed, do not reopen)
- **Theme**: dark only, no light variant; the indigo accent is removed.
- **Palette** (from somafm.com CSS): surfaces `#000` and `#272727` (`#222`/`#444` for controls); text `#eee`; headings/links gold `#D8A24D`; hover/focus accent teal `#1a9096`; green `#4CAF50` only for the primary action (play). Listen-style buttons: `#444` bg, 8px radius.
- **Fonts**: system stacks only, nothing bundled. Headings and Station names: `"Trebuchet MS", Verdana, ui-sans-serif, sans-serif`. Data text only (Now playing, listeners, status): `Monaco, "Lucida Console", Courier, ui-monospace, monospace`.
- **Tokens**: semantic layer in `src/app.css` on top of Open Props (surface, text, accent, focus, radius, motion). Components use only these tokens, never raw `--indigo-5` / `--surface-3`.
- **Window**: resizable, min 480x520 (currently fixed 800x600 in `src-tauri/tauri.conf.json`); Grid and Player screen layouts respond to width.
- **Interaction pass**: visible `:focus-visible` ring; hover/disabled for all buttons and chips; styled volume range; Skeleton tiles while `!store.ready`; Grid<->Player fade/slide honouring `prefers-reduced-motion`; one small inline SVG icon set (play, pause, stop, star, back, volume, mute, warning), no icon dependency.
- **Tile**: artwork, then Station name + genre below; Status badge moved off the artwork; hover play control stays on the art; no live data (listeners / current track) on Tiles.

## Audit findings
- Tiles/buttons lack focus-visible (`Grid.svelte` ~100-111); chips lack hover (~87-91).
- NowPlayingBar buttons have no hover/focus (`NowPlayingBar.svelte` ~35).
- Volume range unstyled; favourite uses star characters (`Player.svelte` ~64-74).
- Player two-column grid has no narrow breakpoint (~81); Station `h1` has no overflow handling.
- Status text is undifferentiated; paused status renders an animating dot.
- Grid renders nothing while loading; `App.svelte` swaps views with no transition.

## Tickets (stop after each for manual verification)
01 tokens + Theme + fonts -> 02 interaction states + icons -> 03 Grid/Tile -> 04 Player screen + NowPlayingBar -> 05 resizable window + responsive + transition.

Every ticket: keep `src/lib/*.test.ts` and `src/views/views.test.ts` green; run `pnpm check:web` (svelte-check + vitest) and, for the final gate, `pnpm check` (use `pnpm check:toolbox` on this host). Update tests only where markup/labels intentionally change, and say so.
