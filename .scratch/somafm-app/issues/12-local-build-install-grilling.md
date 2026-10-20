# 12 Local build and install per platform

Type: grilling
Status: resolved
Blocked by: 

## Question

How does the user build and install the app on Linux, macOS and Windows for personal use (no release packaging or signing)? Decide: the prerequisite system packages per OS (including GNOME's AppIndicator extension), the build command and its output, how the binary is installed or launched (e.g. AppImage/.deb vs a plain binary, unsigned .app and the Gatekeeper step, Windows .exe/.msi), and whether launch-at-login is in scope.

- **Platforms**: the spec gives full build and install steps for Linux, macOS and Windows. Only Linux (the user's Bazzite 44 KDE, immutable Fedora) is verified, and the manual smoke checklist runs there.
- **Toolchain (all OSes)**: Rust stable via rustup, Node LTS, **pnpm**. Dev: `pnpm tauri dev`. Build: `pnpm tauri build` with per-OS targets as below.
- **Linux**: build inside a Fedora 44 `toolbox` that has rustup and the dev packages (`webkit2gtk4.1-devel`, `libappindicator-gtk3-devel`, `alsa-lib-devel`, `librsvg2-devel`, `openssl-devel`, plus the C toolchain). The spec gives the exact `dnf install` line. Do not layer packages on the host. Build with `pnpm tauri build --no-bundle` and copy the binary to `~/.local/bin`. The repo ships a `.desktop` template and icon, installed to `~/.local/share/applications` and `~/.local/share/icons`. The host already has the runtime libraries (`webkit2gtk4.1`, `libappindicator-gtk3`, `alsa-lib`). No AppImage, no rpm. KDE shows the tray natively; stock GNOME still needs the AppIndicator extension (see Media keys and tray).
- **macOS**: `pnpm tauri build --bundles app` (ad-hoc signed), then copy the `.app` to `/Applications`. No dmg. A local build carries no quarantine attribute; `xattr -dr com.apple.quarantine` is only needed when the `.app` was copied from another machine.
- **Windows**: NSIS per-user `-setup.exe` (`--bundles nsis`), no admin and no MSI/WiX. WebView2 comes with Windows 10 and 11. Expect a one-time SmartScreen warning because it is unsigned.
- **Launch at login**: not in v1. On Linux the user adds it through KDE Autostart.
- **Updates**: rebuild and reinstall by hand, with no updater plugin. The bundle identifier is fixed as `dev.defjunx.somafm` and never changes, so settings in the app-data dir survive reinstalls.
- **App icon**: an original generic radio/waveform SVG committed to the repo and expanded with `pnpm tauri icon`, plus a monochrome tray variant. No SomaFM artwork.
