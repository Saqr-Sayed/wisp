# Wisp — Build & Release

## Linux

### Build

Prerequisites: Rust, Node.js, `webkit2gtk4.1` + `librsvg` dev packages (see
Tauri v2 Linux docs), `dpkg-deb`/`rpmbuild` **not** required (Tauri v2 bundles
both formats in pure Rust).

```sh
cd wisp-ui && npm install
npm run tauri build          # bundles .deb + .rpm into src-tauri/target/release/bundle/
cargo build --release -p wisp-daemon   # from workspace root
```

Outputs (Fedora 44, x86_64):
- `target/release/bundle/deb/Wisp_1.0.0_amd64.deb`
- `target/release/bundle/rpm/Wisp-1.0.0-1.x86_64.rpm`
- `target/release/wisp-ui`, `target/release/wisp-daemon` (raw binaries)

### Release assets — unified naming scheme

Every release ships the same asset names, keyed to the tag (`v1`, `v2`, …):

| Asset | Source |
|-------|--------|
| `wisp-<tag>-windows-x86_64.exe` | CI `build-installers` (renamed from `wisp-ui.exe`) |
| `wisp-<tag>-windows-x86_64-setup.exe` | CI `build-installers` (NSIS) |
| `wisp-<tag>-windows-x86_64.msi` | CI `build-installers` (MSI) |
| `wisp-<tag>-linux-x86_64.deb` | `bundle/deb/` (renamed) |
| `wisp-<tag>-linux-x86_64.rpm` | `bundle/rpm/` (renamed) |
| `wisp-<tag>-linux-x86_64-daemon.tar.gz` | `tar czf … target/release/wisp-daemon` |
| `wisp-<tag>-linux-x86_64-gnome-extension.zip` | CI `build-installers` (zip of `packaging/wisp@saqr`) |

The CI workflow renames the Windows bundles automatically; rename the Linux
outputs locally before `gh release upload`.

Note: AppImage bundling is disabled in `tauri.conf.json` (`targets: ["deb","rpm"]`).

### Install (user-level, no root)

1. Copy binaries:
   ```sh
   cp target/release/wisp-ui target/release/wisp-daemon ~/.local/bin/
   ```
2. Desktop entry: `~/.local/share/applications/wisp.desktop` with
   `Exec=/home/<user>/.local/bin/wisp-ui`, `Icon=wisp-ui`, plus the icon at
   `~/.local/share/icons/hicolor/{32x32,128x128,256x256@2}/apps/wisp-ui.png`
   (copy from the .deb payload).
3. Tracker service (systemd user unit, start the daemon at login):
   `~/.config/systemd/user/wisp.service`:
   ```ini
   [Unit]
   Description=Wisp Activity Tracker
   After=graphical-session.target
   BindsTo=graphical-session.target

   [Service]
   Type=dbus
   BusName=com.saqr.wisp
   ExecStart=/home/<user>/.local/bin/wisp-daemon
   Restart=on-failure
   RestartSec=2

   [Install]
   WantedBy=graphical-session.target
   ```
   ```sh
   systemctl --user daemon-reload && systemctl --user enable --now wisp
   ```
4. GNOME Shell extension `wisp@saqr` must be installed for window tracking:
   `~/.local/share/gnome-shell/extensions/wisp@saqr/` (enable via
   `gnome-extensions enable wisp@saqr`).

Data: `~/.local/share/wisp/activity.db`.

## Windows

### What works there

On Windows the app **runs its own tracker in-process** (`Win32Backend` —
foreground-window polling, installed in `lib.rs`); there is **no daemon and no
GNOME extension needed**. Uninstall/autostart are handled by the app (registry
autostart). Closing the window hides to tray.

Known limitation: media/session tagging relies on the foreground window being
a player window. On Windows:

- **Media**: tracked via the System Media Transport Controls (SMTC) session
  manager (`windows_media.rs`), matching the playing app's AUMID against the
  active window; MPRIS content-type mime sniffing stays Linux-only.
- **Session events**: lock → `sleep`, unlock → `wake`, shutdown → `power_off`
  via a hidden WTS-registered message window (`windows_session.rs`); boot and
  login are recorded on first run after each boot. GNOME Shell "session"
  integration (logind) stays Linux-only.
- **File events**: the shared `wisp-core::watcher::spawn_file_watcher` watches
  the user's Desktop/Documents/Downloads/Pictures/Videos/Music (same debounced
  3s store as the Linux daemon).

### Cross-checking the Windows build from Linux

`x86_64-pc-windows-msvc` is installed via rustup. The Rust side compiles
freely; the bundled SQLite (rusqlite `bundled`) needs MSVC headers — fetch
them once with `xwin splat --output ~/.xwin` (license prompt: `yes`), then:

```sh
cd wisp-ui/src-tauri
env CC_x86_64_pc_windows_msvc=clang-cl \
    CFLAGS_x86_64_pc_windows_msvc="/imsvc $HOME/.xwin/crt/include /imsvc $HOME/.xwin/sdk/include/ucrt /imsvc $HOME/.xwin/sdk/include/um /imsvc $HOME/.xwin/sdk/include/shared" \
    AR_x86_64_pc_windows_msvc=llvm-ar \
    cargo check --target x86_64-pc-windows-msvc -p wisp-ui
```

Bundling still requires a real Windows machine (Tauri NSIS/MSI tooling).

### Build (on a Windows machine)

Prerequisites:
- Windows 10/11 — WebView2 runtime is preinstalled.
- Rust with MSVC toolchain: https://rustup.rs (`rustup default stable-msvc`).
- Visual Studio Build Tools 2022, workload "Desktop development with C++".
- Node.js 18+ and npm.

```powershell
cd wisp-ui
npm install
npm run tauri build
```

Output: `wisp-ui\src-tauri\target\release\bundle\nsis\Wisp_1.0.0_x64-setup.exe`
(plus an `.msi`) — the CI `build-installers` job renames them to the unified
scheme (`wisp-<tag>-windows-x86_64[-setup].*`). The first Windows build takes
several minutes (full Rust compile); rebuilds are incremental.

Cross-compiling from Linux is not supported for Tauri bundling — build on
Windows or in a Windows CI runner.

### Install

Run the installer (`wisp-<tag>-windows-x86_64-setup.exe`). The binary is unsigned, so
SmartScreen may warn — "More info → Run anyway". The app installs
per-user, registers autostart on first run, and stores data in
`%LOCALAPPDATA%\wisp\activity.db`.