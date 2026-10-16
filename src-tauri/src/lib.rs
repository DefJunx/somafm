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

fn window_visible(w: &tauri::Window) -> bool {
    w.is_visible().unwrap_or(true) && !w.is_minimized().unwrap_or(false)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::get_stations,
            commands::refresh_stations
        ])
        .setup(|app| {
            let cache_dir = app.path().app_cache_dir()?;
            let feed = Arc::new(Feed::new(feed::FEED_URL, &cache_dir));
            app.manage(feed.clone());

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
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    #[test]
    fn harness_works() {
        assert_eq!(1 + 1, 2);
    }
}
