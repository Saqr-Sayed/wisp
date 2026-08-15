use std::sync::{Arc, Mutex};
use std::time::Duration;

use wisp_core::tracker::MediaMeta;

/// Cached SMTC playback sessions: (source app AUMID, title) per player.
pub type MediaCache = Arc<Mutex<Vec<(String, String)>>>;

/// Polls the Windows System Media Transport Controls session manager every 3s
/// and caches (source app, title) for the current playback session(s).
///
/// ponytail: Only GetCurrentSession is used; if the user ever runs several
/// players at once, switch to GetSessions() and keep a Vec per session id
/// (SMTC gives no media content type, so mime stays None either way).
pub fn spawn_smtc_poller() -> MediaCache {
    let cache: MediaCache = Arc::new(Mutex::new(Vec::new()));
    let worker = cache.clone();
    std::thread::spawn(move || {
        unsafe {
            let _ = windows::Win32::System::WinRT::RoInitialize(
                windows::Win32::System::WinRT::RO_INIT_MULTITHREADED,
            );
        }
        loop {
            let mut current: Vec<(String, String)> = Vec::new();
            if let Ok(op) = windows::Media::Control::GlobalSystemMediaTransportControlsSessionManager::RequestAsync() {
                if let Ok(mgr) = op.get() {
                    if let Ok(sess) = mgr.GetCurrentSession() {
                        if let Ok(aumid) = sess.SourceAppUserModelId() {
                            if let Ok(prop_op) = sess.TryGetMediaPropertiesAsync() {
                                if let Ok(props) = prop_op.get() {
                                    if let Ok(title) = props.Title() {
                                        let title = title.to_string();
                                        if !title.is_empty() {
                                            current.push((aumid.to_string(), title));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            *cache.lock().unwrap() = current;
            std::thread::sleep(Duration::from_secs(3));
        }
    });
    worker
}

/// Metadata hook: returns the cached SMTC title when the foreground app's
/// process name (".exe" stripped, lowercase contains) matches the player's
/// AUMID, mirroring the Linux MPRIS matching in wisp-core/classifier.
pub fn media_hook(cache: &MediaCache, app: &str, _title: &str) -> Option<MediaMeta> {
    let app_base = app.strip_suffix(".exe").unwrap_or(app).to_lowercase();
    if app_base.is_empty() {
        return None;
    }
    let list = cache.lock().unwrap();
    for (aumid, title) in list.iter() {
        if aumid.to_lowercase().contains(&app_base) {
            return Some(MediaMeta { title: Some(title.clone()), mime: None });
        }
    }
    None
}