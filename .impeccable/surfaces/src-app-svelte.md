---
version: 1
slug: "src-app-svelte"
primary_target: "src/App.svelte"
related_targets: ["src/views/Player.svelte","src/views/Grid.svelte","src/views/Liked.svelte","src/views/NowPlayingBar.svelte"]
---

# Surface brief: the app shell (Grid, Player screen, Liked view, Now playing bar)

Scope: whole-app visual redesign, replacing "The Listening Room". Mode: Operate. Build path: code-led (decision sketch `.impeccable/mocks/decision/emission.html` is the composition; QUALITY BAR board/hero `operate-c-emission-line-rail`).

Audience/job: one background listener glancing at what plays, switching Station, liking a track; longer sessions in Liked. Window resizes down to 480x520. Constraints: product truth in PRODUCT.md; no SomaFM assets; reduced motion honoured; keyboard and focus throughout.

## Direction contract

THESIS: Radio as spectrum. Every Station is a line on one tuning rail; state is told by line form (full, dashed, short, doubled) with a plain word beside it, never by colour alone. Refuses the category default: a uniform tile grid with a play-button accent and a separate player card.

OWN-WORLD: charcoal continuum ground #16171a with faint horizontal banding, plate #22242a, hairlines #33353b, bone text #e6e1d6, dim #8c8a85. Seven emission hues at full strength, one per genre, used identically on rail, Grid, Liked and bar: violet #8b5cff, indigo #5b6cff, cyan #33c6ff, green #7dff5a, orange #ff7a2a, deep red #ff3b3b; sodium #ffc83a is reserved for Liked (doubled line, filled heart). 1px emission hairlines, 4px radius, outline buttons, inverted bone primary. One self-hosted grotesque (Schibsted Grotesk), three integer size steps (13/16/40px); hierarchy by tracking, weight and ink.

STORY: the listener sees which Station is live and what it plays in one glance, changes Station from the rail or Grid, hearts the playing track, and later finds it in Liked by its line colour and date.

FIRST VIEWPORT: Player screen. 48px top bar: muted tracked app label, Stations / Liked buttons, settings icon right. Left, an off-centre tuning rail (~220px) runs the window height: every Station a 1px tick in its genre hue, named ticks right-aligned; the live Station's tick extends 48px across the rail into the plate and reads "live". Right, the plate (art 216px, 40px Station name, genres + listeners, description, transport: bone Stop/Play, volume line, favourite), then the Visualizer as 1px emission lines in the Station's hue, then Station history as 32px rows with the live row marked by a 2px line and a heart. 56px Now playing bar along the foot with the hue mark, Station, track, state word, action.

FORM: Emission Line Rail, catalog challenger (operate-c-emission-line-rail) that won the bolder re-roll over the dealt hand; seed:ab2c208da28135f2. Signature move: the tuning rail. Raises: acetate manual (one hue per genre everywhere), bitmap specimen (three size steps), gate board (rows keep identity; new track slides history down in place).

FINISH: unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance
