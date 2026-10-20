# SomaFM desktop app v1: build spec

Self-contained. Builders read this file plus their ticket (`issues/NN-*.md`). Terms (Station, Grid, Tile, Player screen, Active station, Now playing, Favourite, Mirror) follow `GLOSSARY.md`. Decision: native Rust audio, see `docs/adr/0001-native-rust-audio.md`.

## 1. Purpose & scope

- A cross-platform (Linux, macOS, Windows) Tauri 2 desktop radio app: a Grid of SomaFM Stations, a Player screen, audio that keeps playing when you go back to the Grid.
- Personal use only, never distributed: no public releases, installers, store listings or published binaries.
- SomaFM's terms forbid third-party clients. Therefore: no SomaFM artwork, branding or station snapshot is committed to the repo; everything is fetched at runtime. Test fixtures are synthetic (section 9). The app name is a plain descriptive label, not SomaFM branding.
- v1 features: Grid, Player screen, Now playing (artist/title), Favourites, tray, media keys + OS overlay.
- Non-goals: see section 11.

Source: [SomaFM terms vs. this app](../somafm-app/issues/06-third-party-policy-grilling.md), [Spec layout and builder ticket breakdown](../somafm-app/issues/13-spec-layout-grilling.md)

## 2. Stack & project layout

| Item | Decision |
|---|---|
| Shell | Tauri 2 |
| Scaffold | `create-tauri-app` Svelte + TypeScript template, plain Vite (no Vite+) |
| Frontend | Svelte + TypeScript, scoped Svelte CSS + Open Props |
| Package manager | pnpm; Node LTS |
| Rust | stable via rustup |
| Bundle identifier | `dev.defjunx.somafm` (fixed, never changes) |
| Icon | original generic radio/waveform SVG committed to repo, expanded with `pnpm tauri icon`, plus a monochrome tray variant. No SomaFM artwork. |

Layout (repo root is the app root):

- `src-tauri/src/`: `feed` (fetch/cache/parse, `.pls` resolution), `player` (pure reducer core + engine shell: stream adapter, ICY, Mirror failover), `media_controls` (souvlaki), `tray`, `settings` (store wrapper), `commands` (thin IPC). `commands` and `tray` hold no logic.
- `src/lib/api.ts` (typed invoke/listen), `src/lib/stores/` (`playerState`, `stations`, `settings`), `src/views/Grid.svelte`, `src/views/Player.svelte`.
- Rust crates: `reqwest`, `symphonia` (mp3), `rodio`, `souvlaki`, `tokio`, `serde`, `tauri-plugin-store`, `tauri-plugin-window-state`; dev: `wiremock` (or axum/hyper stub).

Source: [Toolchain](../somafm-app/issues/03-toolchain-research.md), [Architecture split](../somafm-app/issues/04-architecture-grilling.md), [Local build and install](../somafm-app/issues/12-local-build-install-grilling.md), [Grid, hover play, player screen](../somafm-app/issues/05-ui-prototype.md)

## 3. Architecture: Rust vs frontend

Rust owns all state and logic. The frontend is a pure view. The webview, tray and media keys all drive the same commands. Every change to `PlayerState` is broadcast as one full-snapshot event.

### 3.1 Rust vs frontend

| Concern | Owner |
|---|---|
| Audio (HTTP stream, decode, output), Mirror failover, timeouts, backoff | Rust `player` |
| Feed fetch/cache, `.pls` resolution + cache | Rust `feed` |
| Settings read/write/migrate | Rust `settings` (only writer) |
| Tray, media keys/overlay, close-to-tray | Rust |
| Rendering, filter chips (view state), navigation Grid <-> Player screen | Frontend |
| Tile images | Webview loads straight from SomaFM URLs (never committed) |

### 3.2 PlayerState (single source of truth, in Rust)

Rust (`serde`, camelCase fields, internally tagged enums):

```rust
#[derive(Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlayerState {
    pub station_id: Option<String>, // loaded station; None only on fresh install
    pub status: Status,
    pub volume: f32,                // 0.0..=1.0
    pub now_playing: Option<NowPlaying>,
}

#[derive(Serialize, Clone, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Status {
    Stopped,
    Connecting { reconnecting: bool },
    Playing,
    Error { error_kind: ErrorKind, message: String },
}

#[derive(Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ErrorKind { Offline, Unavailable, Unsupported }

#[derive(Serialize, Clone, PartialEq)]
pub struct NowPlaying {
    pub raw: String,           // ICY StreamTitle verbatim
    pub artist: Option<String>,
    pub title: String,
}
```

TypeScript mirror (`src/lib/api.ts`):

```ts
export type ErrorKind = "offline" | "unavailable" | "unsupported";
export type Status =
  | { kind: "stopped" }
  | { kind: "connecting"; reconnecting: boolean }
  | { kind: "playing" }
  | { kind: "error"; errorKind: ErrorKind; message: string };
export interface NowPlaying { raw: string; artist: string | null; title: string }
export interface PlayerState {
  stationId: string | null;
  status: Status;
  volume: number;
  nowPlaying: NowPlaying | null;
}
```

Notes: the `error` variant uses the field name `errorKind` (named in spec) because `kind` is the enum tag. "Paused" is not a status: it is `stopped` with `stationId` set (UI and OS overlay call it Paused). `artist` is None when ICY has no ` - ` separator (UI/overlay then show the station name as artist).

### 3.3 Station and feed types (named in spec)

```ts
export interface Station {
  id: string; title: string; description: string; dj: string;
  genres: string[];          // `genre` split on "|"
  listeners: number;         // feed string parsed; 0 if unparsable
  lastPlaying: string;
  image: string; largeImage: string; xlImage: string; // feed image/largeimage/xlimage URLs
}
export interface StationsSnapshot { stations: Station[]; loaded: boolean } // loaded=false: no cache and no successful fetch yet
```

### 3.4 Commands (frontend -> Rust; `invoke`, args camelCase)

| Command | Args | Returns | Behaviour |
|---|---|---|---|
| `get_stations` | none | `StationsSnapshot` | current in-memory feed (cache-backed) |
| `refresh_stations` | none | `StationsSnapshot` | (named in spec) force a feed fetch now; used by the empty-state Retry |
| `get_player_state` | none | `PlayerState` | (named in spec) initial snapshot on webview load |
| `play` | `stationId: string` | void | load + connect station; also the Retry / resume action for the same id |
| `stop` | none | void | stop playback, keep station loaded (Paused), clear `nowPlaying` |
| `set_volume` | `volume: number` | void | clamp 0..1; apply live; persisted debounced ~500 ms |
| `get_settings` | none | `SettingsView` | (named in spec) `{ favourites: string[]; favouritesOnly: boolean }` |
| `toggle_favourite` | `stationId: string` | void | add/remove id; emits `settings-changed` |
| `set_favourites_only` | `value: boolean` | void | (named in spec) persists chip state; emits `settings-changed` |

Errors from commands are returned as `Err(String)` and only for programmer errors (unknown station id). Playback failures are never command errors; they surface as `Status::Error`.

### 3.5 Events (Rust -> frontend; `listen`)

| Event | Payload | When |
|---|---|---|
| `player-state` | `PlayerState` (full snapshot) | on every state change; also tick-free (no periodic emits) |
| `stations-updated` | `StationsSnapshot` | (named in spec) after each feed refresh that changes the data, or the first load |
| `settings-changed` | `SettingsView` | (named in spec) after favourites or favouritesOnly change, from any source (UI or tray) |

The frontend subscribes before calling the initial `get_*` to avoid a race, and replaces store state wholesale on each event.

Source: [Architecture split](../somafm-app/issues/04-architecture-grilling.md), [Stream errors and reconnect](../somafm-app/issues/08-stream-errors-grilling.md), [Media keys and tray](../somafm-app/issues/07-media-keys-tray-grilling.md), [Settings schema and migration](../somafm-app/issues/09-settings-schema-grilling.md)

## 4. Station feed & cache

- Source: `GET https://somafm.com/channels.json` (single feed, ~51 Stations; server caches ~5 s). Fetched by Rust, not the webview.
- Fields used per channel: `id`, `title`, `description`, `dj`, `genre` (pipe-separated -> `genres`), `listeners` (numeric string), `lastPlaying`, `image`, `largeimage`, `xlimage`, `playlists[]` (`{url, format, quality}`; only `format == "mp3"` entries are kept). Missing/extra fields are tolerated; a channel with no `id` is skipped. `aac` and `aacp` playlists are dropped (HE-AAC unsupported; see ADR 0001).
- Refresh: at startup, then every 5 min while the window is visible. Window hidden to tray: no refresh.
- Cache: last good `channels.json` stored in the app cache dir; loaded at startup so the Grid renders at once with no banner.
- Failed refresh: keep showing current data; retry with backoff 30 s, 60 s, ... up to the normal 5 min interval.
- No cache and no network: `StationsSnapshot.loaded = false`; Grid shows empty state "Can't reach SomaFM" with Retry (`refresh_stations`), and keeps retrying on the same backoff. Success emits `stations-updated`.
- Tile image failure: tile shows the Station name on a plain Open Props surface.
- `.pls` Mirror lists: see section 5.6.
- Nothing from the feed (images, text, JSON) is committed to the repo.

Source: [SomaFM data feeds](../somafm-app/issues/02-somafm-data-research.md), [Architecture split](../somafm-app/issues/04-architecture-grilling.md), [Stream errors and reconnect](../somafm-app/issues/08-stream-errors-grilling.md)

## 5. Playback

### 5.1 Pipeline
- `reqwest` streaming GET -> adapter implementing symphonia `MediaSource` -> `symphonia` decode -> `rodio` output. Engine runs on its own thread; the shell executes effects from the pure core (5.4).
- The adapter sends `Icy-MetaData: 1`, reads `icy-metaint`, strips metadata blocks from the audio bytes and yields `StreamTitle` values.
- Supported: MP3 only. AAC (`aac`, `aacp`) is never offered (ADR 0001). All stream URLs are HTTPS.
- Decode failure on a stream -> `error{unsupported}`.

### 5.2 Stream format
- MP3 only (`format == "mp3"` playlists). AAC is not offered: SomaFM's `aac` 128k streams are MPEG-2 ADTS (`0xFFF9`) at a 22.05 kHz core, i.e. very likely HE-AAC, which symphonia cannot decode (ADR 0001).
- A stream that cannot be recognised within the first 128 KiB, or cannot be decoded, ends in `error{unsupported}`.

### 5.3 Now playing (ICY)
- `StreamTitle='...'` is parsed from ICY blocks. Split on the first ` - `: artist before, title after. No separator: `artist = None`, `title = whole string`. Empty `StreamTitle` -> no change. Strip stray quotes/trailing `;`; non-UTF-8 handled lossily.
- `nowPlaying` is kept across reconnects; cleared on `stop` and on station change.
- UI shows `artist - title` (or station name as artist when `artist` is None). Feed `lastPlaying` is not used for the active station.

### 5.4 Core: pure reducer
- `fn reduce(state, event, now: Instant) -> (state, Vec<Effect>)`. Time is passed as data; no Clock trait.
- Events (examples): `PlayRequested(id)`, `StopRequested`, `Connected`, `ConnectFailed(kind)`, `StreamDropped`, `NoBytesTimeout`, `TimerFired`, `TitleChanged`, `DecodeFailed`, `SetVolume`.
- Effects: `ConnectMirror(n)`, `ArmTimer(at)`, `Disconnect`, `EmitSnapshot`, `ApplyVolume`, etc. The shell (tokio/reqwest/rodio) executes them.

### 5.5 Reconnect/error state machine

| Situation | Behaviour |
|---|---|
| First connect (Play from tile, button, media key, tray) | `connecting{reconnecting:false}`; one pass over Mirrors; no backoff cycling; then `error` |
| Drop while `playing` | `connecting{reconnecting:true}`; reconnect at once to the same Mirror, then walk the remaining Mirrors; after one full pass keep cycling with backoff 2 s, 4 s, 8 s, 16 s, capped at 30 s; give up to `error` 2 min after the drop |
| Mirror order | `.pls` order, starting with the Mirror that last worked for that Station this session |
| Connect timeout | 10 s per Mirror (TCP + TLS + first audio bytes) |
| Stall | while `playing`, 15 s with no bytes = a drop. Shorter decoder underruns are silence only, no status change |
| Success | `playing` |
| `error{offline}` | no Mirror reachable at network level |
| `error{unavailable}` | servers answered but refused or sent nothing (HTTP error, empty `.pls`) |
| `error{unsupported}` | decode failed |
| In `error` | Station stays loaded (like Paused); Play/Retry re-runs a first connect |
| Recovery | no auto-resume, no OS network-change watching; user retries |
| `stop` | -> `stopped`, station kept, `nowPlaying` cleared |

Error messages: offline "You're offline", unavailable "Station unavailable", unsupported "Can't play this stream".

### 5.6 `.pls` and Mirror cache
- On `play`, fetch the Station's MP3 `.pls`; parse `FileN=` entries into an ordered Mirror list. Each successful fetch refreshes a per-station Mirror-list cache in the app cache dir next to the feed.
- If the `.pls` fetch fails, use the cached Mirror list (playback works while somafm.com is down but stream servers are up). No cache and fetch fails -> `offline` (network error) or `unavailable` (HTTP error / empty).

Source: [Native Rust audio engine](../../docs/adr/0001-native-rust-audio.md), [Architecture split](../somafm-app/issues/04-architecture-grilling.md), [Stream errors and reconnect](../somafm-app/issues/08-stream-errors-grilling.md), [Media keys and tray](../somafm-app/issues/07-media-keys-tray-grilling.md), [Test strategy](../somafm-app/issues/11-test-strategy-grilling.md)

## 6. UI

Frontend is a pure view of the stores. Two views: `Grid` and `Player`. No toasts or modals anywhere. Styling: Open Props tokens + scoped Svelte CSS; dark-friendly.

**Grid ("Tile wall")**
- Uniform responsive grid of square artwork Tiles (`largeImage`). Hovering a Tile shows a centred play button.
- Clicking anywhere on a Tile opens the Player screen and starts playing that Station (`play(stationId)`).
- Filter chip row above the Grid: All / Favourites / one chip per genre (genres from splitting the feed's `genre`). Favourites chip state is `favouritesOnly` (persisted); with Favourites on, only Favourites present in the feed show (missing ones hidden, never pruned). Genre chip is view state only. Tiles stay in feed order.
- Active station Tile: accent ring + animated equaliser badge. Badge is a pulsing dot while `connecting`, a warning icon in `error`; ring stays.
- Bottom now-playing bar (slim, persistent on the Grid when a station is loaded, including on launch with the restored last station in Paused state): Station name, artist - title, Stop (Play when stopped, Retry in `error`), and a jump to the Player screen. Shows status text ("Connecting...", "Reconnecting...", or the error message).
- Empty state when `loaded=false`: "Can't reach SomaFM" + Retry.

**Player screen**
- Left: big artwork. Right: Station name, description, listeners, Now playing (artist - title), main button, volume slider + mute, Favourite toggle. Back button top left returns to the Grid; audio continues.
- Main button: Stop while `playing`; spinner while `connecting`; Play when `stopped` (Paused display: status line "Paused"); Retry in `error`.
- Status line: "Connecting..." / "Reconnecting..." / error message per kind / "Paused" when stopped with a loaded station.

**Launch**
- Last Station is restored as `stopped` (Paused), shown in the bottom bar with Play. Nothing autoplays.

Source: [Grid, hover play, player screen](../somafm-app/issues/05-ui-prototype.md), [Stream errors and reconnect](../somafm-app/issues/08-stream-errors-grilling.md), [Settings schema and migration](../somafm-app/issues/09-settings-schema-grilling.md)

## 7. Tray, media keys, OS overlay, close-to-tray

**Close-to-tray**: closing the window hides it; playback continues. The window is hidden, never destroyed (Windows SMTC is bound to its HWND). Quit via the tray menu or Cmd/Ctrl+Q.

**Tray menu** (same on every OS; Rust `tray`, no logic beyond state mapping):
1. Disabled now-playing line: `Station - artist - title`; "Reconnecting..." while reconnecting; the error text in `error`.
2. Play/Stop (label follows state; reads "Play" in `error` and when stopped).
3. Favourites submenu (click a Station to play it).
4. Show/Hide window.
5. Quit.
Left-click toggles the window and right-click opens the menu where the OS supports it. Menu is rebuilt/updated from `player-state` and `settings-changed`. Tray icon: monochrome template image on macOS.

**Media keys / overlay** (`souvlaki`: MPRIS on Linux, SMTC on Windows, MPNowPlaying on macOS)

| Input | Behaviour |
|---|---|
| Play/Pause | pause = stop: drops the connection, keeps the station loaded; Play reconnects to the live stream (first-connect rules). With no loaded station: start `lastStationId`; fresh install with none: do nothing |
| Stop | `stop` |
| Next / Previous | not advertised, ignored |
| Seek, OS volume | ignored; OS volume is not mapped to app volume |

- Overlay playback status: Playing while `playing` or `connecting`; **Paused** when `stopped` with a loaded station and in `error` (never Stopped, so the play key keeps routing to the app).
- Overlay metadata: artist = ICY artist (or the station name if no separator); title = ICY title; album = station name; artwork = Station image URL. No duration/position.

**Per-OS quirks**
- Linux: tray is menu-only via AppIndicator (no click events), so Show/Hide is in the menu. Stock GNOME needs the AppIndicator extension (documented requirement, no workaround); KDE shows it natively. Media controls over MPRIS.
- macOS: dock icon stays visible when the window is hidden; clicking it reopens the window. Menu-bar icon is a template image.
- Windows: SMTC bound to the main window HWND; window is hidden, never destroyed.

Source: [Media keys and tray behaviour](../somafm-app/issues/07-media-keys-tray-grilling.md), [Architecture split](../somafm-app/issues/04-architecture-grilling.md), [Stream errors and reconnect](../somafm-app/issues/08-stream-errors-grilling.md)

## 8. Settings schema v1 & migration

One `tauri-plugin-store` file (`settings.json`, OS app-config dir). Rust `settings` is the only writer. camelCase keys.

| Key | Type | Default | Notes |
|---|---|---|---|
| `schemaVersion` | int | `1` | |
| `favourites` | string[] | `[]` | Station ids, deduplicated set, displayed in feed order, no manual reordering |
| `volume` | float 0.0-1.0 | `0.8` | clamped on load; saves debounced ~500 ms |
| `lastStationId` | string \| null | `null` | written when `play` is accepted for a known Station id (even if the connect then fails), not on stop |
| `favouritesOnly` | bool | `false` | Favourites chip state |

- `lastStationId` is written when a `play` call is accepted and the Station id exists in the feed (not on stop).
- Launch: `lastStationId` restored into `PlayerState` as `stopped`; nothing autoplays.
- Missing from feed: a Favourite not in the current feed stays in the store and is hidden from the Grid; never auto-pruned. A missing `lastStationId` is treated as null at load and not rewritten until the next change.
- Migrations: ordered `vN -> vN+1` functions in Rust run on load, then save.
  - No file: defaults.
  - Unparsable file: rename to `settings.json.bak`, use defaults, log.
  - Invalid value for one key: that key's default, keep the rest.
  - `schemaVersion` newer than known: read known keys, leave the version number alone.
- Saves happen after every persistent change.
- Window geometry: `tauri-plugin-window-state`, own file, outside this schema.

Source: [Settings schema and migration](../somafm-app/issues/09-settings-schema-grilling.md)

## 9. Testing & the `check` command

**`check`** (one local command, e.g. `pnpm check`) runs, in order: `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`, `svelte-check`, `vitest run`. No CI; no coverage threshold.

**Strategy**
- Rust core is a pure reducer (5.4); exhaustive table-driven unit tests, no IO, for: `channels.json` -> Stations (missing/extra fields); `.pls` -> ordered Mirrors; ICY `StreamTitle` split (no ` - `, several ` - `, empty, non-UTF-8, stray quotes); backoff schedule and 2-minute budget; settings migrations, per-key fallback, corrupt file -> `.bak`; every PlayerState transition in 5.5.
- Network faked with a local in-process stub HTTP server (wiremock or axum/hyper) serving fixtures and misbehaving on cue (stall, 404, refuse, mid-stream cut). No trait mock over the HTTP client. `tokio::time::pause()` only in shell tests.
- Fixtures synthetic only: invented Stations/URLs; audio is seconds of generated tone encoded MP3 with ICY framing. No SomaFM bytes in the repo.
- One `#[ignore]` live check fetching real `channels.json` and asserting it parses (`cargo test -- --ignored`, by hand).
- Decode pipeline tested against a sink trait with a null sink counting samples.
- Frontend: Vitest + `@testing-library/svelte`; IPC faked with `@tauri-apps/api/mocks` (`mockIPC` + emitted snapshot events). Covers snapshot -> store mapping, the Favourites chip, and state-driven components (active Tile ring + badge, bottom bar, Player screen connecting / reconnecting / error-with-Retry / paused). No pixel tests.
- No automated E2E or audio-output tests.

### Shared Definition of Done (every slice)
1. `check` passes clean.
2. Core Rust (reducer, parsers, migrations) written test-first (tdd skill).
3. New behaviour tested per the strategy above (views tested alongside).
4. The slice's smoke-checklist lines pass on Linux (Bazzite KDE). macOS and Windows never block a slice.
5. Each ticket's own acceptance criteria are ticked.

### Manual smoke checklist (run on Linux; each ID is owned by one ticket)

| ID | Check | Owner |
|---|---|---|
| S1 | App launches; Grid shows Station Tiles with artwork; hover shows play button; genre chips filter | 02 |
| S2 | Offline with cached feed: Grid renders at once, no banner. Offline with cache deleted: "Can't reach SomaFM" + Retry; reconnecting network then Retry fills the Grid | 02 |
| S3 | Click a Tile: Player screen opens and audio plays; volume slider and mute change loudness; Stop stops (status "Paused") | 03 |
| S4 | Back to Grid while playing: audio continues, Tile has ring + equaliser badge, bottom bar shows Station with Stop and jump to Player | 03 |
| S6 | Artist and title appear on the Player screen and bottom bar and update on track change | 04 |
| S7 | Playing, then disconnect network: status "Reconnecting..." with pulsing dot; reconnect network within 2 min resumes by itself | 05 |
| S8 | Stay disconnected > 2 min (or first play while offline): error "You're offline", warning icon, Retry in Player and bar; Retry after reconnecting plays | 05 |
| S9 | Favourite toggle on the Player screen; Favourites chip shows only Favourites; state survives restart | 06 |
| S10 | Restart: volume and last Station restored; app is Paused, bottom bar shows Play, nothing autoplays | 06 |
| S11 | Window size/position restored after restart | 06 |
| S12 | Close window: it hides, audio continues; tray Show/Hide brings it back | 07 |
| S13 | Tray menu: now-playing line, Play/Stop label follows state, Favourites submenu plays a Station, Quit exits | 07 |
| S14 | Media Play/Pause key stops then reconnects live; overlay shows Paused when stopped; Stop key stops | 08 |
| S15 | KDE media widget shows artist, title, album=Station name, artwork; Next/Previous absent | 08 |
| S16 | Installed binary in `~/.local/bin` launches from the app menu via `.desktop` file with icon; settings written under `dev.defjunx.somafm` dirs | 09 |
| S17 | Full pass of S1-S15 on the installed build, no regressions | 09 |

Source: [Test strategy](../somafm-app/issues/11-test-strategy-grilling.md), [Spec layout and builder ticket breakdown](../somafm-app/issues/13-spec-layout-grilling.md)

## 10. Build & install per platform

Common: Rust stable (rustup), Node LTS, pnpm. Dev: `pnpm tauri dev`. Build: `pnpm tauri build` with per-OS flags. Identifier `dev.defjunx.somafm` never changes (settings survive reinstalls). Update = rebuild and reinstall by hand; no updater plugin. Launch at login is not in v1 (Linux: KDE Autostart).

| OS | Build | Install | Notes |
|---|---|---|---|
| Linux (verified, Bazzite 44 KDE, immutable Fedora) | Inside a Fedora 44 `toolbox` with rustup and `sudo dnf install webkit2gtk4.1-devel libappindicator-gtk3-devel alsa-lib-devel librsvg2-devel openssl-devel gcc gcc-c++ make pkgconf-pkg-config` (plus `curl wget file` if missing). Never layer packages on the host. `pnpm tauri build --no-bundle` | Copy the binary to `~/.local/bin`; install repo `.desktop` template to `~/.local/share/applications` and icon to `~/.local/share/icons` | Host has runtime libs (`webkit2gtk4.1`, `libappindicator-gtk3`, `alsa-lib`). No AppImage, no rpm. KDE shows tray natively; stock GNOME needs the AppIndicator extension |
| macOS (unverified) | `pnpm tauri build --bundles app` (ad-hoc signed) | Copy `.app` to `/Applications` | No dmg. `xattr -dr com.apple.quarantine` only if the `.app` was copied from another machine |
| Windows (unverified) | `pnpm tauri build --bundles nsis` | Run per-user `-setup.exe`, no admin | No MSI/WiX. WebView2 ships with Win 10/11. One-time SmartScreen warning (unsigned) |

Repo ships: original app icon SVG, monochrome tray icon, a `.desktop` template.

Source: [Local build and install per platform](../somafm-app/issues/12-local-build-install-grilling.md)

## 11. Out of scope

- Distribution of any kind (releases, installers, stores).
- Accounts, scrobbling, recording or downloading streams, SomaFM donation/shop flows.
- Launch at login; auto-update.
- Sleep timer (none anywhere: not in PlayerState, commands, UI or tray).
- AAC / HE-AAC streams (`aac`, `aacp`); stream quality choice (v1 is MP3 only); Next/Previous; seek; mapping OS volume to app volume.
- Automated E2E and audio-output tests; CI; coverage gates.
- Bundled or committed SomaFM artwork, branding or feed snapshots.
- Verifying macOS/Windows builds (steps are documented only).

Source: [Map: out of scope](../somafm-app/map.md), [Sleep timer UX](../somafm-app/issues/10-sleep-timer-grilling.md), [Local build and install](../somafm-app/issues/12-local-build-install-grilling.md)

## Open questions

None. Resolved with the user, 2026-10-16:

1. v1 is MP3 only (revised 2026-10-16): no stream quality choice. MP3 plays the feed's `format == "mp3"` playlist as-is (about 256-320 kbps).
2. `lastStationId` is written when `play` is accepted for a known Station id, so a failed first connect still restores that Station as Paused.
