# 11 Test strategy

Type: grilling
Status: resolved
Blocked by: 

## Question

How is this app tested? Decide: what is unit-tested in Rust (feed parsing, .pls resolution, Mirror failover and backoff, ICY parsing, PlayerState transitions) and how time and network are faked; what is tested in the frontend (stores, views) and with which runner; whether there are any end-to-end or audio-output tests; and what counts as passing before a builder ticket is done.

## Answer

Resolved with the user, 2026-10-16.

- **Rust core is a pure reducer.** PlayerState transitions, Mirror failover, backoff, and the 10s connect / 15s stall timeouts are a pure function `(state, event, now) -> (state, effects)`. Effects (connect to Mirror N, arm a timer, emit snapshot) are executed by a thin tokio/HTTP/rodio shell. Time is data passed into the core; no `Clock` trait. `tokio::time::pause()` is used only in the few shell tests.
- **Exhaustive table-driven unit tests** (no IO): `channels.json` -> Stations (missing/extra fields); `.pls` -> ordered Mirror list; ICY `StreamTitle` artist/title split (no ` - `, several ` - `, empty, non-UTF-8, stray quotes); backoff schedule and 2-minute reconnect budget; settings migrations, lenient per-key fallback, corrupt file -> `.bak`; every PlayerState transition from the stream-errors ticket.
- **Network faked with a local in-process stub HTTP server** (e.g. `wiremock` or an axum/hyper stub) serving fixtures and misbehaving on cue: stall, 404, refuse, mid-stream cut. Exercises the real timeouts and ICY framing. No trait mock over the HTTP client.
- **Fixtures are synthetic only.** Invented stations and URLs; audio fixtures are seconds of generated tone encoded MP3/AAC with ICY metadata framing. No SomaFM bytes in the repo (consistent with the policy ticket).
- **One opt-in live check**: a `#[ignore]` test that fetches the real `channels.json` and asserts it parses, run by hand (`cargo test -- --ignored`) to catch schema drift. No live stream tests.
- **Frontend**: Vitest + `@testing-library/svelte`, Tauri IPC faked with `@tauri-apps/api/mocks` (`mockIPC` + emitted snapshot events). Covers snapshot -> store mapping, the favourites filter chip, and a small set of state-driven component tests (active tile ring + equaliser badge, now-playing bar, player screen connecting / reconnecting / error-with-Retry / paused). No visual or pixel tests.
- **No automated E2E or audio-output tests.** `tauri-driver` lacks macOS support and speaker checks are flaky. Decode pipeline is tested to a sink trait with a null sink counting samples (MP3/AAC fixture -> PCM flowed). Tray, media keys and real audio are covered by a short manual smoke checklist run per platform in use; where it lives is for the Spec layout ticket.
- **Definition of done for a builder ticket**: `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`, `svelte-check`, `vitest run` all pass, wired into one local `check` command; new Rust core/parser behaviour written test-first (tdd skill), views tested alongside; UI/tray changes walked through the relevant smoke-checklist lines.
- **No coverage threshold.** The test-first rule and the exhaustive unit list stand in for a percentage.
- **No CI.** Personal-use app; the local `check` command is the gate.
