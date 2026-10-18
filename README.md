# somafm

Desktop SomaFM player (Tauri). Closing the window hides it to the tray and playback continues; quit from the tray menu or with Cmd/Ctrl+Q.

## Building & installing

See [BUILD.md](BUILD.md) for platform-specific build and installation instructions.

## Platform notes

- **Linux:** the tray uses AppIndicator, which is menu-only (no click events), so use Show/Hide in the tray menu. KDE shows the tray natively; stock GNOME needs the [AppIndicator extension](https://extensions.gnome.org/extension/615/appindicator-support/). There is no workaround in the app.
- **macOS:** the dock icon stays visible while the window is hidden; clicking it reopens the window.
- **Windows:** the tray icon appears in the notification area.
