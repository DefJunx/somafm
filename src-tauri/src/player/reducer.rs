//! Pure core: `reduce(core, event, now) -> (core, effects)`. No IO. The shell
//! executes the effects and feeds results back as events.

use std::time::Instant;

use super::{ErrorKind, PlayerState, Status};

/// Public state plus the bookkeeping the reducer needs. `attempt` identifies the
/// current connection attempt; events from older attempts are ignored.
#[derive(Clone, Debug, PartialEq)]
pub struct Core {
    pub state: PlayerState,
    pub attempt: u64,
    phase: Phase,
}

#[derive(Clone, Debug, PartialEq)]
enum Phase {
    Idle,
    Resolving,
    /// First-connect pass over `mirrors` Mirrors; `next` is the next index to try.
    Trying {
        mirrors: usize,
        next: usize,
        refused: bool,
    },
    Live,
}

impl Core {
    pub fn new(state: PlayerState) -> Self {
        Self {
            state,
            attempt: 0,
            phase: Phase::Idle,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    PlayRequested {
        station_id: String,
    },
    StopRequested,
    MirrorsResolved {
        attempt: u64,
        count: usize,
    },
    ResolveFailed {
        attempt: u64,
        kind: ErrorKind,
    },
    Connected {
        attempt: u64,
    },
    /// `kind` is `Offline` (network level) or `Unavailable` (refused / nothing sent).
    ConnectFailed {
        attempt: u64,
        kind: ErrorKind,
    },
    DecodeFailed {
        attempt: u64,
    },
    StreamDropped {
        attempt: u64,
    },
    /// A raw ICY `StreamTitle` arrived on the stream.
    TitleChanged {
        attempt: u64,
        raw: String,
    },
    SetVolume(f32),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Effect {
    Resolve {
        attempt: u64,
        station_id: String,
    },
    ConnectMirror {
        attempt: u64,
        index: usize,
    },
    Disconnect,
    ApplyVolume(f32),
    /// Emitted exactly once whenever the public state changed.
    EmitSnapshot,
}

fn fail(core: &mut Core, kind: ErrorKind, fx: &mut Vec<Effect>) {
    core.phase = Phase::Idle;
    core.state.status = Status::Error {
        error_kind: kind,
        message: kind.message().into(),
    };
    fx.push(Effect::Disconnect);
}

fn start(core: &mut Core, fx: &mut Vec<Effect>) {
    core.attempt += 1;
    core.phase = Phase::Resolving;
    core.state.status = Status::Connecting {
        reconnecting: false,
    };
    fx.push(Effect::Disconnect);
    fx.push(Effect::Resolve {
        attempt: core.attempt,
        station_id: core.state.station_id.clone().unwrap_or_default(),
    });
}

fn event_gen(e: &Event) -> Option<u64> {
    match e {
        Event::MirrorsResolved { attempt, .. }
        | Event::ResolveFailed { attempt, .. }
        | Event::Connected { attempt }
        | Event::ConnectFailed { attempt, .. }
        | Event::DecodeFailed { attempt }
        | Event::StreamDropped { attempt }
        | Event::TitleChanged { attempt, .. } => Some(*attempt),
        _ => None,
    }
}

/// `now` is unused until reconnect budgets land (slice 05); it is part of the
/// signature so time stays data.
pub fn reduce(mut core: Core, event: Event, _now: Instant) -> (Core, Vec<Effect>) {
    let before = core.state.clone();
    let mut fx = Vec::new();

    if event_gen(&event).is_some_and(|g| g != core.attempt) {
        return (core, fx);
    }

    match event {
        Event::PlayRequested { station_id } => {
            if core.state.station_id.as_deref() != Some(&station_id) {
                core.state.now_playing = None;
            }
            core.state.station_id = Some(station_id);
            start(&mut core, &mut fx);
        }
        Event::StopRequested => {
            core.attempt += 1;
            core.phase = Phase::Idle;
            core.state.status = Status::Stopped;
            core.state.now_playing = None;
            fx.push(Effect::Disconnect);
        }
        Event::MirrorsResolved { count, .. } if core.phase == Phase::Resolving => {
            if count == 0 {
                fail(&mut core, ErrorKind::Unavailable, &mut fx);
            } else {
                core.phase = Phase::Trying {
                    mirrors: count,
                    next: 1,
                    refused: false,
                };
                fx.push(Effect::ConnectMirror {
                    attempt: core.attempt,
                    index: 0,
                });
            }
        }
        Event::ResolveFailed { kind, .. } if core.phase == Phase::Resolving => {
            fail(&mut core, kind, &mut fx);
        }
        Event::ConnectFailed { kind, .. } => {
            if let Phase::Trying {
                mirrors,
                next,
                refused,
            } = core.phase
            {
                let refused = refused || kind != ErrorKind::Offline;
                if next < mirrors {
                    core.phase = Phase::Trying {
                        mirrors,
                        next: next + 1,
                        refused,
                    };
                    fx.push(Effect::ConnectMirror {
                        attempt: core.attempt,
                        index: next,
                    });
                } else {
                    let kind = if refused {
                        ErrorKind::Unavailable
                    } else {
                        ErrorKind::Offline
                    };
                    fail(&mut core, kind, &mut fx);
                }
            }
        }
        Event::Connected { .. } if matches!(core.phase, Phase::Trying { .. }) => {
            core.phase = Phase::Live;
            core.state.status = Status::Playing;
        }
        Event::DecodeFailed { .. } if matches!(core.phase, Phase::Trying { .. } | Phase::Live) => {
            fail(&mut core, ErrorKind::Unsupported, &mut fx);
        }
        // Minimal ending until reconnect lands in slice 05.
        Event::StreamDropped { .. } if core.phase == Phase::Live => {
            fail(&mut core, ErrorKind::Unavailable, &mut fx);
        }
        Event::TitleChanged { raw, .. }
            if matches!(core.phase, Phase::Trying { .. } | Phase::Live) =>
        {
            if let Some(np) = super::icy::now_playing(&raw) {
                core.state.now_playing = Some(np);
            }
        }
        Event::SetVolume(v) if v.is_finite() => {
            core.state.volume = v.clamp(0.0, 1.0);
            fx.push(Effect::ApplyVolume(core.state.volume));
        }
        _ => {}
    }

    if core.state != before {
        fx.push(Effect::EmitSnapshot);
    }
    (core, fx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ErrorKind::*;

    fn now() -> Instant {
        Instant::now()
    }

    fn run(core: Core, events: Vec<Event>) -> (Core, Vec<Effect>) {
        let mut all = Vec::new();
        let mut core = core;
        for e in events {
            let (c, fx) = reduce(core, e, now());
            core = c;
            all.extend(fx);
        }
        (core, all)
    }

    fn play(id: &str) -> Event {
        Event::PlayRequested {
            station_id: id.into(),
        }
    }

    fn fresh() -> Core {
        Core::new(PlayerState::default())
    }

    /// Core mid first-connect pass: station "a", attempt 1, `n` Mirrors resolved.
    fn trying(n: usize) -> Core {
        run(
            fresh(),
            vec![
                play("a"),
                Event::MirrorsResolved {
                    attempt: 1,
                    count: n,
                },
            ],
        )
        .0
    }

    fn live() -> Core {
        reduce(trying(2), Event::Connected { attempt: 1 }, now()).0
    }

    fn err(kind: ErrorKind) -> Status {
        Status::Error {
            error_kind: kind,
            message: kind.message().into(),
        }
    }

    #[test]
    fn play_starts_connecting_and_resolves() {
        let (c, fx) = reduce(fresh(), play("a"), now());
        assert_eq!(c.state.station_id.as_deref(), Some("a"));
        assert_eq!(
            c.state.status,
            Status::Connecting {
                reconnecting: false
            }
        );
        assert_eq!(
            fx,
            vec![
                Effect::Disconnect,
                Effect::Resolve {
                    attempt: 1,
                    station_id: "a".into(),
                },
                Effect::EmitSnapshot
            ]
        );
    }

    #[test]
    fn resolved_connects_first_mirror_then_plays() {
        let (c, fx) = reduce(
            reduce(fresh(), play("a"), now()).0,
            Event::MirrorsResolved {
                attempt: 1,
                count: 3,
            },
            now(),
        );
        assert_eq!(
            fx,
            vec![Effect::ConnectMirror {
                attempt: 1,
                index: 0
            }]
        );
        let (c, fx) = reduce(c, Event::Connected { attempt: 1 }, now());
        assert_eq!(c.state.status, Status::Playing);
        assert_eq!(fx, vec![Effect::EmitSnapshot]);
    }

    #[test]
    fn first_pass_walks_mirrors_in_order_then_errors() {
        let (c, fx) = run(
            trying(3),
            vec![
                Event::ConnectFailed {
                    attempt: 1,
                    kind: Offline,
                },
                Event::ConnectFailed {
                    attempt: 1,
                    kind: Offline,
                },
            ],
        );
        assert_eq!(
            fx,
            vec![
                Effect::ConnectMirror {
                    attempt: 1,
                    index: 1
                },
                Effect::ConnectMirror {
                    attempt: 1,
                    index: 2
                }
            ]
        );
        assert_eq!(
            c.state.status,
            Status::Connecting {
                reconnecting: false
            }
        );
        let (c, fx) = reduce(
            c,
            Event::ConnectFailed {
                attempt: 1,
                kind: Offline,
            },
            now(),
        );
        assert_eq!(c.state.status, err(Offline));
        assert_eq!(fx, vec![Effect::Disconnect, Effect::EmitSnapshot]);
    }

    #[test]
    fn error_kind_after_pass_table() {
        // (per-Mirror failures, final error kind)
        let cases = [
            (vec![Offline, Offline], Offline),
            (vec![Unavailable, Unavailable], Unavailable),
            (vec![Offline, Unavailable], Unavailable),
            (vec![Unavailable, Offline], Unavailable),
        ];
        for (fails, want) in cases {
            let events = fails
                .iter()
                .map(|&kind| Event::ConnectFailed { attempt: 1, kind })
                .collect();
            let (c, _) = run(trying(2), events);
            assert_eq!(c.state.status, err(want), "{fails:?}");
        }
    }

    #[test]
    fn a_later_mirror_can_succeed() {
        let (c, _) = run(
            trying(2),
            vec![
                Event::ConnectFailed {
                    attempt: 1,
                    kind: Unavailable,
                },
                Event::Connected { attempt: 1 },
            ],
        );
        assert_eq!(c.state.status, Status::Playing);
    }

    #[test]
    fn resolve_failures() {
        let c = reduce(fresh(), play("a"), now()).0;
        for kind in [Offline, Unavailable] {
            let (c, _) = reduce(c.clone(), Event::ResolveFailed { attempt: 1, kind }, now());
            assert_eq!(c.state.status, err(kind));
        }
        let (c, _) = reduce(
            c,
            Event::MirrorsResolved {
                attempt: 1,
                count: 0,
            },
            now(),
        );
        assert_eq!(c.state.status, err(Unavailable));
    }

    #[test]
    fn decode_failure_is_unsupported_while_connecting_or_playing() {
        for c in [trying(1), live()] {
            let (c, fx) = reduce(c, Event::DecodeFailed { attempt: 1 }, now());
            assert_eq!(c.state.status, err(Unsupported));
            assert!(fx.contains(&Effect::Disconnect));
        }
    }

    #[test]
    fn stream_drop_while_playing_ends_in_error_for_now() {
        let (c, _) = reduce(live(), Event::StreamDropped { attempt: 1 }, now());
        assert_eq!(c.state.status, err(Unavailable));
    }

    #[test]
    fn stop_keeps_station_and_clears_now_playing() {
        let mut c = live();
        c.state.now_playing = Some(super::super::NowPlaying {
            raw: "a - b".into(),
            artist: Some("a".into()),
            title: "b".into(),
        });
        let (c, fx) = reduce(c, Event::StopRequested, now());
        assert_eq!(c.state.status, Status::Stopped);
        assert_eq!(c.state.station_id.as_deref(), Some("a"));
        assert_eq!(c.state.now_playing, None);
        assert_eq!(fx, vec![Effect::Disconnect, Effect::EmitSnapshot]);
    }

    #[test]
    fn stop_when_already_stopped_emits_nothing() {
        let (_, fx) = reduce(fresh(), Event::StopRequested, now());
        assert_eq!(fx, vec![Effect::Disconnect]);
    }

    #[test]
    fn stale_events_are_ignored() {
        let (c, _) = run(
            fresh(),
            vec![play("a"), play("b"), Event::StopRequested, play("c")],
        );
        assert_eq!(c.attempt, 4);
        for e in [
            Event::MirrorsResolved {
                attempt: 1,
                count: 2,
            },
            Event::Connected { attempt: 3 },
            Event::ConnectFailed {
                attempt: 2,
                kind: Offline,
            },
            Event::DecodeFailed { attempt: 1 },
            Event::StreamDropped { attempt: 1 },
            Event::ResolveFailed {
                attempt: 3,
                kind: Offline,
            },
        ] {
            let (c2, fx) = reduce(c.clone(), e, now());
            assert_eq!(c2, c);
            assert!(fx.is_empty());
        }
    }

    #[test]
    fn switching_station_clears_now_playing_retry_keeps_it() {
        let mut c = live();
        c.state.now_playing = Some(super::super::NowPlaying {
            raw: "x".into(),
            artist: None,
            title: "x".into(),
        });
        let (same, _) = reduce(c.clone(), play("a"), now());
        assert!(same.state.now_playing.is_some());
        let (other, _) = reduce(c, play("b"), now());
        assert!(other.state.now_playing.is_none());
        assert_eq!(other.state.station_id.as_deref(), Some("b"));
    }

    #[test]
    fn title_changed_sets_now_playing_once() {
        let title = |raw: &str| Event::TitleChanged {
            attempt: 1,
            raw: raw.into(),
        };
        let (c, fx) = reduce(live(), title("A - B"), now());
        let np = c.state.now_playing.clone().unwrap();
        assert_eq!((np.artist.as_deref(), np.title.as_str()), (Some("A"), "B"));
        assert_eq!(fx, vec![Effect::EmitSnapshot]);
        // Same title again: no snapshot.
        let (c2, fx) = reduce(c.clone(), title("A - B"), now());
        assert_eq!(c2, c);
        assert!(fx.is_empty());
        // Empty title: no change.
        let (c3, fx) = reduce(c.clone(), title("  "), now());
        assert_eq!(c3, c);
        assert!(fx.is_empty());
        // New title replaces it.
        let (c4, fx) = reduce(c, title("Just text"), now());
        assert_eq!(c4.state.now_playing.unwrap().artist, None);
        assert_eq!(fx, vec![Effect::EmitSnapshot]);
    }

    #[test]
    fn stale_or_idle_title_is_ignored() {
        let stale = Event::TitleChanged {
            attempt: 9,
            raw: "A - B".into(),
        };
        let (c, fx) = reduce(live(), stale, now());
        assert_eq!(c, live());
        assert!(fx.is_empty());
        let stopped = reduce(live(), Event::StopRequested, now()).0;
        let late = Event::TitleChanged {
            attempt: stopped.attempt,
            raw: "A - B".into(),
        };
        let (c, fx) = reduce(stopped.clone(), late, now());
        assert_eq!(c, stopped);
        assert!(fx.is_empty());
    }

    #[test]
    fn now_playing_survives_stream_error_but_not_station_change() {
        let (c, _) = run(
            live(),
            vec![
                Event::TitleChanged {
                    attempt: 1,
                    raw: "A - B".into(),
                },
                Event::StreamDropped { attempt: 1 },
            ],
        );
        assert!(c.state.now_playing.is_some());
    }

    #[test]
    fn volume_is_clamped_applied_and_snapshotted() {
        let cases = [(0.3, 0.3), (-1.0, 0.0), (5.0, 1.0)];
        for (input, want) in cases {
            let (c, fx) = reduce(fresh(), Event::SetVolume(input), now());
            assert_eq!(c.state.volume, want);
            assert_eq!(fx, vec![Effect::ApplyVolume(want), Effect::EmitSnapshot]);
        }
        let (c, fx) = reduce(fresh(), Event::SetVolume(f32::NAN), now());
        assert_eq!(c, fresh());
        assert!(fx.is_empty());
    }

    #[test]
    fn volume_same_value_applies_without_snapshot() {
        let (_, fx) = reduce(fresh(), Event::SetVolume(0.8), now());
        assert_eq!(fx, vec![Effect::ApplyVolume(0.8)]);
    }

    #[test]
    fn play_after_error_reruns_first_connect() {
        let errored = reduce(
            trying(1),
            Event::ConnectFailed {
                attempt: 1,
                kind: Offline,
            },
            now(),
        )
        .0;
        let (c, fx) = reduce(errored, play("a"), now());
        assert_eq!(
            c.state.status,
            Status::Connecting {
                reconnecting: false
            }
        );
        assert!(matches!(fx[1], Effect::Resolve { attempt: 2, .. }));
    }

    #[test]
    fn exactly_one_snapshot_per_change() {
        let seq = vec![
            play("a"),
            Event::MirrorsResolved {
                attempt: 1,
                count: 1,
            },
            Event::Connected { attempt: 1 },
            Event::SetVolume(0.5),
            Event::StopRequested,
        ];
        let mut core = fresh();
        for e in seq {
            let before = core.state.clone();
            let (c, fx) = reduce(core, e, now());
            let n = fx.iter().filter(|f| **f == Effect::EmitSnapshot).count();
            assert_eq!(n, usize::from(c.state != before));
            core = c;
        }
    }

    #[test]
    fn old_attempt_events_after_stop_or_restart_change_nothing() {
        let switched = reduce(live(), play("a"), now()).0;
        let stopped = reduce(live(), Event::StopRequested, now()).0;
        for core in [switched, stopped] {
            for e in [
                Event::Connected { attempt: 1 },
                Event::DecodeFailed { attempt: 1 },
                Event::StreamDropped { attempt: 1 },
            ] {
                let (after, fx) = reduce(core.clone(), e, now());
                assert_eq!(after, core);
                assert!(fx.is_empty(), "no snapshot or effects for stale events");
            }
        }
    }
}
