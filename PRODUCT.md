# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

A Tauri 2 desktop app (Linux, macOS, Windows) whose UI is a Svelte webview. The interface is the same on every OS; native integration comes from the shell (tray, media keys, OS now-playing overlay), not from per-OS design languages.

## Users

The primary user is the author: a long-time SomaFM listener who keeps a Station playing in the background all day while working, and only glances at or touches the app occasionally (switch Station, check what is playing, like a track). Secondary audience: people who find the public GitHub releases and share that same background-listening habit. Their needs follow the author's; they do not steer the product separately.

## Product Purpose

An unofficial, dedicated desktop player for SomaFM, the listener-supported internet radio service. It replaces a browser tab with a client that stays out of the way, plays reliably, and helps the listener find the music they heard later. Success is forgetting the app is there until you want something from it, and never losing a track you liked.

## Positioning

- **Lives in the background.** Close-to-tray, tray menu with now-playing and favourites, media keys and the OS now-playing overlay: it runs for hours without needing a window.
- **Track discovery.** Station history and Liked tracks, with lookups on Bandcamp, Discogs and MusicBrainz, turn passive listening into music you can find again.
- **Native, light, reliable.** A Rust audio pipeline (ADR 0001) with Mirror fallback, automatic reconnect and smooth Fades: sturdier and better-behaved than the website player.
- **SomaFM-only.** A dedicated client for one service, not a generic radio tuner.

## Operating Context

Runs all day on a desktop, mostly hidden or in a corner, often behind other work. Interaction is brief and occasional: glance at Now playing, heart a track, switch Station from the Grid or tray, pause with a media key. Longer sessions happen in the Liked view, reviewing and looking up tracks. Window is resizable down to a small size, so layouts must work narrow.

## Capabilities and Constraints

- Screens: Grid (all Stations as Tiles), Player screen (transport, volume, Now playing, Station history, Visualizer), Liked view, Settings popover, persistent now-playing bar. Terms follow `GLOSSARY.md`.
- Streams are MP3 only via `.pls` Mirrors; AAC and quality selection are out of scope.
- Station data, artwork and history are fetched from SomaFM at runtime.
- Settings and window state persist locally; Favourites and Liked tracks are stored on the user's machine.
- Linux is the verified platform; macOS and Windows builds exist but are unverified.

## Brand Commitments

- Unofficial and independent: the README disclaimer (not affiliated with or endorsed by SomaFM) and the pointer to support SomaFM stay.
- Never bundle or commit SomaFM logos, artwork, markup or a station snapshot; artwork is fetched at runtime. "SomaFM" is used only as a plain descriptive label, not as the app's branding (`.scratch/somafm-app/issues/06-third-party-policy-grilling.md`).
- Published on GitHub as a personal passion project, MIT licensed.
- The current look is the Emission Line Rail world (dark-only, one emission hue per genre, self-hosted Schibsted Grotesk), recorded in DESIGN.md; it replaced the earlier `.scratch/ui-polish/spec.md` theme. Visual system detail belongs in DESIGN.md, not here.

## Evidence on Hand

- Real product: shipped releases 0.1.0 to 0.3.0 (`CHANGELOG.md`), with checksums and build attestations.
- No testimonials, user counts, reviews or press exist. Do not invent any.

## Product Principles

1. **Stay out of the way.** The default state is background; every surface should be readable at a glance and quick to act on, then let go.
2. **Never lose the music.** Anything the listener heard and liked should be easy to capture and find again.
3. **Reliability over features.** Playback that recovers on its own beats a new control.
4. **Respect SomaFM.** Credit and support the service, never impersonate it or ship its assets.

## Accessibility & Inclusion

Visible focus states and keyboard operation throughout. Motion (screen transitions, the Visualizer) honours `prefers-reduced-motion`; the Visualizer is off by default when reduced motion is requested.
