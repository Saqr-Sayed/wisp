use std::sync::{Arc, Mutex};

/// Shared last-pushed (app, title) pair. Created in `dbus_api::serve`,
/// cloned into `KdeBackend`. Starts ("",""), fills on first PushActive.
pub type ActiveCache = Arc<Mutex<(String, String)>>;

pub fn new_active_cache() -> ActiveCache {
    Arc::new(Mutex::new((String::new(), String::new())))
}

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

/// First-probe-wins backend chain. Later tasks add Sway,
/// Cosmic, and X11 variants plus their match arms here.
pub enum AnyBackend {
    Gnome(GnomeBackend),
    Kde(KdeBackend),
    Hyprland(HyprlandBackend),
    Sway(SwayBackend),
    Cosmic(CosmicBackend),
    X11(X11Backend),
    Idle,
}

impl WindowSource for AnyBackend {
    fn active_window(&mut self) -> (String, String) {
        match self {
            AnyBackend::Gnome(b) => b.active_window(),
            AnyBackend::Kde(b) => b.active_window(),
            AnyBackend::Hyprland(b) => b.active_window(),
            AnyBackend::Sway(b) => b.active_window(),
            AnyBackend::Cosmic(b) => b.active_window(),
            AnyBackend::X11(b) => b.active_window(),
            AnyBackend::Idle => (String::new(), String::new()),
        }
    }
}

/// KDE: the KWin script `packaging/kwin-wisp` pushes active-window changes over
/// D-Bus (`PushActive`); this backend clones the cached pair (starts ("",""),
/// fills on first push). `new` returns `Some` when the desktop hint is KDE or
/// the kwin-wisp script dir exists — a push backend cannot prove liveness
/// synchronously, so construction is hint-based, not handshake-based.
pub struct KdeBackend {
    cache: ActiveCache,
}

impl KdeBackend {
    pub fn script_dir() -> Option<std::path::PathBuf> {
        let home = dirs::home_dir()?;
        let user = home.join(".local/share/kwin/scripts/kwin-wisp");
        if user.is_dir() {
            return Some(user);
        }
        let sys = std::path::PathBuf::from("/usr/share/kwin/scripts/kwin-wisp");
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

use x11rb::connection::Connection as _;
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
        let id = reply.value32()?.next();
        id
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
    let mut order: Vec<&'static str> = Vec::new();
    for h in hints.iter().filter_map(|h| hint_name(h)) {
        if !order.contains(&h) {
            order.push(h);
        }
    }
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

    #[test]
    fn idle_returns_empty_pair() {
        let mut b = AnyBackend::Idle;
        assert_eq!(b.active_window(), (String::new(), String::new()));
    }

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
}
