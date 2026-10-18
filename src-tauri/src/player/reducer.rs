//! Pure core: `reduce(core, event, now) -> (core, effects)`. No IO. The shell
//! executes the effects and feeds results back as events.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use super::{ErrorKind, PlayerState, Status};

/// Public state plus the bookkeeping the reducer needs. `attempt` identifies the
/// current connection attempt; events from older attempts are ignored.
#[derive(Clone, Debug, PartialEq)]
pub struct Core {
    pub state: PlayerState,
    pub attempt: u64,
    phase: Phase,
    /// Per Station: the Mirror index that last connected this session.
    last_working: HashMap<String, usize>,
}

/// Total time spent trying to get a dropped stream back before giving up.
const RECONNECT_BUDGET: Duration = Duration::from_secs(120);
/// Waits between full Mirror passes while reconnecting; the last one repeats.
const BACKOFF_SECS: [u64; 5] = [2, 4, 8, 16, 30];

#[derive(Clone, Debug, PartialEq)]
enum Phase {
    Idle,
    Resolving,
    /// A pass over the Mirrors in `order`; `pos` is the next entry to try.
    /// `deadline` is set only while reconnecting after a drop.
    Trying {
        order: Vec<usize>,
        pos: usize,
        refused: bool,
        deadline: Option<Instant>,
        cycles: usize,
    },
    /// Reconnecting: a full pass failed, waiting for the backoff timer.
    Waiting {
        order: Vec<usize>,
        refused: bool,
        deadline: Instant,
        cycles: usize,
    },
    /// Playing from Mirror `mirror` of `mirrors`.
    Live {
        mirror: usize,
        mirrors: usize,
    },
}

impl Core {
    pub fn new(state: PlayerState) -> Self {
        Self {
            state,
            attempt: 0,
            phase: Phase::Idle,
            last_working: HashMap::new(),
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
    /// The backoff timer armed by `Effect::ArmTimer` fired.
    TimerFired {
        attempt: u64,
    },
    /// The stream ended or went silent for the stall limit after playing started.
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
    /// Dispatch `Event::TimerFired` for `attempt` at `at`; replaces any armed timer.
    ArmTimer {
        attempt: u64,
        at: Instant,
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
        | Event::TimerFired { attempt }
        | Event::StreamDropped { attempt }
        | Event::TitleChanged { attempt, .. } => Some(*attempt),
        _ => None,
    }
}

/// Mirror indices in connect order: `first` (if valid) then the rest in `.pls` order.
fn mirror_order(mirrors: usize, first: Option<usize>) -> Vec<usize> {
    let first = first.filter(|&f| f < mirrors);
    first
        .into_iter()
        .chain((0..mirrors).filter(|&i| Some(i) != first))
        .collect()
}

fn exhausted_kind(refused: bool) -> ErrorKind {
    if refused {
        ErrorKind::Unavailable
    } else {
        ErrorKind::Offline
    }
}

pub fn reduce(mut core: Core, event: Event, now: Instant) -> (Core, Vec<Effect>) {
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
                let last = core
                    .state
                    .station_id
                    .as_ref()
                    .and_then(|id| core.last_working.get(id).copied());
                let order = mirror_order(count, last);
                fx.push(Effect::ConnectMirror {
                    attempt: core.attempt,
                    index: order[0],
                });
                core.phase = Phase::Trying {
                    order,
                    pos: 1,
                    refused: false,
                    deadline: None,
                    cycles: 0,
                };
            }
        }
        Event::ResolveFailed { kind, .. } if core.phase == Phase::Resolving => {
            fail(&mut core, kind, &mut fx);
        }
        Event::ConnectFailed { kind, .. } => {
            if let Phase::Trying {
                order,
                pos,
                refused,
                deadline,
                cycles,
            } = core.phase.clone()
            {
                let refused = refused || kind != ErrorKind::Offline;
                if deadline.is_some_and(|d| now >= d) {
                    fail(&mut core, exhausted_kind(refused), &mut fx);
                } else if pos < order.len() {
                    fx.push(Effect::ConnectMirror {
                        attempt: core.attempt,
                        index: order[pos],
                    });
                    core.phase = Phase::Trying {
                        order,
                        pos: pos + 1,
                        refused,
                        deadline,
                        cycles,
                    };
                } else if let Some(deadline) = deadline {
                    // Full pass failed while reconnecting: back off, within the budget.
                    let delay = BACKOFF_SECS[cycles.min(BACKOFF_SECS.len() - 1)];
                    fx.push(Effect::ArmTimer {
                        attempt: core.attempt,
                        at: (now + Duration::from_secs(delay)).min(deadline),
                    });
                    core.phase = Phase::Waiting {
                        order,
                        refused,
                        deadline,
                        cycles: cycles + 1,
                    };
                } else {
                    fail(&mut core, exhausted_kind(refused), &mut fx);
                }
            }
        }
        Event::TimerFired { .. } => {
            if let Phase::Waiting {
                order,
                refused,
                deadline,
                cycles,
            } = core.phase.clone()
            {
                if now >= deadline {
                    fail(&mut core, exhausted_kind(refused), &mut fx);
                } else {
                    fx.push(Effect::ConnectMirror {
                        attempt: core.attempt,
                        index: order[0],
                    });
                    core.phase = Phase::Trying {
                        order,
                        pos: 1,
                        refused,
                        deadline: Some(deadline),
                        cycles,
                    };
                }
            }
        }
        Event::Connected { .. } => {
            if let Phase::Trying { order, pos, .. } = core.phase.clone() {
                let mirror = order[pos - 1];
                if let Some(id) = core.state.station_id.clone() {
                    core.last_working.insert(id, mirror);
                }
                core.phase = Phase::Live {
                    mirror,
                    mirrors: order.len(),
                };
                core.state.status = Status::Playing;
            }
        }
        Event::DecodeFailed { .. }
            if matches!(core.phase, Phase::Trying { .. } | Phase::Live { .. }) =>
        {
            fail(&mut core, ErrorKind::Unsupported, &mut fx);
        }
        Event::StreamDropped { .. } => {
            if let Phase::Live { mirror, mirrors } = core.phase {
                // Same Mirror at once, then the remaining ones in `.pls` order.
                let order = mirror_order(mirrors, Some(mirror));
                fx.push(Effect::ConnectMirror {
                    attempt: core.attempt,
                    index: mirror,
                });
                core.phase = Phase::Trying {
                    order,
                    pos: 1,
                    refused: false,
                    deadline: Some(now + RECONNECT_BUDGET),
                    cycles: 0,
                };
                core.state.status = Status::Connecting { reconnecting: true };
            }
        }
        Event::TitleChanged { raw, .. }
            if matches!(core.phase, Phase::Trying { .. } | Phase::Live { .. }) =>
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
    use std::time::Duration;

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

    const DEADLINE_SECS: u64 = 120;

    fn t0() -> Instant {
        static T0: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
        *T0.get_or_init(Instant::now)
    }

    fn at(secs: u64) -> Instant {
        t0() + Duration::from_secs(secs)
    }

    fn fail_at(core: Core, kind: ErrorKind, secs: u64) -> (Core, Vec<Effect>) {
        reduce(core, Event::ConnectFailed { attempt: 1, kind }, at(secs))
    }

    fn connect(index: usize) -> Effect {
        Effect::ConnectMirror { attempt: 1, index }
    }

    /// Playing on Mirror `m` of `n`, then the stream drops at t=0.
    fn dropped(n: usize, m: usize) -> Core {
        let mut c = trying(n);
        for _ in 0..m {
            c = fail_at(c, Offline, 0).0;
        }
        let c = reduce(c, Event::Connected { attempt: 1 }, at(0)).0;
        reduce(c, Event::StreamDropped { attempt: 1 }, at(0)).0
    }

    #[test]
    fn drop_retries_same_mirror_at_once_as_reconnecting() {
        let mut c = trying(3);
        c = fail_at(c, Offline, 0).0; // mirror 0 fails, mirror 1 is tried
        c = reduce(c, Event::Connected { attempt: 1 }, at(0)).0;
        let (c, fx) = reduce(c, Event::StreamDropped { attempt: 1 }, at(5));
        assert_eq!(c.state.status, Status::Connecting { reconnecting: true });
        assert_eq!(fx, vec![connect(1), Effect::EmitSnapshot]);
    }

    #[test]
    fn drop_then_walks_remaining_mirrors_in_pls_order() {
        let c = dropped(4, 2); // playing on Mirror 2
        let (c, fx) = fail_at(c, Unavailable, 1);
        assert_eq!(fx, vec![connect(0)]);
        let (c, fx) = fail_at(c, Unavailable, 1);
        assert_eq!(fx, vec![connect(1)]);
        let (c, fx) = fail_at(c, Unavailable, 1);
        assert_eq!(fx, vec![connect(3)]);
        // One full pass done: the next step is a backoff wait, not an error.
        let (c, fx) = fail_at(c, Unavailable, 1);
        assert_eq!(c.state.status, Status::Connecting { reconnecting: true });
        assert_eq!(
            fx,
            vec![Effect::ArmTimer {
                attempt: 1,
                at: at(3)
            }]
        );
    }

    #[test]
    fn backoff_cycles_2_4_8_16_30_then_gives_up_two_minutes_after_drop() {
        let mut c = dropped(1, 0);
        let mut now = 0;
        let mut waits = Vec::new();
        loop {
            // The only Mirror fails instantly; the pass is over.
            let (c2, fx) = fail_at(c, Offline, now);
            c = c2;
            let Effect::ArmTimer { at: wake, .. } = fx[0] else {
                panic!("expected a timer, got {fx:?}");
            };
            let wake_secs = (wake - t0()).as_secs();
            waits.push(wake_secs - now);
            now = wake_secs;
            let (c2, fx) = reduce(c, Event::TimerFired { attempt: 1 }, at(now));
            c = c2;
            if now >= DEADLINE_SECS {
                assert_eq!(c.state.status, err(Offline));
                assert_eq!(fx, vec![Effect::Disconnect, Effect::EmitSnapshot]);
                break;
            }
            assert_eq!(fx, vec![connect(0)]);
            assert_eq!(c.state.status, Status::Connecting { reconnecting: true });
        }
        // Delays double to the 30 s cap; the last wait is clamped to the 120 s budget.
        assert_eq!(waits, [2, 4, 8, 16, 30, 30, 30]);
        assert_eq!(now, DEADLINE_SECS);
    }

    #[test]
    fn wait_is_clamped_to_the_budget_and_late_failure_gives_up() {
        let (c, fx) = fail_at(dropped(1, 0), Offline, 119);
        assert_eq!(
            fx,
            vec![Effect::ArmTimer {
                attempt: 1,
                at: at(120)
            }]
        );
        assert_eq!(c.state.status, Status::Connecting { reconnecting: true });
        // A Mirror attempt that fails after the budget is spent ends in error.
        let (c, fx) = fail_at(dropped(2, 0), Offline, 125);
        assert_eq!(c.state.status, err(Offline));
        assert_eq!(fx, vec![Effect::Disconnect, Effect::EmitSnapshot]);
    }

    #[test]
    fn giving_up_reports_unavailable_when_servers_refused() {
        let c = dropped(2, 0);
        let c = fail_at(c, Unavailable, 1).0;
        let c = fail_at(c, Offline, 1).0; // pass over: arm at 3
        let (c, _) = reduce(c, Event::TimerFired { attempt: 1 }, at(DEADLINE_SECS));
        assert_eq!(c.state.status, err(Unavailable));
    }

    #[test]
    fn timer_before_the_deadline_starts_a_new_pass() {
        let c = fail_at(dropped(2, 1), Offline, 0).0;
        let c = fail_at(c, Offline, 0).0;
        let (c, fx) = reduce(c, Event::TimerFired { attempt: 1 }, at(2));
        assert_eq!(fx, vec![connect(1)]); // same Mirror first
        assert_eq!(c.state.status, Status::Connecting { reconnecting: true });
    }

    #[test]
    fn reconnect_success_plays_and_a_later_drop_gets_a_fresh_budget() {
        let c = fail_at(dropped(1, 0), Offline, 0).0;
        let c = reduce(c, Event::TimerFired { attempt: 1 }, at(2)).0;
        let (c, fx) = reduce(c, Event::Connected { attempt: 1 }, at(3));
        assert_eq!(c.state.status, Status::Playing);
        assert_eq!(fx, vec![Effect::EmitSnapshot]);
        let c = reduce(c, Event::StreamDropped { attempt: 1 }, at(500)).0;
        let (c, fx) = fail_at(c, Offline, 600);
        assert_eq!(
            fx,
            vec![Effect::ArmTimer {
                attempt: 1,
                at: at(602)
            }]
        );
        assert_eq!(c.state.status, Status::Connecting { reconnecting: true });
    }

    #[test]
    fn stop_while_waiting_cancels_and_ignores_the_timer() {
        let c = fail_at(dropped(1, 0), Offline, 0).0;
        let (c, fx) = reduce(c, Event::StopRequested, at(1));
        assert_eq!(c.state.status, Status::Stopped);
        assert!(fx.contains(&Effect::Disconnect));
        let (after, fx) = reduce(c.clone(), Event::TimerFired { attempt: 1 }, at(2));
        assert_eq!(after, c);
        assert!(fx.is_empty());
    }

    #[test]
    fn decode_failure_while_reconnecting_is_unsupported() {
        let (c, _) = reduce(dropped(2, 0), Event::DecodeFailed { attempt: 1 }, at(1));
        assert_eq!(c.state.status, err(Unsupported));
    }

    #[test]
    fn retry_after_giving_up_is_a_first_connect_again() {
        let c = reduce(dropped(1, 0), Event::TimerFired { attempt: 1 }, at(0)).0;
        let c = fail_at(c, Offline, DEADLINE_SECS).0;
        let (c, _) = reduce(c, play("a"), at(DEADLINE_SECS + 1));
        assert_eq!(
            c.state.status,
            Status::Connecting {
                reconnecting: false
            }
        );
        let (c, _) = reduce(
            c,
            Event::MirrorsResolved {
                attempt: 2,
                count: 1,
            },
            at(DEADLINE_SECS + 1),
        );
        let (c, fx) = reduce(
            c,
            Event::ConnectFailed {
                attempt: 2,
                kind: Offline,
            },
            at(DEADLINE_SECS + 2),
        );
        assert_eq!(c.state.status, err(Offline));
        assert!(!fx.iter().any(|f| matches!(f, Effect::ArmTimer { .. })));
    }

    #[test]
    fn mirror_order_starts_with_the_last_working_mirror() {
        // Station "a" connected on Mirror 1 earlier this session.
        let c = fail_at(trying(3), Offline, 0).0;
        let c = reduce(c, Event::Connected { attempt: 1 }, at(0)).0;
        let c = reduce(c, Event::StopRequested, at(1)).0;
        let c = reduce(c, play("a"), at(2)).0;
        let (c, fx) = reduce(
            c,
            Event::MirrorsResolved {
                attempt: 3,
                count: 3,
            },
            at(2),
        );
        assert_eq!(
            fx,
            vec![Effect::ConnectMirror {
                attempt: 3,
                index: 1
            }]
        );
        let order: Vec<_> = (0..2)
            .scan(c, |c, _| {
                let (n, fx) = reduce(
                    c.clone(),
                    Event::ConnectFailed {
                        attempt: 3,
                        kind: Offline,
                    },
                    at(2),
                );
                *c = n;
                Some(fx[0].clone())
            })
            .collect();
        assert_eq!(
            order,
            [
                Effect::ConnectMirror {
                    attempt: 3,
                    index: 0
                },
                Effect::ConnectMirror {
                    attempt: 3,
                    index: 2
                }
            ]
        );
    }

    #[test]
    fn last_working_mirror_is_per_station_and_clamped_to_the_list() {
        let c = fail_at(trying(3), Offline, 0).0;
        let c = fail_at(c, Offline, 0).0;
        let c = reduce(c, Event::Connected { attempt: 1 }, at(0)).0; // Mirror 2
        // Another station starts at Mirror 0.
        let (o, _) = reduce(c.clone(), play("b"), at(1));
        let (_, fx) = reduce(
            o,
            Event::MirrorsResolved {
                attempt: 2,
                count: 3,
            },
            at(1),
        );
        assert_eq!(
            fx,
            vec![Effect::ConnectMirror {
                attempt: 2,
                index: 0
            }]
        );
        // The list shrank: fall back to Mirror 0.
        let (o, _) = reduce(c, play("a"), at(1));
        let (_, fx) = reduce(
            o,
            Event::MirrorsResolved {
                attempt: 2,
                count: 2,
            },
            at(1),
        );
        assert_eq!(
            fx,
            vec![Effect::ConnectMirror {
                attempt: 2,
                index: 0
            }]
        );
    }

    #[test]
    fn first_connect_pass_never_arms_a_timer() {
        let (_, fx) = run(
            trying(2),
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
        assert!(!fx.iter().any(|f| matches!(f, Effect::ArmTimer { .. })));
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
