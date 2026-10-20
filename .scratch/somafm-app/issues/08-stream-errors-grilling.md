# 08 Stream errors and reconnect

Type: grilling
Status: resolved
Blocked by: 04

## Question

How does the player handle failure? Decide: retry and backoff when a stream drops mid-play, the order for failing over between Mirrors and when to give up, what the `error` status carries and how the UI shows it, buffering/connecting feedback, and startup with no network (cached feed, play attempts while offline, recovery once the network returns).

## Answer

Resolved by grilling with the user (2026-10-16).

- **Drop mid-play** (stream was `playing`): reconnect at once to the same Mirror, then walk the remaining Mirrors. After one full pass, keep cycling with exponential backoff (2s, 4s, 8s, 16s, capped at 30s). Give up and go to `error` 2 minutes after the drop.
- **First connect** (user pressed Play from a tile, button, media key or tray): one pass over the Mirrors, no backoff cycling, then `error`.
- **Mirror order**: `.pls` order, starting with the Mirror that last worked for that station this session. No format fallback: Stream quality stays as chosen.
- **Timeouts**: 10s connect timeout per Mirror (TCP, TLS and first audio bytes). While `playing`, 15s with no bytes received counts as a drop. Shorter decoder underruns are silence only, with no status change.
- **Status model**: still `stopped | connecting | playing | error`. `connecting` carries `reconnecting: bool`. `error` carries `{ kind: "offline" | "unavailable" | "unsupported", message }`: `offline` = no Mirror reachable at the network level; `unavailable` = servers answered but refused or sent nothing (HTTP error, empty `.pls`); `unsupported` = decode failed. In `error` the station stays loaded (like Paused), and Play retries. The last Now playing title is kept through reconnects and cleared on `stop` or a station change.
- **Recovery**: no auto-resume and no OS network-change watching. The user retries via Play/Retry (UI, media key or tray).
- **Offline startup**: a cached `channels.json` renders the Grid at once with no banner. A failed feed refresh retries on backoff (30s, 60s, … up to the normal 5 min). Tiles whose image fails show the station name on a plain Open Props surface. With no cache and no network, the Grid shows the empty state "Can't reach SomaFM" with Retry, and keeps retrying on the same backoff.
- **Mirror-list cache**: each station's resolved `.pls` Mirror list is cached in the app cache dir next to the feed and refreshed on every `.pls` fetch, so playback works while somafm.com is down but the stream servers are up.
- **UI**:
  - Player screen: during `connecting`, the main button shows a spinner and the status line reads "Connecting…" or "Reconnecting…". During `error`, the status line shows a message per kind ("You're offline", "Station unavailable", "Can't play this stream"), and the main button becomes Retry.
  - Grid: the Active tile keeps its ring. The equaliser badge becomes a pulsing dot while `connecting` and a warning icon in `error`. The bottom bar shows the same status text, with Retry in place of Stop.
  - Tray and overlay: the tray now-playing line shows "Reconnecting…" or the error text, and Play/Stop reads "Play" in `error`. The OS overlay reports Paused.
  - No toasts or modals.

Note (2026-10-16): the sleep timer was later ruled out of scope; references to it were removed above.
