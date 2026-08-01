use std::sync::Arc;
use std::time::Duration;

use crate::db::{Db, LogEvent};

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

    db.close_dangling(unix_now());

    loop {
        std::thread::sleep(Duration::from_secs(1));
        let (app, title) = backend.active_window();

        // Don't track our own windows (the UI) — it would pollute the timeline
        // and make "current activity" always show Salmonella. Close the ongoing
        // entry so time spent in Salmonella stays an honest gap.
        if app.to_lowercase().contains("salmonella") || title.eq_ignore_ascii_case("salmonella") {
            if let Some(id) = current_log_id {
                db.close_log(id, unix_now());
                current_log_id = None;
            }
            prev_app.clear();
            prev_title.clear();
            continue;
        }

        if (app != prev_app || title != prev_title) && !app.is_empty() {
            let now = unix_now();

            if let Some(id) = current_log_id {
                db.close_log(id, now);
            }

            let enriched = crate::classifier::enrich(&app, &title);
            let friendly = db.friendly_name(&app);
            // تجاوز الفئة بقاعدة مخصصة (لو وُجدت)؛ القاعدة المخصصة لا تُلغي الفئات المدمجة
            // سوى «other» — أي التطابق يُطبَّق فقط على الفئة الافتراضية.
            let category: String = if enriched.category == "other" {
                if let Some(name) = db.match_custom_category("app", &app) {
                    name
                } else if !enriched.site.is_empty() {
                    db.match_custom_category("site", &enriched.site).unwrap_or_else(|| enriched.category.to_string())
                } else {
                    enriched.category.to_string()
                }
            } else {
                enriched.category.to_string()
            };
            let log_event = LogEvent {
                event_type: enriched.event_type,
                category: &category,
                friendly: &friendly,
                site: &enriched.site,
                series: &enriched.series,
                episode: &enriched.episode,
                app: &app,
                title: &title,
            };
            current_log_id = Some(db.insert_log(&log_event, now));
            on_change(&app, &title, now);

            prev_app = app.clone();
            prev_title = title.clone();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Db;

    struct FakeSource(Vec<(String, String)>, usize);
    impl WindowSource for FakeSource {
        fn active_window(&mut self) -> (String, String) {
            let i = self.1.min(self.0.len() - 1);
            let r = self.0[i].clone();
            if self.1 < self.0.len() - 1 { self.1 += 1; }
            r
        }
    }

    #[test]
    fn inserts_enriched_data() {
        let path = std::env::temp_dir().join(format!("salmonella-tracker-{}.db", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let db = std::sync::Arc::new(Db::open(&path));
        let backend = FakeSource(vec![
            ("org.mozilla.firefox.desktop".into(), "عنوان - YouTube — Mozilla Firefox".into()),
        ], 0);
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut first = true;
            run_tracker_loop(db.clone(), backend, |_, _, _| { if first { tx.send(()).unwrap(); first = false; } });
        });
        rx.recv_timeout(std::time::Duration::from_secs(5)).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(100));
        let db = Db::open(&path);
        let rows = db.get_timeline(0, i64::MAX);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].friendly_name, "فايرفوكس");
        assert_eq!(rows[0].site, "YouTube");
        assert_eq!(rows[0].category, "media");
        let _ = std::fs::remove_file(&path);
    }
}
