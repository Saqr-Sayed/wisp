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
}
