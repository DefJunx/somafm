//! Tauri commands (frontend -> Rust).

use std::sync::Arc;

use tauri::{AppHandle, Emitter, State};

use crate::feed::{Feed, StationsSnapshot};
use crate::player::{Player, PlayerState, Quality};

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
pub fn play(player: State<'_, Player>, station_id: String) -> Result<(), String> {
    player.play(&station_id)
}

#[tauri::command]
pub fn stop(player: State<'_, Player>) {
    player.stop();
}

#[tauri::command]
pub fn set_volume(player: State<'_, Player>, volume: f32) {
    player.set_volume(volume);
}

#[tauri::command]
pub fn set_quality(player: State<'_, Player>, quality: Quality) {
    player.set_quality(quality);
}
