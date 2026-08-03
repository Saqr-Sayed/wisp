use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::db::{Db, LogEvent};

/// ميتاداتا وسائط يقدمها خطاف metadata (MPRIS + توقيع ملف في الـ daemon؛
/// ويندوز والاختبارات يمررون &|_, _| None).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MediaMime { Audio, Video }

pub struct MediaMeta {
    pub title: Option<String>,
    pub mime: Option<MediaMime>,
}

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

/// System lifecycle event fed to the tracker (e.g. logind sleep/wake).
pub struct SysEvent {
    pub kind: &'static str,
    pub at: i64,
}

/// Thread-safe queue of pending system events, drained by the tracker loop.
pub struct SysEvents(Mutex<Vec<SysEvent>>);

impl SysEvents {
    pub fn new() -> Self {
        SysEvents(Mutex::new(Vec::new()))
    }

    pub fn push(&self, kind: &'static str, at: i64) {
        self.0.lock().unwrap().push(SysEvent { kind, at });
    }

    pub fn drain(&self) -> Vec<SysEvent> {
        std::mem::take(&mut *self.0.lock().unwrap())
    }
}

/// Polls the backend once per second; on window change, closes the previous
/// log entry, inserts a new one, and calls `on_change` (e.g. D-Bus signal).
/// System events (sleep/wake/...) are processed before each poll; a stale
/// heartbeat + a poll gap closes dangling entries at the suspend moment.
pub fn run_tracker_loop<F>(
    db: Arc<Db>,
    mut backend: impl WindowSource,
    sys: &SysEvents,
    metadata: &dyn Fn(&str, &str) -> Option<MediaMeta>,
    mut on_change: F,
) where
    F: FnMut(&str, &str, i64),
{
    let mut prev_app = String::new();
    let mut prev_title = String::new();
    let mut last_folder: Option<(String, i64)> = None;
    let mut current_log_id: Option<i64> = None;
    let mut sleep_open_id: Option<i64> = None;

    let mut last_hb = unix_now();
    let mut last_poll = unix_now();

    // ponytail: no close_dangling here — callers (daemon main, Windows setup)
    // run it BEFORE inserting point events, so fresh boot/login rows aren't
    // closed (or closed at a stale last_alive → negative durations).

    loop {
        std::thread::sleep(Duration::from_secs(1));
        let now = unix_now();

        if now - last_hb >= 10 {
            db.set_setting("last_alive", &now.to_string());
            last_hb = now;
        }

        let gap = now - last_poll > 120;
        let drained = sys.drain();
        let handled_sleep = drained.iter().any(|e| e.kind == "sleep");

        let close_log_at = |current: &mut Option<i64>, at: i64| {
            if let Some(id) = current.take() {
                db.close_log(id, at);
            }
        };

        for ev in &drained {
            match ev.kind {
                "sleep" => {
                    let at = if gap { last_poll } else { ev.at };
                    close_log_at(&mut current_log_id, at);
                    prev_app.clear();
                    prev_title.clear();
                    sleep_open_id = Some(db.insert_system_event("sleep", "", at, None));
                }
                "wake" => {
                    if let Some(id) = sleep_open_id.take() {
                        db.close_log(id, ev.at);
                    }
                    db.insert_system_event("wake", "", ev.at, Some(ev.at));
                }
                "power_off" | "logout" => {
                    close_log_at(&mut current_log_id, ev.at);
                    prev_app.clear();
                    prev_title.clear();
                    db.insert_system_event(ev.kind, "", ev.at, Some(ev.at));
                }
                _ => {}
            }
        }

        // logind signal missed (loop was frozen through a suspend): close at
        // the last known poll, log a closed sleep event covering the gap.
        if gap && !handled_sleep {
            close_log_at(&mut current_log_id, last_poll);
            prev_app.clear();
            prev_title.clear();
            db.insert_system_event("sleep", "", last_poll, Some(now));
        }

        last_poll = now;

        let (app, title) = backend.active_window();

        if (app != prev_app || title != prev_title) && !app.is_empty() {
            let now = unix_now();

            // سياق المجلد قبل فحص المستثنيات — تجاهل المستخدم لـ Nautilus لا يلغي
            // وظيفة السياق، وصف Nautilus نفسه يُسجَّل كالمعتاد كتطبيق عادي
            if crate::classifier::is_file_manager(&app) {
                last_folder = Some((title.clone(), now));
            }

            if let Some(id) = current_log_id {
                db.close_log(id, now);
            }

            // ميتاداتا الوسائط (MPRIS/توقيع ملف) تُقرأ قبل enrich: العنوان
            // البديل يحل محل العام، وتجاوز mime يقع بعد enrich بحارس is_media_app
            let meta = metadata(&app, &title);
            let effective_title = if crate::classifier::is_generic_title(&app, &title) {
                meta.as_ref().and_then(|m| m.title.clone()).unwrap_or_else(|| title.clone())
            } else {
                title.clone()
            };

            let mut enriched = crate::classifier::enrich(&app, &effective_title);
            // تجاوز mime (القرار 9): مشغلات الوسائط فقط — القراءة (Papers/Evince)
            // والمتصفحات خارج is_media_app فلا يُتجاوز تصنيفها أبداً
            if crate::classifier::is_media_app(&app) {
                match meta.as_ref().and_then(|m| m.mime) {
                    Some(MediaMime::Audio) => enriched.media_kind = "listening",
                    Some(MediaMime::Video) => enriched.media_kind = "watching",
                    None => {}
                }
            }
            // مستبعد: أغلِق الحدث الجاري ولا تسجّل، دون إطلاق إشارة التغيير
            if db.is_ignored("app", &app) || db.is_ignored("site", &enriched.site) {
                if let Some(id) = current_log_id {
                    db.close_log(id, now);
                    current_log_id = None;
                }
                prev_app.clear();
                prev_title.clear();
                continue;
            }
            let friendly = db.friendly_name(&app);
            let site_friendly = db.site_friendly_name(&enriched.site);
            let category: String = db.resolve_category(&app, &enriched.site, enriched.category);
            let override_series = db.resolve_series(&app, &title);
            // سياق المجلد لصفوف الوسائط فقط — صف المدير نفسه لا يستعير عنوانه
            let folder = if crate::classifier::is_file_manager(&app) { None } else { last_folder.as_ref() };
            let series = final_series(&enriched.series, enriched.series_weak, override_series.as_deref(),
                                      folder, now);
            let log_event = LogEvent {
                event_type: enriched.event_type,
                category: &category,
                media_kind: enriched.media_kind,
                friendly: &friendly,
                site: &enriched.site,
                site_friendly: &site_friendly,
                series: &series,
                episode: &enriched.episode,
                app: &app,
                title: &effective_title,
            };
            current_log_id = Some(db.insert_log(&log_event, now));
            on_change(&app, &title, now);

            prev_app = app.clone();
            prev_title = title.clone();
        }
    }
}

/// حل السلسلة النهائية للصف الجديد — أولوية: تجاوز صريح ← سياق المجلد
/// (ضعيفة فقط وضمن 600 ثانية) ← السلسلة المحلَّلة.
fn final_series(enriched_series: &str, weak: bool, override_series: Option<&str>,
                folder: Option<&(String, i64)>, now: i64) -> String {
    if let Some(s) = override_series { return s.to_string(); }      // 1) تجاوز صريح
    if weak {                                                       // 2) سياق المجلد
        if let Some((name, ts)) = folder { if now - ts <= 600 { return name.clone(); } }
    }
    enriched_series.to_string()                                     // 3) المحلَّلة
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
        let sys = SysEvents::new();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut first = true;
            run_tracker_loop(db.clone(), backend, &sys, &|_, _| None, |_, _, _| { if first { tx.send(()).unwrap(); first = false; } });
        });
        rx.recv_timeout(std::time::Duration::from_secs(5)).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(100));
        let db = Db::open(&path);
        let rows = db.get_timeline(0, i64::MAX);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].friendly_name, "فايرفوكس");
        assert_eq!(rows[0].site, "YouTube");
        assert_eq!(rows[0].category, "وسائط");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn skips_ignored_app_and_site() {
        let path = std::env::temp_dir().join(format!("salmonella-tracker-ign-{}.db", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let db = std::sync::Arc::new(Db::open(&path));
        db.ignore_target("app", "org.gnome.Ptyxis.desktop");
        db.ignore_target("site", "YouTube");
        let backend = FakeSource(vec![
            ("code.desktop".into(), "main.rs".into()),
            ("org.mozilla.firefox.desktop".into(), "عنوان - YouTube — Mozilla Firefox".into()),
            ("org.gnome.Ptyxis.desktop".into(), "main.rs".into()),
        ], 0);
        let sys = SysEvents::new();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            run_tracker_loop(db.clone(), backend, &sys, &|_, _| None, |app, _, _| {
                // فقط التطبيق غير المستبعد يطلق الإشارة
                if app == "code.desktop" { tx.send(()).unwrap(); }
            });
        });
        rx.recv_timeout(std::time::Duration::from_secs(5)).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(100));
        let db = Db::open(&path);
        let rows = db.get_timeline(0, i64::MAX);
        assert_eq!(rows.len(), 1, "التطبيق والموقع المستبعدان لا يُسجلان");
        assert_eq!(rows[0].app_name, "code.desktop");
        assert_eq!(rows[0].site, "", "الموقع المستبعد لا يظهر في صف الموقع الجديد");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn tracks_own_app_windows() {
        let path = std::env::temp_dir().join(format!("salmonella-tracker-self-{}.db", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let db = std::sync::Arc::new(Db::open(&path));
        let backend = FakeSource(vec![("salmonella-ui.desktop".into(), "Salmonella".into())], 0);
        let sys = SysEvents::new();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            run_tracker_loop(db.clone(), backend, &sys, &|_, _| None, |_, _, _| { let _ = tx.send(()); });
        });
        rx.recv_timeout(std::time::Duration::from_secs(5)).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(100));
        let db = Db::open(&path);
        let rows = db.get_timeline(0, i64::MAX);
        assert_eq!(rows.len(), 1, "نافذة تطبيقنا تُسجل كأي تطبيق");
        assert!(rows[0].app_name.contains("salmonella"), "التطبيق نفسه يُسجل");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn sleep_event_splits_active_entry() {
        let path = std::env::temp_dir().join(format!("salmonella-tracker-sleep-{}.db", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let db = std::sync::Arc::new(Db::open(&path));
        let backend = FakeSource(vec![("a.desktop".into(), "t".into())], 0);
        let sys = std::sync::Arc::new(SysEvents::new());
        let sys2 = sys.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            run_tracker_loop(db.clone(), backend, &sys2, &|_, _| None, |_, _, _| {
                let _ = tx.send(());
            });
        });
        rx.recv_timeout(std::time::Duration::from_secs(5)).unwrap();

        let t = unix_now();
        sys.push("sleep", t);
        // على_change التالية تطلقها نفس الدورة التي تعالج النوم وتفتح إدخالاً جديداً
        rx.recv_timeout(std::time::Duration::from_secs(5)).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(100));
        let db = Db::open(&path);
        let rows = db.get_timeline(0, i64::MAX);
        let first = rows.iter().find(|r| r.end_time == Some(t)).expect("entry before sleep");
        assert_eq!(first.detail, "");
        let sleep_row = rows.iter().find(|r| r.detail == "sleep").expect("sleep row");
        assert_eq!(sleep_row.event_type, "system");
        assert_eq!(sleep_row.start_time, t);
        assert_eq!(sleep_row.end_time, None, "صف النوم مفتوح حتى الاستيقاظ");
        let fresh = rows.iter().find(|r| r.detail.is_empty() && r.start_time >= t)
            .expect("entry after wake");
        assert_eq!(fresh.end_time, None, "إدخال جديد مفتوح بعد النوم");

        let wake_t = unix_now();
        sys.push("wake", wake_t);
        // لا إشارة change_ عند المعالجة — استطلاع حتى يُغلق صف النوم
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let sleep_end = loop {
            let rows = Db::open(&path).get_timeline(0, i64::MAX);
            let end = rows.iter().find(|r| r.detail == "sleep").unwrap().end_time;
            if end.is_some() { break end.unwrap(); }
            if std::time::Instant::now() > deadline { panic!("sleep row never closed"); }
            std::thread::sleep(std::time::Duration::from_millis(100));
        };
        let now = unix_now();
        assert!(sleep_end >= wake_t && sleep_end <= now, "يُغلق عند لحظة الاستيقاظ");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn folder_context_becomes_weak_series() {
        let path = std::env::temp_dir().join(format!("salmonella-tracker-ctx-{}.db", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let db = std::sync::Arc::new(Db::open(&path));
        let backend = FakeSource(vec![
            ("org.gnome.Nautilus.desktop".into(), "تفسير آية الكرسي".into()),
            ("mpv.desktop".into(), "الدرس 2 - mpv".into()),
        ], 0);
        let sys = SysEvents::new();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut n = 0;
            run_tracker_loop(db.clone(), backend, &sys, &|_, _| None, |_, _, _| {
                n += 1;
                if n == 2 { tx.send(()).unwrap(); }
            });
        });
        rx.recv_timeout(std::time::Duration::from_secs(5)).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(100));
        let db = Db::open(&path);
        let rows = db.get_timeline(0, i64::MAX);
        assert_eq!(rows[0].series, "تفسير آية الكرسي", "سلسلة mpv الضعيفة ترث المجلد المُتصفَّح");
        assert_eq!(rows[0].episode, "2", "الحلقة تبقى من enrich");
        assert_eq!(rows[1].app_name, "org.gnome.Nautilus.desktop", "صف Nautilus يُسجَّل كالمعتاد");
        assert_eq!(rows[1].series, "", "صف المدير نفسه لا يستعير عنوانه كسلسلة");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn final_series_expired_folder_keeps_enriched() {
        let folder = Some(("تفسير آية الكرسي".to_string(), 1000));
        assert_eq!(final_series("الدرس", true, None, folder.as_ref(), 1601), "الدرس",
            "601 ثانية خارج نافذة الـ 600");
        assert_eq!(final_series("الدرس", true, None, folder.as_ref(), 1600), "تفسير آية الكرسي",
            "600 ثانية داخل النافذة");
    }

    #[test]
    fn final_series_folder_never_overrides_strong() {
        let folder = Some(("Videos".to_string(), 1000));
        assert_eq!(final_series("SpongeBob", false, None, folder.as_ref(), 1100), "SpongeBob",
            "سلسلة قوية لا يُجاوزها المجلد");
    }

    #[test]
    fn explicit_override_beats_folder_and_enriched() {
        let path = std::env::temp_dir().join(format!("salmonella-tracker-ovr-{}.db", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let db = std::sync::Arc::new(Db::open(&path));
        db.set_series_override("الدرس", "تفسير آية الكرسي");
        let backend = FakeSource(vec![
            ("mpv.desktop".into(), "الدرس 2 - mpv".into()),
        ], 0);
        let sys = SysEvents::new();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            run_tracker_loop(db.clone(), backend, &sys, &|_, _| None, |_, _, _| { let _ = tx.send(()); });
        });
        rx.recv_timeout(std::time::Duration::from_secs(5)).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(100));
        let db = Db::open(&path);
        let rows = db.get_timeline(0, i64::MAX);
        assert_eq!(rows[0].series, "تفسير آية الكرسي", "التجاوز الصريح يغلب المحلَّلة");
        assert_eq!(rows[0].episode, "2", "الحلقة تبقى من enrich — التجاوز لا يمسّها");
        let _ = std::fs::remove_file(&path);
    }

    fn media_kind_of(path: &std::path::Path, window_title: &str) -> String {
        rusqlite::Connection::open(path).unwrap().query_row(
            "SELECT media_kind FROM activity_logs WHERE window_title=?1 ORDER BY id DESC LIMIT 1",
            [window_title], |r| r.get(0)).unwrap()
    }

    #[test]
    fn mpris_audio_overrides_watching() {
        let path = std::env::temp_dir().join(format!("salmonella-tracker-mpris-a-{}.db", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let db = std::sync::Arc::new(Db::open(&path));
        let backend = FakeSource(vec![
            ("mpv.desktop".into(), "أغنية بلا امتداد - mpv".into()),
        ], 0);
        let sys = SysEvents::new();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let meta = |_: &str, _: &str| Some(MediaMeta { title: None, mime: Some(MediaMime::Audio) });
            run_tracker_loop(db.clone(), backend, &sys, &meta, |_, _, _| { let _ = tx.send(()); });
        });
        rx.recv_timeout(std::time::Duration::from_secs(5)).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(100));
        let db = Db::open(&path);
        let rows = db.get_timeline(0, i64::MAX);
        assert_eq!(media_kind_of(&path, "أغنية بلا امتداد - mpv"), "listening",
            "تجاوز mime الصوتي يغلب watching من فرع المشغل");
        assert_eq!(rows[0].window_title, "أغنية بلا امتداد - mpv",
            "العنوان غير العام لا يُستبدل");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn mpris_title_substituted_for_generic() {
        let path = std::env::temp_dir().join(format!("salmonella-tracker-mpris-t-{}.db", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let db = std::sync::Arc::new(Db::open(&path));
        let backend = FakeSource(vec![
            ("org.gnome.Showtime.desktop".into(), "Video Player".into()),
        ], 0);
        let sys = SysEvents::new();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let meta = |_: &str, _: &str| Some(MediaMeta { title: Some("فيلم".to_string()),
                mime: Some(MediaMime::Video) });
            run_tracker_loop(db.clone(), backend, &sys, &meta, |_, _, _| { let _ = tx.send(()); });
        });
        rx.recv_timeout(std::time::Duration::from_secs(5)).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(100));
        let db = Db::open(&path);
        let rows = db.get_timeline(0, i64::MAX);
        assert_eq!(rows[0].window_title, "فيلم", "العنوان البديل يحل محل العام");
        assert_eq!(media_kind_of(&path, "فيلم"), "watching");
        assert_eq!(rows[0].series, "", "كشف الحلقة على العنوان الحقيقي — بلا سلسلة");
        assert_eq!(rows[0].episode, "");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn mpris_never_overrides_reading() {
        let path = std::env::temp_dir().join(format!("salmonella-tracker-mpris-r-{}.db", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let db = std::sync::Arc::new(Db::open(&path));
        let backend = FakeSource(vec![
            ("org.gnome.Papers.desktop".into(), "رسالة في الطريق الي ثقافتنا.pdf".into()),
        ], 0);
        let sys = SysEvents::new();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let meta = |_: &str, _: &str| Some(MediaMeta { title: Some("x".to_string()),
                mime: Some(MediaMime::Video) });
            run_tracker_loop(db.clone(), backend, &sys, &meta, |_, _, _| { let _ = tx.send(()); });
        });
        rx.recv_timeout(std::time::Duration::from_secs(5)).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(100));
        let db = Db::open(&path);
        let rows = db.get_timeline(0, i64::MAX);
        assert_eq!(media_kind_of(&path, "رسالة في الطريق الي ثقافتنا.pdf"), "reading",
            "Papers خارج is_media_app — لا تجاوز");
        assert_eq!(rows[0].window_title, "رسالة في الطريق الي ثقافتنا.pdf",
            "العنوان غير العام لا يُستبدل حتى مع mime");
        let _ = std::fs::remove_file(&path);
    }
}
