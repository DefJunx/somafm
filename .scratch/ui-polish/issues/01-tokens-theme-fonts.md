# 01 Semantic tokens, Theme and fonts

Status: resolved
Blocked by: none

## Goal
Replace the indigo look with the dark SomaFM-flavoured Theme via a semantic token layer in `src/app.css`, and apply the font stacks. No layout changes.

## Spec sections
- [Decisions](../spec.md#decisions-agreed-do-not-reopen) (Theme, Palette, Fonts, Tokens)

## Acceptance criteria
- [ ] `src/app.css` defines semantic tokens on top of Open Props: surface (base `#000`, raised `#272727`, control `#222`/`#444`), text (`#eee`, muted), heading/link gold `#D8A24D`, accent/focus teal `#1a9096`, primary action green `#4CAF50`, radius (8px control), motion (durations/easing, with a `prefers-reduced-motion` override that zeroes them).
- [ ] Font tokens: heading stack (Trebuchet MS, Verdana, ui-sans-serif) and data stack (Monaco, 'Lucida Console', Courier, ui-monospace). No font files or font packages added.
- [ ] `Grid.svelte`, `Player.svelte`, `NowPlayingBar.svelte` and `App.svelte` use only the semantic tokens; `grep -rn "indigo\|--surface-[0-9]" src` returns nothing outside the token definitions. Raw Open Props values appear only inside `app.css` token definitions.
- [ ] Station names and headings use the heading stack and gold where they are headings/links; Now playing, listeners and status text use the data stack; everything else stays on the body stack.
- [ ] Dark only: no `prefers-color-scheme` light branch, `color-scheme: dark` set.
- [ ] Green is used only on the play action; teal only for hover/focus.
- [ ] `src/lib/*.test.ts` and `src/views/views.test.ts` stay green; `pnpm check:web` passes (no markup changes expected).

## Manual verification (user)
1. `pnpm tauri:dev`; the Grid and Player screen are black/dark grey with gold headings, no indigo anywhere.
2. Station names look like Trebuchet/Verdana; Now playing and status text are monospaced.
3. Play a Station: only the play button is green.
4. Toggle the OS light theme: the app stays dark.

## Notes
- Do not copy SomaFM assets, markup or logo.
