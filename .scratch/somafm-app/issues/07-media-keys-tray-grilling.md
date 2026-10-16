# 07 Media keys and tray behaviour

Type: grilling
Status: resolved
Blocked by: 04

## Question

Given native Rust playback with `souvlaki` media controls and a Rust-owned tray (see [Architecture split](04-architecture-grilling.md)), what does each control do on a live stream? Decide: play/pause semantics (stop vs resume, which station when nothing is active), next/previous (cycle Favourites? all Stations? disabled?), what metadata is published to the OS media overlay, tray menu contents and click behaviour, and per-OS quirks: Wayland tray/MPRIS on Linux, macOS menu-bar and dock, Windows SMTC.

## Answer

Resolved by grilling with the user (2026-10-16).

- **Play/Pause**: pause is stop. It drops the connection and keeps the station loaded, and Play reconnects to the live stream. While a station is loaded the OS overlay reports *Paused*, not *Stopped*, so the play key keeps routing to the app. With no Active station, Play starts the last station from settings; on a fresh install with no last station it does nothing. The Stop key stops playback. Seek keys are ignored. OS volume is not mapped to app volume.
- **Next/Previous**: not supported. They are not advertised to the OS media controls and do nothing.
- **Overlay metadata**: ICY `StreamTitle` is split on the first ` - `: artist before it, title after. Album = station name, artwork = station image URL. If there is no separator, the whole string is the title and the station name is the artist. No duration or position (live stream).
- **Tray menu** (same on every OS): a disabled now-playing line (station and track), Play/Stop (label follows state), a Favourites submenu (click to play), Show/Hide window, Quit. Left-click on the icon toggles the window and right-click opens the menu, where the OS supports it.
- **Per-OS**:
  - Linux: tray is menu-only via AppIndicator (Tauri gets no click events), which is why Show/Hide is in the menu. Media controls go over MPRIS. Stock GNOME needs the AppIndicator extension; document it as a requirement, no workaround.
  - macOS: the dock icon stays visible even when the window is hidden, and clicking it reopens the window. The menu-bar icon is a template image.
  - Windows: SMTC is bound to the main window's HWND; the window is hidden, never destroyed, so media keys keep working.

Note (2026-10-16): the sleep timer was later ruled out of scope; references to it were removed above.
