use std::path::Path;
use std::sync::Mutex;
use rusqlite::{Connection, params};
use serde::Serialize;

use crate::classifier::{builtin_name, short_name};

#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct LogEntry {
    pub id: i64,
    pub event_type: String,
    pub app_name: String,
    pub window_title: String,
    pub start_time: i64,
    pub end_time: Option<i64>,
    pub duration: Option<i64>,
    pub friendly_name: String,
    pub site: String,
    pub site_friendly: String,
    pub category: String,
    pub series: String,
    pub episode: String,
    pub detail: String,
}

/// حمولة الإدراج في السجل.
pub struct LogEvent<'a> {
    pub event_type: &'a str,
    pub category: &'a str,
    pub friendly: &'a str,
    pub site: &'a str,
    pub site_friendly: &'a str,
    pub series: &'a str,
    pub episode: &'a str,
    pub app: &'a str,
    pub title: &'a str,
}

pub struct Db { conn: Mutex<Connection> }

const NEW_COLUMNS: &[(&str, &str)] = &[
    ("friendly_name", "TEXT DEFAULT ''"),
    ("site", "TEXT DEFAULT ''"),
    ("category", "TEXT DEFAULT ''"),
    ("series", "TEXT DEFAULT ''"),
    ("episode", "TEXT DEFAULT ''"),
    ("site_friendly", "TEXT DEFAULT ''"),
    ("detail", "TEXT DEFAULT ''"),
];

impl Db {
    pub fn new() -> Self {
        let path = dirs::data_local_dir().unwrap_or_default().join("salmonella/activity.db");
        std::fs::create_dir_all(path.parent().unwrap()).ok();
        Self::open(&path)
    }

    pub fn open(path: impl AsRef<Path>) -> Self {
        let conn = Connection::open(path).expect("db open");
        let db = Db { conn: Mutex::new(conn) };
        db.migrate();
        db
    }

    fn migrate(&self) {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS activity_logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                event_type TEXT NOT NULL CHECK(event_type IN ('system','app','media')),
                app_name TEXT NOT NULL DEFAULT '',
                window_title TEXT NOT NULL DEFAULT '',
                start_time INTEGER NOT NULL,
                end_time INTEGER,
                duration INTEGER
            );
            CREATE INDEX IF NOT EXISTS idx_start_time ON activity_logs(start_time);
            CREATE INDEX IF NOT EXISTS idx_window_title ON activity_logs(window_title);
            CREATE INDEX IF NOT EXISTS idx_event_type ON activity_logs(event_type);
            CREATE TABLE IF NOT EXISTS name_overrides (
                app_id TEXT PRIMARY KEY,
                friendly TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS limits (
                target TEXT PRIMARY KEY,
                kind TEXT NOT NULL CHECK(kind IN ('app','category','site')),
                daily_minutes INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS custom_categories (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                kind TEXT NOT NULL CHECK(kind IN ('app','site')),
                target TEXT NOT NULL,
                display_name TEXT NOT NULL,
                UNIQUE(kind, target)
            );
            CREATE TABLE IF NOT EXISTS ignored (
                kind TEXT NOT NULL,
                target TEXT NOT NULL,
                PRIMARY KEY (kind, target)
            );
            CREATE TABLE IF NOT EXISTS site_overrides (
                site TEXT PRIMARY KEY,
                friendly TEXT NOT NULL
            );"
        ).expect("migrate tables");

        // ترحيل: limits القديمة تحظر kind='site' (SQLite لا يعدّل CHECK — إعادة بناء الجدول)
        let limits_sql: String = conn.query_row(
            "SELECT sql FROM sqlite_master WHERE type='table' AND name='limits'",
            [], |r| r.get(0)).unwrap_or_default();
        if limits_sql.contains("('app','category')") {
            conn.execute_batch(
                "ALTER TABLE limits RENAME TO limits_old;
                 CREATE TABLE limits (
                    target TEXT PRIMARY KEY,
                    kind TEXT NOT NULL CHECK(kind IN ('app','category','site')),
                    daily_minutes INTEGER NOT NULL
                 );
                 INSERT INTO limits SELECT target, kind, daily_minutes FROM limits_old;
                 DROP TABLE limits_old;"
            ).expect("rebuild limits table");
        }

        // SQLite لا يدعم ADD COLUMN IF NOT EXISTS — فحص يدوي
        let existing: Vec<String> = conn.prepare("PRAGMA table_info(activity_logs)")
            .unwrap().query_map([], |r| r.get::<_, String>(1)).unwrap()
            .filter_map(|r| r.ok()).collect();
        for (col, ty) in NEW_COLUMNS {
            if !existing.contains(&col.to_string()) {
                conn.execute(&format!("ALTER TABLE activity_logs ADD COLUMN {col} {ty}"), []).expect("add column");
            }
        }
        conn.execute("CREATE INDEX IF NOT EXISTS idx_category ON activity_logs(category)", []).ok();
        conn.execute("CREATE INDEX IF NOT EXISTS idx_series ON activity_logs(series)", []).ok();
    }

    pub fn insert_log(&self, e: &LogEvent, t: i64) -> i64 {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO activity_logs(event_type,app_name,window_title,start_time,
                 friendly_name,site,site_friendly,category,series,episode)
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            params![e.event_type, e.app, e.title, t, e.friendly, e.site, e.site_friendly, e.category, e.series, e.episode],
        ).ok();
        conn.last_insert_rowid()
    }

    /// Closes any entries left open by a previous run (crash/kill/reboot).
    /// If the last-alive heartbeat is stale (>60s ago), dangling entries are
    /// closed at that heartbeat instead of at boot, so an open entry doesn't
    /// span a power cut / suspend.
    pub fn close_dangling(&self, now: i64) {
        let last_alive: i64 = self.get_setting("last_alive")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        let end = if last_alive > 0 && now - last_alive > 60 { last_alive } else { now };
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE activity_logs SET end_time=?1, duration=?1-start_time WHERE end_time IS NULL",
            params![end],
        ).unwrap();
    }

    /// Inserts a system event row (boot/login/logout/power_off/sleep/wake/...).
    pub fn insert_system_event(&self, detail: &str, title: &str, start: i64, end: Option<i64>) -> i64 {
        let conn = self.conn.lock().unwrap();
        let (end_time, duration) = match end {
            Some(e) => (Some(e), Some(e - start)),
            None => (None, None),
        };
        conn.execute(
            "INSERT INTO activity_logs(event_type,app_name,window_title,start_time,end_time,duration,
                 friendly_name,site,site_friendly,category,series,episode,detail)
             VALUES('system','',?1,?2,?3,?4,'','','','','','',?5)",
            params![title, start, end_time, duration, detail],
        ).ok();
        conn.last_insert_rowid()
    }

    pub fn close_log(&self, id: i64, end: i64) {
        let conn = self.conn.lock().unwrap();
        if let Ok(start) = conn.query_row(
            "SELECT start_time FROM activity_logs WHERE id=?1", params![id], |r| r.get::<_, i64>(0)
        ) {
            conn.execute(
                "UPDATE activity_logs SET end_time=?1, duration=?2 WHERE id=?3",
                params![end, end - start, id],
            ).ok();
        }
    }

    const LOG_COLS: &'static str = "id,event_type,app_name,window_title,start_time,end_time,duration,
        friendly_name,site,site_friendly,category,series,episode,detail";

    fn row_to_entry(r: &rusqlite::Row) -> rusqlite::Result<LogEntry> {
        Ok(LogEntry {
            id: r.get(0)?, event_type: r.get(1)?, app_name: r.get(2)?,
            window_title: r.get(3)?, start_time: r.get(4)?,
            end_time: r.get(5)?, duration: r.get(6)?,
            friendly_name: r.get(7)?, site: r.get(8)?, site_friendly: r.get(9)?,
            category: r.get(10)?, series: r.get(11)?, episode: r.get(12)?,
            detail: r.get(13)?,
        })
    }

    pub fn get_timeline(&self, from: i64, to: i64) -> Vec<LogEntry> {
        let conn = self.conn.lock().unwrap();
        let sql = format!("SELECT {} FROM activity_logs WHERE start_time>=?1 AND start_time<=?2 ORDER BY start_time DESC", Self::LOG_COLS);
        let mut stmt = conn.prepare(&sql).unwrap();
        stmt.query_map(params![from, to], Self::row_to_entry).unwrap()
            .filter_map(|r| r.ok()).collect()
    }

    pub fn get_status(&self) -> (i64, String, String) {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT start_time,app_name,window_title FROM activity_logs
             WHERE end_time IS NULL AND event_type != 'system' ORDER BY id DESC LIMIT 1",
            [], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        ).unwrap_or_default()
    }

    pub fn search(&self, q: &str) -> Vec<LogEntry> {
        let conn = self.conn.lock().unwrap();
        let p = format!("%{}%", q);
        let sql = format!("SELECT {} FROM activity_logs WHERE window_title LIKE ?1 OR app_name LIKE ?1 ORDER BY start_time DESC LIMIT 100", Self::LOG_COLS);
        let mut stmt = conn.prepare(&sql).unwrap();
        stmt.query_map(params![p], Self::row_to_entry).unwrap()
            .filter_map(|r| r.ok()).collect()
    }

    pub fn get_report(&self, from: i64, to: i64, group_by: &str) -> Vec<(String, i64)> {
        let col = match group_by {
            "app" => "friendly_name",
            "category" => "category",
            "site" => "COALESCE(NULLIF(site_friendly,''), site)",
            "series" => "series",
            _ => return Vec::new(),
        };
        let conn = self.conn.lock().unwrap();
        let sql = format!(
            "SELECT {col}, SUM(COALESCE(duration,0)) AS total FROM activity_logs
             WHERE start_time>=?1 AND start_time<=?2 AND {col} != ''
             GROUP BY {col} ORDER BY total DESC");
        let mut stmt = conn.prepare(&sql).unwrap();
        stmt.query_map(params![from, to], |r| Ok((r.get(0)?, r.get(1)?))).unwrap()
            .filter_map(|r| r.ok()).collect()
    }

    pub fn get_series(&self, from: i64, to: i64) -> Vec<(String, String, i64)> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT series, episode, duration FROM activity_logs
             WHERE series != '' AND episode != '' AND duration IS NOT NULL
               AND start_time>=?1 AND start_time<=?2 ORDER BY start_time").unwrap();
        stmt.query_map(params![from, to], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).unwrap()
            .filter_map(|r| r.ok()).collect()
    }

    pub fn get_limits(&self) -> Vec<(String, String, i64)> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT target, kind, daily_minutes FROM limits").unwrap();
        stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).unwrap()
            .filter_map(|r| r.ok()).collect()
    }

    pub fn set_limit(&self, target: &str, kind: &str, minutes: i64) {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO limits(target,kind,daily_minutes) VALUES(?1,?2,?3)
             ON CONFLICT(target) DO UPDATE SET daily_minutes=?3, kind=?2",
            params![target, kind, minutes],
        ).ok();
    }

    pub fn remove_limit(&self, target: &str) {
        self.conn.lock().unwrap().execute("DELETE FROM limits WHERE target=?1", params![target]).ok();
    }

    pub fn get_name_overrides(&self) -> Vec<(String, String)> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT app_id, friendly FROM name_overrides").unwrap();
        stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?))).unwrap()
            .filter_map(|r| r.ok()).collect()
    }

    pub fn set_name_override(&self, app_id: &str, friendly: &str) {
        {
            let conn = self.conn.lock().unwrap();
            conn.execute(
                "INSERT INTO name_overrides(app_id,friendly) VALUES(?1,?2)
                 ON CONFLICT(app_id) DO UPDATE SET friendly=?2",
                params![app_id, friendly],
            ).ok();
        }
        self.apply_app_rename(app_id, friendly);
    }

    pub fn remove_name_override(&self, app_id: &str) {
        self.conn.lock().unwrap().execute("DELETE FROM name_overrides WHERE app_id=?1", params![app_id]).ok();
    }

    /// تحديث رجعي لاسم العرض في صفوف السجل التاريخية.
    fn apply_app_rename(&self, app: &str, friendly: &str) {
        self.conn.lock().unwrap().execute(
            "UPDATE activity_logs SET friendly_name=?2 WHERE app_name=?1",
            params![app, friendly],
        ).ok();
    }

    pub fn get_site_overrides(&self) -> Vec<(String, String)> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT site, friendly FROM site_overrides").unwrap();
        stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?))).unwrap()
            .filter_map(|r| r.ok()).collect()
    }

    fn get_site_override(&self, site: &str) -> Option<String> {
        self.conn.lock().unwrap().query_row(
            "SELECT friendly FROM site_overrides WHERE site=?1", params![site], |r| r.get::<_, String>(0)
        ).ok()
    }

    pub fn site_friendly_name(&self, site: &str) -> String {
        self.get_site_override(site).unwrap_or_else(|| site.to_string())
    }

    pub fn set_site_override(&self, site: &str, friendly: &str) {
        {
            let conn = self.conn.lock().unwrap();
            conn.execute(
                "INSERT INTO site_overrides(site,friendly) VALUES(?1,?2)
                 ON CONFLICT(site) DO UPDATE SET friendly=?2",
                params![site, friendly],
            ).ok();
        }
        self.conn.lock().unwrap().execute(
            "UPDATE activity_logs SET site_friendly=?2 WHERE site=?1",
            params![site, friendly],
        ).ok();
    }

    pub fn remove_site_override(&self, site: &str) {
        self.conn.lock().unwrap().execute("DELETE FROM site_overrides WHERE site=?1", params![site]).ok();
    }

    pub fn is_ignored(&self, kind: &str, target: &str) -> bool {
        if target.is_empty() { return false; }
        let sql = if kind == "site" {
            "SELECT 1 FROM ignored WHERE kind='site' AND lower(target)=lower(?1)"
        } else {
            "SELECT 1 FROM ignored WHERE kind=?1 AND target=?2"
        };
        let params: Vec<&dyn rusqlite::ToSql> = if kind == "site" {
            vec![&target]
        } else {
            vec![&kind, &target]
        };
        self.conn.lock().unwrap().query_row(sql, params.as_slice(), |_| Ok(())).is_ok()
    }

    pub fn ignore_target(&self, kind: &str, target: &str) {
        self.conn.lock().unwrap().execute(
            "INSERT OR IGNORE INTO ignored(kind,target) VALUES(?1,?2)",
            params![kind, target]).ok();
    }

    pub fn unignore_target(&self, kind: &str, target: &str) {
        let sql = if kind == "site" {
            "DELETE FROM ignored WHERE kind='site' AND lower(target)=lower(?1)"
        } else {
            "DELETE FROM ignored WHERE kind=?1 AND target=?2"
        };
        let params: Vec<&dyn rusqlite::ToSql> = if kind == "site" {
            vec![&target]
        } else {
            vec![&kind, &target]
        };
        self.conn.lock().unwrap().execute(sql, params.as_slice()).ok();
    }

    pub fn list_ignored(&self) -> Vec<(String, String)> {
        self.conn.lock().unwrap()
            .prepare("SELECT kind, target FROM ignored ORDER BY kind, target").unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?))).unwrap()
            .filter_map(|r| r.ok()).collect()
    }

    /// إعادة اشتقاق الموقع للصفوف القديمة بعد تحسين extractor —
    /// للمتصفحات فقط، الفئة لا تتغير، النتيجة خاملة عند التكرار.
    pub fn backfill_sites(&self) {
        let rows: Vec<(i64, String, String)> = {
            let conn = self.conn.lock().unwrap();
            let mut stmt = conn.prepare("SELECT id, app_name, window_title FROM activity_logs").unwrap();
            stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).unwrap()
                .filter_map(|r| r.ok()).collect()
        };
        for (id, app, title) in rows {
            if !crate::classifier::is_browser_app(&app) { continue; }
            let e = crate::classifier::enrich(&app, &title);
            let site_friendly = self.site_friendly_name(&e.site);
            self.conn.lock().unwrap().execute(
                "UPDATE activity_logs SET site=?2, site_friendly=?3 WHERE id=?1",
                params![id, e.site, site_friendly]).ok();
        }
    }

    /// كل التطبيقات المعروفة من السجل، الأحدث استخداماً أولاً.
    /// (القائمة مشتقة من السجل — لا جدول تتبع منفصل؛ تتحدث بفتح تطبيقات جديدة.)
    pub fn get_known_apps(&self) -> Vec<(String, String)> {
        let apps: Vec<String> = {
            let conn = self.conn.lock().unwrap();
            let mut stmt = conn.prepare(
                "SELECT app_name FROM activity_logs
                 WHERE app_name != '' GROUP BY app_name ORDER BY MAX(start_time) DESC"
            ).unwrap();
            stmt.query_map([], |r| r.get::<_, String>(0)).unwrap()
                .filter_map(|r| r.ok()).collect()
        };
        apps.into_iter()
            .map(|app| {
                let disp = self.friendly_name(&app);
                (app, disp)
            }).collect()
    }

    pub fn get_known_sites(&self) -> Vec<(String, String)> {
        // دمج بحروف متشابهة (X و x) مع إبقاء أحدث استخدام؛ استبعاد المهملات فقط
        // (المُستبعدة تبقى ظاهرة حتى يستعيدها المستخدم من الواجهة)
        let sites: Vec<String> = {
            let conn = self.conn.lock().unwrap();
            let mut stmt = conn.prepare(
                "SELECT site FROM activity_logs
                 WHERE site != '' GROUP BY lower(site) ORDER BY MAX(start_time) DESC"
            ).unwrap();
            stmt.query_map([], |r| r.get::<_, String>(0)).unwrap()
                .filter_map(|r| r.ok()).collect()
        };
        sites.into_iter()
            .filter(|s| !crate::classifier::is_junk_site(s))
            .map(|site| {
                let disp = self.site_friendly_name(&site);
                (site, disp)
            }).collect()
    }

    pub fn get_setting(&self, key: &str) -> Option<String> {
        self.conn.lock().unwrap().query_row(
            "SELECT value FROM settings WHERE key=?1", params![key], |r| r.get::<_, String>(0)
        ).ok()
    }

    pub fn set_setting(&self, key: &str, value: &str) {
        self.conn.lock().unwrap().execute(
            "INSERT INTO settings(key,value) VALUES(?1,?2)
             ON CONFLICT(key) DO UPDATE SET value=?2",
            params![key, value],
        ).ok();
    }

    pub fn friendly_name(&self, app: &str) -> String {
        let conn = self.conn.lock().unwrap();
        if let Ok(f) = conn.query_row(
            "SELECT friendly FROM name_overrides WHERE app_id=?1", params![app], |r| r.get::<_, String>(0)
        ) {
            return f;
        }
        builtin_name(app).unwrap_or_else(|| short_name(app))
    }

    pub fn list_custom_categories(&self) -> Vec<(i64, String, String, String)> {
        self.conn.lock().unwrap()
            .prepare("SELECT id, kind, target, display_name FROM custom_categories ORDER BY id")
            .unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
            .unwrap().filter_map(|r| r.ok()).collect()
    }

    pub fn add_custom_category(&self, kind: &str, target: &str, display_name: &str) -> Option<i64> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO custom_categories(kind,target,display_name) VALUES(?1,?2,?3)
             ON CONFLICT(kind,target) DO UPDATE SET display_name=excluded.display_name",
            params![kind, target, display_name],
        ).ok()?;
        let id: i64 = conn.query_row(
            "SELECT id FROM custom_categories WHERE kind=?1 AND target=?2",
            params![kind, target], |r| r.get(0)
        ).ok()?;
        Some(id)
    }

    pub fn remove_custom_category(&self, id: i64) {
        self.conn.lock().unwrap().execute("DELETE FROM custom_categories WHERE id=?1", params![id]).ok();
    }

    /// لو وُجدت قاعدة مخصصة مطابقة (kind=app, target=app أو kind=site, target=site)
    /// فأعد اسم العرض، وإلا None.
    pub fn match_custom_category(&self, kind: &str, target: &str) -> Option<String> {
        if target.is_empty() { return None; }
        self.conn.lock().unwrap().query_row(
            "SELECT display_name FROM custom_categories WHERE kind=?1 AND target=?2 LIMIT 1",
            params![kind, target], |r| r.get::<_, String>(0)
        ).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_db(name: &str) -> Db {
        let path = std::env::temp_dir().join(format!("salmonella-{}-{}.db", std::process::id(), name));
        let _ = std::fs::remove_file(&path);
        Db::open(&path)
    }

    #[test] fn migration_preserves_old_rows() {
        let path = std::env::temp_dir().join(format!("salmonella-{}-migrate-old.db", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let old = rusqlite::Connection::open(&path).unwrap();
        old.execute_batch(
            "CREATE TABLE activity_logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                event_type TEXT NOT NULL CHECK(event_type IN ('system','app','media')),
                app_name TEXT NOT NULL DEFAULT '',
                window_title TEXT NOT NULL DEFAULT '',
                start_time INTEGER NOT NULL,
                end_time INTEGER,
                duration INTEGER
            );
            INSERT INTO activity_logs(event_type, app_name, window_title, start_time)
                VALUES('app','org.mozilla.firefox.desktop','Old Title',1000);"
        ).unwrap();
        drop(old);
        let db = Db::open(&path);
        let rows = db.get_timeline(0, 999_999);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].window_title, "Old Title");
        assert_eq!(rows[0].friendly_name, "");
    }

    #[test] fn migration_adds_columns_to_old_db() {
        let db = tmp_db("migrate");
        db.insert_log(&LogEvent { event_type: "app", category: "productivity", friendly: "الطرفية",
            site: "", site_friendly: "", series: "", episode: "", app: "org.gnome.Ptyxis.desktop", title: "bash" }, 1000);
        let rows = db.get_timeline(0, 2000);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].friendly_name, "الطرفية");
        assert_eq!(rows[0].category, "productivity");
    }

    #[test] fn friendly_name_override_wins() {
        let db = tmp_db("override");
        assert_eq!(db.friendly_name("org.mozilla.firefox.desktop"), "فايرفوكس");
        db.set_name_override("org.mozilla.firefox.desktop", "متصفحي");
        assert_eq!(db.friendly_name("org.mozilla.firefox.desktop"), "متصفحي");
        db.remove_name_override("org.mozilla.firefox.desktop");
        assert_eq!(db.friendly_name("org.mozilla.firefox.desktop"), "فايرفوكس");
    }

    #[test] fn report_groups_by_friendly() {
        let db = tmp_db("report");
        let now = 10_000;
        for (app, friendly, dur_sec) in [("a", "تطبيق أ", 60), ("a", "تطبيق أ", 30), ("b", "تطبيق ب", 10)] {
            let id = db.insert_log(&LogEvent { event_type: "app", category: "other", friendly,
                site: "", site_friendly: "", series: "", episode: "", app, title: "t" }, now);
            db.close_log(id, now + dur_sec);
        }
        let r = db.get_report(0, 999_999, "app");
        assert_eq!(r, vec![("تطبيق أ".to_string(), 90), ("تطبيق ب".to_string(), 10)]);
    }

    #[test] fn report_groups_by_category_and_series() {
        let db = tmp_db("report2");
        let id = db.insert_log(&LogEvent { event_type: "media", category: "media", friendly: "مشغل",
            site: "", site_friendly: "", series: "الدرس", episode: "26", app: "mpv", title: "الدرس 26" }, 1000);
        db.close_log(id, 1100);
        let cat = db.get_report(0, 999_999, "category");
        assert_eq!(cat, vec![("media".to_string(), 100)]);
        let ser = db.get_report(0, 999_999, "series");
        assert_eq!(ser, vec![("الدرس".to_string(), 100)]);
        let series = db.get_series(0, 999_999);
        assert_eq!(series, vec![("الدرس".to_string(), "26".to_string(), 100)]);
    }

    #[test] fn close_dangling_closes_open_rows() {
        let db = tmp_db("dangling");
        let id = db.insert_log(&LogEvent { event_type: "app", category: "other", friendly: "",
            site: "", site_friendly: "", series: "", episode: "", app: "x", title: "y" }, 1000);
        db.close_dangling(2000);
        let (end, dur): (i64, i64) = db.conn.lock().unwrap().query_row(
            "SELECT end_time, duration FROM activity_logs WHERE id=?1", params![id],
            |r| Ok((r.get(0)?, r.get(1)?))).unwrap();
        assert_eq!((end, dur), (2000, 1000));
    }

    #[test] fn close_dangling_uses_last_alive_when_stale() {
        // power cut: stale last_alive (5000) → dangling entry closes there, not at boot (10000)
        let db1 = tmp_db("dangling-la1");
        let id1 = db1.insert_log(&LogEvent { event_type: "app", category: "other", friendly: "",
            site: "", site_friendly: "", series: "", episode: "", app: "x", title: "y" }, 1000);
        db1.set_setting("last_alive", "5000");
        db1.close_dangling(10000);
        let (end, dur): (i64, i64) = db1.conn.lock().unwrap().query_row(
            "SELECT end_time, duration FROM activity_logs WHERE id=?1", params![id1],
            |r| Ok((r.get(0)?, r.get(1)?))).unwrap();
        assert_eq!((end, dur), (5000, 4000));

        // fresh last_alive (≤60s ago) → close at now
        let db2 = tmp_db("dangling-la2");
        let id2 = db2.insert_log(&LogEvent { event_type: "app", category: "other", friendly: "",
            site: "", site_friendly: "", series: "", episode: "", app: "x", title: "y" }, 1000);
        db2.set_setting("last_alive", "9950");
        db2.close_dangling(10000);
        let (end, dur): (i64, i64) = db2.conn.lock().unwrap().query_row(
            "SELECT end_time, duration FROM activity_logs WHERE id=?1", params![id2],
            |r| Ok((r.get(0)?, r.get(1)?))).unwrap();
        assert_eq!((end, dur), (10000, 9000));

        // no last_alive → close at now
        let db3 = tmp_db("dangling-la3");
        let id3 = db3.insert_log(&LogEvent { event_type: "app", category: "other", friendly: "",
            site: "", site_friendly: "", series: "", episode: "", app: "x", title: "y" }, 1000);
        db3.close_dangling(10000);
        let (end, dur): (i64, i64) = db3.conn.lock().unwrap().query_row(
            "SELECT end_time, duration FROM activity_logs WHERE id=?1", params![id3],
            |r| Ok((r.get(0)?, r.get(1)?))).unwrap();
        assert_eq!((end, dur), (10000, 9000));
    }

    #[test] fn insert_system_event_rows() {
        let db = tmp_db("sysevent");
        let id = db.insert_system_event("sleep", "", 1000, None);
        let (ty, det, end, dur): (String, String, Option<i64>, Option<i64>) =
            db.conn.lock().unwrap().query_row(
                "SELECT event_type, detail, end_time, duration FROM activity_logs WHERE id=?1",
                params![id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))).unwrap();
        assert_eq!(ty, "system");
        assert_eq!(det, "sleep");
        assert_eq!(end, None);
        assert_eq!(dur, None);
        assert_eq!(db.get_status(), (0, String::new(), String::new()),
            "الصف النظامي المفتوح لا يُعرض كنشاط حالي");
        db.close_log(id, 2000);
        let (end, dur): (Option<i64>, Option<i64>) = db.conn.lock().unwrap().query_row(
            "SELECT end_time, duration FROM activity_logs WHERE id=?1", params![id],
            |r| Ok((r.get(0)?, r.get(1)?))).unwrap();
        assert_eq!(end, Some(2000));
        assert_eq!(dur, Some(1000));

        let boot = db.insert_system_event("boot", "", 3000, Some(3000));
        let (end, dur): (Option<i64>, Option<i64>) = db.conn.lock().unwrap().query_row(
            "SELECT end_time, duration FROM activity_logs WHERE id=?1", params![boot],
            |r| Ok((r.get(0)?, r.get(1)?))).unwrap();
        assert_eq!(end, Some(3000));
        assert_eq!(dur, Some(0));
    }

    #[test] fn limits_crud() {
        let db = tmp_db("limits");
        assert!(db.get_limits().is_empty());
        db.set_limit("youtube", "category", 30);
        db.set_limit("youtube", "category", 45);
        assert_eq!(db.get_limits(), vec![("youtube".to_string(), "category".to_string(), 45)]);
        db.remove_limit("youtube");
        assert!(db.get_limits().is_empty());
    }

    #[test] fn custom_categories_crud() {
        let db = tmp_db("custom");
        assert!(db.list_custom_categories().is_empty());
        let id = db.add_custom_category("app", "firefox", "برمجة").unwrap();
        assert!(id > 0);
        let id_dup = db.add_custom_category("app", "firefox", "برمجة أخرى").unwrap();
        assert_eq!(id, id_dup, "نفس (kind, target) يُحدّث ولا يضيف");
        assert_eq!(db.match_custom_category("app", "firefox").unwrap(), "برمجة أخرى");
        assert!(db.match_custom_category("site", "").is_none());
        db.remove_custom_category(id);
        assert!(db.list_custom_categories().is_empty());
    }

    #[test] fn site_overrides_crud() {
        let db = tmp_db("siteov");
        assert!(db.get_site_overrides().is_empty());
        assert_eq!(db.site_friendly_name("youtube.com"), "youtube.com");
        db.set_site_override("youtube.com", "يوتيوب");
        assert_eq!(db.site_friendly_name("youtube.com"), "يوتيوب");
        db.set_site_override("youtube.com", "يوتيوب 2");
        assert_eq!(db.site_friendly_name("youtube.com"), "يوتيوب 2", "التحديث يغيّر الاسم");
        db.remove_site_override("youtube.com");
        assert_eq!(db.site_friendly_name("youtube.com"), "youtube.com");
        assert!(db.get_site_overrides().is_empty());
    }

    #[test] fn rename_app_is_retroactive() {
        let db = tmp_db("retroapp");
        let e = LogEvent { event_type: "app", category: "browsing", friendly: "فايرفوكس",
            site: "", site_friendly: "", series: "", episode: "", app: "org.mozilla.firefox.desktop", title: "t" };
        let id = db.insert_log(&e, 1000);
        db.close_log(id, 1100);
        assert_eq!(db.get_timeline(0, 9999)[0].friendly_name, "فايرفوكس");
        db.set_name_override("org.mozilla.firefox.desktop", "متصفحي");
        assert_eq!(db.get_timeline(0, 9999)[0].friendly_name, "متصفحي", "الصفوف القديمة تتحدث");
    }

    #[test] fn rename_site_is_retroactive_and_groups() {
        let db = tmp_db("retrosite");
        let e = LogEvent { event_type: "app", category: "media", friendly: "فايرفوكس",
            site: "youtube.com", site_friendly: "youtube.com", series: "", episode: "",
            app: "org.mozilla.firefox.desktop", title: "t" };
        let id = db.insert_log(&e, 1000);
        db.close_log(id, 1100);
        db.set_site_override("youtube.com", "يوتيوب");
        assert_eq!(db.get_timeline(0, 9999)[0].site_friendly, "يوتيوب");
        let rep = db.get_report(0, 9999, "site");
        assert_eq!(rep, vec![("يوتيوب".to_string(), 100)], "التقرير يجمع تحت الاسم الجديد");
    }

    #[test] fn get_known_apps_derives_distinct() {
        let db = tmp_db("known");
        let e = LogEvent { event_type: "app", category: "browsing", friendly: "فايرفوكس",
            site: "", site_friendly: "", series: "", episode: "", app: "org.mozilla.firefox.desktop", title: "t" };
        let id = db.insert_log(&e, 1000);
        db.close_log(id, 1100);
        let id2 = db.insert_log(&e, 2000);
        db.close_log(id2, 2100);
        let known = db.get_known_apps();
        assert_eq!(known.len(), 1, "التطبيق نفسه لا يتكرر");
        assert_eq!(known[0].0, "org.mozilla.firefox.desktop");
        assert_eq!(known[0].1, "فايرفوكس");
        let sites = db.get_known_sites();
        assert!(sites.is_empty(), "لا مواقع فارغة");
    }

    #[test] fn limits_site_kind() {
        let db = tmp_db("limitsite");
        db.set_limit("youtube.com", "site", 30);
        assert_eq!(db.get_limits(), vec![("youtube.com".to_string(), "site".to_string(), 30)]);
        db.remove_limit("youtube.com");
        assert!(db.get_limits().is_empty());
    }

    #[test] fn limits_migration_to_site_kind() {
        let path = std::env::temp_dir().join(format!("salmonella-{}-limmig.db", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let old = rusqlite::Connection::open(&path).unwrap();
        old.execute_batch(
            "CREATE TABLE limits (
                target TEXT PRIMARY KEY,
                kind TEXT NOT NULL CHECK(kind IN ('app','category')),
                daily_minutes INTEGER NOT NULL
             );
             INSERT INTO limits VALUES('media','category',45);"
        ).unwrap();
        drop(old);
        let db = Db::open(&path);
        assert_eq!(db.get_limits(), vec![("media".to_string(), "category".to_string(), 45)], "الصفوف القديمة تُحفظ");
        db.set_limit("youtube.com", "site", 30);
        assert_eq!(db.get_limits().len(), 2, "kind='site' يعمل بعد الترحيل");
    }

    #[test] fn ignored_crud_and_filtering() {
        let db = tmp_db("ignored");
        let e = LogEvent { event_type: "app", category: "browsing", friendly: "فايرفوكس",
            site: "x", site_friendly: "x", series: "", episode: "",
            app: "org.mozilla.firefox.desktop", title: "t" };
        let id = db.insert_log(&e, 1000);
        db.close_log(id, 1100);
        assert_eq!(db.get_known_apps().len(), 1);
        assert_eq!(db.get_known_sites().len(), 1);
        db.ignore_target("app", "org.mozilla.firefox.desktop");
        db.ignore_target("site", "X");
        assert!(db.is_ignored("app", "org.mozilla.firefox.desktop"));
        assert!(db.is_ignored("site", "x"), "الموقع يتطابق بدون حساسية لحالة الأحرف");
        assert!(db.is_ignored("site", "X"));
        assert!(!db.is_ignored("site", "y"));
        assert!(db.get_known_apps().contains(&("org.mozilla.firefox.desktop".into(), "فايرفوكس".into())),
            "التطبيق المُستبعد يبقى ظاهراً حتى يستعيده المستخدم");
        assert_eq!(db.get_known_sites().len(), 1, "الموقع المُستبعد يبقى ظاهراً");
        assert_eq!(db.list_ignored().len(), 2);
        db.unignore_target("app", "org.mozilla.firefox.desktop");
        db.unignore_target("site", "x");
        assert_eq!(db.get_known_apps().len(), 1);
        assert_eq!(db.get_known_sites().len(), 1);
    }

    #[test] fn known_sites_dedup_case_insensitive() {
        let db = tmp_db("sitededup");
        for (i, s) in ["X", "x", "X / Home"].iter().enumerate() {
            let id = db.insert_log(&LogEvent { event_type: "app", category: "browsing", friendly: "فايرفوكس",
                site: s, site_friendly: s, series: "", episode: "",
                app: "org.mozilla.firefox.desktop", title: "t" }, 1000 + i as i64);
            db.close_log(id, 1100 + i as i64);
        }
        let known = db.get_known_sites();
        assert_eq!(known.len(), 2, "X و x يُدمجان، و X / Home تبقى");
    }

    #[test] fn backfill_reparses_browser_rows_and_is_idempotent() {
        let db = tmp_db("backfill");
        let junk = db.insert_log(&LogEvent { event_type: "app", category: "other", friendly: "فايرفوكس",
            site: "Calculator", site_friendly: "Calculator", series: "", episode: "",
            app: "org.mozilla.firefox.desktop", title: "Calculator — Mozilla Firefox" }, 3000);
        db.close_log(junk, 3100);
        let old_fmt = db.insert_log(
            &LogEvent { event_type: "app", category: "other", friendly: "فايرفوكس",
                site: "X \\ DeepSeek على X: \"طويل جداً جداً من النشر\"",
                site_friendly: "X \\ DeepSeek على X: \"طويل جداً جداً من النشر\"",
                series: "", episode: "",
                app: "org.mozilla.firefox.desktop",
                title: "X \\ DeepSeek على X: \"طويل جداً جداً من النشر\" — Mozilla Firefox" }, 2000);
        db.close_log(old_fmt, 2100);
        let non_browser = db.insert_log(&LogEvent { event_type: "app", category: "productivity", friendly: "",
            site: "whatever", site_friendly: "whatever", series: "", episode: "",
            app: "org.gnome.Ptyxis.desktop", title: "bash" }, 1000);
        db.close_log(non_browser, 1100);

        db.backfill_sites();
        let after = db.get_timeline(0, 999_999);
        assert_eq!(after[0].site, "", "عنوان سلة يُنظَّف");
        assert_eq!(after[1].site, "X", "عنوان قديم الطراز يُعاد تحليله");
        assert_eq!(after[2].site, "whatever", "صفوف غير المتصفح لا تُلمس");
        assert_eq!(after[0].category, "other", "الفئة لا تتغير");

        db.backfill_sites();
        let twice = db.get_timeline(0, 999_999);
        assert_eq!(twice, after, "التشغيل الثاني لا يغيّر شيئاً — خامل");
    }
}
