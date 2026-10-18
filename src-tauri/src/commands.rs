//! Tauri commands (frontend -> Rust).

use std::sync::Arc;

use tauri::{AppHandle, Emitter, State};

use crate::feed::{Feed, StationsSnapshot};
use crate::player::{Player, PlayerState};
use crate::settings::{SettingsService, SettingsView};

pub const STATIONS_UPDATED: &str = "stations-updated";
pub const PLAYER_STATE: &str = "player-state";

#[tauri::command]
pub fn get_stations(feed: State<'_, Arc<Feed>>) -> StationsSnapshot {
    feed.snapshot()
}

/// Forces a fetch now. A failed fetch is not an error: the current snapshot
/// (possibly `loaded: false`) is returned.
#[tauri::command]
pub async fn refresh_stations(
    app: AppHandle,
    feed: State<'_, Arc<Feed>>,
) -> Result<StationsSnapshot, String> {
    if let Ok(true) = feed.refresh().await {
        let _ = app.emit(STATIONS_UPDATED, feed.snapshot());
    }
    Ok(feed.snapshot())
}

#[tauri::command]
pub fn get_player_state(player: State<'_, Player>) -> PlayerState {
    player.state()
}

/// Err only for an unknown station id; playback failures surface as `Status::Error`.
#[tauri::command]
pub fn play(
    player: State<'_, Player>,
    settings: State<'_, SettingsService>,
    station_id: String,
) -> Result<(), String> {
    play_station(&player, &settings, &station_id)
}

/// The one play path (webview and tray): load the Station, then persist `lastStationId`.
pub fn play_station(
    player: &Player,
    settings: &SettingsService,
    station_id: &str,
) -> Result<(), String> {
    player.play(station_id)?;
    settings.set_last_station(station_id);
    Ok(())
}

#[tauri::command]
pub fn stop(player: State<'_, Player>) {
    player.stop();
}

#[tauri::command]
pub fn set_volume(player: State<'_, Player>, settings: State<'_, SettingsService>, volume: f32) {
    player.set_volume(volume);
    settings.set_volume(volume); // debounced write
}

#[tauri::command]
pub fn get_settings(settings: State<'_, SettingsService>) -> SettingsView {
    settings.view()
}

/// Err only for an unknown station id; a Favourite that left the feed can still be removed.
#[tauri::command]
pub fn toggle_favourite(
    settings: State<'_, SettingsService>,
    feed: State<'_, Arc<Feed>>,
    station_id: String,
) -> Result<(), String> {
    let is_favourite = settings.view().favourites.contains(&station_id);
    if !is_favourite && feed.station(&station_id).is_none() {
        return Err(format!("unknown station `{station_id}`"));
    }
    settings.toggle_favourite(&station_id);
    Ok(())
}

#[tauri::command]
pub fn set_favourites_only(settings: State<'_, SettingsService>, value: bool) {
    settings.set_favourites_only(value);
}
