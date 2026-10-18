//! OS media keys and now-playing overlay (souvlaki: MPRIS, SMTC, MPNowPlaying).
//!
//! A thin shell: the only logic is the pure `overlay` (state to playback and
//! metadata) and `action` (key event to what the app does) mappings below.

use std::sync::{Arc, Mutex};

use souvlaki::{MediaControlEvent, MediaControls, MediaMetadata, MediaPlayback, PlatformConfig};
use tauri::{AppHandle, Listener, Manager};

use crate::commands;
use crate::feed::{Feed, Station};
use crate::player::{Player, PlayerState, Status};
use crate::settings::SettingsService;
use crate::tray::{self, ToggleAction};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Playback {
    Playing,
    /// Also used for `stopped` and `error`: never Stopped, so the OS keeps
    /// routing the play key to the app.
    Paused,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Metadata {
    pub artist: Option<String>,
    pub title: Option<String>,
    pub album: Option<String>,
    pub cover_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Overlay {
    pub playback: Playback,
    pub metadata: Metadata,
}

#[derive(Debug, PartialEq)]
pub enum Action {
    Stop,
    Play(String),
    Nothing,
}

/// Maps player state (plus the loaded Station, if known) to the OS overlay.
pub fn overlay(state: &PlayerState, station: Option<&Station>) -> Overlay {
    let playback = match state.status {
        Status::Playing | Status::Connecting { .. } => Playback::Playing,
        Status::Stopped | Status::Error { .. } => Playback::Paused,
    };
    let metadata = match station {
        None => Metadata::default(),
        Some(st) => {
            let (artist, title) = match &state.now_playing {
                Some(np) => (
                    np.artist.clone().unwrap_or_else(|| st.title.clone()),
                    np.title.clone(),
                ),
                None => (st.title.clone(), st.title.clone()),
            };
            Metadata {
                artist: Some(artist),
                title: Some(title),
                album: Some(st.title.clone()),
                cover_url: Some(st.image.clone()).filter(|u| !u.is_empty()),
            }
        }
    };
    Overlay { playback, metadata }
}

/// What a media-key event does. `last_station` is the persisted `lastStationId`,
/// used only when no Station is loaded.
pub fn action(
    event: &MediaControlEvent,
    state: &PlayerState,
    last_station: Option<&str>,
) -> Action {
    let toggle = || match tray::toggle_action(state) {
        ToggleAction::Stop => Action::Stop,
        ToggleAction::Play(id) => Action::Play(id),
        ToggleAction::Nothing => last_station.map_or(Action::Nothing, |id| Action::Play(id.into())),
    };
    let active = matches!(state.status, Status::Playing | Status::Connecting { .. });
    match event {
        MediaControlEvent::Toggle => toggle(),
        MediaControlEvent::Play if !active => toggle(),
        MediaControlEvent::Pause if active => Action::Stop,
        MediaControlEvent::Stop => Action::Stop,
        _ => Action::Nothing,
    }
}

/// Live souvlaki handle plus the last overlay sent (skips redundant updates).
struct Shell {
    controls: MediaControls,
    last: Option<Overlay>,
}

type SharedShell = Arc<Mutex<Shell>>;

fn handle_event(app: &AppHandle, event: &MediaControlEvent) {
    let player = app.state::<Player>();
    let last = app.state::<SettingsService>().snapshot().last_station_id;
    match action(event, &player.state(), last.as_deref()) {
        Action::Stop => player.stop(),
        Action::Play(id) => tray::play(app, &id),
        Action::Nothing => {}
    }
}

/// Pushes the current state to the OS when it differs from what was last sent.
fn sync(app: &AppHandle, shell: &SharedShell) {
    let Ok(mut sh) = shell.lock() else { return };
    let state = app.state::<Player>().state();
    let station = state
        .station_id
        .as_deref()
        .and_then(|id| app.state::<Arc<Feed>>().station(id));
    let next = overlay(&state, station.as_ref());
    if sh.last.as_ref() == Some(&next) {
        return;
    }
    let m = &next.metadata;
    let r = sh.controls.set_metadata(MediaMetadata {
        title: m.title.as_deref(),
        album: m.album.as_deref(),
        artist: m.artist.as_deref(),
        cover_url: m.cover_url.as_deref(),
        duration: None,
    });
    let r = r.and_then(|()| {
        sh.controls.set_playback(match next.playback {
            Playback::Playing => MediaPlayback::Playing { progress: None },
            Playback::Paused => MediaPlayback::Paused { progress: None },
        })
    });
    match r {
        Ok(()) => sh.last = Some(next),
        Err(e) => eprintln!("media controls update failed: {e:?}"),
    }
}

/// Registers OS media controls. Failure (e.g. no D-Bus session bus) is logged
/// and the app carries on without them.
pub fn init(app: &AppHandle) {
    #[cfg(target_os = "windows")]
    let hwnd = app
        .get_webview_window("main")
        .and_then(|w| w.hwnd().ok())
        .map(|h| h.0 as *mut std::ffi::c_void);
    #[cfg(not(target_os = "windows"))]
    let hwnd = None;
    #[cfg(target_os = "windows")]
    if hwnd.is_none() {
        eprintln!("media controls disabled: no main window handle");
        return;
    }

    let mut controls = match MediaControls::new(PlatformConfig {
        display_name: "SomaFM",
        dbus_name: "somafm",
        hwnd,
    }) {
        Ok(c) => c,
        Err(e) => return eprintln!("media controls unavailable: {e:?}"),
    };
    let h = app.clone();
    // Handlers arrive on souvlaki's own thread (the main thread on macOS) and
    // reach into the Player; run them off the caller, as the tray does.
    if let Err(e) = controls.attach(move |ev| {
        let h = h.clone();
        tray::run_deferred(move || handle_event(&h, &ev));
    }) {
        return eprintln!("media controls unavailable: {e:?}");
    }
    let shell: SharedShell = Arc::new(Mutex::new(Shell {
        controls,
        last: None,
    }));
    // `Player` holds its core lock while emitting `player-state`, so the
    // listener only hops to a worker; it never touches the Player inline.
    for event in [commands::PLAYER_STATE, commands::STATIONS_UPDATED] {
        let (h, sh) = (app.clone(), shell.clone());
        app.listen(event, move |_| {
            let (h, sh) = (h.clone(), sh.clone());
            tray::run_deferred(move || sync(&h, &sh));
        });
    }
    app.manage(shell.clone());
    let h = app.clone();
    tray::run_deferred(move || sync(&h, &shell));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::{ErrorKind, NowPlaying};
    use souvlaki::SeekDirection;
    use std::time::Duration;

    fn station() -> Station {
        Station {
            id: "g".into(),
            title: "Groove".into(),
            description: String::new(),
            dj: String::new(),
            genres: vec![],
            listeners: 0,
            last_playing: String::new(),
            image: "https://img.example/g.png".into(),
            large_image: String::new(),
            xl_image: String::new(),
            playlists: vec![],
        }
    }

    fn state(status: Status, id: Option<&str>, np: Option<(Option<&str>, &str)>) -> PlayerState {
        PlayerState {
            station_id: id.map(String::from),
            status,
            volume: 0.8,
            now_playing: np.map(|(artist, title)| NowPlaying {
                raw: String::new(),
                artist: artist.map(String::from),
                title: title.into(),
            }),
        }
    }

    fn conn(reconnecting: bool) -> Status {
        Status::Connecting { reconnecting }
    }

    fn err() -> Status {
        Status::Error {
            error_kind: ErrorKind::Offline,
            message: "You're offline".into(),
        }
    }

    fn meta(artist: Option<&str>, title: Option<&str>) -> Metadata {
        Metadata {
            artist: artist.map(String::from),
            title: title.map(String::from),
            album: Some("Groove".into()),
            cover_url: Some("https://img.example/g.png".into()),
        }
    }

    #[test]
    fn overlay_table() {
        use Playback::*;
        let st = station();
        let cases: Vec<(PlayerState, Playback, Metadata)> = vec![
            (
                state(
                    Status::Playing,
                    Some("g"),
                    Some((Some("Boards"), "Roygbiv")),
                ),
                Playing,
                meta(Some("Boards"), Some("Roygbiv")),
            ),
            (
                state(Status::Playing, Some("g"), Some((None, "Station ID"))),
                Playing,
                meta(Some("Groove"), Some("Station ID")),
            ),
            (
                state(Status::Playing, Some("g"), None),
                Playing,
                meta(Some("Groove"), Some("Groove")),
            ),
            (
                state(conn(false), Some("g"), None),
                Playing,
                meta(Some("Groove"), Some("Groove")),
            ),
            (
                state(conn(true), Some("g"), Some((Some("A"), "B"))),
                Playing,
                meta(Some("A"), Some("B")),
            ),
            (
                state(Status::Stopped, Some("g"), None),
                Paused,
                meta(Some("Groove"), Some("Groove")),
            ),
            (
                state(err(), Some("g"), None),
                Paused,
                meta(Some("Groove"), Some("Groove")),
            ),
        ];
        for (s, playback, metadata) in cases {
            assert_eq!(
                overlay(&s, Some(&st)),
                Overlay { playback, metadata },
                "{s:?}"
            );
        }
    }

    #[test]
    fn overlay_without_a_loaded_station_is_paused_and_empty() {
        for s in [
            state(Status::Stopped, None, None),
            state(Status::Playing, Some("gone"), None),
        ] {
            let o = overlay(&s, None);
            assert_eq!(o.metadata, Metadata::default(), "{s:?}");
        }
        assert_eq!(
            overlay(&state(Status::Stopped, None, None), None).playback,
            Playback::Paused
        );
    }

    #[test]
    fn action_table() {
        use MediaControlEvent as E;
        let play = |id: &str| Action::Play(id.into());
        let playing = state(Status::Playing, Some("g"), None);
        let connecting = state(conn(true), Some("g"), None);
        let stopped = state(Status::Stopped, Some("g"), None);
        let errored = state(err(), Some("g"), None);
        let empty = state(Status::Stopped, None, None);
        let cases: Vec<(E, &PlayerState, Option<&str>, Action)> = vec![
            (E::Toggle, &playing, Some("x"), Action::Stop),
            (E::Toggle, &connecting, None, Action::Stop),
            (E::Toggle, &stopped, Some("x"), play("g")),
            (E::Toggle, &errored, None, play("g")),
            (E::Toggle, &empty, Some("x"), play("x")),
            (E::Toggle, &empty, None, Action::Nothing),
            (E::Play, &stopped, None, play("g")),
            (E::Play, &errored, None, play("g")),
            (E::Play, &empty, Some("x"), play("x")),
            (E::Play, &empty, None, Action::Nothing),
            (E::Play, &playing, None, Action::Nothing),
            (E::Play, &connecting, None, Action::Nothing),
            (E::Pause, &playing, None, Action::Stop),
            (E::Pause, &connecting, None, Action::Stop),
            (E::Pause, &stopped, None, Action::Nothing),
            (E::Pause, &errored, None, Action::Nothing),
            (E::Pause, &empty, Some("x"), Action::Nothing),
            (E::Stop, &playing, None, Action::Stop),
            (E::Stop, &stopped, None, Action::Stop),
            (E::Next, &playing, None, Action::Nothing),
            (E::Previous, &playing, None, Action::Nothing),
            (
                E::Seek(SeekDirection::Forward),
                &playing,
                None,
                Action::Nothing,
            ),
            (
                E::SeekBy(SeekDirection::Backward, Duration::from_secs(5)),
                &playing,
                None,
                Action::Nothing,
            ),
            (E::SetVolume(0.2), &playing, None, Action::Nothing),
            (E::Raise, &playing, None, Action::Nothing),
            (E::Quit, &playing, None, Action::Nothing),
        ];
        for (ev, st, last, want) in cases {
            assert_eq!(action(&ev, st, last), want, "{ev:?} {st:?} {last:?}");
        }
    }
}
