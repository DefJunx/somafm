pub mod commands;
pub mod feed;
pub mod media_controls;
pub mod player;
pub mod settings;
pub mod tray;

use std::sync::Arc;

use tauri::{Emitter, Manager, WindowEvent};
use tokio::sync::watch;

use feed::{Feed, Schedule};
use player::{
    Player, SinkFactory,
    decode::{PcmSink, RodioSink},
};
use settings::{SettingsService, StorePersist};
use tauri_plugin_store::StoreBuilder;

fn window_visible(w: &tauri::Window) -> bool {
    w.is_visible().unwrap_or(true) && !w.is_minimized().unwrap_or(false)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .invoke_handler(tauri::generate_handler![
            commands::get_stations,
            commands::refresh_stations,
            commands::get_player_state,
            commands::play,
            commands::stop,
            commands::set_volume,
            commands::get_settings,
            commands::toggle_favourite,
            commands::set_favourites_only
        ])
        .setup(|app| {
            let cache_dir = app.path().app_cache_dir()?;
            let feed = Arc::new(Feed::new(feed::FEED_URL, &cache_dir));
            app.manage(feed.clone());

            let emitter = app.handle().clone();
            let sink: SinkFactory =
                Arc::new(|| RodioSink::open().map(|s| Box::new(s) as Box<dyn PcmSink>));
            let player = Player::new(
                feed.clone(),
                &cache_dir,
                tauri::async_runtime::handle().inner().clone(),
                sink,
                move |state| {
                    let _ = emitter.emit(commands::PLAYER_STATE, state);
                },
            );
            app.manage(player.clone());

            // Settings: absolute path in the app-config dir (the store plugin would
            // otherwise resolve relative paths against the app-data dir).
            let path = app.path().app_config_dir()?.join(settings::FILE);
            let loaded = settings::load_file(&path, settings::MIGRATIONS);
            let store = StoreBuilder::new(app.handle(), path).build()?;
            let persist = StorePersist(store);
            if loaded.needs_save {
                settings::Persist::save(&persist, &loaded.settings);
            }
            let s = loaded.settings;
            // A last Station that left the feed is treated as null (not rewritten).
            // Without a feed yet (fresh install) the id cannot be checked, so keep it.
            let last = s
                .last_station_id
                .clone()
                .filter(|id| !feed.snapshot().loaded || feed.station(id).is_some());
            player.restore(last, s.volume);
            let emitter = app.handle().clone();
            app.manage(SettingsService::new(
                s,
                persist,
                tauri::async_runtime::handle().inner().clone(),
                settings::VOLUME_DEBOUNCE,
                move |view| {
                    let _ = emitter.emit(settings::SETTINGS_CHANGED, view);
                },
            ));

            // Refresh loop runs only while the main window is visible and not minimised.
            // The tray slice (issue 07) must send false/true on the managed
            // `watch::Sender<bool>` when hiding/showing to tray: GTK may emit neither
            // Focused nor Resized.
            let initial_visible = app
                .get_webview_window("main")
                .is_none_or(|w| window_visible(&w.as_ref().window()));
            let (visible_tx, visible_rx) = watch::channel(initial_visible);
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(feed::run_refresh_loop(
                feed,
                visible_rx,
                Schedule::DEFAULT,
                move |snap| {
                    let _ = handle.emit(commands::STATIONS_UPDATED, snap);
                },
            ));
            let tx = visible_tx.clone();
            app.manage(visible_tx); // keeps the loop alive for the app's lifetime
            if let Some(win) = app.get_webview_window("main") {
                // Focus/resize events fire on minimise/restore/show, so re-evaluate there.
                let w2 = win.as_ref().window();
                win.on_window_event(move |ev| {
                    if matches!(ev, WindowEvent::Focused(_) | WindowEvent::Resized(_)) {
                        let _ = tx.send(window_visible(&w2));
                    }
                });
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                // Don't lose a volume change still inside its debounce window.
                if let Some(s) = app.try_state::<SettingsService>() {
                    s.flush();
                }
            }
        });
}

#[cfg(test)]
mod tests {
    #[test]
    fn harness_works() {
        assert_eq!(1 + 1, 2);
    }
}
