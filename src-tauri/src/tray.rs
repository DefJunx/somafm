//! System tray: a thin shell over the same `play`/`stop`/show/hide paths the
//! webview uses. The only logic is the pure state-to-label mapping below.

use std::sync::Arc;

use tauri::{
    AppHandle, Listener, Manager, Wry,
    image::Image,
    menu::{Menu, MenuBuilder, MenuItemBuilder, SubmenuBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};
use tokio::sync::watch;

use crate::commands;
use crate::feed::Feed;
use crate::player::{Player, PlayerState, Status};
use crate::settings::SettingsService;

const TRAY_ID: &str = "main";
const ID_TOGGLE: &str = "toggle";
const ID_SHOW_HIDE: &str = "show-hide";
const ID_QUIT: &str = "quit";
/// Id of the app-menu Quit item that carries the Cmd/Ctrl+Q accelerator.
const ID_APP_QUIT: &str = "app-quit";
const FAV_PREFIX: &str = "fav:";

#[derive(Debug, PartialEq)]
pub struct TrayView {
    pub now_playing: String,
    pub toggle_label: &'static str,
}

#[derive(Debug, PartialEq)]
pub enum ToggleAction {
    Stop,
    Play(String),
    Nothing,
}

/// Maps player state (plus the loaded Station's title, if known) to tray text.
pub fn view(state: &PlayerState, station: Option<&str>) -> TrayView {
    let name = station.unwrap_or("SomaFM");
    let now_playing = match &state.status {
        Status::Connecting { reconnecting: true } => "Reconnecting...".to_string(),
        Status::Connecting {
            reconnecting: false,
        } => "Connecting...".to_string(),
        Status::Error { message, .. } => message.clone(),
        Status::Stopped => match state.station_id {
            Some(_) => format!("{name} - Paused"),
            None => name.to_string(),
        },
        Status::Playing => match &state.now_playing {
            Some(np) => match &np.artist {
                Some(artist) => format!("{name} - {artist} - {}", np.title),
                None => format!("{name} - {}", np.title),
            },
            None => name.to_string(),
        },
    };
    let toggle_label = match state.status {
        Status::Playing | Status::Connecting { .. } => "Stop",
        Status::Stopped | Status::Error { .. } => "Play",
    };
    TrayView {
        now_playing,
        toggle_label,
    }
}

/// What the Play/Stop item does for this state.
pub fn toggle_action(state: &PlayerState) -> ToggleAction {
    match (&state.status, &state.station_id) {
        (Status::Playing | Status::Connecting { .. }, _) => ToggleAction::Stop,
        (_, Some(id)) => ToggleAction::Play(id.clone()),
        (_, None) => ToggleAction::Nothing,
    }
}

pub fn show_hide_label(window_visible: bool) -> &'static str {
    if window_visible { "Hide" } else { "Show" }
}

fn main_visible(app: &AppHandle) -> bool {
    app.get_webview_window("main")
        .is_some_and(|w| w.is_visible().unwrap_or(false) && !w.is_minimized().unwrap_or(false))
}

/// Shows or hides (never destroys) the main window and tells the feed loop.
pub fn set_main_visible(app: &AppHandle, visible: bool) {
    if let Some(w) = app.get_webview_window("main") {
        if visible {
            let _ = w.unminimize();
            let _ = w.show();
            let _ = w.set_focus();
        } else {
            let _ = w.hide();
        }
    }
    if let Some(tx) = app.try_state::<watch::Sender<bool>>() {
        tx.send_replace(visible);
    }
    refresh(app);
}

pub fn toggle_main(app: &AppHandle) {
    set_main_visible(app, !main_visible(app));
}

fn build_menu(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    let state = app.state::<Player>().state();
    let feed = app.state::<Arc<Feed>>();
    let favourites = app.state::<SettingsService>().view().favourites;
    let snapshot = feed.snapshot();
    let station_title = state
        .station_id
        .as_deref()
        .and_then(|id| snapshot.stations.iter().find(|s| s.id == id))
        .map(|s| s.title.as_str());
    let v = view(&state, station_title);

    let mut fav_menu = SubmenuBuilder::new(app, "Favourites");
    let mut any = false;
    for s in snapshot
        .stations
        .iter()
        .filter(|s| favourites.contains(&s.id))
    {
        any = true;
        fav_menu = fav_menu
            .item(&MenuItemBuilder::with_id(format!("{FAV_PREFIX}{}", s.id), &s.title).build(app)?);
    }
    if !any {
        fav_menu = fav_menu.item(
            &MenuItemBuilder::with_id("no-favourites", "No favourites")
                .enabled(false)
                .build(app)?,
        );
    }

    MenuBuilder::new(app)
        .item(
            &MenuItemBuilder::with_id("now-playing", v.now_playing)
                .enabled(false)
                .build(app)?,
        )
        .item(&MenuItemBuilder::with_id(ID_TOGGLE, v.toggle_label).build(app)?)
        .item(&fav_menu.build()?)
        .separator()
        .item(
            &MenuItemBuilder::with_id(ID_SHOW_HIDE, show_hide_label(main_visible(app)))
                .build(app)?,
        )
        .item(&MenuItemBuilder::with_id(ID_QUIT, "Quit").build(app)?)
        .build()
}

/// Rebuilds the tray menu from current state (on the main thread).
pub fn refresh(app: &AppHandle) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let (Some(tray), Ok(menu)) = (handle.tray_by_id(TRAY_ID), build_menu(&handle)) {
            let _ = tray.set_menu(Some(menu));
        }
    });
}

fn on_menu_event(app: &AppHandle, id: &str) {
    match id {
        ID_TOGGLE => {
            let player = app.state::<Player>();
            match toggle_action(&player.state()) {
                ToggleAction::Stop => player.stop(),
                ToggleAction::Play(id) => play(app, &id),
                ToggleAction::Nothing => {}
            }
        }
        ID_SHOW_HIDE => toggle_main(app),
        ID_QUIT => app.exit(0),
        other => {
            if let Some(id) = other.strip_prefix(FAV_PREFIX) {
                play(app, id);
            }
        }
    }
}

/// Same path as the `play` command (persists `lastStationId`).
fn play(app: &AppHandle, station_id: &str) {
    let _ = commands::play_station(
        &app.state::<Player>(),
        &app.state::<SettingsService>(),
        station_id,
    );
}

/// App menu whose only job is the Cmd/Ctrl+Q accelerator. On macOS it is the
/// menu bar; elsewhere the (hidden) menu bar keeps the shortcut.
fn init_app_menu(app: &AppHandle) -> tauri::Result<()> {
    let quit = MenuItemBuilder::with_id(ID_APP_QUIT, "Quit")
        .accelerator("CmdOrCtrl+Q")
        .build(app)?;
    let menu = MenuBuilder::new(app)
        .item(&SubmenuBuilder::new(app, "somafm").item(&quit).build()?)
        .build()?;
    app.set_menu(menu)?;
    app.on_menu_event(|app, ev| {
        if ev.id().as_ref() == ID_APP_QUIT {
            app.exit(0);
        }
    });
    #[cfg(not(target_os = "macos"))]
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.hide_menu();
    }
    Ok(())
}

/// Builds the tray icon and keeps its menu in sync with player and settings events.
pub fn init(app: &AppHandle) -> tauri::Result<()> {
    init_app_menu(app)?;
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(Image::from_bytes(include_bytes!("../icons/tray.png"))?)
        .icon_as_template(true)
        .show_menu_on_left_click(false)
        .tooltip("somafm")
        .menu(&build_menu(app)?)
        .on_menu_event(|app, ev| on_menu_event(app, ev.id().as_ref()))
        .on_tray_icon_event(|tray, ev| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = ev
            {
                toggle_main(tray.app_handle());
            }
        })
        .build(app)?;
    for event in [
        commands::PLAYER_STATE,
        crate::settings::SETTINGS_CHANGED,
        commands::STATIONS_UPDATED,
    ] {
        let h = app.clone();
        app.listen(event, move |_| refresh(&h));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::{ErrorKind, NowPlaying};

    fn state(
        status: Status,
        station: Option<&str>,
        np: Option<(Option<&str>, &str)>,
    ) -> PlayerState {
        PlayerState {
            station_id: station.map(String::from),
            status,
            volume: 0.8,
            now_playing: np.map(|(artist, title)| NowPlaying {
                raw: String::new(),
                artist: artist.map(String::from),
                title: title.into(),
            }),
        }
    }

    fn err() -> Status {
        Status::Error {
            error_kind: ErrorKind::Offline,
            message: "You're offline".into(),
        }
    }

    #[test]
    fn view_table() {
        let conn = |r| Status::Connecting { reconnecting: r };
        let cases: Vec<(PlayerState, Option<&str>, &str, &str)> = vec![
            (
                state(
                    Status::Playing,
                    Some("g"),
                    Some((Some("Boards"), "Roygbiv")),
                ),
                Some("Groove"),
                "Groove - Boards - Roygbiv",
                "Stop",
            ),
            (
                state(Status::Playing, Some("g"), Some((None, "Station ID"))),
                Some("Groove"),
                "Groove - Station ID",
                "Stop",
            ),
            (
                state(Status::Playing, Some("g"), None),
                Some("Groove"),
                "Groove",
                "Stop",
            ),
            (
                state(conn(false), Some("g"), None),
                Some("Groove"),
                "Connecting...",
                "Stop",
            ),
            (
                state(conn(true), Some("g"), Some((Some("A"), "B"))),
                Some("Groove"),
                "Reconnecting...",
                "Stop",
            ),
            (
                state(err(), Some("g"), None),
                Some("Groove"),
                "You're offline",
                "Play",
            ),
            (
                state(Status::Stopped, Some("g"), None),
                Some("Groove"),
                "Groove - Paused",
                "Play",
            ),
            (state(Status::Stopped, None, None), None, "SomaFM", "Play"),
            (
                state(Status::Playing, Some("gone"), None),
                None,
                "SomaFM",
                "Stop",
            ),
        ];
        for (st, title, line, label) in cases {
            assert_eq!(
                view(&st, title),
                TrayView {
                    now_playing: line.into(),
                    toggle_label: label
                },
                "{st:?}"
            );
        }
    }

    #[test]
    fn toggle_action_table() {
        let play = |id: &str| ToggleAction::Play(id.into());
        assert_eq!(
            toggle_action(&state(Status::Playing, Some("g"), None)),
            ToggleAction::Stop
        );
        assert_eq!(
            toggle_action(&state(
                Status::Connecting { reconnecting: true },
                Some("g"),
                None
            )),
            ToggleAction::Stop
        );
        assert_eq!(
            toggle_action(&state(Status::Stopped, Some("g"), None)),
            play("g")
        );
        assert_eq!(toggle_action(&state(err(), Some("g"), None)), play("g"));
        assert_eq!(
            toggle_action(&state(Status::Stopped, None, None)),
            ToggleAction::Nothing
        );
    }

    #[test]
    fn show_hide_follows_visibility() {
        assert_eq!(show_hide_label(true), "Hide");
        assert_eq!(show_hide_label(false), "Show");
    }
}
