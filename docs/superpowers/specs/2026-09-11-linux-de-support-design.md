# Multi-DE Support Design

Date: 2026-09-11
Status: Approved (Scope C broad, Parity C, Accurate-first A, Option 1 backend chain)

## Decisions (locked)

- **Scope C (broad):** KDE, Hyprland, Sway, XFCE, Cinnamon, MATE, COSMIC + Idle fallback.
  XFCE / Cinnamon / MATE are X11 sessions: covered by the X11 backend, no per-DE code.
- **Parity C:** tracking + autostart + tray on every DE. No per-DE feature matrix beyond that.
- **Accurate-first A:** native per-DE APIs first; generic X11 only where native is
  unavailable; `Idle` (`("","")`) only as last resort. Never prefer a generic source
  when a native one probes OK.
- **Option 1 backend chain:** one `AnyBackend` enum in the daemon, first probe win,
  single 1s `run_tracker_loop` unchanged.

## 1/3 — Backend chain (`wisp-daemon/src/backends.rs`)

New module `backends.rs` exposing one type implementing the existing
`wisp_core::tracker::WindowSource` trait (`wisp-core/src/tracker.rs:17`):

```rust
pub enum AnyBackend { Gnome(GnomeBackend), Kde(KdeBackend), Hyprland(HyprlandBackend),
                      Sway(SwayBackend), Cosmic(CosmicBackend), X11(X11Backend), Idle }
```

`GnomeBackend` moves as-is from `wisp-daemon/src/gnome.rs` (poll `GetActive` on
`com.saqr.wisp.WindowSource` `/com/saqr/wisp/WindowSource`). Probe order is fixed:

1. **GNOME (existing)** — `GnomeBackend::new()` (`NameHasOwner` check). Code moves unchanged
   from `wisp-daemon/src/gnome.rs` into `backends.rs`; `gnome.rs` is deleted.
2. **KDE** — KWin script `packaging/kwin-wisp/contents/code/main.js` pushes active-window
   changes via `callDBus("com.saqr.wisp.WindowSource", "/com/saqr/wisp/WindowSource",
   "com.saqr.wisp.WindowSource", "PushActive", app, title)`. The daemon's existing D-Bus
   server (`wisp-daemon/src/dbus_api.rs`) gains a `PushActive(app: String, title: String)`
   method that writes an `Arc<Mutex<(String, String)>>` cache. `KdeBackend` holds a clone
   of that `Arc`; its `active_window()` clones the cached pair (starts `("","")`, fills
   on first push). `KdeBackend::new(cache)` returns `Some` when the desktop hint is KDE
   or the kwin-wisp script dir exists — a push backend cannot prove liveness
   synchronously, so construction is hint-based, not handshake-based.
3. **Hyprland** — request socket `$XDG_RUNTIME_DIR/hypr/$HYPRLAND_INSTANCE_SIGNATURE/.socket.sock`:
   send `j/activewindow`, parse JSON `{class, title}` → `(class, title)`. Fallback:
   `hyprctl activewindow -j`. Missing socket/`hyprctl` ⇒ probe fails.
4. **Sway** — `$SWAYSOCK` i3-ipc: frame = `"i3-ipc"` + len u32 LE + type u32 LE (`4` =
   `GET_TREE`), empty payload; walk the JSON tree for the `focused` node; app =
   `app_id` else `window_properties.class` else `""`, title = `name` else `""`.
   Missing/unreachable `$SWAYSOCK` ⇒ probe fails.
5. **COSMIC** — `ext-foreign-toplevel-list-v1`, falling back to `wlr-foreign-toplevel`.
   Probe fails when the compositor offers neither.
6. **X11** — `x11rb`: `_NET_ACTIVE_WINDOW` on root → `_NET_WM_NAME` (fallback `WM_NAME`)
   + `WM_CLASS` of the active window. `DISPLAY` unset/unreachable ⇒ probe fails. Covers
   XFCE, Cinnamon, MATE, and Plasma/X11 sessions with no extra code.
7. **Idle** — always constructs, returns `("","")`.

Backend rules: `active_window()` never blocks (socket connects with a short timeout;
any error → `("","")` for that poll). Poll cadence stays 1s — owned by
`run_tracker_loop`, not the backends. Reuse `zbus`, `serde_json`; add `x11rb`;
`wayland-client` only if the COSMIC implementation proves to need it — otherwise
COSMIC falls through to X11/Idle with no new hard dependency.

## 2/3 — Detection + fallback (`wisp-daemon/src/main.rs`)

```rust
fn detect_desktop() -> Vec<String>;   // normalized hints, strongest first
fn pick_backend() -> Option<AnyBackend>;
fn wait_for_backend() -> HotBackend;  // never blocks the tracker
```

- **Detect:** split `XDG_CURRENT_DESKTOP` on `:`/`;`, trim, lowercase; append `DESKTOP_SESSION`
  (same normalization) as fallback hints. Environment signals override/confirm:
  `HYPRLAND_INSTANCE_SIGNATURE` ⇒ hyprland, `SWAYSOCK` ⇒ sway, `WAYLAND_DISPLAY` +
  COSMIC hint ⇒ cosmic, `WAYLAND_DISPLAY` without a known hint ⇒ Wayland-generic,
  `DISPLAY` without Wayland ⇒ X11. `XDG_SESSION_TYPE` disambiguates `x11` vs `wayland`.
- **pick_backend():** probe the hinted backend first, then the full chain in §1 order;
  first `Some` wins. Each probe is soft (`None` on any failure, never panic) and
  the winner is logged (`println!`, matching the existing `"GNOME Shell extension backend active"`).
  A hint never skips probing — it only orders candidates; an unresponsive hinted backend
  falls through to the next probe.
- **wait_for_backend():** try `pick_backend()` once. On success return `HotBackend` wrapping
  it; on total failure return `HotBackend` wrapping `Idle` so MPRIS (`mpris.rs`), logind
  (`logind.rs`), the file watcher, and the D-Bus API keep running. A background thread
  retries `pick_backend()` every 5s and hot-swaps the inner backend on first success
  (logs the swap). This replaces the current block-forever loop (`main.rs:23`), which
  would stall all tracking on DEs without the GNOME extension.
- **Hot-swap without changing the tracker:** `run_tracker_loop(db, backend, …)` takes
  `backend: impl WindowSource` by value, so the swappable handle must itself be the
  `WindowSource`:

```rust
pub struct HotBackend { current: Arc<Mutex<AnyBackend>> }
impl WindowSource for HotBackend {
    fn active_window(&mut self) -> (String, String) {
        self.current.lock().unwrap_or_else(|e| e.into_inner()).active_window()
    }
}
```

`wait_for_backend()` spawns the 5s retry thread holding a clone of the `Arc`; the tracker
loop keeps polling `HotBackend` at 1s, unaffected by swaps. `run_tracker_loop` itself
(`wisp-core/src/tracker.rs:55`) is unchanged.

## 3/3 — Autostart + tray + testing

### Autostart (primary mechanism, all DEs)

Daemon owns `install_autostart()` / `uninstall_autostart()` writing
`~/.config/autostart/wisp.desktop`:

```ini
[Desktop Entry]
Type=Application
Name=Wisp
Exec=<absolute path of current wisp-daemon>
NoDisplay=true
X-GNOME-Autostart-enabled=true
```

`Exec` is resolved from `std::env::current_exe()` at install time. No `OnlyShowIn`
(the entry must start on every DE). CLI: `wisp-daemon --install` writes the file and
exits 0; `wisp-daemon --uninstall` removes it and exits 0; both are parsed in `main()`
before normal startup. The Settings UI toggle calls these via a tauri command that
spawns the daemon binary with the flag (daemon is the single writer of the file).

### `wisp.service` (optional, loosened)

Keep `systemd::install()` (`wisp-daemon/src/systemd.rs`) as best-effort, but loosen the
unit so it is valid outside GNOME:

```ini
[Unit]
Description=Wisp Activity Tracker
After=default.target

[Service]
Type=dbus
BusName=com.saqr.wisp
ExecStart=<abs>/wisp-daemon
Restart=on-failure
RestartSec=2

[Install]
WantedBy=default.target
```

(Drops `BindsTo=graphical-session.target` / `After=graphical-session.target` /
`WantedBy=graphical-session.target`, which don't hold on all DEs.) Install failures
stay ignored; the `.desktop` entry is the primary autostart path.

### Tray (shared setup)

Move `TrayIconBuilder` out of the Windows-only `setup` (`wisp-ui/src-tauri/src/lib.rs:599`)
into the common builder setup with a minimal `Menu` (reuse the existing show/quit items;
an empty menu breaks some Linux hosts, so keep at least those two). Linux dependency:
`libayatana-appindicator3` (+ dev headers at build time); document in README/packaging.

### Testing

- `cargo test`: table-driven `detect_desktop()` cases (`GNOME`, `KDE`, `Hyprland`,
  `XFCE`/`Cinnamon`/`MATE`, `COSMIC`, multi-value `XDG_CURRENT_DESKTOP="KDE;GNOME"`,
  `DESKTOP_SESSION` fallback, signal-env overrides) + autostart write/remove roundtrip
  against a temp `HOME`.
- Manual matrix before release: GNOME 50, Plasma 6 (Wayland + X11), Hyprland, Sway,
  XFCE, COSMIC — verify winner log line, live window change, reboot autostart, tray icon.
