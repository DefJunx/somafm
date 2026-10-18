# 05 Stream errors & reconnect

Status: resolved
Blocked by: 03

## Goal
Complete the reconnect/error state machine with timeouts and backoff, and the error/connecting UI with Retry.

## Spec sections
- [§5 Playback](../spec.md#5-playback) (5.5, 5.6 fallback)
- [§3 Architecture](../spec.md#3-architecture-rust-vs-frontend) (`Status`, `ErrorKind`)
- [§6 UI](../spec.md#6-ui) (Player screen, Grid badge, bottom bar)

## Acceptance criteria
- [ ] Reducer implements 5.5 exactly: first-connect single pass; drop -> immediate same-Mirror retry, then remaining Mirrors, then cycling with backoff 2/4/8/16/30 s, give up to `error` 2 min after the drop; Mirror order starts with last-working Mirror; all transitions table-tested test-first.
- [ ] 10 s per-Mirror connect timeout and 15 s no-bytes stall while `playing` verified against the stub server (stall, 404, refuse, mid-stream cut); short underruns cause no status change.
- [ ] Error kinds `offline` / `unavailable` / `unsupported` classified correctly; cached Mirror list used when the `.pls` fetch fails.
- [ ] Player screen: spinner + "Connecting..." / "Reconnecting...", error messages per kind, Retry button (calls `play` for the loaded Station). Grid: pulsing dot while `connecting`, warning icon in `error`, ring kept; bottom bar shows status text with Retry. No toasts or modals. Vitest covers these states.

## Smoke checklist
S7, S8. Shared Definition of Done in [§9 Testing & DoD](../spec.md#9-testing--the-check-command) applies.

## Notes
- No auto-resume and no OS network watching; only user Retry.
