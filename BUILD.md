# Build & Install Guide

This guide covers building and installing somafm on Linux, macOS (unverified), and Windows (unverified).

## Prerequisites

- Rust stable (via [rustup](https://rustup.rs/))
- Node.js LTS
- pnpm

## Linux

**Targeted at:** (build not yet run by the agent) Bazzite 44 KDE (immutable Fedora)

### 1. Set up build environment

The build requires WebKit and other development libraries. Use a Fedora toolbox container to avoid modifying your host system:

```bash
toolbox create -c fedora-toolbox-44 -i quay.io/fedora-toolbox/fedora-toolbox:44
toolbox run -c fedora-toolbox-44 bash -lc '
sudo dnf install \
  webkit2gtk4.1-devel \
  libappindicator-gtk3-devel \
  alsa-lib-devel \
  librsvg2-devel \
  openssl-devel \
  gcc \
  gcc-c++ \
  make \
  pkgconf-pkg-config
'
```

Ensure `curl`, `wget`, and `file` are also available (usually pre-installed).

### 2. Build the application

```bash
# Inside the toolbox:
toolbox run -c fedora-toolbox-44 bash -lc 'cd /path/to/somafm && pnpm install && pnpm tauri build --no-bundle'
```

The binary will be at `src-tauri/target/release/somafm`.

### 3. Install locally

Run the install script:

```bash
./scripts/install-linux.sh
```

This copies:
- Binary to `~/.local/bin/somafm`
- `.desktop` file to `~/.local/share/applications/somafm.desktop`
- Icon to `~/.local/share/icons/hicolor/128x128/apps/somafm.png`

Ensure `~/.local/bin` is in your `$PATH`. Then launch the app from your application menu or via:

```bash
somafm
```

### Runtime requirements

The host system must have these runtime libraries (installed once):

```bash
sudo dnf install webkit2gtk4.1 libappindicator-gtk3 alsa-lib
```

**Tray on GNOME:** Stock GNOME requires the [AppIndicator extension](https://extensions.gnome.org/extension/615/appindicator-support/). KDE shows the tray natively.

## macOS (unverified)

### Build

```bash
pnpm tauri build --bundles app
```

The bundle will be at `src-tauri/target/release/bundle/macos/somafm.app`.

### Install

Copy to Applications:

```bash
cp -r src-tauri/target/release/bundle/macos/somafm.app /Applications/
```

If the `.app` was copied from another machine, remove the quarantine flag:

```bash
xattr -dr com.apple.quarantine /Applications/somafm.app
```

**Notes:**
- The app is ad-hoc signed.
- The dock icon stays visible when the window is hidden.
- macOS may show a one-time security prompt on first launch.

## Windows (unverified)

### Build

```bash
pnpm tauri build --bundles nsis
```

The installer will be at `src-tauri/target/release/bundle/nsis/somafm_0.1.0_x64-setup.exe`.

### Install

Run the installer:

```
somafm_0.1.0_x64-setup.exe
```

Install per-user (no admin rights required). Windows Defender SmartScreen may warn about an unrecognized app (unsigned installer); click "More info" → "Run anyway".

**Notes:**
- Windows 10/11 includes WebView2 by default.
- The installation is per-user only.
- Launch from the Start menu or via `somafm.exe` from the installation directory.

## Development

For development builds with hot reload:

```bash
# On Linux, inside the toolbox:
pnpm tauri:dev

# On macOS/Windows:
pnpm tauri dev
```

## Verification

Run the test suite:

```bash
pnpm check
```

Or separately:

```bash
pnpm check:rust      # Rust: fmt, clippy, tests
pnpm check:web       # TypeScript, Svelte, and frontend tests
```

(On Linux inside the toolbox: `pnpm check:toolbox`)

## Troubleshooting

- **Build fails on Linux:** Ensure you are inside the toolbox and all dependencies are installed.
- **App won't start:** Check that `~/.local/bin` is in your `$PATH`: `echo $PATH | grep .local/bin`.
- **Tray icon missing on GNOME:** Install the AppIndicator extension (link above).
- **Settings not found after reinstall:** Settings are stored under the bundle identifier `dev.defjunx.somafm` and persist across reinstalls.
