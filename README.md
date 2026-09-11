# Wisp

![CI](https://img.shields.io/github/actions/workflow/status/Saqr-Sayed/wisp/windows-smoke.yml?branch=master)
![License](https://img.shields.io/github/license/Saqr-Sayed/wisp)
![Release](https://img.shields.io/github/v/release/Saqr-Sayed/wisp)

A private, self-hosted **activity & time tracker** for Linux (GNOME, KDE,
Hyprland, Sway, XFCE/Cinnamon/MATE via X11, COSMIC fallback) and
Windows. Wisp records what you actually do on the computer — the apps and
windows you use, the music and video you play, files you touch, and system
events (boot, login, sleep, wake, shutdown) — into a local SQLite database.
No accounts, no cloud, no telemetry. The data is yours and stays on your
machine.

**Interface**: Arabic + English, follow-system theming, desktop notifications.
**Storage**: single SQLite file — `~/.local/share/wisp/activity.db` (Linux) /
`%LOCALAPPDATA%\wisp\activity.db` (Windows).

## Features

- **App & window tracking** — every foreground window is recorded with
  timestamps; apps get a friendly name automatically (and you can override
  any of them).
- **Media tracking** — music/video players (mpv, VLC, Spotify, …) are
  recorded with the actual track/film title:
  - Linux: MPRIS metadata with content-type mime detection.
  - Windows: System Media Transport Controls (SMTC).
- **File events** — file create / delete / rename in your user folders
  (Desktop, Documents, Downloads, Pictures, Videos, Music), debounced and
  summarized (e.g. `5 files in Downloads`).
- **Session events** — boot, login, logout, sleep, wake, power off.
- **Analytics** — timeline view, daily/weekly overview with averages, per-app
  and per-category reports, series/episode trees for movies & TV, search.
- **Productivity control** — per-app/website limits with alerts, category
  assignment, archive & ignore lists.
- **Data control** — every table editable from the settings page; raw DB in
  one file, easy to back up (`cp activity.db backup.db`).

## Architecture

| Component | Role |
|---|---|
| `wisp-core` | shared library: SQLite schema, classifier/enrichment, tracker loop, file watcher, system-event store. 103 unit tests. |
| `wisp-daemon` (Linux only) | D-Bus service (`com.saqr.wisp`) that polls the active window (via GNOME extension OR KWin script OR native sockets: Hyprland socket/hyprctl, Sway i3-ipc, X11), reads MPRIS media, watches files and logind sessions. |
| `wisp-ui` (Tauri v2) | desktop app. On Linux it talks to the daemon over D-Bus; on Windows it **runs the tracker in-process** (no daemon needed) and shows the same UI (Vue 3). |
| GNOME Shell extension `wisp@saqr` | provides active-window info on GNOME. |
| KWin script `kwin-wisp` | pushes active-window info on KDE/Plasma (Hyprland/Sway/X11 need no extra component; COSMIC falls back to idle). |

---

## Installation — Linux

### Option A: release packages (recommended)

Releases are tagged `v1`, `v2`, … and every release ships the same asset
names — `wisp-<tag>-<platform>-<arch>[-<kind>].<ext>`. Download from the
[releases page](https://github.com/Saqr-Sayed/wisp/releases) (e.g. for v1):

- RPM (Fedora/RHEL/OpenSUSE): `wisp-v1-linux-x86_64.rpm`
- DEB (Debian/Ubuntu/Mint): `wisp-v1-linux-x86_64.deb`
- Windows installer (NSIS): `wisp-v1-windows-x86_64-setup.exe`
- Windows MSI: `wisp-v1-windows-x86_64.msi`
- Windows portable exe: `wisp-v1-windows-x86_64.exe`
- Linux daemon binary tarball: `wisp-v1-linux-x86_64-daemon.tar.gz`
- Linux GNOME extension (zip): `wisp-v1-linux-x86_64-gnome-extension.zip`
- Linux KWin script (zip): `wisp-v1-linux-x86_64-kwin-wisp.zip`

```bash
# Fedora/derivatives:
sudo dnf install ./wisp-v1-linux-x86_64.rpm

# Debian/derivatives:
sudo apt install ./wisp-v1-linux-x86_64.deb
```

> **Note:** the tray icon needs `libayatana-appindicator3` at runtime
> (Debian/Ubuntu: `libayatana-appindicator3-1`; Fedora: `libappindicator-gtk3`).
> If the tray icon is missing, install that package and restart Wisp.
> Autostart is a shared tray toggle (Settings) backed by
> `~/.config/autostart/wisp.desktop` (`wisp-daemon --install` / `--uninstall`).

Then install the **daemon** plus the backend for your DE (GNOME extension OR
KWin script; Hyprland/Sway/X11 need no extra component):

```bash
# daemon + systemd service (tarball from the same release):
tar xzf wisp-v1-linux-x86_64-daemon.tar.gz -C ~/.local/bin    # wisp-daemon, wisp.service
mkdir -p ~/.config/systemd/user
cp ~/.local/bin/wisp.service ~/.config/systemd/user/

# GNOME Shell extension (zip from the same release, extracts to wisp@saqr/):
#   (Fedora: sudo dnf install unzip if missing)
unzip wisp-v1-linux-x86_64-gnome-extension.zip -d ~/.local/share/gnome-shell/extensions/

# KDE/Plasma only — KWin script (zip from the same release):
#   (Fedora: sudo dnf install unzip if missing)
unzip wisp-v1-linux-x86_64-kwin-wisp.zip -d ~/.local/share/kwin/scripts/
kpackagetool6 --type=KWin/Script -i ~/.local/share/kwin/scripts/kwin-wisp 2>/dev/null || true
qdbus6 org.kde.KWin /KWin reconfigure 2>/dev/null || qdbus org.kde.KWin /KWin reconfigure 2>/dev/null || true
```

Enable and start the services:

```bash
systemctl --user daemon-reload
systemctl --user enable --now wisp.service        # starts the daemon
# GNOME only:
gnome-extensions enable wisp@saqr                 # then restart GNOME Shell (Alt+F2 → r)
# KDE only: enable kwin-wisp in System Settings → Window Management → KWin Scripts (then Apply)
```

### Option B: build from source

```bash
# Requirements: Rust (rustup), Node.js 18+, cargo dependencies:
#   dnf install webkit2gtk4.1-devel gtk3-devel libappindicator-gtk3-devel rpm-build
#   (Debian: libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev)

cargo build --release -p wisp-daemon
cp target/release/wisp-daemon ~/.local/bin/

cd wisp-ui
npm ci && npm run tauri build        # produces .deb + .rpm in target/release/bundle/

# install the daemon service + backend (ships in the repo under packaging/):
mkdir -p ~/.config/systemd/user ~/.local/share/gnome-shell/extensions
cp ../packaging/wisp.service ~/.config/systemd/user/
# GNOME:
cp -r ../packaging/wisp@saqr ~/.local/share/gnome-shell/extensions/
# KDE/Plasma instead:
# cp -r ../packaging/kwin-wisp ~/.local/share/kwin/scripts/
# kpackagetool6 --type=KWin/Script -i ~/.local/share/kwin/scripts/kwin-wisp
# (Hyprland/Sway/X11 need no extra component)
systemctl --user daemon-reload && systemctl --user enable --now wisp.service
```

### Verifying the Linux install

```bash
systemctl --user status wisp.service        # should be active
journalctl --user -u wisp.service -f        # "file watcher active on …" + "<DE> backend active" (GNOME/KDE/Hyprland/Sway/X11)
sqlite3 ~/.local/share/wisp/activity.db \
  "SELECT detail, datetime(start_time,'unixepoch') FROM activity_logs WHERE event_type='system' ORDER BY id DESC LIMIT 10;"
# expected kinds: boot, login, sleep, wake, file_created, file_deleted, …
```

Open the app (`wisp-ui`) — the tray icon appears; closing the window keeps it
in the tray.

---

## Installation — Windows

### Option A: installer (recommended)

1. Download `wisp-v1-windows-x86_64-setup.exe` (NSIS) or
   `wisp-v1-windows-x86_64.msi` from the
   [releases page](https://github.com/Saqr-Sayed/wisp/releases).
2. Run it — per-user install, no admin needed.
3. The binary is unsigned, so SmartScreen may warn: **More info → Run anyway**.
4. On first run Wisp registers autostart, creates a tray icon, and starts
   tracking. No daemon, no extra components — everything runs in the app
   process.

### Option B: build from source (Windows 10/11)

```powershell
# Requirements: Rust MSVC toolchain (rustup default stable-msvc),
# Visual Studio Build Tools 2022 (Desktop development with C++), Node.js 18+
cd wisp-ui
npm ci
npm run tauri build          # produces NSIS + MSI; release uploads rename
                             # them to wisp-<tag>-windows-x86_64[-setup].*
```

### Verifying the Windows install

```powershell
# After using the app for a minute, check the database
# (e.g. with DB Browser for SQLite, or sqlite3 from the Python distribution):
sqlite3 "$env:LOCALAPPDATA\wisp\activity.db" `
  "SELECT detail, datetime(start_time,'unixepoch') FROM activity_logs WHERE event_type='system' ORDER BY id DESC LIMIT 10;"
# healthy DB shows: boot, login, and after lock/unlock: sleep/wake
```

---

## How tracking works

- **Polling loop** — every second, the current foreground window is read
  (GNOME extension on Linux, Win32 API on Windows) and stored with start/end
  times. Idle gaps become `sleep` rows.
- **Media** — the media title is looked up from the player (MPRIS on Linux,
  SMTC on Windows) and attached to the app rows; content type (`watching` vs
  `listening`) comes from the mime type on Linux.
- **Files** — watched folders are monitored with a 3-second debounce, so a
  burst of save events collapses into one row.
- **Sessions** — logind (Linux) / WTS terminal-session messages (Windows)
  provide lock/unlock/shutdown; boot/login are detected from system uptime.

## Privacy

- Everything runs locally; the app never phones home. No analytics SDK, no
  network calls (the D-Bus/WebView channels are local).
- The database is plain SQLite — you can inspect or delete it at any time.

## License

[MIT](LICENSE) — © 2026 Sayed Saqr. Free to use, modify and redistribute.

## Documentation & support

- `docs/RELEASE.md` — full release/building details for both platforms.
- `DESIGN.md`, `PRODUCT.md` — design and product notes.
- File an issue on GitHub for bugs or feature requests.