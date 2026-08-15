# Wisp

A private, self-hosted **activity & time tracker** for Linux (GNOME) and
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
| `wisp-daemon` (Linux only) | D-Bus service (`com.saqr.wisp`) that polls the active window (via the GNOME Shell extension), reads MPRIS media, watches files and logind sessions. |
| `wisp-ui` (Tauri v2) | desktop app. On Linux it talks to the daemon over D-Bus; on Windows it **runs the tracker in-process** (no daemon needed) and shows the same UI (Vue 3). |
| GNOME Shell extension `wisp@saqr` | provides active-window info to the daemon. |

---

## Installation — Linux

### Option A: release packages (recommended)

Releases are tagged `v1`, `v2`, … (package filenames keep full semver
versions like `1.0.0`). Download from the
[releases page](https://github.com/Saqr-Sayed/wisp/releases):

- RPM (Fedora/RHEL/OpenSUSE): `Wisp-1.0.0-1.x86_64.rpm`
- DEB (Debian/Ubuntu/Mint): `Wisp_1.0.0_amd64.deb`

```bash
# Fedora/derivatives:
sudo dnf install ./Wisp-1.0.0-1.x86_64.rpm

# Debian/derivatives:
sudo apt install ./Wisp_1.0.0_amd64.deb
```

Then install the **daemon** and the **GNOME Shell extension**:

```bash
# daemon (binary tarball from the same release):
tar xzf wisp-daemon-1.0.0-linux-x86_64.tar.gz -C ~/.local/bin

# GNOME Shell extension (from source, one command):
mkdir -p ~/.local/share/gnome-shell/extensions
cp -r src/gnome-extension/wisp@saqr ~/.local/share/gnome-shell/extensions/
```

Enable and start the services:

```bash
systemctl --user enable --now wisp.service        # starts the daemon
gnome-extensions enable wisp@saqr                 # then restart GNOME Shell (Alt+F2 → r)
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

# install the daemon service + extension (ships in the repo under packaging/):
mkdir -p ~/.config/systemd/user ~/.local/share/gnome-shell/extensions
cp ../packaging/wisp.service ~/.config/systemd/user/
cp -r ../packaging/wisp@saqr ~/.local/share/gnome-shell/extensions/
systemctl --user daemon-reload && systemctl --user enable --now wisp.service
```

### Verifying the Linux install

```bash
systemctl --user status wisp.service        # should be active
journalctl --user -u wisp.service -f        # "file watcher active on …" + "GNOME Shell extension backend active"
sqlite3 ~/.local/share/wisp/activity.db \
  "SELECT detail, datetime(start_time,'unixepoch') FROM activity_logs WHERE event_type='system' ORDER BY id DESC LIMIT 10;"
# expected kinds: boot, login, sleep, wake, file_created, file_deleted, …
```

Open the app (`wisp-ui`) — the tray icon appears; closing the window keeps it
in the tray.

---

## Installation — Windows

### Option A: installer (recommended)

1. Download `Wisp_1.0.0_x64-setup.exe` (NSIS) or `.msi` from the
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
npm run tauri build          # outputs bundle\nsis\Wisp_1.0.0_x64-setup.exe (+ .msi)
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