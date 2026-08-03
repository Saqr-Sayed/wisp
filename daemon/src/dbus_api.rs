use std::sync::{Arc, Mutex};
use salmonella_core::db::{Db, LogEntry};
use zbus::{interface, object_server::SignalContext, ConnectionBuilder};

type Row = (i64, String, String, String, i64, i64, i64, String, String, String, String, String, String);

fn to_row(e: &LogEntry) -> Row {
    (e.id, e.event_type.clone(), e.app_name.clone(), e.window_title.clone(),
     e.start_time, e.end_time.unwrap_or(-1), e.duration.unwrap_or(-1),
     e.friendly_name.clone(), e.site.clone(), e.category.clone(),
     e.series.clone(), e.episode.clone(), e.detail.clone())
}

#[derive(Clone)]
pub struct ActivityTracker {
    db: Arc<Db>,
    conn: Arc<Mutex<Option<zbus::Connection>>>,
}

impl ActivityTracker {
    pub fn new(db: Arc<Db>) -> Self {
        ActivityTracker { db, conn: Arc::new(Mutex::new(None)) }
    }

    pub fn set_connection(&self, conn: zbus::Connection) {
        *self.conn.lock().unwrap() = Some(conn);
    }

    pub async fn emit_window_changed(&self, app_name: &str, window_title: &str, since: i64) -> zbus::Result<()> {
        let Some(conn) = self.conn.lock().unwrap().clone() else { return Ok(()) };
        let ctxt = SignalContext::new(&conn, "/com/Saqr/Salomnella")?;
        self.window_changed(&ctxt, app_name, window_title, since).await
    }
}

#[interface(name = "com.Saqr.Salomnella")]
impl ActivityTracker {
    async fn ping(&self) -> bool { true }

    async fn get_status(&self) -> (bool, i64, String, String) {
        let (since, app, title) = self.db.get_status();
        (since > 0, since, app, title)
    }

    async fn get_timeline(&self, from: i64, to: i64) -> Vec<Row> {
        self.db.get_timeline(from, to).iter().map(to_row).collect()
    }

    async fn search(&self, query: &str) -> Vec<Row> {
        self.db.search(query).iter().map(to_row).collect()
    }

    async fn get_report(&self, from: i64, to: i64, group_by: &str) -> Vec<(String, i64)> {
        self.db.get_report(from, to, group_by)
    }

    async fn get_series(&self, from: i64, to: i64) -> Vec<(String, String, i64)> {
        self.db.get_series(from, to)
    }

    async fn get_content(&self, from: i64, to: i64) -> Vec<(String, String, String, i64)> {
        self.db.get_content(from, to)
    }

    async fn get_limits(&self) -> Vec<(String, String, i64)> {
        self.db.get_limits()
    }

    async fn set_limit(&self, target: &str, kind: &str, minutes: i64) {
        self.db.set_limit(target, kind, minutes);
    }

    async fn remove_limit(&self, target: &str) {
        self.db.remove_limit(target);
    }

    async fn get_name_overrides(&self) -> Vec<(String, String)> {
        self.db.get_name_overrides()
    }

    async fn set_name_override(&self, app_id: &str, friendly: &str) {
        self.db.set_name_override(app_id, friendly);
    }

    async fn remove_name_override(&self, app_id: &str) {
        self.db.remove_name_override(app_id);
    }

    async fn get_known_apps(&self) -> Vec<(String, String)> {
        self.db.get_known_apps()
    }

    async fn get_known_sites(&self) -> Vec<(String, String)> {
        self.db.get_known_sites()
    }

    async fn get_site_overrides(&self) -> Vec<(String, String)> {
        self.db.get_site_overrides()
    }

    async fn set_site_override(&self, site: String, friendly: String) {
        self.db.set_site_override(&site, &friendly);
    }

    async fn remove_site_override(&self, site: String) {
        self.db.remove_site_override(&site);
    }

    async fn get_series_overrides(&self) -> Vec<(String, String)> {
        self.db.get_series_overrides()
    }

    async fn set_series_override(&self, pattern: String, name: String) {
        self.db.set_series_override(&pattern, &name);
    }

    async fn remove_series_override(&self, pattern: String) {
        self.db.remove_series_override(&pattern);
    }

    async fn list_ignored(&self) -> Vec<(String, String)> {
        self.db.list_ignored()
    }

    async fn ignore_target(&self, kind: String, target: String) {
        self.db.ignore_target(&kind, &target);
    }

    async fn unignore_target(&self, kind: String, target: String) {
        self.db.unignore_target(&kind, &target);
    }

    async fn get_setting(&self, key: String) -> zbus::fdo::Result<String> {
        Ok(self.db.get_setting(&key).unwrap_or_default())
    }

    async fn set_setting(&self, key: String, value: String) -> zbus::fdo::Result<()> {
        self.db.set_setting(&key, &value);
        Ok(())
    }

    async fn get_categories(&self) -> zbus::fdo::Result<Vec<(i64, String, String, i64, i64, i64)>> {
        Ok(self.db.categories().iter().map(|c| (c.id, c.name.clone(), c.color.clone(),
            c.is_builtin, c.is_deletable, c.sort)).collect())
    }

    async fn get_category_members(&self, id: i64) -> zbus::fdo::Result<Vec<(String, String)>> {
        Ok(self.db.category_members(id))
    }

    async fn add_category(&self, name: String, color: String) -> zbus::fdo::Result<i64> {
        Ok(self.db.add_category(&name, &color))
    }

    async fn rename_category(&self, id: i64, new_name: String) -> zbus::fdo::Result<()> {
        self.db.rename_category(id, &new_name);
        Ok(())
    }

    async fn set_category_color(&self, id: i64, color: String) -> zbus::fdo::Result<()> {
        self.db.set_category_color(id, &color);
        Ok(())
    }

    async fn add_category_member(&self, id: i64, kind: String, target: String) -> zbus::fdo::Result<()> {
        self.db.add_category_member(id, &kind, &target);
        Ok(())
    }

    async fn delete_category_member(&self, kind: String, target: String) -> zbus::fdo::Result<()> {
        self.db.delete_category_member(&kind, &target);
        Ok(())
    }

    async fn delete_category(&self, id: i64) -> zbus::fdo::Result<bool> {
        Ok(self.db.delete_category(id))
    }

    #[zbus(signal)]
    async fn window_changed(&self, ctxt: &zbus::object_server::SignalContext<'_>, app_name: &str, window_title: &str, since: i64) -> zbus::Result<()>;
}

pub async fn serve(db: Arc<Db>) -> zbus::Result<(zbus::Connection, ActivityTracker)> {
    let tracker = ActivityTracker::new(db);
    let emitter = tracker.clone();
    let conn = ConnectionBuilder::session()?
        .name("com.Saqr.Salomnella")?
        .serve_at("/com/Saqr/Salomnella", tracker)?
        .build()
        .await?;
    emitter.set_connection(conn.clone());
    Ok((conn, emitter))
}
