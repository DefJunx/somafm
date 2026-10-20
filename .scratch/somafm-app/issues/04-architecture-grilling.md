# 04 Architecture split: Rust vs frontend

Type: grilling
Status: resolved
Blocked by:

## Question

Given the research, what lives in Rust vs the Svelte frontend? Decide: where station/now-playing fetches happen (webview vs Rust, CORS), audio engine location, state ownership (player state, volume, quality), persistence (favourites, settings), tray and media-key integration, and the module/seam boundaries an implementer follows.

## Answer

Resolved by grilling with the user (2026-10-16). ADR: [Native Rust audio engine](../../../docs/adr/0001-native-rust-audio.md).

- **Audio**: native Rust. `reqwest` streams to an HTTP→`MediaSource` adapter, `symphonia` decodes (MP3, AAC-LC over ADTS) and `rodio` plays. ICY metadata is parsed inline. If a stream fails, the player tries the next Mirror listed in the `.pls`. HE-AAC (`aacp`) is not supported by symphonia, so it is not offered. Mixed content is a non-issue: every stream URL is HTTPS.
- **Feed**: Rust fetches `channels.json` at startup and every 5 min while the window is visible, and keeps the last good copy in the app cache dir as the offline fallback. Tile images load straight from SomaFM's URLs in the webview. Nothing is committed to the repo.
- **State**: Rust owns the only `PlayerState`: active station, status `stopped | connecting | playing | error`, volume, quality, and now playing. The webview, tray and media keys all call the same commands: `get_stations`, `play(stationId)`, `stop`, `set_volume`, `set_quality`, `toggle_favourite`. Every change is broadcast as one full-snapshot `player-state` event. The frontend is a pure view.
- **Now playing**: ICY `StreamTitle` for the active station. `lastPlaying` from the feed is available to the Grid if the UI prototype wants it.
- **Stream quality**: one global setting, MP3 (default) or AAC 128. Each station's `playlists` array is filtered to that format.
- **Persistence**: `tauri-plugin-store`, one JSON file in the OS app-config dir holding favourites, volume, quality and the last station.
- **Window**: closing the window hides it to the tray and playback continues. Quit is in the tray menu or Cmd/Ctrl+Q.
- **Modules**:
  - Rust: `feed` (fetch/cache/parse, `.pls` resolution), `player` (engine thread, stream adapter, ICY, Mirror failover), `media_controls` (souvlaki), `tray`, `settings` (store wrapper), `commands` (thin IPC). `commands` and `tray` hold no logic.
  - Frontend: `lib/api.ts` (typed invoke/listen), a `playerState` store fed by the event, and two views, `Grid` and `Player`.

Note (2026-10-16): the sleep timer was later ruled out of scope; references to it were removed above.
