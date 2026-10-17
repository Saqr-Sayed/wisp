# Linux Tracking Backends Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add multi-DE window tracking to wisp-daemon behind one `AnyBackend` enum with first-probe-wins selection and hot-swap retry.

**Architecture:** All backend logic lives in one new module `wisp-daemon/src/backends.rs` (detection, every backend, `pick_backend`, `HotBackend`, `wait_for_backend`); `main.rs` becomes thin wiring, `dbus_api.rs` gains one `PushActive` method plus a shared cache, and a KWin script feeds KDE pushes. Each task leaves the tree compiling and tested.

**Tech Stack:** Rust (edition 2021), zbus 4 (blocking probe + async D-Bus server), serde_json, x11rb 0.13, KWin JavaScript scripting API (`callDBus`).

**Scope note (tracking only):** This plan covers spec sections 1/3 (backend chain) and 2/3 (detection + fallback). Autostart (`--install`/`--uninstall`, `wisp.desktop`), the loosened `wisp.service` unit, and tray changes are separate plans, not this one.

## Global Constraints

- Probe order is fixed: GNOME → KDE → Hyprland → Sway → COSMIC → X11 → Idle; first `Some` wins.
- Poll cadence stays 1s, owned by `run_tracker_loop`, never by backends.
- `active_window()` never blocks: socket connects use ≤300ms timeouts; any error returns `("","")` for that poll.
- Accurate-first: native per-DE APIs first; generic X11 only where native is unavailable; `Idle` (`("","")`) only as last resort.
- Each probe is soft: `None` on any failure, never panic; the winner is logged with `println!`.
- `run_tracker_loop` (`wisp-core/src/tracker.rs:55`) is unchanged.
- Dependencies: reuse `zbus` and `serde_json`; add `x11rb` only; no `wayland-client` in this plan (COSMIC falls through to X11/Idle).
- XFCE / Cinnamon / MATE are X11 sessions covered by the X11 backend with no per-DE code.
- KWin script D-Bus push targets service `com.saqr.wisp.WindowSource`, path `/com/saqr/wisp/WindowSource`, interface `com.saqr.wisp.WindowSource`, method `PushActive(app: String, title: String)`.

---

## File Structure Map

- **Create `wisp-daemon/src/backends.rs`** — owns everything: `ActiveCache` type alias + `new_active_cache()`, `detect_desktop()`, `chain_order()`, `GnomeBackend`, `HyprlandBackend` (+ `parse_hypr_json`), `SwayBackend` (+ `find_focused`), `KdeBackend`, `X11Backend`, `CosmicBackend`, `AnyBackend` enum + its `WindowSource` impl, `pick_backend()`, `HotBackend` + its `WindowSource` impl, `wait_for_backend()`, and all unit tests.
- **Delete `wisp-daemon/src/gnome.rs`** — code moves as-is into `backends.rs` (Task 2 moves it, Task 7 removes the file).
- **Modify `wisp-daemon/src/main.rs`** — Task 1 adds `mod backends;`; Task 5 updates the `serve()` destructure to the new 3-tuple; Task 7 deletes old `pick_backend`/`wait_for_backend`, switches to `backends::wait_for_backend`, keeps the `run_tracker_loop` call identical.
- **Modify `wisp-daemon/src/dbus_api.rs`** — add `WindowSourcePush` interface with `push_active`, extend `serve()` to claim the second bus name, serve the second path, and return the cache (Task 5).
- **Modify `wisp-daemon/Cargo.toml`** — add `x11rb = "0.13"` (Task 6).
- **Create `packaging/kwin-wisp/contents/code/main.js`** — KWin script pushing active-window changes via `callDBus` (Task 5).
- **Create `packaging/kwin-wisp/metadata.json`** — KWin script plugin metadata (Task 5).

Canonical signatures (every task uses exactly these; do not rename):

```rust
pub type ActiveCache = std::sync::Arc<std::sync::Mutex<(String, String)>>;
pub fn new_active_cache() -> ActiveCache;
pub fn detect_desktop() -> Vec<String>;
pub fn chain_order(hints: &[String]) -> Vec<&'static str>;
pub fn pick_backend(kde_cache: &ActiveCache) -> Option<AnyBackend>;
pub fn wait_for_backend(kde_cache: ActiveCache) -> HotBackend;
pub enum AnyBackend { Gnome(GnomeBackend), Kde(KdeBackend), Hyprland(HyprlandBackend), Sway(SwayBackend), Cosmic(CosmicBackend), X11(X11Backend), Idle }
#[derive(Clone)] pub struct HotBackend { current: std::sync::Arc<std::sync::Mutex<AnyBackend>> }
impl HotBackend { pub fn new(initial: AnyBackend) -> Self; }
pub async fn serve(db: std::sync::Arc<Db>) -> zbus::Result<(zbus::Connection, ActivityTracker, ActiveCache)>;
```

---

### Task 1: DE detection (`detect_desktop`)

**Files:**
- Create: `wisp-daemon/src/backends.rs`
- Modify: `wisp-daemon/src/main.rs:1` (add `mod backends;`)
- Test: `wisp-daemon/src/backends.rs` (`#[cfg(test)] mod tests`)

**Interfaces:**
- Consumes: nothing (first task; only std env + `dirs` already in Cargo.toml is not needed yet).
- Produces: `pub type ActiveCache`, `pub fn new_active_cache() -> ActiveCache`, `pub fn detect_desktop() -> Vec<String>` for Tasks 5–7.

- [ ] **Step 1: Write the failing test**

Append to the new file `wisp-daemon/src/backends.rs` (file does not exist yet, so this step creates it with types + test module only, no `detect_desktop` body):

```rust
use std::sync::{Arc, Mutex};

/// Shared last-pushed (app, title) pair. Created in `dbus_api::serve`,
/// cloned into `KdeBackend`. Starts ("",""), fills on first PushActive.
pub type ActiveCache = Arc<Mutex<(String, String)>>;

pub fn new_active_cache() -> ActiveCache {
    Arc::new(Mutex::new((String::new(), String::new())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    struct EnvGuard {
        saved: Vec<(String, Option<String>)>,
    }

    impl EnvGuard {
        fn set(pairs: &[(&str, Option<&str>)]) -> Self {
            let mut saved = Vec::new();
            for (k, v) in pairs {
                saved.push((k.to_string(), std::env::var(k).ok()));
                match v {
                    Some(val) => std::env::set_var(k, val),
                    None => std::env::remove_var(k),
                }
            }
            EnvGuard { saved }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            for (k, v) in &self.saved {
                match v {
                    Some(val) => std::env::set_var(k, val),
                    None => std::env::remove_var(k),
                }
            }
        }
    }

    pub(super) fn hints_with(
        xdg: Option<&str>,
        session: Option<&str>,
        extra: &[(&str, Option<&str>)],
    ) -> Vec<String> {
        let _guard = env_lock().lock().unwrap();
        let _e1 = EnvGuard::set(&[
            ("XDG_CURRENT_DESKTOP", xdg),
            ("DESKTOP_SESSION", session),
            ("HYPRLAND_INSTANCE_SIGNATURE", None),
            ("SWAYSOCK", None),
            ("WAYLAND_DISPLAY", None),
            ("DISPLAY", None),
            ("XDG_SESSION_TYPE", None),
        ]);
        let _e2 = EnvGuard::set(extra);
        detect_desktop()
    }

    #[test]
    fn detect_splits_colons_and_semicolons() {
        assert_eq!(
            hints_with(Some("KDE;GNOME"), None, &[]),
            vec!["kde".to_string(), "gnome".to_string()]
        );
        assert_eq!(
            hints_with(Some("ubuntu:GNOME"), None, &[]),
            vec!["ubuntu".to_string(), "gnome".to_string()]
        );
    }

    #[test]
    fn detect_session_fallback_and_dedup() {
        assert_eq!(
            hints_with(None, Some("plasma"), &[]),
            vec!["plasma".to_string()]
        );
        assert_eq!(
            hints_with(Some("KDE"), Some("plasma"), &[]),
            vec!["kde".to_string(), "plasma".to_string()]
        );
    }

    #[test]
    fn detect_signal_env_overrides() {
        assert_eq!(
            hints_with(
                Some("GNOME"),
                None,
                &[("HYPRLAND_INSTANCE_SIGNATURE", Some("abc123"))]
            ),
            vec!["hyprland".to_string(), "gnome".to_string()]
        );
        assert_eq!(
            hints_with(
                None,
                None,
                &[("SWAYSOCK", Some("/run/user/1000/sway-ipc.sock"))]
            ),
            vec!["sway".to_string()]
        );
    }

    #[test]
    fn detect_display_fallbacks() {
        assert_eq!(
            hints_with(None, None, &[("DISPLAY", Some(":0"))]),
            vec!["x11".to_string()]
        );
        assert_eq!(
            hints_with(None, None, &[("WAYLAND_DISPLAY", Some("wayland-1"))]),
            vec!["wayland-generic".to_string()]
        );
    }
}
```

Note: `hints_with` is `pub(super)` so later tasks' tests in the same file reuse it; `EnvGuard` + `env_lock` serialize the env-mutating tests because Rust runs tests in parallel threads.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p wisp-daemon detect_splits_colons_and_semicolons`
Expected: FAIL with compilation error `cannot find function 'detect_desktop' in module 'backends'` (the test helper calls a function that does not exist yet).

- [ ] **Step 3: Write minimal implementation**

Add to `wisp-daemon/src/backends.rs`, directly after `new_active_cache`:

```rust
/// Normalized desktop hints, strongest first. Splits XDG_CURRENT_DESKTOP on
/// ':'/';', trims, lowercases; appends DESKTOP_SESSION as fallback hints.
/// HYPRLAND_INSTANCE_SIGNATURE and SWAYSOCK prepend their hint when present.
/// WAYLAND_DISPLAY without a known hint appends "wayland-generic"; DISPLAY
/// without Wayland appends "x11".
pub fn detect_desktop() -> Vec<String> {
    fn split_norm(s: &str) -> Vec<String> {
        s.split([':', ';'])
            .map(|p| p.trim().to_lowercase())
            .filter(|p| !p.is_empty())
            .collect()
    }
    let mut hints: Vec<String> = std::env::var("XDG_CURRENT_DESKTOP")
        .map(|v| split_norm(&v))
        .unwrap_or_default();
    for h in split_norm(&std::env::var("DESKTOP_SESSION").unwrap_or_default()) {
        if !hints.contains(&h) {
            hints.push(h);
        }
    }
    if !std::env::var("HYPRLAND_INSTANCE_SIGNATURE")
        .unwrap_or_default()
        .is_empty()
        && hints.first().map(|s| s.as_str()) != Some("hyprland")
    {
        hints.retain(|h| h != "hyprland");
        hints.insert(0, "hyprland".to_string());
    }
    if !std::env::var("SWAYSOCK").unwrap_or_default().is_empty()
        && hints.first().map(|s| s.as_str()) != Some("sway")
    {
        hints.retain(|h| h != "sway");
        hints.insert(0, "sway".to_string());
    }
    let wayland = !std::env::var("WAYLAND_DISPLAY")
        .unwrap_or_default()
        .is_empty();
    let display = !std::env::var("DISPLAY").unwrap_or_default().is_empty();
    let known = ["cosmic", "gnome", "kde", "plasma", "hyprland", "sway"];
    if wayland && !hints.iter().any(|h| known.contains(&h.as_str())) {
        hints.push("wayland-generic".to_string());
    }
    if display && !wayland && !hints.iter().any(|h| h == "x11") {
        hints.push("x11".to_string());
    }
    hints
}
```

Register the module in `wisp-daemon/src/main.rs:1`. Old line:

```rust
mod dbus_api; mod gnome; mod logind; mod mpris; mod systemd;
```

New line:

```rust
mod backends; mod dbus_api; mod gnome; mod logind; mod mpris; mod systemd;
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p wisp-daemon backends::tests`
Expected: PASS — `test result: ok. 4 passed; 0 failed`.

- [ ] **Step 5: Commit**

```bash
git add wisp-daemon/src/backends.rs wisp-daemon/src/main.rs
git commit -m "feat(daemon): add detect_desktop hints for multi-DE backends"
```

---

### Task 2: Move GNOME backend + `AnyBackend` with Idle

**Files:**
- Modify: `wisp-daemon/src/backends.rs` (append backends; `gnome.rs` stays until Task 7)
- Test: `wisp-daemon/src/backends.rs` (`mod tests`: append idle dispatch test)

**Interfaces:**
- Consumes: `wisp_core::tracker::WindowSource`, `pub type ActiveCache` from Task 1.
- Produces: `pub struct GnomeBackend` + `GnomeBackend::new() -> Option<Self>`, `pub enum AnyBackend { Gnome(GnomeBackend), Idle }` + its `WindowSource` impl for Tasks 6–7. Later tasks only add variants and match arms; these names and shapes are final.

- [ ] **Step 1: Write the failing test**

Append to the existing `mod tests` in `wisp-daemon/src/backends.rs`:

```rust
    #[test]
    fn idle_returns_empty_pair() {
        let mut b = AnyBackend::Idle;
        assert_eq!(b.active_window(), (String::new(), String::new()));
    }
```

No live-D-Bus test for `GnomeBackend::new`: the probe needs a session bus owning `com.saqr.wisp.WindowSource`, which CI lacks; GNOME coverage is the manual matrix in Task 8. The compile itself proves the move.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p wisp-daemon idle_returns_empty_pair`
Expected: FAIL with compilation error `cannot find type 'AnyBackend' in module 'backends'`.

- [ ] **Step 3: Write minimal implementation**

Append to `wisp-daemon/src/backends.rs` (after `detect_desktop`). This is a verbatim move of `wisp-daemon/src/gnome.rs:1-48`:

```rust
use wisp_core::tracker::WindowSource;
use zbus::blocking::Connection;

const DEST: &str = "com.saqr.wisp.WindowSource";
const PATH: &str = "/com/saqr/wisp/WindowSource";
const IFACE: &str = "com.saqr.wisp.WindowSource";

/// Queries the Wisp GNOME Shell extension for the active window.
/// The extension runs inside gnome-shell (GNOME 50 closed org.gnome.Shell.Introspect
/// behind an allowlist, so an extension is the only sanctioned way).
pub struct GnomeBackend {
    conn: Connection,
}

impl GnomeBackend {
    pub fn new() -> Option<Self> {
        let conn = Connection::session().ok()?;
        let reply = conn
            .call_method(
                Some(zbus::names::BusName::from_static_str("org.freedesktop.DBus").unwrap()),
                "/org/freedesktop/DBus",
                Some(zbus::names::InterfaceName::from_static_str("org.freedesktop.DBus").unwrap()),
                "NameHasOwner",
                &(DEST,),
            )
            .ok()?;
        let owned: bool = reply.body().deserialize().ok()?;
        if !owned {
            return None;
        }
        Some(GnomeBackend { conn })
    }
}

impl WindowSource for GnomeBackend {
    fn active_window(&mut self) -> (String, String) {
        match self.conn.call_method(
            Some(zbus::names::BusName::from_static_str(DEST).unwrap()),
            PATH,
            Some(zbus::names::InterfaceName::from_static_str(IFACE).unwrap()),
            "GetActive",
            &(),
        ) {
            Ok(reply) => reply.body().deserialize::<(String, String)>().unwrap_or_default(),
            Err(_) => (String::new(), String::new()),
        }
    }
}

/// First-probe-wins backend chain. Later tasks add Kde, Hyprland, Sway,
/// Cosmic, and X11 variants plus their match arms here.
pub enum AnyBackend {
    Gnome(GnomeBackend),
    Idle,
}

impl WindowSource for AnyBackend {
    fn active_window(&mut self) -> (String, String) {
        match self {
            AnyBackend::Gnome(b) => b.active_window(),
            AnyBackend::Idle => (String::new(), String::new()),
        }
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p wisp-daemon backends::tests`
Expected: PASS — `test result: ok. 5 passed; 0 failed`.

- [ ] **Step 5: Commit**

```bash
git add wisp-daemon/src/backends.rs
git commit -m "feat(daemon): move GnomeBackend into backends with AnyBackend+Idle"
```

---

### Task 3: Hyprland backend (socket + `hyprctl` fallback)

**Files:**
- Modify: `wisp-daemon/src/backends.rs` (add `Hyprland(HyprlandBackend)` variant + arm + struct)
- Test: `wisp-daemon/src/backends.rs` (`mod tests`: append Hyprland parse/socket-path tests)

**Interfaces:**
- Consumes: `detect_desktop` (not needed here), `WindowSource`, `AnyBackend` from Task 2.
- Produces: `pub struct HyprlandBackend` + `HyprlandBackend::new() -> Option<Self>`, `pub fn parse_hypr_json(s: &str) -> (String, String)` for Task 6 (`pick_backend`).

- [ ] **Step 1: Write the failing test**

Append to `mod tests` in `wisp-daemon/src/backends.rs`:

```rust
    #[test]
    fn hypr_parses_class_and_title() {
        assert_eq!(
            parse_hypr_json(r#"{"class":"firefox","title":"Docs — Firefox"}"#),
            ("firefox".to_string(), "Docs — Firefox".to_string())
        );
        assert_eq!(
            parse_hypr_json(r#"{"title":"only"}"#),
            (String::new(), "only".to_string())
        );
        assert_eq!(
            parse_hypr_json("not json"),
            (String::new(), String::new())
        );
    }

    #[test]
    fn hypr_socket_path_uses_runtime_dir_and_signature() {
        let _guard = env_lock().lock().unwrap();
        let _e = EnvGuard::set(&[
            ("XDG_RUNTIME_DIR", Some("/run/user/1000")),
            ("HYPRLAND_INSTANCE_SIGNATURE", Some("sig123")),
        ]);
        assert_eq!(
            HyprlandBackend::socket_path(),
            Some(std::path::PathBuf::from(
                "/run/user/1000/hypr/sig123/.socket.sock"
            ))
        );
        let _e2 = EnvGuard::set(&[("HYPRLAND_INSTANCE_SIGNATURE", None)]);
        assert_eq!(HyprlandBackend::socket_path(), None);
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p wisp-daemon hypr_parses_class_and_title`
Expected: FAIL with compilation error `cannot find function 'parse_hypr_json'`.

- [ ] **Step 3: Write minimal implementation**

Update the `AnyBackend` enum in `wisp-daemon/src/backends.rs`. Old block:

```rust
pub enum AnyBackend {
    Gnome(GnomeBackend),
    Idle,
}
```

New block:

```rust
pub enum AnyBackend {
    Gnome(GnomeBackend),
    Hyprland(HyprlandBackend),
    Idle,
}
```

Update the `WindowSource for AnyBackend` match. Old arm block:

```rust
        match self {
            AnyBackend::Gnome(b) => b.active_window(),
            AnyBackend::Idle => (String::new(), String::new()),
        }
```

New arm block:

```rust
        match self {
            AnyBackend::Gnome(b) => b.active_window(),
            AnyBackend::Hyprland(b) => b.active_window(),
            AnyBackend::Idle => (String::new(), String::new()),
        }
```

Append after the `WindowSource for AnyBackend` impl:

```rust
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::time::Duration;

/// Hyprland: request socket `$XDG_RUNTIME_DIR/hypr/$HYPRLAND_INSTANCE_SIGNATURE/.socket.sock`
/// (`j/activewindow` → JSON `{class, title}`), falling back to `hyprctl activewindow -j`.
/// Missing socket and missing `hyprctl` ⇒ probe fails. Never blocks: 200ms socket timeouts,
/// any error ⇒ ("","") for that poll.
pub struct HyprlandBackend {
    sock: PathBuf,
}

impl HyprlandBackend {
    pub fn socket_path() -> Option<PathBuf> {
        let sig = std::env::var("HYPRLAND_INSTANCE_SIGNATURE")
            .ok()
            .filter(|s| !s.is_empty())?;
        let rt = std::env::var("XDG_RUNTIME_DIR")
            .ok()
            .filter(|s| !s.is_empty())?;
        Some(PathBuf::from(rt).join("hypr").join(sig).join(".socket.sock"))
    }

    fn has_hyprctl() -> bool {
        std::process::Command::new("hyprctl")
            .arg("version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    pub fn new() -> Option<Self> {
        if let Some(p) = Self::socket_path() {
            if p.exists() {
                return Some(HyprlandBackend { sock: p });
            }
        }
        if Self::has_hyprctl() {
            return Some(HyprlandBackend { sock: PathBuf::new() });
        }
        None
    }

    fn query_socket(&self) -> Option<(String, String)> {
        if self.sock.as_os_str().is_empty() {
            return None;
        }
        let mut s = UnixStream::connect(&self.sock).ok()?;
        s.set_read_timeout(Some(Duration::from_millis(200))).ok()?;
        s.set_write_timeout(Some(Duration::from_millis(200))).ok()?;
        s.write_all(b"j/activewindow").ok()?;
        let mut buf = String::new();
        s.read_to_string(&mut buf).ok()?;
        Some(parse_hypr_json(&buf))
    }

    fn query_hyprctl() -> Option<(String, String)> {
        let out = std::process::Command::new("hyprctl")
            .args(["activewindow", "-j"])
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        Some(parse_hypr_json(&String::from_utf8_lossy(&out.stdout)))
    }
}

pub fn parse_hypr_json(s: &str) -> (String, String) {
    let v: serde_json::Value = serde_json::from_str(s).unwrap_or(serde_json::Value::Null);
    let app = v
        .get("class")
        .and_then(|c| c.as_str())
        .unwrap_or("")
        .to_string();
    let title = v
        .get("title")
        .and_then(|t| t.as_str())
        .unwrap_or("")
        .to_string();
    (app, title)
}

impl WindowSource for HyprlandBackend {
    fn active_window(&mut self) -> (String, String) {
        self.query_socket()
            .or_else(Self::query_hyprctl)
            .unwrap_or_default()
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p wisp-daemon backends::tests`
Expected: PASS — `test result: ok. 7 passed; 0 failed`.

- [ ] **Step 5: Commit**

```bash
git add wisp-daemon/src/backends.rs
git commit -m "feat(daemon): add Hyprland socket backend with hyprctl fallback"
```

---

### Task 4: Sway backend (i3-ipc `GET_TREE`)

**Files:**
- Modify: `wisp-daemon/src/backends.rs` (add `Sway(SwayBackend)` variant + arm + struct)
- Test: `wisp-daemon/src/backends.rs` (`mod tests`: append `find_focused` tests)

**Interfaces:**
- Consumes: `WindowSource`, `AnyBackend` from Tasks 2–3; `UnixStream`, `Duration` imports from Task 3.
- Produces: `pub struct SwayBackend` + `SwayBackend::new() -> Option<Self>`, `pub fn find_focused(v: &serde_json::Value) -> Option<(String, String)>` for Task 6 (`pick_backend`).

- [ ] **Step 1: Write the failing test**

Append to `mod tests` in `wisp-daemon/src/backends.rs`:

```rust
    #[test]
    fn sway_finds_wayland_focused_app_id() {
        let tree: serde_json::Value = serde_json::from_str(
            r#"{"focused":false,"nodes":[{"focused":false,"nodes":[
                {"focused":true,"app_id":"org.mozilla.firefox","name":"Docs — Firefox","nodes":[]}
            ]}]}"#,
        )
        .unwrap();
        assert_eq!(
            find_focused(&tree),
            Some((
                "org.mozilla.firefox".to_string(),
                "Docs — Firefox".to_string()
            ))
        );
    }

    #[test]
    fn sway_falls_back_to_xwayland_class() {
        let tree: serde_json::Value = serde_json::from_str(
            r#"{"focused":true,"window_properties":{"class":"Emacs"},"name":"*scratch*","nodes":[]}"#,
        )
        .unwrap();
        assert_eq!(
            find_focused(&tree),
            Some(("Emacs".to_string(), "*scratch*".to_string()))
        );
    }

    #[test]
    fn sway_no_focused_node_is_none() {
        let tree: serde_json::Value =
            serde_json::from_str(r#"{"focused":false,"nodes":[]}"#).unwrap();
        assert_eq!(find_focused(&tree), None);
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p wisp-daemon sway_finds_wayland_focused_app_id`
Expected: FAIL with compilation error `cannot find function 'find_focused'`.

- [ ] **Step 3: Write minimal implementation**

Update the `AnyBackend` enum. Old block:

```rust
pub enum AnyBackend {
    Gnome(GnomeBackend),
    Hyprland(HyprlandBackend),
    Idle,
}
```

New block:

```rust
pub enum AnyBackend {
    Gnome(GnomeBackend),
    Hyprland(HyprlandBackend),
    Sway(SwayBackend),
    Idle,
}
```

Update the match. Old arm block:

```rust
        match self {
            AnyBackend::Gnome(b) => b.active_window(),
            AnyBackend::Hyprland(b) => b.active_window(),
            AnyBackend::Idle => (String::new(), String::new()),
        }
```

New arm block:

```rust
        match self {
            AnyBackend::Gnome(b) => b.active_window(),
            AnyBackend::Hyprland(b) => b.active_window(),
            AnyBackend::Sway(b) => b.active_window(),
            AnyBackend::Idle => (String::new(), String::new()),
        }
```

Append after the `WindowSource for HyprlandBackend` impl:

```rust
/// Sway: `$SWAYSOCK` i3-ipc frame `"i3-ipc"` + len u32 LE + type u32 LE
/// (`4` = GET_TREE), empty payload; walks the JSON tree for the `focused`
/// node. app = `app_id` else `window_properties.class` else "",
/// title = `name` else "". Missing/unreachable `$SWAYSOCK` ⇒ probe fails.
/// Never blocks: 300ms socket timeouts, any error ⇒ ("","") for that poll.
pub struct SwayBackend {
    sock: PathBuf,
}

impl SwayBackend {
    pub fn new() -> Option<Self> {
        let raw = std::env::var("SWAYSOCK").ok().filter(|s| !s.is_empty())?;
        let path = PathBuf::from(&raw);
        if !path.exists() {
            return None;
        }
        UnixStream::connect(&path).ok()?;
        Some(SwayBackend { sock: path })
    }

    fn request_tree(sock: &PathBuf) -> Option<serde_json::Value> {
        let mut s = UnixStream::connect(sock).ok()?;
        s.set_read_timeout(Some(Duration::from_millis(300))).ok()?;
        s.set_write_timeout(Some(Duration::from_millis(300))).ok()?;
        let mut frame = b"i3-ipc".to_vec();
        frame.extend_from_slice(&0u32.to_le_bytes());
        frame.extend_from_slice(&4u32.to_le_bytes());
        s.write_all(&frame).ok()?;
        let mut hdr = [0u8; 14];
        s.read_exact(&mut hdr).ok()?;
        if &hdr[..6] != b"i3-ipc" {
            return None;
        }
        let len = u32::from_le_bytes(hdr[6..10].try_into().ok()?) as usize;
        if len > 32 * 1024 * 1024 {
            return None;
        }
        let mut payload = vec![0u8; len];
        s.read_exact(&mut payload).ok()?;
        serde_json::from_slice(&payload).ok()
    }
}

/// Walk the i3 tree for the node with `"focused": true`.
pub fn find_focused(v: &serde_json::Value) -> Option<(String, String)> {
    let obj = v.as_object()?;
    if obj.get("focused").and_then(|f| f.as_bool()) == Some(true) {
        let app = obj
            .get("app_id")
            .and_then(|a| a.as_str())
            .map(|s| s.to_string())
            .or_else(|| {
                obj.get("window_properties")
                    .and_then(|w| w.get("class"))
                    .and_then(|c| c.as_str())
                    .map(|s| s.to_string())
            })
            .unwrap_or_default();
        let title = obj
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("")
            .to_string();
        return Some((app, title));
    }
    for key in ["nodes", "floating_nodes"] {
        if let Some(arr) = obj.get(key).and_then(|n| n.as_array()) {
            for child in arr {
                if let Some(found) = find_focused(child) {
                    return Some(found);
                }
            }
        }
    }
    None
}

impl WindowSource for SwayBackend {
    fn active_window(&mut self) -> (String, String) {
        Self::request_tree(&self.sock)
            .and_then(|t| find_focused(&t))
            .unwrap_or_default()
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p wisp-daemon backends::tests`
Expected: PASS — `test result: ok. 10 passed; 0 failed`.

- [ ] **Step 5: Commit**

```bash
git add wisp-daemon/src/backends.rs
git commit -m "feat(daemon): add Sway i3-ipc backend"
```

---

### Task 5: KDE push backend + D-Bus `PushActive` + KWin script

**Files:**
- Modify: `wisp-daemon/src/backends.rs` (add `Kde(KdeBackend)` variant + arm + struct)
- Modify: `wisp-daemon/src/dbus_api.rs` (add `WindowSourcePush`, extend `serve()`)
- Modify: `wisp-daemon/src/main.rs:81` (destructure new 3-tuple; full rewiring stays in Task 7)
- Create: `packaging/kwin-wisp/contents/code/main.js`
- Create: `packaging/kwin-wisp/metadata.json`
- Test: `wisp-daemon/src/backends.rs` + `wisp-daemon/src/dbus_api.rs` tests

**Interfaces:**
- Consumes: `ActiveCache`, `new_active_cache`, `detect_desktop` from Task 1; `WindowSource`, `AnyBackend` from Tasks 2–4. `dirs` is already a `wisp-daemon` dependency.
- Produces: `pub struct KdeBackend` + `KdeBackend::new(cache: ActiveCache) -> Option<Self>` for Task 6; `pub async fn serve(db: Arc<Db>) -> zbus::Result<(zbus::Connection, ActivityTracker, ActiveCache)>` + `WindowSourcePush` for Task 7.

- [ ] **Step 1: Write the failing test**

Append to `mod tests` in `wisp-daemon/src/backends.rs`:

```rust
    #[test]
    fn kde_starts_empty_and_reads_pushed_cache() {
        let cache = new_active_cache();
        let mut b = KdeBackend { cache: cache.clone() };
        assert_eq!(b.active_window(), (String::new(), String::new()));
        *cache.lock().unwrap() =
            ("org.kde.dolphin".to_string(), "Home".to_string());
        assert_eq!(
            b.active_window(),
            ("org.kde.dolphin".to_string(), "Home".to_string())
        );
    }

    #[test]
    fn kde_constructs_only_on_hint_or_script_dir() {
        // Neither KDE hint nor script dir on CI ⇒ None. A push backend cannot
        // prove liveness synchronously, so construction is hint-based.
        let _guard = env_lock().lock().unwrap();
        let _e = EnvGuard::set(&[
            ("XDG_CURRENT_DESKTOP", Some("GNOME")),
            ("DESKTOP_SESSION", Some("gnome")),
            ("HOME", None),
        ]);
        if KdeBackend::script_dir().is_none() {
            assert!(KdeBackend::new(new_active_cache()).is_none());
        }
    }
```

Append to `wisp-daemon/src/dbus_api.rs` at the end of the file:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::new_active_cache;

    #[test]
    fn push_active_updates_cache() {
        let cache = new_active_cache();
        let push = WindowSourcePush { cache: cache.clone() };
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(push.push_active(
                "org.kde.dolphin".to_string(),
                "Home".to_string(),
            ));
        assert_eq!(
            *cache.lock().unwrap(),
            ("org.kde.dolphin".to_string(), "Home".to_string())
        );
    }
}
```

The struct-literal `KdeBackend { cache: ... }` requires the field to exist; the dbus test requires `WindowSourcePush::push_active`. Both fail to compile now, which is the failing state.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p wisp-daemon kde_starts_empty`
Expected: FAIL with compilation error `cannot find type 'KdeBackend' in module 'backends'`.

Run: `cargo test -p wisp-daemon push_active_updates_cache`
Expected: FAIL with compilation error `cannot find type 'WindowSourcePush'`.

- [ ] **Step 3: Write minimal implementation**

Update the `AnyBackend` enum. Old block:

```rust
pub enum AnyBackend {
    Gnome(GnomeBackend),
    Hyprland(HyprlandBackend),
    Sway(SwayBackend),
    Idle,
}
```

New block:

```rust
pub enum AnyBackend {
    Gnome(GnomeBackend),
    Kde(KdeBackend),
    Hyprland(HyprlandBackend),
    Sway(SwayBackend),
    Idle,
}
```

Update the match. Old arm block:

```rust
        match self {
            AnyBackend::Gnome(b) => b.active_window(),
            AnyBackend::Hyprland(b) => b.active_window(),
            AnyBackend::Sway(b) => b.active_window(),
            AnyBackend::Idle => (String::new(), String::new()),
        }
```

New arm block:

```rust
        match self {
            AnyBackend::Gnome(b) => b.active_window(),
            AnyBackend::Kde(b) => b.active_window(),
            AnyBackend::Hyprland(b) => b.active_window(),
            AnyBackend::Sway(b) => b.active_window(),
            AnyBackend::Idle => (String::new(), String::new()),
        }
```

Append after the `WindowSource for SwayBackend` impl in `wisp-daemon/src/backends.rs`:

```rust
/// KDE: the KWin script `packaging/kwin-wisp` pushes active-window changes over
/// D-Bus (`PushActive`); this backend clones the cached pair (starts ("",""),
/// fills on first push). `new` returns `Some` when the desktop hint is KDE or
/// the kwin-wisp script dir exists — a push backend cannot prove liveness
/// synchronously, so construction is hint-based, not handshake-based.
pub struct KdeBackend {
    cache: ActiveCache,
}

impl KdeBackend {
    pub fn script_dir() -> Option<PathBuf> {
        let home = dirs::home_dir()?;
        let user = home.join(".local/share/kwin/scripts/kwin-wisp");
        if user.is_dir() {
            return Some(user);
        }
        let sys = PathBuf::from("/usr/share/kwin/scripts/kwin-wisp");
        if sys.is_dir() {
            return Some(sys);
        }
        None
    }

    fn hinted() -> bool {
        detect_desktop()
            .iter()
            .any(|h| h == "kde" || h == "plasma")
    }

    pub fn new(cache: ActiveCache) -> Option<Self> {
        if Self::hinted() || Self::script_dir().is_some() {
            Some(KdeBackend { cache })
        } else {
            None
        }
    }
}

impl WindowSource for KdeBackend {
    fn active_window(&mut self) -> (String, String) {
        self.cache.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }
}
```

Edit `wisp-daemon/src/dbus_api.rs`. Old import line 1:

```rust
use std::sync::{Arc, Mutex};
```

New import lines:

```rust
use crate::backends::{new_active_cache, ActiveCache};
use std::sync::{Arc, Mutex};
```

Append after the `ActivityTracker` signal impl block (after line 201, before `pub async fn serve`), i.e. insert before `pub async fn serve`:

```rust
/// Second D-Bus surface for the KWin script: the daemon owns bus name
/// `com.saqr.wisp.WindowSource` at `/com/saqr/wisp/WindowSource` with interface
/// `com.saqr.wisp.WindowSource`. zbus exposes `push_active` as `PushActive`;
/// the `push_active_updates_cache` test below pins the cache behavior.
#[derive(Clone)]
pub struct WindowSourcePush {
    cache: ActiveCache,
}

#[interface(name = "com.saqr.wisp.WindowSource")]
impl WindowSourcePush {
    async fn push_active(&self, app: String, title: String) {
        *self.cache.lock().unwrap_or_else(|e| e.into_inner()) = (app, title);
    }
}
```

Replace the whole `serve` function (old `wisp-daemon/src/dbus_api.rs:203-213`):

```rust
pub async fn serve(db: Arc<Db>) -> zbus::Result<(zbus::Connection, ActivityTracker)> {
    let tracker = ActivityTracker::new(db);
    let emitter = tracker.clone();
    let conn = ConnectionBuilder::session()?
        .name("com.saqr.wisp")?
        .serve_at("/com/saqr/wisp", tracker)?
        .build()
        .await?;
    emitter.set_connection(conn.clone());
    Ok((conn, emitter))
}
```

New function:

```rust
pub async fn serve(db: Arc<Db>) -> zbus::Result<(zbus::Connection, ActivityTracker, ActiveCache)> {
    let tracker = ActivityTracker::new(db);
    let emitter = tracker.clone();
    let cache = new_active_cache();
    let push = WindowSourcePush { cache: cache.clone() };
    let conn = ConnectionBuilder::session()?
        .name("com.saqr.wisp")?
        .name("com.saqr.wisp.WindowSource")?
        .serve_at("/com/saqr/wisp", tracker)?
        .serve_at("/com/saqr/wisp/WindowSource", push)?
        .build()
        .await?;
    emitter.set_connection(conn.clone());
    Ok((conn, emitter, cache))
}
```

Keep `main.rs` compiling with the new 3-tuple (full rewiring is Task 7). Old line 81:

```rust
    let (_conn, tracker) = dbus_api::serve(db.clone()).await.unwrap();
```

New line:

```rust
    let (_conn, tracker, _kde_cache) = dbus_api::serve(db.clone()).await.unwrap();
```

Create `packaging/kwin-wisp/contents/code/main.js` with exactly:

```js
// kwin-wisp: push active-window changes to the Wisp daemon over D-Bus.
var SERVICE = "com.saqr.wisp.WindowSource";
var PATH = "/com/saqr/wisp/WindowSource";
var IFACE = "com.saqr.wisp.WindowSource";

function pushActive(client) {
    var app = "";
    var title = "";
    try {
        if (client) {
            title = String(client.caption || "");
            // resourceClass is "instance class" or a single class; take the class part.
            var rc = String(client.resourceClass || "");
            var parts = rc.split(" ");
            app = parts.length > 1 ? parts[1] : (parts[0] || "");
        }
        callDBus(SERVICE, PATH, IFACE, "PushActive", app, title);
    } catch (e) {}
}

if (workspace.windowActivated) {
    workspace.windowActivated.connect(pushActive);
} else if (workspace.clientActivated) {
    workspace.clientActivated.connect(pushActive);
}
// Seed current state at (re)load.
try {
    pushActive(workspace.activeWindow || workspace.activeClient || null);
} catch (e) {}
```

Create `packaging/kwin-wisp/metadata.json` with exactly:

```json
{
    "KPlugin": {
        "Authors": [{ "Name": "Wisp" }],
        "Description": "Pushes active-window changes to the Wisp activity tracker over D-Bus.",
        "EnabledByDefault": true,
        "Id": "kwin-wisp",
        "License": "MIT",
        "Name": "kwin-wisp",
        "Version": "1.0"
    },
    "X-Plasma-API": "javascript",
    "X-Plasma-MainScript": "code/main.js"
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p wisp-daemon backends::tests::kde dbus_api::tests`
Expected: PASS — kde tests plus `push_active_updates_cache` all pass; full suite `cargo test -p wisp-daemon backends` shows 12 passed, 0 failed.

Run: `cargo build -p wisp-daemon`
Expected: success (the `_kde_cache` binding keeps `main.rs` compiling until Task 7).

- [ ] **Step 5: Commit**

```bash
git add wisp-daemon/src/backends.rs wisp-daemon/src/dbus_api.rs wisp-daemon/src/main.rs packaging/kwin-wisp/contents/code/main.js packaging/kwin-wisp/metadata.json
git commit -m "feat(daemon): add KDE push backend, PushActive API, kwin-wisp script"
```

---

### Task 6: X11 backend + COSMIC slot + `chain_order` + `pick_backend`

**Files:**
- Modify: `wisp-daemon/Cargo.toml` (add `x11rb`)
- Modify: `wisp-daemon/src/backends.rs` (add `X11`/`Cosmic` variants + arms + structs + `chain_order` + `pick_backend`)
- Test: `wisp-daemon/src/backends.rs` (`mod tests`: append chain-order tests)

**Interfaces:**
- Consumes: every backend constructor from Tasks 2–5 with the exact names `GnomeBackend::new()`, `KdeBackend::new(cache)`, `HyprlandBackend::new()`, `SwayBackend::new()`; `detect_desktop` from Task 1.
- Produces: `pub struct X11Backend` + `X11Backend::new()`, `pub struct CosmicBackend` + `CosmicBackend::new()`, `pub fn chain_order(hints: &[String]) -> Vec<&'static str>`, `pub fn pick_backend(kde_cache: &ActiveCache) -> Option<AnyBackend>` for Task 7. Final `AnyBackend` shape matches the canonical signature in this plan's header.

- [ ] **Step 1: Write the failing test**

Append to `mod tests` in `wisp-daemon/src/backends.rs`:

```rust
    fn order_of(hints: &[&str]) -> Vec<&'static str> {
        let owned: Vec<String> = hints.iter().map(|s| s.to_string()).collect();
        chain_order(&owned)
    }

    #[test]
    fn chain_orders_hint_first_then_full_chain() {
        assert_eq!(
            order_of(&["kde", "gnome"]),
            vec!["kde", "gnome", "hyprland", "sway", "cosmic", "x11", "idle"]
        );
        assert_eq!(
            order_of(&["hyprland"]),
            vec!["hyprland", "gnome", "kde", "sway", "cosmic", "x11", "idle"]
        );
    }

    #[test]
    fn chain_maps_x11_desktops_to_x11() {
        assert_eq!(
            order_of(&["xfce"]),
            vec!["x11", "gnome", "kde", "hyprland", "sway", "cosmic", "idle"]
        );
        assert_eq!(
            order_of(&["x-cinnamon", "mate"]),
            vec!["x11", "gnome", "kde", "hyprland", "sway", "cosmic", "idle"]
        );
    }

    #[test]
    fn chain_always_ends_with_idle() {
        assert_eq!(*order_of(&[]).last().unwrap(), "idle");
        assert!(pick_backend(&new_active_cache()).is_some());
    }
```

`pick_backend` always returns `Some` because `Idle` always constructs; on CI (no DE) the winner is `Idle`.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p wisp-daemon chain_orders_hint_first`
Expected: FAIL with compilation error `cannot find function 'chain_order'`.

- [ ] **Step 3: Write minimal implementation**

Edit `wisp-daemon/Cargo.toml`. Old block:

```toml
[dependencies]
wisp-core = { path = "../wisp-core" }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
zbus = "4"
tokio = { version = "1", features = ["full"] }
dirs = "5"
infer = "0.19"
zvariant = "4.2"
```

New block (one line added, alphabetical position kept):

```toml
[dependencies]
wisp-core = { path = "../wisp-core" }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
x11rb = "0.13"
zbus = "4"
tokio = { version = "1", features = ["full"] }
dirs = "5"
infer = "0.19"
zvariant = "4.2"
```

Update the `AnyBackend` enum. Old block:

```rust
pub enum AnyBackend {
    Gnome(GnomeBackend),
    Kde(KdeBackend),
    Hyprland(HyprlandBackend),
    Sway(SwayBackend),
    Idle,
}
```

New block (final shape):

```rust
pub enum AnyBackend {
    Gnome(GnomeBackend),
    Kde(KdeBackend),
    Hyprland(HyprlandBackend),
    Sway(SwayBackend),
    Cosmic(CosmicBackend),
    X11(X11Backend),
    Idle,
}
```

Update the match. Old arm block:

```rust
        match self {
            AnyBackend::Gnome(b) => b.active_window(),
            AnyBackend::Kde(b) => b.active_window(),
            AnyBackend::Hyprland(b) => b.active_window(),
            AnyBackend::Sway(b) => b.active_window(),
            AnyBackend::Idle => (String::new(), String::new()),
        }
```

New arm block (final):

```rust
        match self {
            AnyBackend::Gnome(b) => b.active_window(),
            AnyBackend::Kde(b) => b.active_window(),
            AnyBackend::Hyprland(b) => b.active_window(),
            AnyBackend::Sway(b) => b.active_window(),
            AnyBackend::Cosmic(b) => b.active_window(),
            AnyBackend::X11(b) => b.active_window(),
            AnyBackend::Idle => (String::new(), String::new()),
        }
```

Append at the end of `wisp-daemon/src/backends.rs`:

```rust
use x11rb::protocol::xproto::{AtomEnum, ConnectionExt};
use x11rb::rust_connection::RustConnection;

/// X11: `_NET_ACTIVE_WINDOW` on root → `_NET_WM_NAME` (fallback `WM_NAME`) +
/// `WM_CLASS` of the active window. `DISPLAY` unset/unreachable ⇒ probe fails.
/// Covers XFCE, Cinnamon, MATE, and Plasma/X11 sessions with no extra code.
/// Never blocks: local X socket only; any error ⇒ ("","") for that poll.
pub struct X11Backend {
    conn: RustConnection,
    root: u32,
}

impl X11Backend {
    pub fn new() -> Option<Self> {
        if std::env::var("DISPLAY")
            .ok()
            .filter(|s| !s.is_empty())
            .is_none()
        {
            return None;
        }
        let (conn, screen) = x11rb::connect(None).ok()?;
        let root = conn.setup().roots.get(screen)?.root;
        Some(X11Backend { conn, root })
    }

    fn atom(&self, name: &[u8]) -> Option<u32> {
        self.conn
            .intern_atom(false, name)
            .ok()?
            .reply()
            .ok()
            .map(|r| r.atom)
    }

    fn active_id(&self) -> Option<u32> {
        let a = self.atom(b"_NET_ACTIVE_WINDOW")?;
        let reply = self
            .conn
            .get_property(false, self.root, a, AtomEnum::WINDOW, 0, 1)
            .ok()?
            .reply()
            .ok()?;
        reply.value32()?.next()
    }

    fn text_prop(&self, win: u32, names: &[&[u8]]) -> String {
        for n in names {
            let Some(a) = self.atom(n) else { continue };
            let Ok(cookie) =
                self.conn
                    .get_property(false, win, a, AtomEnum::ANY, 0, u32::MAX)
            else {
                continue;
            };
            let Ok(reply) = cookie.reply() else {
                continue;
            };
            if reply.value.is_empty() {
                continue;
            }
            let s = String::from_utf8_lossy(&reply.value)
                .trim_matches('\0')
                .to_string();
            if !s.is_empty() {
                return s;
            }
        }
        String::new()
    }

    fn class_of(&self, win: u32) -> String {
        // WM_CLASS is "instance\0class\0": prefer class, fall back to instance.
        let raw = self.text_prop(win, &[b"WM_CLASS"]);
        let mut parts = raw.split('\0');
        let inst = parts.next().unwrap_or("");
        let class = parts.next().unwrap_or("");
        if !class.is_empty() {
            class.to_string()
        } else {
            inst.to_string()
        }
    }
}

impl WindowSource for X11Backend {
    fn active_window(&mut self) -> (String, String) {
        let Some(win) = self.active_id() else {
            return (String::new(), String::new());
        };
        if win == 0 {
            return (String::new(), String::new());
        }
        let title = self.text_prop(win, &[b"_NET_WM_NAME", b"WM_NAME"]);
        let app = self.class_of(win);
        (app, title)
    }
}

/// COSMIC native slot (`ext-foreign-toplevel-list-v1`, falling back to
/// `wlr-foreign-toplevel`). No `wayland-client` dependency in this plan, and
/// the toplevel protocols cannot be probed without it, so `new` returns `None`
/// and COSMIC sessions fall through to X11/Idle per the spec. Upgrade path:
/// add the protocol binding, store the toplevel snapshot here, and return
/// `Some` when the compositor offers either protocol. Chain position and the
/// `"COSMIC backend active"` log line in `pick_backend` stay as-is.
pub struct CosmicBackend;

impl CosmicBackend {
    pub fn new() -> Option<Self> {
        None
    }
}

impl WindowSource for CosmicBackend {
    fn active_window(&mut self) -> (String, String) {
        (String::new(), String::new())
    }
}

/// Candidate order: hinted backends first (in hint order), then the fixed
/// chain gnome → kde → hyprland → sway → cosmic → x11 → idle, deduplicated.
pub fn chain_order(hints: &[String]) -> Vec<&'static str> {
    const CHAIN: &[&str] = &["gnome", "kde", "hyprland", "sway", "cosmic", "x11", "idle"];
    fn hint_name(h: &str) -> Option<&'static str> {
        match h {
            h if h.contains("hyprland") => Some("hyprland"),
            h if h == "sway" => Some("sway"),
            h if h == "cosmic" => Some("cosmic"),
            h if h == "kde" || h == "plasma" => Some("kde"),
            h if h.contains("gnome") => Some("gnome"),
            h if h == "xfce" || h == "x-cinnamon" || h == "cinnamon" || h == "mate" => {
                Some("x11")
            }
            h if h == "x11" => Some("x11"),
            _ => None,
        }
    }
    let mut order: Vec<&'static str> =
        hints.iter().filter_map(|h| hint_name(h)).collect();
    for c in CHAIN {
        if !order.contains(c) {
            order.push(c);
        }
    }
    order
}

/// Probe the hinted backend first, then the full chain; first `Some` wins.
/// Each probe is soft (`None` on any failure, never panic) and the winner is
/// logged. A hint only orders candidates — an unresponsive hinted backend
/// falls through to the next probe.
pub fn pick_backend(kde_cache: &ActiveCache) -> Option<AnyBackend> {
    let hints = detect_desktop();
    for name in chain_order(&hints) {
        let found = match name {
            "gnome" => GnomeBackend::new().map(AnyBackend::Gnome),
            "kde" => KdeBackend::new(kde_cache.clone()).map(AnyBackend::Kde),
            "hyprland" => HyprlandBackend::new().map(AnyBackend::Hyprland),
            "sway" => SwayBackend::new().map(AnyBackend::Sway),
            "cosmic" => CosmicBackend::new().map(AnyBackend::Cosmic),
            "x11" => X11Backend::new().map(AnyBackend::X11),
            _ => Some(AnyBackend::Idle),
        };
        if let Some(b) = found {
            match &b {
                AnyBackend::Gnome(_) => println!("GNOME Shell extension backend active"),
                AnyBackend::Kde(_) => println!("KDE KWin script backend active"),
                AnyBackend::Hyprland(_) => println!("Hyprland socket backend active"),
                AnyBackend::Sway(_) => println!("Sway i3-ipc backend active"),
                AnyBackend::Cosmic(_) => println!("COSMIC backend active"),
                AnyBackend::X11(_) => println!("X11 backend active"),
                AnyBackend::Idle => println!("no native backend found; idle backend active"),
            }
            return Some(b);
        }
    }
    None
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p wisp-daemon backends::tests`
Expected: PASS — `test result: ok. 15 passed; 0 failed` (12 prior + 3 chain tests).

- [ ] **Step 5: Commit**

```bash
git add wisp-daemon/Cargo.toml wisp-daemon/src/backends.rs
git commit -m "feat(daemon): add X11 backend, COSMIC slot, pick_backend chain"
```

---

### Task 7: `HotBackend` + `wait_for_backend` + rewire `main.rs` + delete `gnome.rs`

**Files:**
- Modify: `wisp-daemon/src/backends.rs` (add `HotBackend`, `wait_for_backend`)
- Modify: `wisp-daemon/src/main.rs` (delete old `pick_backend`/`wait_for_backend`, use `backends::wait_for_backend`)
- Delete: `wisp-daemon/src/gnome.rs` (`git rm`)
- Test: `wisp-daemon/src/backends.rs` (`mod tests`: append HotBackend test)

**Interfaces:**
- Consumes: `pick_backend`, `AnyBackend`, `ActiveCache` from Tasks 1–6; `dbus_api::serve` 3-tuple from Task 5.
- Produces: `#[derive(Clone)] pub struct HotBackend { current: Arc<Mutex<AnyBackend>> }` + `HotBackend::new`, `impl WindowSource for HotBackend`, `pub fn wait_for_backend(kde_cache: ActiveCache) -> HotBackend`. Nothing downstream (final tracking task).

- [ ] **Step 1: Write the failing test**

Append to `mod tests` in `wisp-daemon/src/backends.rs`:

```rust
    #[test]
    fn hot_backend_delegates_to_inner() {
        let hot = HotBackend::new(AnyBackend::Idle);
        let mut polled = hot.clone();
        assert_eq!(polled.active_window(), (String::new(), String::new()));
    }

    #[test]
    fn wait_for_backend_returns_promptly_without_a_de() {
        let _guard = env_lock().lock().unwrap();
        let _e = EnvGuard::set(&[
            ("XDG_CURRENT_DESKTOP", Some("UNKNOWN-DE-XYZ")),
            ("DESKTOP_SESSION", Some("unknown-de-xyz")),
            ("HYPRLAND_INSTANCE_SIGNATURE", None),
            ("SWAYSOCK", None),
            ("WAYLAND_DISPLAY", None),
            ("DISPLAY", None),
        ]);
        let start = std::time::Instant::now();
        let mut hot = wait_for_backend(new_active_cache());
        assert!(
            start.elapsed() < std::time::Duration::from_secs(5),
            "wait_for_backend must not block the tracker"
        );
        let _ = hot.active_window();
    }
```

Note: the unknown-DE run falls back to `Idle` (all native probes fail without env signals), so `active_window()` is deterministic here. The spawned 5s retry thread may outlive the test process; that is harmless.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p wisp-daemon hot_backend_delegates_to_inner`
Expected: FAIL with compilation error `cannot find type 'HotBackend'`.

- [ ] **Step 3: Write minimal implementation**

Append at the end of `wisp-daemon/src/backends.rs`:

```rust
/// Swappable handle that IS the `WindowSource`: the tracker loop keeps polling
/// this at 1s while the retry thread hot-swaps the inner backend.
#[derive(Clone)]
pub struct HotBackend {
    current: Arc<Mutex<AnyBackend>>,
}

impl HotBackend {
    pub fn new(initial: AnyBackend) -> Self {
        HotBackend {
            current: Arc::new(Mutex::new(initial)),
        }
    }
}

impl WindowSource for HotBackend {
    fn active_window(&mut self) -> (String, String) {
        self.current
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .active_window()
    }
}

/// Try `pick_backend()` once. On success return `HotBackend` wrapping it; on
/// total failure return `HotBackend` wrapping `Idle` so MPRIS, logind, the file
/// watcher, and the D-Bus API keep running. When the first pick is `Idle`, a
/// background thread retries `pick_backend()` every 5s and hot-swaps the inner
/// backend on the first non-Idle success (logs the swap). This replaces the
/// old block-forever GNOME loop, which would stall all tracking on DEs
/// without the GNOME extension.
pub fn wait_for_backend(kde_cache: ActiveCache) -> HotBackend {
    let initial = pick_backend(&kde_cache).unwrap_or(AnyBackend::Idle);
    let hot = HotBackend::new(initial);
    let is_idle = matches!(
        *hot.current.lock().unwrap_or_else(|e| e.into_inner()),
        AnyBackend::Idle
    );
    if is_idle {
        let current = hot.current.clone();
        std::thread::spawn(move || loop {
            std::thread::sleep(Duration::from_secs(5));
            if let Some(b) = pick_backend(&kde_cache) {
                if !matches!(b, AnyBackend::Idle) {
                    *current.lock().unwrap_or_else(|e| e.into_inner()) = b;
                    println!("backend hot-swapped after retry");
                    break;
                }
            }
        });
    }
    hot
}
```

Rewire `wisp-daemon/src/main.rs`. Old lines 1–31:

```rust
mod backends; mod dbus_api; mod gnome; mod logind; mod mpris; mod systemd;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use wisp_core::db::Db;
use wisp_core::tracker::{run_tracker_loop, unix_now, SysEvents, WindowSource};
use wisp_core::watcher::spawn_file_watcher;
use gnome::GnomeBackend;

fn pick_backend() -> Option<impl WindowSource> {
    if let Some(g) = GnomeBackend::new() {
        println!("GNOME Shell extension backend active");
        return Some(g);
    }
    None
}

/// The daemon starts at login before gnome-shell finishes loading extensions,
/// so retry until the Wisp extension owns its bus name.
fn wait_for_backend() -> impl WindowSource {
    loop {
        if let Some(b) = pick_backend() {
            return b;
        }
        println!("waiting for the GNOME Shell extension...");
        std::thread::sleep(Duration::from_secs(5));
    }
}
```

New lines:

```rust
mod backends; mod dbus_api; mod logind; mod mpris; mod systemd;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use wisp_core::db::Db;
use wisp_core::tracker::{run_tracker_loop, unix_now, SysEvents};
use wisp_core::watcher::spawn_file_watcher;
use backends::wait_for_backend;
```

Old line 81 (already touched in Task 5):

```rust
    let (_conn, tracker, _kde_cache) = dbus_api::serve(db.clone()).await.unwrap();
```

New line:

```rust
    let (_conn, tracker, kde_cache) = dbus_api::serve(db.clone()).await.unwrap();
```

The tracker spawn block (old `main.rs:86-98`) stays byte-identical — `run_tracker_loop` takes `backend: impl WindowSource` by value and `HotBackend` implements `WindowSource`, so `let backend = wait_for_backend(kde_cache);` inside the thread just works:

```rust
    std::thread::spawn(move || {
        let backend = wait_for_backend(kde_cache);
        run_tracker_loop(db2, backend, &sys2, &|app, _| mpris::probe(app), move |app, title, now| {
```

Delete the moved-from file:

```bash
git rm wisp-daemon/src/gnome.rs
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p wisp-daemon`
Expected: PASS — all daemon tests pass including the 2 new HotBackend tests (`test result: ok. 19 passed; 0 failed` across `backends::tests` + `dbus_api::tests`; exact count may vary by one if the environment probes differ, but zero failures).

Run: `cargo build -p wisp-daemon`
Expected: success with no errors.

- [ ] **Step 5: Commit**

```bash
git add wisp-daemon/src/backends.rs wisp-daemon/src/main.rs
git rm wisp-daemon/src/gnome.rs
git commit -m "feat(daemon): hot-swappable AnyBackend selection, remove gnome module"
```

---

### Task 8: Full verification + manual DE matrix

**Files:** none (verification only; fix fallout in place if red).

**Interfaces:**
- Consumes: the finished tree from Task 7.
- Produces: release confidence: green workspace tests for `wisp-daemon` + `wisp-core`, and manual per-DE evidence.

- [ ] **Step 1: Write the failing test**

No new test code: this task runs the accumulated suites. The "failing" gate is explicit — run the full daemon + core suites and require zero failures:

Run: `cargo test -p wisp-daemon -p wisp-core`
Expected (gate): every suite reports `test result: ok`, zero `FAILED`, zero compilation errors. Any failure sends the worker back to the owning task's code before proceeding.

- [ ] **Step 2: Run test to verify it fails**

Same command as Step 4 below. If Step 1 is already green, record that and move on — the gate is the requirement, not a forced red.

- [ ] **Step 3: Write minimal implementation**

No new code. If the gate in Step 1 is red, fix the owning task's code with the smallest change that turns it green (no new files, no refactors).

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p wisp-daemon -p wisp-core`
Expected: PASS — all suites `ok`, e.g.:

```
test result: ok. <N> passed; 0 failed; 0 ignored
```

Run: `cargo build -p wisp-daemon`
Expected: `Finished dev profile` with no errors.

Manual matrix (tracking rows only; autostart/tray belong to their own plans). On each DE, run the debug daemon and check the winner log line plus a live window change:

```bash
./target/debug/wisp-daemon
```

Expected log lines per DE:

- GNOME with extension → `GNOME Shell extension backend active`
- Plasma with kwin-wisp installed → `KDE KWin script backend active`
- Hyprland → `Hyprland socket backend active`
- Sway → `Sway i3-ipc backend active`
- XFCE / Cinnamon / MATE → `X11 backend active`
- Bare session with none of the above → `no native backend found; idle backend active`

KDE end-to-end check: install the script dir (`~/.local/share/kwin/scripts/kwin-wisp` containing `contents/code/main.js` + `metadata.json`), enable the script in System Settings → Window Management → KWin Scripts, switch windows, confirm the daemon logs window changes. Regression check on GNOME: with no other DE signals present, the daemon must pick the GNOME backend exactly as before (same `GetActive` path, moved verbatim).

- [ ] **Step 5: Commit**

No code changes expected, so no commit. If Step 3 required a fix, commit it with:

```bash
git add -A
git commit -m "fix(daemon): <one-line cause>"
```

---

## Execution Handoff

**Plan complete and saved to `docs/superpowers/plans/2026-09-11-linux-tracking-backends.md`. Two execution options:**

**1. Subagent-Driven (recommended)** - I dispatch a fresh subagent per task, review between tasks, fast iteration

**2. Inline Execution** - Execute tasks in this session using executing-plans, batch execution with checkpoints

**Which approach?**
