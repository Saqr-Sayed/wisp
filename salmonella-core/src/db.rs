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

#[derive(Debug, Clone, PartialEq)]
pub struct Category {
    pub id: i64,
    pub name: String,
    pub color: String,
    pub is_builtin: i64,
    pub is_deletable: i64,
    pub sort: i64,
}

/// حمولة الإدراج في السجل.
pub struct LogEvent<'a> {
    pub event_type: &'a str,
    pub category: &'a str,
    pub media_kind: &'a str,
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
    ("media_kind", "TEXT NOT NULL DEFAULT ''"),
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
            CREATE TABLE IF NOT EXISTS categories (
                id           INTEGER PRIMARY KEY AUTOINCREMENT,
                name         TEXT NOT NULL UNIQUE,
                slug         TEXT NOT NULL DEFAULT '',
                color        TEXT NOT NULL DEFAULT '#8a7f6e',
                is_builtin   INTEGER NOT NULL DEFAULT 0,
                is_deletable INTEGER NOT NULL DEFAULT 1,
                sort         INTEGER NOT NULL DEFAULT 0
            );
            CREATE TABLE IF NOT EXISTS category_members (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                category_id INTEGER NOT NULL REFERENCES categories(id) ON DELETE CASCADE,
                kind        TEXT NOT NULL CHECK (kind IN ('app','site')),
                target      TEXT NOT NULL,
                UNIQUE (kind, target)
            );
            CREATE INDEX IF NOT EXISTS idx_member_lookup ON category_members(kind, target);
            CREATE TABLE IF NOT EXISTS ignored (
                kind TEXT NOT NULL,
                target TEXT NOT NULL,
                PRIMARY KEY (kind, target)
            );
            CREATE TABLE IF NOT EXISTS site_overrides (
                site TEXT PRIMARY KEY,
                friendly TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS series_overrides (
                pattern TEXT PRIMARY KEY,
                name TEXT NOT NULL
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
        drop(conn); // ponytail: حرّر قفل migrate قبل استدعاء دوال تفتح conn بنفسها (std Mutex غير قابل لإعادة الدخول — دون drop سيتجمّد الاختبار)
        let was_seeded = self.seed_if_empty();
        let listening_added = self.seed_missing_builtins();
        let migrated = self.migrate_custom_categories();
        if was_seeded || listening_added || migrated || self.has_stale_category_rows() {
            self.reclassify_all();
        }
        self.backfill_media_kind();
        self.backfill_series();
    }

    pub fn insert_log(&self, e: &LogEvent, t: i64) -> i64 {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO activity_logs(event_type,app_name,window_title,start_time,
                 friendly_name,site,site_friendly,category,series,episode,media_kind)
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
            params![e.event_type, e.app, e.title, t, e.friendly, e.site, e.site_friendly, e.category, e.series, e.episode, e.media_kind],
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

    /// صفوف المحتوى الخام: (bucket, series, name, seconds)
    /// name = عنوان مُنظَّف عبر classifier::clean_title
    pub fn get_content(&self, from: i64, to: i64) -> Vec<(String, String, String, i64)> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT media_kind, series, app_name, window_title, duration FROM activity_logs
             WHERE start_time>=?1 AND start_time<=?2
               AND duration IS NOT NULL AND media_kind != ''
             ORDER BY start_time").unwrap();
        stmt.query_map(params![from, to], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?,
                r.get::<_, String>(3)?, r.get::<_, i64>(4)?))
        }).unwrap()
        .filter_map(|r| r.ok())
        .map(|(kind, series, app, title, dur)| {
            (kind, series, crate::classifier::clean_title(&app, &title), dur)
        })
        .collect()
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

    pub fn get_series_overrides(&self) -> Vec<(String, String)> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT pattern, name FROM series_overrides").unwrap();
        stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?))).unwrap()
            .filter_map(|r| r.ok()).collect()
    }

    pub fn set_series_override(&self, pattern: &str, name: &str) {
        {
            let conn = self.conn.lock().unwrap();
            conn.execute(
                "INSERT INTO series_overrides(pattern,name) VALUES(?1,?2)
                 ON CONFLICT(pattern) DO UPDATE SET name=excluded.name",
                params![pattern, name],
            ).ok();
        }
        // تطبيق رجعي فوري بنمط set_site_override: LIKE غير حساس لحالة الأحرف
        // للنص اللاتيني؛ العربية بلا حالة — سقف موثّق.
        self.conn.lock().unwrap().execute(
            "UPDATE activity_logs SET series=?2 WHERE window_title LIKE '%'||?1||'%'",
            params![pattern, name],
        ).ok();
    }

    pub fn remove_series_override(&self, pattern: &str) {
        self.conn.lock().unwrap().execute("DELETE FROM series_overrides WHERE pattern=?1", params![pattern]).ok();
    }

    /// أطول نمط يطابق العنوان (بأحرف صغيرة في Rust — تطابق لاتيني دقيق الحالة،
    /// سقف موثّق) يفوز؛ التعادل يُحسم بالأسبق rowid. المطابقة على العنوان فقط.
    pub fn resolve_series(&self, app: &str, title: &str) -> Option<String> {
        let _ = app; // ponytail: المطابقة على العنوان الخام فقط كما اعتمدت المواصفة
        let title_l = title.to_lowercase();
        let rows: Vec<(String, String, i64)> = {
            let conn = self.conn.lock().unwrap();
            let mut stmt = conn.prepare("SELECT pattern, name, rowid FROM series_overrides").unwrap();
            stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).unwrap()
                .filter_map(|r| r.ok()).collect()
        };
        rows.into_iter()
            .filter(|(p, _, _)| title_l.contains(&p.to_lowercase()))
            .max_by(|a, b| a.0.len().cmp(&b.0.len()).then_with(|| b.2.cmp(&a.2)))
            .map(|(_, name, _)| name)
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

    fn has_stale_category_rows(&self) -> bool {
        // ponytail: query واحدة عند كل إقلاع؛ reclassify يشفي slug الفئات العالقة من إقلاع أول متقطع
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT 1 FROM activity_logs WHERE event_type IN ('app','media')
             AND category NOT IN (SELECT name FROM categories) LIMIT 1",
            [], |r| r.get::<_, i64>(0)).ok().is_some()
    }

    fn seed_if_empty(&self) -> bool {
        let conn = self.conn.lock().unwrap();
        let n: i64 = conn.query_row("SELECT COUNT(*) FROM categories", [], |r| r.get(0)).unwrap();
        if n > 0 { return false; }
        let seeds: &[(&str, &str, &str, i64)] = &[
            ("وسائط", "media", "#e94560", 0),
            ("قراءة", "reading", "#16a34a", 1),
            ("ألعاب", "games", "#9333ea", 2),
            ("ترفيه", "entertainment", "#f59e0b", 3),
            ("إنتاجية", "productivity", "#2563eb", 4),
            ("تصفح", "browsing", "#64748b", 5),
            ("سوشيال ميديا", "social-media", "#db2777", 6),
            ("أخرى", "other", "#8a7f6e", 7),
            ("استماع", "listening", "#0d9488", 8),
        ];
        for (name, slug, color, sort) in seeds {
            conn.execute("INSERT INTO categories(name,slug,color,is_builtin,is_deletable,sort)
                          VALUES(?1,?2,?3,1,0,?4)", params![name, slug, color, *sort as i64]).ok();
        }
        let members: &[(&str, &str, &str)] = &[
            ("media", "site", "youtube"), ("media", "site", "youtu"),
            ("media", "site", "netflix"), ("media", "site", "shahid"),
            ("media", "site", "vimeo"), ("media", "app", "mpv"),
            ("media", "app", "vlc"), ("media", "app", "celluloid"),
            ("media", "app", "totem"),
            ("reading", "app", "evince"), ("reading", "app", "okular"),
            ("reading", "app", "zathura"), ("reading", "app", "xreader"),
            ("reading", "app", "foliate"), ("reading", "app", "calibre"),
            ("games", "app", "steam"), ("games", "app", "lutris"),
            ("games", "app", "heroic"), ("games", "app", "wine"),
            ("productivity", "app", "ptyxis"), ("productivity", "app", "gnome-terminal"),
            ("productivity", "app", "konsole"), ("productivity", "app", "code"),
            ("productivity", "app", "codium"), ("productivity", "app", "cursor"),
            ("productivity", "app", "gedit"), ("productivity", "app", "libreoffice"),
            ("productivity", "app", "obsidian"),
            ("social-media", "app", "telegram"), ("social-media", "app", "whatsapp"),
            ("social-media", "app", "discord"), ("social-media", "app", "slack"),
            ("social-media", "site", "facebook"), ("social-media", "site", "instagram"),
            ("social-media", "site", "x"), ("social-media", "site", "twitter"),
            ("social-media", "site", "tiktok"), ("social-media", "site", "snapchat"),
            ("listening", "app", "spotify"), ("listening", "app", "rhythmbox"),
            ("listening", "app", "audacious"), ("listening", "app", "lollypop"),
            ("listening", "app", "strawberry"), ("listening", "app", "cmus"),
            ("listening", "app", "ncspot"),
        ];
        for (slug, kind, target) in members {
            conn.execute("INSERT INTO category_members(category_id,kind,target)
                SELECT id,?2,?3 FROM categories WHERE slug=?1", params![slug, kind, target]).ok();
        }
        true
    }

    /// إدراج "استماع" في قواعد مثبتة سابقاً (seed_if_empty يعمل على الفارغة فقط).
    /// تعيد true عند إدراجها — يُطلق عندها reclassify_all في migrate.
    fn seed_missing_builtins(&self) -> bool {
        let conn = self.conn.lock().unwrap();
        let exists: i64 = conn.query_row(
            "SELECT COUNT(*) FROM categories WHERE slug='listening'", [], |r| r.get(0)).unwrap();
        if exists > 0 { return false; }
        conn.execute(
            "INSERT INTO categories(name,slug,color,is_builtin,is_deletable,sort)
             VALUES('استماع','listening','#0d9488',1,0,8)", []).ok();
        let members: &[&str] = &["spotify", "rhythmbox", "audacious", "lollypop",
            "strawberry", "cmus", "ncspot"];
        for m in members {
            conn.execute("INSERT OR IGNORE INTO category_members(category_id,kind,target)
                SELECT id,'app',?1 FROM categories WHERE slug='listening'", params![m]).ok();
        }
        true
    }

    fn migrate_custom_categories(&self) -> bool {
        let conn = self.conn.lock().unwrap();
        let rows: Vec<(i64, String, String, String)> = conn.prepare(
            "SELECT id, kind, target, display_name FROM custom_categories").unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))).unwrap()
            .filter_map(|r| r.ok()).collect();
        let mut changed = false;
        for (row_id, kind, target, name) in rows {
            let cid: i64 = conn.query_row("SELECT id FROM categories WHERE name=?1",
                params![&name], |r| r.get(0)).unwrap_or_else(|_| {
                    conn.execute("INSERT INTO categories(name,color,is_builtin,is_deletable,sort)
                        VALUES(?1,'#8a7f6e',0,1,20)", params![&name]).ok();
                    conn.last_insert_rowid()
                });
            conn.execute("INSERT OR REPLACE INTO category_members(category_id,kind,target)
                VALUES(?1,?2,?3)", params![cid, kind, target]).ok();
            conn.execute("DELETE FROM custom_categories WHERE id=?1", params![row_id]).ok();
            changed = true;
        }
        changed
    }

    pub fn categories(&self) -> Vec<Category> {
        self.conn.lock().unwrap()
            .prepare("SELECT id, name, color, is_builtin, is_deletable, sort
                      FROM categories ORDER BY sort, id").unwrap()
            .query_map([], |r| Ok(Category {
                id: r.get(0)?, name: r.get(1)?, color: r.get(2)?,
                is_builtin: r.get(3)?, is_deletable: r.get(4)?, sort: r.get(5)? })).unwrap()
            .filter_map(|r| r.ok()).collect()
    }

    pub fn category_members(&self, id: i64) -> Vec<(String, String)> {
        self.conn.lock().unwrap()
            .prepare("SELECT kind, target FROM category_members WHERE category_id=?1
                      ORDER BY kind, target").unwrap()
            .query_map([id], |r| Ok((r.get(0)?, r.get(1)?))).unwrap()
            .filter_map(|r| r.ok()).collect()
    }

    /// أولوية: member app → member site → slug → "أخرى".
    pub fn resolve_category(&self, app: &str, site: &str, slug: &str) -> String {
        let conn = self.conn.lock().unwrap();
        let app_l = app.to_lowercase();
        let app_hit: Option<String> = conn.prepare(
            "SELECT c.name FROM category_members m JOIN categories c ON c.id=m.category_id
             WHERE m.kind='app' AND ?1 <> '' AND INSTR(?1, LOWER(m.target)) > 0
             ORDER BY LENGTH(m.target) DESC LIMIT 1").unwrap()
            .query_row(params![&app_l], |r| r.get(0)).ok();
        if let Some(n) = app_hit { return n; }
        if !site.trim().is_empty() {
            let s_l = site.trim().to_lowercase();
            let slug_hit: Option<String> = conn.prepare(
                "SELECT c.name FROM category_members m JOIN categories c ON c.id=m.category_id
                 WHERE m.kind='site' AND LOWER(m.target) = ?1 LIMIT 1").unwrap()
                .query_map(params![&s_l], |r| r.get(0)).unwrap()
                .find_map(|r| r.ok());
            if let Some(n) = slug_hit { return n; }
        }
        conn.query_row("SELECT name FROM categories WHERE slug=?1 LIMIT 1",
            params![slug], |r| r.get(0)).unwrap_or_else(|_|
            conn.query_row("SELECT name FROM categories WHERE slug='other'", [],
                |r| r.get(0)).unwrap_or_else(|_| "أخرى".to_string()))
    }

    pub fn add_category(&self, name: &str, color: &str) -> i64 {
        let name = name.trim();
        if name.is_empty() { return 0; }
        let conn = self.conn.lock().unwrap();
        let taken: Option<i64> = conn.query_row(
            "SELECT 1 FROM categories WHERE name=?1", params![name], |r| r.get(0)).ok();
        if taken.is_some() { return 0; }
        if conn.execute("INSERT INTO categories(name,color,is_builtin,is_deletable,sort)
            VALUES(?1,?2,0,1,20)", params![name, color]).is_err() { return 0; }
        conn.last_insert_rowid()
    }

    pub fn rename_category(&self, id: i64, new_name: &str) {
        let conn = self.conn.lock().unwrap();
        let old: String = conn.query_row("SELECT name FROM categories WHERE id=?1",
            params![id], |r| r.get(0)).unwrap_or_default();
        if old.is_empty() || old == new_name { return; }
        let clash: Option<i64> = conn.query_row(
            "SELECT 1 FROM categories WHERE name=?1 AND id<>?2", params![new_name, id], |r| r.get(0)).ok();
        if clash.is_some() { return; }
        if conn.execute("UPDATE categories SET name=?1 WHERE id=?2 AND name=?3",
                params![new_name, id, old]).unwrap_or(0) != 1 { return; }
        conn.execute("UPDATE activity_logs SET category=?1 WHERE category=?2", params![new_name, old]).ok();
    }

    pub fn set_category_color(&self, id: i64, color: &str) {
        self.conn.lock().unwrap().execute(
            "UPDATE categories SET color=?1 WHERE id=?2", params![color, id]).ok();
    }

    pub fn add_category_member(&self, id: i64, kind: &str, target: &str) {
        let t = target.trim().to_lowercase();
        if t.is_empty() { return; }
        {
            let conn = self.conn.lock().unwrap();
            conn.execute("INSERT OR IGNORE INTO category_members(category_id,kind,target)
                VALUES(?1,?2,?3)", params![id, kind, t]).ok();
        }
        self.reclassify_all();
    }

    pub fn delete_category_member(&self, kind: &str, target: &str) {
        let t = target.to_lowercase();
        {
            let conn = self.conn.lock().unwrap();
            conn.execute("DELETE FROM category_members WHERE kind=?1 AND target=?2", params![kind, t]).ok();
        }
        self.reclassify_all();
    }

    pub fn delete_category(&self, id: i64) -> bool {
        let conn = self.conn.lock().unwrap();
        let deletable: Option<i64> = conn.query_row(
            "SELECT is_deletable FROM categories WHERE id=?1", params![id], |r| r.get(0)).ok();
        if deletable != Some(1) { return false; }
        let old: String = conn.query_row("SELECT name FROM categories WHERE id=?1",
            params![id], |r| r.get(0)).unwrap_or_default();
        let other: String = conn.query_row("SELECT name FROM categories WHERE slug='other'", [],
            |r| r.get(0)).unwrap_or_else(|_| "أخرى".to_string());
        conn.execute("UPDATE activity_logs SET category=?1 WHERE category=?2", params![&other, &old]).ok();
        conn.execute("DELETE FROM categories WHERE id=?1", params![id]).ok();
        true
    }

    pub fn reclassify_all(&self) {
        let rows: Vec<(i64, String, String)> = {
            // ponytail: conn.prepare ضمن التعبير الأخير لا يترجم — الـ Statement يبقى
            // مقترضاً من الحارس حتى نهاية الكتلة؛ الإنشاء في جملة مستقلة كـ backfill_sites.
            let conn = self.conn.lock().unwrap();
            let mut stmt = conn.prepare("SELECT id, app_name, window_title FROM activity_logs
                WHERE event_type IN ('app','media')").unwrap();
            stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).unwrap()
                .filter_map(|r| r.ok()).collect()
        };
        let mapped: Vec<(i64, String)> = rows.iter().map(|(id, app, title)| {
            let e = crate::classifier::enrich(app, title);
            (*id, self.resolve_category(app, &e.site, e.category))
        }).collect();
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction().unwrap();
        for (id, name) in mapped {
            tx.execute("UPDATE activity_logs SET category=?1 WHERE id=?2", params![name, id]).ok();
        }
        tx.commit().ok();
    }

    /// تعبئة media_kind للصفوف القديمة من المصنف — تمريرة خاملة بمعاملة
    /// واحدة (بنمط reclassify_all)؛ تتقلص لصفر عمل بعد أول إقلاع.
    fn backfill_media_kind(&self) {
        let rows: Vec<(i64, String, String)> = {
            let conn = self.conn.lock().unwrap();
            let mut stmt = conn.prepare("SELECT id, app_name, window_title FROM activity_logs
                WHERE media_kind = '' AND event_type IN ('app','media')").unwrap();
            stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).unwrap()
                .filter_map(|r| r.ok()).collect()
        };
        if rows.is_empty() { return; }
        let mapped: Vec<(i64, String)> = rows.iter().map(|(id, app, title)| {
            let e = crate::classifier::enrich(app, title);
            (*id, e.media_kind.to_string())
        }).collect();
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction().unwrap();
        for (id, kind) in mapped {
            tx.execute("UPDATE activity_logs SET media_kind=?1 WHERE id=?2", params![kind, id]).ok();
        }
        tx.commit().ok();
    }

    /// اشتقاق series/episode لصفوف الوسائط القديمة الفارغة: تجاوز إن وُجد وإلا
    /// enrich (episode يُعاد اشتقاقه مع السلسلة — كلاهما فارغٌ تاريخياً).
    /// تمريرة خاملة بمعاملة واحدة (بنمط backfill_media_kind)؛ حارس series=''
    /// لا يمسّ الصفوف ذات السلسلة الجيدة؛ سياق المجلد لا يُسترد تاريخياً.
    fn backfill_series(&self) {
        let rows: Vec<(i64, String, String, String)> = {
            let conn = self.conn.lock().unwrap();
            let mut stmt = conn.prepare("SELECT id, app_name, window_title, episode FROM activity_logs
                WHERE series = '' AND media_kind IN ('reading','watching','listening')").unwrap();
            stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))).unwrap()
                .filter_map(|r| r.ok()).collect()
        };
        if rows.is_empty() { return; }
        let mapped: Vec<(i64, String, String)> = rows.iter().map(|(id, app, title, old_episode)| {
            let e = crate::classifier::enrich(app, title);
            let series = self.resolve_series(app, title).unwrap_or(e.series);
            // إن بقي series فارغاً (بلا نمط ولا enrich) نُبقي episode القديم كما هو
            // (حارس: لا نُفسد episode لوحده من صفوف شبه-مصنفة).
            let episode = if series.is_empty() { old_episode.clone() } else { e.episode };
            (*id, series, episode)
        }).collect();
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction().unwrap();
        for (id, series, episode) in mapped {
            tx.execute("UPDATE activity_logs SET series=?1, episode=?2 WHERE id=?3",
                params![series, episode, id]).ok();
        }
        tx.commit().ok();
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
        db.insert_log(&LogEvent { event_type: "app", category: "productivity", media_kind: "", friendly: "الطرفية",
            site: "", site_friendly: "", series: "", episode: "", app: "org.gnome.Ptyxis.desktop", title: "bash" }, 1000);
        let rows = db.get_timeline(0, 2000);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].friendly_name, "الطرفية");
        assert_eq!(rows[0].category, "productivity");
    }

    #[test]
    fn insert_log_stores_media_kind() {
        let db = tmp_db("mediakind");
        let id = db.insert_log(&LogEvent { event_type: "media", category: "listening", media_kind: "listening",
            friendly: "", site: "", site_friendly: "", series: "", episode: "",
            app: "spotify.desktop", title: "أغنية - spotify" }, 1000);
        let kind: String = db.conn.lock().unwrap().query_row(
            "SELECT media_kind FROM activity_logs WHERE id=?1", params![id], |r| r.get(0)).unwrap();
        assert_eq!(kind, "listening");
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
            let id = db.insert_log(&LogEvent { event_type: "app", category: "other", media_kind: "", friendly,
                site: "", site_friendly: "", series: "", episode: "", app, title: "t" }, now);
            db.close_log(id, now + dur_sec);
        }
        let r = db.get_report(0, 999_999, "app");
        assert_eq!(r, vec![("تطبيق أ".to_string(), 90), ("تطبيق ب".to_string(), 10)]);
    }

    #[test] fn report_groups_by_category_and_series() {
        let db = tmp_db("report2");
        let id = db.insert_log(&LogEvent { event_type: "media", category: "media", media_kind: "", friendly: "مشغل",
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
        let id = db.insert_log(&LogEvent { event_type: "app", category: "other", media_kind: "", friendly: "",
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
        let id1 = db1.insert_log(&LogEvent { event_type: "app", category: "other", media_kind: "", friendly: "",
            site: "", site_friendly: "", series: "", episode: "", app: "x", title: "y" }, 1000);
        db1.set_setting("last_alive", "5000");
        db1.close_dangling(10000);
        let (end, dur): (i64, i64) = db1.conn.lock().unwrap().query_row(
            "SELECT end_time, duration FROM activity_logs WHERE id=?1", params![id1],
            |r| Ok((r.get(0)?, r.get(1)?))).unwrap();
        assert_eq!((end, dur), (5000, 4000));

        // fresh last_alive (≤60s ago) → close at now
        let db2 = tmp_db("dangling-la2");
        let id2 = db2.insert_log(&LogEvent { event_type: "app", category: "other", media_kind: "", friendly: "",
            site: "", site_friendly: "", series: "", episode: "", app: "x", title: "y" }, 1000);
        db2.set_setting("last_alive", "9950");
        db2.close_dangling(10000);
        let (end, dur): (i64, i64) = db2.conn.lock().unwrap().query_row(
            "SELECT end_time, duration FROM activity_logs WHERE id=?1", params![id2],
            |r| Ok((r.get(0)?, r.get(1)?))).unwrap();
        assert_eq!((end, dur), (10000, 9000));

        // no last_alive → close at now
        let db3 = tmp_db("dangling-la3");
        let id3 = db3.insert_log(&LogEvent { event_type: "app", category: "other", media_kind: "", friendly: "",
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

    #[test]
    fn seed_creates_nine_categories() {
        let db = tmp_db("cat-seed");
        let cats = db.categories();
        assert_eq!(cats.len(), 9);
        for n in ["سوشيال ميديا", "ترفيه", "وسائط", "أخرى", "استماع"] {
            assert!(cats.iter().any(|c| c.name == n), "missing {n}");
        }
        let social = cats.iter().find(|c| c.name == "سوشيال ميديا").unwrap().clone();
        let members = db.category_members(social.id);
        assert!(members.iter().any(|(k, t)| k == "site" && t == "facebook"));
        assert!(members.iter().any(|(k, t)| k == "app" && t == "telegram"));
        let media = cats.iter().find(|c| c.name == "وسائط").unwrap();
        assert_eq!(media.is_deletable, 0);
        let listening = cats.iter().find(|c| c.name == "استماع").unwrap().clone();
        assert_eq!(listening.color, "#0d9488");
        assert_eq!(listening.is_deletable, 0);
        let lm = db.category_members(listening.id);
        assert!(lm.iter().any(|(k, t)| k == "app" && t == "spotify"));
        assert!(lm.iter().any(|(k, t)| k == "app" && t == "cmus"));
    }

    #[test]
    fn seed_missing_builtins_adds_listening_to_installed_db() {
        // قاعدة مثبتة قبل هذا التحديث: حذف "استماع" وأعضاءها يدوياً ثم إعادة الفتح
        let path = std::env::temp_dir().join(format!("salmonella-{}-seedmiss.db", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let d1 = Db::open(&path);
        {
            let conn = d1.conn.lock().unwrap();
            conn.execute("DELETE FROM category_members WHERE category_id IN
                (SELECT id FROM categories WHERE slug='listening')", []).ok();
            conn.execute("DELETE FROM categories WHERE slug='listening'", []).ok();
        }
        drop(d1);
        let db = Db::open(&path);
        let cats = db.categories();
        assert_eq!(cats.len(), 9, "استماع تُعاد زراعتها عند الفتح");
        let listening = cats.iter().find(|c| c.name == "استماع").unwrap().clone();
        let lm = db.category_members(listening.id);
        assert!(lm.iter().any(|(k, t)| k == "app" && t == "spotify"));
        drop(db);
        let db2 = Db::open(&path);
        assert_eq!(db2.categories().len(), 9, "لا تكرار في الفتح الثاني");
        assert_eq!(db2.resolve_category("spotify", "", "listening"), "استماع");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn backfill_media_kind_fills_old_rows() {
        // قاعدة بلا عمود media_kind — Db::open يضيف العمود ويملأه من المصنف
        let path = std::env::temp_dir().join(format!("salmonella-{}-mediabf.db", std::process::id()));
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
            INSERT INTO activity_logs(event_type, app_name, window_title, start_time, end_time, duration)
                VALUES('app','org.gnome.Evince.desktop','الرياضيات.pdf - Evince',1000,1100,100);
            INSERT INTO activity_logs(event_type, app_name, window_title, start_time, end_time, duration)
                VALUES('media','mpv.desktop','movie.mp4 - mpv',2000,2100,100);
            INSERT INTO activity_logs(event_type, app_name, window_title, start_time, end_time, duration)
                VALUES('app','spotify.desktop','أغنية - spotify',3000,3100,100);
            INSERT INTO activity_logs(event_type, app_name, window_title, start_time, end_time, duration)
                VALUES('system','','__boot__',4000,4000,0);"
        ).unwrap();
        drop(old);
        let db = Db::open(&path);
        let content = db.get_content(0, 999_999);
        assert_eq!(content.len(), 3, "النظامي (media_kind='') لا يظهر");
        assert_eq!(content[0],
            ("reading".to_string(), String::new(), "الرياضيات.pdf - Evince".to_string(), 100));
        assert_eq!(content[1],
            ("watching".to_string(), String::new(), "movie.mp4".to_string(), 100));
        assert_eq!(content[2],
            ("listening".to_string(), String::new(), "أغنية".to_string(), 100));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn get_content_buckets_names_and_cleans_titles() {
        let db = tmp_db("content");
        let r1 = db.insert_log(&LogEvent { event_type: "app", category: "قراءة", media_kind: "reading",
            friendly: "", site: "", site_friendly: "", series: "", episode: "",
            app: "org.gnome.Evince.desktop", title: "الرياضيات.pdf - Evince" }, 1000);
        db.close_log(r1, 1100);
        let w1 = db.insert_log(&LogEvent { event_type: "media", category: "وسائط", media_kind: "watching",
            friendly: "", site: "", site_friendly: "", series: "", episode: "",
            app: "mpv.desktop", title: "movie.mp4 - mpv" }, 2000);
        db.close_log(w1, 2100);
        let s1 = db.insert_log(&LogEvent { event_type: "media", category: "وسائط", media_kind: "watching",
            friendly: "", site: "", site_friendly: "", series: "SpongeBob", episode: "1x3",
            app: "mpv.desktop", title: "SpongeBob S01E03 - mpv" }, 3000);
        db.close_log(s1, 3100);
        let s2 = db.insert_log(&LogEvent { event_type: "media", category: "وسائط", media_kind: "watching",
            friendly: "", site: "", site_friendly: "", series: "SpongeBob", episode: "1x4",
            app: "mpv.desktop", title: "SpongeBob S01E04 - mpv" }, 4000);
        db.close_log(s2, 4100);
        let l1 = db.insert_log(&LogEvent { event_type: "media", category: "استماع", media_kind: "listening",
            friendly: "", site: "", site_friendly: "", series: "", episode: "",
            app: "spotify.desktop", title: "أغنية - spotify" }, 5000);
        db.close_log(l1, 5100);

        let content = db.get_content(0, 999_999);
        assert_eq!(content, vec![
            ("reading".to_string(), String::new(), "الرياضيات.pdf - Evince".to_string(), 100),
            ("watching".to_string(), String::new(), "movie.mp4".to_string(), 100),
            ("watching".to_string(), "SpongeBob".to_string(), "SpongeBob S01E03".to_string(), 100),
            ("watching".to_string(), "SpongeBob".to_string(), "SpongeBob S01E04".to_string(), 100),
            ("listening".to_string(), String::new(), "أغنية".to_string(), 100),
        ]);
    }

    #[test]
    fn get_content_excludes_open_and_unclassified_rows() {
        let db = tmp_db("content-x");
        // صف مفتوح (duration NULL — لم يُغلق)
        db.insert_log(&LogEvent { event_type: "media", category: "وسائط", media_kind: "watching",
            friendly: "", site: "", site_friendly: "", series: "", episode: "",
            app: "mpv.desktop", title: "movie.mp4 - mpv" }, 1000);
        // تطبيق عادي media_kind=''
        db.insert_log(&LogEvent { event_type: "app", category: "إنتاجية", media_kind: "",
            friendly: "", site: "", site_friendly: "", series: "", episode: "",
            app: "code.desktop", title: "main.rs" }, 2000);
        // صف نظامي media_kind=''
        db.insert_system_event("boot", "", 3000, Some(3000));
        assert!(db.get_content(0, 999_999).is_empty());
    }

    #[test]
    fn member_match_wins_over_slug() {
        let db = tmp_db("cat-wins");
        let dev = db.add_category("تطوير", "#ff0000");
        db.add_category_member(dev, "app", "code.desktop");
        assert_eq!(db.resolve_category("code.desktop", "", "productivity"), "تطوير");
        assert_eq!(db.resolve_category("org.mozilla.firefox.desktop", "facebook", "browsing"),
                   "سوشيال ميديا");
    }

    #[test]
    fn resolve_falls_back_to_slug_then_other() {
        let db = tmp_db("cat-fallback");
        assert_eq!(db.resolve_category("games", "vlc", "media"), "وسائط"); // site "vlc" لا يوجد → slug media
        assert_eq!(db.resolve_category("evince", "", "reading"), "قراءة");
        assert_eq!(db.resolve_category("whatever", "", "nonsense"), "أخرى");
        assert_eq!(db.resolve_category("someaudio", "", "listening"), "استماع", "سقوط slug");
        assert_eq!(db.resolve_category("spotify", "", "listening"), "استماع", "عضو الصوت");
    }

    #[test]
    fn rename_updates_history() {
        let db = tmp_db("cat-ren");
        db.insert_log(&LogEvent { event_type: "media", category: "وسائط", media_kind: "", friendly: "",
            site: "", site_friendly: "", series: "", episode: "", app: "mpv", title: "x.mp4" }, 1000);
        let media = db.categories().iter().find(|c| c.name == "وسائط").unwrap().clone();
        db.rename_category(media.id, "فيديو");
        assert_eq!(db.get_timeline(0, 2000)[0].category, "فيديو");
        assert_eq!(db.resolve_category("mpv", "out", "media"), "فيديو");
    }

    #[test]
    fn rename_to_existing_name_keeps_both_and_history() {
        let db = tmp_db("cat-ren-clash");
        db.insert_log(&LogEvent { event_type: "media", category: "وسائط", media_kind: "", friendly: "",
            site: "", site_friendly: "", series: "", episode: "", app: "vlc", title: "x.mp4" }, 1000);
        let media = db.categories().iter().find(|c| c.name == "وسائط").unwrap().clone();
        db.rename_category(media.id, "أخرى");
        assert!(db.categories().iter().any(|c| c.id == media.id && c.name == "وسائط"),
            "وسائط يُبقي اسمه");
        assert_eq!(db.categories().iter().filter(|c| c.name == "أخرى").count(), 1,
            "لا اسم مكرر");
        assert_eq!(db.get_timeline(0, 2000)[0].category, "وسائط",
            "تاريخ وسائط غير مُدمج في أخرى");
    }

    #[test]
    fn add_category_duplicate_name_returns_zero() {
        let db = tmp_db("add-cat-dupe");
        assert_eq!(db.add_category("أخرى", "#000"), 0);
        assert_eq!(db.add_category("", "#000"), 0);
        assert_eq!(db.categories().len(), 9, "لا صف جديد يُضاف");
    }

    #[test]
    fn reclassify_rebinds_after_member_change() {
        // ponytail: زمن neovim أعلى من steam لأن get_timeline يرجع تنازلياً — البريف
        // خلط الترتيب فكان التوقع معكوساً على idx0.
        let db = tmp_db("cat-reclass");
        db.insert_log(&LogEvent { event_type: "app", category: "productivity", media_kind: "", friendly: "",
            site: "", site_friendly: "", series: "", episode: "", app: "steam", title: "CS" }, 1000);
        db.insert_log(&LogEvent { event_type: "app", category: "games", media_kind: "", friendly: "",
            site: "", site_friendly: "", series: "", episode: "", app: "neovim", title: "x.rs" }, 2000);
        db.reclassify_all();
        assert_eq!(db.get_timeline(0, 3000)[0].category, "إنتاجية");
        let reading = db.categories().iter().find(|c| c.name == "قراءة").unwrap().clone();
        db.add_category_member(reading.id, "app", "neovim");
        let rows = db.get_timeline(0, 3000);
        assert_eq!(rows[0].category, "قراءة", "إعادة التصنيف تغيّره");
        assert_eq!(rows[1].category, "ألعاب", "الآخر ثابت");
    }

    #[test]
    fn delete_member_reclassifies_rows() {
        let db = tmp_db("cat-del-mem");
        db.insert_log(&LogEvent { event_type: "app", category: "games", media_kind: "", friendly: "",
            site: "", site_friendly: "", series: "", episode: "", app: "minegames", title: "x" }, 1000);
        let reading = db.categories().iter().find(|c| c.name == "قراءة").unwrap().clone();
        db.add_category_member(reading.id, "app", "minegames");
        assert_eq!(db.get_timeline(0, 2000)[0].category, "قراءة");
        db.delete_category_member("app", "minegames");
        assert_eq!(db.get_timeline(0, 2000)[0].category, "ألعاب", "حذف العضو يعيد التصنيف");
    }

    #[test]
    fn delete_extra_category_maps_rows_to_other() {
        let db = tmp_db("cat-del");
        let id = db.add_category("مخصصة", "#00ff00");
        db.add_category_member(id, "app", "minegames");
        let ok = db.delete_category(id);
        assert!(ok);
        assert!(!db.categories().iter().any(|c| c.id == id));
        // المدمج لا يُحذف
        let media = db.categories().iter().find(|c| c.name == "وسائط").unwrap().clone();
        assert!(!db.delete_category(media.id));
    }

    #[test]
    fn custom_categories_migrate_into_categories() {
        // الزرعة والترحيل يجريان في migrate() عند فتح DB — نعيد الفتح على المسار نفسه
        let path = std::env::temp_dir().join("salmonella-cat-mig2.db");
        let _ = std::fs::remove_file(&path);
        let d1 = Db::open(&path);
        d1.add_custom_category("app", "krita", "رسم");
        drop(d1);
        let db = Db::open(&path);
        assert!(db.categories().iter().any(|c| c.name == "رسم"));
        assert_eq!(db.resolve_category("krita", "", ""), "رسم");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn boot_heals_stale_slug_categories() {
        let path = std::env::temp_dir().join(format!("salmonella-stale-{}.db", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let d1 = Db::open(&path);
        d1.insert_log(&LogEvent { event_type: "app", category: "bogus", media_kind: "", friendly: "فايرفوكس",
            site: "", site_friendly: "", series: "", episode: "",
            app: "org.mozilla.firefox.desktop", title: "t" }, 1000);
        drop(d1);
        let db = Db::open(&path);
        let row = &db.get_timeline(0, 9999)[0];
        assert!(db.categories().iter().all(|c| c.name != "bogus"), "bogus ليست فئة حقيقية");
        assert!(db.categories().iter().any(|c| c.name == row.category),
            "الصف العالق أُعيد تصنيفه لفئة حقيقية عند الإقلاع");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn custom_rule_replaces_seed_member_on_migrate() {
        let path = std::env::temp_dir().join(format!("salmonella-custseed-{}.db", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let d1 = Db::open(&path);
        d1.add_custom_category("app", "telegram", "محادثة");
        drop(d1);
        let db = Db::open(&path);
        assert_eq!(db.resolve_category("telegram", "", "social-media"), "محادثة",
            "قاعدة المستخدم تحلّ محل عضو الزرعة ولا تُهمل");
        let social = db.categories().iter().find(|c| c.name == "سوشيال ميديا").unwrap().clone();
        let members = db.category_members(social.id);
        assert!(members.iter().any(|(k, t)| k == "site" && t == "facebook"),
            "بقية أعضاء الزرعة باقية");
        let _ = std::fs::remove_file(&path);
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
        let e = LogEvent { event_type: "app", category: "browsing", media_kind: "", friendly: "فايرفوكس",
            site: "", site_friendly: "", series: "", episode: "", app: "org.mozilla.firefox.desktop", title: "t" };
        let id = db.insert_log(&e, 1000);
        db.close_log(id, 1100);
        assert_eq!(db.get_timeline(0, 9999)[0].friendly_name, "فايرفوكس");
        db.set_name_override("org.mozilla.firefox.desktop", "متصفحي");
        assert_eq!(db.get_timeline(0, 9999)[0].friendly_name, "متصفحي", "الصفوف القديمة تتحدث");
    }

    #[test] fn rename_site_is_retroactive_and_groups() {
        let db = tmp_db("retrosite");
        let e = LogEvent { event_type: "app", category: "media", media_kind: "", friendly: "فايرفوكس",
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
        let e = LogEvent { event_type: "app", category: "browsing", media_kind: "", friendly: "فايرفوكس",
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
        let e = LogEvent { event_type: "app", category: "browsing", media_kind: "", friendly: "فايرفوكس",
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
            let id = db.insert_log(&LogEvent { event_type: "app", category: "browsing", media_kind: "", friendly: "فايرفوكس",
                site: s, site_friendly: s, series: "", episode: "",
                app: "org.mozilla.firefox.desktop", title: "t" }, 1000 + i as i64);
            db.close_log(id, 1100 + i as i64);
        }
        let known = db.get_known_sites();
        assert_eq!(known.len(), 2, "X و x يُدمجان، و X / Home تبقى");
    }

    #[test] fn backfill_reparses_browser_rows_and_is_idempotent() {
        let db = tmp_db("backfill");
        let junk = db.insert_log(&LogEvent { event_type: "app", category: "other", media_kind: "", friendly: "فايرفوكس",
            site: "Calculator", site_friendly: "Calculator", series: "", episode: "",
            app: "org.mozilla.firefox.desktop", title: "Calculator — Mozilla Firefox" }, 3000);
        db.close_log(junk, 3100);
        let old_fmt = db.insert_log(
            &LogEvent { event_type: "app", category: "other", media_kind: "", friendly: "فايرفوكس",
                site: "X \\ DeepSeek على X: \"طويل جداً جداً من النشر\"",
                site_friendly: "X \\ DeepSeek على X: \"طويل جداً جداً من النشر\"",
                series: "", episode: "",
                app: "org.mozilla.firefox.desktop",
                title: "X \\ DeepSeek على X: \"طويل جداً جداً من النشر\" — Mozilla Firefox" }, 2000);
        db.close_log(old_fmt, 2100);
        let non_browser = db.insert_log(&LogEvent { event_type: "app", category: "productivity", media_kind: "", friendly: "",
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

    #[test] fn series_overrides_crud() {
        let db = tmp_db("seriesov");
        assert!(db.get_series_overrides().is_empty());
        db.set_series_override("الدرس", "تفسير آية الكرسي");
        assert_eq!(db.get_series_overrides(),
            vec![("الدرس".to_string(), "تفسير آية الكرسي".to_string())]);
        db.set_series_override("الدرس", "سلسلة أخرى");
        assert_eq!(db.get_series_overrides(),
            vec![("الدرس".to_string(), "سلسلة أخرى".to_string())], "التحديث يغيّر الاسم");
        db.remove_series_override("الدرس");
        assert!(db.get_series_overrides().is_empty());
    }

    #[test]
    fn resolve_series_partial_match_case_insensitive() {
        let db = tmp_db("seriesres2");
        db.set_series_override("movie", "أفلامي");
        assert_eq!(db.resolve_series("mpv.desktop", "Movie.mp4 - mpv").unwrap(), "أفلامي",
            "تطابق جزئي بحرف صغير على العنوان الخام");
        assert!(db.resolve_series("mpv.desktop", "الدرس 2 - mpv").is_none());
    }

    #[test]
    fn resolve_series_longest_pattern_wins() {
        let db = tmp_db("seriesres");
        db.set_series_override("الدرس", "تفسير آية الكرسي");
        db.set_series_override("الدرس 12", "درس خاص");
        assert_eq!(db.resolve_series("mpv.desktop", "الدرس 12 - mpv").unwrap(), "درس خاص", "الأطول يفوز");
        assert_eq!(db.resolve_series("mpv.desktop", "الدرس 11 - mpv").unwrap(), "تفسير آية الكرسي");
    }

    #[test]
    fn set_series_override_is_retroactive() {
        let db = tmp_db("seriesretro");
        let empty = LogEvent { event_type: "media", category: "وسائط", media_kind: "watching", friendly: "",
            site: "", site_friendly: "", series: "", episode: "", app: "mpv.desktop", title: "الدرس 2 - mpv" };
        let id1 = db.insert_log(&empty, 1000);
        db.close_log(id1, 1100);
        let weak = LogEvent { event_type: "media", category: "وسائط", media_kind: "watching", friendly: "",
            site: "", site_friendly: "", series: "الدرس", episode: "3",
            app: "mpv.desktop", title: "الدرس 3 - mpv" };
        let id2 = db.insert_log(&weak, 2000);
        db.close_log(id2, 2100);
        db.set_series_override("الدرس", "تفسير آية الكرسي");
        let rows = db.get_timeline(0, 9999);
        assert_eq!(rows[0].series, "تفسير آية الكرسي", "الصف الضعيف المخزَّن يتجاوز أيضاً (فجوة الحارس)");
        assert_eq!(rows[1].series, "تفسير آية الكرسي", "الصف الفارغ يتحدث فوراً");
    }

    #[test]
    fn backfill_series_fills_old_rows() {
        // قاعدة بلا عمودي series/media_kind — Db::open يضيفهما: backfill_media_kind
        // يملأ media_kind ثم backfill_series يملأ series/episode من enrich + التجاوزات.
        let path = std::env::temp_dir().join(format!("salmonella-{}-seriesbf.db", std::process::id()));
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
                duration INTEGER,
                series TEXT DEFAULT '',
                episode TEXT DEFAULT ''
            );
            CREATE TABLE series_overrides (
                pattern TEXT PRIMARY KEY,
                name TEXT NOT NULL
            );
            INSERT INTO series_overrides(pattern,name) VALUES('الدرس 12','تفسير آية الكرسي');
            INSERT INTO activity_logs(event_type, app_name, window_title, start_time, end_time, duration, series)
                VALUES('media','mpv.desktop','الدرس 11 - mpv',1000,1100,100,'');
            INSERT INTO activity_logs(event_type, app_name, window_title, start_time, end_time, duration, series)
                VALUES('media','mpv.desktop','الدرس 12 - mpv',2000,2100,100,'');
            INSERT INTO activity_logs(event_type, app_name, window_title, start_time, end_time, duration, series)
                VALUES('media','mpv.desktop','SpongeBob S01E03 - mpv',3000,3100,100,'SpongeBob');"
        ).unwrap();
        drop(old);
        let db = Db::open(&path);
        let rows = db.get_timeline(0, 999_999);
        // الترتيب تنازلي: SpongeBob (start 3000) ثم الدرس 12 ثم الدرس 11
        assert_eq!(rows[0].series, "SpongeBob", "الصف ذو السلسلة الجيدة لا يُلمس — حارس series=''");
        assert_eq!(rows[1].series, "تفسير آية الكرسي", "التجاوز يربح على enrich");
        assert_eq!(rows[1].episode, "12", "الحلقة تُعاد اشتقاقها مع السلسلة");
        assert_eq!(rows[2].series, "الدرس");
        assert_eq!(rows[2].episode, "11");
        db.backfill_series();
        let twice = db.get_timeline(0, 999_999);
        assert_eq!(twice, rows, "التشغيل الثاني خامل — حارس series=''");
        let _ = std::fs::remove_file(&path);
    }
}
