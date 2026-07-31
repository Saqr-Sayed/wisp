use std::sync::Arc;
use std::time::Duration;

use crate::db::Db;
use crate::classifier::classify;

/// Source of the currently focused window (app id/name, window title).
pub trait WindowSource: Send {
    fn active_window(&mut self) -> (String, String);
}

pub fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

/// Polls the backend once per second; on window change, closes the previous
/// log entry, inserts a new one, and calls `on_change` (e.g. D-Bus signal).
pub fn run_tracker_loop<F>(
    db: Arc<Db>,
    mut backend: impl WindowSource,
    mut on_change: F,
) where
    F: FnMut(&str, &str, i64),
{
    let mut prev_app = String::new();
    let mut prev_title = String::new();
    let mut current_log_id: Option<i64> = None;

    loop {
        std::thread::sleep(Duration::from_secs(1));
        let (app, title) = backend.active_window();

        if (app != prev_app || title != prev_title) && !app.is_empty() {
            let now = unix_now();

            if let Some(id) = current_log_id {
                db.close_log(id, now);
            }

            let et = classify(&app, &title);
            current_log_id = Some(db.insert_log(et, &app, &title, now));
            on_change(&app, &title, now);

            prev_app = app.clone();
            prev_title = title.clone();
        }
    }
}
