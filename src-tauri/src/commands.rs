//! Tauri commands (frontend -> Rust).

use std::sync::Arc;

use tauri::{AppHandle, Emitter, State};

use crate::feed::{Feed, StationsSnapshot};

pub const STATIONS_UPDATED: &str = "stations-updated";

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
