# somafm

Desktop SomaFM player (Tauri). Closing the window hides it to the tray and playback continues; quit from the tray menu or with Cmd/Ctrl+Q.

## Platform notes

- **Linux tray:** the tray uses AppIndicator, which is menu-only (no click events), so use Show/Hide in the tray menu. KDE shows the tray natively; stock GNOME needs the AppIndicator extension (e.g. "AppIndicator and KStatusNotifierItem Support"). There is no workaround in the app.
- **macOS:** the dock icon stays visible while the window is hidden; clicking it reopens the window.
