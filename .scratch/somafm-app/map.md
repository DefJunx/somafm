# Map: SomaFM Tauri app

Label: wayfinder:map

## Status

Complete (2026-10-16): every decision is locked. Build-ready spec: [`../somafm-v1/spec.md`](../somafm-v1/spec.md); builder tickets: `../somafm-v1/issues/01-09`.

## Destination

A build-ready spec for a cross-platform Tauri 2 desktop app that lists SomaFM stations as an image grid (hover a tile to reveal play), opens a player screen (stop, volume, etc.), and keeps playing when you go back to the grid. The map ends when every decision an implementing agent needs is locked. Building the app is out of scope for this map.

## Notes

- Domain: internet radio client over SomaFM's public feeds. See `GLOSSARY.md`.
- Stack (decided with user): Tauri 2, plain Vite via the create-tauri-app Svelte-TS template, Svelte + TypeScript. Vite+ dropped. CSS: Open Props (confirmed after the UI prototype).
- Platforms: cross-platform (Linux, macOS, Windows).
- Station data: fetched at runtime from SomaFM's public feed, with a cached fallback for offline or failed fetches.
- Navigation: full player screen with a back button; audio continues on the grid, where the playing station is marked active and has a stop control.
- v1 features (user: all proposed): now-playing track title, stream quality choice, favourites, media keys + system tray. Sleep timer dropped (user, 2026-10-16).
- Research tickets write findings under `research/` in a throwaway branch/worktree; they are never merged.

## Decisions so far

<!-- one line per resolved ticket: [title](issues/NN-slug.md): gist -->

- [Toolchain: Vite+, Tauri 2, Svelte, Pico.css](issues/03-toolchain-research.md): plain Vite via the create-tauri-app Svelte-TS template; Vite+ dropped. CSS: Open Props (confirmed by the user in the UI prototype; Pico.css is archived).
- [Audio playback across webviews](issues/01-audio-playback-research.md): `<audio>` works with caveats (mixed content, GStreamer on Linux, weak media keys); native Rust is the alternative. Choice deferred to architecture.
- [SomaFM data feeds](issues/02-somafm-data-research.md): one CORS-enabled `channels.json` with 51 stations; no structured song history.
- [SomaFM terms vs. this app](issues/06-third-party-policy-grilling.md): SomaFM forbids third-party clients, so the app is personal-use only, never distributed; no SomaFM artwork or snapshots committed to the repo, everything fetched at runtime.
- [Architecture split: Rust vs frontend](issues/04-architecture-grilling.md): native Rust audio (symphonia/rodio, no HE-AAC; MP3 or AAC 128); Rust owns feed+cache, a single PlayerState broadcast as snapshot events, tray, media keys; settings in tauri-plugin-store; close hides to tray. See [ADR 0001](../../docs/adr/0001-native-rust-audio.md).
- [Grid, hover play, player screen](issues/05-ui-prototype.md): Variant A "Tile wall" won. Clicking a tile opens the player and plays. Split player screen. The active tile gets a ring and an equaliser badge, plus a bottom now-playing bar. Favourites are a filter chip. Open Props confirmed.
- [Media keys and tray behaviour](issues/07-media-keys-tray-grilling.md): Play/Pause = stop/live reconnect (overlay shows Paused); no Next/Previous; overlay gets artist/title split from ICY plus station name and art; tray menu = now playing, Play/Stop, Favourites, Show/Hide, Quit; Linux tray is menu-only (GNOME needs AppIndicator), macOS keeps the dock icon.
- [Stream errors and reconnect](issues/08-stream-errors-grilling.md): drops reconnect across Mirrors with backoff for up to 2 min, while a first connect makes one pass; 10s connect / 15s stall timeouts; `connecting{reconnecting}` and `error{kind: offline|unavailable|unsupported}` keep the station loaded, with Retry instead of auto-resume; cached feed and cached `.pls` Mirror lists for offline use.
- [Settings schema and migration](issues/09-settings-schema-grilling.md): v1 store = `schemaVersion`, `favourites` (id set, feed order), `volume` 0.8, `quality` mp3, `lastStationId` (restored stopped, no autoplay), `favouritesOnly`; missing favourites hidden never pruned; ordered migrations, lenient per-key fallback, corrupt file -> `.bak`; window geometry via window-state plugin.
- [Test strategy](issues/11-test-strategy-grilling.md): pure Rust reducer with time passed in; exhaustive parser/reducer unit tests; local stub HTTP server; synthetic fixtures only; one `#[ignore]` live-feed check; Vitest + mockIPC for stores/components; no E2E (null-sink decode test + manual smoke checklist); done = fmt/clippy/cargo test/svelte-check/vitest via one local `check`, core test-first; no coverage gate, no CI.
- [Local build and install per platform](issues/12-local-build-install-grilling.md): pnpm + Rust stable; Linux builds in a Fedora 44 toolbox, `--no-bundle` binary in `~/.local/bin` plus a `.desktop` file; macOS `.app` copied to /Applications; Windows NSIS per-user; manual rebuild to update, identifier `dev.defjunx.somafm`; original app icon; only Linux (Bazzite KDE) verified.
- [Spec layout and builder ticket breakdown](issues/13-spec-layout-grilling.md): self-contained spec at `.scratch/somafm-v1/spec.md` (11 sections, exact `PlayerState`/IPC contract) plus 9 vertical-slice builder tickets (scaffold -> grid -> play -> {now playing, errors, settings/favourites} -> tray, media keys -> build/install); shared DoD + per-ticket acceptance criteria, verified on Linux only.

## Not yet specified

<!-- all fog graduated to tickets 09-13 on 2026-10-16 -->

## Out of scope

- Building the app (this map ends at a spec).
- Distribution of any kind: public releases, installers, app stores (SomaFM terms forbid third-party clients, so this stays personal-use; see [policy ticket](issues/06-third-party-policy-grilling.md)).
- Accounts, scrobbling, recording or downloading streams, SomaFM donation/shop flows.
- Launch at login and auto-update: not in v1; KDE Autostart covers the first, manual rebuilds the second (see [Local build and install](issues/12-local-build-install-grilling.md)).
- Sleep timer: the user does not want one (see [Sleep timer UX](issues/10-sleep-timer-grilling.md), closed).
